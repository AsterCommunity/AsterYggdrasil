//! Runtime metrics recorder initialization.
//!
//! Provider construction lives on [`crate::config::metrics::ObservabilityConfig`]
//! (mirroring AsterDrive); this module only wraps the Forge provider into the
//! instance-owned recorder used across the application.

use std::sync::Arc;

use aster_forge_metrics::{InfrastructureMetricsRecorder, SharedMetricsRecorder};

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
    let provider = config.create_provider().map_err(|error| {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(feature = "metrics", feature = "otlp-metrics"))]
    use actix_web::web;
    use actix_web::{App, http::StatusCode, test as actix_test};

    /// Production shape: the HTTP component registers one process-wide
    /// checker (api/http.rs); tests that mount the scrape route do the same.
    #[cfg(feature = "metrics")]
    fn scrape_auth_data(
        config: &aster_forge_metrics::MetricsBasicAuthConfig,
    ) -> web::Data<crate::api::middleware::metrics_auth::MetricsAuth> {
        web::Data::new(crate::api::middleware::metrics_auth::MetricsAuth::new(
            config.clone(),
        ))
    }

    #[actix_web::test]
    async fn disabled_metrics_leave_health_up_and_scrape_unavailable() {
        crate::runtime::test_support::ensure_static_config();
        let config = ObservabilityConfig::default();
        assert!(!config.create_provider().unwrap().enabled());
        assert!(
            create_metrics_recorder(&config)
                .unwrap()
                .provider()
                .is_none()
        );

        let app = actix_test::init_service(App::new().configure(|cfg| {
            crate::api::configure(cfg, &config);
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
        assert!(!config.create_provider().unwrap().enabled());
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
                .app_data(scrape_auth_data(&Default::default()))
                .configure(|cfg| crate::api::configure(cfg, &config)),
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

            let app = App::new().app_data(web::Data::new(provider.clone()));
            #[cfg(feature = "metrics")]
            let app = app.app_data(scrape_auth_data(&Default::default()));
            let app =
                actix_test::init_service(app.configure(|cfg| crate::api::configure(cfg, &config)))
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
    #[cfg(feature = "metrics")]
    #[actix_web::test]
    async fn basic_auth_gates_the_scrape_route() {
        use actix_web::http::header;
        use base64::{Engine, prelude::BASE64_STANDARD};

        fn basic(user: &str, password: &str) -> String {
            format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{user}:{password}"))
            )
        }

        crate::runtime::test_support::ensure_static_config();
        let state = crate::runtime::test_support::setup_state().await;
        let config = ObservabilityConfig {
            prometheus: crate::config::metrics::PrometheusConfig {
                enabled: true,
                basic_auth: Some(aster_forge_metrics::MetricsBasicAuthConfig {
                    enabled: true,
                    username: "prometheus".to_owned(),
                    password_hash: aster_forge_crypto::hash_password("scrape-pass").unwrap(),
                }),
            },
            ..ObservabilityConfig::default()
        };
        let recorder = create_metrics_recorder(&config).unwrap();
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .app_data(web::Data::new(recorder.provider().unwrap()))
                .app_data(scrape_auth_data(
                    config.prometheus.basic_auth.as_ref().unwrap(),
                ))
                .configure(|cfg| crate::api::configure(cfg, &config)),
        )
        .await;

        // Every rejected shape: empty 401, challenge header, no caching.
        for header_value in [
            None,
            Some("Bearer jwt".to_owned()),
            Some("Basic".to_owned()),
            Some("Basic ".to_owned()),
            Some("Basic !!".to_owned()),
            Some("Basic Og==".to_owned()),
            Some(format!("Basic {}", BASE64_STANDARD.encode("no-colon"))),
            Some(format!(
                "Basic {}",
                BASE64_STANDARD.encode([0xff, b':', b'x'])
            )),
            Some(basic("prometheus", "wrong-pass")),
            Some(basic("other", "scrape-pass")),
        ] {
            let mut request = actix_test::TestRequest::get().uri("/health/metrics");
            if let Some(value) = header_value {
                request = request.insert_header((header::AUTHORIZATION, value));
            }
            let response = actix_test::call_service(&app, request.to_request()).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(
                response.headers().get(header::WWW_AUTHENTICATE).unwrap(),
                "Basic realm=\"AsterYggdrasil metrics\""
            );
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
            assert!(actix_test::read_body(response).await.is_empty());
        }

        // Valid credentials submitted twice in one request are rejected.
        let value = basic("prometheus", "scrape-pass");
        let duplicate = actix_test::TestRequest::get()
            .uri("/health/metrics")
            .append_header((header::AUTHORIZATION, value.clone()))
            .append_header((header::AUTHORIZATION, value))
            .to_request();
        let response = actix_test::call_service(&app, duplicate).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(actix_test::read_body(response).await.is_empty());

        // Mixed-case scheme succeeds; success responses are not cacheable.
        let valid = actix_test::TestRequest::get()
            .uri("/health/metrics")
            .insert_header((
                header::AUTHORIZATION,
                format!("bAsIc {}", BASE64_STANDARD.encode("prometheus:scrape-pass")),
            ))
            .to_request();
        let response = actix_test::call_service(&app, valid).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; version=0.0.4; charset=utf-8"
        );
        assert!(!actix_test::read_body(response).await.is_empty());

        // Authentication stays scoped to the scrape route.
        let health = actix_test::call_service(
            &app,
            actix_test::TestRequest::get().uri("/health").to_request(),
        )
        .await;
        assert_eq!(health.status(), StatusCode::OK);

        crate::runtime::shutdown::shutdown_metrics(&recorder)
            .await
            .unwrap();
    }
}
