use super::TaskProcessFuture;
use crate::config::RuntimeConfig;
use crate::entities::background_task;
use crate::errors::AsterError;
use crate::runtime::AppState;
use crate::services::task_service::{
    dispatch::TaskLane,
    types::{RuntimeTaskPayload, RuntimeTaskResult, TaskPayload, TaskResult},
};
use crate::types::BackgroundTaskKind;
use aster_forge_tasks::TaskExecutionContext;

const NO_STEPS: &[aster_forge_tasks::TaskStepSpec] = &[];

pub(crate) struct SystemRuntimeTask;

impl
    aster_forge_tasks::BackgroundTaskSpec<
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        AsterError,
    > for SystemRuntimeTask
{
    type Kind = BackgroundTaskKind;
    type Lane = TaskLane;
    type Payload = RuntimeTaskPayload;
    type Result = RuntimeTaskResult;
    type PayloadEnvelope = TaskPayload;
    type ResultEnvelope = TaskResult;

    const KIND: BackgroundTaskKind = BackgroundTaskKind::SystemRuntime;

    fn step_specs() -> &'static [aster_forge_tasks::TaskStepSpec] {
        NO_STEPS
    }

    fn lane() -> TaskLane {
        TaskLane::Fallback
    }

    fn max_attempts(_runtime_config: &RuntimeConfig) -> i32 {
        1
    }

    fn wrap_payload(payload: Self::Payload) -> TaskPayload {
        TaskPayload::SystemRuntime(payload)
    }

    fn wrap_result(result: Self::Result) -> TaskResult {
        TaskResult::SystemRuntime(result)
    }

    fn process<'a>(
        _state: &'a AppState,
        task: &'a background_task::Model,
        _context: TaskExecutionContext,
    ) -> TaskProcessFuture<'a> {
        Box::pin(async move {
            Err(AsterError::internal_error(format!(
                "system runtime task #{} should not be dispatched",
                task.id
            )))
        })
    }

    fn retry_class(_error: &AsterError) -> aster_forge_tasks::TaskRetryClass {
        aster_forge_tasks::TaskRetryClass::Never
    }
}
