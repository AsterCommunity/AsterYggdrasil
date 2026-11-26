//! System runtime task records stored in the background task table.
use chrono::{DateTime, Utc};

use super::spec::{self, SystemRuntimeTask};
use super::types::{
    RuntimeSystemHealthResult, RuntimeTaskPayload, RuntimeTaskResult, TaskPresentationCode,
};
use super::{
    TypedTaskCreate, insert_typed_task_record, task_expiration_from, truncate_error,
    truncate_status_text,
};
use crate::db::repository::background_task_repo;
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use aster_yggdrasil_model::entities::background_task;
use aster_yggdrasil_model::types::task::{BackgroundTaskStatus, StoredTaskPayload};

/// Registry entry for one system runtime task: wire name, display name, and
/// presentation code bound to its kind.
pub(crate) type SystemRuntimeTaskDefinition =
    aster_forge_tasks::RuntimeTaskDefinition<SystemRuntimeTaskKind, TaskPresentationCode>;

/// Serializes the stored payload JSON identifying a system runtime task kind.
pub(crate) fn system_runtime_payload_json(
    task_name: SystemRuntimeTaskKind,
) -> Result<StoredTaskPayload> {
    spec::serialize_payload::<SystemRuntimeTask>(&RuntimeTaskPayload {
        task_name: task_name.into(),
    })
}

/// Finds the most recent background task record for a system runtime task
/// kind, matching on the stored payload; backs admin views that read the
/// last run of a maintenance task.
pub(crate) async fn find_latest_system_runtime_by_task_name(
    state: &impl DatabaseRuntimeState,
    task_name: SystemRuntimeTaskKind,
) -> Result<Option<background_task::Model>> {
    let payload_json = system_runtime_payload_json(task_name)?;
    background_task_repo::find_latest_system_runtime_by_payload(state.reader_db(), &payload_json)
        .await
}

/// Kinds of maintenance work the product's own runtime schedules and runs
/// through the background task table; each maps to a fixed wire name and
/// presentation code in the registry below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemRuntimeTaskKind {
    /// Drives pending background tasks onto workers on a schedule.
    BackgroundTaskDispatch,
    /// Periodically probes component health and records a report.
    SystemHealthCheck,
    /// Expires stale auth sessions.
    AuthSessionCleanup,
    /// Expires stalled external auth login/link flows.
    ExternalAuthFlowCleanup,
    /// Hands persisted outbox mail to the SMTP sender.
    MailOutboxDispatch,
    /// Deletes audit log rows past the retention window.
    AuditCleanup,
    /// Deletes expired task records and their temp artifacts.
    TaskCleanup,
    /// Expires revoked/stale Yggdrasil access tokens.
    YggdrasilTokenCleanup,
    /// Reconciles texture records against object storage.
    YggdrasilStorageConsistencyCheck,
    /// Removes orphaned texture objects left by failed uploads.
    YggdrasilTextureCleanup,
}

aster_forge_tasks::runtime_task_registry! {
    pub(super) mod system_runtime_task_registry {
        kind: super::SystemRuntimeTaskKind;
        presentation: crate::services::ops::task::types::TaskPresentationCode;
        tasks {
            super::SystemRuntimeTaskKind::BackgroundTaskDispatch => {
                wire: "background-task-dispatch",
                display: "Background task dispatch",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskBackgroundTaskDispatch,
            },
            super::SystemRuntimeTaskKind::SystemHealthCheck => {
                wire: "system-health-check",
                display: "System health check",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskSystemHealthCheck,
            },
            super::SystemRuntimeTaskKind::AuthSessionCleanup => {
                wire: "auth-session-cleanup",
                display: "Auth session cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskAuthSessionCleanup,
            },
            super::SystemRuntimeTaskKind::ExternalAuthFlowCleanup => {
                wire: "external-auth-flow-cleanup",
                display: "External auth flow cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskExternalAuthFlowCleanup,
            },
            super::SystemRuntimeTaskKind::MailOutboxDispatch => {
                wire: "mail-outbox-dispatch",
                display: "Mail outbox dispatch",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskMailOutboxDispatch,
            },
            super::SystemRuntimeTaskKind::AuditCleanup => {
                wire: "audit-cleanup",
                display: "Audit log cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskAuditCleanup,
            },
            super::SystemRuntimeTaskKind::TaskCleanup => {
                wire: "task-cleanup",
                display: "Task artifact cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskTaskCleanup,
            },
            super::SystemRuntimeTaskKind::YggdrasilTokenCleanup => {
                wire: "yggdrasil-token-cleanup",
                display: "Yggdrasil token cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskYggdrasilTokenCleanup,
            },
            super::SystemRuntimeTaskKind::YggdrasilStorageConsistencyCheck => {
                wire: "yggdrasil-storage-consistency-check",
                display: "Yggdrasil storage consistency check",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskYggdrasilStorageConsistencyCheck,
            },
            super::SystemRuntimeTaskKind::YggdrasilTextureCleanup => {
                wire: "yggdrasil-texture-cleanup",
                display: "Yggdrasil texture cleanup",
                presentation: crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskYggdrasilTextureCleanup,
            },
        }
    }
}

impl SystemRuntimeTaskKind {
    /// Canonical wire name used in stored payloads (e.g. `system-health-check`).
    pub const fn as_str(self) -> &'static str {
        system_runtime_task_registry::as_str(self)
    }

    /// Human-readable name shown in admin task listings.
    pub const fn display_name(self) -> &'static str {
        system_runtime_task_registry::display_name(self)
    }

    /// Presentation code the admin UI localizes this task's title from.
    pub const fn presentation_code(self) -> TaskPresentationCode {
        system_runtime_task_registry::presentation_code(self)
    }

    /// Resolves a wire name back to a kind; `None` for unknown or legacy
    /// names from older deployments.
    pub fn from_wire_value(value: &str) -> Option<Self> {
        system_runtime_task_registry::from_wire_value(value)
    }
}

/// All registered system runtime task definitions in registry order; used by
/// startup to schedule every known maintenance task.
pub(crate) fn registered_system_runtime_tasks() -> &'static [SystemRuntimeTaskDefinition] {
    system_runtime_task_registry::DEFINITIONS
}

impl aster_forge_tasks::RegisteredRuntimeTaskKind for SystemRuntimeTaskKind {
    fn as_str(self) -> &'static str {
        Self::as_str(self)
    }

    fn display_name(self) -> &'static str {
        Self::display_name(self)
    }

    fn from_wire_value(value: &str) -> Option<Self> {
        Self::from_wire_value(value)
    }
}

impl std::fmt::Display for SystemRuntimeTaskKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Terminal verdict of one system runtime task run; `Quiet` runs are not
/// recorded as task records at all.
pub enum RuntimeTaskRunOutcome {
    /// Nothing to do; no task record is written for this run.
    Quiet,
    /// The run completed successfully.
    Succeeded {
        /// Short human-readable outcome line, when produced.
        summary: Option<String>,
        /// Health report snapshot, for tasks that check component health.
        system_health: Option<RuntimeSystemHealthResult>,
    },
    /// The run failed; the task record stores the error.
    Failed {
        /// Short human-readable outcome line, when produced.
        summary: Option<String>,
        /// Failure detail stored as the task's last error.
        error: String,
        /// Health report snapshot, for tasks that check component health.
        system_health: Option<RuntimeSystemHealthResult>,
    },
}

impl RuntimeTaskRunOutcome {
    /// Builds the `Quiet` outcome, which suppresses task record creation.
    pub fn quiet() -> Self {
        Self::Quiet
    }

    /// Builds a successful outcome without a health report.
    pub fn succeeded(summary: Option<String>) -> Self {
        Self::Succeeded {
            summary,
            system_health: None,
        }
    }

    /// Builds a successful outcome carrying a system health report.
    pub fn succeeded_with_system_health(
        summary: Option<String>,
        system_health: RuntimeSystemHealthResult,
    ) -> Self {
        Self::Succeeded {
            summary,
            system_health: Some(system_health),
        }
    }

    /// Builds a failed outcome from an error value.
    pub fn failed(summary: Option<String>, error: impl Into<String>) -> Self {
        Self::Failed {
            summary,
            error: error.into(),
            system_health: None,
        }
    }

    /// Builds a failed outcome carrying a system health report.
    pub fn failed_with_system_health(
        summary: Option<String>,
        error: impl Into<String>,
        system_health: RuntimeSystemHealthResult,
    ) -> Self {
        Self::Failed {
            summary,
            error: error.into(),
            system_health: Some(system_health),
        }
    }

    fn should_record(&self) -> bool {
        !matches!(self, Self::Quiet)
    }

    fn status(&self) -> BackgroundTaskStatus {
        match self {
            Self::Quiet | Self::Succeeded { .. } => BackgroundTaskStatus::Succeeded,
            Self::Failed { .. } => BackgroundTaskStatus::Failed,
        }
    }

    fn summary(&self) -> Option<&str> {
        match self {
            Self::Quiet => None,
            Self::Succeeded { summary, .. } | Self::Failed { summary, .. } => summary.as_deref(),
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { error, .. } => Some(error.as_str()),
            Self::Quiet | Self::Succeeded { .. } => None,
        }
    }

    fn system_health(&self) -> Option<RuntimeSystemHealthResult> {
        match self {
            Self::Succeeded { system_health, .. } | Self::Failed { system_health, .. } => {
                system_health.clone()
            }
            Self::Quiet => None,
        }
    }
}

/// Persists one system runtime task run as a background task record so admins
/// can see when maintenance last ran and what it did. Quiet runs produce no
/// record; consecutive healthy system-health-check runs refresh the latest
/// succeeded record in place instead of stacking duplicates. Returns the
/// stored record, or `None` for quiet runs.
pub async fn record_runtime_task_run(
    state: &(impl DatabaseRuntimeState + RuntimeConfigRuntimeState),
    task_name: SystemRuntimeTaskKind,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    outcome: &RuntimeTaskRunOutcome,
) -> Result<Option<background_task::Model>> {
    record_runtime_task_run_with_dedupe_key(
        state,
        task_name,
        started_at,
        finished_at,
        outcome,
        None,
    )
    .await
}

/// [`record_runtime_task_run`] for scheduler-triggered runs: derives a dedupe
/// key from the scheduled slot so a re-delivered schedule tick cannot create
/// duplicate task records.
pub async fn record_scheduled_runtime_task_run(
    state: &(impl DatabaseRuntimeState + RuntimeConfigRuntimeState),
    task_name: SystemRuntimeTaskKind,
    scheduled_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    outcome: &RuntimeTaskRunOutcome,
) -> Result<Option<background_task::Model>> {
    let dedupe_key = aster_forge_tasks::scheduled_task_dedupe_key(
        "aster_yggdrasil",
        task_name.as_str(),
        scheduled_at,
    )?;
    record_runtime_task_run_with_dedupe_key(
        state,
        task_name,
        started_at,
        finished_at,
        outcome,
        Some(dedupe_key),
    )
    .await
}

async fn record_runtime_task_run_with_dedupe_key(
    state: &(impl DatabaseRuntimeState + RuntimeConfigRuntimeState),
    task_name: SystemRuntimeTaskKind,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    outcome: &RuntimeTaskRunOutcome,
    dedupe_key: Option<aster_forge_tasks::TaskDedupeKey>,
) -> Result<Option<background_task::Model>> {
    if !outcome.should_record() {
        return Ok(None);
    }

    let payload = RuntimeTaskPayload {
        task_name: task_name.into(),
    };
    let payload_json = spec::serialize_payload::<SystemRuntimeTask>(&payload)?;
    let summary = outcome.summary().map(truncate_status_text);
    let last_error = outcome.error().map(truncate_error);
    let result = RuntimeTaskResult::from_timestamps(
        started_at,
        finished_at,
        summary.clone(),
        outcome.system_health(),
    );
    let result_json = spec::serialize_result::<SystemRuntimeTask>(&result)?;

    if should_refresh_latest_success(task_name, outcome)
        && let Some(existing) = background_task_repo::find_latest_system_runtime_by_payload(
            state.writer_db(),
            &payload_json,
        )
        .await?
        && existing.status == BackgroundTaskStatus::Succeeded
        && background_task_repo::refresh_system_runtime_success(
            state.writer_db(),
            background_task_repo::SystemRuntimeSuccessRefresh {
                id: existing.id,
                result_json: result_json.as_ref(),
                status_text: summary.as_deref(),
                next_run_at: finished_at,
                started_at,
                finished_at,
                expires_at: task_expiration_from(state, finished_at),
            },
        )
        .await?
    {
        return background_task_repo::find_by_id(state.writer_db(), existing.id)
            .await
            .map(Some);
    }

    let progress_current = if matches!(outcome, RuntimeTaskRunOutcome::Failed { .. }) {
        0
    } else {
        1
    };
    let mut create = TypedTaskCreate::<SystemRuntimeTask>::new(task_name.display_name(), payload)
        .status(outcome.status())
        .without_steps()
        .progress(progress_current, 1)
        .started_at(started_at)
        .finished_at(finished_at)
        .last_error(last_error)
        .failure_can_retry(if matches!(outcome, RuntimeTaskRunOutcome::Failed { .. }) {
            Some(false)
        } else {
            None
        })
        .result(&result)?;
    if let Some(dedupe_key) = dedupe_key {
        create = create.dedupe_key(dedupe_key);
    }
    if let Some(summary) = summary {
        create = create.status_text(summary);
    }

    let task = insert_typed_task_record(state, state.writer_db(), create).await?;
    Ok(Some(task))
}

fn should_refresh_latest_success(
    task_name: SystemRuntimeTaskKind,
    outcome: &RuntimeTaskRunOutcome,
) -> bool {
    task_name == SystemRuntimeTaskKind::SystemHealthCheck
        && matches!(
            outcome,
            RuntimeTaskRunOutcome::Succeeded {
                system_health: Some(RuntimeSystemHealthResult {
                    status: super::types::RuntimeSystemHealthStatus::Healthy,
                    ..
                }),
                ..
            }
        )
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeTaskRunOutcome, SystemRuntimeTaskKind, find_latest_system_runtime_by_task_name,
        record_runtime_task_run, record_scheduled_runtime_task_run, system_runtime_payload_json,
    };
    use crate::services::ops::task::types::{
        RuntimeSystemHealthComponent, RuntimeSystemHealthResult, RuntimeSystemHealthStatus,
    };
    use aster_yggdrasil_model::types::task::BackgroundTaskStatus;
    use chrono::{Duration, Utc};

    async fn test_state() -> crate::runtime::AppState {
        let db_cfg = crate::config::DatabaseConfig {
            url: "sqlite::memory:".into(),
            pool_size: 1,
            retry_count: 0,
        };
        let db = crate::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("runtime task test database should connect");
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .expect("runtime task test migrations should run");
        crate::services::ops::config::ensure_defaults(&db)
            .await
            .expect("runtime task test defaults should seed");

        let runtime_config = std::sync::Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("runtime task config should reload");
        let config = std::sync::Arc::new(crate::config::Config {
            database: db_cfg,
            cache: aster_forge_cache::CacheConfig {
                ..Default::default()
            },
            ..Default::default()
        });
        let cache = aster_forge_cache::create_cache(&config.cache).await;
        let texture_storage = crate::texture_storage::create_texture_store(&config.texture_storage)
            .expect("object storage should initialize");
        crate::runtime::AppState::from_parts(crate::runtime::AppStateParts {
            audit_writer: std::sync::Arc::new(aster_forge_audit::AuditWriter::new(db.clone())),
            db_handles: aster_forge_db::DbHandles::single(db),
            config,
            runtime_config,
            cache,
            texture_storage,
            mail_sender: aster_forge_mail::memory_sender(),
            config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test(
                "aster_yggdrasil",
            ),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
        })
        .expect("runtime task test AppState should build")
    }

    fn health(status: RuntimeSystemHealthStatus) -> RuntimeSystemHealthResult {
        RuntimeSystemHealthResult {
            status,
            components: vec![RuntimeSystemHealthComponent {
                name: "database".to_string(),
                status: RuntimeSystemHealthStatus::Healthy,
                message: "ok".to_string(),
                details: Vec::new(),
            }],
        }
    }

    #[test]
    fn system_runtime_kind_wire_values_display_names_and_codes_are_stable() {
        assert_eq!(
            SystemRuntimeTaskKind::BackgroundTaskDispatch.as_str(),
            "background-task-dispatch"
        );
        assert_eq!(
            SystemRuntimeTaskKind::SystemHealthCheck.display_name(),
            "System health check"
        );
        assert_eq!(
            SystemRuntimeTaskKind::from_wire_value("auth-session-cleanup"),
            Some(SystemRuntimeTaskKind::AuthSessionCleanup)
        );
        assert_eq!(SystemRuntimeTaskKind::from_wire_value("unknown"), None);
        assert_eq!(
            SystemRuntimeTaskKind::TaskCleanup.to_string(),
            "task-cleanup"
        );
        assert_eq!(
            SystemRuntimeTaskKind::AuditCleanup.presentation_code(),
            crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskAuditCleanup
        );
        assert_eq!(
            SystemRuntimeTaskKind::YggdrasilTextureCleanup.as_str(),
            "yggdrasil-texture-cleanup"
        );
        assert_eq!(
            SystemRuntimeTaskKind::from_wire_value("yggdrasil-texture-cleanup"),
            Some(SystemRuntimeTaskKind::YggdrasilTextureCleanup)
        );
        assert_eq!(
            SystemRuntimeTaskKind::YggdrasilTokenCleanup.presentation_code(),
            crate::services::ops::task::types::TaskPresentationCode::RuntimeTaskYggdrasilTokenCleanup
        );
        assert_eq!(
            SystemRuntimeTaskKind::from_wire_value("yggdrasil-storage-consistency-check"),
            Some(SystemRuntimeTaskKind::YggdrasilStorageConsistencyCheck)
        );
    }

    #[test]
    fn runtime_task_run_outcome_constructors_preserve_variants() {
        assert_eq!(RuntimeTaskRunOutcome::quiet(), RuntimeTaskRunOutcome::Quiet);
        assert_eq!(
            RuntimeTaskRunOutcome::succeeded(Some("ok".to_string())),
            RuntimeTaskRunOutcome::Succeeded {
                summary: Some("ok".to_string()),
                system_health: None,
            }
        );
        assert_eq!(
            RuntimeTaskRunOutcome::failed(Some("bad".to_string()), "boom"),
            RuntimeTaskRunOutcome::Failed {
                summary: Some("bad".to_string()),
                error: "boom".to_string(),
                system_health: None,
            }
        );
    }

    #[tokio::test]
    async fn quiet_runtime_task_outcome_does_not_create_record() {
        let state = test_state().await;
        let now = Utc::now();

        let task = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::TaskCleanup,
            now - Duration::seconds(1),
            now,
            &RuntimeTaskRunOutcome::quiet(),
        )
        .await
        .unwrap();

        assert!(task.is_none());
        assert!(
            find_latest_system_runtime_by_task_name(&state, SystemRuntimeTaskKind::TaskCleanup)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn succeeded_and_failed_runtime_task_runs_create_terminal_records() {
        let state = test_state().await;
        let started_at = Utc::now() - Duration::seconds(3);
        let finished_at = Utc::now();

        let succeeded = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::AuditCleanup,
            started_at,
            finished_at,
            &RuntimeTaskRunOutcome::succeeded(Some("audit cleanup done".to_string())),
        )
        .await
        .unwrap()
        .expect("succeeded runtime task should be recorded");
        assert_eq!(succeeded.status, BackgroundTaskStatus::Succeeded);
        assert_eq!(succeeded.progress_current, 1);
        assert_eq!(succeeded.progress_total, 1);
        assert_eq!(succeeded.status_text.as_deref(), Some("audit cleanup done"));
        assert!(succeeded.failure_can_retry.is_none());
        assert!(succeeded.result_json.is_some());

        let failed = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::TaskCleanup,
            started_at,
            finished_at,
            &RuntimeTaskRunOutcome::failed(Some("task cleanup failed".to_string()), "boom"),
        )
        .await
        .unwrap()
        .expect("failed runtime task should be recorded");
        assert_eq!(failed.status, BackgroundTaskStatus::Failed);
        assert_eq!(failed.progress_current, 0);
        assert_eq!(failed.progress_total, 1);
        assert_eq!(failed.last_error.as_deref(), Some("boom"));
        assert_eq!(failed.failure_can_retry, Some(false));
    }

    #[tokio::test]
    async fn scheduled_runtime_task_run_is_idempotent_for_same_due_time() {
        let state = test_state().await;
        let scheduled_at = Utc::now() - Duration::minutes(5);
        let first_start = scheduled_at + Duration::seconds(1);
        let first_finish = first_start + Duration::seconds(1);

        let first = record_scheduled_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::AuditCleanup,
            scheduled_at,
            first_start,
            first_finish,
            &RuntimeTaskRunOutcome::succeeded(Some("audit cleanup done".to_string())),
        )
        .await
        .unwrap()
        .expect("first scheduled runtime task should record");

        let second = record_scheduled_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::AuditCleanup,
            scheduled_at,
            first_start + Duration::seconds(10),
            first_finish + Duration::seconds(10),
            &RuntimeTaskRunOutcome::succeeded(Some("audit cleanup duplicate".to_string())),
        )
        .await
        .unwrap()
        .expect("duplicate scheduled runtime task should return existing row");

        assert_eq!(second.id, first.id);
        assert_eq!(second.status_text.as_deref(), Some("audit cleanup done"));
        assert_eq!(
            second.dedupe_key.as_deref(),
            Some(
                aster_forge_tasks::scheduled_task_dedupe_key(
                    "aster_yggdrasil",
                    SystemRuntimeTaskKind::AuditCleanup.as_str(),
                    scheduled_at,
                )
                .unwrap()
                .as_str()
            )
        );
    }

    #[tokio::test]
    async fn healthy_system_health_success_refreshes_latest_success_record() {
        let state = test_state().await;
        let first_start = Utc::now() - Duration::seconds(10);
        let first_finish = Utc::now() - Duration::seconds(9);

        let first = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::SystemHealthCheck,
            first_start,
            first_finish,
            &RuntimeTaskRunOutcome::succeeded_with_system_health(
                Some("healthy one".to_string()),
                health(RuntimeSystemHealthStatus::Healthy),
            ),
        )
        .await
        .unwrap()
        .expect("first health check should record");

        let second_start = Utc::now() - Duration::seconds(2);
        let second_finish = Utc::now();
        let second = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::SystemHealthCheck,
            second_start,
            second_finish,
            &RuntimeTaskRunOutcome::succeeded_with_system_health(
                Some("healthy two".to_string()),
                health(RuntimeSystemHealthStatus::Healthy),
            ),
        )
        .await
        .unwrap()
        .expect("second health check should refresh existing record");

        assert_eq!(second.id, first.id);
        assert_eq!(second.status_text.as_deref(), Some("healthy two"));
        assert_eq!(second.started_at, Some(second_start));
        assert_eq!(second.finished_at, Some(second_finish));

        let latest = find_latest_system_runtime_by_task_name(
            &state,
            SystemRuntimeTaskKind::SystemHealthCheck,
        )
        .await
        .unwrap()
        .expect("latest health check should exist");
        assert_eq!(latest.id, first.id);
    }

    #[tokio::test]
    async fn unhealthy_system_health_result_creates_new_failed_record() {
        let state = test_state().await;
        let started_at = Utc::now() - Duration::seconds(5);
        let finished_at = Utc::now();

        let failed = record_runtime_task_run(
            &state,
            SystemRuntimeTaskKind::SystemHealthCheck,
            started_at,
            finished_at,
            &RuntimeTaskRunOutcome::failed_with_system_health(
                Some("unhealthy".to_string()),
                "database down",
                health(RuntimeSystemHealthStatus::Unhealthy),
            ),
        )
        .await
        .unwrap()
        .expect("unhealthy health check should record failure");

        assert_eq!(failed.status, BackgroundTaskStatus::Failed);
        assert_eq!(failed.status_text.as_deref(), Some("unhealthy"));
        assert_eq!(failed.last_error.as_deref(), Some("database down"));
    }

    #[test]
    fn system_runtime_payload_json_uses_typed_task_shape() {
        let payload =
            system_runtime_payload_json(SystemRuntimeTaskKind::ExternalAuthFlowCleanup).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(payload.as_ref()).unwrap(),
            serde_json::json!({ "task_name": "external-auth-flow-cleanup" })
        );
    }
}
