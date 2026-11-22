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

/// Adds the scrape route only when Prometheus is compiled and configured.
pub fn configure_route(scope: Scope, prometheus_enabled: bool) -> Scope {
    // OTLP pushes metrics to a collector and must not expose a scrape endpoint.
    #[cfg(feature = "metrics")]
    if prometheus_enabled {
        return scope.service(aster_forge_observability::actix::prometheus_resource(
            "/metrics",
        ));
    }

    #[cfg(not(feature = "metrics"))]
    let _ = prometheus_enabled;
    scope
}
