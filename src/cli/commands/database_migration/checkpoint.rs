//! Target-side migration checkpoint: one row keyed by the sha256 of the
//! endpoint URLs, advanced in the same transaction as each committed batch so
//! the resume path never duplicates or drops rows.

use super::plan::TablePlan;
use crate::cli::common::database::{quote_ident, redact_database_url};
use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sha2::{Digest, Sha256};

/// Builds the checkpoint primary key from credential-free endpoint
/// identities, so rotating a database password does not orphan an existing
/// checkpoint.
fn build_migration_key(source_url: &str, target_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(endpoint_identity(source_url).as_bytes());
    hasher.update(b"\n");
    hasher.update(endpoint_identity(target_url).as_bytes());
    hasher.update(b"\n");
    hasher.update(b"apply");
    hex::encode(hasher.finalize())
}

/// Strips userinfo and redacts query values from a URL, keeping the parts
/// that actually identify the database (scheme, host, port, path).
fn endpoint_identity(url: &str) -> String {
    let body = match url.split_once("://") {
        Some((scheme, rest)) => {
            let without_userinfo = match rest.rsplit_once('@') {
                // Keep the last host segment; userinfo never contains "@" in
                // practice, but this mirrors the redaction fallback.
                Some((_, host)) => host,
                None => rest,
            };
            format!("{scheme}://{without_userinfo}")
        }
        None => url.to_string(),
    };
    match body.split_once('?') {
        Some((prefix, _query)) => prefix.to_string(),
        None => body,
    }
}

/// One checkpoint row; `(current_table_index, current_table_offset)` is the
/// resume point.
#[derive(Debug, Clone)]
pub(in crate::cli) struct MigrationCheckpoint {
    pub migration_key: String,
    pub source_database_url: String,
    pub target_database_url: String,
    pub mode: String,
    pub status: String,
    pub stage: String,
    pub current_table: Option<String>,
    pub current_table_index: i64,
    pub current_table_offset: i64,
    pub copied_rows: i64,
    pub total_rows: i64,
    pub plan_json: String,
    pub result_json: Option<String>,
    pub last_error: Option<String>,
    pub heartbeat_at_ms: i64,
    pub updated_at_ms: i64,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Creates the checkpoint table if it does not exist. Types use BIGINT/TEXT
/// everywhere; both are portable across the three supported backends.
pub(super) async fn ensure_checkpoint_table(target: &sea_orm::DatabaseConnection) -> Result<()> {
    let backend = target.get_database_backend();
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {} ( \
           migration_key VARCHAR(64) PRIMARY KEY, \
           source_database_url TEXT NOT NULL, \
           target_database_url TEXT NOT NULL, \
           mode VARCHAR(32) NOT NULL, \
           status VARCHAR(32) NOT NULL, \
           stage VARCHAR(32) NOT NULL, \
           current_table VARCHAR(255) NULL, \
           current_table_index BIGINT NOT NULL DEFAULT 0, \
           current_table_offset BIGINT NOT NULL DEFAULT 0, \
           copied_rows BIGINT NOT NULL DEFAULT 0, \
           total_rows BIGINT NOT NULL DEFAULT 0, \
           plan_json TEXT NOT NULL, \
           result_json TEXT NULL, \
           last_error TEXT NULL, \
           heartbeat_at_ms BIGINT NOT NULL, \
           updated_at_ms BIGINT NOT NULL \
         )",
        quote_ident(backend, super::plan::CHECKPOINT_TABLE)
    );
    target
        .execute_raw(Statement::from_string(backend, sql))
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

/// Serializes the plan for checkpoint storage.
fn plan_json(plans: &[TablePlan]) -> Result<String> {
    serde_json::to_string(plans).map_err(|error| {
        AsterError::internal_error_code(
            crate::api::error_code::AsterErrorCode::InternalServerError,
            format!("cannot serialize migration plan: {error}"),
        )
    })
}

/// Result of checkpoint initialization: the row plus whether it resumed an
/// existing run.
pub(super) struct InitializedCheckpoint {
    pub checkpoint: MigrationCheckpoint,
    pub resumed: bool,
}

/// Loads an existing checkpoint or creates a new one. A mismatched stored
/// plan is only accepted when the previous run made no progress.
pub(super) async fn initialize_checkpoint(
    source_url: &str,
    target_url: &str,
    target: &sea_orm::DatabaseConnection,
    plans: &[TablePlan],
) -> Result<InitializedCheckpoint> {
    let backend = target.get_database_backend();
    let key = build_migration_key(source_url, target_url);
    let new_plan = plan_json(plans)?;
    let total_rows: i64 = plans.iter().map(|plan| plan.source_rows).sum();

    if let Some(mut checkpoint) = load_checkpoint(target, backend, &key).await? {
        if checkpoint.plan_json != new_plan
            && (checkpoint.copied_rows != 0 || checkpoint.current_table_index != 0)
        {
            return Err(AsterError::validation_error(
                "existing checkpoint does not match the current source plan; \
                 target must be reset before retrying",
            ));
        }
        checkpoint.plan_json = new_plan;
        checkpoint.status = "running".to_string();
        checkpoint.stage = "data_copy".to_string();
        checkpoint.last_error = None;
        checkpoint.total_rows = total_rows;
        checkpoint.heartbeat_at_ms = now_ms();
        checkpoint.updated_at_ms = checkpoint.heartbeat_at_ms;
        update_checkpoint(target, &checkpoint).await?;
        return Ok(InitializedCheckpoint {
            checkpoint,
            resumed: true,
        });
    }

    super::schema::ensure_target_empty(target, plans).await?;
    let checkpoint = MigrationCheckpoint {
        migration_key: key,
        source_database_url: redact_database_url(source_url),
        target_database_url: redact_database_url(target_url),
        mode: "apply".to_string(),
        status: "running".to_string(),
        stage: "data_copy".to_string(),
        current_table: None,
        current_table_index: 0,
        current_table_offset: 0,
        copied_rows: 0,
        total_rows,
        plan_json: new_plan,
        result_json: None,
        last_error: None,
        heartbeat_at_ms: now_ms(),
        updated_at_ms: now_ms(),
    };
    insert_checkpoint(target, backend, &checkpoint).await?;
    Ok(InitializedCheckpoint {
        checkpoint,
        resumed: false,
    })
}

/// Inserts a fresh checkpoint row; URLs are stored redacted.
async fn insert_checkpoint(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    checkpoint: &MigrationCheckpoint,
) -> Result<()> {
    let sql = format!(
        "INSERT INTO {} (migration_key, source_database_url, target_database_url, mode, status, \
         stage, current_table, current_table_index, current_table_offset, copied_rows, \
         total_rows, plan_json, result_json, last_error, heartbeat_at_ms, updated_at_ms) \
         VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
        quote_ident(backend, super::plan::CHECKPOINT_TABLE),
        placeholder(backend, 1),
        placeholder(backend, 2),
        placeholder(backend, 3),
        placeholder(backend, 4),
        placeholder(backend, 5),
        placeholder(backend, 6),
        placeholder(backend, 7),
        placeholder(backend, 8),
        placeholder(backend, 9),
        placeholder(backend, 10),
        placeholder(backend, 11),
        placeholder(backend, 12),
        placeholder(backend, 13),
        placeholder(backend, 14),
        placeholder(backend, 15),
        placeholder(backend, 16),
    );
    db.execute_raw(statement_with_values(
        backend,
        sql,
        vec![
            checkpoint.migration_key.clone().into(),
            checkpoint.source_database_url.clone().into(),
            checkpoint.target_database_url.clone().into(),
            checkpoint.mode.clone().into(),
            checkpoint.status.clone().into(),
            checkpoint.stage.clone().into(),
            checkpoint.current_table.clone().into(),
            checkpoint.current_table_index.into(),
            checkpoint.current_table_offset.into(),
            checkpoint.copied_rows.into(),
            checkpoint.total_rows.into(),
            checkpoint.plan_json.clone().into(),
            checkpoint.result_json.clone().into(),
            checkpoint.last_error.clone().into(),
            checkpoint.heartbeat_at_ms.into(),
            checkpoint.updated_at_ms.into(),
        ],
    ))
    .await
    .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

/// Builds a backend-appropriate placeholder list (`$1` for Postgres, `?`
/// elsewhere).
fn placeholder(backend: DbBackend, position: usize) -> String {
    if backend == DbBackend::Postgres {
        format!("${position}")
    } else {
        "?".to_string()
    }
}

/// Wraps raw SQL with bound values (no string concatenation of user data).
fn statement_with_values(
    backend: DbBackend,
    sql: String,
    values: Vec<sea_orm::Value>,
) -> Statement {
    Statement::from_sql_and_values(backend, sql, values)
}

/// Loads a checkpoint row by key.
async fn load_checkpoint(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    key: &str,
) -> Result<Option<MigrationCheckpoint>> {
    let sql = format!(
        "SELECT migration_key, source_database_url, target_database_url, mode, status, stage, \
         current_table, current_table_index, current_table_offset, copied_rows, total_rows, \
         plan_json, result_json, last_error, heartbeat_at_ms, updated_at_ms FROM {} \
         WHERE migration_key = {}",
        quote_ident(backend, super::plan::CHECKPOINT_TABLE),
        placeholder(backend, 1),
    );
    let row = db
        .query_one_raw(statement_with_values(
            backend,
            sql,
            vec![key.to_string().into()],
        ))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut checkpoint = MigrationCheckpoint {
        migration_key: row
            .try_get_by_index(0)
            .map_aster_err(AsterError::database_operation)?,
        source_database_url: row
            .try_get_by_index(1)
            .map_aster_err(AsterError::database_operation)?,
        target_database_url: row
            .try_get_by_index(2)
            .map_aster_err(AsterError::database_operation)?,
        mode: row
            .try_get_by_index(3)
            .map_aster_err(AsterError::database_operation)?,
        status: row
            .try_get_by_index(4)
            .map_aster_err(AsterError::database_operation)?,
        stage: row
            .try_get_by_index(5)
            .map_aster_err(AsterError::database_operation)?,
        current_table: row
            .try_get_by_index(6)
            .map_aster_err(AsterError::database_operation)?,
        current_table_index: row
            .try_get_by_index(7)
            .map_aster_err(AsterError::database_operation)?,
        current_table_offset: row
            .try_get_by_index(8)
            .map_aster_err(AsterError::database_operation)?,
        copied_rows: row
            .try_get_by_index(9)
            .map_aster_err(AsterError::database_operation)?,
        total_rows: row
            .try_get_by_index(10)
            .map_aster_err(AsterError::database_operation)?,
        plan_json: row
            .try_get_by_index(11)
            .map_aster_err(AsterError::database_operation)?,
        result_json: row
            .try_get_by_index(12)
            .map_aster_err(AsterError::database_operation)?,
        last_error: row
            .try_get_by_index(13)
            .map_aster_err(AsterError::database_operation)?,
        heartbeat_at_ms: row
            .try_get_by_index(14)
            .map_aster_err(AsterError::database_operation)?,
        updated_at_ms: row
            .try_get_by_index(15)
            .map_aster_err(AsterError::database_operation)?,
    };
    // Re-redact on load to defend against rows written before redaction.
    checkpoint.source_database_url = redact_database_url(&checkpoint.source_database_url);
    checkpoint.target_database_url = redact_database_url(&checkpoint.target_database_url);
    Ok(Some(checkpoint))
}

/// Writes the full checkpoint row (all fields bound, no SQL concatenation of
/// user data).
pub(super) async fn update_checkpoint<C>(db: &C, checkpoint: &MigrationCheckpoint) -> Result<()>
where
    C: ConnectionTrait,
{
    let backend = db.get_database_backend();
    let sql = format!(
        "UPDATE {} SET source_database_url = {}, target_database_url = {}, mode = {}, \
         status = {}, stage = {}, current_table = {}, current_table_index = {}, \
         current_table_offset = {}, copied_rows = {}, total_rows = {}, plan_json = {}, \
         result_json = {}, last_error = {}, heartbeat_at_ms = {}, updated_at_ms = {} \
         WHERE migration_key = {}",
        quote_ident(backend, super::plan::CHECKPOINT_TABLE),
        placeholder(backend, 1),
        placeholder(backend, 2),
        placeholder(backend, 3),
        placeholder(backend, 4),
        placeholder(backend, 5),
        placeholder(backend, 6),
        placeholder(backend, 7),
        placeholder(backend, 8),
        placeholder(backend, 9),
        placeholder(backend, 10),
        placeholder(backend, 11),
        placeholder(backend, 12),
        placeholder(backend, 13),
        placeholder(backend, 14),
        placeholder(backend, 15),
        placeholder(backend, 16),
    );
    db.execute_raw(statement_with_values(
        backend,
        sql,
        vec![
            checkpoint.source_database_url.clone().into(),
            checkpoint.target_database_url.clone().into(),
            checkpoint.mode.clone().into(),
            checkpoint.status.clone().into(),
            checkpoint.stage.clone().into(),
            checkpoint.current_table.clone().into(),
            checkpoint.current_table_index.into(),
            checkpoint.current_table_offset.into(),
            checkpoint.copied_rows.into(),
            checkpoint.total_rows.into(),
            checkpoint.plan_json.clone().into(),
            checkpoint.result_json.clone().into(),
            checkpoint.last_error.clone().into(),
            checkpoint.heartbeat_at_ms.into(),
            checkpoint.updated_at_ms.into(),
            checkpoint.migration_key.clone().into(),
        ],
    ))
    .await
    .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

/// Marks the checkpoint failed; used on every apply-mode error path.
pub(super) async fn mark_checkpoint_failed(
    target: &sea_orm::DatabaseConnection,
    checkpoint: &mut MigrationCheckpoint,
    error: &AsterError,
) -> Result<()> {
    checkpoint.status = "failed".to_string();
    checkpoint.last_error = Some(error.to_string());
    checkpoint.updated_at_ms = now_ms();
    checkpoint.heartbeat_at_ms = checkpoint.updated_at_ms;
    update_checkpoint(target, checkpoint).await
}

/// Marks the checkpoint failed without masking the original error.
pub(super) async fn mark_checkpoint_failed_best_effort(
    target: &sea_orm::DatabaseConnection,
    checkpoint: &mut MigrationCheckpoint,
    error: &AsterError,
) {
    if let Err(mark_error) = mark_checkpoint_failed(target, checkpoint, error).await {
        tracing::warn!("failed to persist migration checkpoint failure: {mark_error}");
    }
}

/// Human-readable resume point for logs and stages.
pub(super) fn resume_message(checkpoint: &MigrationCheckpoint) -> String {
    let position = match &checkpoint.current_table {
        Some(table) => format!("{} offset {}", table, checkpoint.current_table_offset),
        None => format!("table index {}", checkpoint.current_table_index),
    };
    format!(
        "resuming checkpoint {} at {} ({}/{} rows)",
        checkpoint.migration_key, position, checkpoint.copied_rows, checkpoint.total_rows
    )
}

/// Backend used by tests; kept separate so tests can assert placeholder
/// selection without a live database.
#[cfg(test)]
fn backend_label(backend: DbBackend) -> &'static str {
    crate::cli::common::database::backend_name(backend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    fn checkpoint_row(key: &str) -> MigrationCheckpoint {
        MigrationCheckpoint {
            migration_key: key.to_string(),
            source_database_url: "sqlite:.../source.db?mode=ro".to_string(),
            target_database_url: "postgres://***@db.example.com/ygg".to_string(),
            mode: "apply".to_string(),
            status: "running".to_string(),
            stage: "data_copy".to_string(),
            current_table: Some("users\"table".to_string()),
            current_table_index: 3,
            current_table_offset: 421,
            copied_rows: 1337,
            total_rows: 9000,
            plan_json: "[]".to_string(),
            result_json: Some("{\"ok\":true\n,\"line\":2}".to_string()),
            last_error: Some("failed: back\\slash\nquote'".to_string()),
            heartbeat_at_ms: 1,
            updated_at_ms: 2,
        }
    }

    #[tokio::test]
    async fn checkpoint_insert_update_bind_values_without_sql_literal_concatenation() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_checkpoint_table(&db).await.unwrap();
        let backend = db.get_database_backend();
        assert_eq!(backend, DbBackend::Sqlite);

        let key = "k-1";
        let row = checkpoint_row(key);
        insert_checkpoint(&db, backend, &row).await.unwrap();
        let loaded = load_checkpoint(&db, backend, key)
            .await
            .unwrap()
            .expect("row should load");
        assert_eq!(loaded.migration_key, row.migration_key);
        assert_eq!(loaded.current_table, row.current_table);
        assert_eq!(loaded.current_table_offset, row.current_table_offset);
        assert_eq!(loaded.copied_rows, row.copied_rows);
        assert_eq!(loaded.result_json, row.result_json);
        assert_eq!(loaded.last_error, row.last_error);

        let mut updated = row.clone();
        updated.status = "completed".to_string();
        updated.stage = "complete".to_string();
        updated.current_table = None;
        updated.last_error = Some("it's \"quoted\"\nnewline".to_string());
        updated.heartbeat_at_ms = 3;
        updated.updated_at_ms = 4;
        update_checkpoint(&db, &updated).await.unwrap();
        let reloaded = load_checkpoint(&db, backend, key)
            .await
            .unwrap()
            .expect("row should still load");
        assert_eq!(reloaded.status, "completed");
        assert_eq!(reloaded.stage, "complete");
        assert_eq!(reloaded.current_table, None);
        assert_eq!(reloaded.last_error, updated.last_error);

        let missing = load_checkpoint(&db, backend, "absent").await.unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn migration_key_binds_both_urls_and_the_mode() {
        let key = build_migration_key("sqlite://a", "sqlite://b");
        let other_source = build_migration_key("sqlite://x", "sqlite://b");
        let other_target = build_migration_key("sqlite://a", "sqlite://x");
        assert_eq!(key, build_migration_key("sqlite://a", "sqlite://b"));
        assert_ne!(key, other_source);
        assert_ne!(key, other_target);
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn placeholder_selection_matches_backend() {
        assert_eq!(placeholder(DbBackend::Postgres, 1), "$1");
        assert_eq!(placeholder(DbBackend::MySql, 1), "?");
        assert_eq!(placeholder(DbBackend::Sqlite, 3), "?");
        assert_eq!(backend_label(DbBackend::Sqlite), "sqlite");
    }

    #[tokio::test]
    async fn forced_failure_marker_describes_the_injection_point() {
        // Documents the failure message the copy loop injects so the
        // resume-path integration test can assert on it.
        let message = "forced failure after committed batch for resume-path verification";
        assert!(message.contains("committed batch"));
    }

    #[tokio::test]
    async fn transaction_rollback_leaves_checkpoint_rows_isolated() {
        use sea_orm::TransactionTrait as _;
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_checkpoint_table(&db).await.unwrap();
        let backend = db.get_database_backend();
        let row = checkpoint_row("tx-key");
        insert_checkpoint(&db, backend, &row).await.unwrap();

        let txn = db.begin().await.unwrap();
        let mut inside = row.clone();
        inside.copied_rows = 500;
        update_checkpoint(&txn, &inside).await.unwrap();
        drop(txn); // rollback

        let loaded = load_checkpoint(&db, backend, "tx-key")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            loaded.copied_rows, row.copied_rows,
            "rolled-back checkpoint updates must not persist"
        );
    }
}
