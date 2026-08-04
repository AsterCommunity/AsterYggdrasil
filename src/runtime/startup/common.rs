use std::sync::Arc;

use crate::config::Config;
use crate::errors::{AsterError, Result};
use crate::object_storage;
use crate::runtime::{AppState, AppStateParts};

pub(super) async fn prepare_common_state(config: Arc<Config>) -> Result<AppState> {
    aster_forge_runtime::ensure_runtime_temp_dir(&config.server.temp_dir)
        .await
        .map_err(|error| {
            AsterError::config_error(format!("failed to create runtime temp dir: {error}"))
        })?;

    let metrics = crate::runtime::metrics::create_metrics_recorder();
    let db_handles =
        crate::db::runtime::prepare_database_handles(&config.database, metrics.clone()).await?;

    let runtime_config = crate::config::runtime::prepare_runtime_config(
        db_handles.writer(),
        db_handles.reader(),
        &config.auth,
    )
    .await?;
    crate::services::yggdrasil_session_forward_service::prepare_runtime_session_forward_servers(
        db_handles.writer(),
    )
    .await?;
    crate::services::yggdrasil_signature::prepare_runtime_signature_key(db_handles.writer())
        .await?;
    let cache = create_runtime_cache(config.as_ref()).await?;
    let object_storage = object_storage::create_object_storage(&config.object_storage)?;
    let config_sync = aster_forge_config::build_config_sync_runtime(
        &config.config_sync,
        crate::services::config_service::runtime::CONFIG_RELOAD_NAMESPACE,
    )
    .map_err(crate::services::config_service::runtime::map_config_core_error)?;

    crate::services::audit_service::runtime::prepare_runtime_audit_manager(
        db_handles.writer().clone(),
    );

    let mail_sender = crate::services::mail_service::runtime_sender(runtime_config.clone());
    AppState::from_parts(AppStateParts {
        db_handles,
        config,
        runtime_config,
        cache,
        object_storage,
        mail_sender,
        config_sync,
        metrics,
    })
}

async fn create_runtime_cache(
    config: &Config,
) -> Result<std::sync::Arc<dyn aster_forge_cache::CacheBackend>> {
    let failure_policy = if config.deployment.requires_shared_runtime() {
        aster_forge_cache::CacheBackendFailurePolicy::ReturnError
    } else {
        aster_forge_cache::CacheBackendFailurePolicy::FallbackToMemory
    };

    aster_forge_cache::create_cache_with_policy(&config.cache, failure_policy)
        .await
        .map_err(|error| {
            AsterError::config_error(format!(
                "configured cache backend could not be created: {error}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::create_runtime_cache;
    use crate::config::{Config, DeploymentProfile};

    fn redis_config(profile: DeploymentProfile) -> Config {
        let mut config = Config::default();
        config.deployment.profile = profile;
        config.cache.backend = "redis".to_string();
        config.cache.endpoint = "redis://127.0.0.1:1/0".into();
        config
    }

    #[tokio::test]
    async fn single_profile_falls_back_to_memory_when_redis_is_unavailable() {
        let cache = create_runtime_cache(&redis_config(DeploymentProfile::Single))
            .await
            .expect("single profile should fall back to memory cache");

        assert_eq!(cache.backend_name(), "memory");
    }

    #[tokio::test]
    async fn cluster_profile_returns_error_when_redis_is_unavailable() {
        let error = match create_runtime_cache(&redis_config(DeploymentProfile::Cluster)).await {
            Ok(_) => panic!("cluster profile should require redis cache"),
            Err(error) => error,
        };

        assert!(error.message().contains("cache backend"));
    }
}
