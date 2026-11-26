//! Independently scalable queue consumer for the Fallback lane.
//!
//! Adapts [`aster_forge_tasks::TaskConsumer`] so every consuming `serve` role
//! instance claims queue tasks in parallel without holding the runtime lease.
//! The claim transaction's config-row lock and per-row processing-token CAS
//! (see `claim.rs`) keep ownership exclusive across instances.

use std::time::Duration;

use crate::errors::Result;
use crate::runtime::AppState;
use actix_web::web;
use aster_forge_tasks::{BackgroundTasks, TaskConsumer, TaskLease, TaskRuntimeStatus};
use tokio_util::sync::CancellationToken;

use super::claim::claim_due_for_lane;
use super::execute::BackgroundTaskExecutionStore;
use super::lane::{TASK_LANES, TaskLane, TaskLaneConfig, task_lane_configs};
use super::{claimed_task_execution_config, refresh_pending_metric};

/// Forge task-consumer adapter for the single Fallback lane: claims due queue
/// tasks through the transactional claim store and executes each claim under
/// its processing-token lease.
/// Single-flight gate for the detached pending-metric refresh spawned per claim.
#[derive(Clone)]
pub struct FallbackTaskConsumer {
    state: web::Data<AppState>,
    lane: TaskLane,
    refresh_gate: std::sync::Arc<tokio::sync::Semaphore>,
}

const PENDING_METRIC_REFRESH_TIMEOUT_SECS: u64 = 5;

impl FallbackTaskConsumer {
    /// Builds the Fallback lane consumer on top of the shared app state.
    pub fn new(state: web::Data<AppState>) -> Self {
        Self {
            state,
            lane: TASK_LANES[0],
            refresh_gate: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    fn config(&self) -> TaskLaneConfig {
        // Every variant has a configuration; select by enum rather than a global registry.
        task_lane_configs(self.state.get_ref())
            .into_iter()
            .find(|config| config.lane == self.lane)
            .unwrap_or(TaskLaneConfig {
                lane: self.lane,
                kinds: &[],
                limit: 0,
                cluster_limit: Some(0),
            })
    }
}

#[async_trait::async_trait]
impl TaskConsumer for FallbackTaskConsumer {
    type Task = (
        aster_yggdrasil_model::entities::background_task::Model,
        TaskLease,
    );
    type Error = crate::errors::AsterError;

    fn concurrency(&self) -> usize {
        self.config().limit
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(
            crate::config::operations::background_task_dispatch_interval_secs(
                &self.state.runtime_config,
            ),
        )
    }

    fn idle_max_interval(&self) -> Duration {
        Duration::from_secs(
            crate::config::operations::background_task_dispatch_idle_max_interval_secs(
                &self.state.runtime_config,
            ),
        )
    }

    fn name(&self) -> &'static str {
        "fallback"
    }

    fn wakeup(&self) -> &aster_forge_tasks::TaskWakeup {
        self.state.background_task_dispatch_wakeup()
    }

    fn error_kind(&self, error: &crate::errors::AsterError) -> &'static str {
        match error {
            crate::errors::AsterError::DatabaseConnection(_) => "database_connection",
            crate::errors::AsterError::DatabaseOperation(_) => "database_operation",
            _ => error.api_error_code().as_str(),
        }
    }

    async fn claim(&self, available: usize) -> Result<Vec<Self::Task>> {
        let tasks = claim_due_for_lane(self.state.get_ref(), self.config(), available).await?;
        // Backlog gauge refresh must not sit on the claim→execute path: the
        // claimed leases start heartbeating only once their handlers spawn.
        // The single-permit gate keeps at most one refresh in flight, and the
        // timeout stops a slow database from piling refresh tasks up.
        if let Ok(permit) = self.refresh_gate.clone().try_acquire_owned() {
            let state = self.state.get_ref().clone();
            tokio::spawn(async move {
                let _permit = permit;
                let refresh = tokio::time::timeout(
                    Duration::from_secs(PENDING_METRIC_REFRESH_TIMEOUT_SECS),
                    refresh_pending_metric(&state),
                )
                .await;
                if refresh.is_err() {
                    tracing::warn!(
                        timeout_secs = PENDING_METRIC_REFRESH_TIMEOUT_SECS,
                        "background task pending metric refresh timed out"
                    );
                }
            });
        }
        Ok(tasks)
    }

    async fn execute(
        &self,
        (task, lease): Self::Task,
        cancel: CancellationToken,
    ) -> Result<aster_forge_tasks::TaskDispatchOutcome> {
        aster_forge_tasks::process_claimed_task(
            BackgroundTaskExecutionStore {
                state: self.state.get_ref().clone(),
            },
            task,
            lease,
            cancel,
            claimed_task_execution_config(),
        )
        .await
    }

    async fn release_unstarted(&self, (_task, lease): Self::Task) -> Result<()> {
        crate::db::repository::background_task_repo::release_processing(
            self.state.writer_db(),
            lease.task_id,
            lease.processing_token,
            chrono::Utc::now(),
            aster_yggdrasil_model::types::task::BackgroundTaskStatus::Retry,
        )
        .await?;
        Ok(())
    }
}

/// Number of queue consumers equals the number of configured lanes (one Fallback).
pub fn consumer_statuses() -> Vec<TaskRuntimeStatus> {
    TASK_LANES
        .iter()
        .map(|_| TaskRuntimeStatus::default())
        .collect()
}

/// Spawns one independent consumer runner per lane onto the shutdown graph.
pub fn spawn_consumers(
    state: web::Data<AppState>,
    statuses: Vec<TaskRuntimeStatus>,
    stop: CancellationToken,
    tasks: &mut BackgroundTasks,
) {
    for (lane, status) in TASK_LANES.into_iter().zip(statuses) {
        let consumer = FallbackTaskConsumer::new(state.clone());
        let stop = stop.clone();
        let config = state.config.worker.consumer_config();
        tasks.push(async move {
            let report = aster_forge_tasks::run_task_consumer(consumer, stop, config, status).await;
            tracing::info!(?lane, ?report, "task lane consumer stopped");
        });
    }
}
