//! Product metrics configuration and exporter identity.

use crate::errors::{AsterError, Result};
use aster_forge_metrics::ObservabilityConfig;

pub(super) fn deserialize_observability<'de, D>(
    deserializer: D,
) -> std::result::Result<ObservabilityConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    ObservabilityConfig::deserialize(deserializer).map(effective_config)
}

/// Runtime opt-in never enables an exporter omitted from the binary. The
/// deserializer zeroes uncompiled sections, so a config file written for a
/// full build cannot fail startup of one that lacks the exporter.
fn effective_config(config: ObservabilityConfig) -> ObservabilityConfig {
    let config = effective_prometheus(config);
    effective_otlp_metrics(config)
}

/// Compiled-in exporters keep the configured values.
#[cfg(feature = "metrics")]
fn effective_prometheus(config: ObservabilityConfig) -> ObservabilityConfig {
    config
}

/// Prometheus sections are zeroed when the exporter is not compiled in.
#[cfg(not(feature = "metrics"))]
fn effective_prometheus(mut config: ObservabilityConfig) -> ObservabilityConfig {
    config.prometheus = Default::default();
    config
}

/// Compiled-in exporters keep the configured values.
#[cfg(feature = "otlp-metrics")]
fn effective_otlp_metrics(config: ObservabilityConfig) -> ObservabilityConfig {
    config
}

/// OTLP sections are zeroed when the exporter is not compiled in.
#[cfg(not(feature = "otlp-metrics"))]
fn effective_otlp_metrics(mut config: ObservabilityConfig) -> ObservabilityConfig {
    config.otlp_metrics = Default::default();
    config
}

/// Builds the Forge provider under Yggdrasil's product identity.
///
/// The Forge pipeline validates the effective configuration, attaches
/// [`aster_forge_metrics::METRICS_PROFILE_VERSION`] under
/// `com.astercosm.yggdrasil`, and starts the enabled exporters; sections left
/// disabled stay Noop.
pub fn create_provider(
    config: &ObservabilityConfig,
    instance_id: &str,
) -> Result<aster_forge_metrics::MetricsProvider> {
    effective_config(config.clone())
        .create_provider(aster_forge_metrics::MetricsIdentity {
            service_name: "asteryggdrasil".into(),
            service_version: env!("CARGO_PKG_VERSION").into(),
            profile_namespace: "com.astercosm.yggdrasil".into(),
            role: "server".into(),
            node_mode: None,
            instance_id: instance_id.to_owned(),
        })
        .map_err(|error| AsterError::config_error(error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    #[cfg(feature = "metrics")]
    use aster_forge_config::ValidatableConfig as _;
    #[cfg(feature = "metrics")]
    use aster_forge_metrics::ObservabilityConfig;

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
    }

    /// A compiled OTLP exporter keeps its configured values, with secrets
    /// still redacted from `Debug`.
    #[cfg(feature = "otlp-metrics")]
    #[test]
    fn compiled_otlp_exporter_keeps_configured_values() {
        let config: Config = toml::from_str(
            r#"
            [observability.otlp_metrics]
            enabled = true
            [observability.otlp_metrics.headers]
            Authorization = "Bearer test-secret"
            "#,
        )
        .unwrap();
        let observability = config.observability;
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

    /// `effective_config` zeroes an exporter this build lacks.
    #[cfg(not(feature = "otlp-metrics"))]
    #[test]
    fn uncompiled_otlp_exporter_is_zeroed() {
        let config: Config = toml::from_str(
            r#"
            [observability.otlp_metrics]
            enabled = true
            "#,
        )
        .unwrap();
        let observability = config.observability;
        assert!(!observability.otlp_metrics.enabled);
    }

    /// The staged auth section lands in config even while disabled, mirroring
    /// AsterDrive's deserialize round-trip; values are only validated when the
    /// section is switched on. A build without the metrics feature zeroes the
    /// whole Prometheus section instead of keeping staged values.
    #[test]
    fn basic_auth_section_parses_from_toml_while_disabled() {
        let config: Config = toml::from_str(
            r#"
            [observability.prometheus]
            enabled = false
            [observability.prometheus.basic_auth]
            enabled = false
            username = "file-user"
            password_hash = "file-hash"
            "#,
        )
        .unwrap();
        let _ = config;
    }

    /// A compiled metrics backend keeps the staged auth section.
    #[cfg(feature = "metrics")]
    #[test]
    fn compiled_metrics_keeps_staged_basic_auth_section() {
        let config: Config = toml::from_str(
            r#"
            [observability.prometheus]
            enabled = false
            [observability.prometheus.basic_auth]
            enabled = false
            username = "file-user"
            password_hash = "file-hash"
            "#,
        )
        .unwrap();
        let auth = config
            .observability
            .prometheus
            .basic_auth
            .expect("auth section must deserialize");
        assert!(!auth.enabled);
        assert_eq!(auth.username, "file-user");
        assert_eq!(auth.password_hash, "file-hash");
    }

    /// Without the metrics feature the whole Prometheus section is zeroed.
    #[cfg(not(feature = "metrics"))]
    #[test]
    fn uncompiled_metrics_zeroes_staged_basic_auth_section() {
        let config: Config = toml::from_str(
            r#"
            [observability.prometheus]
            enabled = false
            [observability.prometheus.basic_auth]
            enabled = false
            username = "file-user"
            password_hash = "file-hash"
            "#,
        )
        .unwrap();
        assert!(config.observability.prometheus.basic_auth.is_none());
    }

    #[test]
    fn default_config_passes_the_forge_validation_pipeline() {
        let config = Config::default();
        aster_forge_config::load_config(|| Ok::<_, std::convert::Infallible>(config.clone()))
            .expect("default configuration must pass section and cross-section validation");
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn validate_rejects_invalid_deployment_identity_only_when_exporters_enabled() {
        for environment in ["", "   ", "x".repeat(129).as_str(), "bad\u{0007}identity"] {
            let config = ObservabilityConfig {
                environment: environment.to_owned(),
                prometheus: aster_forge_metrics::PrometheusSettings {
                    enabled: true,
                    basic_auth: None,
                },
                ..ObservabilityConfig::default()
            };
            let error = config.validate().unwrap_err();
            assert!(
                error.to_string().contains("environment is invalid"),
                "{environment:?} should be rejected: {error}"
            );
        }

        // The same identities are fine while every exporter is disabled.
        let config = ObservabilityConfig {
            environment: String::new(),
            ..ObservabilityConfig::default()
        };
        config
            .validate()
            .expect("inactive sections must not block startup");
    }

    #[cfg(feature = "otlp-metrics")]
    #[test]
    fn validate_rejects_invalid_active_otlp_push_settings() {
        // timeout must be positive, at most 60 seconds, and not exceed interval.
        let config = ObservabilityConfig {
            otlp_metrics: aster_forge_metrics::OtlpMetricsSettings {
                enabled: true,
                interval_seconds: 5,
                timeout_seconds: 6,
                ..aster_forge_metrics::OtlpMetricsSettings::default()
            },
            ..ObservabilityConfig::default()
        };
        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("timeout must be"), "{error}");

        // Inactive OTLP settings stay unvalidated.
        let config = ObservabilityConfig {
            otlp_metrics: aster_forge_metrics::OtlpMetricsSettings {
                enabled: false,
                interval_seconds: 5,
                timeout_seconds: 6,
                ..aster_forge_metrics::OtlpMetricsSettings::default()
            },
            ..ObservabilityConfig::default()
        };
        config
            .validate()
            .expect("inactive sections must not block startup");
    }

    #[cfg(feature = "metrics")]
    mod basic_auth_tests {
        use aster_forge_config::ValidatableConfig as _;
        use aster_forge_metrics::ObservabilityConfig;

        fn config_with_auth(
            prometheus_enabled: bool,
            enabled: bool,
            username: &str,
            password_hash: &str,
        ) -> ObservabilityConfig {
            ObservabilityConfig {
                prometheus: aster_forge_metrics::PrometheusSettings {
                    enabled: prometheus_enabled,
                    basic_auth: Some(aster_forge_metrics::MetricsBasicAuthConfig {
                        enabled,
                        username: username.to_owned(),
                        password_hash: password_hash.to_owned(),
                    }),
                },
                ..Default::default()
            }
        }

        /// Enabling auth without enabling the exporter is a configuration mistake,
        /// not something to silently ignore.
        #[test]
        fn basic_auth_requires_prometheus_to_be_enabled() {
            let config = config_with_auth(
                false,
                true,
                "scrape",
                &aster_forge_crypto::hash_password("secret").unwrap(),
            );
            let error = config.validate().unwrap_err();
            assert!(error.to_string().contains("basic_auth requires"), "{error}");
        }

        /// Username rules: non-empty, no ':' (it would split the Basic payload
        /// differently) and no control characters.
        #[test]
        fn basic_auth_rejects_invalid_usernames() {
            let valid_hash = aster_forge_crypto::hash_password("secret").unwrap();
            for username in ["", " ", "scrape\n", "us:er", "bad\nuser"] {
                let config = config_with_auth(true, true, username, &valid_hash);
                let error = config.validate().unwrap_err();
                assert!(!error.to_string().contains(&valid_hash), "{error}");
            }
        }

        /// A malformed verifier is rejected offline, and the error never echoes it.
        #[test]
        fn basic_auth_rejects_malformed_hash_without_echoing_it() {
            let marker = "not-a-phc-verifier";
            let config = config_with_auth(true, true, "scrape", marker);
            let error = config.validate().unwrap_err();
            assert!(!error.to_string().contains(marker), "{error}");
        }

        /// A verifier beyond Forge's 1024-byte cap is rejected offline and
        /// never echoed back.
        #[test]
        fn basic_auth_rejects_oversized_verifier() {
            let oversized = "a".repeat(1025);
            let config = config_with_auth(true, true, "scrape", &oversized);
            let error = config.validate().unwrap_err();
            assert!(
                !error.to_string().contains(&"a".repeat(64)),
                "oversized verifier must not be echoed: {error}"
            );
        }

        #[test]
        fn basic_auth_accepts_a_valid_verifier_and_redacts_debug() {
            let hash = aster_forge_crypto::hash_password("secret").unwrap();
            let config = config_with_auth(true, true, "scrape", &hash);
            config.validate().unwrap();
            let debug = format!("{config:?}");
            assert!(!debug.contains(&hash), "verifier leaked through Debug");
            assert!(!debug.contains("scrape"), "username leaked through Debug");
        }

        /// A disabled block may hold staged values: they must not block startup.
        #[test]
        fn disabled_basic_auth_block_is_not_validated() {
            let config = config_with_auth(false, false, "us:er", "not-a-phc-verifier");
            config.validate().unwrap();
        }
    }

    #[test]
    fn unknown_observability_keys_are_rejected_at_each_level() {
        for section in [
            "observability",
            "observability.prometheus",
            "observability.prometheus.basic_auth",
            "observability.otlp_metrics",
        ] {
            let config = format!("[{section}]\nenabeld = true");
            let error = toml::from_str::<Config>(&config).unwrap_err();
            assert!(error.to_string().contains("unknown field `enabeld`"));
        }
    }
}
