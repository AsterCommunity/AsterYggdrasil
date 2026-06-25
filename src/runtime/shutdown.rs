//! Graceful shutdown helpers.

use crate::errors::{AsterError, Result};
use crate::runtime::SharedRuntimeState;
use aster_forge_db::DbHandles;
use aster_forge_runtime::ShutdownCoordinator;
use aster_forge_tasks::BackgroundTasks;

pub async fn wait_for_signal() -> Result<()> {
    aster_forge_runtime::wait_for_termination_signal()
        .await
        .map_err(|error| AsterError::internal_error(error.to_string()))?;
    Ok(())
}

pub async fn perform_shutdown(background_tasks: BackgroundTasks, db_handles: DbHandles) {
    let mut coordinator = ShutdownCoordinator::new();
    let mut background_tasks = Some(background_tasks);
    coordinator.phase("background_tasks", None, move || {
        let background_tasks = background_tasks.take();
        async move {
            if let Some(background_tasks) = background_tasks {
                background_tasks.shutdown().await;
            }
            Ok(())
        }
    });

    coordinator.phase("audit_logs", None, || async {
        crate::services::audit_service::shutdown_global_audit_log_manager().await;
        Ok(())
    });

    let mut db_handles = Some(db_handles);
    coordinator.phase("database_connections", None, move || {
        let db_handles = db_handles.take();
        async move {
            if let Some(db_handles) = db_handles {
                db_handles
                    .close()
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    });

    let report = coordinator.run().await;
    if report.has_failures() {
        tracing::warn!("shutdown completed with one or more failed phases");
    } else {
        tracing::info!("shutdown complete");
    }
}

pub async fn record_server_shutdown<S: SharedRuntimeState>(state: &S) {
    let backend = state.writer_db().get_database_backend();
    crate::services::audit_service::log(
        state,
        &crate::services::audit_service::AuditContext::system(),
        crate::types::AuditAction::ServerShutdown,
        crate::types::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
    tracing::info!(?backend, "server shutdown recorded");
}
