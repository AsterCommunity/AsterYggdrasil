//! Runtime CORS configuration helpers.
//!
//! This module keeps Yggdrasil's runtime configuration keys, defaults, and error
//! mapping while delegating the product-neutral policy shape and origin matching
//! mechanics to `aster_forge_actix_middleware`.

use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};
use aster_forge_actix_middleware::cors as forge_cors;
use aster_forge_config::{
    normalize_non_negative_u64_config_value, normalize_strict_bool_config_value,
};

pub use crate::config::definitions::{
    CORS_ALLOW_CREDENTIALS_KEY, CORS_ALLOWED_ORIGINS_KEY, CORS_ENABLED_KEY, CORS_MAX_AGE_SECS_KEY,
};

pub const DEFAULT_CORS_ENABLED: bool = false;
pub const DEFAULT_CORS_ALLOW_CREDENTIALS: bool = false;
pub const DEFAULT_CORS_MAX_AGE_SECS: u64 = 3600;

pub type CorsAllowedOrigins = forge_cors::CorsAllowedOrigins;
pub type RuntimeCorsPolicy = forge_cors::RuntimeCorsPolicy;

pub fn runtime_cors_policy(runtime_config: &RuntimeConfig) -> RuntimeCorsPolicy {
    let enabled = match runtime_config.get(CORS_ENABLED_KEY) {
        Some(raw) => match parse_bool_str(&raw) {
            Some(value) => value,
            None => {
                tracing::warn!(
                    key = CORS_ENABLED_KEY,
                    value = %raw,
                    "invalid runtime CORS enabled config; using safe default"
                );
                DEFAULT_CORS_ENABLED
            }
        },
        None => DEFAULT_CORS_ENABLED,
    };

    if !enabled {
        return RuntimeCorsPolicy {
            enabled: false,
            allowed_origins: CorsAllowedOrigins::None,
            allow_credentials: false,
            max_age_secs: DEFAULT_CORS_MAX_AGE_SECS,
        };
    }

    let allowed_origins_raw = runtime_config
        .get(CORS_ALLOWED_ORIGINS_KEY)
        .unwrap_or_default();
    let allowed_origins = match parse_allowed_origins_value(&allowed_origins_raw) {
        Ok(origins) => origins,
        Err(err) => {
            tracing::warn!(
                error = %err,
                key = CORS_ALLOWED_ORIGINS_KEY,
                value = %allowed_origins_raw,
                "invalid runtime CORS origins config; denying cross-origin requests"
            );
            CorsAllowedOrigins::None
        }
    };

    let allow_credentials = match runtime_config.get(CORS_ALLOW_CREDENTIALS_KEY) {
        Some(raw) => match parse_bool_str(&raw) {
            Some(value) => value,
            None => {
                tracing::warn!(
                    key = CORS_ALLOW_CREDENTIALS_KEY,
                    value = %raw,
                    "invalid runtime CORS credentials config; using safe default"
                );
                DEFAULT_CORS_ALLOW_CREDENTIALS
            }
        },
        None => DEFAULT_CORS_ALLOW_CREDENTIALS,
    };

    let max_age_secs = match runtime_config.get(CORS_MAX_AGE_SECS_KEY) {
        Some(raw) => match raw.trim().parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                tracing::warn!(
                    key = CORS_MAX_AGE_SECS_KEY,
                    value = %raw,
                    "invalid runtime CORS max_age config; using default"
                );
                DEFAULT_CORS_MAX_AGE_SECS
            }
        },
        None => DEFAULT_CORS_MAX_AGE_SECS,
    };

    if let Err(err) = validate_runtime_cors_combination(&allowed_origins, allow_credentials) {
        tracing::warn!(
            error = %err,
            "invalid runtime CORS policy combination; disabling CORS enforcement"
        );
        return RuntimeCorsPolicy {
            enabled,
            allowed_origins: CorsAllowedOrigins::None,
            allow_credentials: false,
            max_age_secs,
        };
    }

    RuntimeCorsPolicy {
        enabled,
        allowed_origins,
        allow_credentials,
        max_age_secs,
    }
}

pub fn normalize_enabled_config_value(value: &str) -> Result<String> {
    normalize_strict_bool_config_value(CORS_ENABLED_KEY, value).map_err(Into::into)
}

pub fn normalize_allowed_origins_config_value(value: &str) -> Result<String> {
    let parsed = parse_allowed_origins_value(value)?;
    Ok(match parsed {
        CorsAllowedOrigins::None => String::new(),
        CorsAllowedOrigins::Any => "*".to_string(),
        CorsAllowedOrigins::List(origins) => origins.join(","),
    })
}

pub fn normalize_allow_credentials_config_value(value: &str) -> Result<String> {
    normalize_strict_bool_config_value(CORS_ALLOW_CREDENTIALS_KEY, value).map_err(Into::into)
}

pub fn normalize_max_age_config_value(value: &str) -> Result<String> {
    normalize_non_negative_u64_config_value(CORS_MAX_AGE_SECS_KEY, value).map_err(Into::into)
}

pub fn parse_allowed_origins_value(value: &str) -> Result<CorsAllowedOrigins> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(CorsAllowedOrigins::None);
    }

    let mut origins = std::collections::BTreeSet::new();
    let mut wildcard = false;

    for raw_origin in trimmed.split(',') {
        let origin = raw_origin.trim();
        if origin.is_empty() {
            continue;
        }

        let normalized = normalize_origin(origin, true)?;
        if normalized == "*" {
            wildcard = true;
        } else {
            origins.insert(normalized);
        }
    }

    if wildcard && !origins.is_empty() {
        return Err(AsterError::validation_error(
            "cors_allowed_origins cannot mix '*' with explicit origins",
        ));
    }

    if wildcard {
        Ok(CorsAllowedOrigins::Any)
    } else if origins.is_empty() {
        Ok(CorsAllowedOrigins::None)
    } else {
        Ok(CorsAllowedOrigins::List(origins.into_iter().collect()))
    }
}

pub fn normalize_origin(origin: &str, allow_wildcard: bool) -> Result<String> {
    aster_forge_utils::url::normalize_origin(origin, allow_wildcard)
        .map_err(|error| AsterError::validation_error(error.to_string()))
}

pub fn validate_runtime_cors_combination(
    allowed_origins: &CorsAllowedOrigins,
    allow_credentials: bool,
) -> Result<()> {
    if matches!(allowed_origins, CorsAllowedOrigins::Any) && allow_credentials {
        return Err(AsterError::validation_error(
            "cors_allow_credentials cannot be true when cors_allowed_origins is '*'",
        ));
    }

    Ok(())
}

fn parse_bool_str(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::config::RuntimeConfig;
    use crate::config::definitions::CONFIG_CATEGORY_NETWORK_CORS;
    use crate::entities::system_config;

    use super::{
        CORS_ALLOW_CREDENTIALS_KEY, CORS_ALLOWED_ORIGINS_KEY, CORS_ENABLED_KEY,
        CORS_MAX_AGE_SECS_KEY, CorsAllowedOrigins, DEFAULT_CORS_ENABLED, DEFAULT_CORS_MAX_AGE_SECS,
        normalize_allow_credentials_config_value, normalize_allowed_origins_config_value,
        normalize_enabled_config_value, normalize_max_age_config_value, normalize_origin,
        parse_allowed_origins_value, runtime_cors_policy, validate_runtime_cors_combination,
    };

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 0,
            key: key.to_string(),
            value: value.to_string(),
            value_type: crate::types::SystemConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: crate::types::SystemConfigSource::System,
            visibility: crate::types::SystemConfigVisibility::Private,
            namespace: String::new(),
            category: CONFIG_CATEGORY_NETWORK_CORS.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn parse_empty_origins_as_none() {
        assert_eq!(
            parse_allowed_origins_value("   ").unwrap(),
            CorsAllowedOrigins::None
        );
    }

    #[test]
    fn normalize_origin_trims_trailing_slash_and_lowercases() {
        assert_eq!(
            normalize_origin(" HTTPS://Example.COM:8443/ ", false).unwrap(),
            "https://example.com:8443"
        );
    }

    #[test]
    fn parse_origin_list_deduplicates_and_sorts() {
        assert_eq!(
            normalize_allowed_origins_config_value(
                "https://b.example.com, https://a.example.com/, https://b.example.com"
            )
            .unwrap(),
            "https://a.example.com,https://b.example.com"
        );
    }

    #[test]
    fn reject_mixed_wildcard_and_explicit_origins() {
        let err = parse_allowed_origins_value("*,https://app.example.com").unwrap_err();
        assert!(
            err.message().contains(CORS_ALLOWED_ORIGINS_KEY)
                || err.message().contains("explicit origins")
        );
    }

    #[test]
    fn reject_wildcard_with_credentials() {
        let allowed = CorsAllowedOrigins::Any;
        let err = validate_runtime_cors_combination(&allowed, true).unwrap_err();
        assert!(err.message().contains("cors_allow_credentials"));
    }

    #[test]
    fn normalize_origin_rejects_path() {
        let err = normalize_origin("https://app.example.com/path", false).unwrap_err();
        assert!(err.message().contains("must not include a path"));
    }

    #[test]
    fn normalize_origin_rejects_query() {
        let err = normalize_origin("https://app.example.com?x=1", false).unwrap_err();
        assert!(err.message().contains("must not include query"));
    }

    #[test]
    fn normalize_origin_rejects_userinfo() {
        let err = normalize_origin("https://user@app.example.com", false).unwrap_err();
        assert!(err.message().contains("must not include userinfo"));
    }

    #[test]
    fn normalize_origin_rejects_non_http_scheme() {
        let err = normalize_origin("ftp://app.example.com", false).unwrap_err();
        assert!(err.message().contains("must use http or https"));
    }

    #[test]
    fn normalize_allow_credentials_rejects_invalid_value() {
        let err = normalize_allow_credentials_config_value("yes").unwrap_err();
        assert!(err.message().contains("true"));
    }

    #[test]
    fn normalize_enabled_rejects_invalid_value() {
        let err = normalize_enabled_config_value("yes").unwrap_err();
        assert!(err.message().contains("true"));
    }

    #[test]
    fn normalize_max_age_accepts_zero_and_rejects_negative() {
        assert_eq!(normalize_max_age_config_value(" 0 ").unwrap(), "0");
        let err = normalize_max_age_config_value("-1").unwrap_err();
        assert!(err.message().contains("non-negative integer"));
    }

    #[test]
    fn runtime_policy_invalid_boolean_uses_safe_default() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(CORS_ENABLED_KEY, "true"));
        runtime_config.apply(config_model(CORS_ALLOW_CREDENTIALS_KEY, "yes"));

        let policy = runtime_cors_policy(&runtime_config);
        assert!(!policy.allow_credentials);
    }

    #[test]
    fn runtime_policy_invalid_enabled_boolean_uses_safe_default() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(CORS_ENABLED_KEY, "yes"));

        let policy = runtime_cors_policy(&runtime_config);
        assert_eq!(policy.enabled, DEFAULT_CORS_ENABLED);
        assert!(!policy.enforces_requests());
    }

    #[test]
    fn runtime_policy_invalid_max_age_uses_default() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(CORS_ENABLED_KEY, "true"));
        runtime_config.apply(config_model(CORS_MAX_AGE_SECS_KEY, "abc"));

        let policy = runtime_cors_policy(&runtime_config);
        assert_eq!(policy.max_age_secs, DEFAULT_CORS_MAX_AGE_SECS);
    }

    #[test]
    fn runtime_policy_invalid_origin_config_fails_closed() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(CORS_ENABLED_KEY, "true"));
        runtime_config.apply(config_model(
            CORS_ALLOWED_ORIGINS_KEY,
            "https://app.example.com/path",
        ));

        let policy = runtime_cors_policy(&runtime_config);
        assert!(policy.enabled);
        assert_eq!(policy.allowed_origins, CorsAllowedOrigins::None);
        assert!(!policy.enforces_requests());
        assert!(!policy.allows_origin("https://app.example.com"));
    }

    #[test]
    fn runtime_policy_wildcard_with_credentials_downgrades_to_safe_policy() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(CORS_ENABLED_KEY, "true"));
        runtime_config.apply(config_model(CORS_ALLOWED_ORIGINS_KEY, "*"));
        runtime_config.apply(config_model(CORS_ALLOW_CREDENTIALS_KEY, "true"));

        let policy = runtime_cors_policy(&runtime_config);
        assert!(policy.enabled);
        assert_eq!(policy.allowed_origins, CorsAllowedOrigins::None);
        assert!(!policy.allow_credentials);
        assert!(!policy.enforces_requests());
    }
}
