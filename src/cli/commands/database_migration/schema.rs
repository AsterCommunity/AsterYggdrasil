//! Backend-specific catalog discovery: source tables, columns, primary keys,
//! target type hints, and backend validation.

use super::apply::convert::BindingKind;
use super::plan::{ColumnSchema, TablePlan};
use crate::cli::common::database::backend_name;
use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

/// Validates that both endpoints are supported backends.
pub(super) fn validate_backends(source: DbBackend, target: DbBackend) -> Result<()> {
    for (label, backend) in [("source", source), ("target", target)] {
        if !matches!(
            backend,
            DbBackend::Sqlite | DbBackend::Postgres | DbBackend::MySql
        ) {
            return Err(AsterError::validation_error(format!(
                "unsupported {label} database backend: {}",
                backend_name(backend)
            )));
        }
    }
    Ok(())
}

/// Lists the tables visible in the source database.
pub(super) async fn source_table_names<C>(db: &C, backend: DbBackend) -> Result<Vec<String>>
where
    C: ConnectionTrait,
{
    let sql = match backend {
        DbBackend::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
        }
        DbBackend::Postgres => {
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        }
        DbBackend::MySql => {
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'"
        }
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                backend_name(other)
            )));
        }
    };
    let rows = db
        .query_all_raw(Statement::from_string(backend, sql))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row
            .try_get_by_index(0)
            .map_aster_err(AsterError::database_operation)?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// One raw catalog column before binding-kind normalization.
struct RawColumn {
    name: String,
    raw_type: String,
    pk_order: i32,
}

/// Loads column schemas with normalized binding kinds and PK order.
pub(super) async fn load_column_schemas<C>(
    db: &C,
    backend: DbBackend,
    table: &str,
) -> Result<Vec<ColumnSchema>>
where
    C: ConnectionTrait,
{
    let raw = match backend {
        DbBackend::Sqlite => sqlite_columns(db, table).await?,
        DbBackend::Postgres => postgres_columns(db, table).await?,
        DbBackend::MySql => mysql_columns(db, table).await?,
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                backend_name(other)
            )));
        }
    };
    Ok(raw
        .into_iter()
        .map(|column| ColumnSchema {
            binding_kind: binding_kind_from_raw_type(backend, &column.raw_type),
            name: column.name,
            raw_type: column.raw_type,
            pk_order: column.pk_order,
        })
        .collect())
}

/// SQLite column discovery via `PRAGMA table_info`.
async fn sqlite_columns<C>(db: &C, table: &str) -> Result<Vec<RawColumn>>
where
    C: ConnectionTrait,
{
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info({table})"),
        ))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut columns = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row
            .try_get_by_index(1)
            .map_aster_err(AsterError::database_operation)?;
        let raw_type: String = row
            .try_get_by_index(2)
            .map_aster_err(AsterError::database_operation)?;
        let pk_order: i32 = row
            .try_get_by_index(5)
            .map_aster_err(AsterError::database_operation)?;
        columns.push(RawColumn {
            name,
            raw_type,
            pk_order,
        });
    }
    Ok(columns)
}

/// PostgreSQL column discovery via the information schema and PK constraints.
async fn postgres_columns<C>(db: &C, table: &str) -> Result<Vec<RawColumn>>
where
    C: ConnectionTrait,
{
    let literal = crate::cli::common::database::quote_literal(table);
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            format!(
                "SELECT c.column_name, c.udt_name, \
                 COALESCE(pk.ordinal_position, 0) AS pk_order \
                 FROM information_schema.columns c \
                 LEFT JOIN ( \
                   SELECT kcu.column_name, kcu.ordinal_position \
                   FROM information_schema.table_constraints tc \
                   JOIN information_schema.key_column_usage kcu \
                     ON tc.constraint_name = kcu.constraint_name \
                    AND tc.table_schema = kcu.table_schema \
                   WHERE tc.table_name = {literal} AND tc.constraint_type = 'PRIMARY KEY' \
                 ) pk ON pk.column_name = c.column_name \
                 WHERE c.table_name = {literal} AND c.table_schema = 'public' \
                 ORDER BY c.ordinal_position"
            ),
        ))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut columns = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row
            .try_get_by_index(0)
            .map_aster_err(AsterError::database_operation)?;
        let raw_type: String = row
            .try_get_by_index(1)
            .map_aster_err(AsterError::database_operation)?;
        let pk_order: i32 = row
            .try_get_by_index(2)
            .map_aster_err(AsterError::database_operation)?;
        columns.push(RawColumn {
            name,
            raw_type,
            pk_order,
        });
    }
    Ok(columns)
}

/// MySQL column discovery; database-generated columns are dropped from the
/// plan (their values are computed by the target anyway). MariaDB reports a
/// SQL NULL generation expression for ordinary columns and is accepted.
async fn mysql_columns<C>(db: &C, table: &str) -> Result<Vec<RawColumn>>
where
    C: ConnectionTrait,
{
    let literal = crate::cli::common::database::quote_literal(table);
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::MySql,
            format!(
                "SELECT c.column_name, c.column_type, \
                 COALESCE(pk.ordinal_position, 0) AS pk_order, c.generation_expression \
                 FROM information_schema.columns c \
                 LEFT JOIN ( \
                   SELECT kcu.column_name, kcu.ordinal_position \
                   FROM information_schema.table_constraints tc \
                   JOIN information_schema.key_column_usage kcu \
                     ON tc.constraint_name = kcu.constraint_name \
                    AND tc.table_schema = kcu.table_schema \
                    AND tc.table_name = kcu.table_name \
                   WHERE tc.table_name = {literal} \
                     AND tc.table_schema = DATABASE() \
                     AND tc.constraint_type = 'PRIMARY KEY' \
                 ) pk ON pk.column_name = c.column_name \
                 WHERE c.table_name = {literal} AND c.table_schema = DATABASE() \
                 ORDER BY c.ordinal_position"
            ),
        ))
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut columns = Vec::with_capacity(rows.len());
    for row in rows {
        let column_context = |step: &'static str, error: sea_orm::DbErr| {
            AsterError::database_operation(format!("mysql columns of '{table}' at {step}: {error}"))
        };
        let name = row
            .try_get_by_index::<String>(0)
            .map_err(|error| column_context("name", error))?;
        let raw_type = row
            .try_get_by_index::<String>(1)
            .map_err(|error| column_context("type", error))?;
        let pk_order = row
            .try_get_by_index::<i32>(2)
            .map_err(|error| column_context("pk_order", error))?;
        let generation_expression = row
            .try_get_by_index::<Option<String>>(3)
            .map_err(|error| column_context("generation_expression", error))?;
        if generation_expression
            .as_deref()
            .is_some_and(|expression| !expression.trim().is_empty())
        {
            continue;
        }
        columns.push(RawColumn {
            name,
            raw_type,
            pk_order,
        });
    }
    Ok(columns)
}

/// Normalizes a backend-specific raw type into a binding kind.
pub(super) fn binding_kind_from_raw_type(backend: DbBackend, raw_type: &str) -> BindingKind {
    let lower = raw_type.to_ascii_lowercase();
    match backend {
        DbBackend::Postgres => {
            if lower == "bool" {
                BindingKind::Bool
            } else if lower == "int8" {
                BindingKind::Int64
            } else if lower == "int2" || lower == "int4" {
                BindingKind::Int32
            } else if matches!(lower.as_str(), "float4" | "float8" | "numeric") {
                BindingKind::Float64
            } else if lower == "bytea" {
                BindingKind::Bytes
            } else if lower == "json" || lower == "jsonb" {
                BindingKind::Json
            } else if lower.contains("timestamp") {
                BindingKind::Timestamp
            } else {
                BindingKind::String
            }
        }
        DbBackend::MySql => {
            if lower.starts_with("tinyint(1)") || lower.contains("boolean") {
                BindingKind::Bool
            } else if lower.starts_with("bigint") {
                BindingKind::Int64
            } else if lower.contains("int") {
                BindingKind::Int32
            } else if lower.contains("double")
                || lower.contains("float")
                || lower.contains("decimal")
            {
                BindingKind::Float64
            } else if lower.contains("blob") || lower.contains("binary") {
                BindingKind::Bytes
            } else if lower == "json" {
                BindingKind::Json
            } else if lower.contains("timestamp") || lower.contains("datetime") {
                BindingKind::Timestamp
            } else {
                BindingKind::String
            }
        }
        DbBackend::Sqlite => {
            if lower.contains("bool") {
                BindingKind::Bool
            } else if lower.contains("json") {
                BindingKind::Json
            } else if lower.contains("timestamp") || lower.contains("datetime") {
                BindingKind::Timestamp
            } else if lower.contains("blob") {
                BindingKind::Bytes
            } else if lower.contains("double")
                || lower.contains("float")
                || lower.contains("real")
                || lower.contains("decimal")
            {
                BindingKind::Float64
            } else if lower.contains("int") {
                BindingKind::Int64
            } else {
                BindingKind::String
            }
        }
        _ => BindingKind::String,
    }
}

/// Loads the target column types for every planned table and validates that
/// each planned column exists on the target.
pub(super) async fn load_target_type_hints(
    target: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    plans: &[TablePlan],
) -> Result<std::collections::BTreeMap<String, std::collections::BTreeMap<String, BindingKind>>> {
    let mut hints = std::collections::BTreeMap::new();
    for plan in plans {
        let columns = load_column_schemas(target, backend, &plan.name).await?;
        let mut by_name = std::collections::BTreeMap::new();
        for column in columns {
            by_name.insert(column.name, column.binding_kind);
        }
        for column in &plan.columns {
            if !by_name.contains_key(&column.name) {
                return Err(AsterError::validation_error(format!(
                    "target table '{}' is missing column '{}' present in the source",
                    plan.name, column.name
                )));
            }
        }
        hints.insert(plan.name.clone(), by_name);
    }
    Ok(hints)
}

/// Fills in target row counts for the report.
pub(super) async fn refresh_target_rows(
    target: &sea_orm::DatabaseConnection,
    reports: &mut [super::model::TableReport],
) -> Result<()> {
    let backend = target.get_database_backend();
    for report in reports {
        report.target_rows =
            Some(super::plan::table_row_count(target, backend, &report.name).await?);
    }
    Ok(())
}

/// Rejects apply runs whose target already holds rows in planned tables.
pub(super) async fn ensure_target_empty(
    target: &sea_orm::DatabaseConnection,
    plans: &[TablePlan],
) -> Result<()> {
    let backend = target.get_database_backend();
    let mut occupied = Vec::new();
    for plan in plans {
        let rows = super::plan::table_row_count(target, backend, &plan.name).await?;
        if rows > 0 {
            occupied.push(format!("{}={rows}", plan.name));
        }
    }
    if !occupied.is_empty() {
        return Err(AsterError::validation_error(format!(
            "target database already contains rows in planned tables: {}; \
             reset the target before copying",
            occupied.join(", ")
        )));
    }
    Ok(())
}
