use crate::config::operations;
use crate::db::{
    repository::{background_task_repo, system_config_repo},
    transaction,
};
use crate::entities::background_task;
use crate::errors::Result;
use crate::runtime::DatabaseRuntimeState;
use crate::types::task::BackgroundTaskKind;
use aster_forge_tasks::TaskLease;

use super::TASK_PROCESSING_STALE_SECS;
use super::lane::{TaskLane, TaskLaneConfig, task_lane};

struct BackgroundTaskClaimStore<'a, State> {
    state: &'a State,
}

impl aster_forge_tasks::ClaimableTaskRecord<BackgroundTaskKind> for background_task::Model {
    fn processing_token(&self) -> i64 {
        self.processing_token
    }
}

#[async_trait::async_trait]
impl<State> aster_forge_tasks::TaskClaimStore<background_task::Model, BackgroundTaskKind, TaskLane>
    for BackgroundTaskClaimStore<'_, State>
where
    State: DatabaseRuntimeState + Sync,
{
    type Error = crate::errors::AsterError;

    /// Atomically claims up to `request.max_claim` due tasks for the lane.
    ///
    /// Selection, the shared concurrency quota recheck, and the per-row
    /// processing-token CAS all run inside one transaction: the shared quota
    /// row is locked first, the currently active task count is rechecked
    /// against `lane_config.limit`, and only then are due rows claimed with
    /// fenced token updates.
    async fn claim(
        &self,
        request: aster_forge_tasks::TaskClaimRequest<BackgroundTaskKind, TaskLane>,
    ) -> Result<Vec<(background_task::Model, TaskLease)>> {
        let lane_config = request.lane;
        let claim_limit = aster_forge_tasks::claim_limit_to_u64(request.max_claim);

        transaction::with_transaction(self.state.writer_db(), async |txn| {
            system_config_repo::lock_by_key(txn, operations::BACKGROUND_TASK_MAX_CONCURRENCY_KEY)
                .await?;
            let active = background_task_repo::count_active_processing_by_kinds(
                txn,
                request.now,
                lane_config.kinds,
            )
            .await?;
            let available = aster_forge_tasks::available_lane_capacity(lane_config.limit, active);
            if available == 0 {
                tracing::debug!(
                    lane = ?lane_config.lane,
                    active,
                    limit = lane_config.limit,
                    "background task lane reached capacity before batch claim"
                );
                return Ok(Vec::new());
            }

            let due = background_task_repo::list_claimable_by_kinds(
                txn,
                request.now,
                request.stale_before,
                lane_config.kinds,
                claim_limit,
            )
            .await?;
            let candidates = aster_forge_tasks::task_claim_candidates(&due)?;

            let mut claimed = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                if claimed.len() >= available {
                    break;
                }

                let did_claim = background_task_repo::try_claim(
                    txn,
                    candidate.task_id,
                    candidate.expected_processing_token,
                    request.now,
                    request.stale_before,
                    candidate.next_processing_token,
                    request.lease_expires_at,
                )
                .await?;
                if !did_claim {
                    continue;
                }

                if let Some(row) = due.iter().find(|row| row.id == candidate.task_id) {
                    claimed.push((
                        row.clone(),
                        TaskLease::new(candidate.task_id, candidate.next_processing_token),
                    ));
                }
            }

            Ok(claimed)
        })
        .await
    }
}

pub(super) async fn claim_due_for_lane(
    state: &(impl DatabaseRuntimeState + Sync),
    lane_config: TaskLaneConfig,
) -> Result<Vec<(background_task::Model, TaskLease)>> {
    aster_forge_tasks::claim_due_for_lane(
        &BackgroundTaskClaimStore { state },
        lane_config,
        lane_config.limit,
        TASK_PROCESSING_STALE_SECS,
        task_lane,
    )
    .await
}
