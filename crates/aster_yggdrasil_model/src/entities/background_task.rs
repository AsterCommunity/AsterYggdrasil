//! SeaORM entity definition for `background_task`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::types::{
    task::BackgroundTaskKind, task::BackgroundTaskStatus, task::StoredTaskPayload,
    task::StoredTaskResult, task::StoredTaskRuntime, task::StoredTaskSteps,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = BackgroundTaskModel))]
#[sea_orm(table_name = "background_tasks")]
/// Tracked background task record, from enqueueing through retries to a terminal state.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Task type; selects which registered handler executes the payload.
    pub kind: BackgroundTaskKind,
    /// Task lifecycle state (`pending`, `processing`, `retry`, `succeeded`, `failed`, `canceled`).
    pub status: BackgroundTaskStatus,
    /// User who triggered the task; `None` for system-initiated work, cleared when that user is deleted.
    pub creator_user_id: Option<i64>,
    /// Human-readable title shown in admin task listings.
    pub display_name: String,
    /// Optional idempotency key; unique across tasks when present so repeated enqueues
    /// collapse onto one record.
    pub dedupe_key: Option<String>,
    /// Task input consumed by the handler, stored as JSON text.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub payload_json: StoredTaskPayload,
    /// Handler-produced output stored as JSON text once the task succeeds.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub result_json: Option<StoredTaskResult>,
    /// Runtime scratch state persisted between attempts, stored as JSON text.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub runtime_json: Option<StoredTaskRuntime>,
    /// Step list describing the task's progress plan, stored as JSON text.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub steps_json: Option<StoredTaskSteps>,
    /// Progress units completed so far, as reported by the handler.
    pub progress_current: i64,
    /// Total progress units for the whole task; `0` when progress is not tracked.
    pub progress_total: i64,
    /// Short human-readable progress message for display.
    pub status_text: Option<String>,
    /// Number of execution attempts made so far.
    pub attempt_count: i32,
    /// Attempt budget; the task fails permanently once exhausted.
    pub max_attempts: i32,
    /// When the task is next eligible to run.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub next_run_at: DateTimeUtc,
    /// Lease ownership token; changes on each claim so a stale worker cannot
    /// overwrite the state written by the current owner.
    pub processing_token: i64,
    /// When the current (or most recent) processing lease began.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub processing_started_at: Option<DateTimeUtc>,
    /// Last heartbeat received from the worker holding the lease.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_heartbeat_at: Option<DateTimeUtc>,
    /// When the current processing lease lapses and the task becomes reclaimable.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub lease_expires_at: Option<DateTimeUtc>,
    /// When the first execution attempt started.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub started_at: Option<DateTimeUtc>,
    /// When the task reached a terminal state.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub finished_at: Option<DateTimeUtc>,
    /// Message from the most recent failure.
    pub last_error: Option<String>,
    /// Whether the most recent failure may be retried; `None` until a failure occurs.
    pub failure_can_retry: Option<bool>,
    /// When the record becomes eligible for retention cleanup.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: DateTimeUtc,
    /// When the task was enqueued.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the task record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the task creator.
pub enum Relation {
    /// User who triggered the task, when one exists.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatorUserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl aster_forge_tasks::TaskRecord<crate::types::task::BackgroundTaskKind> for Model {
    fn id(&self) -> i64 {
        self.id
    }

    fn kind(&self) -> crate::types::task::BackgroundTaskKind {
        self.kind
    }

    fn payload_json(&self) -> &str {
        self.payload_json.as_ref()
    }

    fn result_json(&self) -> Option<&str> {
        self.result_json.as_ref().map(AsRef::as_ref)
    }
}

impl aster_forge_tasks::ClaimableTaskRecord<crate::types::task::BackgroundTaskKind> for Model {
    fn processing_token(&self) -> i64 {
        self.processing_token
    }
}

impl aster_forge_tasks::ExecutableTaskRecord<crate::types::task::BackgroundTaskKind> for Model {
    fn cancellation_requested(&self) -> bool {
        // The product has no user-facing task cancellation yet; reclaimed
        // snapshots are never awaiting a cancellation acknowledgement.
        false
    }

    fn attempt_count(&self) -> i32 {
        self.attempt_count
    }

    fn max_attempts(&self) -> i32 {
        self.max_attempts
    }
}
