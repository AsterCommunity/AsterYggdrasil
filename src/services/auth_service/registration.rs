//! Local authentication and session service.

use super::contact_verification::issue_contact_verification_token;
use super::session::{AuthTokenBundle, issue_tokens};
use super::shared::{AuthUserInfo, create_user, validate_identity_input};
use crate::api::error_code::AsterErrorCode;
use crate::config::site_url::{PUBLIC_SITE_URL_KEY, normalize_public_site_url_config_value};
use crate::db::repository::{system_config_repo, user_repo};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::SharedRuntimeState;
use crate::services::audit_service;
use crate::services::mail::outbox::enqueue;
use crate::services::mail::template::MailTemplatePayload;
use actix_web::HttpRequest;
use aster_yggdrasil_model::types::{auth::VerificationPurpose, user::UserRole, user::UserStatus};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub enum RegisterOutcome {
    Authenticated(AuthTokenBundle),
    PendingActivation(AuthUserInfo),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RegisterResponse {
    pub expires_in: u64,
    pub requires_activation: bool,
}

impl RegisterOutcome {
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
    let audit_ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log(
        state,
        &audit_ctx,
        audit_service::AuditAction::SystemSetup,
        audit_service::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        None,
    )
    .await;
    tracing::debug!(user_id = user.id, "first admin setup completed");
    Ok(response)
}

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
    let audit_ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserRegister,
        audit_service::AuditEntityType::User,
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
