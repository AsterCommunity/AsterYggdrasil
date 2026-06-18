//! Password change service.

use actix_web::HttpRequest;
use sea_orm::ConnectionTrait;

use super::{is_email_verified, update_password_in_connection};
use crate::api::error_code::AsterErrorCode;
use crate::db::{repository::user_repo, transaction};
use crate::entities::user;
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::{audit_service, auth_service};
use crate::utils::hash::verify_password;

async fn verify_change_password_input<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<user::Model> {
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
    if !verify_password(current_password, &user.password_hash)? {
        return Err(AsterError::auth_invalid_credentials("wrong password"));
    }
    if current_password == new_password {
        return Err(AsterError::validation_error(
            "new password must be different from current password",
        ));
    }
    Ok(user)
}

pub async fn change_password<S>(
    state: &S,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<auth_service::AuthUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let updated = transaction::with_transaction(state.writer_db(), async |txn| {
        let user =
            verify_change_password_input(txn, user_id, current_password, new_password).await?;
        let updated = update_password_in_connection(txn, user, new_password).await?;
        user_repo::revoke_sessions_for_user(txn, updated.id).await?;
        Ok(updated)
    })
    .await?;
    auth_service::auth_user_info(state, updated).await
}

pub async fn change_password_with_audit<S>(
    state: &S,
    req: &HttpRequest,
    user_id: i64,
    current_password: &str,
    new_password: &str,
) -> Result<auth_service::AuthUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let user = change_password(state, user_id, current_password, new_password).await?;
    let audit_ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserChangePassword,
        audit_service::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        None,
    )
    .await;
    Ok(user)
}
