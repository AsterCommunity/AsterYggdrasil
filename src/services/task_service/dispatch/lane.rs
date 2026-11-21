use crate::config::operations;
use crate::runtime::RuntimeConfigRuntimeState;
use crate::types::task::BackgroundTaskKind;

use super::super::registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskLane {
    Fallback,
}

pub(super) type TaskLaneConfig = aster_forge_tasks::TaskLaneConfig<BackgroundTaskKind, TaskLane>;

pub(super) const TASK_LANES: [TaskLane; 1] = [TaskLane::Fallback];

/// Shared background task concurrency is stored in runtime config. The value is
/// both the process-local lane limit and the cluster-wide quota that the claim
/// transaction rechecks behind the config row lock.
pub(super) fn task_lane_configs(state: &impl RuntimeConfigRuntimeState) -> Vec<TaskLaneConfig> {
    let limit = operations::background_task_max_concurrency(state.runtime_config());
    vec![TaskLaneConfig {
        lane: TaskLane::Fallback,
        kinds: registry::task_lane_kinds(TaskLane::Fallback),
        limit,
        cluster_limit: Some(limit),
    }]
}

pub(super) fn task_lane(kind: BackgroundTaskKind) -> TaskLane {
    registry::task_lane(kind)
}

#[cfg(test)]
mod tests {
    use super::{TASK_LANES, TaskLane, TaskLaneConfig, task_lane};
    use crate::services::task_service::registry;
    use crate::types::task::BackgroundTaskKind;

    #[test]
    fn fallback_lane_contains_system_runtime_tasks() {
        assert_eq!(TASK_LANES, [TaskLane::Fallback]);
        assert_eq!(
            task_lane(BackgroundTaskKind::SystemRuntime),
            TaskLane::Fallback
        );

        let config = TaskLaneConfig {
            lane: TaskLane::Fallback,
            kinds: registry::task_lane_kinds(TaskLane::Fallback),
            limit: 4,
            cluster_limit: Some(4),
        };
        assert_eq!(config.kinds, &[BackgroundTaskKind::SystemRuntime]);
        assert_eq!(config.cluster_limit, Some(4));
    }
}
