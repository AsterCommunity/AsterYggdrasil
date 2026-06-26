//! Runtime configuration startup helpers.

use std::sync::Arc;

use sea_orm::ConnectionTrait;

use crate::config::{AuthConfig, RuntimeConfig};
use crate::errors::Result;

/// Ensures system configuration defaults and loads the runtime cache.
pub async fn prepare_runtime_config<C>(
    writer: &C,
    reader: &C,
    auth_config: &AuthConfig,
) -> Result<Arc<RuntimeConfig>>
where
    C: ConnectionTrait,
{
    crate::services::config_service::bootstrap_insecure_cookies(
        writer,
        auth_config.bootstrap_insecure_cookies,
    )
    .await?;
    crate::services::config_service::ensure_defaults(writer).await?;

    let runtime_config = Arc::new(RuntimeConfig::new());
    runtime_config.reload(reader).await?;
    Ok(runtime_config)
}
