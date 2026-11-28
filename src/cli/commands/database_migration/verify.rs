//! Post-copy verification: row counts, unique constraints, and foreign keys.
//! Problems land in the report (`ready_to_cutover = false`), never in the
//! exit code.

use super::model::{ConstraintCheck, CountMismatch, VerificationReport};
use super::plan::TablePlan;
use crate::cli::common::database::{backend_name, quote_ident, quote_literal, scalar_i64};
use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use std::collections::BTreeSet;

/// Runs every verification against the target and returns the report; only
/// catalog/SQL failures return Err.
pub(super) async fn verify_target(
    target: &sea_orm::DatabaseConnection,
    plans: &[TablePlan],
) -> Result<VerificationReport> {
    let backend = target.get_database_backend();
    let mut report = VerificationReport {
        checked: true,
        ..VerificationReport::default()
    };

    // 1. Row counts against the planned source snapshot.
    for plan in plans {
        let target_rows = super::plan::table_row_count(target, backend, &plan.name).await?;
        if target_rows != plan.source_rows {
            report.count_mismatches.push(CountMismatch {
                table: plan.name.clone(),
                source_rows: plan.source_rows,
                target_rows,
            });
        }
    }

    let planned: BTreeSet<&str> = plans.iter().map(|plan| plan.name.as_str()).collect();
    // 2. Unique constraints discovered from the target catalog.
    let unique_indexes = unique_indexes(target, backend).await?;
    for (table, constraint, columns) in &unique_indexes {
        if !planned.contains(table.as_str()) {
            continue;
        }
        let violations = duplicate_group_count(target, backend, table, columns).await?;
        report.checked_unique_constraints += 1;
        if violations > 0 {
            report.unique_conflicts.push(ConstraintCheck {
                table: table.clone(),
                constraint: constraint.clone(),
                columns: columns.clone(),
                violations,
            });
        }
    }

    // 3. Foreign keys discovered from the target catalog.
    let foreign_keys = foreign_keys(target, backend).await?;
    for (child, constraint, columns, parent, parent_columns) in &foreign_keys {
        if !planned.contains(child.as_str()) || !planned.contains(parent.as_str()) {
            continue;
        }
        let violations =
            orphan_count(target, backend, child, columns, parent, parent_columns).await?;
        report.checked_foreign_keys += 1;
        if violations > 0 {
            report.foreign_key_violations.push(ConstraintCheck {
                table: child.clone(),
                constraint: constraint.clone(),
                columns: columns.clone(),
                violations,
            });
        }
    }

    Ok(report)
}

/// `(table, constraint_name, columns)` triples of unique indexes.
type IndexList = Vec<(String, String, Vec<String>)>;

/// Discovers unique indexes per backend. PostgreSQL only accepts plain column
/// indexes (`indexprs IS NULL`); expression and partial indexes are excluded
/// because Yggdrasil's schema defines none.
async fn unique_indexes(
    target: &sea_orm::DatabaseConnection,
    backend: DbBackend,
) -> Result<IndexList> {
    let mut indexes = Vec::new();
    match backend {
        DbBackend::Postgres => {
            let rows = target
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT tc.table_name, tc.constraint_name, \
                     kcu.column_name, kcu.ordinal_position \
                     FROM information_schema.table_constraints tc \
                     JOIN information_schema.key_column_usage kcu \
                       ON tc.constraint_name = kcu.constraint_name \
                      AND tc.table_schema = kcu.table_schema \
                     WHERE tc.constraint_type = 'UNIQUE' \
                       AND tc.table_schema = 'public' \
                     ORDER BY tc.table_name, tc.constraint_name, kcu.ordinal_position",
                ))
                .await
                .map_aster_err(AsterError::database_operation)?;
            push_grouped(&mut indexes, rows, 0, 1, 2)?;
        }
        DbBackend::MySql => {
            let rows = target
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT table_name, index_name, column_name, seq_in_index \
                     FROM information_schema.statistics \
                     WHERE table_schema = DATABASE() AND non_unique = 0 \
                     ORDER BY table_name, index_name, seq_in_index",
                ))
                .await
                .map_aster_err(AsterError::database_operation)?;
            push_grouped(&mut indexes, rows, 0, 1, 2)?;
        }
        DbBackend::Sqlite => {
            let tables = super::schema::source_table_names(target, backend).await?;
            for table in tables {
                let table_literal = quote_literal(&table);
                let index_rows = target
                    .query_all_raw(Statement::from_string(
                        backend,
                        format!("SELECT name, \"unique\" FROM pragma_index_list({table_literal})"),
                    ))
                    .await
                    .map_aster_err(AsterError::database_operation)?;
                for index_row in index_rows {
                    let index_name: String = index_row
                        .try_get_by_index(0)
                        .map_aster_err(AsterError::database_operation)?;
                    let is_unique: i64 = index_row
                        .try_get_by_index(1)
                        .map_aster_err(AsterError::database_operation)?;
                    if is_unique == 0 {
                        continue;
                    }
                    let column_rows = target
                        .query_all_raw(Statement::from_string(
                            backend,
                            format!(
                                "SELECT name FROM pragma_index_info({})",
                                quote_literal(&index_name)
                            ),
                        ))
                        .await
                        .map_aster_err(AsterError::database_operation)?;
                    let mut columns = Vec::new();
                    let mut expression_column = false;
                    for column_row in column_rows {
                        // Expression index entries decode to NULL here; the
                        // migration cannot verify those with column sets.
                        let name: Option<String> = column_row
                            .try_get_by_index(0)
                            .map_aster_err(AsterError::database_operation)?;
                        match name {
                            Some(name) => columns.push(name),
                            None => expression_column = true,
                        }
                    }
                    if !columns.is_empty() && !expression_column {
                        indexes.push((table.clone(), index_name, columns));
                    }
                }
            }
        }
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                backend_name(other)
            )));
        }
    }
    Ok(indexes)
}

/// Groups flat (table, constraint, column, position) rows into index triples.
fn push_grouped(
    indexes: &mut IndexList,
    rows: Vec<sea_orm::QueryResult>,
    table_index: usize,
    constraint_index: usize,
    column_index: usize,
) -> Result<()> {
    let mut current: Option<(String, String, Vec<String>)> = None;
    let mut current_has_expression_column = false;
    for row in rows {
        let table: String = row
            .try_get_by_index(table_index)
            .map_aster_err(AsterError::database_operation)?;
        let constraint: String = row
            .try_get_by_index(constraint_index)
            .map_aster_err(AsterError::database_operation)?;
        // Expression index entries (MySQL function indexes) report NULL here;
        // they cannot be verified with a column set, so the whole index is
        // dropped rather than misread.
        let column: Option<String> = row
            .try_get_by_index(column_index)
            .map_aster_err(AsterError::database_operation)?;
        let Some(column) = column else {
            if current
                .as_ref()
                .is_some_and(|(existing_table, existing_constraint, _)| {
                    existing_table == &table && existing_constraint == &constraint
                })
            {
                current_has_expression_column = true;
            }
            continue;
        };
        match &mut current {
            Some((current_table, current_constraint, columns))
                if current_table == &table && current_constraint == &constraint =>
            {
                columns.push(column);
            }
            _ => {
                if let Some(existing) = current.take()
                    && !current_has_expression_column
                {
                    indexes.push(existing);
                }
                current = Some((table, constraint, vec![column]));
                current_has_expression_column = false;
            }
        }
    }
    if let Some(existing) = current
        && !current_has_expression_column
    {
        indexes.push(existing);
    }
    Ok(())
}

/// `(child, constraint, columns, parent, parent_columns)` tuples.
type ForeignKeyList = Vec<(String, String, Vec<String>, String, Vec<String>)>;

/// Discovers foreign keys per backend, grouped per constraint.
async fn foreign_keys(
    target: &sea_orm::DatabaseConnection,
    backend: DbBackend,
) -> Result<ForeignKeyList> {
    let mut keys: ForeignKeyList = Vec::new();
    match backend {
        DbBackend::Postgres => {
            // conkey/confkey pair child and parent columns by ordinal
            // position; joining information_schema views alone would produce
            // a cartesian mix for composite foreign keys.
            let rows = target
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT con.conrelid::regclass::text AS child_table, \
                     con.conname AS constraint_name, \
                     child_att.attname AS child_column, \
                     con.confrelid::regclass::text AS parent_table, \
                     parent_att.attname AS parent_column, \
                     pair.ordinality \
                     FROM pg_constraint con \
                     CROSS JOIN LATERAL unnest(con.conkey) WITH ORDINALITY AS pair(child_attnum, ordinality) \
                     JOIN pg_attribute child_att \
                       ON child_att.attrelid = con.conrelid \
                      AND child_att.attnum = pair.child_attnum \
                     JOIN pg_attribute parent_att \
                       ON parent_att.attrelid = con.confrelid \
                      AND parent_att.attnum = con.confkey[pair.ordinality] \
                     WHERE con.contype = 'f' \
                     ORDER BY child_table, constraint_name, pair.ordinality",
                ))
                .await
                .map_aster_err(AsterError::database_operation)?;
            push_fk_grouped(&mut keys, rows)?;
        }
        DbBackend::MySql => {
            let rows = target
                .query_all_raw(Statement::from_string(
                    backend,
                    "SELECT table_name, constraint_name, column_name, \
                     referenced_table_name, referenced_column_name, ordinal_position \
                     FROM information_schema.key_column_usage \
                     WHERE table_schema = DATABASE() \
                       AND referenced_table_name IS NOT NULL \
                     ORDER BY table_name, constraint_name, ordinal_position",
                ))
                .await
                .map_aster_err(AsterError::database_operation)?;
            push_fk_grouped(&mut keys, rows)?;
        }
        DbBackend::Sqlite => {
            let tables = super::schema::source_table_names(target, backend).await?;
            for table in tables {
                let table_literal = quote_literal(&table);
                let rows = target
                    .query_all_raw(Statement::from_string(
                        backend,
                        format!(
                            "SELECT id, `from`, `table`, `to` FROM pragma_foreign_key_list({table_literal})"
                        ),
                    ))
                    .await
                    .map_aster_err(AsterError::database_operation)?;
                for row in rows {
                    let constraint_id: i64 = row
                        .try_get_by_index(0)
                        .map_aster_err(AsterError::database_operation)?;
                    let column: String = row
                        .try_get_by_index(1)
                        .map_aster_err(AsterError::database_operation)?;
                    let parent: String = row
                        .try_get_by_index(2)
                        .map_aster_err(AsterError::database_operation)?;
                    // `to` is NULL when the FK references the parent's primary
                    // key implicitly; skip such constraints since the copy
                    // preserves column values anyway.
                    let parent_column: Option<String> = row
                        .try_get_by_index(3)
                        .map_aster_err(AsterError::database_operation)?;
                    let Some(parent_column) = parent_column else {
                        continue;
                    };
                    keys.push((
                        table.clone(),
                        format!("{table}_fk_{constraint_id}"),
                        vec![column],
                        parent,
                        vec![parent_column],
                    ));
                }
            }
        }
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                backend_name(other)
            )));
        }
    }
    Ok(keys)
}

/// Groups flat FK rows into constraint tuples.
fn push_fk_grouped(keys: &mut ForeignKeyList, rows: Vec<sea_orm::QueryResult>) -> Result<()> {
    for row in rows {
        let child: String = row
            .try_get_by_index(0)
            .map_aster_err(AsterError::database_operation)?;
        let constraint: String = row
            .try_get_by_index(1)
            .map_aster_err(AsterError::database_operation)?;
        let column: String = row
            .try_get_by_index(2)
            .map_aster_err(AsterError::database_operation)?;
        let parent: String = row
            .try_get_by_index(3)
            .map_aster_err(AsterError::database_operation)?;
        let parent_column: String = row
            .try_get_by_index(4)
            .map_aster_err(AsterError::database_operation)?;
        match keys
            .iter_mut()
            .find(|(existing_child, existing_constraint, _, _, _)| {
                existing_child == &child && existing_constraint == &constraint
            }) {
            Some((_, _, columns, _, parent_columns)) => {
                columns.push(column);
                parent_columns.push(parent_column);
            }
            None => keys.push((child, constraint, vec![column], parent, vec![parent_column])),
        }
    }
    Ok(())
}

/// Counts duplicate groups under the plain-SQL unique semantics (NULLs
/// excluded from the uniqueness constraint).
async fn duplicate_group_count(
    target: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    table: &str,
    columns: &[String],
) -> Result<i64> {
    let null_guard = columns
        .iter()
        .map(|column| format!("{} IS NOT NULL", quote_ident(backend, column)))
        .collect::<Vec<_>>()
        .join(" AND ");
    let group_by = columns
        .iter()
        .map(|column| quote_ident(backend, column))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT COUNT(*) FROM (SELECT 1 FROM {} WHERE {null_guard} \
         GROUP BY {group_by} HAVING COUNT(*) > 1) AS duplicates",
        quote_ident(backend, table)
    );
    scalar_i64(target, backend, &sql).await
}

/// Counts child rows whose referenced parent is missing.
async fn orphan_count(
    target: &sea_orm::DatabaseConnection,
    backend: DbBackend,
    child: &str,
    columns: &[String],
    parent: &str,
    parent_columns: &[String],
) -> Result<i64> {
    let mut conditions = Vec::new();
    let mut on = Vec::new();
    for (child_column, parent_column) in columns.iter().zip(parent_columns.iter()) {
        conditions.push(format!(
            "c.{} IS NOT NULL",
            quote_ident(backend, child_column)
        ));
        on.push(format!(
            "c.{} = p.{}",
            quote_ident(backend, child_column),
            quote_ident(backend, parent_column)
        ));
    }
    // The parent-side NULL condition is what isolates orphans: without it,
    // every valid non-null reference counts as a violation.
    let mut join_conditions = conditions;
    join_conditions.push(format!(
        "p.{} IS NULL",
        quote_ident(backend, &parent_columns[0])
    ));
    let sql = format!(
        "SELECT COUNT(*) FROM {} c LEFT JOIN {} p ON {} WHERE {}",
        quote_ident(backend, child),
        quote_ident(backend, parent),
        on.join(" AND "),
        join_conditions.join(" AND ")
    );
    scalar_i64(target, backend, &sql).await
}

#[cfg(test)]
mod tests {

    #[test]
    fn planned_table_count_is_stable() {
        assert_eq!(
            super::super::plan::COPY_TABLE_ORDER.len(),
            27,
            "table list changed; update the FK-ordered list deliberately"
        );
    }
}
