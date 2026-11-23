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
        return scope.service(aster_forge_observability::actix::prometheus_resource(
            "/metrics",
        ));
    }

    #[cfg(not(feature = "metrics"))]
    let _ = config;
    scope
}
