//! Background task DTOs and stored payload shapes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use super::runtime::SystemRuntimeTaskKind;
use aster_yggdrasil_model::types::task::{BackgroundTaskKind, BackgroundTaskStatus};

/// Stable presentation codes the admin UI maps to localized strings; the
/// wire values must not drift once a frontend ships against them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TaskPresentationCode {
    /// Status line for a health-check task that found problems; params carry
    /// the aggregate status and the unhealthy components.
    RuntimeSystemHealthIssueDetail,
    /// Title line for the audit log cleanup runtime task.
    RuntimeTaskAuditCleanup,
    /// Title line for the auth session cleanup runtime task.
    RuntimeTaskAuthSessionCleanup,
    /// Title line for the background task dispatch runtime task.
    RuntimeTaskBackgroundTaskDispatch,
    /// Title line for the external auth flow cleanup runtime task.
    RuntimeTaskExternalAuthFlowCleanup,
    /// Title line for the mail outbox dispatch runtime task.
    RuntimeTaskMailOutboxDispatch,
    /// Title line for the system health check runtime task.
    RuntimeTaskSystemHealthCheck,
    /// Title line for the task artifact cleanup runtime task.
    RuntimeTaskTaskCleanup,
    /// Title line for the Yggdrasil storage consistency check runtime task.
    RuntimeTaskYggdrasilStorageConsistencyCheck,
    /// Title line for the Yggdrasil texture cleanup runtime task.
    RuntimeTaskYggdrasilTextureCleanup,
    /// Title line for the Yggdrasil token cleanup runtime task.
    RuntimeTaskYggdrasilTokenCleanup,
    /// Status line for a task that finished failed; params may carry the
    /// last error.
    StatusTextFailed,
    /// Status line for a task that ended quietly (canceled, nothing done).
    StatusTextQuiet,
    /// Status line for a task that finished successfully.
    StatusTextSucceeded,
    /// Status line for a health-check task whose report is fully healthy.
    StatusTextSystemHealthy,
    /// Title line for a texture preview generation task; params carry the
    /// texture hash being previewed.
    TexturePreviewGenerateHash,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Localizable display message for one task line: a presentation code plus
/// the parameters the client substitutes into the translated template.
pub struct TaskPresentationMessage {
    /// Presentation code the client maps to a translation key.
    pub code: TaskPresentationCode,
    /// Template parameters keyed by placeholder name; omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, serde_json::Value>,
}

/// Localized title/status lines for a task, so the admin UI can render
/// human-readable task entries without knowing task internals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct TaskPresentation {
    /// What the task is, for task kinds that have a stable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<TaskPresentationMessage>,
    /// How the task ended or is doing, when the state has a display line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskPresentationMessage>,
}

/// The user who created a task, joined into admin task listings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct TaskCreatorSummary {
    /// Creator's user ID; absent from the listing when the account is gone.
    pub id: i64,
    /// Creator's username at query time.
    pub username: String,
    /// Creator's email at query time.
    pub email: String,
}

/// Strongly typed wire name for a system runtime task; known kinds resolve to
/// [`SystemRuntimeTaskKind`], unknown strings are preserved verbatim for
/// forward compatibility.
pub type RuntimeTaskName = aster_forge_tasks::RuntimeTaskName<SystemRuntimeTaskKind>;

/// Generation target for the `texture_preview_generate` task: the texture hash
/// plus the skin model it binds to. The preview spec (engine, size, quality)
/// is intentionally absent — the worker always renders with the runtime
/// config's current spec, so spec changes are picked up by the next task
/// without re-enqueueing by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct TexturePreviewGenerateTaskPayload {
    /// Hash of the texture the preview renders.
    pub texture_hash: String,
    /// Skin model the texture is bound to, which decides the render shape.
    pub texture_model: aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel,
}

/// Result stored by a finished texture preview generation task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct TexturePreviewGenerateTaskResult {
    /// Preview file name the task ensured in object storage; a task that found
    /// the preview already cached reports the existing name with
    /// `reused_existing = true`.
    pub file_name: String,
    /// True when an up-to-date preview was already in storage and no render
    /// work was done.
    pub reused_existing: bool,
}

/// Stored payload identifying which system runtime task a background record
/// drives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RuntimeTaskPayload {
    /// Wire name of the runtime task this record represents.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub task_name: RuntimeTaskName,
}

/// Aggregate or per-component system health verdict reported by runtime
/// health-check tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSystemHealthStatus {
    /// Component or system fully operational.
    Healthy,
    /// Operational but degraded; functionality is available with fallbacks.
    Degraded,
    /// Not operational; the readiness gate must fail closed.
    Unhealthy,
}

/// One runtime component's health verdict inside a system health report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RuntimeSystemHealthComponent {
    /// Stable component identifier (e.g. `database`, `cache`).
    pub name: String,
    /// Verdict for this component.
    pub status: RuntimeSystemHealthStatus,
    /// Human-readable explanation of the verdict.
    pub message: String,
    /// Structured key/value details for diagnostics; omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<aster_forge_runtime::HealthComponentDetail>,
}

/// System health report captured by a runtime health-check task, consumed by
/// the admin overview and the task status line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RuntimeSystemHealthResult {
    /// Worst verdict across all checked components.
    pub status: RuntimeSystemHealthStatus,
    /// Per-component verdicts.
    pub components: Vec<RuntimeSystemHealthComponent>,
}

/// Stored result of one system runtime task run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RuntimeTaskResult {
    /// Wall-clock run duration in milliseconds; clamped to zero when the
    /// finished timestamp precedes the started one.
    pub duration_ms: i64,
    /// Short human-readable outcome line, when the task produced one.
    pub summary: Option<String>,
    /// Health report snapshot, for tasks that check component health.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_health: Option<RuntimeSystemHealthResult>,
}

impl RuntimeTaskResult {
    /// Builds a result from run timestamps, deriving the duration and
    /// clamping clock skew so `duration_ms` never goes negative.
    pub fn from_timestamps(
        started_at: chrono::DateTime<chrono::Utc>,
        finished_at: chrono::DateTime<chrono::Utc>,
        summary: Option<String>,
        system_health: Option<RuntimeSystemHealthResult>,
    ) -> Self {
        Self {
            duration_ms: (finished_at - started_at).num_milliseconds().max(0),
            summary,
            system_health,
        }
    }
}

/// Typed stored payload for a background task; the tag drives deserialization
/// of the record's payload JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskPayload {
    /// A system runtime maintenance task run.
    SystemRuntime(RuntimeTaskPayload),
    /// A texture preview generation task.
    TexturePreviewGenerate(TexturePreviewGenerateTaskPayload),
}

/// Typed stored result for a finished background task; variants mirror
/// [`TaskPayload`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskResult {
    /// Result of a system runtime maintenance task run.
    SystemRuntime(RuntimeTaskResult),
    /// Result of a texture preview generation task.
    TexturePreviewGenerate(TexturePreviewGenerateTaskResult),
}

/// One background task as served to the admin task management API: the task
/// record plus its lifecycle, progress, and localized presentation.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct TaskInfo {
    /// Task record ID.
    pub id: i64,
    /// Broad task class (system runtime vs. typed product tasks).
    pub kind: BackgroundTaskKind,
    /// Current lifecycle status (pending, processing, retry, succeeded,
    /// failed, canceled).
    pub status: BackgroundTaskStatus,
    /// Human-readable task name for listings.
    pub display_name: String,
    /// ID of the user who enqueued the task; `None` for system-created tasks.
    pub creator_user_id: Option<i64>,
    /// Creator account summary, joined when the account still exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<TaskCreatorSummary>,
    /// Units of work completed so far.
    pub progress_current: i64,
    /// Total units of work the task expects to process.
    pub progress_total: i64,
    /// Progress as 0-100 percent, derived from current/total.
    pub progress_percent: i32,
    /// Plain-text status line, when the task reported one.
    pub status_text: Option<String>,
    /// Attempts made so far, including the current one.
    pub attempt_count: i32,
    /// Attempts allowed before the task gives up and stays failed.
    pub max_attempts: i32,
    /// Error message from the most recent failed attempt.
    pub last_error: Option<String>,
    /// Typed payload describing the work.
    pub payload: TaskPayload,
    /// Typed result, present once the task reached a terminal state.
    pub result: Option<TaskResult>,
    /// Per-step execution trace recorded while the task ran.
    pub steps: Vec<aster_forge_tasks::TaskStepInfo>,
    /// Whether an admin may retry this task from its current state.
    pub can_retry: bool,
    /// Localized title/status lines for admin UI display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation: Option<TaskPresentation>,
    /// When the current dispatch lease lapses; a worker holding an expired
    /// lease must stop writing terminal state. Absent while not leased.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub lease_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the first attempt began (UTC), if the task has started.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the task reached a terminal state (UTC), if it has.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the task record becomes eligible for cleanup (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// When the task record was created (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the task record was last touched (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeSystemHealthComponent, RuntimeSystemHealthResult, RuntimeSystemHealthStatus,
        RuntimeTaskName, RuntimeTaskPayload, RuntimeTaskResult, TaskPayload, TaskPresentationCode,
        TaskPresentationMessage, TaskResult,
    };
    use crate::services::ops::task::runtime::SystemRuntimeTaskKind;
    use aster_forge_runtime::{HealthComponentDetail, HealthComponentDetailValue};
    use chrono::{Duration, Utc};
    use std::collections::BTreeMap;

    #[test]
    fn runtime_task_name_round_trips_known_and_legacy_wire_values() {
        let known = RuntimeTaskName::from("system-health-check");
        assert_eq!(
            known.known(),
            Some(SystemRuntimeTaskKind::SystemHealthCheck)
        );
        assert_eq!(known.as_str(), "system-health-check");
        assert_eq!(known.display_name(), "System health check");
        assert_eq!(
            serde_json::to_string(&known).unwrap(),
            r#""system-health-check""#
        );

        let legacy = RuntimeTaskName::from("legacy-cleanup-job");
        assert_eq!(legacy.known(), None);
        assert_eq!(legacy.as_str(), "legacy-cleanup-job");
        assert_eq!(legacy.display_name(), "legacy cleanup job");
        assert_eq!(legacy.to_string(), "legacy-cleanup-job");

        let decoded: RuntimeTaskName = serde_json::from_str(r#""task-cleanup""#).unwrap();
        assert_eq!(decoded.known(), Some(SystemRuntimeTaskKind::TaskCleanup));
    }

    #[test]
    fn runtime_task_result_duration_never_goes_negative() {
        let started_at = Utc::now();
        let finished_at = started_at + Duration::milliseconds(42);
        let result = RuntimeTaskResult::from_timestamps(
            started_at,
            finished_at,
            Some("done".to_string()),
            None,
        );
        assert_eq!(result.duration_ms, 42);
        assert_eq!(result.summary.as_deref(), Some("done"));

        let backwards = RuntimeTaskResult::from_timestamps(finished_at, started_at, None, None);
        assert_eq!(backwards.duration_ms, 0);
    }

    #[test]
    fn task_payload_and_result_use_tagged_wire_shape() {
        let payload = TaskPayload::SystemRuntime(RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::AuditCleanup),
        });
        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            serde_json::json!({
                "kind": "system_runtime",
                "task_name": "audit-cleanup",
            })
        );

        let result = TaskResult::SystemRuntime(RuntimeTaskResult {
            duration_ms: 12,
            summary: Some("ok".to_string()),
            system_health: Some(RuntimeSystemHealthResult {
                status: RuntimeSystemHealthStatus::Degraded,
                components: vec![RuntimeSystemHealthComponent {
                    name: "database".to_string(),
                    status: RuntimeSystemHealthStatus::Healthy,
                    message: "ok".to_string(),
                    details: vec![HealthComponentDetail::new("backend", "sqlite")],
                }],
            }),
        });
        let encoded = serde_json::to_value(&result).unwrap();
        assert_eq!(encoded["kind"], "system_runtime");
        assert_eq!(encoded["duration_ms"], 12);
        assert_eq!(encoded["system_health"]["status"], "degraded");
        assert_eq!(
            encoded["system_health"]["components"][0]["details"][0]["key"],
            "backend"
        );
        assert_eq!(
            encoded["system_health"]["components"][0]["details"][0]["value"],
            serde_json::json!({ "type": "text", "value": "sqlite" })
        );
        let TaskResult::SystemRuntime(runtime_result) = &result else {
            panic!("expected system runtime result variant");
        };
        assert_eq!(
            runtime_result
                .system_health
                .as_ref()
                .expect("system health should be present")
                .components[0]
                .details[0]
                .value,
            HealthComponentDetailValue::Text("sqlite".to_string())
        );
    }

    #[test]
    fn task_presentation_message_omits_empty_params() {
        let empty = TaskPresentationMessage {
            code: TaskPresentationCode::StatusTextSucceeded,
            params: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            serde_json::json!({ "code": "status_text_succeeded" })
        );

        let mut params = BTreeMap::new();
        params.insert("task".to_string(), serde_json::json!("cleanup"));
        let message = TaskPresentationMessage {
            code: TaskPresentationCode::RuntimeTaskYggdrasilTextureCleanup,
            params,
        };
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({
                "code": "runtime_task_yggdrasil_texture_cleanup",
                "params": { "task": "cleanup" },
            })
        );
    }
}
