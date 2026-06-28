//! Generic health and readiness checks.

use crate::errors::{AsterError, Result};
use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
use aster_forge_runtime::{
    HealthCheckScope, HealthStatus, RuntimeComponentBundle, RuntimeComponentBundleRegistration,
    RuntimeComponentRegistry, SystemHealthReport,
};

pub async fn check_ready<S: DatabaseRuntimeState>(state: &S) -> Result<()> {
    tracing::debug!("running readiness check");
    let report = run_health_scope(
        HealthCheckScope::Readiness,
        aster_forge_db::database_health_component(state.reader_db().clone()),
    )
    .await;
    record_health_metrics(HealthCheckScope::Readiness, &report);
    if report.has_issues() {
        return Err(AsterError::runtime_unavailable_retryable(
            report.issue_details(),
        ));
    }

    Ok(())
}

pub async fn run_system_health_checks<S>(state: &S) -> SystemHealthReport
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    tracing::debug!("running system health checks");
    let report =
        run_health_scope(HealthCheckScope::Diagnostics, core_health_component(state)).await;
    record_health_metrics(HealthCheckScope::Diagnostics, &report);
    tracing::debug!(
        component_count = report.components.len(),
        unhealthy_count = report
            .components
            .iter()
            .filter(|component| matches!(component.status, HealthStatus::Unhealthy))
            .count(),
        degraded_count = report
            .components
            .iter()
            .filter(|component| matches!(component.status, HealthStatus::Degraded))
            .count(),
        "completed system health checks"
    );
    report
}

pub fn core_health_component<S>(
    state: &S,
) -> RuntimeComponentBundleRegistration<impl RuntimeComponentBundle + use<S>>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    aster_forge_runtime::runtime_component((
        aster_forge_db::database_health_component(state.reader_db().clone()),
        aster_forge_cache::cache_health_component(
            state.config().cache.clone(),
            state.cache().clone(),
        ),
    ))
}

async fn run_health_scope<B>(scope: HealthCheckScope, bundle: B) -> SystemHealthReport
where
    B: RuntimeComponentBundle,
{
    let mut registry = RuntimeComponentRegistry::new();
    registry.register_bundle(bundle);
    registry.run_health(scope).await
}

fn record_health_metrics(scope: HealthCheckScope, report: &SystemHealthReport) {
    crate::metrics::record_health_report(scope, report);
}

#[cfg(test)]
mod tests {
    use super::{HealthCheckScope, HealthStatus, core_health_component};
    use crate::config::{Config, DatabaseConfig};
    use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
    use aster_forge_cache::CacheBackend;
    use aster_forge_runtime::RuntimeComponentBundle;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;

    struct HealthState {
        db: DatabaseConnection,
        config: Arc<Config>,
        cache: Arc<dyn CacheBackend>,
    }

    impl DatabaseRuntimeState for HealthState {
        fn writer_db(&self) -> &DatabaseConnection {
            &self.db
        }

        fn reader_db(&self) -> &DatabaseConnection {
            &self.db
        }
    }

    impl AppConfigRuntimeState for HealthState {
        fn config(&self) -> &Arc<Config> {
            &self.config
        }
    }

    impl CacheRuntimeState for HealthState {
        fn cache(&self) -> &Arc<dyn CacheBackend> {
            &self.cache
        }
    }

    #[tokio::test]
    async fn core_health_component_registers_database_and_cache_components() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        let config = Arc::new(Config::default());
        let cache: Arc<dyn CacheBackend> = Arc::new(aster_forge_cache::MemoryCache::new(60));
        let state = HealthState { db, config, cache };
        let mut registry = aster_forge_runtime::RuntimeComponentRegistry::new();

        core_health_component(&state).register(&mut registry);

        assert_eq!(registry.len(), 2);
        let report = registry
            .run_health(aster_forge_runtime::HealthCheckScope::Diagnostics)
            .await;
        let component_names = report
            .components
            .iter()
            .map(|component| component.name)
            .collect::<Vec<_>>();
        assert_eq!(component_names, vec!["database", "cache"]);
        assert_eq!(report.status(), HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn readiness_health_checks_register_only_database_component() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        let state = HealthState {
            db,
            config: Arc::new(Config::default()),
            cache: Arc::new(aster_forge_cache::MemoryCache::new(60)),
        };
        let mut registry = aster_forge_runtime::RuntimeComponentRegistry::new();

        registry.register_bundle(aster_forge_db::database_health_component(
            state.reader_db().clone(),
        ));

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
