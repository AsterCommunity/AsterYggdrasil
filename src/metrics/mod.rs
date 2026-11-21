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
    #[cfg(feature = "metrics")]
    metrics.record_health(scope.as_str(), report);

    #[cfg(not(feature = "metrics"))]
    let _ = (metrics, scope, report);
}

/// Adds the metrics HTTP route when the metrics feature is enabled.
pub fn configure_route(scope: Scope) -> Scope {
    #[cfg(feature = "metrics")]
    return scope.service(aster_forge_observability::actix::prometheus_resource("/metrics"));

    #[cfg(not(feature = "metrics"))]
    {
        let _ = scope;
        scope
    }
}
