//! Local authentication and session service.

mod password_change;
mod token_scope;

use crate::api::error_code::AsterErrorCode;
use crate::api::pagination::{AdminUserSortBy, OffsetPage, SortOrder};
use crate::config::site_url::{PUBLIC_SITE_URL_KEY, normalize_public_site_url_config_value};
use crate::db::repository::{
    auth_session_repo, contact_verification_token_repo, system_config_repo, user_repo,
};
use crate::entities::{auth_session, contact_verification_token, user};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{AppConfigRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::profile_service::{self, AvatarAudience, AvatarInfo, UserProfileInfo};
use crate::services::{
    audit_service, mail_outbox_service, mail_service, mail_template::MailTemplatePayload,
};
use crate::types::{
    AvatarSource, TokenType, UserRole, UserStatus, VerificationChannel, VerificationPurpose,
};
use crate::utils::email::normalize_email;
use crate::utils::hash::{self, hash_password, verify_password};
use crate::utils::numbers::{i64_to_u64, u64_to_i64, u64_to_usize};
use actix_web::HttpRequest;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngExt;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, IntoActiveModel};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use uuid::Uuid;

const SUPER_ADMIN_USER_ID: i64 = 1;
const GENERATED_PASSWORD_LENGTH: usize = 24;
const GENERATED_PASSWORD_UPPERCASE: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
const GENERATED_PASSWORD_LOWERCASE: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
const GENERATED_PASSWORD_DIGITS: &[u8] = b"23456789";
const GENERATED_PASSWORD_SYMBOLS: &[u8] = b"!@#$%^&*-_+=";
const GENERATED_PASSWORD_CHARSET: &[u8] =
    b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_+=";

pub use password_change::{change_password, change_password_with_audit};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuthUserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub pending_email: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub must_change_password: bool,
    pub profile: UserProfileInfo,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AdminUserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub pending_email: Option<String>,
    pub role: UserRole,
    pub status: UserStatus,
    pub must_change_password: bool,
    pub session_version: i64,
    pub profile_count: u64,
    pub active_session_count: u64,
    pub profile: UserProfileInfo,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateAdminUserOutput {
    pub user: AdminUserInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AdminUpdateUserInput {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub must_change_password: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AdminUserListFilters {
    pub keyword: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
}

fn generate_temporary_password() -> String {
    let mut rng = rand::rng();
    let mut bytes = Vec::with_capacity(GENERATED_PASSWORD_LENGTH);
    for charset in [
        GENERATED_PASSWORD_UPPERCASE,
        GENERATED_PASSWORD_LOWERCASE,
        GENERATED_PASSWORD_DIGITS,
        GENERATED_PASSWORD_SYMBOLS,
    ] {
        let index = rng.random_range(0..charset.len());
        bytes.push(charset[index]);
    }
    while bytes.len() < GENERATED_PASSWORD_LENGTH {
        let index = rng.random_range(0..GENERATED_PASSWORD_CHARSET.len());
        bytes.push(GENERATED_PASSWORD_CHARSET[index]);
    }
    for index in (1..bytes.len()).rev() {
        let swap_index = rng.random_range(0..=index);
        bytes.swap(index, swap_index);
    }
    bytes.into_iter().map(char::from).collect()
}

impl From<user::Model> for AuthUserInfo {
    fn from(value: user::Model) -> Self {
        Self {
            id: value.id,
            username: value.username,
            email: value.email,
            email_verified: value.email_verified_at.is_some(),
            pending_email: value.pending_email,
            role: value.role,
            status: value.status,
            must_change_password: value.must_change_password,
            profile: default_user_profile_info(),
        }
    }
}

fn default_user_profile_info() -> UserProfileInfo {
    UserProfileInfo {
        display_name: None,
        avatar: AvatarInfo {
            source: AvatarSource::None,
            url_512: None,
            url_1024: None,
            version: 0,
        },
    }
}

pub async fn auth_user_info<S>(state: &S, user: user::Model) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let profile = profile_service::get_profile_info(state, &user, AvatarAudience::SelfUser).await?;
    Ok(AuthUserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        email_verified: user.email_verified_at.is_some(),
        pending_email: user.pending_email,
        role: user.role,
        status: user.status,
        must_change_password: user.must_change_password,
        profile,
    })
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AuthTokenStatus {
    Authenticated,
    PasswordChangeRequired,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuthTokenResponse {
    pub expires_in: u64,
    pub status: AuthTokenStatus,
}

#[derive(Debug, Clone)]
pub struct AuthTokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub status: AuthTokenStatus,
    pub user: AuthUserInfo,
}

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

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ContactVerificationConfirmResult {
    pub purpose: VerificationPurpose,
    pub user_id: i64,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuthSessionInfo {
    pub id: String,
    pub is_current: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked: bool,
}

impl From<auth_session::Model> for AuthSessionInfo {
    fn from(value: auth_session::Model) -> Self {
        Self {
            id: value.id,
            is_current: false,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            refresh_expires_at: value.refresh_expires_at,
            revoked: value.revoked_at.is_some(),
        }
    }
}

impl AuthSessionInfo {
    fn from_model(value: auth_session::Model, current_refresh_jti: Option<&str>) -> Self {
        let is_current =
            current_refresh_jti.is_some_and(|refresh_jti| refresh_jti == value.current_refresh_jti);
        Self {
            id: value.id,
            is_current,
            user_agent: value.user_agent,
            ip_address: value.ip_address,
            created_at: value.created_at,
            last_seen_at: value.last_seen_at,
            refresh_expires_at: value.refresh_expires_at,
            revoked: value.revoked_at.is_some(),
        }
    }
}

impl AuthTokenBundle {
    pub fn response(&self) -> AuthTokenResponse {
        AuthTokenResponse {
            expires_in: self.expires_in,
            status: self.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AccessClaims {
    pub sub: String,
    pub user_id: i64,
    pub session_version: i64,
    pub jti: Option<String>,
    #[serde(default)]
    pub password_change: bool,
    pub token_type: TokenType,
    pub exp: usize,
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
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
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
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
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
            let user = shared::create_user_with_role(
                txn,
                state,
                shared::CreateUserWithRoleInput {
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
            mail_outbox_service::enqueue(
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
        create_user(state.writer_db(), username, email, password, role).await?
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

pub async fn login<S>(
    state: &S,
    identifier: &str,
    password: &str,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!(
        identifier_len = identifier.len(),
        identifier_has_at = identifier.contains('@'),
        "starting local login"
    );
    let Some(user) = user_repo::find_by_identifier(state.reader_db(), identifier).await? else {
        tracing::debug!(
            identifier_len = identifier.len(),
            identifier_has_at = identifier.contains('@'),
            "local login rejected because identifier was not found"
        );
        return Err(AsterError::auth_invalid_credentials("invalid credentials"));
    };
    if !user.status.is_active() {
        tracing::debug!(
            user_id = user.id,
            status = ?user.status,
            "local login rejected because user is not active"
        );
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if !is_email_verified(&user) {
        tracing::debug!(
            user_id = user.id,
            "local login rejected because email activation is pending"
        );
        return Err(AsterError::auth_pending_activation(
            "account email activation is pending",
        ));
    }
    if !verify_password(password, &user.password_hash)? {
        tracing::debug!(
            user_id = user.id,
            "local login rejected because password did not match"
        );
        return Err(AsterError::auth_invalid_credentials("invalid credentials"));
    }
    let response = issue_tokens(state, user.clone(), req).await?;
    let audit_ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserLogin,
        audit_service::AuditEntityType::AuthSession,
        None,
        Some(&user.username),
        audit_service::details(audit_service::LoginAuditDetails { identifier }),
    )
    .await;
    tracing::debug!(user_id = user.id, "local login completed");
    Ok(response)
}

pub async fn refresh<S>(
    state: &S,
    refresh_token: &str,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!("starting local refresh token rotation");
    let claims = decode_claims(state, refresh_token)?;
    ensure_token_type(&claims, TokenType::Refresh)?;
    let refresh_jti = claims
        .jti
        .as_deref()
        .ok_or_else(|| AsterError::auth_token_invalid("refresh token missing jti"))?;
    let Some(session) = auth_session_repo::find_by_refresh_jti(state.writer_db(), refresh_jti)
        .await?
        .filter(|session| session.revoked_at.is_none())
    else {
        let reused =
            auth_session_repo::find_by_previous_refresh_jti(state.writer_db(), refresh_jti).await?;
        if reused.is_some() {
            tracing::debug!(
                user_id = claims.user_id,
                "local refresh rejected because refresh token was already rotated"
            );
            return Err(AsterError::auth_token_invalid("refresh token is stale"));
        }
        tracing::debug!(
            user_id = claims.user_id,
            "local refresh rejected because refresh session was not found"
        );
        return Err(AsterError::auth_token_invalid("invalid refresh token"));
    };
    if session.refresh_expires_at <= Utc::now() {
        tracing::debug!(
            user_id = claims.user_id,
            session_id = %session.id,
            "local refresh rejected because refresh session expired"
        );
        return Err(AsterError::auth_token_expired("refresh token expired"));
    }
    if session.user_id != claims.user_id {
        tracing::debug!(
            claims_user_id = claims.user_id,
            session_user_id = session.user_id,
            session_id = %session.id,
            "local refresh rejected because token user did not match session user"
        );
        return Err(AsterError::auth_token_invalid("invalid refresh token"));
    }

    let user = user_repo::find_by_id(state.reader_db(), claims.user_id).await?;
    if !user.status.is_active() {
        tracing::debug!(
            user_id = user.id,
            status = ?user.status,
            "local refresh rejected because user is not active"
        );
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if claims.session_version != user.session_version {
        tracing::debug!(
            user_id = user.id,
            token_session_version = claims.session_version,
            current_session_version = user.session_version,
            "local refresh rejected because session version is stale"
        );
        return Err(AsterError::auth_token_invalid("session is stale"));
    }

    if user.must_change_password || claims.password_change {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPasswordChangeRequired,
            "password change required",
        ));
    }

    let response = rotate_tokens(state, user.clone(), &session, req).await?;
    let audit_ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserRefreshToken,
        audit_service::AuditEntityType::AuthSession,
        None,
        Some(&user.username),
        None,
    )
    .await;
    tracing::debug!(user_id = user.id, session_id = %session.id, "local refresh token rotation completed");
    Ok(response)
}

pub async fn logout<S>(state: &S, refresh_token: &str, req: &HttpRequest) -> Result<bool>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!("starting local logout");
    let claims = match decode_claims(state, refresh_token) {
        Ok(claims) => claims,
        Err(AsterError::AuthTokenExpired(_) | AsterError::AuthTokenInvalid(_)) => {
            tracing::debug!("local logout ignored because refresh token is invalid or expired");
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if ensure_token_type(&claims, TokenType::Refresh).is_err() {
        tracing::debug!(
            user_id = claims.user_id,
            "local logout ignored because token type is not refresh"
        );
        return Ok(false);
    }
    let Some(refresh_jti) = claims.jti.as_deref() else {
        tracing::debug!(
            user_id = claims.user_id,
            "local logout ignored because refresh token has no jti"
        );
        return Ok(false);
    };

    let session = auth_session_repo::find_by_refresh_jti(state.writer_db(), refresh_jti).await?;
    let revoked = auth_session_repo::revoke_by_refresh_jti(state.writer_db(), refresh_jti).await?;
    tracing::debug!(
        user_id = claims.user_id,
        revoked,
        "local logout refresh session revocation finished"
    );
    if revoked {
        let audit_user_id = session
            .map(|session| session.user_id)
            .unwrap_or(claims.user_id);
        let audit_ctx = audit_service::AuditContext::from_request(req, audit_user_id);
        audit_service::log(
            state,
            &audit_ctx,
            audit_service::AuditAction::UserLogout,
            audit_service::AuditEntityType::AuthSession,
            None,
            None,
            None,
        )
        .await;
    }
    Ok(revoked)
}

fn refresh_jti_from_token<S>(state: &S, refresh_token: &str) -> Result<String>
where
    S: AppConfigRuntimeState,
{
    let claims = decode_claims(state, refresh_token)?;
    ensure_token_type(&claims, TokenType::Refresh)?;
    claims
        .jti
        .ok_or_else(|| AsterError::auth_token_invalid("refresh token missing jti"))
}

fn optional_refresh_jti_from_token<S>(state: &S, refresh_token: Option<&str>) -> Option<String>
where
    S: AppConfigRuntimeState,
{
    refresh_token.and_then(|token| refresh_jti_from_token(state, token).ok())
}

pub async fn revoke_session<S>(
    state: &S,
    user_id: i64,
    session_id: &str,
    current_refresh_token: Option<&str>,
    req: &HttpRequest,
) -> Result<bool>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!(user_id, session_id, "revoking auth session");
    let current_refresh_jti = optional_refresh_jti_from_token(state, current_refresh_token);
    let revoked_current =
        crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
            let session = auth_session_repo::find_by_id_for_user(txn, user_id, session_id)
                .await?
                .ok_or_else(|| {
                    AsterError::record_not_found_code(
                        AsterErrorCode::AuthSessionNotFound,
                        format!("auth session '{session_id}'"),
                    )
                })?;
            let revoked_current = current_refresh_jti
                .as_deref()
                .is_some_and(|refresh_jti| refresh_jti == session.current_refresh_jti);
            auth_session_repo::revoke_by_id_for_user(txn, user_id, session_id, Utc::now()).await?;
            Ok(revoked_current)
        })
        .await?;

    let audit_ctx = audit_service::AuditContext::from_request(req, user_id);
    audit_service::log_with_details(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserRevokeSession,
        audit_service::AuditEntityType::AuthSession,
        None,
        Some(session_id),
        || {
            audit_service::details(audit_service::AuthSessionAuditDetails {
                session_id: Some(session_id),
                removed: None,
                revoked_current: Some(revoked_current),
            })
        },
    )
    .await;

    tracing::debug!(
        user_id,
        session_id,
        revoked_current,
        "auth session revocation completed"
    );
    Ok(revoked_current)
}

pub async fn revoke_other_sessions<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: &str,
    req: &HttpRequest,
) -> Result<u64>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!(user_id, "revoking other auth sessions");
    let current_refresh_jti = refresh_jti_from_token(state, current_refresh_token)?;
    let removed = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let current_session = auth_session_repo::find_by_refresh_jti(txn, &current_refresh_jti)
            .await?
            .ok_or_else(|| AsterError::auth_token_invalid("missing current session"))?;
        if current_session.user_id != user_id {
            return Err(AsterError::auth_forbidden_code(
                AsterErrorCode::AuthSessionRevocationFailed,
                "current session does not belong to user",
            ));
        }
        if current_session.revoked_at.is_some() {
            return Err(AsterError::auth_token_invalid("missing current session"));
        }
        auth_session_repo::revoke_all_for_user_except_id(
            txn,
            user_id,
            &current_session.id,
            Utc::now(),
        )
        .await
    })
    .await?;

    let audit_ctx = audit_service::AuditContext::from_request(req, user_id);
    audit_service::log_with_details(
        state,
        &audit_ctx,
        audit_service::AuditAction::UserRevokeOtherSessions,
        audit_service::AuditEntityType::AuthSession,
        None,
        None,
        || {
            audit_service::details(audit_service::AuthSessionAuditDetails {
                session_id: None,
                removed: Some(removed),
                revoked_current: Some(false),
            })
        },
    )
    .await;

    tracing::debug!(user_id, removed, "other auth sessions revoked");
    Ok(removed)
}

pub async fn current_user<S>(state: &S, req: &HttpRequest) -> Result<user::Model>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState,
{
    let token = crate::api::request_auth::access_token(req)
        .ok_or_else(|| AsterError::auth_token_invalid("missing access token"))?;
    let (user, claims) = current_user_and_claims_from_token(state, &token).await?;
    if claims.password_change && !token_scope::password_change_request_allowed(req) {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPasswordChangeRequired,
            "password change required",
        ));
    }
    Ok(user)
}

pub async fn current_user_from_token<S>(state: &S, token: &str) -> Result<user::Model>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState,
{
    current_user_and_claims_from_token(state, token)
        .await
        .map(|(user, _claims)| user)
}

async fn current_user_and_claims_from_token<S>(
    state: &S,
    token: &str,
) -> Result<(user::Model, AccessClaims)>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState,
{
    tracing::debug!("resolving current user from access token");
    let claims = decode_access_claims(state, token)?;
    let user = user_repo::find_by_id(state.reader_db(), claims.user_id).await?;
    if !user.status.is_active() {
        tracing::debug!(
            user_id = user.id,
            status = ?user.status,
            "current user rejected because user is not active"
        );
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthUserDisabled,
            "user is disabled",
        ));
    }
    if user.session_version != claims.session_version {
        tracing::debug!(
            user_id = user.id,
            token_session_version = claims.session_version,
            current_session_version = user.session_version,
            "current user rejected because session version is stale"
        );
        return Err(AsterError::auth_token_invalid("session is stale"));
    }
    if user.must_change_password && !claims.password_change {
        return Err(AsterError::auth_token_invalid("password change required"));
    }
    if !user.must_change_password && claims.password_change {
        return Err(AsterError::auth_token_invalid(
            "password change session is no longer valid",
        ));
    }
    tracing::debug!(user_id = user.id, "current user resolved");
    Ok((user, claims))
}

pub async fn list_sessions<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: Option<&str>,
) -> Result<Vec<AuthSessionInfo>>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState,
{
    let current_refresh_jti = optional_refresh_jti_from_token(state, current_refresh_token);
    let sessions = auth_session_repo::list_by_user(state.reader_db(), user_id).await?;
    tracing::debug!(
        user_id,
        count = sessions.len(),
        has_current_refresh_token = current_refresh_jti.is_some(),
        "listed auth sessions"
    );
    Ok(sessions
        .into_iter()
        .map(|session| AuthSessionInfo::from_model(session, current_refresh_jti.as_deref()))
        .collect())
}

pub async fn list_sessions_paginated<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<OffsetPage<AuthSessionInfo>>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState,
{
    let current_refresh_jti = optional_refresh_jti_from_token(state, current_refresh_token);
    let page = auth_session_repo::list_by_user_paginated(state.reader_db(), user_id, limit, offset)
        .await?;
    let items = page
        .items
        .into_iter()
        .map(|session| AuthSessionInfo::from_model(session, current_refresh_jti.as_deref()))
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id,
        returned = items.len(),
        total = page.total,
        limit = page.limit,
        offset = page.offset,
        has_current_refresh_token = current_refresh_jti.is_some(),
        "listed auth sessions page"
    );
    Ok(OffsetPage::new(items, page.total, page.limit, page.offset))
}

pub async fn cleanup_expired_auth_sessions<S: DatabaseRuntimeState>(state: &S) -> Result<u64> {
    auth_session_repo::delete_expired(state.writer_db(), Utc::now()).await
}

pub async fn list_admin_users<S>(
    state: &S,
    limit: u64,
    offset: u64,
    filters: AdminUserListFilters,
    sort_by: AdminUserSortBy,
    sort_order: SortOrder,
) -> Result<OffsetPage<AdminUserInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        limit,
        offset,
        has_keyword = filters.keyword.is_some(),
        has_role_filter = filters.role.is_some(),
        has_status_filter = filters.status.is_some(),
        sort_by = ?sort_by,
        sort_order = ?sort_order,
        "listing admin users"
    );
    let page = user_repo::list_admin_paginated(
        state.reader_db(),
        user_repo::AdminUserFilters {
            keyword: filters.keyword,
            role: filters.role,
            status: filters.status,
        },
        sort_by,
        sort_order,
        limit,
        offset,
    )
    .await?;
    let items = hydrate_admin_users(state, page.items).await?;
    tracing::debug!(
        returned = items.len(),
        total = page.total,
        "listed admin users"
    );
    Ok(OffsetPage::new(items, page.total, page.limit, page.offset))
}

pub async fn get_admin_user<S>(state: &S, id: i64) -> Result<AdminUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(user_id = id, "loading admin user");
    let user = user_repo::find_by_id(state.reader_db(), id).await?;
    let users = hydrate_admin_users(state, vec![user]).await?;
    users
        .into_iter()
        .next()
        .ok_or_else(|| AsterError::internal_error("admin user hydration returned no item"))
}

pub async fn create_admin_user<S>(
    state: &S,
    username: &str,
    email: &str,
    password: Option<&str>,
    role: UserRole,
    status: UserStatus,
    must_change_password: Option<bool>,
) -> Result<CreateAdminUserOutput>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        username,
        role = ?role,
        status = ?status,
        "creating admin user"
    );
    let explicit_password = password.map(str::trim).filter(|value| !value.is_empty());
    let generated_password = explicit_password
        .is_none()
        .then(generate_temporary_password);
    let password = generated_password
        .as_deref()
        .or(explicit_password)
        .ok_or_else(|| AsterError::internal_error("temporary password generation failed"))?;
    validate_password(password)?;
    let must_change_password =
        generated_password.is_some() || must_change_password.unwrap_or(false);
    let user = create_user_with_options(
        state.writer_db(),
        username,
        email,
        password,
        role,
        status,
        must_change_password,
    )
    .await?;
    let users = hydrate_admin_users(state, vec![user]).await?;
    tracing::debug!(username, "admin user created");
    let user = users.into_iter().next().ok_or_else(|| {
        AsterError::internal_error("created admin user hydration returned no item")
    })?;
    Ok(CreateAdminUserOutput {
        user,
        generated_password,
    })
}

pub async fn update_admin_user<S>(
    state: &S,
    id: i64,
    input: AdminUpdateUserInput,
) -> Result<AdminUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let AdminUpdateUserInput {
        username,
        email,
        password,
        role,
        status,
        must_change_password,
    } = input;
    tracing::debug!(
        user_id = id,
        username_changed = username.is_some(),
        email_changed = email.is_some(),
        password_changed = password.is_some(),
        role_changed = role.is_some(),
        status_changed = status.is_some(),
        must_change_password_changed = must_change_password.is_some(),
        "updating admin user"
    );
    if id == SUPER_ADMIN_USER_ID && (role.is_some() || status.is_some()) {
        let existing = user_repo::find_by_id(state.reader_db(), id).await?;
        let role_changed = role.is_some_and(|next| next != existing.role);
        let status_changed = status.is_some_and(|next| next != existing.status);
        if role_changed || status_changed {
            return Err(AsterError::auth_forbidden(
                "super administrator role and status cannot be changed",
            ));
        }
    }

    let normalized_username = username
        .map(|value| {
            validate_username(&value)?;
            Ok::<_, AsterError>(value.trim().to_string())
        })
        .transpose()?;
    let normalized_email = email.map(|value| normalize_email(&value)).transpose()?;
    let password_hash = password
        .map(|password| {
            validate_password(&password)?;
            hash_password(&password)
        })
        .transpose()?;
    let bump_session_version = password_hash.is_some()
        || status == Some(UserStatus::Disabled)
        || must_change_password.is_some();
    let user = user_repo::update_admin(
        state.writer_db(),
        id,
        user_repo::AdminUpdateUserInput {
            username: normalized_username,
            email: normalized_email,
            password_hash,
            role,
            status,
            must_change_password,
            bump_session_version,
        },
    )
    .await?;
    let users = hydrate_admin_users(state, vec![user]).await?;
    tracing::debug!(user_id = id, "admin user updated");
    users
        .into_iter()
        .next()
        .ok_or_else(|| AsterError::internal_error("updated admin user hydration returned no item"))
}

pub async fn revoke_admin_user_sessions<S>(state: &S, user_id: i64) -> Result<u64>
where
    S: DatabaseRuntimeState,
{
    tracing::debug!(user_id, "revoking admin user sessions");
    user_repo::find_by_id(state.reader_db(), user_id).await?;
    user_repo::bump_session_version(state.writer_db(), user_id).await?;
    let removed = user_repo::revoke_sessions_for_user(state.writer_db(), user_id).await?;
    tracing::debug!(user_id, removed, "admin user sessions revoked");
    Ok(removed)
}

async fn hydrate_admin_users<S>(state: &S, users: Vec<user::Model>) -> Result<Vec<AdminUserInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let ids = users.iter().map(|user| user.id).collect::<Vec<_>>();
    tracing::debug!(count = ids.len(), "hydrating admin user summaries");
    let profile_counts = user_repo::count_profiles_by_user_ids(state.reader_db(), &ids).await?;
    let active_session_counts =
        user_repo::count_active_sessions_by_user_ids(state.reader_db(), &ids).await?;
    let profile_infos =
        profile_service::get_profile_info_map(state, &users, AvatarAudience::AdminUser).await?;
    Ok(users
        .into_iter()
        .map(|user| AdminUserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            pending_email: user.pending_email,
            role: user.role,
            status: user.status,
            must_change_password: user.must_change_password,
            session_version: user.session_version,
            profile_count: profile_counts.get(&user.id).copied().unwrap_or(0),
            active_session_count: active_session_counts.get(&user.id).copied().unwrap_or(0),
            profile: profile_infos
                .get(&user.id)
                .cloned()
                .unwrap_or_else(default_user_profile_info),
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
        .collect())
}

async fn create_user<C: ConnectionTrait>(
    db: &C,
    username: &str,
    email: &str,
    password: &str,
    role: UserRole,
) -> Result<user::Model> {
    create_user_with_options(
        db,
        username,
        email,
        password,
        role,
        UserStatus::Active,
        false,
    )
    .await
}

async fn create_user_with_options<C: ConnectionTrait>(
    db: &C,
    username: &str,
    email: &str,
    password: &str,
    role: UserRole,
    status: UserStatus,
    must_change_password: bool,
) -> Result<user::Model> {
    tracing::debug!(username, role = ?role, "creating user record");
    let email = validate_identity_input(username, email, password)?;
    let password_hash = hash_password(password)?;
    let user = user_repo::create_with_options(
        db,
        username,
        &email,
        &password_hash,
        role,
        status,
        must_change_password,
    )
    .await?;
    tracing::debug!(user_id = user.id, username, role = ?user.role, "user record created");
    Ok(user)
}

pub fn validate_username(username: &str) -> Result<()> {
    let username = username.trim();
    if username.len() < 4 {
        return Err(AsterError::validation_error(
            "username must be 4-16 characters",
        ));
    }
    if username.len() > 16 {
        return Err(AsterError::validation_error(
            "username must be 4-16 characters",
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AsterError::validation_error(
            "username may only contain letters, numbers, underscores and hyphens",
        ));
    }
    Ok(())
}

pub fn validate_email(email: &str) -> Result<()> {
    crate::utils::email::normalize_email(email).map(|_| ())
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthPasswordPolicyFailed,
            "password must be 8-128 characters",
        ));
    }
    if password.len() > 128 {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthPasswordPolicyFailed,
            "password must be 8-128 characters",
        ));
    }
    Ok(())
}

pub fn is_email_verified(user: &user::Model) -> bool {
    user.email_verified_at.is_some()
}

async fn ensure_email_available<C: ConnectionTrait>(
    db: &C,
    email: &str,
    exclude_user_id: Option<i64>,
) -> Result<()> {
    if let Some(existing) = user_repo::find_by_email(db, email).await?
        && Some(existing.id) != exclude_user_id
    {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthEmailExists,
            "email already exists",
        ));
    }
    if let Some(existing) = user_repo::find_by_pending_email(db, email).await?
        && Some(existing.id) != exclude_user_id
    {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthEmailExists,
            "email already exists",
        ));
    }
    Ok(())
}

async fn update_password_in_connection<C: ConnectionTrait>(
    db: &C,
    user: user::Model,
    new_password: &str,
) -> Result<user::Model> {
    validate_password(new_password)?;
    let mut active = user.into_active_model();
    active.password_hash = Set(hash_password(new_password)?);
    active.must_change_password = Set(false);
    active.session_version = Set(active.session_version.unwrap().saturating_add(1));
    active.updated_at = Set(Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
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
        mail_outbox_service::enqueue(
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
        mail_outbox_service::enqueue(
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
        mail_outbox_service::enqueue(
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
    let token_hash = hash::sha256_hex(token.as_bytes());
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
        let updated = update_password_in_connection(txn, existing_user, new_password).await?;
        mail_outbox_service::enqueue(
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
        mail_outbox_service::enqueue(
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
    let token_hash = hash::sha256_hex(token.as_bytes());
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
                    mail_outbox_service::enqueue(
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

async fn issue_contact_verification_token<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    purpose: VerificationPurpose,
    target: &str,
    ttl_secs: u64,
) -> Result<String> {
    let now = Utc::now();
    let token = mail_service::build_verification_token();
    let token_hash = hash::sha256_hex(token.as_bytes());
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

pub mod shared {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

    use super::{validate_email, validate_password, validate_username};
    use crate::api::error_code::AsterErrorCode;
    use crate::db::repository::user_repo;
    use crate::entities::user;
    use crate::errors::{AsterError, Result};
    use crate::runtime::RuntimeConfigRuntimeState;
    use crate::types::{UserRole, UserStatus};
    use crate::utils::email::normalize_email;
    use crate::utils::hash::hash_password;

    pub struct CreateUserWithRoleInput<'a> {
        pub username: &'a str,
        pub email: &'a str,
        pub password: &'a str,
        pub role: UserRole,
        pub status: UserStatus,
        pub must_change_password: bool,
        pub email_verified_at: Option<chrono::DateTime<Utc>>,
    }

    pub async fn find_user_by_identifier<C: ConnectionTrait>(
        db: &C,
        identifier: &str,
    ) -> Result<Option<user::Model>> {
        user_repo::find_by_identifier(db, identifier).await
    }

    pub async fn create_user_with_role<C, S>(
        db: &C,
        _state: &S,
        input: CreateUserWithRoleInput<'_>,
    ) -> Result<user::Model>
    where
        C: ConnectionTrait,
        S: RuntimeConfigRuntimeState + ?Sized,
    {
        validate_username(input.username)?;
        let email = normalize_email(input.email)?;
        validate_email(&email)?;
        validate_password(input.password)?;

        if user_repo::find_by_username(db, input.username)
            .await?
            .is_some()
        {
            return Err(AsterError::validation_error_code(
                AsterErrorCode::AuthUsernameExists,
                "username already exists",
            ));
        }
        if user_repo::find_by_email(db, &email).await?.is_some()
            || user_repo::find_by_pending_email(db, &email)
                .await?
                .is_some()
        {
            return Err(AsterError::validation_error_code(
                AsterErrorCode::AuthEmailExists,
                "email already exists",
            ));
        }

        let now = Utc::now();
        let public_uuid = user_repo::unique_public_uuid(db).await?;
        user::ActiveModel {
            public_uuid: Set(public_uuid),
            username: Set(input.username.to_string()),
            email: Set(email),
            password_hash: Set(hash_password(input.password)?),
            role: Set(input.role),
            status: Set(input.status),
            must_change_password: Set(input.must_change_password),
            session_version: Set(1),
            email_verified_at: Set(input.email_verified_at),
            pending_email: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AsterError::from)
    }
}

fn validate_identity_input(username: &str, email: &str, password: &str) -> Result<String> {
    validate_username(username)?;
    let email = normalize_email(email)?;
    validate_password(password)?;
    Ok(email)
}

async fn issue_tokens<S>(state: &S, user: user::Model, req: &HttpRequest) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    let password_change_required = user.must_change_password;
    let tokens = create_token_pair(
        state,
        user.id,
        user.session_version,
        None,
        password_change_required,
    )?;
    persist_auth_session(state.writer_db(), user.id, &tokens, req).await?;
    tracing::debug!(
        user_id = user.id,
        session_id = %tokens.session_id,
        expires_in = tokens.access_expires_in,
        "issued local auth token pair"
    );

    Ok(AuthTokenBundle {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.access_expires_in,
        status: if password_change_required {
            AuthTokenStatus::PasswordChangeRequired
        } else {
            AuthTokenStatus::Authenticated
        },
        user: AuthUserInfo::from(user),
    })
}

pub async fn issue_tokens_for_authenticated_user<S>(
    state: &S,
    user: user::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    issue_tokens(state, user, req).await
}

pub async fn issue_tokens_for_user<S>(
    state: &S,
    user: user::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    issue_tokens(state, user, req).await
}

pub async fn issue_tokens_for_user_id<S>(
    state: &S,
    user_id: i64,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    let user = user_repo::find_by_id(state.reader_db(), user_id).await?;
    issue_tokens(state, user, req).await
}

async fn rotate_tokens<S>(
    state: &S,
    user: user::Model,
    session: &auth_session::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    let tokens = create_token_pair(
        state,
        user.id,
        user.session_version,
        Some(&session.id),
        false,
    )?;
    let next_ip = peer_ip(req).or_else(|| session.ip_address.clone());
    let next_user_agent = user_agent(req).or_else(|| session.user_agent.clone());
    if !auth_session_repo::rotate_refresh(
        state.writer_db(),
        &session.current_refresh_jti,
        &tokens.refresh_jti,
        tokens.refresh_expires_at,
        next_ip.as_deref(),
        next_user_agent.as_deref(),
        Utc::now(),
    )
    .await?
    {
        tracing::debug!(
            user_id = user.id,
            session_id = %session.id,
            "local auth refresh rotation lost refresh session race"
        );
        return Err(AsterError::auth_token_invalid("refresh token is stale"));
    }
    tracing::debug!(
        user_id = user.id,
        session_id = %session.id,
        expires_in = tokens.access_expires_in,
        "rotated local auth token pair"
    );

    Ok(AuthTokenBundle {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.access_expires_in,
        status: AuthTokenStatus::Authenticated,
        user: AuthUserInfo::from(user),
    })
}

struct IssuedTokens {
    access_token: String,
    refresh_token: String,
    session_id: String,
    refresh_jti: String,
    refresh_expires_at: chrono::DateTime<Utc>,
    access_expires_in: u64,
}

fn create_token_pair<S>(
    state: &S,
    user_id: i64,
    session_version: i64,
    session_id: Option<&str>,
    password_change: bool,
) -> Result<IssuedTokens>
where
    S: RuntimeConfigRuntimeState + AppConfigRuntimeState,
{
    let now = Utc::now();
    let auth_policy =
        crate::config::auth_runtime::RuntimeAuthPolicy::from_runtime_config(state.runtime_config());
    let access = create_token(CreateTokenInput {
        user_id,
        session_version,
        token_type: TokenType::Access,
        ttl_secs: auth_policy.access_token_ttl_secs,
        secret: &state.config().auth.jwt_secret,
        jti: None,
        password_change,
        now,
    })?;
    let refresh_jti = Uuid::new_v4().to_string();
    let refresh = create_token(CreateTokenInput {
        user_id,
        session_version,
        token_type: TokenType::Refresh,
        ttl_secs: auth_policy.refresh_token_ttl_secs,
        secret: &state.config().auth.jwt_secret,
        jti: Some(refresh_jti.clone()),
        password_change,
        now,
    })?;

    Ok(IssuedTokens {
        access_token: access.token,
        refresh_token: refresh.token,
        session_id: session_id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        refresh_jti,
        refresh_expires_at: refresh.expires_at,
        access_expires_in: auth_policy.access_token_ttl_secs,
    })
}

struct CreatedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

struct CreateTokenInput<'a> {
    user_id: i64,
    session_version: i64,
    token_type: TokenType,
    ttl_secs: u64,
    secret: &'a str,
    jti: Option<String>,
    password_change: bool,
    now: chrono::DateTime<Utc>,
}

fn create_token(input: CreateTokenInput<'_>) -> Result<CreatedToken> {
    let CreateTokenInput {
        user_id,
        session_version,
        token_type,
        ttl_secs,
        secret,
        jti,
        password_change,
        now,
    } = input;
    let now_secs = i64_to_u64(now.timestamp(), "jwt issued-at unix timestamp")?;
    let exp_secs = now_secs.checked_add(ttl_secs).ok_or_else(|| {
        AsterError::internal_error(format!("jwt exp overflow: {now_secs} + {ttl_secs}"))
    })?;
    let expires_at = now
        .checked_add_signed(Duration::seconds(u64_to_i64(ttl_secs, "jwt ttl secs")?))
        .ok_or_else(|| AsterError::internal_error("jwt expires_at overflow"))?;
    let claims = AccessClaims {
        sub: user_id.to_string(),
        user_id,
        session_version,
        jti,
        password_change,
        token_type,
        exp: u64_to_usize(exp_secs, "jwt exp")?,
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_aster_err(AsterError::internal_error)?;
    Ok(CreatedToken { token, expires_at })
}

async fn persist_auth_session<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    tokens: &IssuedTokens,
    req: &HttpRequest,
) -> Result<auth_session::Model> {
    let now = Utc::now();
    auth_session_repo::create(
        db,
        auth_session::ActiveModel {
            id: Set(tokens.session_id.clone()),
            user_id: Set(user_id),
            current_refresh_jti: Set(tokens.refresh_jti.clone()),
            previous_refresh_jti: Set(None),
            refresh_expires_at: Set(tokens.refresh_expires_at),
            user_agent: Set(user_agent(req)),
            ip_address: Set(peer_ip(req)),
            created_at: Set(now),
            last_seen_at: Set(now),
            revoked_at: Set(None),
        },
    )
    .await
}

fn ensure_token_type(claims: &AccessClaims, expected: TokenType) -> Result<()> {
    if claims.token_type != expected {
        return Err(AsterError::auth_token_invalid(format!(
            "not an {} token",
            expected.as_str()
        )));
    }
    Ok(())
}

fn decode_access_claims<S: AppConfigRuntimeState>(state: &S, token: &str) -> Result<AccessClaims> {
    let claims = decode_claims(state, token)?;
    ensure_token_type(&claims, TokenType::Access)?;
    Ok(claims)
}

fn decode_claims<S: AppConfigRuntimeState>(state: &S, token: &str) -> Result<AccessClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(state.config().auth.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(AsterError::from)
}

pub fn verify_token(token: &str, jwt_secret: &str) -> Result<AccessClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(AsterError::from)
}

pub(crate) fn user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get(actix_web::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub(crate) fn peer_ip(req: &HttpRequest) -> Option<String> {
    req.peer_addr().map(|addr| addr.ip().to_string())
}
