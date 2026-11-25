//! Runtime shutdown tail.
//!
//! Owns the ordered drain that runs after [`crate::runtime::assembly::run`]
//! returns: unconfirmed audit records are preserved for operator reconciliation
//! first, then the metrics provider closes last so in-flight exports finish, and
//! the original runtime error always wins over shutdown failures.

use std::io;
use std::time::Duration;

/// Drains audit records and the metrics provider, returning the runtime result
/// unchanged unless the runtime succeeded and a drain failed.
pub async fn finish_runtime(
    result: io::Result<()>,
    audit_writer: &aster_forge_audit::AuditWriter,
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
) -> io::Result<()> {
    finish_runtime_with_recovery(
        result,
        audit_writer,
        metrics,
        crate::runtime::audit_recovery::recovery_directory().as_path(),
    )
    .await
}

/// Testable variant taking the audit-recovery destination explicitly.
async fn finish_runtime_with_recovery(
    result: io::Result<()>,
    audit_writer: &aster_forge_audit::AuditWriter,
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
    recovery_directory: &std::path::Path,
) -> io::Result<()> {
    // Audit recovery comes first: the writer may still hold unconfirmed records,
    // and preservation must not race the exporter tail.
    let audit =
        crate::runtime::audit_recovery::finish_audit(result, audit_writer, recovery_directory)
            .await;
    let shutdown = shutdown_metrics(metrics).await;
    audit.and(shutdown.map_err(io::Error::other))
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
    use std::io;

    use super::finish_runtime;
    use aster_forge_audit::AuditWriter;

    /// A fresh audit writer over an in-memory database; the tail stops it
    /// without pending records, so it never disturbs the result chain.
    async fn test_writer() -> AuditWriter {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let writer = AuditWriter::new(db);
        writer.start().await.unwrap();
        writer
    }

    #[tokio::test]
    async fn no_op_metrics_shutdown_preserves_a_successful_runtime() {
        let writer = test_writer().await;
        finish_runtime(Ok(()), &writer, &aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("no-op metrics shutdown should preserve success");
    }

    #[tokio::test]
    async fn audit_tail_preserves_the_original_runtime_error() {
        let writer = test_writer().await;
        let error = finish_runtime(
            Err(io::Error::other("original runtime failure")),
            &writer,
            &aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect_err("the original runtime error should be returned");
        assert_eq!(error.to_string(), "original runtime failure");
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn configured_prometheus_provider_shuts_down_after_success() {
        let writer = test_writer().await;
        let mut config = crate::config::metrics::ObservabilityConfig::default();
        config.prometheus.enabled = true;
        let recorder = crate::runtime::metrics::create_metrics_recorder(&config)
            .expect("Prometheus recorder should initialize");

        finish_runtime(Ok(()), &writer, &recorder)
            .await
            .expect("configured metrics provider should shut down cleanly");

        assert!(recorder.provider().unwrap().shutdown_result().is_some());
    }

    /// A writer that cannot confirm its records: the drain deadline passes with
    /// the record still unconfirmed.
    async fn failed_audit_writer() -> AuditWriter {
        use aster_forge_db::AuditLogCreate;

        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let writer = AuditWriter::new(db);
        writer.start().await.unwrap();
        writer
            .record(AuditLogCreate {
                user_id: 0,
                action: "server_shutdown".into(),
                entity_type: "system_config".into(),
                entity_id: None,
                entity_name: None,
                details: None,
                ip_address: None,
                user_agent: None,
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        assert!(
            writer
                .flush(tokio::time::Instant::now() + std::time::Duration::from_secs(5))
                .await
                .is_err()
        );
        writer
    }

    fn prometheus_recorder() -> aster_forge_metrics::SharedMetricsRecorder {
        let mut config = crate::config::metrics::ObservabilityConfig::default();
        config.prometheus.enabled = true;
        crate::runtime::metrics::create_metrics_recorder(&config)
            .expect("Prometheus recorder should initialize")
    }

    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn audit_error_wins_and_metrics_still_drains_on_any_platform() {
        use super::finish_runtime_with_recovery;

        let writer = failed_audit_writer().await;
        let recorder = prometheus_recorder();

        let temp = aster_forge_test::temp::TestTempDir::new("shutdown-audit-recovery");
        let recovery_dir = temp.path().join("audit-recovery");
        let error = finish_runtime_with_recovery(Ok(()), &writer, &recorder, &recovery_dir)
            .await
            .expect_err("unconfirmed audit records must fail the shutdown tail");
        assert!(
            error.to_string().contains("unconfirmed audit records"),
            "{error}"
        );
        assert!(
            recorder.provider().unwrap().shutdown_result().is_some(),
            "the metrics drain must still run after the audit tail fails"
        );
    }

    /// Forge's recovery files only exist on unix; other platforms fail closed
    /// before writing anything.
    #[cfg(all(feature = "metrics", unix))]
    #[tokio::test]
    async fn audit_preservation_writes_exactly_one_recovery_envelope() {
        use super::finish_runtime_with_recovery;

        let writer = failed_audit_writer().await;
        let recorder = prometheus_recorder();

        // Forge opens every path component with NOFOLLOW, so the target must be
        // fully resolved (macOS /var is a symlink).
        let temp = aster_forge_test::temp::TestTempDir::new("shutdown-audit-recovery-files");
        let recovery_dir = temp.path().canonicalize().unwrap().join("audit-recovery");
        let error = finish_runtime_with_recovery(Ok(()), &writer, &recorder, &recovery_dir)
            .await
            .expect_err("unconfirmed audit records must fail the shutdown tail");
        assert!(
            error.to_string().contains("saved to"),
            "unix drains report the preservation path: {error}"
        );
        assert_eq!(
            std::fs::read_dir(&recovery_dir)
                .expect("recovery directory should exist")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("audit-"))
                .count(),
            1,
            "the failed drain should leave exactly one recovery envelope"
        );
    }

    #[cfg(feature = "otlp-metrics")]
    #[actix_web::test]
    async fn runtime_error_is_preserved_when_otlp_shutdown_export_fails() {
        use actix_web::{App, HttpResponse, HttpServer, web};
        use std::net::TcpListener;

        let writer = test_writer().await;
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

        let error = finish_runtime(
            Err(io::Error::other("original runtime failure")),
            &writer,
            &recorder,
        )
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
