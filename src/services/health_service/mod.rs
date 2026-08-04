//! Generic health and readiness checks.

use crate::errors::{AsterError, Result};
use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
use aster_forge_runtime::{
    HealthCheckScope, HealthStatus, RuntimeComponentBundle, RuntimeComponentBundleRegistration,
    RuntimeComponentRegistry, SystemHealthReport,
};

pub async fn check_ready<S>(state: &S) -> Result<()>
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    tracing::debug!("running readiness check");
    let mut report = run_health_scope(
        HealthCheckScope::Readiness,
        aster_forge_db::database_health_component(state.reader_db().clone()),
    )
    .await;
    if report.has_issues() {
        record_health_metrics(HealthCheckScope::Readiness, &report);
        return Err(AsterError::database_connection(report.issue_details()));
    }

    let cache_report =
        aster_forge_cache::check_cache_component(&state.config().cache, state.cache().as_ref())
            .await;
    let cache_status = cache_report.status;
    let cache_message = cache_report.message.clone();
    report.components.push(cache_report);
    record_health_metrics(HealthCheckScope::Readiness, &report);

    match cache_status {
        HealthStatus::Healthy => {}
        HealthStatus::Degraded if !state.config().deployment.requires_shared_runtime() => {
            tracing::warn!(
                cache_status = cache_status.as_str(),
                cache_message,
                "single deployment readiness accepted degraded cache fallback"
            );
        }
        HealthStatus::Degraded | HealthStatus::Unhealthy => {
            return Err(AsterError::runtime_unavailable_retryable(cache_message));
        }
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
    let database = aster_forge_db::database_health_component(state.reader_db().clone());
    let cache = aster_forge_cache::cache_health_component(
        state.config().cache.clone(),
        state.cache().clone(),
    );

    aster_forge_runtime::runtime_component(move |registry: &mut RuntimeComponentRegistry| {
        registry.register_bundle(database).register_bundle(cache);
    })
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
    use super::{HealthStatus, core_health_component};
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
                url: "sqlite::memory:".into(),
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
    async fn single_readiness_accepts_degraded_memory_fallback() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        let mut config = Config::default();
        config.cache.backend = "redis".to_string();
        config.cache.endpoint = "redis://cache:6379/0".into();
        let state = HealthState {
            db,
            config: Arc::new(config),
            cache: Arc::new(aster_forge_cache::MemoryCache::new(60)),
        };

        super::check_ready(&state)
            .await
            .expect("single readiness should accept memory fallback");
    }

    #[tokio::test]
    async fn cluster_readiness_rejects_degraded_memory_fallback() {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        let mut config = Config::default();
        config.deployment.profile = crate::config::DeploymentProfile::Cluster;
        config.cache.backend = "redis".to_string();
        config.cache.endpoint = "redis://cache:6379/0".into();
        let state = HealthState {
            db,
            config: Arc::new(config),
            cache: Arc::new(aster_forge_cache::MemoryCache::new(60)),
        };

        let error = super::check_ready(&state)
            .await
            .expect_err("cluster readiness should reject memory fallback");
        assert_eq!(
            error.api_error_code(),
            crate::api::error_code::AsterErrorCode::RuntimeUnavailable
        );
        assert_eq!(error.retryable(), Some(true));
    }
}
