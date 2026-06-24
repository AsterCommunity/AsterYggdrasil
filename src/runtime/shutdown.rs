//! Graceful shutdown helpers.

use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::SharedRuntimeState;
use aster_forge_db::DbHandles;
use aster_forge_tasks::BackgroundTasks;

pub async fn wait_for_signal() -> Result<()> {
    wait_for_termination_signal().await
}

#[cfg(unix)]
async fn wait_for_termination_signal() -> Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt())
        .map_aster_err_ctx("install SIGINT handler", AsterError::internal_error)?;
    let mut sigterm = signal(SignalKind::terminate())
        .map_aster_err_ctx("install SIGTERM handler", AsterError::internal_error)?;

    tokio::select! {
        _ = sigint.recv() => tracing::info!("received SIGINT, shutting down gracefully..."),
        _ = sigterm.recv() => tracing::info!("received SIGTERM, shutting down gracefully..."),
    }
    Ok(())
}

#[cfg(not(unix))]
async fn wait_for_termination_signal() -> Result<()> {
    tokio::signal::ctrl_c()
        .await
        .map_aster_err_ctx("install Ctrl+C handler", AsterError::internal_error)?;
    tracing::info!("received Ctrl+C, shutting down gracefully...");
    Ok(())
}

pub async fn perform_shutdown(background_tasks: BackgroundTasks, db_handles: DbHandles) {
    tracing::info!("stopping background tasks...");
    background_tasks.shutdown().await;
    tracing::info!("background tasks stopped");

    tracing::info!("flushing audit logs...");
    crate::services::audit_service::shutdown_global_audit_log_manager().await;

    tracing::info!("closing database connections...");
    if let Err(error) = db_handles.close().await {
        tracing::error!("error closing database connections: {error}");
    } else {
        tracing::info!("database connections closed");
    }
    tracing::info!("shutdown complete");
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
