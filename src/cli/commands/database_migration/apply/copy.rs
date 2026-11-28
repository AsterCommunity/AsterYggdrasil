//! Batched table copy with per-batch transactional checkpointing.

use super::super::checkpoint::MigrationCheckpoint;
use super::super::model::StageReport;
use super::super::plan::TablePlan;
use super::convert::decode_row_values;
use crate::cli::common::database::{backend_name, quote_ident};
use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use std::collections::BTreeMap;

/// Progress sink for the copy loop; renders to stderr.
pub(super) struct ProgressReporter {
    enabled: bool,
    total_rows: i64,
    copied_rows: i64,
    label: String,
}

impl ProgressReporter {
    pub(super) fn new(total_rows: i64, label: String, already_copied: i64) -> Self {
        let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
        let forced = std::env::var_os(super::super::plan::PROGRESS_ENV).is_some();
        Self {
            enabled: tty || forced,
            total_rows,
            copied_rows: already_copied,
            label,
        }
    }

    pub(super) fn batch(&mut self, rows: i64) {
        self.copied_rows += rows;
        if !self.enabled || self.total_rows == 0 {
            return;
        }
        let percent = (self.copied_rows * 100 / self.total_rows).min(100);
        eprintln!(
            "[{}] data_copy: {percent:3}% ({}/{})",
            self.label, self.copied_rows, self.total_rows
        );
    }
}

/// Copies every planned table in order, resuming from the checkpoint. Each
/// committed batch advances the checkpoint inside the same transaction, so
/// the resume point is always exact.
pub(super) async fn copy_tables_with_resume(
    source: &sea_orm::DatabaseConnection,
    target: &sea_orm::DatabaseConnection,
    plans: &[TablePlan],
    target_type_hints: &BTreeMap<String, BTreeMap<String, super::convert::BindingKind>>,
    checkpoint: &mut MigrationCheckpoint,
    progress: &mut ProgressReporter,
) -> Result<()> {
    let source_backend = source.get_database_backend();
    let batch_size = super::super::plan::copy_batch_size()?;
    // One multi-row INSERT binds every cell of every row; Postgres caps a
    // statement at 65,535 parameters, so the effective batch shrinks by the
    // widest planned row.
    let max_row_columns = plans
        .iter()
        .map(|plan| plan.columns.len().max(1))
        .max()
        .unwrap_or(1);
    let batch_size = batch_size.min(65_535 / max_row_columns).max(1);
    let fail_after_batches = super::super::plan::fail_after_batches()?;
    let start_table_index = aster_forge_utils::numbers::i64_to_usize(
        checkpoint.current_table_index.clamp(0, plans.len() as i64),
        "checkpoint current_table_index",
    )?;

    for (table_index, plan) in plans.iter().enumerate().skip(start_table_index) {
        let offset = if table_index == start_table_index {
            checkpoint.current_table_offset.clamp(0, plan.source_rows)
        } else {
            0
        };
        checkpoint.stage = "data_copy".to_string();
        checkpoint.status = "running".to_string();
        checkpoint.current_table = Some(plan.name.clone());
        checkpoint.current_table_index = table_index as i64;
        checkpoint.current_table_offset = offset;
        checkpoint.heartbeat_at_ms = chrono::Utc::now().timestamp_millis();
        checkpoint.updated_at_ms = checkpoint.heartbeat_at_ms;
        super::super::checkpoint::update_checkpoint(target, checkpoint).await?;

        let mut committed_batches: u64 = 0;
        let mut offset = offset;
        while offset < plan.source_rows {
            let rows = fetch_source_batch(
                source,
                source_backend,
                plan,
                target_type_hints,
                offset,
                batch_size,
            )
            .await?;
            if rows.is_empty() {
                break;
            }
            let txn = target
                .begin()
                .await
                .map_aster_err(AsterError::database_operation)?;
            insert_batch(&txn, plan, &rows).await?;
            // The in-transaction checkpoint must advance from the committed
            // values, not from the in-memory ones: if the commit itself
            // fails, the in-memory copy must stay at the last committed
            // position so the failure handler and any retry resume from
            // what the database actually holds.
            let mut committed_view = checkpoint.clone();
            committed_view.current_table_offset += rows.len() as i64;
            committed_view.copied_rows += rows.len() as i64;
            committed_view.heartbeat_at_ms = chrono::Utc::now().timestamp_millis();
            committed_view.updated_at_ms = committed_view.heartbeat_at_ms;
            super::super::checkpoint::update_checkpoint(&txn, &committed_view).await?;
            let commit_result = txn.commit().await;
            match commit_result {
                Ok(()) => {
                    // Commit acknowledged: the in-memory view now matches
                    // the persisted row exactly.
                    *checkpoint = committed_view;
                }
                Err(error) => {
                    // Commit outcome unknown: the batch may have persisted.
                    // Do not write the stale in-memory offsets back; the
                    // failure handler only marks the status, and a retry
                    // reloads the checkpoint from the target database.
                    return Err(AsterError::database_operation(format!(
                        "checkpoint commit outcome unknown after batch at offset {}: {error}",
                        checkpoint.current_table_offset
                    )));
                }
            }
            progress.batch(rows.len() as i64);
            committed_batches += 1;
            if let Some(limit) = fail_after_batches
                && committed_batches >= limit
            {
                return Err(AsterError::internal_error_code(
                    crate::api::error_code::AsterErrorCode::InternalServerError,
                    "forced failure after committed batch for resume-path verification",
                ));
            }
            offset = checkpoint.current_table_offset;
        }

        checkpoint.current_table = None;
        checkpoint.current_table_index = (table_index + 1) as i64;
        checkpoint.current_table_offset = 0;
        checkpoint.heartbeat_at_ms = chrono::Utc::now().timestamp_millis();
        checkpoint.updated_at_ms = checkpoint.heartbeat_at_ms;
        super::super::checkpoint::update_checkpoint(target, checkpoint).await?;
    }
    Ok(())
}

/// Reads one batch of rows from the source in target-shaped form.
async fn fetch_source_batch(
    source: &sea_orm::DatabaseConnection,
    source_backend: DbBackend,
    plan: &TablePlan,
    target_type_hints: &BTreeMap<String, BTreeMap<String, super::convert::BindingKind>>,
    offset: i64,
    limit: usize,
) -> Result<Vec<Vec<sea_orm::Value>>> {
    let columns = plan
        .columns
        .iter()
        .map(|column| quote_ident(source_backend, &column.name))
        .collect::<Vec<_>>()
        .join(", ");
    let table = quote_ident(source_backend, &plan.name);
    let order_by = if plan.primary_key.is_empty() {
        String::new()
    } else {
        format!(
            " ORDER BY {}",
            plan.primary_key
                .iter()
                .map(|key| quote_ident(source_backend, key))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let sql = format!("SELECT {columns} FROM {table}{order_by} LIMIT {limit} OFFSET {offset}");
    let rows = source
        .query_all_raw(Statement::from_string(source_backend, sql))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let hints = target_type_hints
        .get(&plan.name)
        .cloned()
        .unwrap_or_default();
    let mut decoded = Vec::with_capacity(rows.len());
    for row in &rows {
        decoded.push(decode_row_values(row, plan, &hints)?);
    }
    Ok(decoded)
}

/// Inserts one batch with a single multi-row parameterized statement.
async fn insert_batch<C>(target: &C, plan: &TablePlan, rows: &[Vec<sea_orm::Value>]) -> Result<()>
where
    C: ConnectionTrait,
{
    use sea_orm::sea_query::{
        Alias, Expr, MysqlQueryBuilder, PostgresQueryBuilder, Query, SqliteQueryBuilder,
    };
    let backend = target.get_database_backend();
    let columns: Vec<Alias> = plan
        .columns
        .iter()
        .map(|column| Alias::new(column.name.as_str()))
        .collect();
    let mut statement = Query::insert()
        .into_table(Alias::new(plan.name.as_str()))
        .columns(columns)
        .to_owned();
    for row in rows {
        let values = row
            .iter()
            .map(|value| Expr::Value(value.clone()))
            .collect::<Vec<_>>();
        statement.values_panic(values);
    }
    let (sql, values) = match backend {
        DbBackend::MySql => statement.build(MysqlQueryBuilder),
        DbBackend::Postgres => statement.build(PostgresQueryBuilder),
        DbBackend::Sqlite => statement.build(SqliteQueryBuilder),
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                crate::cli::common::database::backend_name(other)
            )));
        }
    };
    target
        .execute_raw(Statement::from_sql_and_values(backend, sql, values))
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

/// Resets autoincrement sequences for integer `id` primary keys so post-copy
/// inserts do not collide with copied rows.
pub(super) async fn reset_sequences(
    target: &sea_orm::DatabaseConnection,
    plans: &[TablePlan],
) -> Result<()> {
    let backend = target.get_database_backend();
    for plan in plans.iter().filter(|plan| plan.sequence_reset) {
        let table = quote_ident(backend, &plan.name);
        match backend {
            DbBackend::Postgres => {
                let sql = format!(
                    "SELECT setval(pg_get_serial_sequence({}, 'id'), \
                     COALESCE((SELECT MAX(id) FROM {table}), 1), \
                     EXISTS(SELECT 1 FROM {table}))",
                    quote_literal(&format!("\"{}\"", plan.name))
                );
                target
                    .execute_raw(Statement::from_string(backend, sql))
                    .await
                    .map_aster_err(AsterError::database_operation)?;
            }
            DbBackend::MySql => {
                let next = crate::cli::common::database::scalar_i64(
                    target,
                    backend,
                    &format!("SELECT COALESCE(MAX(id), 0) + 1 FROM {table}"),
                )
                .await?
                .max(1);
                let sql = format!("ALTER TABLE {table} AUTO_INCREMENT = {next}");
                target
                    .execute_raw(Statement::from_string(backend, sql))
                    .await
                    .map_aster_err(AsterError::database_operation)?;
            }
            DbBackend::Sqlite => {}
            other => {
                return Err(AsterError::validation_error(format!(
                    "unsupported backend: {}",
                    backend_name(other)
                )));
            }
        }
    }
    Ok(())
}

/// Stage report for a completed or interrupted copy phase.
pub(super) fn data_copy_stage(message: String, status: &str) -> StageReport {
    StageReport {
        name: "data_copy".to_string(),
        status: status.to_string(),
        message,
    }
}

fn quote_literal(value: &str) -> String {
    crate::cli::common::database::quote_literal(value)
}
