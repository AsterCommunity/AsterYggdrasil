//! Runtime configuration startup helpers.

use std::sync::Arc;

use sea_orm::ConnectionTrait;

use crate::config::{AuthConfig, RuntimeConfig};
use crate::errors::Result;
use secrecy::ExposeSecret;

/// Loads the runtime configuration cache and installs auth runtimes.
///
/// Callers are responsible for bootstrapping system configuration defaults
/// (see `services::ops::config::ensure_defaults`) before invoking this; the
/// config module stays a pure definition/reader layer.
pub async fn prepare_runtime_config<C>(
    reader: &C,
    auth_config: &AuthConfig,
) -> Result<Arc<RuntimeConfig>>
where
    C: ConnectionTrait,
{
    let runtime_config = Arc::new(RuntimeConfig::new());
    runtime_config.reload(reader).await?;
    runtime_config
        .install_auth_runtimes(
            auth_config.jwt_secret.expose_secret(),
            auth_config.password_hash_max_concurrency,
        )
        .await?;
    Ok(runtime_config)
}
