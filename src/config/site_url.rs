//! Public site URL runtime configuration helpers.
//!
//! Yggdrasil owns the runtime config key and logging context for public site
//! origins. The parsing, normalization, request-origin preference, and URL
//! joining mechanics are shared through `aster_forge_utils`.

use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};

pub use crate::config::definitions::PUBLIC_SITE_URL_KEY;

fn map_url_error(error: aster_forge_utils::UtilsError) -> AsterError {
    AsterError::validation_error(error.to_string())
}

/// Validates a stored `public_site_url` JSON array of origins: http(s) only,
/// no paths, wildcards, or duplicate hosts; lowercases and preserves order.
pub fn normalize_public_site_url_config_value(value: &str) -> Result<String> {
    aster_forge_utils::url::normalize_public_site_origins_config_value(value).map_err(map_url_error)
}

/// Parses a stored `public_site_url` JSON array into normalized origins,
/// dropping blank entries.
pub fn parse_public_site_url_value(value: &str) -> Result<Vec<String>> {
    aster_forge_utils::url::parse_public_site_origins(value).map_err(map_url_error)
}

/// Returns the raw `public_site_url` runtime value, trimmed; `None` when unset
/// or blank.
pub fn public_site_url_config_value(runtime_config: &RuntimeConfig) -> Option<String> {
    runtime_config
        .get(PUBLIC_SITE_URL_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Returns the configured public origins, ignoring invalid entries (and
/// warning about them) so one bad origin cannot break URL building.
pub fn public_site_urls(runtime_config: &RuntimeConfig) -> Vec<String> {
    aster_forge_utils::url::runtime_public_site_origins_with(
        public_site_url_config_value(runtime_config).as_deref(),
        |entry, err| match entry {
            Some(entry) => {
                tracing::warn!(
                    error = %err,
                    key = PUBLIC_SITE_URL_KEY,
                    entry = %entry,
                    "invalid runtime public_site_url origin; ignoring entry"
                );
            }
            None => {
                tracing::warn!(
                    error = %err,
                    key = PUBLIC_SITE_URL_KEY,
                    "invalid runtime public_site_url config; ignoring configured public origins"
                );
            }
        },
    )
}

/// Returns the first configured public origin, or `None` when none is set.
pub fn public_site_url(runtime_config: &RuntimeConfig) -> Option<String> {
    public_site_urls(runtime_config).into_iter().next()
}

/// Picks the configured origin matching the request's scheme and host,
/// falling back to the first configured origin when no entry matches.
pub fn public_site_url_for_request(
    runtime_config: &RuntimeConfig,
    scheme: &str,
    host: &str,
) -> Option<String> {
    aster_forge_utils::url::public_site_origin_for_request(
        &public_site_urls(runtime_config),
        scheme,
        host,
    )
}

/// Joins the first configured public origin with `path`; `None` when no
/// public origin is configured.
pub fn public_app_url(runtime_config: &RuntimeConfig, path: &str) -> Option<String> {
    let base = public_site_url(runtime_config)?;
    Some(join_origin_and_path(&base, path))
}

/// Joins the request-matched public origin with `path`; `None` when no origin
/// applies.
pub fn public_app_url_for_request(
    runtime_config: &RuntimeConfig,
    path: &str,
    scheme: &str,
    host: &str,
) -> Option<String> {
    let base = public_site_url_for_request(runtime_config, scheme, host)?;
    Some(join_origin_and_path(&base, path))
}

/// Like [`public_app_url`] but falls back to the bare (root-relative) `path`
/// when no public origin is configured, so links stay functional in-app.
pub fn public_app_url_or_path(runtime_config: &RuntimeConfig, path: &str) -> String {
    public_app_url(runtime_config, path).unwrap_or_else(|| path.to_string())
}

/// Like [`public_app_url_for_request`] but falls back to the bare
/// (root-relative) `path` when no origin matches the request.
pub fn public_app_url_or_path_for_request(
    runtime_config: &RuntimeConfig,
    path: &str,
    scheme: &str,
    host: &str,
) -> String {
    public_app_url_for_request(runtime_config, path, scheme, host)
        .unwrap_or_else(|| path.to_string())
}

/// Joins a configured origin with a root-relative path, ensuring exactly one
/// slash between the two.
pub fn join_origin_and_path(base: &str, path: &str) -> String {
    aster_forge_utils::url::join_origin_and_path(base, path)
}

#[cfg(test)]
mod tests {
    use super::{
        PUBLIC_SITE_URL_KEY, normalize_public_site_url_config_value, parse_public_site_url_value,
        public_app_url, public_app_url_for_request, public_site_url, public_site_url_for_request,
        public_site_urls,
    };
    use crate::config::RuntimeConfig;
    use aster_forge_db::system_config;
    use chrono::Utc;

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: aster_forge_config::ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: aster_forge_config::ConfigSource::System,
            visibility: aster_forge_config::ConfigVisibility::Private,
            namespace: String::new(),
            category: crate::config::definitions::CONFIG_CATEGORY_SITE_PUBLIC.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn normalize_public_site_url_accepts_empty_and_valid_origins() {
        assert!(normalize_public_site_url_config_value("   ").is_err());
        assert_eq!(
            normalize_public_site_url_config_value(r#"[" HTTPS://Forge.EXAMPLE.com/ "]"#).unwrap(),
            r#"["https://forge.example.com"]"#
        );
        assert_eq!(
            normalize_public_site_url_config_value(r#"["http://forge.example.com:8080"]"#).unwrap(),
            r#"["http://forge.example.com:8080"]"#
        );
    }

    #[test]
    fn normalize_public_site_url_accepts_ordered_origin_lists() {
        assert_eq!(
            normalize_public_site_url_config_value(
                r#"[" HTTPS://Forge.EXAMPLE.com/ ","https://Panel.example.com","https://forge.example.com"]"#
            )
            .unwrap(),
            r#"["https://forge.example.com","https://panel.example.com"]"#
        );
        assert_eq!(
            normalize_public_site_url_config_value(
                r#"["https://forge.example.com","https://panel.example.com"]"#
            )
            .unwrap(),
            r#"["https://forge.example.com","https://panel.example.com"]"#
        );
        assert_eq!(
            parse_public_site_url_value(
                r#"["https://forge.example.com","","https://api.example.com"]"#
            )
            .unwrap(),
            vec![
                "https://forge.example.com".to_string(),
                "https://api.example.com".to_string()
            ]
        );
    }

    #[test]
    fn normalize_public_site_url_rejects_paths_and_non_http_schemes() {
        assert!(
            normalize_public_site_url_config_value(r#"["https://forge.example.com/app"]"#).is_err()
        );
        assert!(normalize_public_site_url_config_value(r#"["ftp://forge.example.com"]"#).is_err());
        assert!(normalize_public_site_url_config_value(r#"["*.example.com"]"#).is_err());
        assert!(normalize_public_site_url_config_value(r#"["*"]"#).is_err());
        assert!(normalize_public_site_url_config_value(r#""https://forge.example.com""#).is_err());
    }

    #[test]
    fn public_app_url_joins_configured_origin_with_root_paths() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            PUBLIC_SITE_URL_KEY,
            r#"["https://forge.example.com"]"#,
        ));

        assert_eq!(
            public_site_url(&runtime_config).as_deref(),
            Some("https://forge.example.com")
        );
        assert_eq!(
            public_site_urls(&runtime_config),
            vec!["https://forge.example.com".to_string()]
        );
        assert_eq!(
            public_app_url(&runtime_config, "/admin/settings").as_deref(),
            Some("https://forge.example.com/admin/settings")
        );
    }

    #[test]
    fn public_site_url_for_request_uses_matching_configured_origin_only() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            PUBLIC_SITE_URL_KEY,
            r#"["https://forge.example.com","https://panel.example.com"]"#,
        ));

        assert_eq!(
            public_site_url_for_request(&runtime_config, "https", "panel.example.com").as_deref(),
            Some("https://panel.example.com")
        );
        assert_eq!(
            public_site_url_for_request(&runtime_config, "https", "evil.example.com").as_deref(),
            Some("https://forge.example.com")
        );
        assert_eq!(
            public_app_url_for_request(
                &runtime_config,
                "admin/settings",
                "https",
                "panel.example.com"
            )
            .as_deref(),
            Some("https://panel.example.com/admin/settings")
        );
    }

    #[test]
    fn public_site_urls_ignores_invalid_runtime_entries_individually() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            PUBLIC_SITE_URL_KEY,
            r#"["https://forge.example.com","https://panel.example.com/app","https://api.example.com"]"#,
        ));

        assert_eq!(
            public_site_urls(&runtime_config),
            vec![
                "https://forge.example.com".to_string(),
                "https://api.example.com".to_string(),
            ]
        );
    }
}
