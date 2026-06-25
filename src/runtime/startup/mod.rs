//! Runtime startup assembly.

mod common;
mod follower;
mod primary;

use std::sync::Arc;

use crate::config::Config;
use crate::config::node_mode::NodeRuntimeMode;
use crate::errors::Result;
use crate::runtime::AppState;
use aster_forge_runtime::{StartupReport, run_required_startup_phase};

pub use follower::{PreparedFollowerRuntime, prepare_follower};
pub use primary::{PreparedPrimaryRuntime, prepare_primary};

pub struct PreparedRuntime {
    pub state: AppState,
    pub startup_report: StartupReport,
}

pub async fn prepare(config: Arc<Config>) -> Result<PreparedRuntime> {
    let start_mode = config.server.start_mode;
    let mut phase_reports = Vec::new();
    let prepared = run_required_startup_phase("prepare_runtime_state", move || {
        let config = config.clone();
        async move {
            match start_mode {
                NodeRuntimeMode::Primary => {
                    prepare_primary(config).await.map(|prepared| prepared.state)
                }
                NodeRuntimeMode::Follower => prepare_follower(config)
                    .await
                    .map(|prepared| prepared.state),
            }
        }
    })
    .await?;
    let state = prepared.value;
    phase_reports.push(prepared.report);

    let audit = run_required_startup_phase("record_server_start", || async {
        record_server_start(&state).await;
        Ok::<(), crate::errors::AsterError>(())
    })
    .await?;
    phase_reports.push(audit.report);

    Ok(PreparedRuntime {
        state,
        startup_report: StartupReport::new(phase_reports),
    })
}

pub async fn record_server_start(state: &impl crate::runtime::SharedRuntimeState) {
    crate::services::audit_service::log(
        state,
        &crate::services::audit_service::AuditContext::system(),
        crate::types::AuditAction::ServerStart,
        crate::types::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use migration::Migrator;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    use super::record_server_start;
    use crate::runtime::AppState;

    async fn test_state() -> (AppState, sea_orm::DatabaseConnection) {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("test database should connect");
        Migrator::up(&db, None)
            .await
            .expect("test database migration should run");
        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("runtime config should load");
        let cache = crate::cache::create_cache(&crate::config::CacheConfig {
            ..Default::default()
        })
        .await;
        crate::services::audit_service::init_global_audit_log_manager(db.clone());

        let state = AppState {
            db_handles: aster_forge_db::DbHandles::single(db.clone()),
            config: Arc::new(crate::config::Config::default()),
            runtime_config,
            cache,
            object_storage: crate::object_storage::create_object_storage(
                &crate::config::Config::default().object_storage,
            )
            .expect("test object storage should initialize"),
            mail_sender: crate::services::mail_service::memory_sender(),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
            started_at: AppState::new_started_at(),
            yggdrasil_rate_limiter: AppState::new_yggdrasil_rate_limiter(
                &crate::config::Config::default(),
            ),
            yggdrasil_session_forward_http_client:
                AppState::new_yggdrasil_session_forward_http_client()
                    .expect("Yggdrasil session forward HTTP client should build"),
            background_task_dispatch_wakeup: AppState::new_background_task_dispatch_wakeup(),
        };

        (state, db)
    }

    #[tokio::test]
    async fn record_server_start_writes_audit_log() {
        let (state, db) = test_state().await;

        record_server_start(&state).await;
        crate::services::audit_service::flush_global_audit_log_manager().await;

        let count = crate::entities::audit_log::Entity::find()
            .filter(
                crate::entities::audit_log::Column::Action
                    .eq(crate::types::AuditAction::ServerStart),
            )
            .count(&db)
            .await
            .expect("audit log query should succeed");
        assert_eq!(count, 1);
    }
}
