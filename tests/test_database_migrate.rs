//! Integration tests for the `database_migrate` CLI command (SQLite →
//! SQLite, real binary). Cross-backend (Postgres/MySQL) coverage needs
//! testcontainers and lives outside this file.

use std::path::Path;
use std::process::{Command, Output, Stdio};

use sea_orm::{ConnectionTrait, Database, Statement};

fn aster_yggdrasil_bin() -> &'static str {
    env!("CARGO_BIN_EXE_aster_yggdrasil")
}

fn temp_dir(label: &'static str) -> aster_forge_utils::raii::TempDirGuard {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "aster-yggdrasil-db-migrate-{label}-{nanos}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("data")).expect("temp data dir should create");
    aster_forge_utils::raii::TempDirGuard::new(dir, "database migrate test temp dir")
}

fn sqlite_url(dir: &Path, file: &str) -> String {
    format!("sqlite://{}/data/{file}?mode=rwc", dir.display())
}

/// Builds a fully migrated source database and seeds fixture rows.
async fn prepare_source(dir: &Path, file: &str) -> sea_orm::DatabaseConnection {
    let db = Database::connect(sqlite_url(dir, file))
        .await
        .expect("source database should connect");
    aster_yggdrasil_migration::Migrator::up(&db, None)
        .await
        .expect("source schema should migrate");
    for (username, email) in [("alice", "alice@example.com"), ("bob", "bob@example.com")] {
        db.execute_raw(Statement::from_sql_and_values(
            sea_orm::DbBackend::Sqlite,
            "INSERT INTO users (public_uuid, username, email, password_hash, created_at, \
             updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [
                format!("{:032x}", username.len()).into(),
                username.into(),
                email.into(),
                "argon2-hash-placeholder".into(),
            ],
        ))
        .await
        .expect("fixture user should insert");
    }
    // A child row with a live foreign key: the orphan check must count zero
    // violations for valid references (parent-side join must still match).
    db.execute_raw(Statement::from_sql_and_values(
        sea_orm::DbBackend::Sqlite,
        "INSERT INTO user_profiles (user_id, display_name, created_at, updated_at) VALUES (?, ?, \
         strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [1i64.into(), "alice-display".into()],
    ))
    .await
    .expect("fixture profile should insert");
    db.execute_raw(Statement::from_string(
        sea_orm::DbBackend::Sqlite,
        "INSERT INTO system_config (key, value, category, description, namespace, updated_at) \
         VALUES ('test_key', 'test_value', 'test', 'fixture row', 'test',          strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
    ))
    .await
    .expect("fixture config should insert");
    db
}

fn run_migrate(args: &[&str], envs: &[(&str, &str)]) -> Output {
    run_migrate_verbose(args, envs, false)
}

fn run_migrate_verbose(args: &[&str], envs: &[(&str, &str)], trace: bool) -> Output {
    let mut command = Command::new(aster_yggdrasil_bin());
    command
        .arg("database-migrate")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output().expect("aster_yggdrasil should spawn");
    if trace {
        eprintln!("CHILD-ERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    output
}

async fn scalar(db: &sea_orm::DatabaseConnection, sql: &str) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(sea_orm::DbBackend::Sqlite, sql))
        .await
        .expect("scalar query should run")
        .expect("scalar query should return a row");
    row.try_get_by_index::<i64>(0)
        .expect("scalar value should decode")
}

#[tokio::test]
async fn dry_run_and_apply_copy_all_tables_with_checkpoint() {
    let source_dir = temp_dir("dry-apply-source");
    let target_dir = temp_dir("dry-apply-target");
    let _source_db = prepare_source(source_dir.path(), "source.db").await;

    // Dry run: reports success and leaves the target untouched.
    // The dry-run target starts as an existing empty database file.
    let empty_target = Database::connect(format!(
        "{}?mode=rwc",
        sqlite_url(target_dir.path(), "target.db").trim_end_matches("?mode=rwc")
    ))
    .await
    .expect("empty target file should create");
    drop(empty_target);

    let output = run_migrate(
        &[
            "--source-database-url",
            &sqlite_url(source_dir.path(), "source.db"),
            "--target-database-url",
            &sqlite_url(target_dir.path(), "target.db"),
            "--dry-run",
        ],
        &[],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "dry run should succeed: {stdout}");
    assert!(stdout.contains("\"ok\": true"), "dry run emits envelope");
    assert!(stdout.contains("dry_run"), "dry run reports its mode");
    let target_probe = Database::connect(sqlite_url(target_dir.path(), "target.db"))
        .await
        .expect("target should exist");
    let pending_tables = scalar(
        &target_probe,
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
    )
    .await;
    assert_eq!(
        pending_tables, 0,
        "dry run must not create the target schema"
    );

    // Apply: copies every planned table and writes a checkpoint row.
    let output = run_migrate(
        &[
            "--source-database-url",
            &sqlite_url(source_dir.path(), "source.db"),
            "--target-database-url",
            &sqlite_url(target_dir.path(), "target.db"),
            "--yes",
        ],
        &[],
    );
    let apply_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "apply should succeed");
    assert!(
        apply_stdout.contains("\"ready_to_cutover\": true"),
        "apply should be ready for cutover: {apply_stdout}"
    );

    let target = Database::connect(sqlite_url(target_dir.path(), "target.db"))
        .await
        .expect("target database should connect");
    let users = scalar(&target, "SELECT COUNT(*) FROM users").await;
    assert_eq!(users, 2, "fixture users should copy");
    let checkpoint_rows = scalar(
        &target,
        "SELECT COUNT(*) FROM aster_cli_database_migrations",
    )
    .await;
    assert_eq!(
        checkpoint_rows, 1,
        "apply writes exactly one checkpoint row"
    );
    let stored_url: String = target
        .query_one_raw(Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT source_database_url FROM aster_cli_database_migrations",
        ))
        .await
        .expect("checkpoint query should run")
        .expect("checkpoint row should exist")
        .try_get_by_index(0)
        .expect("checkpoint URL should decode");
    assert!(
        !stored_url.contains("mode=rwc") || stored_url.contains(".../"),
        "checkpoint stores a redacted URL: {stored_url}"
    );
    drop(target_probe);
}

#[tokio::test]
async fn resume_after_forced_failure_completes_the_copy() {
    let source_dir = temp_dir("resume-source");
    let target_dir = temp_dir("resume-target");
    let db = prepare_source(source_dir.path(), "source.db").await;
    // Extra rows so the copy needs several batches at size 1.
    for index in 0..6 {
        db.execute_raw(Statement::from_sql_and_values(
            sea_orm::DbBackend::Sqlite,
            "INSERT INTO users (public_uuid, username, email, password_hash, created_at, \
             updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [
                format!("{:032x}", index + 100).into(),
                format!("user-{index}").into(),
                format!("user-{index}@example.com").into(),
                "argon2-hash-placeholder".into(),
            ],
        ))
        .await
        .expect("fixture user should insert");
    }
    drop(db);

    let source_url = sqlite_url(source_dir.path(), "source.db");
    let target_url = sqlite_url(target_dir.path(), "target.db");
    let failed = run_migrate(
        &[
            "--source-database-url",
            &source_url,
            "--target-database-url",
            &target_url,
            "--yes",
        ],
        &[
            ("ASTER_CLI_COPY_BATCH_SIZE", "1"),
            ("ASTER_CLI_FAIL_AFTER_BATCHES", "1"),
            ("ASTER_CLI_PROGRESS", "1"),
        ],
    );
    assert!(!failed.status.success(), "forced failure must exit nonzero");
    let stdout = String::from_utf8_lossy(&failed.stdout);
    assert!(
        stdout.contains("forced failure after committed batch"),
        "the injected failure message must surface: {stdout}"
    );

    let resumed = run_migrate(
        &[
            "--source-database-url",
            &source_url,
            "--target-database-url",
            &target_url,
            "--yes",
        ],
        &[],
    );
    assert!(
        resumed.status.success(),
        "resume should complete: {}",
        String::from_utf8_lossy(&resumed.stdout)
    );
    let target = Database::connect(target_url.clone())
        .await
        .expect("target should connect");
    let users = scalar(&target, "SELECT COUNT(*) FROM users").await;
    assert_eq!(
        users, 8,
        "resume must neither duplicate nor drop rows (got {users})"
    );
    let checkpoint_status: String = target
        .query_one_raw(Statement::from_string(
            sea_orm::DbBackend::Sqlite,
            "SELECT status FROM aster_cli_database_migrations",
        ))
        .await
        .expect("checkpoint query should run")
        .expect("checkpoint row should exist")
        .try_get_by_index(0)
        .expect("checkpoint status should decode");
    assert_eq!(checkpoint_status, "completed");
}

#[tokio::test]
async fn preflight_rejects_unknown_source_tables() {
    let source_dir = temp_dir("preflight-source");
    let target_dir = temp_dir("preflight-target");
    let db = prepare_source(source_dir.path(), "source.db").await;
    db.execute_raw(Statement::from_string(
        sea_orm::DbBackend::Sqlite,
        "CREATE TABLE product_blog_posts (id INTEGER PRIMARY KEY, title TEXT)",
    ))
    .await
    .expect("unknown table should create");
    drop(db);
    let empty_target = Database::connect(format!(
        "{}?mode=rwc",
        sqlite_url(target_dir.path(), "target.db").trim_end_matches("?mode=rwc")
    ))
    .await
    .expect("empty target file should create");
    drop(empty_target);

    let output = run_migrate(
        &[
            "--source-database-url",
            &sqlite_url(source_dir.path(), "source.db"),
            "--target-database-url",
            &sqlite_url(target_dir.path(), "target.db"),
            "--dry-run",
        ],
        &[],
    );
    assert!(
        !output.status.success(),
        "unknown tables must fail preflight"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("product_blog_posts"),
        "the error must name the offending table: {stdout}"
    );
    assert!(
        stdout.contains("\"ok\": false"),
        "errors emit error envelope"
    );
}

#[path = "common/mod.rs"]
mod common;

mod cross_backend {
    use super::*;
    use chrono::Utc;
    use sea_orm::Value;

    /// Backend-native placeholder (sea-orm does not rewrite `?` for PG).
    fn placeholder(backend: sea_orm::DbBackend, position: usize) -> String {
        match backend {
            sea_orm::DbBackend::Postgres => format!("${position}"),
            _ => "?".to_string(),
        }
    }

    /// Seeds the same fixture on any backend, binding timestamps so the
    /// copy path exercises cross-backend timestamp round-trips.
    async fn seed_on(db: &sea_orm::DatabaseConnection, backend: sea_orm::DbBackend) {
        aster_yggdrasil_migration::Migrator::up(db, None)
            .await
            .expect("schema should migrate");
        for (username, email) in [("alice", "alice@example.com"), ("bob", "bob@example.com")] {
            db.execute_raw(Statement::from_sql_and_values(
                backend,
                format!(
                    "INSERT INTO users (public_uuid, username, email, password_hash, created_at, \
                     updated_at) VALUES ({}, {}, {}, {}, {}, {})",
                    placeholder(backend, 1),
                    placeholder(backend, 2),
                    placeholder(backend, 3),
                    placeholder(backend, 4),
                    placeholder(backend, 5),
                    placeholder(backend, 6)
                ),
                [
                    format!("{:032x}", username.len()).into(),
                    username.into(),
                    email.into(),
                    "argon2-hash-placeholder".into(),
                    Value::ChronoDateTimeUtc(Some(Utc::now())),
                    Value::ChronoDateTimeUtc(Some(Utc::now())),
                ],
            ))
            .await
            .expect("fixture user should insert");
        }
        let (quote_left, quote_right) = match backend {
            sea_orm::DbBackend::MySql => ("`", "`"),
            _ => ("\"", "\""),
        };
        db.execute_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "INSERT INTO system_config ({q}key{r}, {q}value{r}, category, description, \
                 namespace, updated_at) VALUES ({p1}, {p2}, {p3}, {p4}, {p5}, {p6})",
                q = quote_left,
                r = quote_right,
                p1 = placeholder(backend, 1),
                p2 = placeholder(backend, 2),
                p3 = placeholder(backend, 3),
                p4 = placeholder(backend, 4),
                p5 = placeholder(backend, 5),
                p6 = placeholder(backend, 6)
            ),
            [
                "test_key".into(),
                "测试迁移产物 ✓".into(),
                "test".into(),
                "fixture row".into(),
                "test".into(),
                Value::ChronoDateTimeUtc(Some(Utc::now())),
            ],
        ))
        .await
        .expect("fixture config should insert");
    }

    async fn migrate_and_assert(source_url: &str, target_url: &str) {
        let output = run_migrate_verbose(
            &[
                "--source-database-url",
                source_url,
                "--target-database-url",
                target_url,
                "--yes",
            ],
            &[],
            true,
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cross-backend migration should succeed: {stdout}{stderr}"
        );
        assert!(
            stdout.contains("\"ready_to_cutover\": true"),
            "verification should pass: {stdout}"
        );
        assert!(
            stdout.contains("\"unique_conflicts\": []"),
            "no unique conflicts expected: {stdout}"
        );
        assert!(
            stdout.contains("\"foreign_key_violations\": []"),
            "no FK violations expected: {stdout}"
        );
    }

    async fn target_user_count(target_url: &str) -> i64 {
        let target = Database::connect(target_url)
            .await
            .expect("target should connect");
        let row = target
            .query_one_raw(Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                // Backend is inferred per connection; use a portable COUNT.
                "SELECT COUNT(*) FROM users",
            ))
            .await
            .expect("count should run")
            .expect("count row should exist");
        row.try_get_by_index(0).expect("count should decode")
    }

    #[tokio::test]
    async fn sqlite_to_postgres_copies_all_tables() {
        let source_dir = temp_dir("pg-source");
        let db = prepare_source(source_dir.path(), "source.db").await;
        drop(db);
        let target_url = common::empty_postgres_test_database_url().await;
        migrate_and_assert(&sqlite_url(source_dir.path(), "source.db"), &target_url).await;
        assert_eq!(target_user_count(&target_url).await, 2);
    }

    #[tokio::test]
    async fn postgres_to_mysql_copies_all_tables() {
        let source_url = common::empty_postgres_test_database_url().await;
        let source = Database::connect(&source_url)
            .await
            .expect("source should connect");
        seed_on(&source, sea_orm::DbBackend::Postgres).await;
        drop(source);
        let target_url = common::empty_mysql_test_database_url().await;
        migrate_and_assert(&source_url, &target_url).await;
        assert_eq!(target_user_count(&target_url).await, 2);
    }

    #[tokio::test]
    async fn mysql_to_sqlite_copies_all_tables() {
        let source_url = common::empty_mysql_test_database_url().await;
        let source = Database::connect(&source_url)
            .await
            .expect("source should connect");
        seed_on(&source, sea_orm::DbBackend::MySql).await;
        drop(source);
        let target_dir = temp_dir("sqlite-target");
        migrate_and_assert(&source_url, &sqlite_url(target_dir.path(), "target.db")).await;
        let target = Database::connect(sqlite_url(target_dir.path(), "target.db"))
            .await
            .expect("target should connect");
        let users = scalar(&target, "SELECT COUNT(*) FROM users").await;
        assert_eq!(users, 2);
    }
}
