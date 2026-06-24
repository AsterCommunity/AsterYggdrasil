//! Configuration loading and shared runtime access.
//!
//! The concrete schema lives in `schema`, while this module exposes the
//! configuration types and process-wide initialization helpers used by the
//! server startup path. Runtime-editable settings are kept in `RuntimeConfig`;
//! static boot settings such as database, cache, and startup-only auth values
//! are loaded once into `Config`.

pub mod audit;
pub mod auth_runtime;
pub mod avatar;
pub mod branding;
pub mod cors;
pub mod definitions;
mod loader;
pub mod local_email_policy;
pub mod mail;
pub mod node_mode;
pub mod operations;
mod runtime_config;
mod schema;
pub mod site_url;
pub mod system_config;
pub mod texture_library;
pub mod texture_preview;
pub mod yggdrasil;

pub use runtime_config::RuntimeConfig;
pub use schema::{
    AuthConfig, CacheConfig, Config, DEFAULT_AUTH_CSRF_COOKIE_NAME, DEFAULT_AUTH_CSRF_HEADER_NAME,
    DatabaseConfig, LoggingConfig, NetworkTrustConfig, ObjectStorageConfig, RateLimitConfig,
    RateLimitTier, S3ObjectStorageConfig, ServerConfig, ServerFollowerConfig,
};

use std::sync::Arc;
use std::sync::OnceLock;

static CONFIG: OnceLock<Arc<Config>> = OnceLock::new();

pub fn ensure_default_config_for_current_dir(
    default: &Config,
) -> crate::errors::Result<std::path::PathBuf> {
    loader::ensure_default_config_for_current_dir(default)
}

pub fn init_config() -> crate::errors::Result<Arc<Config>> {
    let cfg = loader::load()?;
    let cfg = CONFIG.get_or_init(|| Arc::new(cfg)).clone();
    Ok(cfg)
}

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
