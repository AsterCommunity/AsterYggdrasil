//! Metrics backend provided by AsterForge.

use actix_web::Scope;
use aster_forge_metrics::SharedMetricsRecorder;
use aster_forge_runtime::{HealthCheckScope, SystemHealthReport};

/// Records a health report when the metrics backend is enabled.
pub fn record_health_report(
    metrics: &SharedMetricsRecorder,
    scope: HealthCheckScope,
    report: &SystemHealthReport,
) {
    #[cfg(any(feature = "metrics", feature = "otlp-metrics"))]
    metrics.record_health(scope.as_str(), report);

    #[cfg(not(any(feature = "metrics", feature = "otlp-metrics")))]
    let _ = (metrics, scope, report);
}

/// Adds the scrape route only when Prometheus is compiled and configured,
/// mirroring AsterDrive's `attach_optional_routes(scope, config)`.
pub fn configure_route(
    scope: Scope,
    config: &crate::config::metrics::ObservabilityConfig,
) -> Scope {
    // OTLP pushes metrics to a collector and must not expose a scrape endpoint.
    #[cfg(feature = "metrics")]
    if config.prometheus_enabled() {
        // The middleware is mounted even when Basic auth is disabled; it
        // returns immediately in that case. Keeping one wiring path avoids a
        // second resource definition drifting from the authenticated one.
        return scope
            .app_data(actix_web::web::Data::new(
                config.prometheus.basic_auth.clone(),
            ))
            .service(
                aster_forge_observability::actix::prometheus_resource("/metrics").wrap(
                    actix_web::middleware::from_fn(
                        crate::api::middleware::metrics_auth::middleware,
                    ),
                ),
            );
    }

    #[cfg(not(feature = "metrics"))]
    let _ = config;
    scope
}
