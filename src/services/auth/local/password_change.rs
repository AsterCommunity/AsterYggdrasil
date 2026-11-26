//! Password change service.

use actix_web::HttpRequest;
use sea_orm::ConnectionTrait;

use super::shared::{is_email_verified, update_password_in_connection};
use crate::api::error_code::AsterErrorCode;
use crate::db::{repository::user_repo, transaction};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    DatabaseRuntimeState, PermissionRegistryRuntimeState, RuntimeConfigRuntimeState,
    SharedRuntimeState,
};
use crate::services::{auth::local, ops::audit};
use aster_yggdrasil_model::entities::user;

async fn verify_change_password_input<S>(
    state: &S,
    db: &impl ConnectionTrait,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<user::Model>
where
    S: RuntimeConfigRuntimeState,
{
    let user = user_repo::find_by_id(db, user_id).await?;
    if !user.status.is_active() {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if !is_email_verified(&user) {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPendingActivation,
            "account email activation is pending",
        ));
    }
    if !local::verify_user_password(state, current_password, &user.password_hash).await? {
        return Err(AsterError::auth_invalid_credentials("wrong password"));
    }
    if current_password == new_password {
        return Err(AsterError::validation_error(
            "new password must be different from current password",
        ));
    }
    Ok(user)
}

/// Changes a user's password after verifying the current one: rehashes the
/// new password, bumps the session version, and revokes all sessions so other
/// devices must re-authenticate.
pub async fn change_password<S>(
    state: &S,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<local::AuthUserInfo>
where
    S: DatabaseRuntimeState + PermissionRegistryRuntimeState + RuntimeConfigRuntimeState,
{
    let updated = transaction::with_transaction(state.writer_db(), async |txn| {
        let user =
            verify_change_password_input(state, txn, user_id, current_password, new_password)
                .await?;
        let updated = update_password_in_connection(state, txn, user, new_password).await?;
        user_repo::revoke_sessions_for_user(txn, updated.id).await?;
        Ok(updated)
    })
    .await?;
    local::auth_user_info(state, updated).await
}

/// [`change_password`] plus a user audit entry attributed to the request.
pub async fn change_password_with_audit<S>(
    state: &S,
    req: &HttpRequest,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<local::AuthUserInfo>
where
    S: PermissionRegistryRuntimeState + SharedRuntimeState,
{
    let user = change_password(state, user_id, current_password, new_password).await?;
    let audit_ctx = audit::AuditContext::from_request(req, user.id);
    audit::log(
        state,
        &audit_ctx,
        audit::AuditAction::UserChangePassword,
        audit::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        None,
    )
    .await;
    Ok(user)
}
