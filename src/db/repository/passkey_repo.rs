//! Repository helpers for passkey credentials.

use crate::errors::{AsterError, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::passkey::{self, Entity as Passkey};
use aster_yggdrasil_model::types::passkey::StoredPasskeyCredential;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, sea_query::Expr,
};

/// Lists all passkeys registered by a user, most recently used first, then newest created.
pub async fn list_for_user(db: &DatabaseConnection, user_id: i64) -> Result<Vec<passkey::Model>> {
    Passkey::find()
        .filter(passkey::Column::UserId.eq(user_id))
        .order_by_desc(passkey::Column::LastUsedAt)
        .order_by_desc(passkey::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Paginates a user's passkeys using a keyset cursor over `(created_at, id)`, newest first.
///
/// `limit` is clamped to 1..=100; `after` is the cursor of the last item of the previous page.
/// The returned [`CursorSlice`] carries the user's total passkey count and a next cursor, which
/// is absent on the last page.
pub async fn list_for_user_cursor(
    db: &DatabaseConnection,
    user_id: i64,
    limit: u64,
    after: Option<(chrono::DateTime<Utc>, i64)>,
) -> Result<CursorSlice<passkey::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = Passkey::find().filter(passkey::Column::UserId.eq(user_id));
    let total = query.clone().count(db).await.map_err(AsterError::from)?;
    if let Some((created_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(passkey::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(passkey::Column::CreatedAt.eq(created_at))
                        .add(passkey::Column::Id.lt(id)),
                ),
        );
    }
    let items = query
        .order_by_desc(passkey::Column::CreatedAt)
        .order_by_desc(passkey::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_err(AsterError::from)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Loads a passkey by id, scoped to its owner.
///
/// Returns `None` when the passkey does not exist or belongs to a different user.
pub async fn find_by_id_for_user(
    db: &DatabaseConnection,
    id: i64,
    user_id: i64,
) -> Result<Option<passkey::Model>> {
    Passkey::find()
        .filter(passkey::Column::Id.eq(id))
        .filter(passkey::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Loads a passkey by its WebAuthn credential id during authentication.
///
/// Returns `None` when no credential with that id is registered.
pub async fn find_by_credential_id(
    db: &DatabaseConnection,
    credential_id: &str,
) -> Result<Option<passkey::Model>> {
    Passkey::find()
        .filter(passkey::Column::CredentialId.eq(credential_id))
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Loads a passkey matching both the WebAuthn user handle and credential id.
///
/// Returns `None` on any mismatch; used to detect credentials registered to a different
/// account than the one being asserted.
pub async fn find_by_user_handle_and_credential_id(
    db: &DatabaseConnection,
    user_handle: &str,
    credential_id: &str,
) -> Result<Option<passkey::Model>> {
    Passkey::find()
        .filter(passkey::Column::UserHandle.eq(user_handle))
        .filter(passkey::Column::CredentialId.eq(credential_id))
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Reports whether any passkey is registered under the WebAuthn user handle.
pub async fn user_handle_exists(db: &DatabaseConnection, user_handle: &str) -> Result<bool> {
    let found = Passkey::find()
        .select_only()
        .column(passkey::Column::Id)
        .filter(passkey::Column::UserHandle.eq(user_handle))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(AsterError::from)?;
    Ok(found.is_some())
}

/// Renames a passkey owned by `user_id`, stamping `updated_at`.
///
/// Returns `false` when the passkey does not exist or belongs to a different user.
pub async fn update_name_for_user(
    db: &DatabaseConnection,
    id: i64,
    user_id: i64,
    name: &str,
) -> Result<bool> {
    let result = Passkey::update_many()
        .col_expr(passkey::Column::Name, Expr::value(name.to_string()))
        .col_expr(passkey::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(passkey::Column::Id.eq(id))
        .filter(passkey::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Persists the authenticator state observed after a successful WebAuthn assertion.
///
/// Stores the returned credential bytes, backup flags, and monotonic sign count, and stamps
/// `last_used_at`. Rejects a negative sign count as a validation error. Returns `false` when
/// the passkey row no longer exists.
pub async fn update_credential_after_auth(
    db: &DatabaseConnection,
    id: i64,
    credential: StoredPasskeyCredential,
    backup_eligible: bool,
    backed_up: bool,
    sign_count: i64,
    last_used_at: chrono::DateTime<Utc>,
) -> Result<bool> {
    if sign_count < 0 {
        return Err(AsterError::validation_error(
            "passkey sign count cannot be negative",
        ));
    }

    let result = Passkey::update_many()
        .col_expr(passkey::Column::Credential, Expr::value(credential))
        .col_expr(
            passkey::Column::BackupEligible,
            Expr::value(backup_eligible),
        )
        .col_expr(passkey::Column::BackedUp, Expr::value(backed_up))
        .col_expr(passkey::Column::SignCount, Expr::value(sign_count))
        .col_expr(passkey::Column::LastUsedAt, Expr::value(Some(last_used_at)))
        .col_expr(passkey::Column::UpdatedAt, Expr::value(last_used_at))
        .filter(passkey::Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Stamps `last_used_at` on a passkey without touching credential state.
///
/// Returns `false` when the passkey row no longer exists.
pub async fn touch_last_used(
    db: &DatabaseConnection,
    id: i64,
    last_used_at: chrono::DateTime<Utc>,
) -> Result<bool> {
    let result = Passkey::update_many()
        .col_expr(passkey::Column::LastUsedAt, Expr::value(Some(last_used_at)))
        .col_expr(passkey::Column::UpdatedAt, Expr::value(last_used_at))
        .filter(passkey::Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Deletes a passkey owned by `user_id`.
///
/// Returns `false` when the passkey does not exist or belongs to a different user.
pub async fn delete_for_user(db: &DatabaseConnection, id: i64, user_id: i64) -> Result<bool> {
    let result = Passkey::delete_many()
        .filter(passkey::Column::Id.eq(id))
        .filter(passkey::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}
