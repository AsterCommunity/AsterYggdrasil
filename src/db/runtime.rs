//! Database runtime component registration.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_db::DbHandles;
use aster_forge_metrics::SharedMetricsRecorder;

const DATABASE_MIGRATION_LOCK_NAMESPACE: &str = "aster_yggdrasil:database_migrations";

/// Database handles close only after every component that may touch them has
/// stopped. Queue consumers and the leased scheduled group exist only on roles
/// that consume tasks; process maintenance (config reload) runs on every role.
fn database_shutdown_dependencies(consumes_tasks: bool) -> &'static [&'static str] {
    if consumes_tasks {
        &[
            aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
            crate::components::TASK_CONSUMERS_COMPONENT,
            crate::components::SCHEDULED_TASKS_COMPONENT,
            aster_forge_mail::MAIL_OUTBOX_COMPONENT,
            aster_forge_audit::AUDIT_MANAGER_COMPONENT,
        ]
    } else {
        &[
            aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
            aster_forge_mail::MAIL_OUTBOX_COMPONENT,
            aster_forge_audit::AUDIT_MANAGER_COMPONENT,
        ]
    }
}

/// Creates the database runtime component used by the product entrypoint.
///
/// `consumes_tasks` mirrors `ServeRole::consumes_tasks()` for the process role
/// the component is registered for; it is passed as a plain flag so this
/// module does not depend on the runtime assembly.
pub fn database_component(
    db_handles: DbHandles,
    consumes_tasks: bool,
) -> aster_forge_runtime::RuntimeComponentBundleRegistration<aster_forge_db::DatabaseRuntimeComponent>
{
    aster_forge_db::database_component_after(
        db_handles,
        database_shutdown_dependencies(consumes_tasks),
    )
}

/// Connects database handles and applies pending migrations.
pub async fn prepare_database_handles(
    config: &crate::config::DatabaseConfig,
    metrics: SharedMetricsRecorder,
) -> Result<DbHandles> {
    let writer = crate::db::connect_with_metrics(config, metrics.clone()).await?;
    let migration_lock =
        aster_forge_db_migration::MigrationLockOptions::new(DATABASE_MIGRATION_LOCK_NAMESPACE);
    aster_forge_db_migration::run_migrator_with_lock::<aster_yggdrasil_migration::Migrator>(
        &writer,
        &migration_lock,
        None,
    )
    .await
    .map_aster_err(AsterError::database_operation)?;
    crate::db::connect_reader_for_writer_with_metrics(config, writer, metrics).await
}

#[cfg(test)]
mod tests {
    use super::{database_component, database_shutdown_dependencies, prepare_database_handles};
    use crate::config::DatabaseConfig;
    use aster_forge_runtime::RuntimeComponentBundle;
    use aster_forge_runtime::{HealthCheckScope, HealthStatus};

    #[tokio::test]
    async fn database_component_registers_shutdown_dependency() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("database runtime test database should connect");
        let db_handles = aster_forge_db::DbHandles::single(db);

        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            database_component(db_handles, true).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_db::DATABASE_COMPONENT)
            .expect("database component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Database
        );
        assert_eq!(
            descriptor.dependencies,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
                crate::components::TASK_CONSUMERS_COMPONENT,
                crate::components::SCHEDULED_TASKS_COMPONENT,
                aster_forge_mail::MAIL_OUTBOX_COMPONENT,
                aster_forge_audit::AUDIT_MANAGER_COMPONENT
            ]
        );
        assert_eq!(descriptor.shutdown.len(), 1);
        assert_eq!(
            descriptor.shutdown[0].phase_name,
            aster_forge_db::DATABASE_CONNECTIONS_SHUTDOWN_PHASE
        );
        assert_eq!(descriptor.health_checks.len(), 1);
    }

    #[test]
    fn shutdown_dependencies_track_task_consumption() {
        let dependencies = database_shutdown_dependencies(true);
        assert!(
            dependencies.contains(&crate::components::TASK_CONSUMERS_COMPONENT),
            "task-consuming roles must wait for queue consumers"
        );
        assert!(
            dependencies.contains(&crate::components::SCHEDULED_TASKS_COMPONENT),
            "task-consuming roles must wait for the scheduled group"
        );
        for component in [
            crate::components::TASK_CONSUMERS_COMPONENT,
            crate::components::SCHEDULED_TASKS_COMPONENT,
        ] {
            assert!(
                !database_shutdown_dependencies(false).contains(&component),
                "api-only processes must not depend on {component} they never register"
            );
        }
    }

    #[tokio::test]
    async fn prepare_database_handles_connects_and_migrates_database() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".into(),
            pool_size: 1,
            retry_count: 0,
        };

        let db_handles = prepare_database_handles(&config, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("database handles should prepare");

        assert!(db_handles.writer().ping().await.is_ok());
        assert!(db_handles.reader().ping().await.is_ok());
    }

    #[tokio::test]
    async fn database_component_reports_ping_success_and_failure() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("database health test database should connect");

        let healthy = aster_forge_db::check_database_component(&db).await;
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "database ping succeeded");

        db.close_by_ref()
            .await
            .expect("database health test database should close");
        let unhealthy = aster_forge_db::check_database_component(&db).await;
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(unhealthy.message.contains("database ping failed"));
    }

    #[tokio::test]
    async fn database_health_check_registers_readiness_component() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("database readiness test database should connect");
        let mut registry = aster_forge_runtime::RuntimeComponentRegistry::new();

        registry.register_bundle(aster_forge_db::database_health_component(db));

        assert_eq!(registry.len(), 1);
        let report = registry.run_health(HealthCheckScope::Readiness).await;
        let component_names = report
            .components
            .iter()
            .map(|component| component.name)
            .collect::<Vec<_>>();
        assert_eq!(component_names, vec![aster_forge_db::DATABASE_COMPONENT]);
        assert_eq!(report.status(), HealthStatus::Healthy);
    }
}
