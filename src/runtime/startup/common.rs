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
    let cache = aster_forge_cache::create_cache(&config.cache).await;
    let object_storage = object_storage::create_object_storage(&config.object_storage)?;

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
        metrics,
    })
}
