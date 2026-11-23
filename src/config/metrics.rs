//! Startup configuration for metrics exporters.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
/// Startup-only exporter settings; omitted sections keep metrics disabled.
pub struct ObservabilityConfig {
    pub environment: String,
    pub prometheus: PrometheusConfig,
    pub otlp_metrics: OtlpMetricsConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            environment: "production".into(),
            prometheus: PrometheusConfig::default(),
            otlp_metrics: OtlpMetricsConfig::default(),
        }
    }
}

impl ObservabilityConfig {
    // Runtime opt-in never enables an exporter omitted from the binary.
    pub(crate) fn prometheus_enabled(&self) -> bool {
        cfg!(feature = "metrics") && self.prometheus.enabled
    }

    pub(crate) fn otlp_enabled(&self) -> bool {
        cfg!(feature = "otlp-metrics") && self.otlp_metrics.enabled
    }

    /// Startup-time validation, mirroring AsterDrive's `ObservabilityConfig`:
    /// enabled exporters require a sane deployment identity, and enabled OTLP
    /// settings must satisfy Forge's push configuration rules. Inactive
    /// sections stay unvalidated — they must not block startup.
    pub fn validate(&self) -> crate::errors::Result<()> {
        if (self.prometheus_enabled() || self.otlp_enabled())
            && (self.environment.trim().is_empty()
                || self.environment.len() > 128
                || self.environment.chars().any(char::is_control))
        {
            return Err(crate::errors::AsterError::config_error(
                "invalid observability deployment identity",
            ));
        }
        #[cfg(feature = "otlp-metrics")]
        if self.otlp_metrics.enabled {
            self.otlp_metrics
                .forge_config()
                .validate()
                .map_err(|error| crate::errors::AsterError::config_error(error.to_string()))?;
        }
        Ok(())
    }

    /// Builds the Forge provider for this configuration, mirroring
    /// AsterDrive's `ObservabilityConfig::create_provider`: an empty builder
    /// for all-disabled sections, then resource identity, the enabled
    /// exporters, and the push profile before build.
    pub fn create_provider(
        &self,
    ) -> aster_forge_metrics::MetricResult<aster_forge_metrics::MetricsProvider> {
        let builder = aster_forge_metrics::MetricsProvider::builder();
        if !self.prometheus_enabled() && !self.otlp_enabled() {
            // Inactive OTLP settings need not be valid and must not start a worker.
            return builder.build();
        }

        let builder = builder.with_resource(aster_forge_metrics::MetricsResource {
            service_name: "asteryggdrasil".into(),
            service_version: env!("CARGO_PKG_VERSION").into(),
            environment: self.environment.clone(),
            role: "server".into(),
            ..aster_forge_metrics::MetricsResource::default()
        })?;

        #[cfg(feature = "metrics")]
        let builder = if self.prometheus_enabled() {
            builder.with_prometheus()?
        } else {
            builder
        };

        #[cfg(feature = "otlp-metrics")]
        let builder = if self.otlp_enabled() {
            builder.with_otlp(self.otlp_metrics.forge_config())?.0
        } else {
            builder
        };

        builder
            .with_profile("com.astercosm.yggdrasil", "astercosm-metrics/v1")?
            .build()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrometheusConfig {
    pub enabled: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OtlpMetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub headers: BTreeMap<String, String>,
}

impl Default for OtlpMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://127.0.0.1:4318/v1/metrics".into(),
            interval_seconds: 60,
            timeout_seconds: 5,
            headers: BTreeMap::new(),
        }
    }
}

impl fmt::Debug for OtlpMetricsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Both endpoint URLs and headers may contain collector credentials.
        f.debug_struct("OtlpMetricsConfig")
            .field("enabled", &self.enabled)
            .field("interval_seconds", &self.interval_seconds)
            .field("timeout_seconds", &self.timeout_seconds)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "otlp-metrics")]
impl OtlpMetricsConfig {
    pub(crate) fn forge_config(&self) -> aster_forge_metrics::OtlpMetricsConfig {
        use std::time::Duration;

        aster_forge_metrics::OtlpMetricsConfig {
            endpoint: self.endpoint.clone(),
            interval: Duration::from_secs(self.interval_seconds),
            timeout: Duration::from_secs(self.timeout_seconds),
            headers: self.headers.clone(),
            max_request_bytes: 16 * 1024 * 1024,
            max_response_bytes: 64 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn absent_observability_disables_both_exporters() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.observability.prometheus.enabled);
        assert!(!config.observability.otlp_metrics.enabled);
    }

    #[test]
    fn exporters_can_be_configured_independently() {
        let config: Config = toml::from_str(
            r#"
            [observability]
            environment = "staging"
            [observability.otlp_metrics]
            enabled = true
            [observability.otlp_metrics.headers]
            Authorization = "Bearer test-secret"
            "#,
        )
        .unwrap();
        let observability = config.observability;
        assert_eq!(observability.environment, "staging");
        assert!(!observability.prometheus.enabled);
        assert!(observability.otlp_metrics.enabled);
        assert_eq!(observability.otlp_metrics.interval_seconds, 60);
        assert_eq!(observability.otlp_metrics.timeout_seconds, 5);
        assert_eq!(
            observability.otlp_metrics.headers["Authorization"],
            "Bearer test-secret"
        );
        let debug = format!("{observability:?}");
        assert!(!debug.contains("test-secret"));
        assert!(!debug.contains("127.0.0.1"));
    }

    #[test]
    fn unknown_observability_keys_are_rejected_at_each_level() {
        for section in [
            "observability",
            "observability.prometheus",
            "observability.otlp_metrics",
        ] {
            let config = format!("[{section}]\nenabeld = true");
            let error = toml::from_str::<Config>(&config).unwrap_err();
            assert!(error.to_string().contains("unknown field `enabeld`"));
        }
    }
}
