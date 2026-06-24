//! Cache backend exports from AsterForge with AsterYggdrasil configuration wiring.

use std::sync::Arc;

use crate::config::CacheConfig;

pub use aster_forge_cache::{CacheBackend, CacheExt};

/// Creates a cache backend from application config.
pub async fn create_cache(config: &CacheConfig) -> Arc<dyn CacheBackend> {
    let forge_config = aster_forge_cache::CacheConfig {
        backend: config.backend.clone(),
        redis_url: config.redis_url.clone(),
        default_ttl: config.default_ttl,
    };
    aster_forge_cache::create_cache(&forge_config).await
}
