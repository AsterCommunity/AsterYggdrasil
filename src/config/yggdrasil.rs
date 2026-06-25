//! Runtime Yggdrasil protocol configuration.

use crate::config::{RuntimeConfig, site_url};
use crate::errors::{AsterError, Result};
use aster_forge_config::{
    normalize_positive_u64_config_value, parse_string_array_config_value,
    read_positive_u64 as forge_read_positive_u64,
};
use aster_forge_utils::url::HttpBaseUrlOptions;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use url::Url;

pub use crate::config::definitions::{
    YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY, YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
    YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY, YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY,
    YGGDRASIL_ENABLE_PROFILE_KEY_KEY, YGGDRASIL_MAX_ACTIVE_TOKENS_KEY,
    YGGDRASIL_MAX_TEXTURE_PIXELS_KEY, YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
    YGGDRASIL_PUBLIC_BASE_URL_KEY, YGGDRASIL_SERVER_NAME_KEY, YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
    YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY, YGGDRASIL_SKIN_DOMAINS_KEY,
    YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY, YGGDRASIL_TOKEN_TTL_DAYS_KEY,
};

pub const DEFAULT_YGGDRASIL_SERVER_NAME: &str = "AsterYggdrasil";
pub const DEFAULT_YGGDRASIL_API_ROOT: &str = "/api/yggdrasil";
pub const DEFAULT_YGGDRASIL_API_ROOT_ALI: &str = "/api/yggdrasil/";
pub const DEFAULT_YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN: bool = true;
pub const DEFAULT_YGGDRASIL_ALLOW_SKIN_UPLOAD: bool = true;
pub const DEFAULT_YGGDRASIL_ALLOW_CAPE_UPLOAD: bool = true;
pub const DEFAULT_YGGDRASIL_ENABLE_PROFILE_KEY: bool = true;
pub const DEFAULT_YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES: bool = true;
pub const DEFAULT_YGGDRASIL_TOKEN_TTL_DAYS: u64 = 15;
pub const DEFAULT_YGGDRASIL_MAX_ACTIVE_TOKENS: u64 = 10;
pub const DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES: u64 = 4 * 1024 * 1024;
pub const DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS: u64 = 4096 * 4096;
pub const DEFAULT_YGGDRASIL_SKIN_DOMAINS: &[&str] = &[".minecraft.net", ".mojang.com"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeYggdrasilPolicy {
    pub server_name: String,
    pub allow_profile_name_login: bool,
    pub allow_skin_upload: bool,
    pub allow_cape_upload: bool,
    pub enable_profile_key: bool,
    pub enable_mojang_anti_features: bool,
    pub token_ttl_days: u64,
    pub max_active_tokens: u64,
    pub max_texture_upload_bytes: u64,
    pub max_texture_pixels: u64,
    pub skin_domains: Vec<String>,
    pub public_base_urls: Vec<String>,
    pub texture_public_base_url: Option<String>,
    pub signature_public_key: String,
    pub signature_private_key: String,
}

impl RuntimeYggdrasilPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let public_base_urls = read_effective_public_base_urls(runtime_config);
        let texture_public_base_url = read_texture_public_base_url(runtime_config);
        let skin_domains = read_effective_skin_domains(
            runtime_config,
            &public_base_urls,
            &texture_public_base_url,
        );
        Self {
            server_name: runtime_config
                .get_string_or(YGGDRASIL_SERVER_NAME_KEY, DEFAULT_YGGDRASIL_SERVER_NAME),
            allow_profile_name_login: runtime_config.get_bool_or(
                YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
                DEFAULT_YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN,
            ),
            allow_skin_upload: runtime_config.get_bool_or(
                YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY,
                DEFAULT_YGGDRASIL_ALLOW_SKIN_UPLOAD,
            ),
            allow_cape_upload: runtime_config.get_bool_or(
                YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY,
                DEFAULT_YGGDRASIL_ALLOW_CAPE_UPLOAD,
            ),
            enable_profile_key: runtime_config.get_bool_or(
                YGGDRASIL_ENABLE_PROFILE_KEY_KEY,
                DEFAULT_YGGDRASIL_ENABLE_PROFILE_KEY,
            ),
            enable_mojang_anti_features: runtime_config.get_bool_or(
                YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY,
                DEFAULT_YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES,
            ),
            token_ttl_days: forge_read_positive_u64(
                runtime_config,
                YGGDRASIL_TOKEN_TTL_DAYS_KEY,
                DEFAULT_YGGDRASIL_TOKEN_TTL_DAYS,
            ),
            max_active_tokens: forge_read_positive_u64(
                runtime_config,
                YGGDRASIL_MAX_ACTIVE_TOKENS_KEY,
                DEFAULT_YGGDRASIL_MAX_ACTIVE_TOKENS,
            ),
            max_texture_upload_bytes: forge_read_positive_u64(
                runtime_config,
                YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
                DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES,
            ),
            max_texture_pixels: forge_read_positive_u64(
                runtime_config,
                YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
                DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
            ),
            skin_domains,
            public_base_urls,
            texture_public_base_url,
            signature_public_key: runtime_config
                .get_string_or(YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY, ""),
            signature_private_key: runtime_config
                .get_string_or(YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY, ""),
        }
    }

    pub fn uploadable_textures_value(&self) -> String {
        let mut values = Vec::new();
        if self.allow_skin_upload {
            values.push("skin");
        }
        if self.allow_cape_upload {
            values.push("cape");
        }
        values.join(",")
    }
}

pub fn default_skin_domains_config() -> String {
    serde_json::to_string(DEFAULT_YGGDRASIL_SKIN_DOMAINS).unwrap_or_else(|error| {
        tracing::error!(%error, "failed to serialize default Yggdrasil skin domains");
        "[]".to_string()
    })
}

pub fn normalize_yggdrasil_config_value(key: &str, value: &str) -> Result<String> {
    match key {
        YGGDRASIL_PUBLIC_BASE_URL_KEY => normalize_public_base_url_config_value(value),
        YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY => {
            normalize_texture_public_base_url_config_value(value)
        }
        YGGDRASIL_SKIN_DOMAINS_KEY => normalize_skin_domains_config_value(value),
        YGGDRASIL_TOKEN_TTL_DAYS_KEY
        | YGGDRASIL_MAX_ACTIVE_TOKENS_KEY
        | YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY
        | YGGDRASIL_MAX_TEXTURE_PIXELS_KEY => {
            normalize_positive_u64_config_value(key, value).map_err(Into::into)
        }
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY => {
            validate_signature_private_key_config_value(value)?;
            Ok(value.trim().to_string())
        }
        YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY => {
            validate_signature_public_key_config_value(value)?;
            Ok(value.trim().to_string())
        }
        _ => Ok(value.to_string()),
    }
}

pub fn normalize_texture_public_base_url_config_value(value: &str) -> Result<String> {
    aster_forge_utils::url::normalize_http_base_url(
        value,
        "yggdrasil texture public base URL",
        HttpBaseUrlOptions::optional_without_query_fragment(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))
    .map(|normalized| normalized.unwrap_or_default())
}

pub fn normalize_public_base_url_config_value(value: &str) -> Result<String> {
    let urls = parse_string_array_config_value(value, YGGDRASIL_PUBLIC_BASE_URL_KEY)?
        .into_iter()
        .map(|value| normalize_required_public_base_url(&value))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .fold(Vec::new(), |mut urls, url| {
            if !urls.contains(&url) {
                urls.push(url);
            }
            urls
        });
    serde_json::to_string(&urls).map_err(|error| {
        AsterError::internal_error(format!(
            "failed to serialize Yggdrasil public base URLs: {error}"
        ))
    })
}

pub fn normalize_skin_domains_config_value(value: &str) -> Result<String> {
    let domains = parse_string_array_config_value(value, YGGDRASIL_SKIN_DOMAINS_KEY)?
        .into_iter()
        .map(|value| normalize_required_skin_domain(&value))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .fold(Vec::new(), |mut domains, domain| {
            if !domains.contains(&domain) {
                domains.push(domain);
            }
            domains
        });
    serde_json::to_string(&domains).map_err(|error| {
        AsterError::internal_error(format!(
            "failed to serialize Yggdrasil skin domains: {error}"
        ))
    })
}

pub fn validate_signature_private_key_config_value(value: &str) -> Result<()> {
    let pem = value.trim();
    if pem.is_empty() {
        return Err(AsterError::validation_error(
            "yggdrasil signature private key must be a non-empty RSA PEM",
        ));
    }
    parse_signature_private_key_pem(pem).map(|_| ())
}

pub fn validate_signature_public_key_config_value(value: &str) -> Result<()> {
    let pem = value.trim();
    if pem.is_empty() {
        return Ok(());
    }
    parse_signature_public_key_pem(pem).map(|_| ())
}

pub fn parse_signature_private_key_pem(pem: &str) -> Result<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
        .map_err(|error| {
            AsterError::validation_error(format!(
                "invalid yggdrasil signature private key PEM: {error}"
            ))
        })
}

pub fn parse_signature_public_key_pem(pem: &str) -> Result<RsaPublicKey> {
    RsaPublicKey::from_public_key_pem(pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(pem))
        .map_err(|error| {
            AsterError::validation_error(format!(
                "invalid yggdrasil signature public key PEM: {error}"
            ))
        })
}

pub fn public_base_url_hosts_missing_from_skin_domains(
    public_base_urls: &[String],
    skin_domains: &[String],
) -> Vec<String> {
    public_base_urls
        .iter()
        .filter_map(|base_url| Url::parse(base_url).ok())
        .filter_map(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
        .filter(|host| {
            !skin_domains
                .iter()
                .any(|domain| skin_domain_matches_host(domain, host))
        })
        .fold(Vec::new(), |mut missing, host| {
            if !missing.contains(&host) {
                missing.push(host);
            }
            missing
        })
}

fn read_string_array(runtime_config: &RuntimeConfig, key: &str) -> Vec<String> {
    runtime_config
        .get(key)
        .and_then(|raw| parse_string_array_config_value(&raw, key).ok())
        .unwrap_or_default()
}

fn read_effective_public_base_urls(runtime_config: &RuntimeConfig) -> Vec<String> {
    let configured = read_public_base_urls(runtime_config, YGGDRASIL_PUBLIC_BASE_URL_KEY);
    if !configured.is_empty() {
        return configured;
    }

    site_url::public_site_urls(runtime_config)
        .into_iter()
        .map(|origin| site_url::join_origin_and_path(&origin, DEFAULT_YGGDRASIL_API_ROOT))
        .fold(Vec::new(), |mut urls, url| {
            if !urls.contains(&url) {
                urls.push(url);
            }
            urls
        })
}

fn read_public_base_urls(runtime_config: &RuntimeConfig, key: &str) -> Vec<String> {
    read_string_array(runtime_config, key)
        .into_iter()
        .filter_map(|value| normalize_public_base_url(&value))
        .fold(Vec::new(), |mut urls, url| {
            if !urls.contains(&url) {
                urls.push(url);
            }
            urls
        })
}

fn read_texture_public_base_url(runtime_config: &RuntimeConfig) -> Option<String> {
    runtime_config
        .get(YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY)
        .and_then(
            |value| match normalize_texture_public_base_url_config_value(&value) {
                Ok(value) if !value.is_empty() => Some(value),
                Ok(_) => None,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        key = YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY,
                        "invalid yggdrasil texture public base URL; ignoring configured value"
                    );
                    None
                }
            },
        )
}

fn read_effective_skin_domains(
    runtime_config: &RuntimeConfig,
    public_base_urls: &[String],
    texture_public_base_url: &Option<String>,
) -> Vec<String> {
    DEFAULT_YGGDRASIL_SKIN_DOMAINS
        .iter()
        .map(|domain| (*domain).to_string())
        .chain(read_string_array(
            runtime_config,
            YGGDRASIL_SKIN_DOMAINS_KEY,
        ))
        .filter_map(|domain| normalize_skin_domain(&domain))
        .chain(
            public_base_urls
                .iter()
                .filter_map(|base_url| Url::parse(base_url).ok())
                .filter_map(|url| url.host_str().map(|host| host.to_ascii_lowercase())),
        )
        .chain(
            texture_public_base_url
                .iter()
                .filter_map(|base_url| Url::parse(base_url).ok())
                .filter_map(|url| url.host_str().map(|host| host.to_ascii_lowercase())),
        )
        .fold(Vec::new(), |mut domains, domain| {
            if !domains.contains(&domain) {
                domains.push(domain);
            }
            domains
        })
}

fn normalize_public_base_url(value: &str) -> Option<String> {
    match aster_forge_utils::url::normalize_http_base_url(
        value,
        "yggdrasil public base URL",
        HttpBaseUrlOptions::optional_without_query_fragment(),
    ) {
        Ok(normalized) => normalized,
        Err(error) => {
            tracing::warn!(
                error = %error,
                value = %value.trim(),
                key = YGGDRASIL_PUBLIC_BASE_URL_KEY,
                "invalid yggdrasil public base URL; ignoring entry"
            );
            None
        }
    }
}

fn normalize_skin_domain(value: &str) -> Option<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_required_public_base_url(value: &str) -> Result<String> {
    aster_forge_utils::url::normalize_http_base_url(
        value,
        "yggdrasil public base URL",
        HttpBaseUrlOptions::required_without_query_fragment(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))
    .and_then(|normalized| {
        normalized.ok_or_else(|| {
            AsterError::validation_error("yggdrasil public base URL cannot be empty")
        })
    })
}

fn normalize_required_skin_domain(value: &str) -> Result<String> {
    let Some(domain) = normalize_skin_domain(value) else {
        return Err(AsterError::validation_error(
            "yggdrasil skin domain entries cannot be empty",
        ));
    };
    validate_skin_domain_rule(&domain)?;
    Ok(domain)
}

fn validate_skin_domain_rule(domain: &str) -> Result<()> {
    if domain.contains("://")
        || domain.contains('/')
        || domain.contains(':')
        || domain.contains('*')
        || domain.chars().any(char::is_whitespace)
    {
        return Err(AsterError::validation_error(
            "yggdrasil skin domain must be a host rule, not a URL or wildcard",
        ));
    }
    let host = domain.strip_prefix('.').unwrap_or(domain);
    if host.is_empty() || host.starts_with('.') || host.ends_with('.') {
        return Err(AsterError::validation_error(
            "yggdrasil skin domain must contain a non-empty host",
        ));
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            return Err(AsterError::validation_error(
                "yggdrasil skin domain contains an invalid host label",
            ));
        }
    }
    Ok(())
}

fn skin_domain_matches_host(domain: &str, host: &str) -> bool {
    let domain = domain.trim().to_ascii_lowercase();
    let host = host.trim().to_ascii_lowercase();
    if let Some(suffix) = domain.strip_prefix('.') {
        return host.ends_with(&domain) && host.len() > suffix.len();
    }
    host == domain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::system_config;
    use crate::types::{SystemConfigSource, SystemConfigValueType, SystemConfigVisibility};

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: SystemConfigValueType::StringArray,
            requires_restart: false,
            is_sensitive: false,
            source: SystemConfigSource::System,
            visibility: SystemConfigVisibility::Private,
            namespace: String::new(),
            category: String::new(),
            description: String::new(),
            updated_at: chrono::Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn public_base_urls_preserve_paths_trim_slashes_and_dedupe() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            YGGDRASIL_PUBLIC_BASE_URL_KEY,
            r#"[
                " https://skin.example.test/yggdrasil/ ",
                "https://skin.example.test/yggdrasil",
                "http://localhost:8080/",
                "ftp://skin.example.test",
                "not-a-url",
                ""
            ]"#,
        ));

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.public_base_urls,
            vec![
                "https://skin.example.test/yggdrasil".to_string(),
                "http://localhost:8080".to_string(),
            ]
        );
    }

    #[test]
    fn invalid_public_base_url_config_is_ignored() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(YGGDRASIL_PUBLIC_BASE_URL_KEY, "not json"));

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert!(policy.public_base_urls.is_empty());
    }

    #[test]
    fn public_base_urls_fall_back_to_public_site_url_api_root() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            crate::config::site_url::PUBLIC_SITE_URL_KEY,
            r#"["https://skin.example.test","https://panel.example.test"]"#,
        ));

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.public_base_urls,
            vec![
                "https://skin.example.test/api/yggdrasil".to_string(),
                "https://panel.example.test/api/yggdrasil".to_string(),
            ]
        );
    }

    #[test]
    fn texture_public_base_url_is_optional_normalized_and_included_in_skin_domains() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY,
            " https://cdn.example.test/texture-root/ ",
        ));

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.texture_public_base_url.as_deref(),
            Some("https://cdn.example.test/texture-root")
        );
        assert!(
            policy
                .skin_domains
                .contains(&"cdn.example.test".to_string())
        );
        assert_eq!(
            normalize_texture_public_base_url_config_value("  ").unwrap(),
            ""
        );
        assert!(
            normalize_texture_public_base_url_config_value("https://cdn.example.test/root?x=1")
                .is_err()
        );
        assert!(
            normalize_texture_public_base_url_config_value("https://cdn.example.test/root#frag")
                .is_err()
        );
        assert!(normalize_texture_public_base_url_config_value("ftp://cdn.example.test").is_err());
    }

    #[test]
    fn uploadable_textures_follow_runtime_switches() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(system_config::Model {
            value_type: SystemConfigValueType::Boolean,
            ..config_model(YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY, "false")
        });
        runtime_config.apply(system_config::Model {
            id: 2,
            value_type: SystemConfigValueType::Boolean,
            ..config_model(YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY, "true")
        });

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.uploadable_textures_value(), "cape");
    }

    #[test]
    fn minecraft_services_capability_flags_default_enabled_and_follow_runtime_switches() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);
        assert!(policy.enable_profile_key);
        assert!(policy.enable_mojang_anti_features);

        runtime_config.apply(system_config::Model {
            value_type: SystemConfigValueType::Boolean,
            ..config_model(YGGDRASIL_ENABLE_PROFILE_KEY_KEY, "false")
        });
        runtime_config.apply(system_config::Model {
            id: 2,
            value_type: SystemConfigValueType::Boolean,
            ..config_model(YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY, "false")
        });

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);
        assert!(!policy.enable_profile_key);
        assert!(!policy.enable_mojang_anti_features);
    }

    #[test]
    fn texture_upload_limits_use_defaults_and_validate_raw_values() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.max_texture_upload_bytes,
            DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES
        );
        assert_eq!(
            policy.max_texture_pixels,
            DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS
        );
        assert_eq!(
            normalize_positive_u64_config_value(YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY, "1")
                .unwrap(),
            "1"
        );
        assert_eq!(
            normalize_positive_u64_config_value(YGGDRASIL_MAX_TEXTURE_PIXELS_KEY, "4096").unwrap(),
            "4096"
        );
        assert!(
            normalize_positive_u64_config_value(YGGDRASIL_MAX_TEXTURE_PIXELS_KEY, "0").is_err()
        );
    }

    #[test]
    fn token_limits_require_positive_integer_values() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(YGGDRASIL_TOKEN_TTL_DAYS_KEY, "0"));
        runtime_config.apply(config_model(YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, "1.5"));
        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.token_ttl_days, DEFAULT_YGGDRASIL_TOKEN_TTL_DAYS);
        assert_eq!(
            policy.max_active_tokens,
            DEFAULT_YGGDRASIL_MAX_ACTIVE_TOKENS
        );
        assert_eq!(
            normalize_yggdrasil_config_value(YGGDRASIL_TOKEN_TTL_DAYS_KEY, "7").unwrap(),
            "7"
        );
        assert_eq!(
            normalize_yggdrasil_config_value(YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, "10").unwrap(),
            "10"
        );
        assert!(normalize_yggdrasil_config_value(YGGDRASIL_TOKEN_TTL_DAYS_KEY, "0").is_err());
        assert!(normalize_yggdrasil_config_value(YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, "1.5").is_err());
    }

    #[test]
    fn skin_domains_include_official_sources_and_texture_hosts() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            YGGDRASIL_SKIN_DOMAINS_KEY,
            r#"[" .Minecraft.net ","custom.example.test","custom.example.test",""]"#,
        ));
        runtime_config.apply(config_model(
            YGGDRASIL_PUBLIC_BASE_URL_KEY,
            r#"[
                "https://skin.example.test/yggdrasil/",
                "https://custom.example.test/textures"
            ]"#,
        ));

        let policy = RuntimeYggdrasilPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.skin_domains,
            vec![
                ".minecraft.net".to_string(),
                ".mojang.com".to_string(),
                "custom.example.test".to_string(),
                "skin.example.test".to_string(),
            ]
        );
    }

    #[test]
    fn yggdrasil_config_normalizers_validate_and_dedupe_values() {
        assert_eq!(
            normalize_public_base_url_config_value(
                r#"[" https://skin.example.test/yggdrasil/ ","https://skin.example.test/yggdrasil"]"#
            )
            .unwrap(),
            r#"["https://skin.example.test/yggdrasil"]"#
        );
        assert!(normalize_public_base_url_config_value(r#"["ftp://skin.example.test"]"#).is_err());
        assert!(
            normalize_public_base_url_config_value(r#"["https://skin.example.test/root?x=1"]"#)
                .is_err()
        );
        assert!(
            normalize_public_base_url_config_value(r#"["https://skin.example.test/root#frag"]"#)
                .is_err()
        );

        assert_eq!(
            normalize_skin_domains_config_value(
                r#"[" .Minecraft.net ","skin.example.test","skin.example.test"]"#
            )
            .unwrap(),
            r#"[".minecraft.net","skin.example.test"]"#
        );
        assert!(normalize_skin_domains_config_value(r#"["https://skin.example.test"]"#).is_err());
    }

    #[test]
    fn public_base_url_host_diagnostics_respect_exact_and_suffix_skin_domain_rules() {
        assert_eq!(
            public_base_url_hosts_missing_from_skin_domains(
                &[
                    "https://skin.example.test/yggdrasil".to_string(),
                    "https://cdn.asset.test".to_string(),
                    "https://deep.asset.test".to_string(),
                ],
                &["skin.example.test".to_string(), ".asset.test".to_string()]
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            public_base_url_hosts_missing_from_skin_domains(
                &["https://example.test/yggdrasil".to_string()],
                &[".example.test".to_string()]
            ),
            vec!["example.test".to_string()]
        );
    }
}
