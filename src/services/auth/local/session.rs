//! Local authentication and session service.

use super::login::{optional_refresh_jti_from_token, refresh_jti_from_token};
use super::shared::AuthUserInfo;
use super::tokens::{
    AuthenticatedClaims, decode_access_claims, forge_restriction, peer_ip, user_agent,
};
use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{auth_session_repo, user_repo};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    AppConfigRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, SharedRuntimeState,
};
use crate::services::ops::audit;
use actix_web::HttpRequest;
use aster_yggdrasil_model::entities::{auth_session, user};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, ConnectionTrait};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
/// Session state carried by a token pair: normal authentication or a
/// restricted session that may only change the password.
pub enum AuthTokenStatus {
    /// Full access session.
    Authenticated,
    /// Restricted session issued when `must_change_password` is set; only
    /// password-change routes accept it.
    PasswordChangeRequired,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Client-safe subset of a token bundle; tokens themselves travel in cookies.
pub struct AuthTokenResponse {
    /// Access token lifetime in seconds.
    pub expires_in: u64,
    /// Whether this is a full session or restricted to password change.
    pub status: AuthTokenStatus,
}

#[derive(Debug, Clone)]
/// A freshly issued access/refresh token pair plus the session metadata the
/// route layer needs to respond and audit.
pub struct AuthTokenBundle {
    /// Signed JWT access token.
    pub access_token: String,
    /// Signed JWT refresh token; its jti is tracked on the session row.
    pub refresh_token: String,
    /// Access token lifetime in seconds.
    pub expires_in: u64,
    /// Whether this is a full session or restricted to password change.
    pub status: AuthTokenStatus,
    /// Authenticated user snapshot returned alongside the tokens.
    pub user: AuthUserInfo,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// One user session as shown in the account "active sessions" list.
pub struct AuthSessionInfo {
    /// Session id (UUID) used when revoking this session.
    pub id: String,
    /// Whether this session matches the refresh token the request presented.
    pub is_current: bool,
    /// Client user agent captured at session creation/refresh.
    pub user_agent: Option<String>,
    /// Peer IP captured at session creation/refresh.
    pub ip_address: Option<String>,
    /// When the session was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last time this session refreshed its tokens or was seen.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    /// When the session's current refresh token expires.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
    /// Whether the session has been revoked (explicitly or by revoking others).
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
    /// Strips the token values and user, leaving the client-safe response.
    pub fn response(&self) -> AuthTokenResponse {
        AuthTokenResponse {
            expires_in: self.expires_in,
            status: self.status,
        }
    }
}

/// Revokes one of the user's sessions by id and writes a user audit entry.
/// Returns whether the revoked session was the one that presented
/// `current_refresh_token`, so the route layer can clear cookies.
pub async fn revoke_session<S>(
    state: &S,
    user_id: i64,
    session_id: &str,
    current_refresh_token: Option<&str>,
    req: &HttpRequest,
) -> Result<bool>
where
    S: SharedRuntimeState,
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

    let audit_ctx = audit::AuditContext::from_request(req, user_id);
    audit::log_with_details(
        state,
        &audit_ctx,
        audit::AuditAction::UserRevokeSession,
        audit::AuditEntityType::AuthSession,
        None,
        Some(session_id),
        || {
            audit::details(audit::AuthSessionAuditDetails {
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

/// Revokes every session of the user except the one that presented
/// `current_refresh_token` and writes a user audit entry. Returns the number
/// of sessions revoked. Fails when the current token has no live session.
pub async fn revoke_other_sessions<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: &str,
    req: &HttpRequest,
) -> Result<u64>
where
    S: SharedRuntimeState,
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

    let audit_ctx = audit::AuditContext::from_request(req, user_id);
    audit::log_with_details(
        state,
        &audit_ctx,
        audit::AuditAction::UserRevokeOtherSessions,
        audit::AuditEntityType::AuthSession,
        None,
        None,
        || {
            audit::details(audit::AuthSessionAuditDetails {
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

/// Resolves the authenticated user from the request's access token. Enforces
/// user status, session version, and password-change restrictions; a
/// restricted token is only accepted on password-change routes.
pub async fn current_user<S>(state: &S, req: &HttpRequest) -> Result<user::Model>
where
    S: SharedRuntimeState,
{
    let token = crate::api::request_auth::access_token(req)
        .ok_or_else(|| AsterError::auth_token_invalid("missing access token"))?;
    let (user, claims) = current_user_and_claims_from_token(state, &token).await?;
    if claims.password_change && !super::token_scope::password_change_request_allowed(req) {
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::AuthPasswordChangeRequired,
            "password change required",
        ));
    }
    Ok(user)
}

/// Same checks as [`current_user`], but against a raw access token instead of
/// the request's Authorization header/cookie.
pub async fn current_user_from_token<S>(state: &S, token: &str) -> Result<user::Model>
where
    S: SharedRuntimeState,
{
    current_user_and_claims_from_token(state, token)
        .await
        .map(|(user, _claims)| user)
}

async fn current_user_and_claims_from_token<S>(
    state: &S,
    token: &str,
) -> Result<(user::Model, AuthenticatedClaims)>
where
    S: SharedRuntimeState,
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

/// Lists all of the user's sessions (revoked ones included), flagging which
/// one matches `current_refresh_token`.
pub async fn list_sessions<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: Option<&str>,
) -> Result<Vec<AuthSessionInfo>>
where
    S: SharedRuntimeState,
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

/// Cursor-paginated variant of [`list_sessions`], ordered by
/// `(last_seen_at, id)`; `limit` is clamped to 1..=100.
pub async fn list_sessions_cursor<S>(
    state: &S,
    user_id: i64,
    current_refresh_token: Option<&str>,
    limit: u64,
    cursor: Option<(chrono::DateTime<chrono::Utc>, String)>,
) -> Result<aster_forge_api::CursorPage<AuthSessionInfo, aster_forge_api::DateTimeStringCursor>>
where
    S: SharedRuntimeState,
{
    let current_refresh_jti = optional_refresh_jti_from_token(state, current_refresh_token);
    let limit = limit.clamp(1, 100);
    let page =
        auth_session_repo::list_by_user_cursor(state.reader_db(), user_id, limit, cursor).await?;
    let next_cursor = if page.has_more {
        page.items
            .last()
            .map(|session| aster_forge_api::DateTimeStringCursor {
                value: session.last_seen_at,
                id: session.id.clone(),
            })
    } else {
        None
    };
    let items = page
        .items
        .into_iter()
        .map(|session| AuthSessionInfo::from_model(session, current_refresh_jti.as_deref()))
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id,
        returned = items.len(),
        total = page.total,
        limit,
        has_current_refresh_token = current_refresh_jti.is_some(),
        "listed auth sessions page"
    );
    Ok(aster_forge_api::CursorPage::new(
        items,
        page.total,
        limit,
        next_cursor,
    ))
}

/// Deletes sessions whose refresh tokens have expired. Returns the number of
/// rows removed; intended for the cleanup task.
pub async fn cleanup_expired_auth_sessions<S: DatabaseRuntimeState>(state: &S) -> Result<u64> {
    auth_session_repo::delete_expired(state.writer_db(), Utc::now()).await
}

pub(super) async fn issue_tokens<S>(
    state: &S,
    user: user::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
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

/// Issues a fresh session (token pair) for an already-authenticated user;
/// entry point for flows such as passkey login completion.
pub async fn issue_tokens_for_authenticated_user<S>(
    state: &S,
    user: user::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
{
    issue_tokens(state, user, req).await
}

/// Issues a fresh session (token pair) for an already-authenticated user;
/// entry point for flows such as external auth login completion.
pub async fn issue_tokens_for_user<S>(
    state: &S,
    user: user::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
{
    issue_tokens(state, user, req).await
}

/// Loads the user by id and issues a fresh session (token pair); entry point
/// for flows that authenticate by user id, e.g. after a password change.
pub async fn issue_tokens_for_user_id<S>(
    state: &S,
    user_id: i64,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
{
    let user = user_repo::find_by_id(state.reader_db(), user_id).await?;
    issue_tokens(state, user, req).await
}

pub(super) async fn rotate_tokens<S>(
    state: &S,
    user: user::Model,
    session: &auth_session::Model,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
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

pub(super) struct IssuedTokens {
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
    S: AppConfigRuntimeState + RuntimeConfigRuntimeState,
{
    use aster_forge_auth::{SessionId, SessionVersion, Subject};

    let auth_policy =
        crate::config::auth_runtime::RuntimeAuthPolicy::from_runtime_config(state.runtime_config());
    let runtimes = state.runtime_config().auth_runtimes()?;
    let session = SessionId::new(
        session_id
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
    )
    .map_err(|error| AsterError::internal_error(format!("session id: {error}")))?;
    let subject = Subject::new(user_id.to_string())
        .map_err(|error| AsterError::internal_error(format!("token subject: {error}")))?;
    let version = SessionVersion::new(u64::try_from(session_version).map_err(|_| {
        AsterError::internal_error("session version must be non-negative for jwt claims")
    })?);
    let pair = runtimes
        .jwt_signer()
        .issue(
            subject,
            session.clone(),
            version,
            forge_restriction(password_change),
        )
        .map_err(|error| {
            AsterError::internal_error(format!("forge jwt issuance failed: {error}"))
        })?;
    let refresh_expires_at = chrono::DateTime::from_timestamp(pair.refresh_expires_at, 0)
        .ok_or_else(|| AsterError::internal_error("forge refresh expiry is not representable"))?
        .with_timezone(&Utc);

    Ok(IssuedTokens {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        session_id: session.as_str().to_owned(),
        refresh_jti: pair.refresh_jti.as_str().to_owned(),
        refresh_expires_at,
        access_expires_in: auth_policy.access_token_ttl_secs,
    })
}

pub(super) async fn persist_auth_session<C: ConnectionTrait>(
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
