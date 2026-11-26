//! PostgreSQL / MySQL smoke tests backed by testcontainers.

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::config::definitions::BRANDING_TITLE_KEY;
use aster_yggdrasil::db::repository::{background_task_repo, system_config_repo, user_repo};
use aster_yggdrasil::entities::background_task;
use aster_yggdrasil::types::{
    task::BackgroundTaskKind, task::BackgroundTaskStatus, task::StoredTaskPayload,
    task::StoredTaskResult,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::Value;
use tokio::time::timeout;

const OLD_BACKGROUND_TASK_DISPLAY_NAME_LIMIT: usize = 255;
const EXPANDED_BACKGROUND_TASK_DISPLAY_NAME_LIMIT: usize = 512;

async fn assert_core_tables_exist(db: &DatabaseConnection, backend: DbBackend) {
    let table_names = match backend {
        DbBackend::Postgres => {
            let rows = db
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT table_name \
                     FROM information_schema.tables \
                     WHERE table_schema = 'public' \
                       AND table_type = 'BASE TABLE'",
                ))
                .await
                .expect("postgres table list should query");
            rows.into_iter()
                .map(|row| row.try_get_by_index(0).expect("table name should exist"))
                .collect::<Vec<String>>()
        }
        DbBackend::MySql => {
            let rows = db
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT table_name \
                     FROM information_schema.tables \
                     WHERE table_schema = DATABASE() \
                       AND table_type = 'BASE TABLE'",
                ))
                .await
                .expect("mysql table list should query");
            rows.into_iter()
                .map(|row| row.try_get_by_index(0).expect("table name should exist"))
                .collect::<Vec<String>>()
        }
        backend => panic!("unsupported test database backend: {backend:?}"),
    };

    for expected in [
        "users",
        "auth_sessions",
        "external_auth_providers",
        "external_auth_identities",
        "external_auth_login_flows",
        "system_config",
        "audit_logs",
        "background_tasks",
        "seaql_migrations",
    ] {
        assert!(
            table_names.iter().any(|name| name == expected),
            "{expected} table should exist in {table_names:?}"
        );
    }
}

async fn assert_mysql_datetime_columns(db: &DatabaseConnection) {
    let timestamp_count = db
        .query_one_raw(Statement::from_string(
            DbBackend::MySql,
            "SELECT COUNT(*) \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() \
               AND TABLE_NAME <> 'seaql_migrations' \
               AND DATA_TYPE = 'timestamp'",
        ))
        .await
        .expect("mysql timestamp count should query")
        .expect("mysql timestamp count should return one row");
    let timestamp_count: i64 = timestamp_count
        .try_get_by_index(0)
        .expect("mysql timestamp count should decode");
    assert_eq!(
        timestamp_count, 0,
        "application tables should use DATETIME instead of TIMESTAMP on MySQL"
    );

    let datetime_count = db
        .query_one_raw(Statement::from_string(
            DbBackend::MySql,
            "SELECT COUNT(*) \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() \
               AND TABLE_NAME <> 'seaql_migrations' \
               AND DATA_TYPE = 'datetime' \
               AND DATETIME_PRECISION = 6",
        ))
        .await
        .expect("mysql datetime count should query")
        .expect("mysql datetime count should return one row");
    let datetime_count: i64 = datetime_count
        .try_get_by_index(0)
        .expect("mysql datetime count should decode");
    assert!(
        datetime_count >= 20,
        "foundation timestamp columns should be datetime(6), got {datetime_count}"
    );
}

async fn assert_background_task_display_name_column_len(
    db: &DatabaseConnection,
    backend: DbBackend,
) {
    let sql = match backend {
        DbBackend::Postgres => {
            "SELECT character_maximum_length::bigint \
             FROM information_schema.columns \
             WHERE table_schema = 'public' \
               AND table_name = 'background_tasks' \
               AND column_name = 'display_name'"
        }
        DbBackend::MySql => {
            "SELECT CAST(CHARACTER_MAXIMUM_LENGTH AS SIGNED) \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() \
               AND TABLE_NAME = 'background_tasks' \
               AND COLUMN_NAME = 'display_name'"
        }
        backend => panic!("unsupported test database backend: {backend:?}"),
    };

    let row = db
        .query_one_raw(Statement::from_string(backend, sql))
        .await
        .expect("background_tasks.display_name length should query")
        .expect("background_tasks.display_name column should exist");
    let max_len: i64 = row
        .try_get_by_index(0)
        .expect("background_tasks.display_name max length should decode");
    assert_eq!(
        max_len,
        i64::try_from(EXPANDED_BACKGROUND_TASK_DISPLAY_NAME_LIMIT).unwrap()
    );
}

async fn assert_background_task_display_name_accepts_expanded_len(db: &DatabaseConnection) {
    let now = Utc::now();
    let display_name = "x".repeat(OLD_BACKGROUND_TASK_DISPLAY_NAME_LIMIT + 1);
    assert!(display_name.len() <= EXPANDED_BACKGROUND_TASK_DISPLAY_NAME_LIMIT);

    let task = background_task_repo::create(
        db,
        background_task::ActiveModel {
            kind: Set(BackgroundTaskKind::SystemRuntime),
            status: Set(BackgroundTaskStatus::Succeeded),
            creator_user_id: Set(None),
            display_name: Set(display_name.clone()),
            payload_json: Set(StoredTaskPayload(
                r#"{"task_name":"expanded-display-name-smoke"}"#.to_string(),
            )),
            result_json: Set(Some(StoredTaskResult(
                r#"{"duration_ms":0,"summary":"expanded display name accepted"}"#.to_string(),
            ))),
            runtime_json: Set(None),
            steps_json: Set(None),
            progress_current: Set(1),
            progress_total: Set(1),
            status_text: Set(Some("expanded display name accepted".to_string())),
            attempt_count: Set(0),
            max_attempts: Set(1),
            next_run_at: Set(now),
            processing_token: Set(0),
            processing_started_at: Set(None),
            last_heartbeat_at: Set(None),
            lease_expires_at: Set(None),
            started_at: Set(Some(now)),
            finished_at: Set(Some(now)),
            last_error: Set(None),
            failure_can_retry: Set(None),
            expires_at: Set(now + Duration::hours(1)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        },
    )
    .await
    .expect("expanded background task display_name should insert");

    assert_eq!(task.display_name, display_name);
}

async fn exercise_foundation_api_smoke(database_url: &str, backend: DbBackend) {
    let state = common::setup_with_database_url(database_url).await;
    assert_eq!(state.writer_db().get_database_backend(), backend);

    assert_core_tables_exist(state.writer_db(), backend).await;
    if backend == DbBackend::MySql {
        assert_mysql_datetime_columns(state.writer_db()).await;
    }
    assert_background_task_display_name_column_len(state.writer_db(), backend).await;
    assert_background_task_display_name_accepts_expanded_len(state.writer_db()).await;

    let configs = system_config_repo::find_all(state.writer_db())
        .await
        .expect("system config defaults should query");
    assert!(
        configs.iter().any(|item| item.key == BRANDING_TITLE_KEY),
        "{BRANDING_TITLE_KEY} default config should be seeded"
    );
    assert_eq!(
        user_repo::count_all(state.writer_db())
            .await
            .expect("user count should query"),
        0
    );

    let app = create_test_app!(state);

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(
        body.get("data").is_none(),
        "public health should use a minimal probe response"
    );

    let req = test::TestRequest::get().uri("/health/ready").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "ready");

    let token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/config")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["data"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/tasks?limit=10")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["data"]["total"]
            .as_u64()
            .is_some_and(|total| total >= 1),
        "admin task list should include the inserted smoke task: {body}"
    );
}

#[actix_web::test]
async fn test_sqlite_transactions_are_serialized_by_single_connection_pool() {
    use sea_orm::TransactionTrait;

    let database_path = format!(
        "/tmp/asteryggdrasil-sqlite-lock-{}.db",
        uuid::Uuid::new_v4()
    );
    let database_url = format!("sqlite://{database_path}");
    let cfg = aster_yggdrasil::config::DatabaseConfig {
        url: database_url.into(),
        pool_size: 8,
        retry_count: 0,
    };
    let db =
        aster_yggdrasil::db::connect_with_metrics(&cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("sqlite lock smoke database should connect");

    let txn = db
        .begin()
        .await
        .expect("first sqlite transaction should start");
    let second_begin = timeout(std::time::Duration::from_millis(100), db.begin()).await;
    assert!(
        second_begin.is_err(),
        "SQLite should serialize transactions by exposing only one pooled connection"
    );

    txn.commit()
        .await
        .expect("first sqlite transaction should commit");

    let second_txn = timeout(std::time::Duration::from_secs(1), db.begin())
        .await
        .expect("second transaction should start after the first commit")
        .expect("second sqlite transaction should start");
    second_txn
        .commit()
        .await
        .expect("second sqlite transaction should commit");

    let _ = tokio::fs::remove_file(database_path).await;
}

#[actix_web::test]
async fn test_postgres_smoke_foundation_contracts() {
    let database_url = common::postgres_test_database_url().await;

    exercise_foundation_api_smoke(&database_url, DbBackend::Postgres).await;
}

#[actix_web::test]
async fn test_mysql_smoke_foundation_contracts() {
    let database_url = common::mysql_test_database_url().await;

    exercise_foundation_api_smoke(&database_url, DbBackend::MySql).await;
}

// ---------------------------------------------------------------------------
// Namespaced permission identifier migration rewrite
// ---------------------------------------------------------------------------

/// Number of migrations applied up to and including `user_bans`; legacy
/// scope rows are seeded against this schema state before the namespaced
/// identifier migration runs.
const MIGRATIONS_THROUGH_USER_BANS: u32 = 15;

async fn exercise_permission_id_migration_rewrite(database_url: &str, backend: DbBackend) {
    let db = match backend {
        DbBackend::Sqlite => {
            let cfg = aster_yggdrasil::config::DatabaseConfig {
                url: database_url.to_string().into(),
                pool_size: 1,
                retry_count: 0,
            };
            aster_yggdrasil::db::connect_with_metrics(&cfg, aster_forge_metrics::NoopMetrics::arc())
                .await
                .expect("migration rewrite database should connect")
        }
        _ => sea_orm::Database::connect(database_url)
            .await
            .expect("migration rewrite database should connect"),
    };

    aster_yggdrasil_migration::Migrator::up(&db, Some(MIGRATIONS_THROUGH_USER_BANS))
        .await
        .expect("partial migrations through user_bans should apply");

    seed_legacy_permission_rows(&db, backend).await;

    aster_yggdrasil_migration::Migrator::up(&db, None)
        .await
        .expect("remaining migrations should apply");

    assert_rewritten_permission_rows(&db, backend).await;
    assert_operator_scope_column_width(&db, backend, 256).await;

    // The reverse path must fold namespaced ids back before narrowing the
    // column, or legacy applications cannot read the rolled-back rows. Only
    // this migration is rolled back: older SQLite down paths have
    // pre-existing drop-column quirks outside this change's scope.
    use aster_yggdrasil_migration::MigratorTrait;
    aster_yggdrasil_migration::Migrator::down(&db, Some(1))
        .await
        .expect("namespaced permission migration rollback should apply");
    assert_legacy_permission_rows(&db, backend).await;
    assert_operator_scope_column_width(&db, backend, 32).await;

    db.close()
        .await
        .expect("migration rewrite database should close");
}

/// Inserts legacy short-name scope rows with raw SQL; the typed application
/// layer now rejects these values by design, so the test must bypass it.
async fn seed_legacy_permission_rows(db: &DatabaseConnection, backend: DbBackend) {
    let now = Utc::now();

    let users_sql = match backend {
        DbBackend::Postgres => {
            "INSERT INTO users \
             (public_uuid, username, email, password_hash, role, status, session_version, \
              created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 'user', 'active', 1, $5, $6)"
        }
        _ => {
            "INSERT INTO users \
             (public_uuid, username, email, password_hash, role, status, session_version, \
              created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'user', 'active', 1, ?, ?)"
        }
    };
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        users_sql,
        [
            sea_orm::sea_query::Value::from("0123456789abcdef0123456789abcdef"),
            sea_orm::sea_query::Value::from("legacy-user"),
            sea_orm::sea_query::Value::from("legacy-user@example.com"),
            sea_orm::sea_query::Value::from("legacy-password-hash"),
            sea_orm::sea_query::Value::from(now),
            sea_orm::sea_query::Value::from(now),
        ],
    ))
    .await
    .expect("legacy user row should insert");

    for (order, legacy_scope) in ["overview", "users"].iter().enumerate() {
        let scopes_sql = match backend {
            DbBackend::Postgres => {
                "INSERT INTO user_operator_scopes (user_id, scope, created_at) \
                 VALUES ($1, $2, $3)"
            }
            _ => "INSERT INTO user_operator_scopes (user_id, scope, created_at) VALUES (?, ?, ?)",
        };
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            scopes_sql,
            [
                sea_orm::sea_query::Value::from(1i64),
                sea_orm::sea_query::Value::from(*legacy_scope),
                sea_orm::sea_query::Value::from(now + chrono::Duration::milliseconds(order as i64)),
            ],
        ))
        .await
        .expect("legacy operator scope row should insert");
    }

    let bans_sql = match backend {
        DbBackend::Postgres => {
            "INSERT INTO user_bans \
             (user_id, scopes, status, reason, starts_at, created_at, updated_at) \
             VALUES ($1, $2, 'active', 'legacy restriction', $3, $4, $5)"
        }
        _ => {
            "INSERT INTO user_bans \
             (user_id, scopes, status, reason, starts_at, created_at, updated_at) \
             VALUES (?, ?, 'active', 'legacy restriction', ?, ?, ?)"
        }
    };
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        bans_sql,
        [
            sea_orm::sea_query::Value::from(1i64),
            sea_orm::sea_query::Value::from(r#"["texture_upload","yggdrasil_access"]"#),
            sea_orm::sea_query::Value::from(now),
            sea_orm::sea_query::Value::from(now),
            sea_orm::sea_query::Value::from(now),
        ],
    ))
    .await
    .expect("legacy ban row should insert");

    let events: Vec<(&str, Option<&str>, &str)> = vec![
        ("created", None, r#"["texture_upload"]"#),
        (
            "updated",
            Some(r#"["yggdrasil_access"]"#),
            r#"["texture_upload","yggdrasil_access"]"#,
        ),
    ];
    for (order, (event_type, previous_scopes, next_scopes)) in events.into_iter().enumerate() {
        let events_sql = match backend {
            DbBackend::Postgres => {
                "INSERT INTO user_ban_events \
                 (ban_id, event_type, previous_scopes, next_scopes, created_at) \
                 VALUES ($1, $2, $3, $4, $5)"
            }
            _ => {
                "INSERT INTO user_ban_events \
                 (ban_id, event_type, previous_scopes, next_scopes, created_at) \
                 VALUES (?, ?, ?, ?, ?)"
            }
        };
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            events_sql,
            [
                sea_orm::sea_query::Value::from(1i64),
                sea_orm::sea_query::Value::from(event_type),
                previous_scopes
                    .map(sea_orm::sea_query::Value::from)
                    .unwrap_or(sea_orm::sea_query::Value::BigUnsigned(None)),
                sea_orm::sea_query::Value::from(next_scopes),
                sea_orm::sea_query::Value::from(now + chrono::Duration::milliseconds(order as i64)),
            ],
        ))
        .await
        .expect("legacy ban event row should insert");
    }
}

async fn assert_rewritten_permission_rows(db: &DatabaseConnection, backend: DbBackend) {
    let operator_scopes = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT scope FROM user_operator_scopes ORDER BY scope",
        ))
        .await
        .expect("operator scopes should query after migration");
    let operator_scopes: Vec<String> = operator_scopes
        .into_iter()
        .map(|row| {
            row.try_get_by_index::<String>(0)
                .expect("operator scope value should decode")
        })
        .collect();
    assert_eq!(
        operator_scopes,
        vec![
            "com.astercosm.yggdrasil.permission.admin.overview",
            "com.astercosm.yggdrasil.permission.admin.users",
        ],
        "operator scopes should be rewritten to namespaced ids"
    );

    let ban_scopes = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT scopes FROM user_bans WHERE user_id = 1",
        ))
        .await
        .expect("ban scopes should query after migration");
    let ban_scopes: Vec<String> = ban_scopes
        .into_iter()
        .map(|row| {
            row.try_get_by_index::<String>(0)
                .expect("ban scopes value should decode")
        })
        .collect();
    assert_eq!(
        ban_scopes,
        vec![
            r#"["com.astercosm.yggdrasil.capability.texture.upload","com.astercosm.yggdrasil.capability.yggdrasil.access"]"#
        ],
        "ban scope arrays should be rewritten to namespaced ids"
    );

    let event_scopes = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT previous_scopes, next_scopes FROM user_ban_events ORDER BY created_at, id",
        ))
        .await
        .expect("ban event scopes should query after migration");
    let event_scopes: Vec<(Option<String>, String)> = event_scopes
        .into_iter()
        .map(|row| {
            (
                row.try_get_by_index::<Option<String>>(0)
                    .expect("previous scopes should decode"),
                row.try_get_by_index::<String>(1)
                    .expect("next scopes should decode"),
            )
        })
        .collect();
    assert_eq!(
        event_scopes,
        vec![
            (
                None,
                r#"["com.astercosm.yggdrasil.capability.texture.upload"]"#.to_string()
            ),
            (
                Some(r#"["com.astercosm.yggdrasil.capability.yggdrasil.access"]"#.to_string()),
                r#"["com.astercosm.yggdrasil.capability.texture.upload","com.astercosm.yggdrasil.capability.yggdrasil.access"]"#.to_string()
            ),
        ],
        "ban event scope snapshots should be rewritten to namespaced ids"
    );
}

/// Asserts the rows match the legacy seed exactly after `down` runs.
async fn assert_legacy_permission_rows(db: &DatabaseConnection, backend: DbBackend) {
    let operator_scopes = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT scope FROM user_operator_scopes ORDER BY scope",
        ))
        .await
        .expect("operator scopes should query after rollback");
    let operator_scopes: Vec<String> = operator_scopes
        .into_iter()
        .map(|row| {
            row.try_get_by_index::<String>(0)
                .expect("operator scope value should decode")
        })
        .collect();
    assert_eq!(
        operator_scopes,
        vec!["overview".to_string(), "users".to_string()],
        "operator scopes should fold back to legacy short names"
    );

    let ban_scopes = db
        .query_all_raw(Statement::from_string(
            backend,
            "SELECT scopes FROM user_bans WHERE user_id = 1",
        ))
        .await
        .expect("ban scopes should query after rollback");
    let ban_scopes: Vec<String> = ban_scopes
        .into_iter()
        .map(|row| {
            row.try_get_by_index::<String>(0)
                .expect("ban scopes value should decode")
        })
        .collect();
    assert_eq!(
        ban_scopes,
        vec![r#"["texture_upload","yggdrasil_access"]"#.to_string()],
        "ban scope arrays should fold back to legacy short names"
    );
}

async fn assert_operator_scope_column_width(
    db: &DatabaseConnection,
    backend: DbBackend,
    expected_len: i64,
) {
    if backend == DbBackend::Sqlite {
        // SQLite does not enforce VARCHAR lengths.
        return;
    }
    let sql = match backend {
        DbBackend::Postgres => {
            "SELECT character_maximum_length::bigint \
             FROM information_schema.columns \
             WHERE table_schema = 'public' \
               AND table_name = 'user_operator_scopes' \
               AND column_name = 'scope'"
        }
        DbBackend::MySql => {
            "SELECT CAST(CHARACTER_MAXIMUM_LENGTH AS SIGNED) \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = DATABASE() \
               AND TABLE_NAME = 'user_operator_scopes' \
               AND COLUMN_NAME = 'scope'"
        }
        backend => panic!("unsupported backend: {backend:?}"),
    };
    let row = db
        .query_one_raw(Statement::from_string(backend, sql))
        .await
        .expect("user_operator_scopes.scope length should query")
        .expect("user_operator_scopes.scope column should exist");
    let max_len: i64 = row
        .try_get_by_index(0)
        .expect("scope max length should decode");
    assert_eq!(
        max_len, expected_len,
        "operator scope column should match the migration target width"
    );
}

#[actix_web::test]
async fn test_sqlite_permission_id_migration_rewrite() {
    let database_path = format!(
        "/tmp/asteryggdrasil-scope-rewrite-{}.db",
        uuid::Uuid::new_v4()
    );
    let database_url = format!("sqlite://{database_path}");
    exercise_permission_id_migration_rewrite(&database_url, DbBackend::Sqlite).await;
    let _ = tokio::fs::remove_file(database_path).await;
}

#[actix_web::test]
async fn test_postgres_permission_id_migration_rewrite() {
    let database_url = common::empty_postgres_test_database_url().await;
    exercise_permission_id_migration_rewrite(&database_url, DbBackend::Postgres).await;
}

#[actix_web::test]
async fn test_mysql_permission_id_migration_rewrite() {
    let database_url = common::empty_mysql_test_database_url().await;
    exercise_permission_id_migration_rewrite(&database_url, DbBackend::MySql).await;
}
