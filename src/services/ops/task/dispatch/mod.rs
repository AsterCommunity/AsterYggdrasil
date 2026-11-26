//! Background task queue dispatch.
//!
//! Queue tasks are consumed by independently scalable lane consumers
//! ([`consumer::FallbackTaskConsumer`]) driven by Forge's `run_task_consumer`:
//! every consuming `serve` role instance claims due tasks in parallel, and the
//! claim transaction's config-row lock plus per-row processing-token CAS keep
//! ownership exclusive across instances. No dispatcher holds the runtime
//! lease; the leased scheduled maintenance group lives in
//! `crate::runtime::tasks` and stays cluster-singleton.

mod claim;
mod consumer;
mod execute;
mod lane;
mod maintenance;

use chrono::{DateTime, Utc};

pub use consumer::{FallbackTaskConsumer, consumer_statuses, spawn_consumers};
pub(in crate::services::ops::task) use lane::TaskLane;
pub use maintenance::cleanup_expired;

use crate::runtime::{DatabaseRuntimeState, MetricsRuntimeState};

use super::{TASK_HEARTBEAT_INTERVAL_SECS, TASK_PROCESSING_STALE_SECS, task_lease_expires_at};

/// Refreshes the pending/retry gauge after claim activity so dashboards track
/// backlog movement without polling the table directly.
pub(super) async fn refresh_pending_metric(
    state: &(impl DatabaseRuntimeState + MetricsRuntimeState),
) {
    match crate::db::repository::background_task_repo::count_pending_or_retry(state.writer_db())
        .await
    {
        Ok(pending) => state.metrics().set_background_tasks_pending(pending),
        Err(error) => tracing::warn!(
            error = %error,
            "failed to refresh background task pending metric"
        ),
    }
}

/// Function-pointer shapes of the claimed-task execution hooks, so the
/// shared config type stays nameable.
type ClaimedTaskExecutionHooks = aster_forge_tasks::ClaimedTaskExecutionConfig<
    fn(DateTime<Utc>) -> DateTime<Utc>,
    fn(i32) -> i64,
>;

/// Lease/heartbeat/retry configuration for claimed-task execution, shared by
/// the queue consumer and test drivers so fenced semantics stay identical.
pub(crate) fn claimed_task_execution_config() -> ClaimedTaskExecutionHooks {
    aster_forge_tasks::ClaimedTaskExecutionConfig {
        renewal_timeout: aster_forge_tasks::task_lease_renewal_timeout(
            TASK_PROCESSING_STALE_SECS,
            TASK_HEARTBEAT_INTERVAL_SECS,
        ),
        heartbeat_interval: std::time::Duration::from_secs(TASK_HEARTBEAT_INTERVAL_SECS),
        lease_expires_at: task_lease_expires_at,
        retry_delay_secs: aster_forge_tasks::default_task_retry_delay_secs,
    }
}
