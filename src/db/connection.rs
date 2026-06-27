//! Database connection wiring backed by the shared AsterForge implementation.

use crate::config::DatabaseConfig;
use crate::errors::Result;
use aster_forge_metrics::SharedMetricsRecorder;
use sea_orm::DatabaseConnection;

fn forge_database_config(cfg: &DatabaseConfig) -> aster_forge_db::DatabaseConfig {
    aster_forge_db::DatabaseConfig {
        url: cfg.url.clone(),
        pool_size: cfg.pool_size,
        retry_count: cfg.retry_count,
    }
}

/// Connects to the configured database and installs a metrics callback.
pub async fn connect_with_metrics(
    cfg: &DatabaseConfig,
    metrics: SharedMetricsRecorder,
) -> Result<DatabaseConnection> {
    aster_forge_db::connect_with_metrics(&forge_database_config(cfg), metrics)
        .await
        .map_err(Into::into)
}

/// Creates reader/writer handles for an existing writer connection and metrics recorder.
pub async fn connect_reader_for_writer_with_metrics(
    cfg: &DatabaseConfig,
    writer: DatabaseConnection,
    metrics: SharedMetricsRecorder,
) -> Result<aster_forge_db::DbHandles> {
    aster_forge_db::connect_reader_for_writer_with_metrics(
        &forge_database_config(cfg),
        writer,
        metrics,
    )
    .await
    .map_err(Into::into)
}
