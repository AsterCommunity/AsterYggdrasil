use std::sync::Arc;

use crate::cache;
use crate::config::{Config, RuntimeConfig};
use crate::db;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::object_storage;
use crate::services::{system_config_service, yggdrasil_session_forward_service};
use aster_forge_metrics::SharedMetricsRecorder;

pub(super) struct CommonRuntimeParts {
    pub config: Arc<Config>,
    pub db_handles: aster_forge_db::DbHandles,
    pub runtime_config: Arc<RuntimeConfig>,
    pub cache: Arc<dyn aster_forge_cache::CacheBackend>,
    pub object_storage: Arc<dyn object_storage::ObjectStorage>,
    pub metrics: SharedMetricsRecorder,
}

pub(super) async fn prepare_common(config: Arc<Config>) -> Result<CommonRuntimeParts> {
    aster_forge_runtime::ensure_runtime_temp_dir(&config.server.temp_dir)
        .await
        .map_err(|error| {
            AsterError::config_error(format!("failed to create runtime temp dir: {error}"))
        })?;

    let metrics = create_metrics_recorder();
    let writer = db::connect_with_metrics(&config.database, metrics.clone()).await?;
    migration::Migrator::up(&writer, None)
        .await
        .map_aster_err(AsterError::database_operation)?;
    let db_handles =
        db::connect_reader_for_writer_with_metrics(&config.database, writer, metrics.clone())
            .await?;

    system_config_service::bootstrap_insecure_cookies(
        db_handles.writer(),
        config.auth.bootstrap_insecure_cookies,
    )
    .await?;
    system_config_service::ensure_defaults(db_handles.writer()).await?;
    yggdrasil_session_forward_service::ensure_builtin_servers(db_handles.writer()).await?;
    crate::services::yggdrasil_signature::ensure_signature_key(db_handles.writer()).await?;
    let runtime_config = Arc::new(RuntimeConfig::new());
    runtime_config.reload(db_handles.reader()).await?;
    let cache = cache::create_cache(&config.cache).await;
    let object_storage = object_storage::create_object_storage(&config.object_storage)?;

    crate::services::audit_service::init_global_audit_log_manager(db_handles.writer().clone());

    Ok(CommonRuntimeParts {
        config,
        db_handles,
        runtime_config,
        cache,
        object_storage,
        metrics,
    })
}

fn create_metrics_recorder() -> SharedMetricsRecorder {
    #[cfg(feature = "metrics")]
    {
        return aster_forge_metrics::init_metrics_or_noop(crate::metrics::init_metrics, || {
            crate::metrics::PrometheusMetricsRecorder
        });
    }

    aster_forge_metrics::NoopMetrics::arc()
}
