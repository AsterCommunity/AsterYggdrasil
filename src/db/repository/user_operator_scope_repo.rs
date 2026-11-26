//! Operator scope repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_yggdrasil_model::entities::user_operator_scope::{self, Entity as UserOperatorScope};
use aster_yggdrasil_model::types::permission::PermissionId;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::{BTreeMap, BTreeSet};

/// Lists all operator scopes granted to a user, sorted by scope value ascending.
pub async fn list_for_user<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<Vec<PermissionId>> {
    let rows = UserOperatorScope::find()
        .filter(user_operator_scope::Column::UserId.eq(user_id))
        .order_by_asc(user_operator_scope::Column::Scope)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(rows.into_iter().map(|row| row.scope).collect())
}

/// Batch-loads operator scopes for several users into a map keyed by user id.
///
/// An empty `user_ids` slice returns an empty map without querying; each value list is sorted
/// by scope ascending, and users without any scope are absent from the map.
pub async fn list_for_user_ids<C: ConnectionTrait>(
    db: &C,
    user_ids: &[i64],
) -> Result<BTreeMap<i64, Vec<PermissionId>>> {
    if user_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = UserOperatorScope::find()
        .filter(user_operator_scope::Column::UserId.is_in(user_ids.iter().copied()))
        .order_by_asc(user_operator_scope::Column::UserId)
        .order_by_asc(user_operator_scope::Column::Scope)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut map = BTreeMap::new();
    for row in rows {
        map.entry(row.user_id)
            .or_insert_with(Vec::new)
            .push(row.scope);
    }
    Ok(map)
}

/// Replaces the full scope set of a user: deletes all existing rows, then inserts the
/// deduplicated `scopes` in sorted order.
///
/// Callers running against a non-transactional connection must wrap this in a transaction;
/// each insert commits independently otherwise.
pub async fn replace_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    scopes: &[PermissionId],
) -> Result<()> {
    UserOperatorScope::delete_many()
        .filter(user_operator_scope::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    let now = chrono::Utc::now();
    let unique_scopes = scopes.iter().cloned().collect::<BTreeSet<_>>();
    for scope in unique_scopes {
        user_operator_scope::ActiveModel {
            user_id: Set(user_id),
            scope: Set(scope),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    }
    Ok(())
}
