//! Static configuration schema.
//!
//! Values in this module are loaded from the deployment configuration at
//! process startup. They are intentionally separate from `RuntimeConfig`, which
//! stores settings that can be edited through the admin API while the service
//! is running.

use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64};

use aster_forge_cache::CacheConfig;
use aster_forge_config::{ConfigCoreError, ConfigSyncConfig, ValidatableConfig};
use aster_forge_logging::LoggingConfig;
use aster_forge_metrics::ObservabilityConfig;
use aster_forge_utils::numbers::{non_zero_u32, non_zero_u64};
use aster_yggdrasil_texture_storage::TextureStorageConfig;
use secrecy::SecretString;

/// Fixed CSRF cookie name shared across Aster products (Drive uses the same
/// name); cookies are host-only, so same-named cookies cannot collide across
/// deployment hosts.
pub const DEFAULT_AUTH_CSRF_COOKIE_NAME: &str = "aster_csrf";
/// Fixed CSRF request header name aligned with AsterDrive and common backend
/// framework conventions.
pub const DEFAULT_AUTH_CSRF_HEADER_NAME: &str = "X-CSRF-Token";

/// Default per-process limit for concurrent Argon2 password hash/verify jobs,
/// sourced from `aster_forge_auth` so Aster product defaults stay coherent.
pub const DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY: usize =
    aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY;

/// Root of the static TOML configuration file. Every section defaults so a
/// minimal file boots; only startup-time settings live here, while
/// runtime-editable settings are served from the `system_config` table.
#[derive(Debug, Deserialize, Serialize, Clone, Default, aster_forge_config::ValidateConfig)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Deployment topology and its shared-dependency requirements.
    #[serde(default)]
    pub deployment: crate::config::deployment::DeploymentConfig,
    /// HTTP bind address, worker count, and temp directory.
    #[serde(default)]
    pub server: ServerConfig,
    /// Graceful shutdown budgets for background workers.
    #[serde(default)]
    pub worker: crate::config::worker::WorkerConfig,
    /// Database URL, pool size, and connect retry count.
    #[serde(default)]
    pub database: DatabaseConfig,
    /// JWT secrets, CSRF names, and password-hash concurrency.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Cache backend selection and connection settings.
    #[serde(default)]
    pub cache: CacheConfig,
    /// Pub/sub backend used to propagate runtime config reloads between
    /// instances; defaults to the `aster_yggdrasil.config_reload` topic.
    #[serde(default = "default_config_sync")]
    pub config_sync: ConfigSyncConfig,
    /// Where texture files are stored (local disk or S3/MinIO).
    /// TOML/wire key remains `object_storage` for deployed configuration files.
    #[serde(default, rename = "object_storage", alias = "texture_storage")]
    pub texture_storage: TextureStorageConfig,
    /// Log format, level, and output wiring.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Metrics/observability exporter settings; loading tolerates a missing
    /// or partially specified section.
    #[serde(
        default,
        deserialize_with = "crate::config::metrics::deserialize_observability"
    )]
    pub observability: ObservabilityConfig,
    /// Trusted proxy ranges used to recover real client IPs.
    #[serde(default)]
    pub network_trust: NetworkTrustConfig,
    /// Per-tier rate limit thresholds and global switch.
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl ValidatableConfig for Config {
    /// Cross-section rules only: section invariants run through the generated
    /// `ValidateConfig` pass before this.
    fn validate(&self) -> aster_forge_config::Result<()> {
        let issues = crate::config::deployment::static_issues(self);
        if issues.is_empty() {
            return Ok(());
        }
        Err(ConfigCoreError::invalid_value(format!(
            "invalid deployment profile '{}': {}",
            self.deployment.profile.as_str(),
            issues.join("; ")
        )))
    }
}

fn default_config_sync() -> ConfigSyncConfig {
    ConfigSyncConfig {
        topic: "aster_yggdrasil.config_reload".to_string(),
        ..ConfigSyncConfig::default()
    }
}

/// `[server]` section: HTTP listener and per-process scratch settings.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    /// Bind address; defaults to `127.0.0.1` so a fresh install is not
    /// exposed on all interfaces.
    #[serde(default = "ServerConfig::default_host")]
    pub host: String,
    /// Bind port; defaults to 3000.
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,
    /// `0` means use the number of CPUs.
    #[serde(default)]
    pub workers: usize,
    /// Scratch directory for uploads and other transient files; relative
    /// values resolve against the instance root at load time. Defaults to
    /// `<data>/.tmp`.
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

impl ValidatableConfig for ServerConfig {
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
}

/// `[database]` section: connection target and pool sizing.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    /// Database connection URL (SQLite by default, MySQL/PostgreSQL supported);
    /// relative sqlite file paths resolve against the instance root. Defaults
    /// to `sqlite://data/asteryggdrasil.db?mode=rwc`.
    #[serde(default = "DatabaseConfig::default_url")]
    pub url: aster_forge_db::DatabaseUrl,
    /// Maximum connections in the pool; defaults to 10.
    #[serde(default = "DatabaseConfig::default_pool_size")]
    pub pool_size: u32,
    /// Connection attempts before giving up at startup; defaults to 3.
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

impl ValidatableConfig for DatabaseConfig {
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
}

/// `[auth]` section: secrets and auth-related naming set only at startup.
#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    /// HMAC secret for signing and verifying JWTs. Generated on first startup
    /// and persisted into the config file; rotating it invalidates every
    /// outstanding token.
    #[serde(default = "AuthConfig::default_jwt_secret")]
    pub jwt_secret: secrecy::SecretString,
    /// Symmetric secret for MFA-related key derivation. Generated on first
    /// startup and persisted into the config file.
    #[serde(default = "AuthConfig::default_mfa_secret_key")]
    pub mfa_secret_key: secrecy::SecretString,
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
        let mut state = serializer.serialize_struct("AuthConfig", 4)?;
        state.serialize_field("jwt_secret", self.jwt_secret.expose_secret())?;
        state.serialize_field("mfa_secret_key", self.mfa_secret_key.expose_secret())?;
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
    fn default_password_hash_max_concurrency() -> usize {
        DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY
    }
    fn default_bootstrap_insecure_cookies() -> bool {
        false
    }
}

impl ValidatableConfig for AuthConfig {
    fn validate(&self) -> aster_forge_config::Result<()> {
        if self.password_hash_max_concurrency == 0 {
            return Err(ConfigCoreError::invalid_value(
                "auth.password_hash_max_concurrency must be positive",
            ));
        }
        Ok(())
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

impl ValidatableConfig for NetworkTrustConfig {
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
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
    /// Master switch; defaults to true so authentication endpoints are
    /// protected out of the box.
    #[serde(default = "RateLimitConfig::default_enabled")]
    pub enabled: bool,
    /// Tier for authentication and password verification; defaults to 2
    /// seconds per request with a burst of 5.
    #[serde(default = "RateLimitConfig::default_auth")]
    pub auth: RateLimitTier,
    /// Tier for unauthenticated public endpoints; defaults to 1 second per
    /// request with a burst of 30.
    #[serde(default = "RateLimitConfig::default_public")]
    pub public: RateLimitTier,
    /// Tier for general authenticated reads and writes; defaults to 1 second
    /// per request with a burst of 120.
    #[serde(default = "RateLimitConfig::default_api")]
    pub api: RateLimitTier,
    /// Tier for expensive write or administrative operations; defaults to 2
    /// seconds per request with a burst of 10.
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

impl ValidatableConfig for RateLimitConfig {
    /// Tier bounds are enforced by the `NonZero` field types.
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
}

/// One rate limit tier: a refill interval plus a bucket depth.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimitTier {
    /// Sustained rate: at least this many seconds must pass between requests
    /// for the bucket to fully refill. Must be non-zero.
    #[serde(default = "RateLimitTier::default_seconds")]
    pub seconds_per_request: NonZeroU64,
    /// Allowed short-term burst on top of the sustained rate, in requests.
    /// Must be non-zero.
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

    /// A zero Argon2 budget is a startup-blocking section failure: the loader
    /// pipeline must refuse to hand back the configuration.
    #[test]
    fn auth_section_rejects_zero_password_hash_concurrency() {
        let config: Config = toml::from_str("[auth]\npassword_hash_max_concurrency = 0\n").unwrap();
        let error = aster_forge_config::load_config(|| Ok::<_, std::convert::Infallible>(config))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("password_hash_max_concurrency must be positive"),
            "{error}"
        );
    }
}
