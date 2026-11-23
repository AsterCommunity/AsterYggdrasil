use std::sync::Arc;

use crate::config::Config;
use crate::errors::{AsterError, Result};
use crate::runtime::{AppState, AppStateParts};
use crate::texture_storage;

pub async fn prepare_common_state(config: Arc<Config>) -> Result<AppState> {
    aster_forge_runtime::ensure_runtime_temp_dir(&config.server.temp_dir)
        .await
        .map_err(|error| {
            AsterError::config_error(format!("failed to create runtime temp dir: {error}"))
        })?;

    let metrics = crate::runtime::metrics::create_metrics_recorder(&config.observability)?;
    let result = prepare_state(config, &metrics).await;
    if result.is_err() {
        // Preserve the startup failure if draining the exporter also fails.
        let _shutdown = crate::runtime::shutdown::shutdown_metrics(&metrics).await;
    }
    result
}

async fn prepare_state(
    config: Arc<Config>,
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
) -> Result<AppState> {
    let db_handles =
        crate::db::runtime::prepare_database_handles(&config.database, metrics.clone()).await?;

    let runtime_config = crate::config::runtime::prepare_runtime_config(
        db_handles.writer(),
        db_handles.reader(),
        &config.auth,
    )
    .await?;
    crate::services::yggdrasil::session_forward::prepare_runtime_session_forward_servers(
        db_handles.writer(),
    )
    .await?;
    crate::services::yggdrasil::signature::prepare_runtime_signature_key(db_handles.writer())
        .await?;
    let cache = create_runtime_cache(config.as_ref()).await?;
    let texture_storage = texture_storage::create_texture_store(&config.texture_storage)?;
    let config_sync = aster_forge_config::build_config_sync_runtime(
        &config.config_sync,
        crate::services::ops::config::runtime::CONFIG_RELOAD_NAMESPACE,
    )
    .map_err(crate::services::ops::config::runtime::map_config_core_error)?;

    let audit_writer = std::sync::Arc::new(aster_forge_audit::AuditWriter::new(
        db_handles.writer().clone(),
    ));
    let mail_sender = crate::services::mail::sender::runtime_sender(runtime_config.clone());
    AppState::from_parts(AppStateParts {
        db_handles,
        config,
        runtime_config,
        cache,
        texture_storage,
        mail_sender,
        config_sync,
        metrics: metrics.clone(),
        audit_writer,
    })
}

async fn create_runtime_cache(
    config: &Config,
) -> Result<std::sync::Arc<dyn aster_forge_cache::CacheBackend>> {
    let failure_policy = if config.deployment.requires_shared_runtime() {
        aster_forge_cache::CacheBackendFailurePolicy::ReturnError
    } else {
        aster_forge_cache::CacheBackendFailurePolicy::FallbackToMemory
    };

    aster_forge_cache::create_cache_with_policy(&config.cache, failure_policy)
        .await
        .map_err(|error| {
            AsterError::config_error(format!(
                "configured cache backend could not be created: {error}"
            ))
        })
}

#[cfg(test)]
mod observability_startup_tests {
    use super::prepare_common_state;
    use crate::config::Config;
    use std::{
        path::PathBuf,
        sync::{Arc, OnceLock},
    };

    struct TestRuntimeRoot(PathBuf);

    impl TestRuntimeRoot {
        fn create() -> Self {
            let root = std::env::temp_dir().join(format!(
                "aster-yggdrasil-runtime-test-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&root).expect("temporary runtime root should be created");
            Self(root)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TestRuntimeRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn initialize_test_crypto() {
        static INIT: OnceLock<()> = OnceLock::new();
        INIT.get_or_init(|| {
            let _ = aster_forge_auth::initialize_jwt_crypto_provider();
        });
    }

    fn isolated_config(root: &std::path::Path) -> Config {
        let mut config = Config::default();
        config.database.url = format!(
            "sqlite://{}?mode=rwc",
            root.join("aster_yggdrasil.db").display()
        )
        .into();
        config.database.retry_count = 0;
        config.server.temp_dir = root.join("temp").display().to_string();
        config.texture_storage.local_root = root.join("textures").display().to_string();
        config
    }

    #[tokio::test]
    async fn prepared_state_keeps_the_configured_metrics_recorder() {
        initialize_test_crypto();
        let root = TestRuntimeRoot::create();
        let mut config = isolated_config(root.path());
        config.observability.prometheus.enabled = cfg!(feature = "metrics");

        let state = prepare_common_state(Arc::new(config))
            .await
            .expect("runtime state should prepare with an isolated SQLite database");

        #[cfg(feature = "metrics")]
        assert!(
            state
                .metrics()
                .provider()
                .is_some_and(|provider| provider.enabled())
        );
        #[cfg(not(feature = "metrics"))]
        assert!(state.metrics().provider().is_none());

        crate::runtime::shutdown::shutdown_metrics(state.metrics())
            .await
            .expect("configured metrics provider should shut down");
    }

    #[cfg(feature = "otlp-metrics")]
    #[actix_web::test]
    async fn startup_failure_shuts_down_the_configured_otlp_exporter() {
        use actix_web::{App, HttpResponse, HttpServer, web};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/metrics", listener.local_addr().unwrap());
        let (send, mut received) = tokio::sync::mpsc::unbounded_channel();
        let server = HttpServer::new(move || {
            let send = send.clone();
            App::new().route(
                "/v1/metrics",
                web::post().to(move |_body: web::Bytes| {
                    let send = send.clone();
                    async move {
                        let _ = send.send(());
                        HttpResponse::Ok().finish()
                    }
                }),
            )
        })
        .workers(1)
        .disable_signals()
        .listen(listener)
        .unwrap()
        .run();
        let handle = server.handle();
        let task = actix_web::rt::spawn(server);

        let root = TestRuntimeRoot::create();
        let mut config = isolated_config(root.path());
        config.database.url = format!(
            "sqlite://{}/missing/aster_yggdrasil.db?mode=rwc",
            root.path().display()
        )
        .into();
        config.observability.otlp_metrics.enabled = true;
        config.observability.otlp_metrics.endpoint = endpoint;
        config.observability.otlp_metrics.interval_seconds = 300;

        let error = match prepare_common_state(Arc::new(config)).await {
            Ok(_) => panic!("database connection should fail for a missing parent directory"),
            Err(error) => error,
        };
        assert!(error.message().contains("database"));
        tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .expect("startup failure should drain and shut down the OTLP exporter")
            .expect("the local collector should receive the final export");

        handle.stop(true).await;
        task.await.unwrap().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::create_runtime_cache;
    use crate::config::{Config, DeploymentProfile};

    fn redis_config(profile: DeploymentProfile) -> Config {
        let mut config = Config::default();
        config.deployment.profile = profile;
        config.cache.backend = "redis".to_string();
        config.cache.endpoint = "redis://127.0.0.1:1/0".into();
        config
    }

    #[tokio::test]
    async fn single_profile_falls_back_to_memory_when_redis_is_unavailable() {
        let cache = create_runtime_cache(&redis_config(DeploymentProfile::Single))
            .await
            .expect("single profile should fall back to memory cache");

        assert_eq!(cache.backend_name(), "memory");
    }

    #[tokio::test]
    async fn cluster_profile_returns_error_when_redis_is_unavailable() {
        let error = match create_runtime_cache(&redis_config(DeploymentProfile::Cluster)).await {
            Ok(_) => panic!("cluster profile should require redis cache"),
            Err(error) => error,
        };

        assert!(error.message().contains("cache backend"));
    }
}
