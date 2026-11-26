use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveEnum, ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    sea_query::Expr,
};

use crate::errors::{AsterError, Result};
use aster_yggdrasil_model::entities::background_task::{self, Entity as BackgroundTask};
use aster_yggdrasil_model::types::task::{BackgroundTaskKind, BackgroundTaskStatus};
/// Inserts a new background task row from the given active model.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: background_task::ActiveModel,
) -> Result<background_task::Model> {
    model.insert(db).await.map_err(AsterError::from)
}

/// Fields for the idempotent success refresh applied to `SystemRuntime` tasks.
pub struct SystemRuntimeSuccessRefresh<'a> {
    /// Id of the `SystemRuntime` task row to refresh.
    pub id: i64,
    /// Serialized success result stored as `result_json`.
    pub result_json: &'a str,
    /// Optional human-readable status text replacing the previous one.
    pub status_text: Option<&'a str>,
    /// Next scheduled run time; also serves as the schedule bookkeeping for the next cycle.
    pub next_run_at: DateTime<Utc>,
    /// Run start time recorded for this refresh cycle.
    pub started_at: DateTime<Utc>,
    /// Run completion time; also written to `updated_at`.
    pub finished_at: DateTime<Utc>,
    /// Retention deadline after which cleanup may delete the row.
    pub expires_at: DateTime<Utc>,
}

/// Overwrites the terminal success state of an existing `Succeeded` `SystemRuntime` task.
///
/// Progress is reset to 1/1, lease and error fields are cleared, and the timestamps come from
/// the refresh payload. Returns `false` when no `Succeeded` `SystemRuntime` task with that id
/// exists, in which case nothing is modified.
pub async fn refresh_system_runtime_success<C: ConnectionTrait>(
    db: &C,
    refresh: SystemRuntimeSuccessRefresh<'_>,
) -> Result<bool> {
    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Succeeded.to_value()),
        )
        .col_expr(
            background_task::Column::ResultJson,
            Expr::value(Some(refresh.result_json.to_string())),
        )
        .col_expr(background_task::Column::ProgressCurrent, Expr::value(1))
        .col_expr(background_task::Column::ProgressTotal, Expr::value(1))
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(refresh.status_text.map(str::to_string)),
        )
        .col_expr(
            background_task::Column::NextRunAt,
            Expr::value(refresh.next_run_at),
        )
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StartedAt,
            Expr::value(Some(refresh.started_at)),
        )
        .col_expr(
            background_task::Column::FinishedAt,
            Expr::value(Some(refresh.finished_at)),
        )
        .col_expr(
            background_task::Column::LastError,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::FailureCanRetry,
            Expr::value(Option::<bool>::None),
        )
        .col_expr(
            background_task::Column::ExpiresAt,
            Expr::value(refresh.expires_at),
        )
        .col_expr(
            background_task::Column::UpdatedAt,
            Expr::value(refresh.finished_at),
        )
        .filter(background_task::Column::Id.eq(refresh.id))
        .filter(background_task::Column::Kind.eq(BackgroundTaskKind::SystemRuntime))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Succeeded))
        .exec(db)
        .await
        .map_err(AsterError::from)?;

    Ok(result.rows_affected == 1)
}

/// Fields for a progress report on a leased, processing task.
pub struct TaskProgressUpdate<'a> {
    /// Id of the task row receiving the progress update.
    pub id: i64,
    /// Processing token that must still own the task's lease.
    pub processing_token: i64,
    /// Instant used for the heartbeat and `updated_at`.
    pub now: DateTime<Utc>,
    /// Renewed lease deadline written alongside the heartbeat.
    pub lease_expires_at: DateTime<Utc>,
    /// Completed unit count; unit semantics are defined by the task kind.
    pub current: i64,
    /// Expected total unit count.
    pub total: i64,
    /// Optional human-readable status text.
    pub status_text: Option<&'a str>,
    /// Replaces the persisted step list when provided; left untouched when `None`.
    pub steps_json: Option<&'a str>,
}

/// Reports progress on a leased task and renews its heartbeat and lease in the same statement.
///
/// Returns `false` when the task is not `Processing` under `processing_token` (e.g. the lease
/// was lost); nothing is modified in that case.
pub async fn mark_progress<C: ConnectionTrait>(
    db: &C,
    update: TaskProgressUpdate<'_>,
) -> Result<bool> {
    let mut statement = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::ProgressCurrent,
            Expr::value(update.current),
        )
        .col_expr(
            background_task::Column::ProgressTotal,
            Expr::value(update.total),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(update.status_text.map(str::to_string)),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Some(update.now)),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Some(update.lease_expires_at)),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(update.now))
        .filter(background_task::Column::Id.eq(update.id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(update.processing_token));
    if let Some(steps_json) = update.steps_json {
        statement = statement.col_expr(
            background_task::Column::StepsJson,
            Expr::value(Some(steps_json.to_string())),
        );
    }
    let result = statement.exec(db).await.map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Replaces a leased task's `runtime_json`, clearing it when `runtime_json` is `None`.
///
/// Returns `false` when the task is not `Processing` under `processing_token`; nothing is
/// modified in that case.
pub async fn set_runtime_json<C: ConnectionTrait>(
    db: &C,
    id: i64,
    processing_token: i64,
    runtime_json: Option<&str>,
    now: DateTime<Utc>,
) -> Result<bool> {
    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::RuntimeJson,
            Expr::value(runtime_json.map(str::to_string)),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(now))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(processing_token))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Sets a leased task's display name while it is still processing.
///
/// Returns `false` when the task is not `Processing` under `processing_token`; nothing is
/// modified in that case.
pub async fn set_display_name<C: ConnectionTrait>(
    db: &C,
    id: i64,
    processing_token: i64,
    display_name: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::DisplayName,
            Expr::value(display_name.to_string()),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(now))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(processing_token))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Fields for transitioning a leased task to the terminal `Succeeded` state.
pub struct TaskSuccessUpdate<'a> {
    /// Id of the task row being completed.
    pub id: i64,
    /// Processing token that must still own the task's lease.
    pub processing_token: i64,
    /// Serialized success result stored as `result_json`; left empty when `None`.
    pub result_json: Option<&'a str>,
    /// Replaces the persisted step list when provided; left untouched when `None`.
    pub steps_json: Option<&'a str>,
    /// Final completed unit count.
    pub current: i64,
    /// Final expected total unit count.
    pub total: i64,
    /// Optional human-readable status text.
    pub status_text: Option<&'a str>,
    /// Completion time; also written to `updated_at`.
    pub finished_at: DateTime<Utc>,
    /// Retention deadline after which cleanup may delete the row.
    pub expires_at: DateTime<Utc>,
}

/// Transitions a leased task to `Succeeded`, stamping completion, retention, and lease clearing.
///
/// Returns `false` when the task is not `Processing` under `processing_token`; nothing is
/// modified in that case.
pub async fn mark_succeeded<C: ConnectionTrait>(
    db: &C,
    success: TaskSuccessUpdate<'_>,
) -> Result<bool> {
    let mut update = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Succeeded.to_value()),
        )
        .col_expr(
            background_task::Column::ResultJson,
            Expr::value(success.result_json.map(str::to_string)),
        )
        .col_expr(
            background_task::Column::ProgressCurrent,
            Expr::value(success.current),
        )
        .col_expr(
            background_task::Column::ProgressTotal,
            Expr::value(success.total),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(success.status_text.map(str::to_string)),
        )
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::FinishedAt,
            Expr::value(Some(success.finished_at)),
        )
        .col_expr(
            background_task::Column::ExpiresAt,
            Expr::value(success.expires_at),
        )
        .col_expr(
            background_task::Column::UpdatedAt,
            Expr::value(success.finished_at),
        )
        .filter(background_task::Column::Id.eq(success.id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(success.processing_token));
    if let Some(steps_json) = success.steps_json {
        update = update.col_expr(
            background_task::Column::StepsJson,
            Expr::value(Some(steps_json.to_string())),
        );
    }
    let result = update.exec(db).await.map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Moves a leased task back to `Retry` with a new scheduled run time, spending one attempt.
///
/// Clears the lease, records `last_error`, and optionally replaces the step list. Returns
/// `false` when the task is not `Processing` under `processing_token`; nothing is modified in
/// that case.
pub async fn mark_retry<C: ConnectionTrait>(
    db: &C,
    id: i64,
    processing_token: i64,
    attempt_count: i32,
    next_run_at: DateTime<Utc>,
    last_error: &str,
    steps_json: Option<&str>,
) -> Result<bool> {
    let mut update = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Retry.to_value()),
        )
        .col_expr(
            background_task::Column::AttemptCount,
            Expr::value(attempt_count),
        )
        .col_expr(background_task::Column::NextRunAt, Expr::value(next_run_at))
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::LastError,
            Expr::value(Some(last_error.to_string())),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(processing_token));
    if let Some(steps_json) = steps_json {
        update = update.col_expr(
            background_task::Column::StepsJson,
            Expr::value(Some(steps_json.to_string())),
        );
    }
    let result = update.exec(db).await.map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Returns a leased task to the dispatcher without spending retry budget.
///
/// Intended for cooperative shutdown only: the task goes back to `Pending` or `Retry` (other
/// statuses are rejected) and becomes immediately claimable. Crash recovery is handled by
/// normal lease expiry instead. Returns `false` when the task is not `Processing` under
/// `processing_token`.
pub async fn release_processing<C: ConnectionTrait>(
    db: &C,
    id: i64,
    processing_token: i64,
    now: DateTime<Utc>,
    status: BackgroundTaskStatus,
) -> Result<bool> {
    // This is only for cooperative shutdown. It gives the exact leased worker
    // row back to the dispatcher without spending retry budget or recording a
    // business failure. Crashes and stale workers are still handled by normal
    // lease expiry and reclaim.
    if !matches!(
        status,
        BackgroundTaskStatus::Pending | BackgroundTaskStatus::Retry
    ) {
        return Err(AsterError::internal_error(
            "released background task status must be pending or retry",
        ));
    }

    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(status.to_value()),
        )
        .col_expr(background_task::Column::NextRunAt, Expr::value(now))
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(Option::<String>::None),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(now))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(processing_token))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Fields for transitioning a leased task to the terminal `Failed` state.
pub struct TaskFailureUpdate<'a> {
    /// Id of the task row being marked as failed.
    pub id: i64,
    /// Processing token that must still own the task's lease.
    pub processing_token: i64,
    /// Attempt count to persist after this failure.
    pub attempt_count: i32,
    /// Error description stored as `last_error`.
    pub last_error: &'a str,
    /// Failure time; also written to `updated_at`.
    pub finished_at: DateTime<Utc>,
    /// Retention deadline after which cleanup may delete the row.
    pub expires_at: DateTime<Utc>,
    /// Replaces the persisted step list when provided; left untouched when `None`.
    pub steps_json: Option<&'a str>,
    /// Whether the failure could in principle be retried; persisted for admin display.
    pub failure_can_retry: bool,
}

/// Transitions a leased task to the terminal `Failed` state, recording the error and timestamps.
///
/// Returns `false` when the task is not `Processing` under `processing_token`; nothing is
/// modified in that case.
pub async fn mark_failed<C: ConnectionTrait>(
    db: &C,
    update: TaskFailureUpdate<'_>,
) -> Result<bool> {
    let mut statement = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Failed.to_value()),
        )
        .col_expr(
            background_task::Column::AttemptCount,
            Expr::value(update.attempt_count),
        )
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::LastError,
            Expr::value(Some(update.last_error.to_string())),
        )
        .col_expr(
            background_task::Column::FailureCanRetry,
            Expr::value(Some(update.failure_can_retry)),
        )
        .col_expr(
            background_task::Column::FinishedAt,
            Expr::value(Some(update.finished_at)),
        )
        .col_expr(
            background_task::Column::ExpiresAt,
            Expr::value(update.expires_at),
        )
        .col_expr(
            background_task::Column::UpdatedAt,
            Expr::value(update.finished_at),
        )
        .filter(background_task::Column::Id.eq(update.id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(update.processing_token));
    if let Some(steps_json) = update.steps_json {
        statement = statement.col_expr(
            background_task::Column::StepsJson,
            Expr::value(Some(steps_json.to_string())),
        );
    }
    let result = statement.exec(db).await.map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Resets a `Failed` task for a manual retry from the admin panel.
///
/// Clears state back to `Pending` with attempts and progress zeroed, `max_attempts` set to
/// `max_attempts`, and all result/error/lease fields cleared; the task becomes immediately
/// claimable. Returns `false` when no `Failed` task with that id exists.
pub async fn reset_for_manual_retry<C: ConnectionTrait>(
    db: &C,
    id: i64,
    now: DateTime<Utc>,
    max_attempts: i32,
    steps_json: Option<&str>,
) -> Result<bool> {
    let mut update = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Pending.to_value()),
        )
        .col_expr(background_task::Column::AttemptCount, Expr::value(0))
        .col_expr(background_task::Column::ProgressCurrent, Expr::value(0))
        .col_expr(
            background_task::Column::MaxAttempts,
            Expr::value(max_attempts),
        )
        .col_expr(background_task::Column::NextRunAt, Expr::value(now))
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StartedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::FinishedAt,
            Expr::value(Option::<DateTime<Utc>>::None),
        )
        .col_expr(
            background_task::Column::StatusText,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::LastError,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::ResultJson,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            background_task::Column::FailureCanRetry,
            Expr::value(Option::<bool>::None),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(now))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Failed));
    if let Some(steps_json) = steps_json {
        update = update.col_expr(
            background_task::Column::StepsJson,
            Expr::value(Some(steps_json.to_string())),
        );
    }
    let result = update.exec(db).await.map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}
