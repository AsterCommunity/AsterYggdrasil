//! Metrics backend provided by AsterForge.

use actix_web::Scope;
use aster_forge_metrics::SharedMetricsRecorder;
use aster_forge_runtime::{HealthCheckScope, SystemHealthReport};

/// Records a health report when the metrics backend is enabled.
#[cfg(any(feature = "metrics", feature = "otlp-metrics"))]
pub fn record_health_report(
    metrics: &SharedMetricsRecorder,
    scope: HealthCheckScope,
    report: &SystemHealthReport,
) {
    metrics.record_health(scope.as_str(), report);
}

/// No-op variant for binaries without a compiled metrics backend.
#[cfg(not(any(feature = "metrics", feature = "otlp-metrics")))]
pub fn record_health_report(
    _metrics: &SharedMetricsRecorder,
    _scope: HealthCheckScope,
    _report: &SystemHealthReport,
) {
}

/// Adds the scrape route only when Prometheus is compiled and configured,
/// mirroring AsterDrive's `attach_optional_routes(scope, config)`.
///
/// OTLP pushes metrics to a collector and must not expose a scrape endpoint.
/// The checker itself is registered once per process as `Data<MetricsAuth>`
/// by the HTTP component (api/http.rs). Mounting the middleware even when
/// Basic auth is disabled keeps a single wiring path from drifting into an
/// unauthenticated variant.
#[cfg(feature = "metrics")]
pub fn configure_route(scope: Scope, config: &aster_forge_metrics::ObservabilityConfig) -> Scope {
    if config.prometheus.enabled {
        return scope.service(
            aster_forge_observability::actix::prometheus_resource("/metrics").wrap(
                actix_web::middleware::from_fn(crate::api::middleware::metrics_auth::middleware),
            ),
        );
    }
    scope
}

/// Route-free variant for binaries without the Prometheus exporter.
#[cfg(not(feature = "metrics"))]
pub fn configure_route(scope: Scope, _config: &aster_forge_metrics::ObservabilityConfig) -> Scope {
    scope
}
