//! Configuration loading and shared runtime access.
//!
//! The concrete schema lives in `schema`, while this module exposes the
//! configuration types and process-wide initialization helpers used by the
//! server startup path. Runtime-editable settings are kept in `RuntimeConfig`;
//! static boot settings such as database, cache, and startup-only auth values
//! are loaded once into `Config`.

/// Fixed JWT secret for test-built runtime states; production reads the secret
/// from the static configuration file instead.
pub const TEST_JWT_SECRET: &str = "aster-yggdrasil-test-jwt-secret-0123456789abcdef";

/// Runtime-editable audit log settings helpers.
pub mod audit;
/// Startup-time authentication budgets and derived runtime auth settings.
pub mod auth_runtime;
/// Avatar (Gravatar) base URL runtime configuration binding.
pub mod avatar;
/// Branding and site appearance runtime configuration helpers.
pub mod branding;
/// CORS origin policy runtime configuration helpers.
pub mod cors;
/// Central registry of runtime config keys, defaults, and schema entries.
pub mod definitions;
/// Deployment profile handling for the static config file.
pub mod deployment;
/// Static config file discovery, loading, and default generation.
pub mod loader;
/// Local SMTP delivery policy for outbound mail.
pub mod local_email_policy;
/// SMTP and mail-sending runtime configuration helpers.
pub mod mail;
/// Prometheus metrics runtime configuration helpers.
pub mod metrics;
/// Operations/maintenance mode runtime configuration helpers.
pub mod operations;
/// Static boot configuration handle shared by the whole process.
pub mod runtime;
/// Global runtime config store and JWT issuer/audience constants.
mod runtime_config;
/// Typed schema of the static TOML configuration file.
mod schema;
/// Site URL derivation and normalization helpers.
pub mod site_url;
/// Typed access to the `system_config` table model.
pub mod system_config;
/// Texture storage library root directory configuration.
pub mod texture_library;
/// Server-side texture rendering/preview configuration helpers.
pub mod texture_preview;
/// Static per-process shutdown budgets for background workers.
pub mod worker;
/// Yggdrasil/authlib-injector protocol runtime configuration helpers.
pub mod yggdrasil;

pub use deployment::{DeploymentConfig, DeploymentProfile};
pub use runtime_config::{JWT_AUDIENCE, JWT_ISSUER, RuntimeConfig};
pub use schema::{
    AuthConfig, Config, DEFAULT_AUTH_CSRF_COOKIE_NAME, DEFAULT_AUTH_CSRF_HEADER_NAME,
    DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY, DatabaseConfig, NetworkTrustConfig, RateLimitConfig,
    RateLimitTier, ServerConfig,
};
pub use worker::WorkerConfig;

use std::sync::Arc;
use std::sync::OnceLock;

static CONFIG: OnceLock<Arc<Config>> = OnceLock::new();

/// Ensures a default config file exists for the current working directory and
/// returns its path, without loading it into the global config.
pub fn ensure_default_config_for_current_dir(
    default: &Config,
) -> crate::errors::Result<std::path::PathBuf> {
    loader::ensure_default_config_for_current_dir(default)
}

/// Initializes the global configuration from a CLI-resolved location.
///
/// Generated default files, stable-key backfill, and relative-path resolution
/// behave identically to the implicit current-directory location.
pub fn init_config_at(location: &loader::ConfigLocation) -> crate::errors::Result<Arc<Config>> {
    let cfg = loader::load_at(location)?;
    let cfg = CONFIG.get_or_init(|| Arc::new(cfg)).clone();
    Ok(cfg)
}

/// Returns the globally initialized [`Config`].
///
/// Panics if `init_config`/`init_config_at` has not run yet; only call this
/// after startup has completed.
#[expect(
    clippy::expect_used,
    reason = "route registration is only reached after init_config() succeeds during process startup"
)]
pub fn get_config() -> Arc<Config> {
    CONFIG
        .get()
        .expect("Config not initialized. Call init_config() first.")
        .clone()
}

/// Attempts to get the initialized configuration.
pub fn try_get_config() -> Option<Arc<Config>> {
    CONFIG.get().cloned()
}

/// Manually sets the global configuration for tests.
pub fn set_config_for_test(config: Arc<Config>) -> Result<(), Arc<Config>> {
    CONFIG.set(config)
}
