use crate::api::api_error_code::ApiErrorCode;
use crate::config::site_url;
use crate::entities::external_auth_provider;
use crate::errors::{Result, validation_error_with_code};
use crate::external_auth::MapExternalAuthResult;
use crate::runtime::SharedRuntimeState;
use crate::services::auth_service;
use crate::types::{ExternalAuthProtocol, ExternalAuthProviderKind};
use aster_forge_api::NullablePatch;
use aster_forge_external_auth::normalize as forge_normalize;

use super::{
    DEFAULT_SCOPES, EXTERNAL_AUTH_IDENTITY_NAMESPACE_MAX_LEN, EXTERNAL_AUTH_URL_MAX_LEN,
    REDACTED_SECRET,
};

pub(super) fn parse_allowed_domains(raw: Option<&str>) -> Result<Vec<String>> {
    forge_normalize::parse_allowed_domains(raw).map_external_auth()
}

pub(super) fn normalize_key(value: &str) -> Result<String> {
    forge_normalize::normalize_provider_key(value).map_external_auth()
}

pub(super) fn normalize_required(value: &str, field: &str, max_len: usize) -> Result<String> {
    forge_normalize::normalize_required_field(value, field, max_len).map_external_auth()
}

pub(super) fn normalize_optional_claim(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>> {
    forge_normalize::normalize_optional_claim(value, field).map_external_auth()
}

pub(super) fn normalize_secret_create(value: Option<String>) -> Option<String> {
    value
        .map(|secret| secret.trim().to_string())
        .filter(|secret| !secret.is_empty() && secret != REDACTED_SECRET)
}

pub(super) fn normalize_secret_update(
    value: NullablePatch<String>,
    existing: Option<String>,
) -> Option<String> {
    match value {
        NullablePatch::Absent => existing,
        NullablePatch::Null => None,
        NullablePatch::Value(secret) => {
            let trimmed = secret.trim();
            if trimmed.is_empty() {
                None
            } else if trimmed == REDACTED_SECRET {
                existing
            } else {
                Some(trimmed.to_string())
            }
        }
    }
}

pub(super) fn normalize_scopes_with_default(
    value: Option<&str>,
    default_scopes: &str,
    protocol: ExternalAuthProtocol,
) -> Result<String> {
    forge_normalize::normalize_scopes_with_default(value, default_scopes, protocol.into())
        .map_external_auth()
}

pub(super) fn normalize_scopes(
    value: Option<&str>,
    protocol: ExternalAuthProtocol,
) -> Result<String> {
    normalize_scopes_with_default(value, DEFAULT_SCOPES, protocol)
}

pub(super) fn normalize_icon_url_input(value: Option<String>) -> Result<Option<String>> {
    forge_normalize::normalize_icon_url_input(value, EXTERNAL_AUTH_URL_MAX_LEN).map_external_auth()
}

pub(super) fn normalize_issuer_url_input(
    value: Option<String>,
    required: bool,
) -> Result<Option<String>> {
    forge_normalize::normalize_issuer_url_input(
        value,
        required,
        EXTERNAL_AUTH_IDENTITY_NAMESPACE_MAX_LEN,
    )
    .map_external_auth()
}

pub(super) fn normalize_manual_endpoint_input(
    value: Option<String>,
    field: &str,
    required: bool,
    supported: bool,
) -> Result<Option<String>> {
    forge_normalize::normalize_manual_endpoint_input(
        value,
        field,
        required,
        supported,
        EXTERNAL_AUTH_URL_MAX_LEN,
    )
    .map_external_auth()
}

pub(super) fn normalize_allowed_domains(value: Option<Vec<String>>) -> Result<Option<String>> {
    forge_normalize::normalize_allowed_domains(value).map_external_auth()
}

pub(super) fn state_hash(state: &str) -> String {
    forge_normalize::state_hash(state)
}

pub(super) fn token_hash(token: &str) -> String {
    forge_normalize::token_hash(token)
}

pub(super) fn normalize_return_path(value: Option<&str>) -> Result<String> {
    forge_normalize::normalize_return_path(value, EXTERNAL_AUTH_URL_MAX_LEN).map_external_auth()
}

pub(super) fn normalize_flow_token(value: &str) -> Result<String> {
    forge_normalize::normalize_flow_token(value, 128).map_external_auth()
}

pub(super) fn normalize_email_for_external_auth(value: &str) -> Result<String> {
    let email = value.trim().to_string();
    auth_service::validate_email(&email)?;
    Ok(email)
}

fn callback_path(provider_kind: ExternalAuthProviderKind, provider_key: &str) -> String {
    format!(
        "/api/v1/auth/external-auth/{}/{provider_key}/callback",
        provider_kind.as_str()
    )
}

pub fn callback_redirect_uri(
    state: &impl SharedRuntimeState,
    req: &actix_web::HttpRequest,
    provider_kind: ExternalAuthProviderKind,
    provider_key: &str,
) -> Result<String> {
    let conn = req.connection_info();
    let scheme = conn.scheme();
    let host = conn.host();
    let path = callback_path(provider_kind, provider_key);
    let uri = site_url::public_app_url_for_request(state.runtime_config(), &path, scheme, host)
        .ok_or_else(|| {
            validation_error_with_code(
                ApiErrorCode::ExternalAuthCallbackRedirectUriRequired,
                "cannot build external auth callback redirect URI; configure public_site_url",
            )
        })?;
    if uri.starts_with('/') {
        return Err(validation_error_with_code(
            ApiErrorCode::ExternalAuthCallbackRedirectUriRequired,
            "external auth callback redirect URI must be absolute; configure public_site_url",
        ));
    }
    Ok(uri)
}

pub(super) fn email_domain_allowed(
    provider: &external_auth_provider::Model,
    email: &str,
) -> Result<bool> {
    forge_normalize::email_domain_allowed(provider.allowed_domains.as_deref(), email)
        .map_external_auth()
}
