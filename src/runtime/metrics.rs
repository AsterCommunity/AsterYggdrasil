//! Runtime metrics recorder initialization.

use aster_forge_metrics::SharedMetricsRecorder;

/// Creates the runtime metrics recorder for this build.
///
/// The Forge provider is instance-owned: the Prometheus export path reads it from
/// the recorder (`MetricsRecorder::provider`), and no global SDK provider is installed.
pub fn create_metrics_recorder() -> SharedMetricsRecorder {
    #[cfg(feature = "metrics")]
    {
        match build_enabled_recorder() {
            Ok(recorder) => recorder,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "failed to initialize metrics provider; falling back to noop metrics"
                );
                aster_forge_metrics::NoopMetrics::arc()
            }
        }
    }

    #[cfg(not(feature = "metrics"))]
    {
        aster_forge_metrics::NoopMetrics::arc()
    }
}

#[cfg(feature = "metrics")]
fn build_enabled_recorder() -> aster_forge_metrics::MetricResult<SharedMetricsRecorder> {
    use std::sync::Arc;

    use aster_forge_metrics::{InfrastructureMetricsRecorder, MetricsProvider, MetricsResource};

    let provider = MetricsProvider::builder()
        .with_resource(MetricsResource {
            service_name: "asteryggdrasil".into(),
            service_version: env!("CARGO_PKG_VERSION").into(),
            environment: std::env::var("ASTER_DEPLOYMENT_ENVIRONMENT").unwrap_or_default(),
            node_mode: String::new(),
            role: "server".into(),
            instance_id: String::new(),
        })?
        .with_prometheus()?
        .build()?;
    let recorder = InfrastructureMetricsRecorder::new(provider)?;
    Ok(Arc::new(recorder))
}
