//! Local authentication and session service.

use super::contact_verification::issue_contact_verification_token;
use super::session::{AuthTokenBundle, issue_tokens};
use super::shared::{AuthUserInfo, create_user, validate_identity_input};
use crate::api::error_code::AsterErrorCode;
use crate::config::site_url::{PUBLIC_SITE_URL_KEY, normalize_public_site_url_config_value};
use crate::db::repository::{system_config_repo, user_repo};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::SharedRuntimeState;
use crate::services::mail::outbox::enqueue;
use crate::services::mail::template::MailTemplatePayload;
use crate::services::ops::audit;
use actix_web::HttpRequest;
use aster_yggdrasil_model::types::{auth::VerificationPurpose, user::UserRole, user::UserStatus};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone)]
/// Terminal state of self-service registration: a live session when email
/// activation is disabled (or the account is the first admin), otherwise an
/// unactivated account awaiting the emailed activation link.
pub enum RegisterOutcome {
    /// Registration completed with an issued session.
    Authenticated(AuthTokenBundle),
    /// Account created but locked until the emailed activation link is
    /// confirmed.
    PendingActivation(AuthUserInfo),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Client-safe registration response; tokens travel in cookies.
pub struct RegisterResponse {
    /// Access token lifetime in seconds; `0` when activation is pending.
    pub expires_in: u64,
    /// Whether the account must confirm the emailed activation link before
    /// logging in.
    pub requires_activation: bool,
}

impl RegisterOutcome {
    /// Strips tokens/user, leaving the client-safe registration response.
    pub fn response(&self) -> RegisterResponse {
        match self {
            Self::Authenticated(bundle) => RegisterResponse {
                expires_in: bundle.expires_in,
                requires_activation: false,
            },
            Self::PendingActivation(_) => RegisterResponse {
                expires_in: 0,
                requires_activation: true,
            },
        }
    }
}

/// One-time bootstrap: creates the first account as an Admin, optionally
/// seeds the `public_site_url` runtime config, issues a session, and writes a
/// system-setup audit entry. Rejected once any user exists.
pub async fn setup_first_admin<S>(
    state: &S,
    username: &str,
    email: &str,
    password: &str,
    public_site_url: Option<&str>,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
{
    tracing::debug!(
        username,
        has_public_site_url = public_site_url
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
        "starting first admin setup"
    );
    if user_repo::count_all(state.writer_db()).await? > 0 {
        tracing::debug!("first admin setup rejected because system is already initialized");
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthSetupAlreadyCompleted,
            "system is already initialized",
        ));
    }
    let normalized_public_site_url = match public_site_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(origin) => {
            let value = serde_json::to_string(&vec![origin.to_string()]).map_aster_err_ctx(
                "failed to serialize setup public_site_url",
                AsterError::internal_error,
            )?;
            Some(normalize_public_site_url_config_value(&value)?)
        }
        None => None,
    };
    validate_identity_input(username, email, password)?;
    let user = create_user(
        state,
        state.writer_db(),
        username,
        email,
        password,
        UserRole::Admin,
    )
    .await?;
    if let Some(value) = normalized_public_site_url {
        let saved = system_config_repo::upsert_with_options(
            state.writer_db(),
            PUBLIC_SITE_URL_KEY,
            &value,
            None,
            Some(user.id),
        )
        .await?;
        state.runtime_config().apply(saved);
    }
    let response = issue_tokens(state, user.clone(), req).await?;
    let audit_ctx = audit::AuditContext::from_request(req, user.id);
    audit::log(
        state,
        &audit_ctx,
        audit::AuditAction::SystemSetup,
        audit::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        None,
    )
    .await;
    tracing::debug!(user_id = user.id, "first admin setup completed");
    Ok(response)
}

/// Self-service registration. The first account becomes Admin; later ones are
/// Users and, when `register_activation_enabled`, start unverified with an
/// activation email queued instead of receiving a session.
pub async fn register<S>(
    state: &S,
    username: &str,
    email: &str,
    password: &str,
    req: &HttpRequest,
) -> Result<RegisterOutcome>
where
    S: SharedRuntimeState,
{
    tracing::debug!(username, "starting local user registration");
    let auth_policy =
        crate::config::auth_runtime::RuntimeAuthPolicy::from_runtime_config(state.runtime_config());
    if !auth_policy.allow_user_registration {
        tracing::debug!(
            username,
            "local user registration rejected because registration is disabled"
        );
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthRegistrationDisabled,
            "registration is disabled",
        ));
    }
    crate::config::local_email_policy::LocalEmailPolicy::from_runtime_config(
        state.runtime_config(),
    )
    .check(email)?;

    let is_first_user = user_repo::count_all(state.writer_db()).await? == 0;
    let role = if is_first_user {
        UserRole::Admin
    } else {
        UserRole::User
    };
    tracing::debug!(username, role = ?role, "creating local user");
    let activation_required = auth_policy.register_activation_enabled && !is_first_user;
    let user = if activation_required {
        let policy =
            crate::config::auth_runtime::RuntimeContactVerificationPolicy::from_runtime_config(
                state.runtime_config(),
            );
        let site_name = crate::config::branding::title_or_default(state.runtime_config());
        crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
            let user = super::shared::create_user_with_role(
                txn,
                state,
                super::shared::CreateUserWithRoleInput {
                    username,
                    email,
                    password,
                    role,
                    status: UserStatus::Active,
                    must_change_password: false,
                    email_verified_at: None,
                },
            )
            .await?;
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
            Ok(user)
        })
        .await?
    } else {
        create_user(state, state.writer_db(), username, email, password, role).await?
    };
    let audit_ctx = audit::AuditContext::from_request(req, user.id);
    audit::log(
        state,
        &audit_ctx,
        audit::AuditAction::UserRegister,
        audit::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        None,
    )
    .await;
    tracing::debug!(user_id = user.id, role = ?user.role, "local user registration completed");
    if activation_required {
        Ok(RegisterOutcome::PendingActivation(AuthUserInfo::from(user)))
    } else {
        let response = issue_tokens(state, user, req).await?;
        Ok(RegisterOutcome::Authenticated(response))
    }
}
