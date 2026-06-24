use chrono::{DateTime, Utc};

use crate::db::{
    repository::{background_task_repo, system_config_repo},
    transaction,
};
use crate::entities::background_task;
use crate::errors::Result;
use crate::runtime::DatabaseRuntimeState;
use crate::types::BackgroundTaskKind;
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

    async fn list_claimable_by_kinds(
        &self,
        now: DateTime<Utc>,
        stale_before: DateTime<Utc>,
        kinds: &'static [BackgroundTaskKind],
        limit: u64,
    ) -> Result<Vec<background_task::Model>> {
        background_task_repo::list_claimable_by_kinds(
            self.state.writer_db(),
            now,
            stale_before,
            kinds,
            limit,
        )
        .await
    }

    async fn count_active_processing_by_kinds(
        &self,
        now: DateTime<Utc>,
        kinds: &'static [BackgroundTaskKind],
    ) -> Result<u64> {
        background_task_repo::count_active_processing_by_kinds(self.state.writer_db(), now, kinds)
            .await
    }

    async fn claim_candidates_for_lane(
        &self,
        lane_config: TaskLaneConfig,
        candidates: &[aster_forge_tasks::TaskClaimCandidate],
        stale_before: DateTime<Utc>,
        claimed_at: DateTime<Utc>,
        lease_expires_at: DateTime<Utc>,
    ) -> Result<Vec<aster_forge_tasks::ClaimedTask>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        transaction::with_transaction(self.state.writer_db(), async |txn| {
            system_config_repo::lock_by_key(txn, lane_config.lock_key).await?;
            let active = background_task_repo::count_active_processing_by_kinds(
                txn,
                claimed_at,
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

            let mut claimed = Vec::with_capacity(available.min(candidates.len()));
            for candidate in candidates {
                if claimed.len() >= available {
                    break;
                }

                let did_claim = background_task_repo::try_claim(
                    txn,
                    candidate.task_id,
                    candidate.expected_processing_token,
                    claimed_at,
                    stale_before,
                    candidate.next_processing_token,
                    lease_expires_at,
                )
                .await?;
                if !did_claim {
                    continue;
                }

                claimed.push(aster_forge_tasks::ClaimedTask {
                    index: candidate.index,
                    task_id: candidate.task_id,
                    processing_token: candidate.next_processing_token,
                });
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
        TASK_PROCESSING_STALE_SECS,
        task_lane,
    )
    .await
}
