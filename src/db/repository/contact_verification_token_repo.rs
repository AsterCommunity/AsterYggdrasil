//! Contact verification token repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_yggdrasil_model::entities::contact_verification_token::{
    self, Entity as ContactVerificationToken,
};
use aster_yggdrasil_model::types::auth::{VerificationChannel, VerificationPurpose};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    sea_query::Expr,
};

/// Inserts a new contact verification token row.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: contact_verification_token::ActiveModel,
) -> Result<contact_verification_token::Model> {
    model
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Looks up a verification token by its hash.
///
/// Returns `None` when no token with that hash exists; consumption state and expiry are not
/// filtered here and must be checked by the caller.
pub async fn find_by_token_hash<C: ConnectionTrait>(
    db: &C,
    token_hash: &str,
) -> Result<Option<contact_verification_token::Model>> {
    ContactVerificationToken::find()
        .filter(contact_verification_token::Column::TokenHash.eq(token_hash))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Finds the newest still-active token for a user, channel, and purpose combination.
///
/// "Active" means unconsumed and not expired at the current instant; ordering is by
/// `created_at` descending. Returns `None` when no such token exists.
pub async fn find_latest_active_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    channel: VerificationChannel,
    purpose: VerificationPurpose,
) -> Result<Option<contact_verification_token::Model>> {
    ContactVerificationToken::find()
        .filter(contact_verification_token::Column::UserId.eq(user_id))
        .filter(contact_verification_token::Column::Channel.eq(channel))
        .filter(contact_verification_token::Column::Purpose.eq(purpose))
        .filter(contact_verification_token::Column::ConsumedAt.is_null())
        .filter(contact_verification_token::Column::ExpiresAt.gt(Utc::now()))
        .order_by_desc(contact_verification_token::Column::CreatedAt)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Deletes all unconsumed tokens for a user, channel, and purpose combination, invalidating
/// codes issued earlier.
///
/// Already consumed tokens are kept for audit purposes.
pub async fn delete_active_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    channel: VerificationChannel,
    purpose: VerificationPurpose,
) -> Result<()> {
    ContactVerificationToken::delete_many()
        .filter(contact_verification_token::Column::UserId.eq(user_id))
        .filter(contact_verification_token::Column::Channel.eq(channel))
        .filter(contact_verification_token::Column::Purpose.eq(purpose))
        .filter(contact_verification_token::Column::ConsumedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

/// Marks a token as consumed with a guarded single-use update.
///
/// Returns `true` only when the token was still unconsumed; returns `false` when it was
/// already consumed, so concurrent verifications yield exactly one winner.
pub async fn mark_consumed_if_unused<C: ConnectionTrait>(db: &C, token_id: i64) -> Result<bool> {
    let result = ContactVerificationToken::update_many()
        .col_expr(
            contact_verification_token::Column::ConsumedAt,
            Expr::value(Some(Utc::now())),
        )
        .filter(contact_verification_token::Column::Id.eq(token_id))
        .filter(contact_verification_token::Column::ConsumedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Deletes tokens whose `expires_at` is in the past, returning the number of rows removed.
pub async fn delete_expired<C: ConnectionTrait>(db: &C) -> Result<u64> {
    let result = ContactVerificationToken::delete_many()
        .filter(contact_verification_token::Column::ExpiresAt.lt(Utc::now()))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}
