//! User invitation repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::user_invitation::{self, Entity as UserInvitation};
use aster_yggdrasil_model::types::user::UserInvitationStatus;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, sea_query::Expr,
};

/// Inserts a new user invitation row.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: user_invitation::ActiveModel,
) -> Result<user_invitation::Model> {
    model
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads an invitation by id.
///
/// Returns `Err(record_not_found)` when no invitation with that id exists.
pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<user_invitation::Model> {
    UserInvitation::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("user invitation #{id}")))
}

/// Looks up an invitation by its token hash.
///
/// Returns `None` when no invitation with that hash exists; status and expiry are not filtered
/// here and must be checked by the caller.
pub async fn find_by_token_hash<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
) -> Result<Option<user_invitation::Model>> {
    UserInvitation::find()
        .filter(user_invitation::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Lists all still-pending invitations for an email address, newest created first.
///
/// Used to detect duplicate outstanding invites before issuing a new one.
pub async fn find_pending_by_email<C: ConnectionTrait>(
    db: &C,
    email: &str,
) -> Result<Vec<user_invitation::Model>> {
    UserInvitation::find()
        .filter(user_invitation::Column::Email.eq(email))
        .filter(user_invitation::Column::Status.eq(UserInvitationStatus::Pending))
        .order_by_desc(user_invitation::Column::CreatedAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Opens the invitation list at its first page, newest created first.
///
/// See [`list_cursor_after`] for pagination semantics.
pub async fn list_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
) -> Result<CursorSlice<user_invitation::Model>> {
    list_cursor_after(db, limit, None).await
}

/// Paginates invitations using a keyset cursor over `(created_at, id)`, newest first.
///
/// `limit` is clamped to 1..=100; `after` is the cursor of the last item of the previous page.
/// The returned [`CursorSlice`] carries the total invitation count and a next cursor, which is
/// absent on the last page.
pub async fn list_cursor_after<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after: Option<(chrono::DateTime<chrono::Utc>, i64)>,
) -> Result<CursorSlice<user_invitation::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = UserInvitation::find();
    let total = query
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    if let Some((created_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(user_invitation::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(user_invitation::Column::CreatedAt.eq(created_at))
                        .add(user_invitation::Column::Id.lt(id)),
                ),
        );
    }
    let items = query
        .order_by_desc(user_invitation::Column::CreatedAt)
        .order_by_desc(user_invitation::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Revokes an invitation with a guarded update that only applies while it is still `Pending`.
///
/// Returns `true` on transition, `false` when the invitation is no longer pending (already
/// accepted, revoked, or expired).
pub async fn mark_revoked_if_pending<C: ConnectionTrait>(db: &C, id: i64) -> Result<bool> {
    let now = Utc::now();
    let result = UserInvitation::update_many()
        .col_expr(
            user_invitation::Column::Status,
            Expr::value(UserInvitationStatus::Revoked),
        )
        .col_expr(user_invitation::Column::UpdatedAt, Expr::value(now))
        .col_expr(user_invitation::Column::RevokedAt, Expr::value(Some(now)))
        .filter(user_invitation::Column::Id.eq(id))
        .filter(user_invitation::Column::Status.eq(UserInvitationStatus::Pending))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Marks an invitation as expired with a guarded update that only applies while still `Pending`.
///
/// Returns `true` on transition, `false` when the invitation is no longer pending.
pub async fn mark_expired_if_pending<C: ConnectionTrait>(db: &C, id: i64) -> Result<bool> {
    let result = UserInvitation::update_many()
        .col_expr(
            user_invitation::Column::Status,
            Expr::value(UserInvitationStatus::Expired),
        )
        .col_expr(user_invitation::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(user_invitation::Column::Id.eq(id))
        .filter(user_invitation::Column::Status.eq(UserInvitationStatus::Pending))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Accepts a pending invitation on behalf of `accepted_user_id`.
///
/// The guarded update only applies while the invitation is still `Pending` and unexpired at
/// the current instant, so concurrent signups with the same token yield exactly one winner.
/// Returns `true` on transition, `false` otherwise.
pub async fn mark_accepted_if_pending<C: ConnectionTrait>(
    db: &C,
    id: i64,
    accepted_user_id: i64,
) -> Result<bool> {
    let now = Utc::now();
    let result = UserInvitation::update_many()
        .col_expr(
            user_invitation::Column::Status,
            Expr::value(UserInvitationStatus::Accepted),
        )
        .col_expr(
            user_invitation::Column::AcceptedUserId,
            Expr::value(Some(accepted_user_id)),
        )
        .col_expr(user_invitation::Column::AcceptedAt, Expr::value(Some(now)))
        .col_expr(user_invitation::Column::UpdatedAt, Expr::value(now))
        .filter(user_invitation::Column::Id.eq(id))
        .filter(user_invitation::Column::Status.eq(UserInvitationStatus::Pending))
        .filter(user_invitation::Column::ExpiresAt.gt(now))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}
