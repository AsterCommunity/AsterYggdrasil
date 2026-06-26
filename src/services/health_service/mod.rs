//! Generic health and readiness checks.

mod cache;

use crate::errors::{AsterError, Result};
use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
use aster_forge_runtime::{
    HealthCheckScope, HealthStatus, RuntimeComponentRegistry, SystemHealthReport,
};

pub async fn check_ready<S: DatabaseRuntimeState>(state: &S) -> Result<()> {
    tracing::debug!("running readiness check");
    let report = run_health_scope(state, HealthCheckScope::Readiness, |registry, state| {
        crate::db::runtime::register_database_health_check(registry, state);
    })
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
    let report = run_health_scope(state, HealthCheckScope::Diagnostics, |registry, state| {
        register_core_health_checks(registry, state);
    })
    .await;
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

pub fn register_core_health_checks<S>(registry: &mut RuntimeComponentRegistry, state: &S)
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    crate::db::runtime::register_database_health_check(registry, state);
    cache::register_cache_health_check(registry, state);
}

async fn run_health_scope<S, F>(
    state: &S,
    scope: HealthCheckScope,
    configure: F,
) -> SystemHealthReport
where
    F: FnOnce(&mut RuntimeComponentRegistry, &S),
{
    let mut registry = RuntimeComponentRegistry::configured_with_state(state, configure);
    registry.run_health(scope).await
}

fn record_health_metrics(scope: HealthCheckScope, report: &SystemHealthReport) {
    #[cfg(feature = "metrics")]
    report.record_metrics(scope.as_str(), &crate::metrics::PrometheusMetricsRecorder);

    #[cfg(not(feature = "metrics"))]
    let _ = (scope, report);
}

#[cfg(test)]
mod tests {
    use super::{HealthCheckScope, HealthStatus, register_core_health_checks};
    use crate::config::{Config, DatabaseConfig};
    use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
    use aster_forge_cache::CacheBackend;
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
    async fn core_health_checks_register_database_and_cache_components() {
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

        register_core_health_checks(&mut registry, &state);

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

        crate::db::runtime::register_database_health_check(&mut registry, &state);

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
