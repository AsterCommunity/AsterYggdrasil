//! Fixed copy order, source-side preflight validation, and test hooks.

use crate::cli::common::database::{MIGRATION_TABLE, quote_ident, scalar_i64};
use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend};

/// The table copy order: parents before children per the Yggdrasil foreign
/// key graph. The migration checkpoint table and the SeaORM migration
/// history are handled outside this list.
pub(in crate::cli) const COPY_TABLE_ORDER: &[&str] = &[
    "users",
    "user_profiles",
    "auth_sessions",
    "passkeys",
    "external_auth_providers",
    "external_auth_identities",
    "external_auth_login_flows",
    "external_auth_email_verification_flows",
    "contact_verification_tokens",
    "user_invitations",
    "user_operator_scopes",
    "user_bans",
    "user_ban_events",
    "minecraft_profiles",
    "yggdrasil_tokens",
    "minecraft_textures",
    "minecraft_profile_textures",
    "minecraft_texture_tags",
    "minecraft_texture_tag_bindings",
    "minecraft_texture_reports",
    "yggdrasil_session_forward_servers",
    "system_config",
    "audit_logs",
    "mail_outbox",
    "background_tasks",
    "runtime_leases",
    "scheduled_tasks",
];

/// The target-side checkpoint table; created outside the SeaORM migrations.
pub(in crate::cli) const CHECKPOINT_TABLE: &str = "aster_cli_database_migrations";

/// Rows per INSERT batch unless overridden for testing.
pub(in crate::cli) const DEFAULT_COPY_BATCH_SIZE: usize = 200;

/// Forces stderr progress output even without a TTY.
pub(in crate::cli) const PROGRESS_ENV: &str = "ASTER_CLI_PROGRESS";

/// Overrides the copy batch size for tests.
pub(in crate::cli) const COPY_BATCH_SIZE_ENV: &str = "ASTER_CLI_COPY_BATCH_SIZE";

/// Injects a committed-batch failure so the resume path can be exercised.
pub(in crate::cli) const FAIL_AFTER_BATCHES_ENV: &str = "ASTER_CLI_FAIL_AFTER_BATCHES";

/// One column of a source table as discovered from the source catalog.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(in crate::cli) struct ColumnSchema {
    pub name: String,
    pub raw_type: String,
    pub pk_order: i32,
    pub binding_kind: super::apply::convert::BindingKind,
}

/// Everything the copy and verify stages need to know about one table.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(in crate::cli) struct TablePlan {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub primary_key: Vec<String>,
    pub source_rows: i64,
    pub sequence_reset: bool,
}

/// Loads plans for every table in [`COPY_TABLE_ORDER`] from the source
/// database, after validating that the source contains only known tables.
pub(super) async fn load_source_plans(
    source: &sea_orm::DatabaseConnection,
) -> Result<Vec<TablePlan>> {
    let backend = source.get_database_backend();
    validate_source_tables(source, backend).await?;

    let mut plans = Vec::with_capacity(COPY_TABLE_ORDER.len());
    for table in COPY_TABLE_ORDER {
        plans.push(load_table_plan(source, backend, table).await?);
    }
    Ok(plans)
}

/// Loads one table's column/PK schema and row count from the source catalog.
pub(super) async fn load_table_plan(
    db: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    table: &str,
) -> Result<TablePlan> {
    let columns = super::schema::load_column_schemas(db, backend, table).await?;
    if columns.is_empty() {
        return Err(AsterError::database_operation(format!(
            "table '{table}' has no readable columns"
        )));
    }
    let mut primary_key: Vec<(i32, String)> = columns
        .iter()
        .filter(|column| column.pk_order > 0)
        .map(|column| (column.pk_order, column.name.clone()))
        .collect();
    primary_key.sort();
    let primary_key: Vec<String> = primary_key.into_iter().map(|(_, name)| name).collect();
    if primary_key.is_empty() {
        // Offset pagination over a table with no stable ordering can skip or
        // duplicate rows across batches; refuse rather than corrupt silently.
        return Err(AsterError::validation_error(format!(
            "table '{table}' has no primary key; \
             the migration cannot order deterministic batches over it"
        )));
    }
    let source_rows = scalar_i64(
        db,
        backend,
        &format!("SELECT COUNT(*) FROM {}", quote_ident(backend, table)),
    )
    .await?;
    let sequence_reset = primary_key.len() == 1
        && primary_key.first().is_some_and(|name| name == "id")
        && matches!(
            columns
                .iter()
                .find(|column| column.name == "id")
                .map(|column| column.binding_kind),
            Some(
                super::apply::convert::BindingKind::Int32
                    | super::apply::convert::BindingKind::Int64
            )
        );
    Ok(TablePlan {
        name: table.to_string(),
        columns,
        primary_key,
        source_rows,
        sequence_reset,
    })
}

/// Rejects source databases containing tables the migration does not know
/// about, so nothing is silently dropped.
async fn validate_source_tables(
    source: &sea_orm::DatabaseConnection,
    backend: DbBackend,
) -> Result<()> {
    let tables = super::schema::source_table_names(source, backend).await?;
    let mut known: std::collections::BTreeSet<&str> = COPY_TABLE_ORDER.iter().copied().collect();
    known.insert(MIGRATION_TABLE);
    known.insert(CHECKPOINT_TABLE);
    // The SQLite catalog query already hides `sqlite_*` internals; tables on
    // PostgreSQL and MySQL are always subject to the unknown-table check.
    let unknown: Vec<&str> = tables
        .iter()
        .map(|table| table.as_str())
        .filter(|table| !known.contains(table))
        .collect();
    if !unknown.is_empty() {
        return Err(AsterError::validation_error(format!(
            "source database contains unsupported tables that would not be migrated: {}",
            unknown.join(", ")
        )));
    }
    for table in COPY_TABLE_ORDER {
        if !tables.iter().any(|existing| existing == table) {
            return Err(AsterError::validation_error(format!(
                "source database is missing expected table '{table}'"
            )));
        }
    }
    Ok(())
}

/// Initializes the per-table report list from the plans, all counters at zero.
pub(super) fn plans_to_reports(plans: &[TablePlan]) -> Vec<super::model::TableReport> {
    plans
        .iter()
        .map(|plan| super::model::TableReport {
            name: plan.name.clone(),
            primary_key: plan.primary_key.clone(),
            source_rows: plan.source_rows,
            target_rows: None,
            copied_rows: 0,
            sequence_reset: plan.sequence_reset,
        })
        .collect()
}

/// Sum of all planned source rows.
pub(super) fn total_source_rows(plans: &[TablePlan]) -> i64 {
    plans.iter().map(|plan| plan.source_rows).sum()
}

/// Resolves the effective copy batch size from the environment.
pub(super) fn copy_batch_size() -> Result<usize> {
    let Some(raw) = std::env::var_os(COPY_BATCH_SIZE_ENV) else {
        return Ok(DEFAULT_COPY_BATCH_SIZE);
    };
    let raw = raw
        .to_str()
        .ok_or_else(|| AsterError::validation_error("batch size override must be valid UTF-8"))?;
    let size: usize = raw
        .parse()
        .map_err(|error| AsterError::validation_error(format!("invalid batch size: {error}")))?;
    if size == 0 {
        return Err(AsterError::validation_error(
            "batch size override must be a positive integer",
        ));
    }
    Ok(size)
}

/// Resolves the forced-failure batch count for resume-path tests.
pub(super) fn fail_after_batches() -> Result<Option<u64>> {
    let Some(raw) = std::env::var_os(FAIL_AFTER_BATCHES_ENV) else {
        return Ok(None);
    };
    let raw = raw.to_str().ok_or_else(|| {
        AsterError::validation_error("failure injection override must be valid UTF-8")
    })?;
    let count: u64 = raw
        .parse()
        .map_err(|error| AsterError::validation_error(format!("invalid batch count: {error}")))?;
    if count == 0 {
        return Err(AsterError::validation_error(
            "failure injection override must be a positive integer",
        ));
    }
    Ok(Some(count))
}

/// Counts rows of a table on the fly (used by legacy-style preflight checks).
pub(super) async fn table_row_count<C>(db: &C, backend: DbBackend, table: &str) -> Result<i64>
where
    C: ConnectionTrait,
{
    scalar_i64(
        db,
        backend,
        &format!("SELECT COUNT(*) FROM {}", quote_ident(backend, table)),
    )
    .await
    .map_aster_err(AsterError::database_operation)
}
