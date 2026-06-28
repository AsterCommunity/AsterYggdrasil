//! Runtime synchronization for system configuration changes.
//!
//! Yggdrasil stores configuration values in its own database tables, but
//! multi-process deployments need one shared reload signal when an admin API
//! mutation happens on a different node. This module keeps that signal
//! transport-neutral at the service boundary: Redis pub/sub is only the
//! transport, while every receiver reloads from the authoritative database.

use std::sync::Arc;

use aster_forge_config::{ConfigReloadObservation, ConfigSyncRuntime};
use aster_forge_metrics::SharedMetricsRecorder;
use tokio_util::sync::CancellationToken;

use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, MetricsRuntimeState, RuntimeConfigRuntimeState};

/// Product namespace used for cross-process config reload notifications.
pub const CONFIG_RELOAD_NAMESPACE: &str = "aster_yggdrasil";

/// Runs the config reload subscription worker until shutdown.
pub async fn run_config_reload_subscription<S>(
    state: Arc<S>,
    runtime: ConfigSyncRuntime,
    shutdown: CancellationToken,
) -> Result<()>
where
    S: DatabaseRuntimeState
        + RuntimeConfigRuntimeState
        + MetricsRuntimeState
        + Send
        + Sync
        + 'static,
{
    let metrics = state.metrics().clone();
    let observer = move |observation: ConfigReloadObservation| {
        record_config_reload_observation(&metrics, observation);
    };

    runtime
        .run_reload_subscription_with_observer(
            shutdown,
            move |message| {
                let state = state.clone();
                async move {
                    tracing::debug!(
                        keys = ?message.keys,
                        origin_runtime_id = %message.origin_runtime_id,
                        "reloading runtime config after remote config sync notification"
                    );
                    state
                        .runtime_config()
                        .reload(state.reader_db())
                        .await
                        .map_err(|error| {
                            aster_forge_config::ConfigCoreError::store(error.to_string())
                        })?;
                    Ok(())
                }
            },
            Some(&observer),
        )
        .await
        .map_err(map_config_core_error)
}

fn record_config_reload_observation(
    metrics: &SharedMetricsRecorder,
    observation: ConfigReloadObservation,
) {
    metrics.record_config_reload(
        observation.source,
        observation.decision.as_label(),
        observation.status,
        observation.changed_keys,
        observation.duration_seconds,
    );
}

pub(crate) fn map_config_core_error(error: aster_forge_config::ConfigCoreError) -> AsterError {
    AsterError::internal_error(format!("config sync failed: {error}"))
}

#[cfg(test)]
mod tests {
    use aster_forge_config::ConfigSyncConfig;

    #[test]
    fn config_sync_settings_are_disabled_by_default() {
        let runtime = aster_forge_config::build_config_sync_runtime(
            &ConfigSyncConfig::default(),
            super::CONFIG_RELOAD_NAMESPACE,
        )
        .expect("default config sync should be valid");

        assert!(!runtime.enabled());
        assert_eq!(runtime.namespace(), "aster_yggdrasil");
        assert!(runtime.runtime_id().starts_with("runtime-"));
    }

    #[test]
    fn redis_config_sync_requires_endpoint() {
        let result = aster_forge_config::build_config_sync_runtime(
            &ConfigSyncConfig {
                backend: aster_forge_config::CONFIG_SYNC_BACKEND_REDIS.to_string(),
                endpoint: String::new(),
                topic: "aster.test".to_string(),
            },
            super::CONFIG_RELOAD_NAMESPACE,
        );
        let Err(error) = result else {
            panic!("redis config sync without endpoint should fail");
        };

        assert!(
            error
                .to_string()
                .contains("config_sync.endpoint is required")
        );
    }
}
