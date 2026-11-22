//! Static per-process shutdown budgets.
//!
//! `drain_timeout_secs` bounds how long in-flight HTTP requests may finish, and
//! `cancellation_timeout_secs` is the follow-up budget for background workers
//! before the remaining ones are aborted.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Per-process shutdown budgets loaded once at startup.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerConfig {
    /// Budget for finishing in-flight work before shutdown forces a stop.
    pub drain_timeout_secs: u64,
    /// Follow-up budget before remaining background workers are aborted.
    pub cancellation_timeout_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            drain_timeout_secs: 20,
            cancellation_timeout_secs: 5,
        }
    }
}

impl WorkerConfig {
    pub fn validate(&self) -> crate::errors::Result<()> {
        if self
            .drain_timeout_secs
            .checked_add(self.cancellation_timeout_secs)
            .is_none_or(|sum| sum > 86_400)
        {
            return Err(crate::errors::AsterError::config_error(
                "worker shutdown budget must not exceed 24 hours",
            ));
        }
        Ok(())
    }

    pub fn consumer_config(&self) -> aster_forge_tasks::TaskConsumerConfig {
        aster_forge_tasks::TaskConsumerConfig {
            drain_timeout: Duration::from_secs(self.drain_timeout_secs),
            cancellation_timeout: Duration::from_secs(self.cancellation_timeout_secs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkerConfig;
    use std::time::Duration;

    #[test]
    fn defaults_match_forge_consumer_budgets() {
        let worker = WorkerConfig::default();

        assert_eq!(worker.drain_timeout_secs, 20);
        assert_eq!(worker.cancellation_timeout_secs, 5);
        worker
            .validate()
            .expect("default worker budgets should validate");

        let consumer = worker.consumer_config();
        assert_eq!(consumer.drain_timeout, Duration::from_secs(20));
        assert_eq!(consumer.cancellation_timeout, Duration::from_secs(5));
    }

    #[test]
    fn budgets_validate_overflow_and_immediate_shutdown() {
        for (drain, cancellation, valid) in [
            (0, 0, true),
            (86_400, 0, true),
            (86_400, 1, false),
            (u64::MAX, 1, false),
        ] {
            let worker = WorkerConfig {
                drain_timeout_secs: drain,
                cancellation_timeout_secs: cancellation,
            };

            assert_eq!(worker.validate().is_ok(), valid, "{drain}/{cancellation}");
        }
    }

    #[test]
    fn worker_section_is_wired_into_static_config() {
        let config: crate::config::Config = toml::from_str(
            r#"
[worker]
drain_timeout_secs = 60
cancellation_timeout_secs = 5
"#,
        )
        .expect("worker section should deserialize into the static config");

        assert_eq!(config.worker.drain_timeout_secs, 60);
        assert_eq!(config.worker.cancellation_timeout_secs, 5);

        let consumer = config.worker.consumer_config();
        assert_eq!(consumer.drain_timeout, Duration::from_secs(60));
        assert_eq!(consumer.cancellation_timeout, Duration::from_secs(5));
    }

    #[test]
    fn deserialization_rejects_negative_and_unknown_values() {
        assert!(toml::from_str::<WorkerConfig>("drain_timeout_secs = -1").is_err());
        assert!(toml::from_str::<WorkerConfig>("unknown_budget = 5").is_err());
    }
}
