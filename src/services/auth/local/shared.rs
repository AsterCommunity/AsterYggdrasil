//! Local authentication and session service.

use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{user_operator_scope_repo, user_repo};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::user::profile::{self, AvatarAudience, AvatarInfo, UserProfileInfo};

use aster_forge_validation::email::normalize_email;
use aster_yggdrasil_model::entities::user;
use aster_yggdrasil_model::types::{
    user::AvatarSource, user::OperatorScope, user::UserRole, user::UserStatus,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, IntoActiveModel};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuthUserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub pending_email: Option<String>,
    pub role: UserRole,
    pub operator_scopes: Vec<OperatorScope>,
    pub status: UserStatus,
    pub must_change_password: bool,
    pub profile: UserProfileInfo,
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
            operator_scopes: Vec::new(),
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
    let profile = profile::get_profile_info(state, &user, AvatarAudience::SelfUser).await?;
    let operator_scopes = auth_user_operator_scopes(state, &user).await?;
    Ok(AuthUserInfo {
        id: user.id,
        username: user.username,
        email: user.email,
        email_verified: user.email_verified_at.is_some(),
        pending_email: user.pending_email,
        role: user.role,
        operator_scopes,
        status: user.status,
        must_change_password: user.must_change_password,
        profile,
    })
}

pub async fn auth_request_user_info<S>(state: &S, user: user::Model) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState,
{
    let operator_scopes = auth_user_operator_scopes(state, &user).await?;
    Ok(AuthUserInfo {
        operator_scopes,
        ..AuthUserInfo::from(user)
    })
}

async fn auth_user_operator_scopes<S>(state: &S, user: &user::Model) -> Result<Vec<OperatorScope>>
where
    S: DatabaseRuntimeState,
{
    if user.role != UserRole::Operator {
        return Ok(Vec::new());
    }
    user_operator_scope_repo::list_for_user(state.reader_db(), user.id).await
}

pub(super) async fn create_user<C, S>(
    state: &S,
    db: &C,
    username: &str,
    email: &str,
    password: &str,
    role: UserRole,
) -> Result<user::Model>
where
    C: ConnectionTrait,
    S: RuntimeConfigRuntimeState + ?Sized,
{
    create_user_with_options(
        state,
        db,
        CreateUserInput {
            username,
            email,
            password,
            role,
            status: UserStatus::Active,
            must_change_password: false,
        },
    )
    .await
}

struct CreateUserInput<'a> {
    username: &'a str,
    email: &'a str,
    password: &'a str,
    role: UserRole,
    status: UserStatus,
    must_change_password: bool,
}

async fn create_user_with_options<C, S>(
    state: &S,
    db: &C,
    input: CreateUserInput<'_>,
) -> Result<user::Model>
where
    C: ConnectionTrait,
    S: RuntimeConfigRuntimeState + ?Sized,
{
    let CreateUserInput {
        username,
        email,
        password,
        role,
        status,
        must_change_password,
    } = input;
    tracing::debug!(username, role = ?role, "creating user record");
    let email = validate_identity_input(username, email, password)?;
    let runtimes = state.runtime_config().auth_runtimes()?;
    let password_hash = runtimes
        .password_hash_runtime()
        .hash_password(password)
        .await
        .map_err(|error| AsterError::internal_error(format!("password hashing failed: {error}")))?;
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
    Ok(aster_forge_validation::email::normalize_email(email).map(|_| ())?)
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

/// Verifies a password against a known identifier's stored hash; failure keeps
/// the invalid-credentials response. Public for the shared password flows.
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
    state: &S,
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
        password_hash: Set(state
            .runtime_config()
            .auth_runtimes()?
            .password_hash_runtime()
            .hash_password(input.password)
            .await
            .map_err(|error| {
                AsterError::internal_error(format!("password hashing failed: {error}"))
            })?),
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

pub(super) fn validate_identity_input(
    username: &str,
    email: &str,
    password: &str,
) -> Result<String> {
    validate_username(username)?;
    let email = normalize_email(email)?;
    validate_password(password)?;
    Ok(email)
}

/// Verifies a password against a known identifier's stored hash; failure keeps
/// the invalid-credentials response. Public for the shared password flows.
pub(super) async fn update_password_in_connection<C, S>(
    state: &S,
    db: &C,
    user: user::Model,
    new_password: &str,
) -> Result<user::Model>
where
    C: ConnectionTrait,
    S: RuntimeConfigRuntimeState + ?Sized,
{
    validate_password(new_password)?;
    let mut active = user.into_active_model();
    let runtimes = state.runtime_config().auth_runtimes()?;
    let new_hash = runtimes
        .password_hash_runtime()
        .hash_password(new_password)
        .await
        .map_err(|error| AsterError::internal_error(format!("password hashing failed: {error}")))?;
    active.password_hash = Set(new_hash);
    active.must_change_password = Set(false);
    active.session_version = Set(active.session_version.unwrap().saturating_add(1));
    active.updated_at = Set(Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub(super) async fn ensure_email_available<C: ConnectionTrait>(
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

pub async fn verify_user_password<S>(state: &S, password: &str, stored_hash: &str) -> Result<bool>
where
    S: RuntimeConfigRuntimeState,
{
    let runtimes = state.runtime_config().auth_runtimes()?;
    Ok(runtimes
        .password_hash_runtime()
        .verify_password(password, stored_hash)
        .await
        .map(|verification| verification.is_valid)
        .unwrap_or(false))
}

/// Hashes a password through the runtime argon2 worker pool.
///
/// # Errors
/// Returns an internal error when hashing fails.
pub async fn hash_password_with_runtime<S>(state: &S, password: &str) -> Result<String>
where
    S: RuntimeConfigRuntimeState,
{
    let runtimes = state.runtime_config().auth_runtimes()?;
    runtimes
        .password_hash_runtime()
        .hash_password(password)
        .await
        .map_err(|error| AsterError::internal_error(format!("password hashing failed: {error}")))
}
