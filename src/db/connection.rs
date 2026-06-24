//! Database connection wiring backed by the shared AsterForge implementation.

use std::sync::Arc;

use crate::config::DatabaseConfig;
use crate::errors::{AsterError, Result};
use aster_forge_metrics::SharedMetricsRecorder;
use sea_orm::DatabaseConnection;

pub use aster_forge_db::DbHandles;

fn forge_database_config(cfg: &DatabaseConfig) -> aster_forge_db::DatabaseConfig {
    aster_forge_db::DatabaseConfig {
        url: cfg.url.clone(),
        pool_size: cfg.pool_size,
        retry_count: cfg.retry_count,
    }
}

fn forge_metrics(metrics: SharedMetricsRecorder) -> aster_forge_db::SharedDbMetricsRecorder {
    metrics as Arc<dyn aster_forge_db::DbMetricsRecorder>
}

fn map_db_error(error: aster_forge_db::DbError) -> AsterError {
    match error {
        aster_forge_db::DbError::DatabaseConnection(message) => {
            AsterError::database_connection(message)
        }
        aster_forge_db::DbError::DatabaseOperation(message)
        | aster_forge_db::DbError::NonRetryable(message) => AsterError::database_operation(message),
        aster_forge_db::DbError::RetryExhausted => {
            AsterError::database_operation("retry exhausted")
        }
    }
}

/// Connects to the configured database and installs a metrics callback.
pub async fn connect_with_metrics(
    cfg: &DatabaseConfig,
    metrics: SharedMetricsRecorder,
) -> Result<DatabaseConnection> {
    aster_forge_db::connect_with_metrics(&forge_database_config(cfg), forge_metrics(metrics))
        .await
        .map_err(map_db_error)
}

/// Creates reader/writer handles for an existing writer connection and metrics recorder.
pub async fn connect_reader_for_writer_with_metrics(
    cfg: &DatabaseConfig,
    writer: DatabaseConnection,
    metrics: SharedMetricsRecorder,
) -> Result<DbHandles> {
    aster_forge_db::connect_reader_for_writer_with_metrics(
        &forge_database_config(cfg),
        writer,
        forge_metrics(metrics),
    )
    .await
    .map_err(map_db_error)
}
