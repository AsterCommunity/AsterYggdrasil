use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, Condition, QueryFilter, Select};

use aster_yggdrasil_model::entities::background_task::{self, Entity as BackgroundTask};
use aster_yggdrasil_model::types::task::{BackgroundTaskKind, BackgroundTaskStatus};
/// Optional kind/status filters applied to admin background-task list queries.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdminTaskFilters {
    /// Restricts results to a single task kind when set.
    pub kind: Option<BackgroundTaskKind>,
    /// Restricts results to a single task status when set.
    pub status: Option<BackgroundTaskStatus>,
}

/// Filters for terminal-task cleanup sweeps.
#[derive(Debug, Clone, Copy)]
pub struct TerminalTaskCleanupFilters {
    /// Cutoff instant: only tasks finished (or last updated) before this time are deleted.
    pub finished_before: DateTime<Utc>,
    /// Restricts the sweep to a single task kind when set.
    pub kind: Option<BackgroundTaskKind>,
    /// Restricts the sweep to a single terminal status when set; otherwise covers all terminal statuses.
    pub status: Option<BackgroundTaskStatus>,
}

pub(super) fn apply_admin_filters(
    mut query: Select<BackgroundTask>,
    filters: &AdminTaskFilters,
) -> Select<BackgroundTask> {
    if let Some(kind) = filters.kind {
        query = query.filter(background_task::Column::Kind.eq(kind));
    }
    if let Some(status) = filters.status {
        query = query.filter(background_task::Column::Status.eq(status));
    }
    query
}

pub(super) fn terminal_cleanup_condition(filters: &TerminalTaskCleanupFilters) -> Condition {
    let mut condition = Condition::all();
    condition = condition.add(match filters.status {
        Some(status) => background_task::Column::Status.eq(status),
        None => background_task::Column::Status.is_in([
            BackgroundTaskStatus::Succeeded,
            BackgroundTaskStatus::Failed,
            BackgroundTaskStatus::Canceled,
        ]),
    });
    if let Some(kind) = filters.kind {
        condition = condition.add(background_task::Column::Kind.eq(kind));
    }
    condition.add(
        Condition::any()
            .add(background_task::Column::FinishedAt.lte(filters.finished_before))
            .add(
                Condition::all()
                    .add(background_task::Column::FinishedAt.is_null())
                    .add(background_task::Column::UpdatedAt.lte(filters.finished_before)),
            ),
    )
}

pub(super) fn active_processing_by_kinds_condition(
    now: DateTime<Utc>,
    kinds: &[BackgroundTaskKind],
) -> Condition {
    Condition::all()
        .add(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .add(background_task::Column::LeaseExpiresAt.is_not_null())
        .add(background_task::Column::LeaseExpiresAt.gt(now))
        .add(background_task::Column::Kind.is_in(kinds.iter().copied()))
}

pub(super) fn claimable_condition(now: DateTime<Utc>, _stale_before: DateTime<Utc>) -> Condition {
    // Claimable tasks are either:
    // 1. Pending / Retry rows whose next_run_at is due.
    // 2. Processing rows whose lease is stale enough for a new worker to take over.
    Condition::any()
        .add(
            Condition::all()
                .add(
                    background_task::Column::Status
                        .is_in([BackgroundTaskStatus::Pending, BackgroundTaskStatus::Retry]),
                )
                .add(background_task::Column::NextRunAt.lte(now)),
        )
        .add(processing_stale_condition(now))
}

fn processing_stale_condition(now: DateTime<Utc>) -> Condition {
    Condition::all()
        .add(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .add(background_task::Column::LeaseExpiresAt.is_not_null())
        .add(background_task::Column::LeaseExpiresAt.lte(now))
}
