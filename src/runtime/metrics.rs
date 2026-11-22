//! Runtime metrics recorder initialization.

use std::sync::Arc;

use aster_forge_metrics::{
    InfrastructureMetricsRecorder, MetricResult, MetricsProvider, MetricsResource,
    SharedMetricsRecorder,
};

use crate::config::metrics::ObservabilityConfig;
use crate::errors::{AsterError, Result};

/// Creates one recorder from the startup configuration.
///
/// The Forge provider is instance-owned: the Prometheus export path reads it from
/// the recorder (`MetricsRecorder::provider`), and no global SDK provider is installed.
pub fn create_metrics_recorder(config: &ObservabilityConfig) -> Result<SharedMetricsRecorder> {
    if config.prometheus.enabled && !config.prometheus_enabled() {
        tracing::warn!("Prometheus is configured but this build lacks the metrics feature");
    }
    if config.otlp_metrics.enabled && !config.otlp_enabled() {
        tracing::warn!("OTLP metrics are configured but this build lacks the otlp-metrics feature");
    }
    let provider = build_provider(config).map_err(|error| {
        AsterError::config_error(format!("failed to configure observability: {error}"))
    })?;
    if !provider.enabled() {
        return Ok(aster_forge_metrics::NoopMetrics::arc());
    }
    let recorder = InfrastructureMetricsRecorder::new(provider).map_err(|error| {
        AsterError::internal_error(format!("failed to initialize metrics recorder: {error}"))
    })?;
    Ok(Arc::new(recorder))
}

fn build_provider(config: &ObservabilityConfig) -> MetricResult<MetricsProvider> {
    let builder = MetricsProvider::builder();
    if !config.prometheus_enabled() && !config.otlp_enabled() {
        // Inactive OTLP settings need not be valid and must not start a worker.
        return builder.build();
    }

    let builder = builder.with_resource(MetricsResource {
        service_name: "asteryggdrasil".into(),
        service_version: env!("CARGO_PKG_VERSION").into(),
        environment: config.environment.clone(),
        role: "server".into(),
        ..MetricsResource::default()
    })?;

    #[cfg(feature = "metrics")]
    let builder = if config.prometheus_enabled() {
        builder.with_prometheus()?
    } else {
        builder
    };

    #[cfg(feature = "otlp-metrics")]
    let builder = if config.otlp_enabled() {
        builder.with_otlp(config.otlp_metrics.forge_config())?.0
    } else {
        builder
    };

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(feature = "metrics", feature = "otlp-metrics"))]
    use actix_web::web;
    use actix_web::{App, http::StatusCode, test as actix_test};

    #[actix_web::test]
    async fn disabled_metrics_leave_health_up_and_scrape_unavailable() {
        crate::runtime::test_support::ensure_static_config();
        let config = ObservabilityConfig::default();
        assert!(!build_provider(&config).unwrap().enabled());
        assert!(
            create_metrics_recorder(&config)
                .unwrap()
                .provider()
                .is_none()
        );

        let app = actix_test::init_service(App::new().configure(|cfg| {
            crate::api::configure(cfg, config.prometheus_enabled());
        }))
        .await;
        for (uri, expected) in [
            ("/health", StatusCode::OK),
            ("/health/metrics", StatusCode::NOT_FOUND),
        ] {
            let request = actix_test::TestRequest::get().uri(uri).to_request();
            assert_eq!(
                actix_test::call_service(&app, request).await.status(),
                expected
            );
        }
    }

    #[test]
    fn uncompiled_exporters_do_not_start_or_validate_inactive_settings() {
        let mut config = ObservabilityConfig::default();
        config.prometheus.enabled = !cfg!(feature = "metrics");
        config.otlp_metrics.enabled = !cfg!(feature = "otlp-metrics");
        config.otlp_metrics.endpoint = "not an endpoint".into();
        assert!(!build_provider(&config).unwrap().enabled());
    }

    #[cfg(feature = "metrics")]
    #[actix_web::test]
    async fn configured_prometheus_route_exports_metrics_and_environment() {
        crate::runtime::test_support::ensure_static_config();
        let mut config = ObservabilityConfig {
            environment: "staging".into(),
            ..ObservabilityConfig::default()
        };
        config.prometheus.enabled = true;
        let recorder = create_metrics_recorder(&config).unwrap();
        recorder.record_http_request("GET", "http", "/health", 200, 0.01);
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(recorder.provider().unwrap()))
                .configure(|cfg| crate::api::configure(cfg, config.prometheus_enabled())),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/health/metrics")
            .to_request();
        let response = actix_test::call_service(&app, req).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = actix_test::read_body(response).await;
        let exposition = std::str::from_utf8(&body).unwrap();
        assert!(exposition.contains("deployment_environment_name=\"staging\""));
        assert!(exposition.contains("service_name=\"asteryggdrasil\""));
        crate::runtime::shutdown::shutdown_metrics(&recorder)
            .await
            .unwrap();
    }

    #[cfg(feature = "otlp-metrics")]
    #[test]
    fn invalid_active_otlp_config_fails_without_echoing_credentials() {
        let mut config = ObservabilityConfig::default();
        config.otlp_metrics.enabled = true;
        config.otlp_metrics.timeout_seconds = config.otlp_metrics.interval_seconds + 1;
        config
            .otlp_metrics
            .headers
            .insert("Authorization".into(), "Bearer test-secret".into());
        let error = create_metrics_recorder(&config).err().unwrap();
        assert!(error.message().contains("observability"));
        assert!(!error.message().contains("test-secret"));
    }

    #[cfg(feature = "otlp-metrics")]
    #[actix_web::test]
    async fn otlp_push_works_independently_of_the_prometheus_route() {
        use actix_web::{HttpResponse, HttpServer};
        use std::{net::TcpListener, time::Duration};

        crate::runtime::test_support::ensure_static_config();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/metrics", listener.local_addr().unwrap());
        let (send, mut received) = tokio::sync::mpsc::unbounded_channel();
        let server = HttpServer::new(move || {
            let send = send.clone();
            App::new().route(
                "/v1/metrics",
                web::post().to(move |body: web::Bytes| {
                    let send = send.clone();
                    async move {
                        drop(send.send(body));
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

        // In a combined build, verify the second exporter does not disable OTLP.
        for prometheus in [false, true] {
            if prometheus && !cfg!(feature = "metrics") {
                continue;
            }
            let mut config = ObservabilityConfig {
                environment: format!("otlp-test-{prometheus}"),
                ..ObservabilityConfig::default()
            };
            config.prometheus.enabled = prometheus;
            config.otlp_metrics.enabled = true;
            config.otlp_metrics.endpoint = endpoint.clone();
            let recorder = create_metrics_recorder(&config).unwrap();
            let provider = recorder.provider().unwrap();
            assert!(provider.enabled());
            recorder.record_http_request("GET", "http", "/health", 200, 0.01);
            provider.force_flush(Duration::from_secs(6)).await.unwrap();
            let body = tokio::time::timeout(Duration::from_secs(2), received.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(
                body.windows(config.environment.len())
                    .any(|bytes| bytes == config.environment.as_bytes())
            );

            let app = actix_test::init_service(
                App::new()
                    .app_data(web::Data::new(provider.clone()))
                    .configure(|cfg| crate::api::configure(cfg, config.prometheus_enabled())),
            )
            .await;
            let request = actix_test::TestRequest::get()
                .uri("/health/metrics")
                .to_request();
            let expected = if prometheus {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            };
            assert_eq!(
                actix_test::call_service(&app, request).await.status(),
                expected
            );
            crate::runtime::shutdown::shutdown_metrics(&recorder)
                .await
                .unwrap();
            while received.try_recv().is_ok() {}
        }

        handle.stop(true).await;
        task.await.unwrap().unwrap();
    }
}
