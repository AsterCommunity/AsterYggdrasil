use std::collections::{HashMap, HashSet};

use chrono::Utc;
use sea_orm::DatabaseConnection;

use crate::config::operations;
use crate::db::repository::{background_task_repo, user_repo};
use crate::entities::{background_task, user};
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState, TaskRuntimeState};
use crate::services::audit_service;
use crate::types::{BackgroundTaskKind, BackgroundTaskStatus};
use crate::utils::numbers::i64_to_i32;
use aster_forge_api::{CursorPage, DateTimeIdCursor};

use super::{
    TaskCreatorSummary, TaskInfo, TaskResult, build_task_presentation,
    cleanup_task_temp_dir_for_task, decode_task_payload, decode_task_result, parse_task_steps_json,
    registry, serialize_task_steps,
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AdminTaskListFilters {
    pub(crate) kind: Option<BackgroundTaskKind>,
    pub(crate) status: Option<BackgroundTaskStatus>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdminTaskCleanupFilters {
    pub(crate) finished_before: chrono::DateTime<chrono::Utc>,
    pub(crate) kind: Option<BackgroundTaskKind>,
    pub(crate) status: Option<BackgroundTaskStatus>,
}

pub(crate) async fn list_tasks_paginated_for_admin(
    state: &(impl DatabaseRuntimeState + RuntimeConfigRuntimeState),
    limit: u64,
    filters: AdminTaskListFilters,
    cursor: Option<(chrono::DateTime<chrono::Utc>, i64)>,
) -> Result<CursorPage<TaskInfo, DateTimeIdCursor>> {
    let limit = limit.clamp(1, operations::task_list_max_limit(state.runtime_config()));
    tracing::debug!(
        limit,
        kind = ?filters.kind,
        status = ?filters.status,
        "admin listing background tasks"
    );
    let page = background_task_repo::find_cursor_filtered(
        state.writer_db(),
        limit,
        &background_task_repo::AdminTaskFilters {
            kind: filters.kind,
            status: filters.status,
        },
        cursor,
    )
    .await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|task| DateTimeIdCursor {
            value: task.updated_at,
            id: task.id,
        })
    } else {
        None
    };

    let items = page
        .items
        .into_iter()
        .map(|task| build_task_info(task, None))
        .collect::<Result<Vec<_>>>()?;
    let items = hydrate_task_creators(state.reader_db(), items).await?;
    tracing::debug!(
        returned = items.len(),
        total = page.total,
        limit,
        "admin listed background tasks"
    );
    Ok(CursorPage::new(items, page.total, limit, next_cursor))
}

pub(crate) async fn cleanup_tasks_for_admin(
    state: &impl DatabaseRuntimeState,
    filters: AdminTaskCleanupFilters,
) -> Result<u64> {
    tracing::debug!(
        finished_before = %filters.finished_before,
        kind = ?filters.kind,
        status = ?filters.status,
        "admin cleaning up background tasks"
    );
    validate_admin_task_cleanup_status(filters.status)?;
    let removed = background_task_repo::delete_terminal_by_filters(
        state.writer_db(),
        &background_task_repo::TerminalTaskCleanupFilters {
            finished_before: filters.finished_before,
            kind: filters.kind,
            status: filters.status,
        },
    )
    .await?;
    tracing::debug!(removed, "admin cleaned up background tasks");
    Ok(removed)
}

pub(crate) async fn retry_task_for_admin(
    state: &impl TaskRuntimeState,
    task_id: i64,
) -> Result<TaskInfo> {
    tracing::debug!(task_id, "admin retrying background task");
    let task = background_task_repo::find_by_id(state.writer_db(), task_id).await?;
    retry_task_record(state, &task).await?;
    let task = background_task_repo::find_by_id(state.writer_db(), task_id).await?;
    let info = build_task_info_with_creator(state.reader_db(), task).await?;
    tracing::debug!(
        task_id,
        kind = ?info.kind,
        status = ?info.status,
        "admin retried background task"
    );
    Ok(info)
}

pub(crate) async fn retry_task_for_admin_with_audit(
    state: &impl TaskRuntimeState,
    task_id: i64,
    audit_ctx: &audit_service::AuditContext,
) -> Result<TaskInfo> {
    tracing::debug!(task_id, "admin retrying background task with audit");
    let previous = background_task_repo::find_by_id(state.writer_db(), task_id).await?;
    retry_task_record(state, &previous).await?;
    let task = background_task_repo::find_by_id(state.writer_db(), task_id).await?;
    let task_info = build_task_info_with_creator(state.reader_db(), task).await?;
    audit_service::log_with_details(
        state,
        audit_ctx,
        audit_service::AuditAction::TaskRetry,
        audit_service::AuditEntityType::Task,
        Some(task_info.id),
        Some(&task_info.display_name),
        || {
            audit_service::details(audit_service::TaskRetryAuditDetails {
                kind: previous.kind.to_string(),
                previous_attempt_count: previous.attempt_count,
            })
        },
    )
    .await;
    tracing::debug!(
        task_id,
        previous_attempt_count = previous.attempt_count,
        "admin retried background task with audit"
    );
    Ok(task_info)
}

async fn retry_task_record(
    state: &impl TaskRuntimeState,
    task: &background_task::Model,
) -> Result<()> {
    if task.status != BackgroundTaskStatus::Failed {
        tracing::debug!(
            task_id = task.id,
            status = ?task.status,
            "task retry rejected because task is not failed"
        );
        return Err(AsterError::validation_error(
            "only failed tasks can be retried",
        ));
    }
    if !task_can_retry(task) {
        tracing::debug!(
            task_id = task.id,
            failure_can_retry = task.failure_can_retry,
            "task retry rejected because failure cannot be retried"
        );
        return Err(AsterError::validation_error(
            "this task failure cannot be retried",
        ));
    }

    cleanup_task_temp_dir_for_task(state, task.id).await?;
    let steps_json = serialize_task_steps(&registry::initial_task_steps(task.kind))?;
    let max_attempts = registry::max_attempts(state.runtime_config().as_ref(), task.kind);
    let now = Utc::now();

    if !background_task_repo::reset_for_manual_retry(
        state.writer_db(),
        task.id,
        now,
        max_attempts,
        Some(steps_json.as_ref()),
    )
    .await?
    {
        return Err(AsterError::internal_error(format!(
            "failed to reset task #{} for retry",
            task.id
        )));
    }
    tracing::debug!(
        task_id = task.id,
        kind = ?task.kind,
        max_attempts,
        "task reset for manual retry"
    );
    state.wake_background_task_dispatcher();
    Ok(())
}

async fn build_task_info_with_creator(
    db: &DatabaseConnection,
    task: background_task::Model,
) -> Result<TaskInfo> {
    let creator = match task.creator_user_id {
        Some(user_id) => Some(TaskCreatorSummary::from(
            user_repo::find_by_id(db, user_id).await?,
        )),
        None => None,
    };
    build_task_info(task, creator)
}

async fn hydrate_task_creators(
    db: &DatabaseConnection,
    tasks: Vec<TaskInfo>,
) -> Result<Vec<TaskInfo>> {
    let creator_ids = tasks
        .iter()
        .filter_map(|task| task.creator_user_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if creator_ids.is_empty() {
        tracing::debug!(
            task_count = tasks.len(),
            "task creator hydration skipped because no tasks have creators"
        );
        return Ok(tasks);
    }

    tracing::debug!(
        task_count = tasks.len(),
        creator_count = creator_ids.len(),
        "hydrating task creators"
    );
    let creators = user_repo::find_by_ids(db, &creator_ids)
        .await?
        .into_iter()
        .map(|user| (user.id, TaskCreatorSummary::from(user)))
        .collect::<HashMap<_, _>>();

    Ok(tasks
        .into_iter()
        .map(|mut task| {
            task.creator = task
                .creator_user_id
                .and_then(|user_id| creators.get(&user_id).cloned());
            task
        })
        .collect())
}

pub(in crate::services::task_service) fn build_task_info(
    task: background_task::Model,
    creator: Option<TaskCreatorSummary>,
) -> Result<TaskInfo> {
    tracing::debug!(
        task_id = task.id,
        kind = ?task.kind,
        status = ?task.status,
        "building task info"
    );
    let progress_percent = if task.progress_total <= 0 {
        if task.status == BackgroundTaskStatus::Succeeded {
            100
        } else {
            0
        }
    } else {
        i64_to_i32(
            ((task.progress_current.saturating_mul(100)) / task.progress_total).clamp(0, 100),
            "task progress percent",
        )?
    };
    let payload = decode_task_payload(&task)?;
    let result = decode_task_result_or_none(&task);
    let steps = parse_task_steps_json(task.steps_json.as_ref().map(|raw| raw.as_ref()))?;
    let can_retry = task_can_retry(&task);
    let presentation = build_task_presentation(
        &payload,
        result.as_ref(),
        task.status,
        task.last_error.as_deref(),
    );

    Ok(TaskInfo {
        id: task.id,
        kind: task.kind,
        status: task.status,
        display_name: task.display_name,
        creator_user_id: task.creator_user_id,
        creator,
        progress_current: task.progress_current,
        progress_total: task.progress_total,
        progress_percent,
        status_text: task.status_text,
        attempt_count: task.attempt_count,
        max_attempts: task.max_attempts,
        last_error: task.last_error,
        payload,
        result,
        steps,
        can_retry,
        presentation,
        lease_expires_at: task.lease_expires_at,
        started_at: task.started_at,
        finished_at: task.finished_at,
        expires_at: task.expires_at,
        created_at: task.created_at,
        updated_at: task.updated_at,
    })
}

impl From<user::Model> for TaskCreatorSummary {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
        }
    }
}

fn decode_task_result_or_none(task: &background_task::Model) -> Option<TaskResult> {
    match decode_task_result(task) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(
                task_id = task.id,
                error = %error,
                "failed to decode background task result; continuing without result"
            );
            None
        }
    }
}

fn task_can_retry(task: &background_task::Model) -> bool {
    task.status == BackgroundTaskStatus::Failed && task.failure_can_retry.unwrap_or(true)
}

pub(in crate::services::task_service) fn validate_admin_task_cleanup_status(
    status: Option<BackgroundTaskStatus>,
) -> Result<()> {
    if status.is_some_and(|value| !value.is_terminal()) {
        tracing::debug!(
            status = ?status,
            "admin task cleanup rejected non-terminal status"
        );
        return Err(AsterError::validation_error(
            "only completed task statuses can be cleaned up",
        ));
    }
    Ok(())
}
