//! Runtime configuration startup helpers.

use std::sync::Arc;

use sea_orm::ConnectionTrait;

use crate::config::{AuthConfig, RuntimeConfig};
use crate::errors::Result;
use secrecy::ExposeSecret;

/// Ensures system configuration defaults and loads the runtime cache.
pub async fn prepare_runtime_config<C>(
    writer: &C,
    reader: &C,
    auth_config: &AuthConfig,
) -> Result<Arc<RuntimeConfig>>
where
    C: ConnectionTrait,
{
    crate::services::ops::config::bootstrap_insecure_cookies(
        writer,
        auth_config.bootstrap_insecure_cookies,
    )
    .await?;
    crate::services::ops::config::ensure_defaults(writer).await?;

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
