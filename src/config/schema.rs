//! Static configuration schema.
//!
//! Values in this module are loaded from the deployment configuration at
//! process startup. They are intentionally separate from `RuntimeConfig`, which
//! stores settings that can be edited through the admin API while the service
//! is running.

use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64};

use aster_forge_utils::numbers::{non_zero_u32, non_zero_u64};

pub const DEFAULT_AUTH_CSRF_COOKIE_NAME: &str = "aster_yggdrasil_csrf";
pub const DEFAULT_AUTH_CSRF_HEADER_NAME: &str = "X-Aster-Yggdrasil-CSRF";

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default, alias = "texture_storage")]
    pub object_storage: ObjectStorageConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub network_trust: NetworkTrustConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
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
    #[serde(default)]
    pub follower: ServerFollowerConfig,
    /// Static node role selected at startup. Changing it requires a process restart.
    #[serde(default)]
    pub start_mode: crate::config::node_mode::NodeRuntimeMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerFollowerConfig {
    /// Root directory for follower local ingress profiles managed by the primary.
    /// Local destinations pushed by the primary must be relative paths under this root.
    #[serde(default = "ServerFollowerConfig::default_managed_ingress_local_root")]
    pub managed_ingress_local_root: String,
}

impl Default for ServerFollowerConfig {
    fn default() -> Self {
        Self {
            managed_ingress_local_root: Self::default_managed_ingress_local_root(),
        }
    }
}

impl ServerFollowerConfig {
    fn default_managed_ingress_local_root() -> String {
        "managed-ingress".to_string()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Self::default_host(),
            port: Self::default_port(),
            workers: 0,
            temp_dir: Self::default_temp_dir(),
            follower: ServerFollowerConfig::default(),
            start_mode: crate::config::node_mode::NodeRuntimeMode::Primary,
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
    pub url: String,
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
    fn default_url() -> String {
        crate::utils::paths::DEFAULT_CONFIG_SQLITE_DATABASE_URL.to_string()
    }
    fn default_pool_size() -> u32 {
        10
    }
    fn default_retry_count() -> u32 {
        3
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthConfig {
    #[serde(default = "AuthConfig::default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "AuthConfig::default_mfa_secret_key")]
    pub mfa_secret_key: String,
    #[serde(default = "AuthConfig::default_csrf_cookie_name")]
    pub csrf_cookie_name: String,
    #[serde(default = "AuthConfig::default_csrf_header_name")]
    pub csrf_header_name: String,
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
            bootstrap_insecure_cookies: Self::default_bootstrap_insecure_cookies(),
        }
    }
}

impl AuthConfig {
    fn random_hex_secret() -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        let bytes: [u8; 32] = rng.random();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
    fn default_jwt_secret() -> String {
        Self::random_hex_secret()
    }
    fn default_mfa_secret_key() -> String {
        Self::random_hex_secret()
    }
    fn default_csrf_cookie_name() -> String {
        DEFAULT_AUTH_CSRF_COOKIE_NAME.to_string()
    }
    fn default_csrf_header_name() -> String {
        DEFAULT_AUTH_CSRF_HEADER_NAME.to_string()
    }
    fn default_bootstrap_insecure_cookies() -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "CacheConfig::default_backend")]
    pub backend: String, // "memory" | "redis"
    #[serde(default)]
    pub redis_url: String,
    #[serde(default = "CacheConfig::default_ttl")]
    pub default_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            backend: Self::default_backend(),
            redis_url: String::new(),
            default_ttl: Self::default_ttl(),
        }
    }
}

impl CacheConfig {
    fn default_backend() -> String {
        "memory".to_string()
    }
    fn default_ttl() -> u64 {
        3600
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ObjectStorageConfig {
    #[serde(default = "ObjectStorageConfig::default_backend")]
    pub backend: String,
    #[serde(default = "ObjectStorageConfig::default_local_root")]
    pub local_root: String,
    #[serde(default)]
    pub s3: S3ObjectStorageConfig,
}

impl Default for ObjectStorageConfig {
    fn default() -> Self {
        Self {
            backend: Self::default_backend(),
            local_root: Self::default_local_root(),
            s3: S3ObjectStorageConfig::default(),
        }
    }
}

impl ObjectStorageConfig {
    fn default_backend() -> String {
        "local".to_string()
    }

    fn default_local_root() -> String {
        "storage".to_string()
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct S3ObjectStorageConfig {
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
    pub secret_access_key: String,
    #[serde(default)]
    pub force_path_style: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "LoggingConfig::default_level")]
    pub level: String,
    #[serde(default = "LoggingConfig::default_format")]
    pub format: String, // "text" | "json"
    #[serde(default)]
    pub file: String, // Empty means stdout only.
    /// Enables daily log rotation when file logging is configured.
    #[serde(default = "LoggingConfig::default_enable_rotation")]
    pub enable_rotation: bool,
    /// Number of historical log files to retain.
    #[serde(default = "LoggingConfig::default_max_backups")]
    pub max_backups: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            format: Self::default_format(),
            file: String::new(),
            enable_rotation: Self::default_enable_rotation(),
            max_backups: Self::default_max_backups(),
        }
    }
}

impl LoggingConfig {
    fn default_level() -> String {
        "info".to_string()
    }
    fn default_format() -> String {
        "text".to_string()
    }
    fn default_enable_rotation() -> bool {
        true
    }
    fn default_max_backups() -> u32 {
        5
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
}
