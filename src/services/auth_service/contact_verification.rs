//! Contact verification flows: email change, password reset, register activation.

use super::shared::{
    AuthUserInfo, auth_user_info, ensure_email_available, is_email_verified,
    update_password_in_connection, validate_password,
};

use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{contact_verification_token_repo, user_repo};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::mail;
use crate::services::mail::outbox::enqueue;
use crate::services::mail::template::MailTemplatePayload;
use aster_forge_crypto::sha256_hex;
use aster_forge_utils::numbers::u64_to_i64;
use aster_forge_validation::email::normalize_email;
use aster_yggdrasil_model::entities::{contact_verification_token, user};
use aster_yggdrasil_model::types::{auth::VerificationChannel, auth::VerificationPurpose};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ContactVerificationConfirmResult {
    pub purpose: VerificationPurpose,
    pub user_id: i64,
    pub target: String,
}

async fn password_reset_request_allowed<S, C>(state: &S, db: &C, user_id: i64) -> Result<bool>
where
    S: RuntimeConfigRuntimeState,
    C: ConnectionTrait,
{
    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let Some(latest) = contact_verification_token_repo::find_latest_active_for_user(
        db,
        user_id,
        VerificationChannel::Email,
        VerificationPurpose::PasswordReset,
    )
    .await?
    else {
        return Ok(true);
    };
    let cooldown = Duration::seconds(u64_to_i64(
        policy.password_reset_request_cooldown_secs,
        "password reset request cooldown",
    )?);
    Ok(latest.created_at + cooldown <= Utc::now())
}

#[derive(Debug, Clone)]
pub struct PasswordResetRequestResult {
    pub user: Option<AuthUserInfo>,
}

pub async fn request_email_change<S>(
    state: &S,
    user_id: i64,
    new_email: &str,
) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let normalized_email = normalize_email(new_email)?;
    let existing = user_repo::find_by_id(state.writer_db(), user_id).await?;
    if !existing.status.is_active() {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if !is_email_verified(&existing) {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPendingActivation,
            "account must be activated before changing email",
        ));
    }
    if existing.email == normalized_email {
        return Err(AsterError::validation_error(
            "new email must be different from current email",
        ));
    }
    crate::config::local_email_policy::LocalEmailPolicy::from_runtime_config(
        state.runtime_config(),
    )
    .check(&normalized_email)?;
    ensure_email_available(state.writer_db(), &normalized_email, Some(existing.id)).await?;
    if existing.pending_email.as_deref() == Some(normalized_email.as_str())
        && !register_activation_resend_allowed(state, state.writer_db(), existing.id).await?
    {
        return Err(AsterError::rate_limited(
            "please wait before resending verification email",
        ));
    }

    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let site_name = crate::config::branding::title_or_default(state.runtime_config());
    let user = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let mut active: user::ActiveModel = existing.into();
        active.pending_email = Set(Some(normalized_email.clone()));
        active.updated_at = Set(Utc::now());
        let updated = active
            .update(txn)
            .await
            .map_aster_err(AsterError::database_operation)?;
        let token = issue_contact_verification_token(
            txn,
            updated.id,
            VerificationPurpose::ContactChange,
            &normalized_email,
            policy.contact_change_ttl_secs,
        )
        .await?;
        enqueue(
            txn,
            &normalized_email,
            Some(&updated.username),
            MailTemplatePayload::contact_change_confirmation(&updated.username, &token, &site_name),
        )
        .await?;
        Ok(updated)
    })
    .await?;
    auth_user_info(state, user).await
}

pub async fn resend_email_change<S>(state: &S, user_id: i64) -> Result<Option<AuthUserInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    let pending_email = user
        .pending_email
        .clone()
        .ok_or_else(|| AsterError::validation_error("no pending email change request"))?;
    if !user.status.is_active() {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if !is_email_verified(&user) {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPendingActivation,
            "account must be activated before changing email",
        ));
    }
    crate::config::local_email_policy::LocalEmailPolicy::from_runtime_config(
        state.runtime_config(),
    )
    .check_not_blocked(&pending_email)?;
    ensure_email_available(state.writer_db(), &pending_email, Some(user.id)).await?;
    if !register_activation_resend_allowed(state, state.writer_db(), user.id).await? {
        return Ok(None);
    }

    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let site_name = crate::config::branding::title_or_default(state.runtime_config());
    crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let token = issue_contact_verification_token(
            txn,
            user.id,
            VerificationPurpose::ContactChange,
            &pending_email,
            policy.contact_change_ttl_secs,
        )
        .await?;
        enqueue(
            txn,
            &pending_email,
            Some(&user.username),
            MailTemplatePayload::contact_change_confirmation(&user.username, &token, &site_name),
        )
        .await?;
        Ok(())
    })
    .await?;
    auth_user_info(state, user).await.map(Some)
}

pub async fn request_password_reset<S>(state: &S, email: &str) -> Result<PasswordResetRequestResult>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let normalized_email = normalize_email(email)?;
    let Some(user) = user_repo::find_by_email(state.writer_db(), &normalized_email).await? else {
        return Ok(PasswordResetRequestResult { user: None });
    };
    if !user.status.is_active() || !is_email_verified(&user) {
        return Ok(PasswordResetRequestResult { user: None });
    }
    if !password_reset_request_allowed(state, state.writer_db(), user.id).await? {
        return Ok(PasswordResetRequestResult {
            user: Some(auth_user_info(state, user).await?),
        });
    }
    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let site_name = crate::config::branding::title_or_default(state.runtime_config());
    crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let token = issue_contact_verification_token(
            txn,
            user.id,
            VerificationPurpose::PasswordReset,
            &user.email,
            policy.password_reset_ttl_secs,
        )
        .await?;
        enqueue(
            txn,
            &user.email,
            Some(&user.username),
            MailTemplatePayload::password_reset(&user.username, &token, &site_name),
        )
        .await?;
        Ok(())
    })
    .await?;
    Ok(PasswordResetRequestResult {
        user: Some(auth_user_info(state, user).await?),
    })
}

pub async fn confirm_password_reset<S>(
    state: &S,
    token: &str,
    new_password: &str,
) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    validate_password(new_password)?;
    let token_hash = sha256_hex(token.as_bytes());
    let record =
        contact_verification_token_repo::find_by_token_hash(state.writer_db(), &token_hash)
            .await?
            .ok_or_else(|| {
                AsterError::contact_verification_invalid("password reset link is invalid")
            })?;
    if record.purpose != VerificationPurpose::PasswordReset {
        return Err(AsterError::contact_verification_invalid(
            "password reset link is invalid",
        ));
    }
    if record.consumed_at.is_some() {
        return Err(AsterError::contact_verification_invalid(
            "password reset link has already been used",
        ));
    }
    if record.expires_at <= Utc::now() {
        return Err(AsterError::contact_verification_expired(
            "password reset link has expired",
        ));
    }

    let site_name = crate::config::branding::title_or_default(state.runtime_config());
    let updated = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let existing_user = user_repo::find_by_id(txn, record.user_id).await?;
        if !existing_user.status.is_active() {
            return Err(AsterError::auth_forbidden_code(
                AsterErrorCode::AuthUserDisabled,
                "user is disabled",
            ));
        }
        if !is_email_verified(&existing_user) || existing_user.email != record.target {
            return Err(AsterError::contact_verification_invalid(
                "password reset request no longer exists",
            ));
        }
        if !contact_verification_token_repo::mark_consumed_if_unused(txn, record.id).await? {
            return Err(AsterError::contact_verification_invalid(
                "password reset link has already been used",
            ));
        }
        let updated =
            update_password_in_connection(state, txn, existing_user, new_password).await?;
        enqueue(
            txn,
            &updated.email,
            Some(&updated.username),
            MailTemplatePayload::password_reset_notice(&updated.username, &site_name),
        )
        .await?;
        Ok(updated)
    })
    .await?;
    auth_user_info(state, updated).await
}

#[derive(Debug, Clone)]
pub enum RegisterActivationResendOutcome {
    Sent(AuthUserInfo),
    EmailNotFound,
    AlreadyActive,
    AccountDisabled,
    Cooldown,
    EmailPolicyRejected,
}

pub async fn resend_register_activation<S>(
    state: &S,
    identifier: &str,
) -> Result<RegisterActivationResendOutcome>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let Some(user) = user_repo::find_by_identifier(state.writer_db(), identifier).await? else {
        return Ok(RegisterActivationResendOutcome::EmailNotFound);
    };

    if !user.status.is_active() {
        return Ok(RegisterActivationResendOutcome::AccountDisabled);
    }
    if is_email_verified(&user) {
        return Ok(RegisterActivationResendOutcome::AlreadyActive);
    }
    if crate::config::local_email_policy::LocalEmailPolicy::from_runtime_config(
        state.runtime_config(),
    )
    .check(&user.email)
    .is_err()
    {
        return Ok(RegisterActivationResendOutcome::EmailPolicyRejected);
    }
    if !register_activation_resend_allowed(state, state.writer_db(), user.id).await? {
        return Ok(RegisterActivationResendOutcome::Cooldown);
    }

    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let site_name = crate::config::branding::title_or_default(state.runtime_config());
    crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let token = issue_contact_verification_token(
            txn,
            user.id,
            VerificationPurpose::RegisterActivation,
            &user.email,
            policy.register_activation_ttl_secs,
        )
        .await?;
        enqueue(
            txn,
            &user.email,
            Some(&user.username),
            MailTemplatePayload::register_activation(&user.username, &token, &site_name),
        )
        .await?;
        Ok(())
    })
    .await?;
    Ok(RegisterActivationResendOutcome::Sent(AuthUserInfo::from(
        user,
    )))
}

pub async fn confirm_contact_verification<S>(
    state: &S,
    token: &str,
) -> Result<ContactVerificationConfirmResult>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let token_hash = sha256_hex(token.as_bytes());
    let record =
        contact_verification_token_repo::find_by_token_hash(state.writer_db(), &token_hash)
            .await?
            .ok_or_else(|| {
                AsterError::contact_verification_invalid("contact verification link is invalid")
            })?;
    if record.consumed_at.is_some() {
        return Err(AsterError::contact_verification_invalid(
            "contact verification link has already been used",
        ));
    }
    if record.expires_at <= Utc::now() {
        return Err(AsterError::contact_verification_expired(
            "contact verification link has expired",
        ));
    }
    let target = record.target.clone();
    let purpose = record.purpose;
    let user_id = record.user_id;
    crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let user = user_repo::find_by_id(txn, user_id).await?;
        if !user.status.is_active() {
            return Err(AsterError::auth_forbidden_code(
                AsterErrorCode::AuthUserDisabled,
                "user is disabled",
            ));
        }
        if purpose == VerificationPurpose::PasswordReset {
            return Err(AsterError::contact_verification_invalid(
                "password reset token cannot be confirmed from this endpoint",
            ));
        }
        if !contact_verification_token_repo::mark_consumed_if_unused(txn, record.id).await? {
            return Err(AsterError::contact_verification_invalid(
                "contact verification link has already been used",
            ));
        }
        let now = Utc::now();
        match purpose {
            VerificationPurpose::RegisterActivation => {
                if user.email != target {
                    return Err(AsterError::contact_verification_invalid(
                        "contact verification target mismatch",
                    ));
                }
                if user.email_verified_at.is_none() {
                    let mut active: user::ActiveModel = user.into();
                    active.email_verified_at = Set(Some(now));
                    active.updated_at = Set(now);
                    active
                        .update(txn)
                        .await
                        .map_aster_err(AsterError::database_operation)?;
                }
            }
            VerificationPurpose::ContactChange => {
                if user.email != target && user.pending_email.as_deref() != Some(target.as_str()) {
                    return Err(AsterError::contact_verification_invalid(
                        "contact change request no longer exists",
                    ));
                }
                ensure_email_available(txn, &target, Some(user.id)).await?;
                if user.email != target {
                    let previous_email = user.email.clone();
                    let username = user.username.clone();
                    let site_name =
                        crate::config::branding::title_or_default(state.runtime_config());
                    let mut active: user::ActiveModel = user.into();
                    active.email = Set(target.clone());
                    active.pending_email = Set(None);
                    active.email_verified_at = Set(Some(now));
                    active.updated_at = Set(now);
                    active
                        .update(txn)
                        .await
                        .map_aster_err(AsterError::database_operation)?;
                    enqueue(
                        txn,
                        &previous_email,
                        Some(&username),
                        MailTemplatePayload::contact_change_notice(
                            &username,
                            &previous_email,
                            &target,
                            &site_name,
                        ),
                    )
                    .await?;
                }
            }
            VerificationPurpose::PasswordReset => {
                return Err(AsterError::contact_verification_invalid(
                    "password reset token cannot be confirmed from this endpoint",
                ));
            }
        }
        Ok(())
    })
    .await?;

    Ok(ContactVerificationConfirmResult {
        purpose,
        user_id,
        target,
    })
}

pub async fn cleanup_expired_contact_verification_tokens<S: DatabaseRuntimeState>(
    state: &S,
) -> Result<u64> {
    contact_verification_token_repo::delete_expired(state.writer_db()).await
}

async fn register_activation_resend_allowed<S, C>(state: &S, db: &C, user_id: i64) -> Result<bool>
where
    S: RuntimeConfigRuntimeState,
    C: ConnectionTrait,
{
    let policy = crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let Some(latest) = contact_verification_token_repo::find_latest_active_for_user(
        db,
        user_id,
        VerificationChannel::Email,
        VerificationPurpose::RegisterActivation,
    )
    .await?
    else {
        return Ok(true);
    };
    let cooldown = Duration::seconds(u64_to_i64(
        policy.resend_cooldown_secs,
        "contact verification resend cooldown",
    )?);
    Ok(latest.created_at + cooldown <= Utc::now())
}

pub(super) async fn issue_contact_verification_token<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    purpose: VerificationPurpose,
    target: &str,
    ttl_secs: u64,
) -> Result<String> {
    let now = Utc::now();
    let token = mail::sender::build_verification_token();
    let token_hash = sha256_hex(token.as_bytes());
    contact_verification_token_repo::delete_active_for_user(
        db,
        user_id,
        VerificationChannel::Email,
        purpose,
    )
    .await?;
    contact_verification_token_repo::create(
        db,
        contact_verification_token::ActiveModel {
            user_id: Set(user_id),
            channel: Set(VerificationChannel::Email),
            purpose: Set(purpose),
            target: Set(target.to_string()),
            token_hash: Set(token_hash),
            expires_at: Set(
                now + Duration::seconds(u64_to_i64(ttl_secs, "contact verification ttl")?)
            ),
            consumed_at: Set(None),
            created_at: Set(now),
            ..Default::default()
        },
    )
    .await?;
    Ok(token)
}
