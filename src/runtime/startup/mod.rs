//! Runtime startup assembly.

mod common;
mod state;

use std::sync::Arc;

use crate::config::Config;
use crate::errors::Result;
use crate::runtime::AppState;
use aster_forge_runtime::{StartupReport, run_required_startup_phase};

pub use state::{PreparedRuntimeState, prepare_runtime_state};

pub struct PreparedRuntime {
    pub state: AppState,
    pub startup_report: StartupReport,
}

pub async fn prepare(config: Arc<Config>) -> Result<PreparedRuntime> {
    let mut phase_reports = Vec::new();
    let prepared = run_required_startup_phase("prepare_runtime_state", move || {
        let config = config.clone();
        async move {
            prepare_runtime_state(config)
                .await
                .map(|prepared| prepared.state)
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
    use crate::runtime::{AppState, AppStateParts};

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
        let config = Arc::new(crate::config::Config::default());
        let cache = aster_forge_cache::create_cache(&config.cache).await;
        crate::services::audit_service::init_global_audit_log_manager(db.clone());

        let state = AppState::from_parts(AppStateParts {
            db_handles: aster_forge_db::DbHandles::single(db.clone()),
            config: config.clone(),
            runtime_config,
            cache,
            object_storage: crate::object_storage::create_object_storage(&config.object_storage)
                .expect("test object storage should initialize"),
            mail_sender: aster_forge_mail::memory_sender(),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
        })
        .expect("runtime startup test AppState should build");

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
