//! Local authentication and session service.

use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{user_operator_scope_repo, user_repo};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{
    DatabaseRuntimeState, PermissionRegistryRuntimeState, RuntimeConfigRuntimeState,
};
use crate::services::user::profile::{self, AvatarAudience, AvatarInfo, UserProfileInfo};

use aster_forge_validation::email::normalize_email;
use aster_yggdrasil_model::entities::user;
use aster_yggdrasil_model::types::{
    permission::PermissionId, user::AvatarSource, user::UserRole, user::UserStatus,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, IntoActiveModel};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Authenticated-user view returned by login, registration, and `/me`
/// endpoints. Never includes credential material.
pub struct AuthUserInfo {
    /// Local user id.
    pub id: i64,
    /// Unique login username (4-16 ASCII letters/digits/`_`/`-`).
    pub username: String,
    /// Normalized account email.
    pub email: String,
    /// Whether the account email has been verified.
    pub email_verified: bool,
    /// New email awaiting verification during an email change; `None` when no
    /// change is in flight.
    pub pending_email: Option<String>,
    /// Account role (User, Operator, Admin).
    pub role: UserRole,
    /// Operator-only scope grants; empty for non-operator accounts unless
    /// explicitly loaded.
    pub operator_scopes: Vec<PermissionId>,
    /// Account status (active, disabled, pending activation).
    pub status: UserStatus,
    /// Whether the account must change its password before normal use.
    pub must_change_password: bool,
    /// Display name and avatar for the self audience; left at defaults by
    /// constructors that skip the profile lookup.
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

/// Builds the full auth user view: profile info for the self audience
/// (display name + avatar) and operator scopes when the user is an operator.
pub async fn auth_user_info<S>(state: &S, user: user::Model) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState + PermissionRegistryRuntimeState + RuntimeConfigRuntimeState,
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

/// Builds the auth user view without the profile lookup; `profile` stays at
/// its default placeholder. Only operator scopes are loaded.
pub async fn auth_request_user_info<S>(state: &S, user: user::Model) -> Result<AuthUserInfo>
where
    S: DatabaseRuntimeState + PermissionRegistryRuntimeState,
{
    let operator_scopes = auth_user_operator_scopes(state, &user).await?;
    Ok(AuthUserInfo {
        operator_scopes,
        ..AuthUserInfo::from(user)
    })
}

async fn auth_user_operator_scopes<S>(state: &S, user: &user::Model) -> Result<Vec<PermissionId>>
where
    S: DatabaseRuntimeState + PermissionRegistryRuntimeState,
{
    if user.role != UserRole::Operator {
        return Ok(Vec::new());
    }
    let stored = user_operator_scope_repo::list_for_user(state.reader_db(), user.id).await?;
    // Only registry-recognized identifiers may participate in authorization
    // gates; stray rows (manual SQL, future deprecations) never match.
    let registry = state.permission_registry();
    let mut scopes = Vec::with_capacity(stored.len());
    for scope in stored {
        if registry.contains(&scope) {
            scopes.push(scope);
        } else {
            tracing::warn!(
                user_id = user.id,
                permission = %scope,
                "dropping unregistered operator scope from stored grants"
            );
        }
    }
    Ok(scopes)
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

/// Validates the username policy: 4-16 characters from ASCII letters,
/// digits, `_`, and `-`.
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

/// Validates the email syntax through the shared normalizer (which also
/// lower-cases the domain); does not check availability.
pub fn validate_email(email: &str) -> Result<()> {
    Ok(aster_forge_validation::email::normalize_email(email).map(|_| ())?)
}

/// Validates the password policy: 8-128 characters; no composition rules.
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

/// Whether the account's current email has a verification timestamp.
pub fn is_email_verified(user: &user::Model) -> bool {
    user.email_verified_at.is_some()
}

/// Fully validated inputs for [`create_user_with_role`].
pub struct CreateUserWithRoleInput<'a> {
    /// Username; checked against the username policy and uniqueness.
    pub username: &'a str,
    /// Email; normalized and checked against active and pending uniqueness.
    pub email: &'a str,
    /// Plaintext password; hashed through the runtime Argon2 worker.
    pub password: &'a str,
    /// Role assigned to the new account.
    pub role: UserRole,
    /// Initial account status.
    pub status: UserStatus,
    /// Whether the account must change its password before normal use.
    pub must_change_password: bool,
    /// Verification timestamp to seed; `Some` marks the email as
    /// pre-verified (e.g. asserted by an external auth provider).
    pub email_verified_at: Option<chrono::DateTime<Utc>>,
}

/// Looks up a user by username or email (exact match); used to resolve
/// login identifiers. Returns `None` when no account matches.
pub async fn find_user_by_identifier<C: ConnectionTrait>(
    db: &C,
    identifier: &str,
) -> Result<Option<user::Model>> {
    user_repo::find_by_identifier(db, identifier).await
}

/// Creates a user with explicit role/status options: validates username,
/// email, and password, enforces username/email uniqueness (including pending
/// emails), generates a public UUID, and hashes the password through the
/// runtime Argon2 worker. Session version starts at 1.
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

/// Sets a new password inside an existing transaction: validates the policy,
/// stores a fresh Argon2 hash, clears `must_change_password`, and bumps the
/// session version so previously issued tokens become stale.
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

/// Rejects an email that another account already uses as its address or as a
/// pending change; `exclude_user_id` exempts that account (the owner).
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

/// Verifies a plaintext password against a stored Argon2 hash through the
/// runtime worker pool; returns `false` on any verification failure instead
/// of erroring. Public for the shared password flows.
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
