//! Runtime shutdown tail.
//!
//! Owns the ordered drain that runs after [`crate::runtime::assembly::run`]
//! returns: the metrics provider closes last so in-flight exports finish, and
//! the original runtime error always wins over shutdown failures. When Forge
//! audit recovery lands upstream (AsterCommunity/AsterForge#74), its
//! `finish_audit` call belongs here, before the metrics drain.

use std::io;
use std::time::Duration;

/// Drains the metrics provider and returns the runtime result unchanged unless
/// the runtime succeeded and the drain failed.
pub async fn finish_runtime(
    result: io::Result<()>,
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
) -> io::Result<()> {
    // Always drain exporters, but keep the original runtime error if both fail.
    let shutdown = shutdown_metrics(metrics).await;
    result.and(shutdown.map_err(io::Error::other))
}

pub(super) async fn shutdown_metrics(
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
) -> aster_forge_metrics::MetricResult<()> {
    if let Some(provider) = metrics.provider() {
        // Fixed 10-second budget, deliberately independent of `[worker]`
        // drain_timeout_secs (matching AsterDrive): the worker budgets bound
        // in-flight *product work* (HTTP requests, background tasks), while
        // this tail only flushes the metrics exporter. Tuning worker budgets
        // for batch deployments must not silently move metrics-export limits.
        let shutdown = provider.shutdown(Duration::from_secs(10)).await;
        if let Err(error) = &shutdown {
            tracing::warn!(%error, "metrics shutdown failed");
        }
        return shutdown;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::finish_runtime;
    #[cfg(feature = "otlp-metrics")]
    use std::io;

    #[tokio::test]
    async fn no_op_metrics_shutdown_preserves_a_successful_runtime() {
        finish_runtime(Ok(()), &aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("no-op metrics shutdown should preserve success");
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn configured_prometheus_provider_shuts_down_after_success() {
        let mut config = crate::config::metrics::ObservabilityConfig::default();
        config.prometheus.enabled = true;
        let recorder = crate::runtime::metrics::create_metrics_recorder(&config)
            .expect("Prometheus recorder should initialize");

        finish_runtime(Ok(()), &recorder)
            .await
            .expect("configured metrics provider should shut down cleanly");

        assert!(recorder.provider().unwrap().shutdown_result().is_some());
    }

    #[cfg(feature = "otlp-metrics")]
    #[actix_web::test]
    async fn runtime_error_is_preserved_when_otlp_shutdown_export_fails() {
        use actix_web::{App, HttpResponse, HttpServer, web};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/metrics", listener.local_addr().unwrap());
        let (send, mut received) = tokio::sync::mpsc::unbounded_channel();
        let server = HttpServer::new(move || {
            let send = send.clone();
            App::new().route(
                "/v1/metrics",
                web::post().to(move || {
                    let send = send.clone();
                    async move {
                        let _ = send.send(());
                        HttpResponse::InternalServerError().finish()
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

        let mut config = crate::config::metrics::ObservabilityConfig::default();
        config.otlp_metrics.enabled = true;
        config.otlp_metrics.endpoint = endpoint;
        config.otlp_metrics.interval_seconds = 300;
        let recorder = crate::runtime::metrics::create_metrics_recorder(&config)
            .expect("OTLP recorder should initialize before the collector responds");
        recorder.record_http_request("GET", "http", "/health", 200, 0.01);

        let error = finish_runtime(Err(io::Error::other("original runtime failure")), &recorder)
            .await
            .expect_err("the original runtime error should be returned");
        assert_eq!(error.to_string(), "original runtime failure");
        assert!(
            recorder
                .provider()
                .unwrap()
                .shutdown_result()
                .is_some_and(|result| result.is_err())
        );
        tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .expect("shutdown should attempt a final OTLP export")
            .expect("collector should observe the failed final export");

        handle.stop(true).await;
        task.await.unwrap().unwrap();
    }
}
