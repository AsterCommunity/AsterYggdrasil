//! Rewrite stored permission identifiers to the namespaced form and widen
//! the operator scope column.
//!
//! Legacy rows hold bare short names (`overview`, `yggdrasil_access`, ...)
//! while the open identifier system stores full reversed-DNS ids under
//! `com.astercosm.yggdrasil.permission.admin.*` and
//! `com.astercosm.yggdrasil.capability.*`. The rewrite is deterministic and
//! idempotent: rows already carrying namespaced ids pass through unchanged.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Deterministic legacy short name to namespaced id mapping for admin-area
/// operator scopes.
const OPERATOR_SCOPE_REWRITES: &[(&str, &str)] = &[
    (
        "overview",
        "com.astercosm.yggdrasil.permission.admin.overview",
    ),
    ("users", "com.astercosm.yggdrasil.permission.admin.users"),
    (
        "profiles",
        "com.astercosm.yggdrasil.permission.admin.profiles",
    ),
    (
        "texture_library",
        "com.astercosm.yggdrasil.permission.admin.texture_library",
    ),
    ("audit", "com.astercosm.yggdrasil.permission.admin.audit"),
    ("tasks", "com.astercosm.yggdrasil.permission.admin.tasks"),
    (
        "settings",
        "com.astercosm.yggdrasil.permission.admin.settings",
    ),
    (
        "external_auth",
        "com.astercosm.yggdrasil.permission.admin.external_auth",
    ),
];

/// Deterministic legacy short name to namespaced id mapping for player
/// capability scopes.
const BAN_SCOPE_REWRITES: &[(&str, &str)] = &[
    (
        "yggdrasil_access",
        "com.astercosm.yggdrasil.capability.yggdrasil.access",
    ),
    (
        "yggdrasil_join",
        "com.astercosm.yggdrasil.capability.yggdrasil.join",
    ),
    (
        "minecraft_profile_manage",
        "com.astercosm.yggdrasil.capability.profile.manage",
    ),
    (
        "texture_upload",
        "com.astercosm.yggdrasil.capability.texture.upload",
    ),
    (
        "texture_library_interact",
        "com.astercosm.yggdrasil.capability.texture_library.interact",
    ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Widen first: namespaced ids exceed the legacy VARCHAR(32) width.
        widen_operator_scope_column(manager, 256).await?;
        rewrite_operator_scope_rows(manager, OPERATOR_SCOPE_REWRITES).await?;
        rewrite_json_scope_columns(manager, BAN_SCOPE_REWRITES, false).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rewrite_json_scope_columns(manager, BAN_SCOPE_REWRITES, true).await?;
        // Fold namespaced ids back to legacy short names before narrowing;
        // the forward map cannot match rows it already rewrote.
        let reversed: Vec<(&str, &str)> = OPERATOR_SCOPE_REWRITES
            .iter()
            .map(|(legacy, namespaced)| (*namespaced, *legacy))
            .collect();
        rewrite_operator_scope_rows(manager, &reversed).await?;
        widen_operator_scope_column(manager, 32).await?;
        Ok(())
    }
}

/// Expands or contracts `user_operator_scopes.scope`; SQLite does not
/// enforce VARCHAR lengths, so it only needs the data rewrite.
async fn widen_operator_scope_column(manager: &SchemaManager<'_>, len: u32) -> Result<(), DbErr> {
    match manager.get_database_backend() {
        DatabaseBackend::Sqlite => Ok(()),
        DatabaseBackend::MySql | DatabaseBackend::Postgres => {
            manager
                .alter_table(
                    Table::alter()
                        .table(UserOperatorScopes::Table)
                        .modify_column(
                            ColumnDef::new(UserOperatorScopes::Scope)
                                .string_len(len)
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await
        }
        backend => Err(DbErr::Migration(format!(
            "unsupported database backend for operator scope widening: {backend:?}"
        ))),
    }
}

/// Rewrites the `scope` column of `user_operator_scopes` one mapping at a
/// time; plain SQL keeps the pass identical across all three backends.
async fn rewrite_operator_scope_rows(
    manager: &SchemaManager<'_>,
    rewrites: &[(&str, &str)],
) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let (set_param, where_param) = match manager.get_database_backend() {
        DatabaseBackend::Postgres => ("$1", "$2"),
        _ => ("?", "?"),
    };
    for (legacy, namespaced) in rewrites {
        db.execute_raw(Statement::from_sql_and_values(
            manager.get_database_backend(),
            // Idempotent: namespaced rows never match the legacy names.
            format!(
                "UPDATE user_operator_scopes SET scope = {set_param} WHERE scope = {where_param}"
            ),
            [Value::from(*namespaced), Value::from(*legacy)],
        ))
        .await?;
    }
    Ok(())
}

/// Rewrites the JSON array columns of `user_bans` and `user_ban_events`
/// element-by-element in memory, which keeps behavior identical across
/// SQLite/MySQL/PostgreSQL JSON dialects.
///
/// With `reverse` set, namespaced ids are folded back to legacy short names
/// (used by `down`); otherwise legacy names are expanded.
async fn rewrite_json_scope_columns(
    manager: &SchemaManager<'_>,
    rewrites: &[(&str, &str)],
    reverse: bool,
) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let backend = manager.get_database_backend();

    for (table, column) in [
        ("user_bans", "scopes"),
        ("user_ban_events", "previous_scopes"),
        ("user_ban_events", "next_scopes"),
    ] {
        let rows = db
            .query_all_raw(Statement::from_string(
                backend,
                format!("SELECT id, {column} FROM {table}"),
            ))
            .await?;
        for row in rows {
            let id: i64 = row.try_get_by_index(0)?;
            let raw: Option<String> = row.try_get_by_index(1)?;
            let Some(raw) = raw else { continue };
            let Some(rewritten) = rewrite_scope_json(&raw, rewrites, reverse) else {
                continue;
            };
            let (first, second) = match backend {
                DatabaseBackend::Postgres => ("$1", "$2"),
                _ => ("?", "?"),
            };
            db.execute_raw(Statement::from_sql_and_values(
                backend,
                format!("UPDATE {table} SET {column} = {first} WHERE id = {second}"),
                [Value::from(rewritten), Value::from(id)],
            ))
            .await?;
        }
    }
    Ok(())
}

/// Rewrites one stored JSON scope array, returning `None` when nothing
/// changes. Unparseable arrays are left untouched rather than failing the
/// whole migration; the application layer treats them as invalid anyway.
fn rewrite_scope_json(raw: &str, rewrites: &[(&str, &str)], reverse: bool) -> Option<String> {
    let parsed: Vec<serde_json::Value> = serde_json::from_str(raw).ok()?;
    let mut changed = false;
    let mut rewritten = Vec::with_capacity(parsed.len());
    for item in parsed {
        let Some(text) = item.as_str() else {
            rewritten.push(item);
            continue;
        };
        let mapped = if reverse {
            rewrites
                .iter()
                .find(|(_, namespaced)| *namespaced == text)
                .map(|(legacy, _)| *legacy)
        } else {
            rewrites
                .iter()
                .find(|(legacy, _)| *legacy == text)
                .map(|(_, namespaced)| *namespaced)
        };
        match mapped {
            Some(next) => {
                changed = true;
                rewritten.push(serde_json::Value::String(next.to_string()));
            }
            None => rewritten.push(item),
        }
    }
    if !changed {
        return None;
    }
    serde_json::to_string(&rewritten).ok()
}

#[derive(DeriveIden)]
enum UserOperatorScopes {
    Table,
    Scope,
}
