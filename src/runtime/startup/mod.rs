//! Runtime startup assembly.

mod common;
mod state;

pub use state::prepare_runtime_state;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aster_yggdrasil_migration::Migrator;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    use crate::runtime::{AppState, AppStateParts};

    async fn test_state() -> (AppState, sea_orm::DatabaseConnection) {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".into(),
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

        let state = AppState::from_parts(AppStateParts {
            db_handles: aster_forge_db::DbHandles::single(db.clone()),
            config: config.clone(),
            runtime_config,
            cache,
            object_storage: crate::object_storage::create_object_storage(&config.object_storage)
                .expect("test object storage should initialize"),
            mail_sender: aster_forge_mail::memory_sender(),
            config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test(
                "aster_yggdrasil",
            ),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
            audit_writer: std::sync::Arc::new(aster_forge_audit::AuditWriter::new(db.clone())),
        })
        .expect("runtime startup test AppState should build");

        (state, db)
    }

    #[tokio::test]
    async fn record_server_start_writes_audit_log() {
        let (state, db) = test_state().await;

        state
            .audit_writer()
            .start()
            .await
            .expect("audit writer should start");
        crate::services::audit_service::runtime::record_server_start(&state).await;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        state
            .audit_writer()
            .flush(deadline)
            .await
            .expect("audit writer flush should succeed");

        let count = aster_yggdrasil_model::entities::audit_log::Entity::find()
            .filter(
                aster_yggdrasil_model::entities::audit_log::Column::Action
                    .eq(aster_yggdrasil_model::types::audit::AuditAction::ServerStart),
            )
            .count(&db)
            .await
            .expect("audit log query should succeed");
        assert_eq!(count, 1);
    }
}
