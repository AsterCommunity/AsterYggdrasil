use crate::config::operations;
use crate::runtime::RuntimeConfigRuntimeState;
use crate::types::BackgroundTaskKind;

use super::super::registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::services::task_service) enum TaskLane {
    Fallback,
}

pub(super) type TaskLaneConfig = aster_forge_tasks::TaskLaneConfig<BackgroundTaskKind, TaskLane>;

pub(super) const TASK_LANES: [TaskLane; 1] = [TaskLane::Fallback];

pub(super) fn task_lane_configs(state: &impl RuntimeConfigRuntimeState) -> Vec<TaskLaneConfig> {
    vec![TaskLaneConfig {
        lane: TaskLane::Fallback,
        kinds: registry::task_lane_kinds(TaskLane::Fallback),
        limit: operations::background_task_max_concurrency(state.runtime_config()),
        fast_continue: false,
        lock_key: operations::BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
    }]
}

pub(super) fn task_lane(kind: BackgroundTaskKind) -> TaskLane {
    registry::task_lane(kind)
}

#[cfg(test)]
mod tests {
    use super::{TASK_LANES, TaskLane, TaskLaneConfig, task_lane};
    use crate::config::operations;
    use crate::services::task_service::registry;
    use crate::types::BackgroundTaskKind;

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
            fast_continue: false,
            lock_key: operations::BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        };
        assert_eq!(config.kinds, &[BackgroundTaskKind::SystemRuntime]);
        assert_eq!(config.lock_key, "background_task_max_concurrency");
    }
}
