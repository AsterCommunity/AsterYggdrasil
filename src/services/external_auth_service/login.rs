use chrono::{Duration, Utc};
use sea_orm::ActiveValue::Set;

use crate::db::repository::{external_auth_login_flow_repo, external_auth_provider_repo};
use crate::entities::external_auth_login_flow;
use crate::errors::{AsterError, Result};
use crate::external_auth::{ExternalAuthCallback, MapExternalAuthResult, registry};
use crate::runtime::SharedRuntimeState;
use crate::types::ExternalAuthProviderKind;
use aster_forge_utils::numbers::u64_to_i64;

use super::normalize::{callback_redirect_uri, normalize_key, normalize_return_path, state_hash};
use super::providers::external_auth_provider_config;
use super::resolution::{
    external_auth_claims_missing_email, resolve_existing_external_auth_identity,
    resolve_external_auth_user,
};
use super::verification::create_pending_email_verification_flow;
use super::{
    ExternalAuthCallbackOutcome, ExternalAuthCallbackQuery, ExternalAuthCallbackResult,
    ExternalAuthPrimaryLogin, ExternalAuthStartLoginResponse, FLOW_TTL_SECS,
};

pub async fn start_login(
    state: &impl SharedRuntimeState,
    req: &actix_web::HttpRequest,
    provider_kind: ExternalAuthProviderKind,
    provider_key: &str,
    return_path: Option<&str>,
) -> Result<ExternalAuthStartLoginResponse> {
    let provider_key = normalize_key(provider_key)?;
    tracing::debug!(
        provider_kind = ?provider_kind,
        provider_key,
        has_return_path = return_path.is_some(),
        "starting external auth login"
    );
    let provider = external_auth_provider_repo::find_by_kind_key(
        state.writer_db(),
        provider_kind,
        &provider_key,
    )
    .await?
    .ok_or_else(|| {
        AsterError::record_not_found(format!(
            "external auth provider '{}:{provider_key}'",
            provider_kind.as_str()
        ))
    })?;
    if !provider.enabled {
        tracing::debug!(
            provider_id = provider.id,
            provider_kind = ?provider.provider_kind,
            provider_key = %provider.key,
            "external auth login rejected because provider is disabled"
        );
        return Err(AsterError::auth_forbidden(
            "external auth provider is disabled",
        ));
    }

    let return_path = normalize_return_path(return_path)?;
    let redirect_uri = callback_redirect_uri(state, req, provider.provider_kind, &provider.key)?;
    let auth_start = registry::default_registry()
        .get_driver(provider.provider_kind)?
        .start_authorization(&external_auth_provider_config(&provider), &redirect_uri)
        .await
        .map_external_auth()?;
    let now = Utc::now();
    let ttl = u64_to_i64(FLOW_TTL_SECS, "external auth login flow ttl")?;
    let flow = external_auth_login_flow::ActiveModel {
        provider_id: Set(provider.id),
        state_hash: Set(state_hash(&auth_start.state)),
        nonce: Set(auth_start.nonce),
        pkce_verifier: Set(auth_start.pkce_verifier),
        redirect_uri: Set(redirect_uri),
        return_path: Set(Some(return_path)),
        created_at: Set(now),
        expires_at: Set(now + Duration::seconds(ttl)),
        consumed_at: Set(None),
        ..Default::default()
    };
    external_auth_login_flow_repo::create(state.writer_db(), flow).await?;
    tracing::debug!(
        provider_id = provider.id,
        provider_kind = ?provider.provider_kind,
        provider_key = %provider.key,
        "external auth login flow created"
    );

    Ok(ExternalAuthStartLoginResponse {
        authorization_url: auth_start.authorization_url,
    })
}

pub async fn finish_callback(
    state: &impl SharedRuntimeState,
    provider_kind: ExternalAuthProviderKind,
    provider_key: &str,
    query: &ExternalAuthCallbackQuery,
    _ip_address: Option<&str>,
    _user_agent: Option<&str>,
) -> Result<ExternalAuthCallbackOutcome> {
    tracing::debug!(
        provider_kind = ?provider_kind,
        provider_key,
        has_error = query.error.is_some(),
        has_code = query.code.is_some(),
        has_state = query.state.is_some(),
        "finishing external auth callback"
    );
    if let Some(error) = query.error.as_deref() {
        let description = query
            .error_description
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(error);
        return Err(AsterError::auth_invalid_credentials(format!(
            "external auth provider returned error: {description}"
        )));
    }
    let code = query.code.as_deref().ok_or_else(|| {
        AsterError::auth_invalid_credentials("external auth callback missing code")
    })?;
    let state_value = query.state.as_deref().ok_or_else(|| {
        AsterError::auth_invalid_credentials("external auth callback missing state")
    })?;

    let flow = external_auth_login_flow_repo::consume_by_state_hash(
        state.writer_db(),
        &state_hash(state_value),
        Utc::now(),
    )
    .await?
    .ok_or_else(|| {
        AsterError::auth_invalid_credentials("external auth state is invalid or expired")
    })?;
    tracing::debug!(
        flow_id = flow.id,
        provider_id = flow.provider_id,
        "external auth callback flow consumed"
    );
    let provider =
        external_auth_provider_repo::find_by_id(state.writer_db(), flow.provider_id).await?;
    if provider.provider_kind != provider_kind {
        tracing::debug!(
            flow_id = flow.id,
            expected_provider_kind = ?provider.provider_kind,
            actual_provider_kind = ?provider_kind,
            "external auth callback rejected provider kind mismatch"
        );
        return Err(AsterError::auth_invalid_credentials(
            "external auth callback provider kind does not match login flow",
        ));
    }
    let expected_key = normalize_key(provider_key)?;
    if provider.key != expected_key {
        tracing::debug!(
            flow_id = flow.id,
            provider_id = provider.id,
            expected_provider_key = %provider.key,
            actual_provider_key = %expected_key,
            "external auth callback rejected provider key mismatch"
        );
        return Err(AsterError::auth_invalid_credentials(
            "external auth callback provider does not match login flow",
        ));
    }
    if !provider.enabled {
        tracing::debug!(
            provider_id = provider.id,
            provider_kind = ?provider.provider_kind,
            provider_key = %provider.key,
            "external auth callback rejected disabled provider"
        );
        return Err(AsterError::auth_forbidden(
            "external auth provider is disabled",
        ));
    }

    let user_claims = registry::default_registry()
        .get_driver(provider.provider_kind)?
        .exchange_callback(
            &external_auth_provider_config(&provider),
            ExternalAuthCallback {
                code: code.to_string(),
                nonce: flow.nonce,
                pkce_verifier: flow.pkce_verifier,
                redirect_uri: flow.redirect_uri.clone(),
            },
        )
        .await
        .map_external_auth()?;

    tracing::debug!(
        provider_id = provider.id,
        provider_kind = ?provider.provider_kind,
        provider_key = %provider.key,
        has_email = user_claims.email.as_ref().is_some_and(|email| !email.is_empty()),
        email_verified = user_claims.email_verified,
        has_display_name = user_claims.display_name.is_some(),
        has_preferred_username = user_claims.preferred_username.is_some(),
        "external auth callback exchanged claims"
    );

    if external_auth_claims_missing_email(&user_claims) {
        // Existing bindings are keyed by issuer + subject, so they may sign in
        // even when the current callback cannot provide an email snapshot.
        if let Some(resolved) =
            resolve_existing_external_auth_identity(state.writer_db(), &user_claims, Utc::now())
                .await?
        {
            tracing::debug!(
                provider_id = provider.id,
                user_id = resolved.user.id,
                linked = resolved.linked,
                auto_provisioned = resolved.auto_provisioned,
                "external auth callback resolved existing identity without email claim"
            );
            return Ok(ExternalAuthCallbackOutcome::Login(
                ExternalAuthCallbackResult {
                    primary_login: ExternalAuthPrimaryLogin {
                        user: resolved.user,
                        return_path: flow.return_path.unwrap_or_else(|| "/".to_string()),
                        provider_key: provider.key,
                        issuer: user_claims.identity_namespace,
                        subject: user_claims.subject,
                        linked: resolved.linked,
                        auto_provisioned: resolved.auto_provisioned,
                    },
                },
            ));
        }
        if provider.provider_kind == ExternalAuthProviderKind::GitHub
            && provider.require_email_verified
        {
            return Err(AsterError::auth_forbidden(
                "GitHub provider requires a verified primary email",
            ));
        }
        let pending = create_pending_email_verification_flow(
            state,
            &provider,
            &user_claims,
            flow.return_path.clone(),
        )
        .await?;
        tracing::debug!(
            provider_id = provider.id,
            "external auth callback requires local email verification"
        );
        return Ok(ExternalAuthCallbackOutcome::EmailVerificationRequired(
            pending,
        ));
    }

    let resolved = match resolve_external_auth_user(state, &provider, &user_claims).await? {
        Some(resolved) => resolved,
        None => {
            let pending = create_pending_email_verification_flow(
                state,
                &provider,
                &user_claims,
                flow.return_path.clone(),
            )
            .await?;
            tracing::debug!(
                provider_id = provider.id,
                "external auth callback requires email verification after resolution"
            );
            return Ok(ExternalAuthCallbackOutcome::EmailVerificationRequired(
                pending,
            ));
        }
    };

    tracing::debug!(
        provider_id = provider.id,
        user_id = resolved.user.id,
        linked = resolved.linked,
        auto_provisioned = resolved.auto_provisioned,
        "external auth callback resolved login"
    );
    Ok(ExternalAuthCallbackOutcome::Login(
        ExternalAuthCallbackResult {
            primary_login: ExternalAuthPrimaryLogin {
                user: resolved.user,
                return_path: flow.return_path.unwrap_or_else(|| "/".to_string()),
                provider_key: provider.key,
                issuer: user_claims.identity_namespace,
                subject: user_claims.subject,
                linked: resolved.linked,
                auto_provisioned: resolved.auto_provisioned,
            },
        },
    ))
}
