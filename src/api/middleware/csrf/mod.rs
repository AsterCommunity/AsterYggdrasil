//! Product boundary for CSRF validation.
//!
//! The shared Actix middleware crate owns the product-neutral CSRF mechanics:
//! random token generation, double-submit cookie/header checks, and request
//! source validation using `Origin`, `Referer`, and `Sec-Fetch-Site`. This
//! module keeps Yggdrasil-specific concerns at the edge by injecting runtime
//! public-site origins and mapping Forge CSRF categories into `AsterError`
//! variants with stable API error codes.

use std::sync::OnceLock;

use actix_web::{HttpRequest, dev::ServiceRequest};
use aster_forge_actix_middleware::csrf::{
    CsrfError, CsrfErrorKind, CsrfTokenNames, RequestSourceMode,
};

use crate::config::{
    AuthConfig, DEFAULT_AUTH_CSRF_COOKIE_NAME, DEFAULT_AUTH_CSRF_HEADER_NAME, RuntimeConfig,
    site_url,
};
use crate::errors::{AsterError, Result};

static CSRF_TOKEN_NAMES: OnceLock<CsrfTokenNames> = OnceLock::new();

/// Initializes process-wide CSRF token names from static startup configuration.
///
/// CSRF token names are intentionally initialized once instead of being read
/// from runtime configuration on every request. Changing the cookie or header
/// name while a process is serving traffic would desynchronize browser cookies,
/// frontend request headers, and backend validation.
pub fn init_token_names_from_auth_config(config: &AuthConfig) -> Result<()> {
    init_token_names(&config.csrf_cookie_name, &config.csrf_header_name)
}

/// Initializes process-wide CSRF token names.
///
/// Calling this more than once with the same values is accepted so tests and
/// startup paths can be idempotent. Calling it with different values after
/// initialization is rejected because a running process cannot safely migrate
/// active browser sessions between token names.
pub fn init_token_names(cookie_name: &str, header_name: &str) -> Result<()> {
    let names = CsrfTokenNames::new(cookie_name, header_name)
        .map_err(|error| AsterError::config_error(error.message()))?;
    match CSRF_TOKEN_NAMES.set(names) {
        Ok(()) => Ok(()),
        Err(names) if token_names() == &names => Ok(()),
        Err(_) => Err(AsterError::config_error(
            "CSRF token names are already initialized",
        )),
    }
}

/// Returns the process-wide CSRF token names.
///
/// Tests and embedded callers that do not run the normal startup path still
/// receive Yggdrasil's product-specific defaults instead of Forge's
/// compatibility defaults, so generated cookies, CORS headers, and frontend
/// bootstrap metadata stay aligned.
pub fn token_names() -> &'static CsrfTokenNames {
    CSRF_TOKEN_NAMES.get_or_init(default_token_names)
}

fn default_token_names() -> CsrfTokenNames {
    match CsrfTokenNames::new(DEFAULT_AUTH_CSRF_COOKIE_NAME, DEFAULT_AUTH_CSRF_HEADER_NAME) {
        Ok(names) => names,
        Err(error) => {
            tracing::error!(
                message = error.message(),
                "invalid built-in Yggdrasil CSRF token names; falling back to Forge defaults"
            );
            CsrfTokenNames::default()
        }
    }
}

fn map_csrf_error(error: CsrfError) -> AsterError {
    match error.kind() {
        CsrfErrorKind::TokenNameInvalid => AsterError::config_error(error.message()),
        CsrfErrorKind::CookieMissing | CsrfErrorKind::HeaderMissing => {
            AsterError::auth_csrf_missing(error.message())
        }
        CsrfErrorKind::TokenInvalid => AsterError::auth_csrf_invalid(error.message()),
        CsrfErrorKind::RequestSchemeInvalid
        | CsrfErrorKind::RequestHostInvalid
        | CsrfErrorKind::RequestOriginInvalid
        | CsrfErrorKind::RequestRefererInvalid
        | CsrfErrorKind::RequestHeaderValueInvalid => AsterError::validation_error(error.message()),
        CsrfErrorKind::RequestSourceUntrusted
        | CsrfErrorKind::RequestOriginUntrusted
        | CsrfErrorKind::RequestRefererUntrusted
        | CsrfErrorKind::RequestSourceMissing => AsterError::auth_forbidden(error.message()),
    }
}

pub fn ensure_double_submit_token(req: &HttpRequest) -> Result<()> {
    aster_forge_actix_middleware::csrf::ensure_double_submit_token_with_names(req, token_names())
        .map_err(map_csrf_error)
}

pub fn ensure_service_double_submit_token(req: &ServiceRequest) -> Result<()> {
    aster_forge_actix_middleware::csrf::ensure_service_double_submit_token_with_names(
        req,
        token_names(),
    )
    .map_err(map_csrf_error)
}

pub fn ensure_request_source_allowed(
    req: &HttpRequest,
    runtime_config: &RuntimeConfig,
    mode: RequestSourceMode,
) -> Result<()> {
    aster_forge_actix_middleware::csrf::ensure_request_source_allowed(
        req,
        &site_url::public_site_urls(runtime_config),
        mode,
    )
    .map_err(map_csrf_error)
}

pub fn ensure_service_request_source_allowed(
    req: &ServiceRequest,
    runtime_config: &RuntimeConfig,
    mode: RequestSourceMode,
) -> Result<()> {
    aster_forge_actix_middleware::csrf::ensure_service_request_source_allowed(
        req,
        &site_url::public_site_urls(runtime_config),
        mode,
    )
    .map_err(map_csrf_error)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use actix_web::cookie::Cookie;

    use crate::api::error_code::AsterErrorCode;
    use crate::config::{RuntimeConfig, site_url};
    use crate::entities::system_config;
    use crate::errors::AsterError;
    use aster_forge_actix_middleware::csrf::{RequestSourceMode, build_csrf_token};
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};

    use super::{ensure_double_submit_token, ensure_request_source_allowed, token_names};
    fn error_code(error: &AsterError) -> Option<AsterErrorCode> {
        match error {
            AsterError::Public { code, .. } => Some(*code),
            _ => None,
        }
    }

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: ConfigSource::System,
            visibility: ConfigVisibility::Private,
            namespace: String::new(),
            category: String::new(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn csrf_tokens_are_forge_generated_url_safe_values() {
        let token_a = build_csrf_token();
        let token_b = build_csrf_token();

        assert_ne!(token_a, token_b);
        assert!(token_a.len() >= 32);
        assert!(
            token_a
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        );
    }

    #[test]
    fn double_submit_errors_keep_yggdrasil_error_codes() {
        let missing = actix_web::test::TestRequest::post()
            .uri("/api/v1/auth/profile")
            .to_http_request();
        let err = ensure_double_submit_token(&missing).unwrap_err();
        assert_eq!(error_code(&err), Some(AsterErrorCode::AuthCsrfMissing));

        let mismatch = actix_web::test::TestRequest::patch()
            .uri("/api/v1/auth/profile")
            .cookie(Cookie::new(token_names().cookie_name(), "token-a"))
            .insert_header((token_names().header_name(), "token-b"))
            .to_http_request();
        let err = ensure_double_submit_token(&mismatch).unwrap_err();
        assert_eq!(error_code(&err), Some(AsterErrorCode::AuthCsrfInvalid));
    }

    #[test]
    fn request_source_validation_uses_runtime_public_site_origins() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            site_url::PUBLIC_SITE_URL_KEY,
            r#"["https://panel.example.com"]"#,
        ));

        let req = actix_web::test::TestRequest::post()
            .insert_header(("Host", "api.example.com"))
            .insert_header(("Origin", "https://panel.example.com"))
            .to_http_request();

        assert!(
            ensure_request_source_allowed(&req, &runtime_config, RequestSourceMode::Required)
                .is_ok()
        );
    }
}
