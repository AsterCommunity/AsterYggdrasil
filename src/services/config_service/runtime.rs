//! Runtime synchronization for system configuration changes.
//!
//! Yggdrasil stores configuration values in its own database tables, but
//! multi-process deployments need one shared reload signal when an admin API
//! mutation happens on a different node. This module keeps that signal
//! transport-neutral at the service boundary: Redis pub/sub is only the
//! transport, while every receiver reloads from the authoritative database.

use std::sync::Arc;

use aster_forge_config::{
    ConfigReloadObservation, ConfigSyncConnectionObservation, ConfigSyncRuntime,
};
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
    let reload_metrics = state.metrics().clone();
    let reload_observer = move |observation: ConfigReloadObservation| {
        record_config_reload_observation(&reload_metrics, observation);
    };
    let connection_metrics = state.metrics().clone();
    let connection_observer = move |observation: ConfigSyncConnectionObservation| {
        connection_metrics.record_application_event(
            "config_sync",
            observation.state.as_label(),
            "ok",
        );
    };
    let reconcile_state = state.clone();

    runtime
        .run_reload_subscription_with_reconcile_and_observers(
            shutdown,
            move || {
                let state = reconcile_state.clone();
                async move {
                    tracing::debug!(
                        "reconciling runtime config after config sync subscription connected"
                    );
                    reload_runtime_config_from_writer(state.as_ref()).await
                }
            },
            move |message| {
                let state = state.clone();
                async move {
                    tracing::debug!(
                        keys = ?message.keys,
                        origin_runtime_id = %message.origin_runtime_id,
                        "reloading runtime config after remote config sync notification"
                    );
                    reload_runtime_config_from_writer(state.as_ref()).await
                }
            },
            Some(&reload_observer),
            Some(&connection_observer),
        )
        .await
        .map_err(map_config_core_error)
}

async fn reload_runtime_config_from_writer<S>(
    state: &S,
) -> std::result::Result<(), aster_forge_config::ConfigCoreError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    state
        .runtime_config()
        .reload(state.writer_db())
        .await
        .map_err(|error| aster_forge_config::ConfigCoreError::store(error.to_string()))
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
    use std::sync::Arc;

    use aster_forge_config::ConfigSyncConfig;
    use sea_orm::DatabaseConnection;

    use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};

    struct ReloadState {
        writer: DatabaseConnection,
        reader: DatabaseConnection,
        runtime_config: Arc<crate::config::RuntimeConfig>,
    }

    impl DatabaseRuntimeState for ReloadState {
        fn writer_db(&self) -> &DatabaseConnection {
            &self.writer
        }

        fn reader_db(&self) -> &DatabaseConnection {
            &self.reader
        }
    }

    impl RuntimeConfigRuntimeState for ReloadState {
        fn runtime_config(&self) -> &Arc<crate::config::RuntimeConfig> {
            &self.runtime_config
        }
    }

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
                endpoint: String::new().into(),
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

    #[tokio::test]
    async fn reconcile_reloads_authoritative_writer_database() {
        let writer = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("writer database should connect");
        let reader = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("reader database should connect");
        migration::Migrator::up(&writer, None)
            .await
            .expect("writer migrations should succeed");
        migration::Migrator::up(&reader, None)
            .await
            .expect("reader migrations should succeed");
        crate::db::repository::system_config_repo::upsert_with_options(
            &writer,
            crate::config::definitions::BRANDING_TITLE_KEY,
            "Writer Title",
            None,
            None,
        )
        .await
        .expect("writer config should persist");
        crate::db::repository::system_config_repo::upsert_with_options(
            &reader,
            crate::config::definitions::BRANDING_TITLE_KEY,
            "Stale Reader Title",
            None,
            None,
        )
        .await
        .expect("reader config should persist");
        let state = ReloadState {
            writer,
            reader,
            runtime_config: Arc::new(crate::config::RuntimeConfig::new()),
        };

        super::reload_runtime_config_from_writer(&state)
            .await
            .expect("writer reconcile should succeed");

        assert_eq!(
            state
                .runtime_config
                .get(crate::config::definitions::BRANDING_TITLE_KEY)
                .as_deref(),
            Some("Writer Title")
        );
    }
}
