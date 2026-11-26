//! Shared integration test helpers.

#![expect(
    dead_code,
    reason = "shared integration-test support exposes helpers used by different test binaries"
)]

use aster_yggdrasil::runtime::{AppState, AppStateParts};
use secrecy::ExposeSecret;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

thread_local! {
    static LAST_CSRF_TOKENS_BY_SESSION: RefCell<HashMap<String, String>> =
        RefCell::new(HashMap::new());
}

const TEST_DATABASE_BACKEND_ENV: &str = "ASTER_TEST_DATABASE_BACKEND";
const TEST_DATABASE_BACKEND_ALIAS_ENV: &str = "ASTER_DATABASE_BACKEND";
const SHARED_TEST_CONTAINER_STATE_DIR: &str = "/tmp/asteryggdrasil-testcontainers";

fn init_test_process_state() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // Forge JWT issue/verify refuses to run until the crypto backend is
        // initialized; binaries do this in bootstrap, test processes do it here.
        let _ = aster_forge_auth::initialize_jwt_crypto_provider();
    });
}

pub async fn set_foreign_key_checks(
    db: &sea_orm::DatabaseConnection,
    enabled: bool,
) -> Result<(), sea_orm::DbErr> {
    use sea_orm::ConnectionTrait;

    let sql = match (db.get_database_backend(), enabled) {
        (sea_orm::DbBackend::Sqlite, true) => "PRAGMA foreign_keys=ON;",
        (sea_orm::DbBackend::Sqlite, false) => "PRAGMA foreign_keys=OFF;",
        (sea_orm::DbBackend::Postgres, true) => "SET session_replication_role = origin;",
        (sea_orm::DbBackend::Postgres, false) => "SET session_replication_role = replica;",
        (sea_orm::DbBackend::MySql, true) => "SET FOREIGN_KEY_CHECKS = 1;",
        (sea_orm::DbBackend::MySql, false) => "SET FOREIGN_KEY_CHECKS = 0;",
        _ => return Ok(()),
    };

    db.execute_unprepared(sql).await.map(|_| ())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestDatabaseBackend {
    Sqlite,
    Postgres,
    MySql,
}

struct MySqlSchemaTemplate {
    database_name: String,
    create_table_sql: Vec<String>,
}

struct PostgresDatabaseTemplate {
    database_name: String,
}

static POSTGRES_TEST_CONTAINER: tokio::sync::OnceCell<
    aster_forge_test::postgres::PostgresTestContainer,
> = tokio::sync::OnceCell::const_new();
static MYSQL_TEST_CONTAINER: tokio::sync::OnceCell<aster_forge_test::mysql::MysqlTestContainer> =
    tokio::sync::OnceCell::const_new();
const POSTGRES_TEST_SESSION_OPTIONS: &str =
    "-c max_parallel_workers_per_gather=0 -c max_parallel_maintenance_workers=0";

fn test_container_suite() -> &'static aster_forge_test::suite::TestContainerSuite {
    static SUITE: OnceLock<aster_forge_test::suite::TestContainerSuite> = OnceLock::new();
    SUITE.get_or_init(|| aster_forge_test::suite::TestContainerSuite::new("asteryggdrasil"))
}
static POSTGRES_DATABASE_TEMPLATE: tokio::sync::OnceCell<PostgresDatabaseTemplate> =
    tokio::sync::OnceCell::const_new();
static MYSQL_SCHEMA_TEMPLATE: tokio::sync::OnceCell<MySqlSchemaTemplate> =
    tokio::sync::OnceCell::const_new();

impl TestDatabaseBackend {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
        }
    }
}

fn test_backend_from_database_backend(backend: sea_orm::DbBackend) -> Option<TestDatabaseBackend> {
    match backend {
        sea_orm::DbBackend::Postgres => Some(TestDatabaseBackend::Postgres),
        sea_orm::DbBackend::MySql => Some(TestDatabaseBackend::MySql),
        _ => None,
    }
}

async fn drop_stale_test_databases(
    backend: sea_orm::DbBackend,
    admin_database_url: &str,
    database_names: &[String],
) {
    if database_names.is_empty() {
        return;
    }

    use sea_orm::ConnectionTrait;

    let admin_cfg = aster_yggdrasil::config::DatabaseConfig {
        url: admin_database_url.into(),
        pool_size: 1,
        retry_count: 0,
    };
    let admin_db = aster_yggdrasil::db::connect_with_metrics(
        &admin_cfg,
        aster_forge_metrics::NoopMetrics::arc(),
    )
    .await
    .expect("stale test database cleanup should connect");

    for database_name in database_names {
        let drop_sql = format!(
            "DROP DATABASE IF EXISTS {}",
            quote_database_identifier(backend, database_name)
        );
        admin_db
            .execute_unprepared(&drop_sql)
            .await
            .expect("stale test database should drop");
    }
}

async fn ensure_mysql_test_user_access(admin_database_url: &str, username: &str) {
    use sea_orm::ConnectionTrait;

    let admin_cfg = aster_yggdrasil::config::DatabaseConfig {
        url: admin_database_url.into(),
        pool_size: 1,
        retry_count: 0,
    };
    let admin_db = aster_yggdrasil::db::connect_with_metrics(
        &admin_cfg,
        aster_forge_metrics::NoopMetrics::arc(),
    )
    .await
    .expect("mysql test admin connection should succeed");

    let username = quote_mysql_string(username);
    let password = quote_mysql_string("asterpass");
    admin_db
        .execute_unprepared(&format!(
            "CREATE USER IF NOT EXISTS {username}@'%' IDENTIFIED BY {password}"
        ))
        .await
        .expect("mysql test user should exist");
    admin_db
        .execute_unprepared(&format!(
            "ALTER USER {username}@'%' IDENTIFIED BY {password}"
        ))
        .await
        .expect("mysql test user password should be current");

    let grant_sql = format!("GRANT ALL PRIVILEGES ON *.* TO {username}@'%'");
    admin_db
        .execute_unprepared(&grant_sql)
        .await
        .expect("mysql test user grant should succeed");
    admin_db
        .close()
        .await
        .expect("mysql test user setup connection should close");
}

fn configured_database_backend_env_value() -> Option<String> {
    std::env::var(TEST_DATABASE_BACKEND_ENV)
        .ok()
        .or_else(|| std::env::var(TEST_DATABASE_BACKEND_ALIAS_ENV).ok())
}

fn configured_test_database_backend() -> TestDatabaseBackend {
    match configured_database_backend_env_value()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        None | Some("") | Some("sqlite") => TestDatabaseBackend::Sqlite,
        Some("postgres") | Some("postgresql") => TestDatabaseBackend::Postgres,
        Some("mysql") => TestDatabaseBackend::MySql,
        Some(other) => panic!(
            "unsupported {TEST_DATABASE_BACKEND_ENV}/{TEST_DATABASE_BACKEND_ALIAS_ENV} value '{other}', expected sqlite/postgres/mysql"
        ),
    }
}

async fn start_mysql_test_container() -> aster_forge_test::mysql::MysqlTestContainer {
    let container =
        aster_forge_test::mysql::MysqlTestContainer::start(test_container_suite()).await;
    ensure_mysql_test_user_access(container.root_url(), "aster").await;
    drop_stale_test_databases(
        sea_orm::DbBackend::MySql,
        container.root_url(),
        container.stale_resources(),
    )
    .await;
    container.forget_resources(container.stale_resources());
    container
}

fn postgres_test_session_url(database_url: &str) -> String {
    let mut url = url::Url::parse(database_url).expect("postgres test URL should parse");
    url.query_pairs_mut()
        .append_pair("options", POSTGRES_TEST_SESSION_OPTIONS);
    url.to_string()
}

/// Builds the product database URL with backend-specific test credentials and limits.
fn product_database_url(database_url: &str, backend: TestDatabaseBackend) -> String {
    let mut url = url::Url::parse(database_url).expect("test database URL should parse");
    url.set_path("/asteryggdrasil");
    if backend == TestDatabaseBackend::MySql {
        url.set_username("aster")
            .expect("MySQL test URL should accept a username");
        url.set_password(Some("asterpass"))
            .expect("MySQL test URL should accept a password");
    }
    let database_url = url.to_string();
    if backend == TestDatabaseBackend::Postgres {
        postgres_test_session_url(&database_url)
    } else {
        database_url
    }
}

async fn shared_test_database_urls(backend: TestDatabaseBackend) -> (String, String) {
    match backend {
        TestDatabaseBackend::Sqlite => {
            ("sqlite::memory:".to_string(), "sqlite::memory:".to_string())
        }
        TestDatabaseBackend::Postgres => {
            let container = POSTGRES_TEST_CONTAINER
                .get_or_init(|| {
                    aster_forge_test::postgres::PostgresTestContainer::start(test_container_suite())
                })
                .await;
            (
                container.admin_url().to_string(),
                product_database_url(container.admin_url(), backend),
            )
        }
        TestDatabaseBackend::MySql => {
            let container = MYSQL_TEST_CONTAINER
                .get_or_init(start_mysql_test_container)
                .await;
            (
                container.root_url().to_string(),
                product_database_url(container.root_url(), backend),
            )
        }
    }
}

fn sanitized_database_name_prefix(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "asteryggdrasil".to_string()
    } else {
        trimmed.to_string()
    }
}

fn isolated_database_name(base_name: &str, max_len: usize) -> String {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let reserved = "_it_".len() + suffix.len();
    let max_prefix_len = max_len.saturating_sub(reserved).max(1);
    let prefix = sanitized_database_name_prefix(base_name)
        .chars()
        .take(max_prefix_len)
        .collect::<String>();

    format!("{prefix}_it_{suffix}")
}

fn database_name_from_url(url: &url::Url) -> Option<String> {
    url.path_segments()
        .and_then(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .rfind(|segment| !segment.is_empty())
                .map(str::to_string)
        })
        .filter(|value| !value.is_empty())
}

fn replace_database_name(mut url: url::Url, database_name: &str) -> String {
    url.set_path(&format!("/{database_name}"));
    url.to_string()
}

fn quote_database_identifier(backend: sea_orm::DbBackend, database_name: &str) -> String {
    match backend {
        sea_orm::DbBackend::Postgres => format!("\"{}\"", database_name.replace('"', "\"\"")),
        sea_orm::DbBackend::MySql => format!("`{}`", database_name.replace('`', "``")),
        _ => database_name.to_string(),
    }
}

fn quote_mysql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

async fn provision_isolated_test_database_url_with_template(
    admin_database_url: &str,
    database_url: &str,
    template_database_name: Option<&str>,
) -> String {
    if database_url == "sqlite::memory:" || database_url.starts_with("sqlite://") {
        return database_url.to_string();
    }

    let parsed_url = url::Url::parse(database_url).expect("test database url should parse");
    let base_name =
        database_name_from_url(&parsed_url).unwrap_or_else(|| "asteryggdrasil".to_string());
    let backend = match parsed_url.scheme() {
        "postgres" | "postgresql" => sea_orm::DbBackend::Postgres,
        "mysql" => sea_orm::DbBackend::MySql,
        scheme => panic!("unsupported isolated test database URL scheme: {scheme}"),
    };

    let isolated_name = match backend {
        sea_orm::DbBackend::Postgres => isolated_database_name(&base_name, 63),
        sea_orm::DbBackend::MySql => isolated_database_name(&base_name, 64),
        _ => unreachable!("isolated database provisioning only supports postgres/mysql"),
    };

    match backend {
        sea_orm::DbBackend::Postgres => {
            let container = POSTGRES_TEST_CONTAINER
                .get_or_init(|| {
                    aster_forge_test::postgres::PostgresTestContainer::start(test_container_suite())
                })
                .await;
            let database = match template_database_name {
                Some(template) => {
                    container
                        .create_database_from_template(&isolated_name, template)
                        .await
                }
                None => container.create_database(&isolated_name).await,
            };
            postgres_test_session_url(database.url())
        }
        sea_orm::DbBackend::MySql => {
            use sea_orm::ConnectionTrait;

            let container = MYSQL_TEST_CONTAINER
                .get_or_init(start_mysql_test_container)
                .await;
            container.remember_resource(&isolated_name);

            let admin_cfg = aster_yggdrasil::config::DatabaseConfig {
                url: admin_database_url.into(),
                pool_size: 1,
                retry_count: 0,
            };
            let admin_db = aster_yggdrasil::db::connect_with_metrics(
                &admin_cfg,
                aster_forge_metrics::NoopMetrics::arc(),
            )
            .await
            .expect("MySQL test admin connection should succeed");
            admin_db
                .execute_unprepared(&format!(
                    "CREATE DATABASE {}",
                    quote_database_identifier(backend, &isolated_name)
                ))
                .await
                .expect("isolated MySQL test database should be created");
            admin_db
                .close()
                .await
                .expect("MySQL test admin connection should close");

            replace_database_name(parsed_url, &isolated_name)
        }
        _ => unreachable!("isolated database provisioning only supports postgres/mysql"),
    }
}

async fn provision_isolated_test_database_url(
    admin_database_url: &str,
    database_url: &str,
) -> String {
    provision_isolated_test_database_url_with_template(admin_database_url, database_url, None).await
}

async fn build_postgres_database_template() -> PostgresDatabaseTemplate {
    let (admin_database_url, database_url) =
        shared_test_database_urls(TestDatabaseBackend::Postgres).await;
    let template_database_url =
        provision_isolated_test_database_url(&admin_database_url, &database_url).await;

    let db_cfg = aster_yggdrasil::config::DatabaseConfig {
        url: template_database_url.clone().into(),
        pool_size: 1,
        retry_count: 0,
    };
    let db =
        aster_yggdrasil::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("postgres template database connection should succeed");

    aster_yggdrasil_migration::Migrator::up(&db, None)
        .await
        .expect("postgres template database migrations should succeed");
    db.close()
        .await
        .expect("postgres template database should close cleanly");

    let template_database_name = url::Url::parse(&template_database_url)
        .ok()
        .and_then(|url| database_name_from_url(&url))
        .expect("postgres template database name should exist");

    PostgresDatabaseTemplate {
        database_name: template_database_name,
    }
}

async fn resolve_test_database_url_for(backend: TestDatabaseBackend) -> String {
    let (admin_database_url, database_url) = shared_test_database_urls(backend).await;
    match backend {
        TestDatabaseBackend::Postgres => {
            let template = POSTGRES_DATABASE_TEMPLATE
                .get_or_init(build_postgres_database_template)
                .await;
            provision_isolated_test_database_url_with_template(
                &admin_database_url,
                &database_url,
                Some(&template.database_name),
            )
            .await
        }
        _ => provision_isolated_test_database_url(&admin_database_url, &database_url).await,
    }
}

async fn resolve_test_database_url() -> String {
    resolve_test_database_url_for(configured_test_database_backend()).await
}

pub async fn postgres_test_database_url() -> String {
    resolve_test_database_url_for(TestDatabaseBackend::Postgres).await
}

pub async fn mysql_test_database_url() -> String {
    resolve_test_database_url_for(TestDatabaseBackend::MySql).await
}

/// Provisions an isolated PostgreSQL database with **no** migrations applied,
/// for tests that replay partial migration sequences against legacy data.
pub async fn empty_postgres_test_database_url() -> String {
    let (admin_database_url, database_url) =
        shared_test_database_urls(TestDatabaseBackend::Postgres).await;
    provision_isolated_test_database_url(&admin_database_url, &database_url).await
}

/// Provisions an isolated MySQL database with **no** migrations applied.
pub async fn empty_mysql_test_database_url() -> String {
    let (admin_database_url, database_url) =
        shared_test_database_urls(TestDatabaseBackend::MySql).await;
    provision_isolated_test_database_url(&admin_database_url, &database_url).await
}

pub async fn setup() -> AppState {
    init_test_process_state();
    let database_url = resolve_test_database_url().await;
    setup_with_database_url(&database_url).await
}

/// Like [`setup`], but with storage and temp roots isolated to a fresh temp
/// directory, so tests that touch preview caches never see objects written by
/// other tests (or by a developer's local instance running in this directory).
pub async fn setup_with_isolated_storage() -> (AppState, aster_forge_utils::raii::TempDirGuard) {
    let base = setup().await;
    let root = std::env::temp_dir().join(format!(
        "asteryggdrasil-isolated-storage-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(root.join("tmp")).expect("isolated storage root should create");
    // The guard cleans up even when an assertion panics mid-test.
    let guard = aster_forge_utils::raii::TempDirGuard::new(root, "isolated storage test");

    let mut config = (*base.config).clone();
    let root = guard.path().to_path_buf();
    config.texture_storage.local_root = root.join("storage").to_string_lossy().into_owned();
    config.server.temp_dir = root.join("tmp").to_string_lossy().into_owned();
    let config = std::sync::Arc::new(config);

    let state = AppState::from_parts(AppStateParts {
        db_handles: base.db_handles,
        config: config.clone(),
        runtime_config: base.runtime_config,
        cache: base.cache,
        texture_storage: aster_yggdrasil::texture_storage::create_texture_store(
            &config.texture_storage,
        )
        .expect("isolated object storage should initialize"),
        mail_sender: aster_forge_mail::memory_sender(),
        config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test("aster_yggdrasil"),
        metrics: base.metrics,
        audit_writer: base.audit_writer,
    })
    .expect("isolated-storage test AppState should build");

    (state, guard)
}

pub async fn setup_with_memory_cache() -> AppState {
    let base = setup().await;
    let cache_config = aster_forge_cache::CacheConfig {
        backend: "memory".to_string(),
        default_ttl: 60,
        ..Default::default()
    };
    let cache = aster_forge_cache::create_cache(&cache_config).await;

    AppState::from_parts(AppStateParts {
        db_handles: base.db_handles,
        config: base.config.clone(),
        runtime_config: base.runtime_config,
        cache,
        texture_storage: base.texture_storage,
        mail_sender: aster_forge_mail::memory_sender(),
        config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test("aster_yggdrasil"),
        metrics: aster_forge_metrics::NoopMetrics::arc(),
        audit_writer: base.audit_writer.clone(),
    })
    .expect("memory-cache test AppState should build")
}

fn should_use_mysql_schema_template(database_url: &str) -> bool {
    database_url.starts_with("mysql://")
        && configured_test_database_backend() == TestDatabaseBackend::MySql
}

async fn load_mysql_schema_template(
    db: &sea_orm::DatabaseConnection,
    database_name: String,
) -> MySqlSchemaTemplate {
    use sea_orm::{ConnectionTrait, Statement};

    let tables = db
        .query_all_raw(Statement::from_string(
            sea_orm::DbBackend::MySql,
            "SHOW FULL TABLES WHERE Table_type = 'BASE TABLE'",
        ))
        .await
        .expect("mysql schema template should list tables");

    let mut table_names: Vec<String> = tables
        .into_iter()
        .map(|row| {
            row.try_get_by_index(0)
                .expect("mysql schema template table name should exist")
        })
        .collect();
    table_names.sort();

    let mut create_table_sql = Vec::with_capacity(table_names.len());
    for table_name in &table_names {
        let ddl_row = db
            .query_one_raw(Statement::from_string(
                sea_orm::DbBackend::MySql,
                format!(
                    "SHOW CREATE TABLE {}",
                    quote_database_identifier(sea_orm::DbBackend::MySql, table_name)
                ),
            ))
            .await
            .expect("mysql schema template should load table ddl")
            .expect("mysql schema template show create table should return one row");

        let ddl: String = ddl_row
            .try_get_by_index(1)
            .expect("mysql schema template ddl should exist");
        create_table_sql.push(ddl);
    }

    MySqlSchemaTemplate {
        database_name,
        create_table_sql,
    }
}

async fn build_mysql_schema_template() -> MySqlSchemaTemplate {
    let (admin_database_url, database_url) =
        shared_test_database_urls(TestDatabaseBackend::MySql).await;
    let template_database_url =
        provision_isolated_test_database_url(&admin_database_url, &database_url).await;

    let db_cfg = aster_yggdrasil::config::DatabaseConfig {
        url: template_database_url.clone().into(),
        pool_size: 1,
        retry_count: 0,
    };
    let db =
        aster_yggdrasil::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("mysql schema template connection should succeed");

    aster_yggdrasil_migration::Migrator::up(&db, None)
        .await
        .expect("mysql schema template migrations should succeed");

    let template_database_name = url::Url::parse(&template_database_url)
        .ok()
        .and_then(|url| database_name_from_url(&url))
        .expect("mysql schema template database name should exist");

    load_mysql_schema_template(&db, template_database_name).await
}

async fn clone_mysql_schema_from_template(db: &sea_orm::DatabaseConnection) {
    use sea_orm::ConnectionTrait;

    let template = MYSQL_SCHEMA_TEMPLATE
        .get_or_init(build_mysql_schema_template)
        .await;

    set_foreign_key_checks(db, false)
        .await
        .expect("mysql schema clone should disable foreign key checks");

    for ddl in &template.create_table_sql {
        db.execute_unprepared(ddl)
            .await
            .expect("mysql schema clone should create table");
    }

    db.execute_unprepared(&format!(
        "INSERT INTO seaql_migrations SELECT * FROM {}.seaql_migrations",
        quote_database_identifier(sea_orm::DbBackend::MySql, &template.database_name)
    ))
    .await
    .expect("mysql schema clone should copy seaql_migrations rows");

    set_foreign_key_checks(db, true)
        .await
        .expect("mysql schema clone should restore foreign key checks");
}

pub async fn setup_with_database_url(database_url: &str) -> AppState {
    init_test_process_state();

    let db_cfg = aster_yggdrasil::config::DatabaseConfig {
        url: database_url.into(),
        pool_size: 1,
        retry_count: 0,
    };
    let writer =
        aster_yggdrasil::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("test database should connect");

    if should_use_mysql_schema_template(database_url) {
        clone_mysql_schema_from_template(&writer).await;
    } else {
        aster_yggdrasil_migration::Migrator::up(&writer, None)
            .await
            .expect("test database migrations should run");
    }

    aster_yggdrasil::services::ops::config::ensure_defaults(&writer)
        .await
        .expect("system config defaults should seed");
    aster_yggdrasil::services::yggdrasil::session_forward::ensure_builtin_servers(&writer)
        .await
        .expect("yggdrasil session forward defaults should seed");
    aster_yggdrasil::services::yggdrasil::signature::ensure_signature_key(&writer)
        .await
        .expect("yggdrasil signature key should initialize");

    let test_dir = format!("/tmp/asteryggdrasil-test-{}", uuid::Uuid::new_v4());
    let temp_dir = format!("{test_dir}/temp");
    std::fs::create_dir_all(&temp_dir).expect("test temp dir should exist");

    let config = std::sync::Arc::new(aster_yggdrasil::config::Config {
        server: aster_yggdrasil::config::ServerConfig {
            temp_dir,
            ..Default::default()
        },
        database: db_cfg.clone(),
        auth: aster_yggdrasil::config::AuthConfig {
            jwt_secret: secrecy::SecretString::from(
                "test-secret-key-for-integration-tests".to_string(),
            ),
            bootstrap_insecure_cookies: true,
            ..Default::default()
        },
        cache: aster_forge_cache::CacheConfig {
            ..Default::default()
        },
        rate_limit: aster_yggdrasil::config::RateLimitConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    });

    let _ = aster_yggdrasil::config::set_config_for_test(config.clone());

    let runtime_config = std::sync::Arc::new(aster_yggdrasil::config::RuntimeConfig::new());
    runtime_config
        .reload(&writer)
        .await
        .expect("runtime config should reload");
    runtime_config
        .install_auth_runtimes(
            config.auth.jwt_secret.expose_secret(),
            config.auth.password_hash_max_concurrency,
        )
        .await
        .expect("auth runtimes should install");

    let cache = aster_forge_cache::create_cache(&config.cache).await;
    let texture_storage =
        aster_yggdrasil::texture_storage::create_texture_store(&config.texture_storage)
            .expect("texture storage should initialize");
    let db_handles = aster_yggdrasil::db::connect_reader_for_writer_with_metrics(
        &db_cfg,
        writer.clone(),
        aster_forge_metrics::NoopMetrics::arc(),
    )
    .await
    .expect("test db handles should initialize");
    let state = AppState::from_parts(AppStateParts {
        db_handles,
        config,
        runtime_config,
        cache,
        texture_storage,
        mail_sender: aster_forge_mail::memory_sender(),
        config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test("aster_yggdrasil"),
        metrics: aster_forge_metrics::NoopMetrics::arc(),
        audit_writer: std::sync::Arc::new(aster_forge_audit::AuditWriter::new(writer)),
    })
    .expect("integration test AppState should build");

    // The runtime component bundle never runs in this process, so start the
    // writer here for audit-writing endpoints under test.
    state
        .audit_writer()
        .start()
        .await
        .expect("integration test audit writer should start");

    state
}

pub fn bearer_header(access_token: impl AsRef<str>) -> (&'static str, String) {
    ("Authorization", format!("Bearer {}", access_token.as_ref()))
}

pub fn access_cookie_header(access_token: impl AsRef<str>) -> String {
    let access_token = access_token.as_ref();
    let csrf_token = csrf_token_for(access_token);
    pin_csrf_token_for_current_thread(access_token, &csrf_token);
    format!("aster_access={access_token}; aster_csrf={csrf_token}")
}

pub fn refresh_cookie_header(
    refresh_token: impl AsRef<str>,
    csrf_token: impl AsRef<str>,
) -> String {
    let refresh_token = refresh_token.as_ref();
    let csrf_token = csrf_token.as_ref();
    pin_csrf_token_for_current_thread(refresh_token, csrf_token);
    format!("aster_refresh={refresh_token}; aster_csrf={csrf_token}")
}

pub fn access_and_refresh_cookie_header(
    access_token: impl AsRef<str>,
    refresh_token: impl AsRef<str>,
    csrf_token: impl AsRef<str>,
) -> String {
    let access_token = access_token.as_ref();
    let refresh_token = refresh_token.as_ref();
    let csrf_token = csrf_token.as_ref();
    pin_csrf_token_for_current_thread(access_token, csrf_token);
    pin_csrf_token_for_current_thread(refresh_token, csrf_token);
    format!("aster_access={access_token}; aster_refresh={refresh_token}; aster_csrf={csrf_token}")
}

pub fn csrf_header(csrf_token: impl AsRef<str>) -> (&'static str, String) {
    ("X-CSRF-Token", csrf_token.as_ref().to_string())
}

fn csrf_tokens_by_session() -> &'static Mutex<HashMap<String, String>> {
    static TOKENS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember_csrf_token(session_token: &str, csrf_token: &str) {
    csrf_tokens_by_session()
        .lock()
        .expect("csrf token map should lock")
        .insert(session_token.to_string(), csrf_token.to_string());
}

fn pin_csrf_token_for_current_thread(session_token: &str, csrf_token: &str) {
    LAST_CSRF_TOKENS_BY_SESSION.with(|tokens| {
        tokens
            .borrow_mut()
            .insert(session_token.to_string(), csrf_token.to_string());
    });
}

pub fn csrf_token_for(session_token: impl AsRef<str>) -> String {
    let token = session_token.as_ref();
    if let Some(csrf_token) =
        LAST_CSRF_TOKENS_BY_SESSION.with(|tokens| tokens.borrow().get(token).cloned())
    {
        return csrf_token;
    }

    csrf_tokens_by_session()
        .lock()
        .expect("csrf token map should lock")
        .get(token)
        .cloned()
        .unwrap_or_else(|| panic!("missing csrf token for session token: {token}"))
}

pub fn csrf_header_for(session_token: impl AsRef<str>) -> (&'static str, String) {
    csrf_header(csrf_token_for(session_token))
}

pub fn extract_cookie<B>(resp: &actix_web::dev::ServiceResponse<B>, name: &str) -> Option<String> {
    let value = resp
        .headers()
        .get_all(actix_web::http::header::SET_COOKIE)
        .filter_map(|value| value.to_str().ok())
        .find_map(|raw| {
            raw.split(';')
                .next()
                .and_then(|pair| pair.strip_prefix(&format!("{name}=")))
                .map(str::to_string)
        })?;

    if matches!(name, "aster_access" | "aster_refresh")
        && let Some(csrf_token) = resp
            .headers()
            .get_all(actix_web::http::header::SET_COOKIE)
            .filter_map(|value| value.to_str().ok())
            .find_map(|raw| {
                raw.split(';')
                    .next()
                    .and_then(|pair| pair.strip_prefix("aster_csrf="))
                    .map(str::to_string)
            })
    {
        remember_csrf_token(&value, &csrf_token);
    }

    Some(value)
}

pub async fn flush_mail_outbox(state: &AppState) {
    flush_mail_outbox_with(state.writer_db(), &state.runtime_config, &state.mail_sender).await;
}

pub async fn flush_mail_outbox_with(
    db: &sea_orm::DatabaseConnection,
    runtime_config: &std::sync::Arc<aster_yggdrasil::config::RuntimeConfig>,
    mail_sender: &std::sync::Arc<dyn aster_forge_mail::MailSender>,
) {
    aster_yggdrasil::services::mail::outbox::drain_with(db, runtime_config, mail_sender)
        .await
        .expect("mail outbox drain should succeed");
}

fn extract_token_from_content(content: &str, marker: &str) -> Option<String> {
    let (_, suffix) = content.split_once(marker)?;
    let encoded: String = suffix
        .chars()
        .take_while(|ch| !matches!(ch, '"' | '\'' | '<' | '>' | '&' | ' ' | '\r' | '\n'))
        .collect();
    if encoded.is_empty() {
        return None;
    }

    urlencoding::decode(&encoded)
        .ok()
        .map(|value| value.into_owned())
}

pub fn extract_token_from_mail_message(
    message: &aster_forge_mail::MailMessage,
    marker: &str,
) -> Option<String> {
    extract_token_from_content(&message.text_body, marker)
        .or_else(|| extract_token_from_content(&message.html_body, marker))
}

pub fn system_config_model(key: &str, value: &str) -> aster_forge_db::system_config::Model {
    aster_forge_db::system_config::Model {
        id: 0,
        key: key.to_string(),
        value: value.to_string(),
        value_type: aster_forge_config::ConfigValueType::String,
        requires_restart: false,
        is_sensitive: false,
        source: aster_forge_config::ConfigSource::System,
        visibility: aster_forge_config::ConfigVisibility::Private,
        namespace: String::new(),
        category: aster_yggdrasil::config::definitions::CONFIG_CATEGORY_SITE_PUBLIC.to_string(),
        description: "test".to_string(),
        updated_at: chrono::Utc::now(),
        updated_by: None,
    }
}

/// Test helper: Builds an actix test app wired to the given app state.
#[macro_export]
macro_rules! create_test_app {
    ($state:expr) => {{
        use actix_web::{App, test, web};

        let state = $state;
        let metrics = state.metrics.clone();

        test::init_service(
            App::new()
                .wrap(aster_forge_middleware::actix::request_id::RequestIdMiddleware)
                .wrap(aster_yggdrasil::api::middleware::cors::RuntimeCors)
                .wrap(aster_forge_middleware::actix::security_headers::default_headers())
                .wrap(aster_forge_middleware::actix::metrics::MetricsMiddleware)
                .app_data(web::PayloadConfig::new(10 * 1024 * 1024))
                .app_data(web::JsonConfig::default().limit(1024 * 1024))
                .app_data(web::Data::new(metrics))
                .app_data(web::Data::new(state))
                .service(
                    web::scope(aster_yggdrasil::config::yggdrasil::DEFAULT_YGGDRASIL_API_ROOT)
                        .configure(aster_yggdrasil::api::routes::yggdrasil::configure),
                )
                .service(aster_yggdrasil::api::routes::health::routes(
                    &aster_yggdrasil::config::Config::default().observability,
                ))
                .service(
                    web::scope("/api/v1").configure(aster_yggdrasil::api::routes::configure_api),
                )
                .service(aster_yggdrasil::api::routes::frontend::routes()),
        )
        .await
    }};
}

/// Test helper: Asserts an app call ends with the expected HTTP status, covering both success and error responses.
#[macro_export]
macro_rules! assert_service_status {
    ($app:expr, $req:expr, $status:expr) => {{
        use actix_web::test;

        let result = test::try_call_service(&$app, $req).await;
        match result {
            Ok(resp) => assert_eq!(resp.status(), $status),
            Err(err) => assert_eq!(err.error_response().status(), $status),
        }
    }};
    ($app:expr, $req:expr, $status:expr, $msg:expr) => {{
        use actix_web::test;

        let result = test::try_call_service(&$app, $req).await;
        match result {
            Ok(resp) => assert_eq!(resp.status(), $status, $msg),
            Err(err) => assert_eq!(err.error_response().status(), $status, $msg),
        }
    }};
}

/// Test helper: Completes first-run setup and returns the admin session token.
#[macro_export]
macro_rules! setup_admin {
    ($app:expr) => {{
        use actix_web::test;
        use serde_json::Value;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/setup")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "password1234"
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "admin setup should return 200");
        let access_token =
            common::extract_cookie(&resp, "aster_access").expect("admin setup access cookie missing");
        let body: Value = test::read_body_json(resp).await;
        assert!(body["data"]["expires_in"].is_number());
        access_token
    }};
}

/// Test helper: Registers a user account and returns the activation token response.
#[macro_export]
macro_rules! register_user {
    ($app:expr, $username:expr, $email:expr, $password:expr) => {{
        use actix_web::test;
        use serde_json::Value;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "username": $username,
                "email": $email,
                "password": $password
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "register should return 200");
        let access_token =
            common::extract_cookie(&resp, "aster_access").expect("register access cookie missing");
        let body: Value = test::read_body_json(resp).await;
        assert!(body["data"]["expires_in"].is_number());
        access_token
    }};
}

/// Test helper: Logs a user in and returns the access token response.
#[macro_export]
macro_rules! login_user {
    ($app:expr, $identifier:expr, $password:expr) => {{
        use actix_web::test;
        use serde_json::Value;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "identifier": $identifier,
                "password": $password
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "login should return 200");
        let access_token =
            common::extract_cookie(&resp, "aster_access").expect("login access cookie missing");
        let body: Value = test::read_body_json(resp).await;
        assert!(body["data"]["expires_in"].is_number());
        access_token
    }};
}

/// Test helper: Registers and activates a fresh user, returning the login response.
#[macro_export]
macro_rules! register_and_login {
    ($app:expr) => {{
        use actix_web::test;

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "username": "testuser",
                "email": "test@example.com",
                "password": "password123"
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "register should return 200");

        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "identifier": "testuser",
                "password": "password123"
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "login should return 200");
        let access =
            common::extract_cookie(&resp, "aster_access").expect("access cookie missing");
        let refresh =
            common::extract_cookie(&resp, "aster_refresh").expect("refresh cookie missing");
        (access, refresh)
    }};
}

/// Test helper: Creates a user as an admin and returns the admin API response.
#[macro_export]
macro_rules! admin_create_user {
    ($app:expr, $admin_token:expr, $username:expr, $email:expr, $password:expr) => {{
        use actix_web::test;
        use serde_json::Value;

        let req = test::TestRequest::post()
            .uri("/api/v1/admin/users")
            .insert_header(("Cookie", common::access_cookie_header(&$admin_token)))
            .insert_header(common::csrf_header_for(&$admin_token))
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "username": $username,
                "email": $email,
                "password": $password
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        if resp.status() != 201 {
            let status = resp.status();
            let body: Value = test::read_body_json(resp).await;
            panic!("admin create user should return 201, got {status}: {body}");
        }
        let body: Value = test::read_body_json(resp).await;
        body["data"]["user"]["id"].as_i64().unwrap()
    }};
}

/// Drives one queue-consumer pass against the real claim/execute path: the
/// fallback lane consumer claims up to `limit` due tasks and executes each to
/// terminal state. Mirrors what a worker process does per poll tick.
pub async fn run_pending_tasks_once(
    state: &AppState,
    limit: usize,
) -> Result<Vec<aster_forge_tasks::TaskDispatchOutcome>, aster_yggdrasil::errors::AsterError> {
    use aster_forge_tasks::TaskConsumer;

    let consumer = aster_yggdrasil::services::ops::task::FallbackTaskConsumer::new(
        actix_web::web::Data::new(state.clone()),
    );
    let claimed = TaskConsumer::claim(&consumer, limit).await?;
    let mut outcomes = Vec::with_capacity(claimed.len());
    for (task, lease) in claimed {
        outcomes.push(
            TaskConsumer::execute(
                &consumer,
                (task, lease),
                tokio_util::sync::CancellationToken::new(),
            )
            .await?,
        );
    }
    Ok(outcomes)
}
