//! Static configuration schema.
//!
//! Values in this module are loaded from the deployment configuration at
//! process startup. They are intentionally separate from `RuntimeConfig`, which
//! stores settings that can be edited through the admin API while the service
//! is running.

use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64};

use aster_forge_cache::CacheConfig;
use aster_forge_config::ConfigSyncConfig;
use aster_forge_logging::LoggingConfig;
use aster_forge_utils::numbers::{non_zero_u32, non_zero_u64};
use secrecy::SecretString;

pub const DEFAULT_AUTH_CSRF_COOKIE_NAME: &str = "aster_yggdrasil_csrf";
pub const DEFAULT_AUTH_CSRF_HEADER_NAME: &str = "X-Aster-Yggdrasil-CSRF";

/// Default per-process limit for concurrent Argon2 password hash/verify jobs,
/// sourced from `aster_forge_auth` so Aster product defaults stay coherent.
pub const DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY: usize =
    aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub deployment: crate::config::deployment::DeploymentConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default = "default_config_sync")]
    pub config_sync: ConfigSyncConfig,
    /// TOML/wire key remains `object_storage` for deployed configuration files.
    #[serde(default, rename = "object_storage", alias = "texture_storage")]
    pub texture_storage: TextureStorageConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub network_trust: NetworkTrustConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

fn default_config_sync() -> ConfigSyncConfig {
    ConfigSyncConfig {
        topic: "aster_yggdrasil.config_reload".to_string(),
        ..ConfigSyncConfig::default()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_host")]
    pub host: String,
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,
    /// `0` means use the number of CPUs.
    #[serde(default)]
    pub workers: usize,
    #[serde(default = "ServerConfig::default_temp_dir")]
    pub temp_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            workers: 0,
            temp_dir: Self::default_temp_dir(),
        }
    }
}

impl ServerConfig {
    fn default_host() -> String {
        "127.0.0.1".to_string()
    }
    fn default_port() -> u16 {
        3000
    }
    fn default_temp_dir() -> String {
        crate::utils::paths::DEFAULT_CONFIG_TEMP_DIR.to_string()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "DatabaseConfig::default_url")]
    pub url: aster_forge_db::DatabaseUrl,
    #[serde(default = "DatabaseConfig::default_pool_size")]
    pub pool_size: u32,
    #[serde(default = "DatabaseConfig::default_retry_count")]
    pub retry_count: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: Self::default_url(),
            pool_size: Self::default_pool_size(),
            retry_count: Self::default_retry_count(),
        }
    }
}

impl DatabaseConfig {
    fn default_url() -> aster_forge_db::DatabaseUrl {
        crate::utils::paths::DEFAULT_CONFIG_SQLITE_DATABASE_URL.into()
    }
    fn default_pool_size() -> u32 {
        10
    }
    fn default_retry_count() -> u32 {
        3
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    #[serde(default = "AuthConfig::default_jwt_secret")]
    pub jwt_secret: secrecy::SecretString,
    #[serde(default = "AuthConfig::default_mfa_secret_key")]
    pub mfa_secret_key: secrecy::SecretString,
    #[serde(default = "AuthConfig::default_csrf_cookie_name")]
    pub csrf_cookie_name: String,
    #[serde(default = "AuthConfig::default_csrf_header_name")]
    pub csrf_header_name: String,
    /// Per-process cap for concurrent Argon2 password hash/verify jobs.
    ///
    /// Each job reserves about 64 MiB of Argon2 working memory. Changing this
    /// value requires a restart.
    #[serde(default = "AuthConfig::default_password_hash_max_concurrency")]
    pub password_hash_max_concurrency: usize,
    /// Whether the first system_config seed should set auth_cookie_secure to false.
    #[serde(default = "AuthConfig::default_bootstrap_insecure_cookies")]
    pub bootstrap_insecure_cookies: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: Self::default_jwt_secret(),
            mfa_secret_key: Self::default_mfa_secret_key(),
            csrf_cookie_name: Self::default_csrf_cookie_name(),
            csrf_header_name: Self::default_csrf_header_name(),
            password_hash_max_concurrency: Self::default_password_hash_max_concurrency(),
            bootstrap_insecure_cookies: Self::default_bootstrap_insecure_cookies(),
        }
    }
}

// Manual impl: the default `config.toml` writer needs the real values (a
// generated secret must land in the file), while `Debug` stays redacted via
// `SecretString`.
impl serde::Serialize for AuthConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use secrecy::ExposeSecret as _;
        use serde::ser::SerializeStruct as _;
        let mut state = serializer.serialize_struct("AuthConfig", 6)?;
        state.serialize_field("jwt_secret", self.jwt_secret.expose_secret())?;
        state.serialize_field("mfa_secret_key", self.mfa_secret_key.expose_secret())?;
        state.serialize_field("csrf_cookie_name", &self.csrf_cookie_name)?;
        state.serialize_field("csrf_header_name", &self.csrf_header_name)?;
        state.serialize_field(
            "password_hash_max_concurrency",
            &self.password_hash_max_concurrency,
        )?;
        state.serialize_field(
            "bootstrap_insecure_cookies",
            &self.bootstrap_insecure_cookies,
        )?;
        state.end()
    }
}

impl AuthConfig {
    fn random_hex_secret() -> SecretString {
        use rand::RngExt;
        let mut rng = rand::rng();
        let bytes: [u8; 32] = rng.random();
        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        SecretString::from(hex)
    }
    fn default_jwt_secret() -> SecretString {
        Self::random_hex_secret()
    }
    fn default_mfa_secret_key() -> SecretString {
        Self::random_hex_secret()
    }
    fn default_csrf_cookie_name() -> String {
        DEFAULT_AUTH_CSRF_COOKIE_NAME.to_string()
    }
    fn default_csrf_header_name() -> String {
        DEFAULT_AUTH_CSRF_HEADER_NAME.to_string()
    }
    fn default_password_hash_max_concurrency() -> usize {
        DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY
    }
    fn default_bootstrap_insecure_cookies() -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextureStorageConfig {
    #[serde(default = "TextureStorageConfig::default_backend")]
    pub backend: String,
    #[serde(default = "TextureStorageConfig::default_local_root")]
    pub local_root: String,
    #[serde(default)]
    pub s3: S3TextureStorageConfig,
}

impl Default for TextureStorageConfig {
    fn default() -> Self {
        Self {
            backend: Self::default_backend(),
            local_root: Self::default_local_root(),
            s3: S3TextureStorageConfig::default(),
        }
    }
}

impl TextureStorageConfig {
    fn default_backend() -> String {
        "local".to_string()
    }

    fn default_local_root() -> String {
        "storage".to_string()
    }

    pub fn normalized_backend(&self) -> String {
        self.backend.trim().to_ascii_lowercase()
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct S3TextureStorageConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub bucket: String,
    /// Optional bucket prefix. Stored texture keys stay prefix-free in the
    /// database; the S3 backend prepends this only for object-storage calls.
    #[serde(default)]
    pub base_path: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub secret_access_key: secrecy::SecretString,
    #[serde(default)]
    pub force_path_style: bool,
}

// Manual impl: same rationale as `AuthConfig` — the default `config.toml`
// writer needs the real secret value, while `Debug` stays redacted.
impl serde::Serialize for S3TextureStorageConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use secrecy::ExposeSecret as _;
        use serde::ser::SerializeStruct as _;
        let mut state = serializer.serialize_struct("S3TextureStorageConfig", 7)?;
        state.serialize_field("endpoint", &self.endpoint)?;
        state.serialize_field("region", &self.region)?;
        state.serialize_field("bucket", &self.bucket)?;
        state.serialize_field("base_path", &self.base_path)?;
        state.serialize_field("access_key_id", &self.access_key_id)?;
        state.serialize_field("secret_access_key", self.secret_access_key.expose_secret())?;
        state.serialize_field("force_path_style", &self.force_path_style)?;
        state.end()
    }
}

/// Network trust configuration from `config.toml`.
///
/// Trusted proxy information affects real client IP detection for rate limiting,
/// authentication, audit logging, and other request-bound modules.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct NetworkTrustConfig {
    /// Trusted upstream proxy IP ranges, in CIDR or single-IP form.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

/// Rate limiting configuration.
///
/// Four tiers let request classes use different thresholds:
/// - `auth`: authentication and password verification.
/// - `public`: unauthenticated public endpoints.
/// - `api`: general authenticated reads and writes.
/// - `write`: expensive write or administrative operations.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimitConfig {
    #[serde(default = "RateLimitConfig::default_enabled")]
    pub enabled: bool,
    #[serde(default = "RateLimitConfig::default_auth")]
    pub auth: RateLimitTier,
    #[serde(default = "RateLimitConfig::default_public")]
    pub public: RateLimitTier,
    #[serde(default = "RateLimitConfig::default_api")]
    pub api: RateLimitTier,
    #[serde(default = "RateLimitConfig::default_write")]
    pub write: RateLimitTier,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: Self::default_enabled(),
            auth: Self::default_auth(),
            public: Self::default_public(),
            api: Self::default_api(),
            write: Self::default_write(),
        }
    }
}

impl RateLimitConfig {
    fn default_enabled() -> bool {
        true
    }
    fn default_auth() -> RateLimitTier {
        RateLimitTier {
            seconds_per_request: non_zero_u64(2),
            burst_size: non_zero_u32(5),
        }
    }
    fn default_public() -> RateLimitTier {
        RateLimitTier {
            seconds_per_request: non_zero_u64(1),
            burst_size: non_zero_u32(30),
        }
    }
    fn default_api() -> RateLimitTier {
        RateLimitTier {
            seconds_per_request: non_zero_u64(1),
            burst_size: non_zero_u32(120),
        }
    }
    fn default_write() -> RateLimitTier {
        RateLimitTier {
            seconds_per_request: non_zero_u64(2),
            burst_size: non_zero_u32(10),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimitTier {
    #[serde(default = "RateLimitTier::default_seconds")]
    pub seconds_per_request: NonZeroU64,
    #[serde(default = "RateLimitTier::default_burst")]
    pub burst_size: NonZeroU32,
}

impl Default for RateLimitTier {
    fn default() -> Self {
        Self {
            seconds_per_request: Self::default_seconds(),
            burst_size: Self::default_burst(),
        }
    }
}

impl RateLimitTier {
    fn default_seconds() -> NonZeroU64 {
        non_zero_u64(1)
    }
    fn default_burst() -> NonZeroU32 {
        non_zero_u32(60)
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, RateLimitConfig};

    #[test]
    fn rate_limit_defaults_to_enabled() {
        assert!(RateLimitConfig::default().enabled);
        assert!(Config::default().rate_limit.enabled);
    }

    #[test]
    fn auth_config_uses_forge_password_hash_concurrency_default() {
        let config = Config::default();
        assert_eq!(
            config.auth.password_hash_max_concurrency,
            aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY
        );
    }
}
