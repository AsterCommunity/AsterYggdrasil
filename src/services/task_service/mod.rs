//! Persisted background task subsystem.
#![allow(dead_code)]

mod admin;
mod create;
pub(crate) mod dispatch;
mod lease;
mod presentation;
mod registry;
mod retry;
pub(crate) mod runtime;
mod spec;
mod steps;
#[cfg(test)]
mod tests;
pub mod types;

use chrono::{Duration, Utc};
use sea_orm::DatabaseConnection;

use crate::config::operations;
use crate::db::repository::background_task_repo;
use crate::errors::{AsterError, Result};
use crate::runtime::{AppConfigRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::types::StoredTaskResult;
use aster_forge_tasks::{TaskLease, TaskLeaseGuard};

pub(crate) use admin::{
    AdminTaskCleanupFilters, AdminTaskListFilters, cleanup_tasks_for_admin,
    list_tasks_paginated_for_admin, retry_task_for_admin_with_audit,
};
pub(in crate::services::task_service) use create::{TypedTaskCreate, insert_typed_task_record};
pub use dispatch::{cleanup_expired, dispatch_due, drain};
use lease::{
    is_task_lease_lost, is_task_lease_renewal_timed_out, is_task_worker_shutdown_requested,
};
use presentation::build_task_presentation;
use registry::{decode_task_payload, decode_task_result};
pub use runtime::{RuntimeTaskRunOutcome, SystemRuntimeTaskKind, record_runtime_task_run};
use spec::BackgroundTaskSpec;
use steps::{parse_task_steps_json, serialize_task_steps};
use types::{TaskCreatorSummary, TaskInfo, TaskResult};

pub(super) const DEFAULT_TASK_RETENTION_HOURS: i64 = 24;
pub(super) const TASK_HEARTBEAT_INTERVAL_SECS: u64 = 10;
pub(super) const TASK_PROCESSING_STALE_SECS: i64 = 60;
pub(super) const TASK_DISPLAY_NAME_MAX_LEN: usize = 512;
pub(super) const TASK_LAST_ERROR_MAX_LEN: usize = 1024;
pub(super) const TASK_STATUS_TEXT_MAX_LEN: usize = 255;
pub(super) const TASK_DRAIN_MAX_ROUNDS: usize = 32;

pub(in crate::services::task_service) async fn mark_task_progress(
    state: &impl DatabaseRuntimeState,
    lease_guard: &TaskLeaseGuard,
    current: i64,
    total: i64,
    status_text: Option<&str>,
    steps: &[aster_forge_tasks::TaskStepInfo],
) -> Result<()> {
    update_task_progress_db(
        state.writer_db(),
        lease_guard,
        current,
        total,
        status_text,
        steps,
    )
    .await
}

pub(in crate::services::task_service) async fn update_task_progress_db(
    db: &DatabaseConnection,
    lease_guard: &TaskLeaseGuard,
    current: i64,
    total: i64,
    status_text: Option<&str>,
    steps: &[aster_forge_tasks::TaskStepInfo],
) -> Result<()> {
    let status_text = status_text.map(truncate_status_text);
    let steps_json = serialize_task_steps(steps)?;
    let lease = lease_guard.lease();
    let now = Utc::now();
    tracing::debug!(
        task_id = lease.task_id,
        processing_token = lease.processing_token,
        current,
        total,
        has_status_text = status_text.is_some(),
        step_count = steps.len(),
        "updating background task progress"
    );
    if background_task_repo::mark_progress(
        db,
        background_task_repo::TaskProgressUpdate {
            id: lease.task_id,
            processing_token: lease.processing_token,
            now,
            lease_expires_at: task_lease_expires_at(now),
            current,
            total,
            status_text: status_text.as_deref(),
            steps_json: Some(steps_json.as_ref()),
        },
    )
    .await?
    {
        lease_guard.record_renewed();
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task progress updated"
        );
        Ok(())
    } else {
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task progress update lost lease"
        );
        Err(AsterError::from(lease_guard.mark_lost()))
    }
}

pub(in crate::services::task_service) async fn set_task_runtime_json(
    state: &impl DatabaseRuntimeState,
    lease_guard: &TaskLeaseGuard,
    runtime_json: Option<&str>,
) -> Result<()> {
    let lease = lease_guard.lease();
    let now = Utc::now();
    tracing::debug!(
        task_id = lease.task_id,
        processing_token = lease.processing_token,
        has_runtime_json = runtime_json.is_some(),
        "setting background task runtime json"
    );
    if background_task_repo::set_runtime_json(
        state.writer_db(),
        lease.task_id,
        lease.processing_token,
        runtime_json,
        now,
    )
    .await?
    {
        lease_guard.record_renewed();
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task runtime json set"
        );
        Ok(())
    } else {
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task runtime json update lost lease"
        );
        Err(AsterError::from(lease_guard.mark_lost()))
    }
}

pub(in crate::services::task_service) async fn set_task_display_name(
    state: &impl DatabaseRuntimeState,
    lease_guard: &TaskLeaseGuard,
    display_name: &str,
) -> Result<()> {
    let lease = lease_guard.lease();
    let now = Utc::now();
    let display_name = truncate_display_name(display_name);
    tracing::debug!(
        task_id = lease.task_id,
        processing_token = lease.processing_token,
        display_name_len = display_name.len(),
        "setting background task display name"
    );
    if background_task_repo::set_display_name(
        state.writer_db(),
        lease.task_id,
        lease.processing_token,
        &display_name,
        now,
    )
    .await?
    {
        lease_guard.record_renewed();
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task display name set"
        );
        Ok(())
    } else {
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task display name update lost lease"
        );
        Err(AsterError::from(lease_guard.mark_lost()))
    }
}

pub(in crate::services::task_service) async fn mark_task_succeeded(
    state: &(impl DatabaseRuntimeState + RuntimeConfigRuntimeState),
    lease_guard: &TaskLeaseGuard,
    result_json: Option<&StoredTaskResult>,
    current: i64,
    total: i64,
    status_text: Option<&str>,
    steps: &[aster_forge_tasks::TaskStepInfo],
) -> Result<()> {
    let now = Utc::now();
    let status_text = status_text.map(truncate_status_text);
    let steps_json = serialize_task_steps(steps)?;
    let lease = lease_guard.lease();
    tracing::debug!(
        task_id = lease.task_id,
        processing_token = lease.processing_token,
        current,
        total,
        has_result = result_json.is_some(),
        has_status_text = status_text.is_some(),
        step_count = steps.len(),
        "marking background task succeeded"
    );
    if background_task_repo::mark_succeeded(
        state.writer_db(),
        background_task_repo::TaskSuccessUpdate {
            id: lease.task_id,
            processing_token: lease.processing_token,
            result_json: result_json.map(AsRef::as_ref),
            steps_json: Some(steps_json.as_ref()),
            current,
            total,
            status_text: status_text.as_deref(),
            finished_at: now,
            expires_at: task_expiration_from(state, now),
        },
    )
    .await?
    {
        lease_guard.record_renewed();
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task marked succeeded"
        );
        Ok(())
    } else {
        tracing::debug!(
            task_id = lease.task_id,
            processing_token = lease.processing_token,
            "background task success update lost lease"
        );
        Err(AsterError::from(lease_guard.mark_lost()))
    }
}

pub(in crate::services::task_service) async fn prepare_task_temp_dir(
    state: &impl AppConfigRuntimeState,
    lease: TaskLease,
) -> Result<String> {
    prepare_task_temp_dir_in_root(&state.config().server.temp_dir, lease).await
}

pub(in crate::services::task_service) async fn prepare_task_temp_dir_in_root(
    temp_root: &str,
    lease: TaskLease,
) -> Result<String> {
    aster_forge_tasks::prepare_task_temp_dir_in_root(temp_root, lease)
        .await
        .map_err(AsterError::from)
}

pub(in crate::services::task_service) async fn cleanup_task_temp_dir_for_lease_in_root(
    temp_root: &str,
    lease: TaskLease,
) -> Result<()> {
    aster_forge_tasks::cleanup_task_temp_dir_for_lease_in_root(temp_root, lease)
        .await
        .map_err(AsterError::from)
}

pub(super) async fn cleanup_task_temp_dir_for_task(
    state: &impl AppConfigRuntimeState,
    task_id: i64,
) -> Result<()> {
    tracing::debug!(task_id, "cleaning background task temp dir");
    cleanup_task_temp_dir_for_task_in_root(&state.config().server.temp_dir, task_id).await
}

pub(super) async fn cleanup_task_temp_dir_for_task_in_root(
    temp_root: &str,
    task_id: i64,
) -> Result<()> {
    aster_forge_tasks::cleanup_task_temp_dir_for_task_in_root(temp_root, task_id)
        .await
        .map_err(AsterError::from)
}

pub(super) fn task_expiration_from(
    state: &impl RuntimeConfigRuntimeState,
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    now + Duration::hours(load_task_retention_hours(state))
}

pub(super) fn task_lease_expires_at(
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    aster_forge_tasks::task_lease_expires_at(now, TASK_PROCESSING_STALE_SECS)
}

fn load_task_retention_hours(state: &impl RuntimeConfigRuntimeState) -> i64 {
    let Some(raw) = state
        .runtime_config()
        .get(operations::TASK_RETENTION_HOURS_KEY)
    else {
        return DEFAULT_TASK_RETENTION_HOURS;
    };
    match raw.parse::<i64>() {
        Ok(hours) if hours > 0 => hours,
        _ => {
            tracing::warn!(
                "invalid task_retention_hours value '{}', using default",
                raw
            );
            DEFAULT_TASK_RETENTION_HOURS
        }
    }
}

pub(super) fn truncate_display_name(value: &str) -> String {
    aster_forge_utils::text::truncate_utf8_to_max_bytes(value, TASK_DISPLAY_NAME_MAX_LEN)
}

pub(super) fn truncate_status_text(value: &str) -> String {
    value.chars().take(TASK_STATUS_TEXT_MAX_LEN).collect()
}

pub(super) fn truncate_error(error: &str) -> String {
    error.chars().take(TASK_LAST_ERROR_MAX_LEN).collect()
}
