//! Database runtime component registration.

use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::DatabaseRuntimeState;
use aster_forge_db::DbHandles;
use aster_forge_metrics::SharedMetricsRecorder;
use aster_forge_runtime::{
    HealthCheckOptions, HealthCheckScopes, HealthComponentReport, RuntimeComponentBundle,
    RuntimeComponentBundleRegistration, RuntimeComponentKind, RuntimeComponentRegistry,
    runtime_component,
};
use sea_orm::DatabaseConnection;
use std::time::Duration;

const DATABASE_HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Runtime component that closes database handles during graceful shutdown.
pub struct DatabaseRuntimeComponent {
    db_handles: DbHandles,
}

impl DatabaseRuntimeComponent {
    /// Creates a database runtime component from product-owned handles.
    pub const fn new(db_handles: DbHandles) -> Self {
        Self { db_handles }
    }
}

impl RuntimeComponentBundle for DatabaseRuntimeComponent {
    fn register(self, registry: &mut RuntimeComponentRegistry) {
        register_database_shutdown(registry, self.db_handles);
    }
}

/// Creates the database runtime component used by the product entrypoint.
pub fn database_component(
    db_handles: DbHandles,
) -> RuntimeComponentBundleRegistration<DatabaseRuntimeComponent> {
    runtime_component(DatabaseRuntimeComponent::new(db_handles))
}

/// Connects database handles and applies pending migrations.
pub async fn prepare_database_handles(
    config: &crate::config::DatabaseConfig,
    metrics: SharedMetricsRecorder,
) -> Result<DbHandles> {
    let writer = crate::db::connect_with_metrics(config, metrics.clone()).await?;
    migration::Migrator::up(&writer, None)
        .await
        .map_aster_err(AsterError::database_operation)?;
    crate::db::connect_reader_for_writer_with_metrics(config, writer, metrics).await
}

/// Registers database shutdown after task, mail outbox, and audit phases have completed.
pub fn register_database_shutdown(registry: &mut RuntimeComponentRegistry, db_handles: DbHandles) {
    registry
        .component("database")
        .kind(RuntimeComponentKind::Database)
        .depends_on_all(&["background_tasks", "mail_outbox", "audit_manager"])
        .shutdown_once(
            "database_connections",
            None,
            db_handles,
            |db_handles| async move {
                db_handles
                    .close()
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(())
            },
        );
}

/// Registers database readiness and diagnostics health checks.
pub fn register_database_health_check<S>(registry: &mut RuntimeComponentRegistry, state: &S)
where
    S: DatabaseRuntimeState,
{
    let reader_db = state.reader_db().clone();
    registry.component_health_with_options(
        "database",
        RuntimeComponentKind::Database,
        "database",
        database_health_options(),
        move || {
            let reader_db = reader_db.clone();
            async move { check_database_component(&reader_db).await }
        },
    );
}

fn database_health_options() -> HealthCheckOptions {
    HealthCheckOptions::required(Some(DATABASE_HEALTH_CHECK_TIMEOUT))
        .with_scopes(HealthCheckScopes::readiness_and_diagnostics())
}

async fn check_database_component(db: &DatabaseConnection) -> HealthComponentReport {
    match ping_database(db).await {
        Ok(()) => {
            tracing::debug!("database health check succeeded");
            HealthComponentReport::healthy("database", "database ping succeeded")
        }
        Err(error) => {
            tracing::debug!(error = %error, "database health check failed");
            HealthComponentReport::unhealthy("database", format!("database ping failed: {error}"))
        }
    }
}

async fn ping_database(db: &DatabaseConnection) -> Result<()> {
    tracing::debug!("pinging database health check");
    db.ping()
        .await
        .map_aster_err(AsterError::database_operation)
}

#[cfg(test)]
mod tests {
    use super::{
        check_database_component, database_component, prepare_database_handles,
        register_database_health_check, register_database_shutdown,
    };
    use crate::config::DatabaseConfig;
    use crate::runtime::DatabaseRuntimeState;
    use aster_forge_runtime::RuntimeComponentBundle;
    use aster_forge_runtime::{HealthCheckScope, HealthStatus};
    use sea_orm::DatabaseConnection;

    struct DatabaseHealthState {
        db: DatabaseConnection,
    }

    impl DatabaseRuntimeState for DatabaseHealthState {
        fn writer_db(&self) -> &DatabaseConnection {
            &self.db
        }

        fn reader_db(&self) -> &DatabaseConnection {
            &self.db
        }
    }

    #[tokio::test]
    async fn database_component_registers_shutdown_dependency() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("database runtime test database should connect");
        let db_handles = aster_forge_db::DbHandles::single(db);

        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            database_component(db_handles).register(registry);
        });

        let descriptor = registry
            .descriptor("database")
            .expect("database component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Database
        );
        assert_eq!(
            descriptor.dependencies,
            vec!["background_tasks", "mail_outbox", "audit_manager"]
        );
        assert_eq!(
            descriptor
                .shutdown
                .expect("database shutdown should be registered")
                .phase_name,
            "database_connections"
        );
    }

    #[tokio::test]
    async fn database_shutdown_registrar_can_be_used_directly() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("database registrar test database should connect");
        let db_handles = aster_forge_db::DbHandles::single(db);

        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            register_database_shutdown(registry, db_handles);
        });

        assert!(registry.descriptor("database").is_some());
    }

    #[tokio::test]
    async fn prepare_database_handles_connects_and_migrates_database() {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
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
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("database health test database should connect");

        let healthy = check_database_component(&db).await;
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "database ping succeeded");

        db.close_by_ref()
            .await
            .expect("database health test database should close");
        let unhealthy = check_database_component(&db).await;
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(unhealthy.message.contains("database ping failed"));
    }

    #[tokio::test]
    async fn database_health_check_registers_readiness_component() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("database readiness test database should connect");
        let state = DatabaseHealthState { db };
        let mut registry = aster_forge_runtime::RuntimeComponentRegistry::new();

        register_database_health_check(&mut registry, &state);

        assert_eq!(registry.len(), 1);
        let report = registry.run_health(HealthCheckScope::Readiness).await;
        let component_names = report
            .components
            .iter()
            .map(|component| component.name)
            .collect::<Vec<_>>();
        assert_eq!(component_names, vec!["database"]);
        assert_eq!(report.status(), HealthStatus::Healthy);
    }
}
