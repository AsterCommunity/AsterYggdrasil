//! Local authentication and session service.

use crate::errors::{AsterError, Result};
use crate::runtime::{RuntimeConfigRuntimeState, SharedRuntimeState};
use actix_web::HttpRequest;
use aster_yggdrasil_model::types::auth::TokenType;

/// Product view over Forge [`aster_forge_auth::TokenClaims`].
#[derive(Debug, Clone)]
pub struct AuthenticatedClaims {
    /// Local user id parsed from the JWT subject.
    pub user_id: i64,
    /// Session version embedded at issuance; must match the user row or the
    /// token is treated as stale.
    pub session_version: i64,
    /// JWT id, present on refresh tokens and matched against the session row
    /// during rotation.
    pub jti: Option<String>,
    /// Whether the token carries the must-change-password restriction.
    pub password_change: bool,
    /// Access or refresh; verified against the token's declared type.
    pub token_type: TokenType,
}

fn claims_from_forge(claims: aster_forge_auth::TokenClaims) -> Result<AuthenticatedClaims> {
    use aster_forge_auth::{AuthTokenType, Restriction};
    let user_id = claims
        .sub
        .as_str()
        .parse::<i64>()
        .map_err(|_| AsterError::auth_token_invalid("token subject is not a user id"))?;
    let session_version = i64::try_from(claims.session_version.value())
        .map_err(|_| AsterError::auth_token_invalid("token session version is invalid"))?;
    Ok(AuthenticatedClaims {
        user_id,
        session_version,
        jti: claims.jti.as_ref().map(|jti| jti.as_str().to_owned()),
        password_change: claims.restriction == Restriction::MustChangePassword,
        token_type: match claims.token_type {
            AuthTokenType::Access => TokenType::Access,
            AuthTokenType::Refresh => TokenType::Refresh,
        },
    })
}

pub(super) fn forge_restriction(password_change: bool) -> aster_forge_auth::Restriction {
    if password_change {
        aster_forge_auth::Restriction::MustChangePassword
    } else {
        aster_forge_auth::Restriction::None
    }
}

pub(super) fn ensure_token_type(claims: &AuthenticatedClaims, expected: TokenType) -> Result<()> {
    if claims.token_type != expected {
        return Err(AsterError::auth_token_invalid(format!(
            "not an {} token",
            expected.as_str()
        )));
    }
    Ok(())
}

pub(super) fn decode_access_claims<S: SharedRuntimeState>(
    state: &S,
    token: &str,
) -> Result<AuthenticatedClaims> {
    let claims = decode_claims(state, token, aster_forge_auth::AuthTokenType::Access)?;
    ensure_token_type(&claims, TokenType::Access)?;
    Ok(claims)
}

pub(super) fn decode_claims<S: SharedRuntimeState>(
    state: &S,
    token: &str,
    expected: aster_forge_auth::AuthTokenType,
) -> Result<AuthenticatedClaims> {
    let runtimes = state.runtime_config().auth_runtimes()?;
    let verified = runtimes
        .jwt_verifier()
        .verify(token, expected)
        .map_err(|error| AsterError::auth_token_invalid(error.to_string()))?;
    claims_from_forge(verified.into_claims())
}

/// Verifies an access token against an ad-hoc HMAC secret. Test and tooling
/// entry point; production verification goes through the runtime config
/// verifier.
pub fn verify_token(token: &str, jwt_secret: &str) -> Result<AuthenticatedClaims> {
    verify_typed_token(token, jwt_secret, aster_forge_auth::AuthTokenType::Access)
}

/// Verifies a token of an explicit type against an ad-hoc secret. Test and
/// tooling entry point; production verification goes through the runtime
/// config verifier.
///
/// # Errors
/// Returns an invalid-token error on any Forge verification failure.
/// Verifies a token of an explicit type against an ad-hoc secret. Test and
/// tooling entry point; production verification goes through the runtime
/// config verifier.
///
/// # Errors
/// Returns an invalid-token error on any Forge verification failure.
pub fn verify_typed_token(
    token: &str,
    jwt_secret: &str,
    expected: aster_forge_auth::AuthTokenType,
) -> Result<AuthenticatedClaims> {
    use aster_forge_auth::{JwtKey, JwtPolicy, JwtVerifier};
    let key = JwtKey::hmac(jwt_secret)
        .map_err(|error| AsterError::internal_error(format!("jwt key: {error}")))?;
    let policy = JwtPolicy::new(crate::config::JWT_ISSUER, crate::config::JWT_AUDIENCE, 1, 1)
        .map_err(|error| AsterError::internal_error(format!("jwt policy: {error}")))?;
    let verifier = JwtVerifier::with_system_clock(key, policy);
    let verified = verifier
        .verify(token, expected)
        .map_err(|error| AsterError::auth_token_invalid(error.to_string()))?;
    claims_from_forge(verified.into_claims())
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

/// Burns one argon2 verification for unknown identifiers so login timing does
/// not reveal account existence. This must queue like the known-account
/// `verify_password` path: the non-blocking `try_verify_password` variant
/// returns `Busy` immediately whenever the Argon2 permit pool is saturated,
/// and that response-time gap would expose whether an account exists.
pub(super) async fn burn_dummy_password<S>(state: &S)
where
    S: RuntimeConfigRuntimeState,
{
    let Ok(runtimes) = state.runtime_config().auth_runtimes() else {
        return;
    };
    let _ = runtimes
        .password_hash_runtime()
        .verify_password("not-the-password", runtimes.dummy_password_hash())
        .await;
}
