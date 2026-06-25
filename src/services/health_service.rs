//! Generic health and readiness checks.

use crate::config::CacheConfig;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState};
use aster_forge_cache::CacheBackend;
use aster_forge_runtime::{
    HealthCheckCriticality, HealthCheckRegistry, HealthComponentReport, HealthStatus,
    SystemHealthReport,
};
use sea_orm::DatabaseConnection;
use std::time::Duration;

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn ping_database(db: &DatabaseConnection) -> Result<()> {
    tracing::debug!("pinging database health check");
    db.ping()
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn check_ready<S: DatabaseRuntimeState>(state: &S) -> Result<()> {
    tracing::debug!("running readiness check");
    ping_database(state.reader_db()).await
}

pub async fn run_system_health_checks<S>(state: &S) -> SystemHealthReport
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    tracing::debug!("running system health checks");
    let mut registry = HealthCheckRegistry::new();

    register_core_health_checks(&mut registry, state);

    let report = registry.run().await;
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

pub fn register_core_health_checks<S>(registry: &mut HealthCheckRegistry, state: &S)
where
    S: DatabaseRuntimeState + AppConfigRuntimeState + CacheRuntimeState,
{
    let reader_db = state.reader_db().clone();
    registry.register(
        "database",
        HealthCheckCriticality::Critical,
        Some(HEALTH_CHECK_TIMEOUT),
        move || {
            let reader_db = reader_db.clone();
            async move { check_database_component(&reader_db).await }
        },
    );

    let cache_config = state.config().cache.clone();
    let cache = state.cache().clone();
    registry.register(
        "cache",
        HealthCheckCriticality::NonCritical,
        Some(HEALTH_CHECK_TIMEOUT),
        move || {
            let cache_config = cache_config.clone();
            let cache = cache.clone();
            async move { check_cache_component(&cache_config, cache.as_ref()).await }
        },
    );
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

async fn check_cache_component(
    config: &CacheConfig,
    cache: &dyn CacheBackend,
) -> HealthComponentReport {
    if config.backend != cache.backend_name() {
        tracing::debug!(
            configured_backend = %config.backend,
            active_backend = cache.backend_name(),
            "cache backend is using fallback"
        );
        return HealthComponentReport::degraded(
            "cache",
            format!(
                "configured cache backend '{}' is using active backend '{}'",
                config.backend,
                cache.backend_name()
            ),
        );
    }

    match cache.health_check().await {
        Ok(()) => {
            tracing::debug!(
                backend = cache.backend_name(),
                "cache health check succeeded"
            );
            HealthComponentReport::healthy("cache", "cache health check succeeded")
        }
        Err(error) => {
            tracing::debug!(backend = cache.backend_name(), error = %error, "cache health check failed");
            HealthComponentReport::unhealthy(
                "cache",
                format!(
                    "cache backend '{}' health check failed: {error}",
                    cache.backend_name()
                ),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HealthStatus, check_cache_component, check_database_component, register_core_health_checks,
    };
    use crate::config::{CacheConfig, Config, DatabaseConfig};
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

    use async_trait::async_trait;

    struct FakeCache {
        backend_name: &'static str,
        healthy: bool,
    }

    impl FakeCache {
        const fn new(backend_name: &'static str) -> Self {
            Self {
                backend_name,
                healthy: true,
            }
        }

        const fn unhealthy(backend_name: &'static str) -> Self {
            Self {
                backend_name,
                healthy: false,
            }
        }
    }

    #[async_trait]
    impl CacheBackend for FakeCache {
        fn backend_name(&self) -> &'static str {
            self.backend_name
        }

        async fn health_check(&self) -> aster_forge_cache::Result<()> {
            if self.healthy {
                Ok(())
            } else {
                Err(aster_forge_cache::CacheError::RedisHealthCheck(
                    "cache probe failed".to_string(),
                ))
            }
        }

        async fn get_bytes(&self, _key: &str) -> Option<Vec<u8>> {
            None
        }

        async fn take_bytes(&self, _key: &str) -> Option<Vec<u8>> {
            None
        }

        async fn set_bytes(&self, _key: &str, _value: Vec<u8>, _ttl_secs: Option<u64>) {}

        async fn set_bytes_if_absent(
            &self,
            _key: &str,
            _value: Vec<u8>,
            _ttl_secs: Option<u64>,
        ) -> bool {
            false
        }

        async fn delete(&self, _key: &str) {}

        async fn invalidate_prefix(&self, _prefix: &str) {}
    }

    #[tokio::test]
    async fn cache_component_reports_configured_backend_fallback() {
        let config = CacheConfig {
            backend: "redis".to_string(),
            redis_url: "redis://example.com:6379/0".to_string(),
            default_ttl: 60,
        };
        let cache = FakeCache::new("memory");

        let report = check_cache_component(&config, &cache).await;

        assert_eq!(report.name, "cache");
        assert_eq!(report.status, HealthStatus::Degraded);
        assert_eq!(
            report.message,
            "configured cache backend 'redis' is using active backend 'memory'"
        );
    }

    #[tokio::test]
    async fn cache_component_reports_active_backend_probe_result() {
        let config = CacheConfig {
            backend: "redis".to_string(),
            redis_url: "redis://example.com:6379/0".to_string(),
            default_ttl: 60,
        };

        let healthy = check_cache_component(&config, &FakeCache::new("redis")).await;
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "cache health check succeeded");

        let degraded = check_cache_component(&config, &FakeCache::unhealthy("redis")).await;
        assert_eq!(degraded.status, HealthStatus::Unhealthy);
        assert!(
            degraded
                .message
                .contains("cache backend 'redis' health check failed")
        );
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
        .unwrap();

        let healthy = check_database_component(&db).await;
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "database ping succeeded");

        db.close_by_ref().await.unwrap();
        let unhealthy = check_database_component(&db).await;
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert!(unhealthy.message.contains("database ping failed"));
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
        let mut registry = aster_forge_runtime::HealthCheckRegistry::new();

        register_core_health_checks(&mut registry, &state);

        assert_eq!(registry.len(), 2);
        let report = registry.run().await;
        let component_names = report
            .components
            .iter()
            .map(|component| component.name)
            .collect::<Vec<_>>();
        assert_eq!(component_names, vec!["database", "cache"]);
        assert_eq!(report.status(), HealthStatus::Healthy);
    }
}
