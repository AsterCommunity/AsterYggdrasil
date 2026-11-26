//! Generic runtime operation settings.

use crate::config::RuntimeConfig;
use crate::errors::Result;

pub use crate::config::definitions::{
    BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY,
    BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY, BACKGROUND_TASK_MAX_ATTEMPTS_KEY,
    BACKGROUND_TASK_MAX_CONCURRENCY_KEY, MAIL_OUTBOX_DISPATCH_INTERVAL_SECS_KEY,
    MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY, TASK_LIST_MAX_LIMIT_KEY, TASK_RETENTION_HOURS_KEY,
};

/// Default background task poll interval, in seconds; short enough that queued
/// tasks start promptly while keeping poll traffic negligible.
pub const DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS: u64 = 5;
/// Default upper bound, in seconds, for the dispatcher's idle backoff between
/// polls; trades a slower idle poll rate for prompt pickup once work appears.
pub const DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS: u64 = 60;
/// Default number of background tasks executed concurrently; sized modestly so
/// a single instance stays responsive under default deployments.
pub const DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY: usize = 4;
/// Default number of attempts before a background task is marked failed;
/// low enough to bound retry noise, high enough to survive transient faults.
pub const DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS: i32 = 3;
/// Default page-size cap for admin task list queries, in records per request.
pub const DEFAULT_TASK_LIST_MAX_LIMIT: u64 = 100;
/// Default interval, in seconds, between maintenance cleanup sweeps of expired
/// task records and similar residue.
pub const DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS: u64 = 3600;
/// Default interval, in seconds, between attempts to dispatch queued mail from
/// the persistent outbox.
pub const DEFAULT_MAIL_OUTBOX_DISPATCH_INTERVAL_SECS: u64 = 5;

/// Validates a stored interval value as a positive integer number of seconds,
/// rejecting zero, negatives, and non-numeric input.
pub fn normalize_interval_config_value(key: &str, value: &str) -> Result<String> {
    aster_forge_config::normalize_positive_u64_config_value(key, value).map_err(Into::into)
}

/// Returns the background task poll interval in seconds, falling back to
/// [`DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS`] for missing or
/// non-positive values.
pub fn background_task_dispatch_interval_secs(runtime_config: &RuntimeConfig) -> u64 {
    aster_forge_config::read_positive_u64(
        runtime_config,
        BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY,
        DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS,
    )
}

/// Returns the dispatcher's maximum idle backoff interval in seconds, falling
/// back to [`DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS`] for
/// missing or non-positive values.
pub fn background_task_dispatch_idle_max_interval_secs(runtime_config: &RuntimeConfig) -> u64 {
    aster_forge_config::read_positive_u64(
        runtime_config,
        BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY,
        DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS,
    )
}

/// Returns how many background tasks may run concurrently, falling back to
/// [`DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY`] for missing or non-positive
/// values.
pub fn background_task_max_concurrency(runtime_config: &RuntimeConfig) -> usize {
    aster_forge_config::read_positive_usize(
        runtime_config,
        BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY,
    )
}

/// Returns the maximum attempts per background task before it is marked
/// failed, falling back to [`DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS`] for
/// missing, non-positive, or out-of-range values.
pub fn background_task_max_attempts(runtime_config: &RuntimeConfig) -> i32 {
    aster_forge_config::read_positive_i32(
        runtime_config,
        BACKGROUND_TASK_MAX_ATTEMPTS_KEY,
        DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS,
    )
}

/// Returns the page-size cap for admin task list queries, falling back to
/// [`DEFAULT_TASK_LIST_MAX_LIMIT`] for missing or non-positive values.
pub fn task_list_max_limit(runtime_config: &RuntimeConfig) -> u64 {
    aster_forge_config::read_positive_u64(
        runtime_config,
        TASK_LIST_MAX_LIMIT_KEY,
        DEFAULT_TASK_LIST_MAX_LIMIT,
    )
}

/// Returns the maintenance cleanup sweep interval in seconds, falling back to
/// [`DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS`] for missing or non-positive
/// values.
pub fn maintenance_cleanup_interval_secs(runtime_config: &RuntimeConfig) -> u64 {
    aster_forge_config::read_positive_u64(
        runtime_config,
        MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY,
        DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS,
    )
}

/// Returns the mail outbox dispatch poll interval in seconds, falling back to
/// [`DEFAULT_MAIL_OUTBOX_DISPATCH_INTERVAL_SECS`] for missing or non-positive
/// values.
pub fn mail_outbox_dispatch_interval_secs(runtime_config: &RuntimeConfig) -> u64 {
    aster_forge_config::read_positive_u64(
        runtime_config,
        MAIL_OUTBOX_DISPATCH_INTERVAL_SECS_KEY,
        DEFAULT_MAIL_OUTBOX_DISPATCH_INTERVAL_SECS,
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{
        BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY,
        BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY, BACKGROUND_TASK_MAX_ATTEMPTS_KEY,
        BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS,
        DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS, DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS,
        DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY, DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS,
        DEFAULT_TASK_LIST_MAX_LIMIT, MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY,
        TASK_LIST_MAX_LIMIT_KEY, background_task_dispatch_idle_max_interval_secs,
        background_task_dispatch_interval_secs, background_task_max_attempts,
        background_task_max_concurrency, maintenance_cleanup_interval_secs,
        normalize_interval_config_value, task_list_max_limit,
    };
    use crate::config::{RuntimeConfig, definitions::CONFIG_CATEGORY_RUNTIME_TASKS};
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};
    use aster_forge_db::system_config;
    fn model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: ConfigSource::System,
            visibility: ConfigVisibility::Private,
            namespace: String::new(),
            category: CONFIG_CATEGORY_RUNTIME_TASKS.to_string(),
            description: "test runtime operation config".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    fn runtime_config(values: &[(&str, &str)]) -> RuntimeConfig {
        let runtime_config = RuntimeConfig::new();
        for (key, value) in values {
            runtime_config.apply(model(key, value));
        }
        runtime_config
    }

    #[test]
    fn normalize_interval_accepts_positive_integer() {
        assert_eq!(
            normalize_interval_config_value("test_interval", " 60 ").unwrap(),
            "60"
        );
    }

    #[test]
    fn normalize_interval_rejects_zero_and_non_numbers() {
        assert!(normalize_interval_config_value("test_interval", "0").is_err());
        assert!(normalize_interval_config_value("test_interval", "abc").is_err());
    }

    #[test]
    fn runtime_operation_readers_use_defaults_when_values_are_missing() {
        let runtime_config = RuntimeConfig::new();

        assert_eq!(
            background_task_dispatch_interval_secs(&runtime_config),
            DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS
        );
        assert_eq!(
            background_task_dispatch_idle_max_interval_secs(&runtime_config),
            DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS
        );
        assert_eq!(
            background_task_max_concurrency(&runtime_config),
            DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY
        );
        assert_eq!(
            background_task_max_attempts(&runtime_config),
            DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS
        );
        assert_eq!(
            task_list_max_limit(&runtime_config),
            DEFAULT_TASK_LIST_MAX_LIMIT
        );
        assert_eq!(
            maintenance_cleanup_interval_secs(&runtime_config),
            DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS
        );
    }

    #[test]
    fn runtime_operation_readers_accept_positive_values() {
        let runtime_config = runtime_config(&[
            (BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY, "7"),
            (BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY, "90"),
            (BACKGROUND_TASK_MAX_CONCURRENCY_KEY, "8"),
            (BACKGROUND_TASK_MAX_ATTEMPTS_KEY, "5"),
            (TASK_LIST_MAX_LIMIT_KEY, "250"),
            (MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY, "7200"),
        ]);

        assert_eq!(background_task_dispatch_interval_secs(&runtime_config), 7);
        assert_eq!(
            background_task_dispatch_idle_max_interval_secs(&runtime_config),
            90
        );
        assert_eq!(background_task_max_concurrency(&runtime_config), 8);
        assert_eq!(background_task_max_attempts(&runtime_config), 5);
        assert_eq!(task_list_max_limit(&runtime_config), 250);
        assert_eq!(maintenance_cleanup_interval_secs(&runtime_config), 7200);
    }

    #[test]
    fn runtime_operation_readers_fall_back_for_invalid_or_non_positive_values() {
        let runtime_config = runtime_config(&[
            (BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY, "0"),
            (BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY, "-1"),
            (BACKGROUND_TASK_MAX_CONCURRENCY_KEY, "not-a-number"),
            (BACKGROUND_TASK_MAX_ATTEMPTS_KEY, "0"),
            (TASK_LIST_MAX_LIMIT_KEY, ""),
            (MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY, "1.5"),
        ]);

        assert_eq!(
            background_task_dispatch_interval_secs(&runtime_config),
            DEFAULT_BACKGROUND_TASK_DISPATCH_INTERVAL_SECS
        );
        assert_eq!(
            background_task_dispatch_idle_max_interval_secs(&runtime_config),
            DEFAULT_BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS
        );
        assert_eq!(
            background_task_max_concurrency(&runtime_config),
            DEFAULT_BACKGROUND_TASK_MAX_CONCURRENCY
        );
        assert_eq!(
            background_task_max_attempts(&runtime_config),
            DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS
        );
        assert_eq!(
            task_list_max_limit(&runtime_config),
            DEFAULT_TASK_LIST_MAX_LIMIT
        );
        assert_eq!(
            maintenance_cleanup_interval_secs(&runtime_config),
            DEFAULT_MAINTENANCE_CLEANUP_INTERVAL_SECS
        );
    }

    #[test]
    fn background_task_max_attempts_falls_back_when_value_exceeds_i32() {
        let runtime_config = runtime_config(&[(BACKGROUND_TASK_MAX_ATTEMPTS_KEY, "2147483648")]);

        assert_eq!(
            background_task_max_attempts(&runtime_config),
            DEFAULT_BACKGROUND_TASK_MAX_ATTEMPTS
        );
    }
}
