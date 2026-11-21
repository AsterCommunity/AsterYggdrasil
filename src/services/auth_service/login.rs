//! Local authentication and session service.

use super::session::{AuthTokenBundle, issue_tokens, rotate_tokens};
use super::shared::is_email_verified;
use super::tokens::{decode_claims, ensure_token_type};
use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{auth_session_repo, user_repo};
use crate::errors::{AsterError, Result};
use crate::runtime::SharedRuntimeState;
use crate::services::audit_service;
use actix_web::HttpRequest;
use aster_yggdrasil_model::types::auth::TokenType;
use chrono::Utc;

pub async fn login<S>(
    state: &S,
    identifier: &str,
    password: &str,
    req: &HttpRequest,
) -> Result<AuthTokenBundle>
where
    S: SharedRuntimeState,
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
        super::tokens::burn_dummy_password(state).await;
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
    let runtimes = state.runtime_config().auth_runtimes()?;
    let password_matches = runtimes
        .password_hash_runtime()
        .verify_password(password, &user.password_hash)
        .await
        .map(|verification| verification.is_valid)
        .unwrap_or(false);
    if !password_matches {
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
    S: SharedRuntimeState,
{
    tracing::debug!("starting local refresh token rotation");
    let claims = decode_claims(
        state,
        refresh_token,
        aster_forge_auth::AuthTokenType::Refresh,
    )?;
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
    S: SharedRuntimeState,
{
    tracing::debug!("starting local logout");
    let claims = match decode_claims(
        state,
        refresh_token,
        aster_forge_auth::AuthTokenType::Refresh,
    ) {
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

pub(super) fn refresh_jti_from_token<S>(state: &S, refresh_token: &str) -> Result<String>
where
    S: SharedRuntimeState,
{
    let claims = decode_claims(
        state,
        refresh_token,
        aster_forge_auth::AuthTokenType::Refresh,
    )?;
    ensure_token_type(&claims, TokenType::Refresh)?;
    claims
        .jti
        .ok_or_else(|| AsterError::auth_token_invalid("refresh token missing jti"))
}

pub(super) fn optional_refresh_jti_from_token<S>(
    state: &S,
    refresh_token: Option<&str>,
) -> Option<String>
where
    S: SharedRuntimeState,
{
    refresh_token.and_then(|token| refresh_jti_from_token(state, token).ok())
}
