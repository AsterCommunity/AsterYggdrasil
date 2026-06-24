//! Background task spec registry.

use crate::config::RuntimeConfig;
use crate::entities::background_task;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::types::BackgroundTaskKind;
use aster_forge_tasks::initial_task_steps_from_specs;

use super::dispatch::TaskLane;
use super::spec::{ErasedBackgroundTaskSpec, SystemRuntimeTask, TaskProcessFuture};
use super::types::{TaskPayload, TaskResult};

aster_forge_tasks::task_registry! {
    pub(super) mod registered {
        state: crate::runtime::AppState;
        task: crate::entities::background_task::Model;
        config: crate::config::RuntimeConfig;
        context: aster_forge_tasks::TaskExecutionContext;
        error: crate::errors::AsterError;
        kind: crate::types::BackgroundTaskKind;
        lane: crate::services::task_service::dispatch::TaskLane;
        payload: crate::services::task_service::types::TaskPayload;
        result: crate::services::task_service::types::TaskResult;
        specs {
            SYSTEM_RUNTIME: super::SystemRuntimeTask => crate::types::BackgroundTaskKind::SystemRuntime,
        }
        lanes {
            crate::services::task_service::dispatch::TaskLane::Fallback => [
                crate::types::BackgroundTaskKind::SystemRuntime,
            ],
        }
    }
}

pub(super) fn spec_for_kind(kind: BackgroundTaskKind) -> &'static ErasedBackgroundTaskSpec {
    registered::spec_for_kind(kind)
}

pub(super) fn decode_task_payload(task: &background_task::Model) -> Result<TaskPayload> {
    spec_for_kind(task.kind)
        .decode_payload(task)
        .map_err(AsterError::from)
}

pub(super) fn decode_task_result(task: &background_task::Model) -> Result<Option<TaskResult>> {
    spec_for_kind(task.kind)
        .decode_result(task)
        .map_err(AsterError::from)
}

pub(super) fn task_retry_class(
    kind: BackgroundTaskKind,
    error: &AsterError,
) -> aster_forge_tasks::TaskRetryClass {
    spec_for_kind(kind).retry_class(error)
}

pub(super) fn process_task<'a>(
    state: &'a AppState,
    task: &'a background_task::Model,
    context: aster_forge_tasks::TaskExecutionContext,
) -> TaskProcessFuture<'a> {
    spec_for_kind(task.kind).process(state, task, context)
}

pub(super) fn initial_task_steps(kind: BackgroundTaskKind) -> Vec<aster_forge_tasks::TaskStepInfo> {
    initial_task_steps_from_specs(spec_for_kind(kind).step_specs())
}

pub(super) fn max_attempts(runtime_config: &RuntimeConfig, kind: BackgroundTaskKind) -> i32 {
    spec_for_kind(kind).max_attempts(runtime_config)
}

pub(in crate::services::task_service) fn task_lane(kind: BackgroundTaskKind) -> TaskLane {
    spec_for_kind(kind).lane()
}

pub(in crate::services::task_service) fn task_lane_kinds(
    lane: TaskLane,
) -> &'static [BackgroundTaskKind] {
    registered::task_lane_kinds(lane)
}
