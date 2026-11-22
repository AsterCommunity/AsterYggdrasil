//! Strongly typed background task specifications.

use crate::config::RuntimeConfig;
use crate::errors::AsterError;
use crate::runtime::AppState;
use aster_yggdrasil_model::entities::background_task;
use aster_yggdrasil_model::types::task::BackgroundTaskKind;

use super::dispatch::TaskLane;
use super::types::{TaskPayload, TaskResult};
use aster_forge_tasks::TaskExecutionContext;

pub(super) type TaskProcessFuture<'a> = aster_forge_tasks::TaskProcessFuture<'a, AsterError>;
pub(super) type TaskSpecAdapter<S> = aster_forge_tasks::TaskSpecAdapter<S>;
pub(super) type ErasedBackgroundTaskSpec = dyn aster_forge_tasks::ErasedBackgroundTaskSpec<
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        BackgroundTaskKind,
        TaskLane,
        TaskPayload,
        TaskResult,
        AsterError,
    >;

pub(super) trait BackgroundTaskSpec:
    aster_forge_tasks::BackgroundTaskSpec<
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        AsterError,
        Kind = BackgroundTaskKind,
        Lane = TaskLane,
        PayloadEnvelope = TaskPayload,
        ResultEnvelope = TaskResult,
    >
{
}

impl<T> BackgroundTaskSpec for T where
    T: aster_forge_tasks::BackgroundTaskSpec<
            AppState,
            background_task::Model,
            RuntimeConfig,
            TaskExecutionContext,
            AsterError,
            Kind = BackgroundTaskKind,
            Lane = TaskLane,
            PayloadEnvelope = TaskPayload,
            ResultEnvelope = TaskResult,
        >
{
}

pub(super) fn serialize_payload<S>(
    payload: &S::Payload,
) -> crate::errors::Result<aster_yggdrasil_model::types::task::StoredTaskPayload>
where
    S: aster_forge_tasks::BackgroundTaskSpec<
            AppState,
            background_task::Model,
            RuntimeConfig,
            TaskExecutionContext,
            AsterError,
            Kind = BackgroundTaskKind,
            Lane = TaskLane,
            PayloadEnvelope = TaskPayload,
            ResultEnvelope = TaskResult,
        >,
{
    aster_forge_tasks::serialize_payload::<
        S,
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        AsterError,
    >(payload)
    .map(aster_yggdrasil_model::types::task::StoredTaskPayload)
    .map_err(AsterError::from)
}

pub(super) fn serialize_result<S>(
    result: &S::Result,
) -> crate::errors::Result<aster_yggdrasil_model::types::task::StoredTaskResult>
where
    S: aster_forge_tasks::BackgroundTaskSpec<
            AppState,
            background_task::Model,
            RuntimeConfig,
            TaskExecutionContext,
            AsterError,
            Kind = BackgroundTaskKind,
            Lane = TaskLane,
            PayloadEnvelope = TaskPayload,
            ResultEnvelope = TaskResult,
        >,
{
    aster_forge_tasks::serialize_result::<
        S,
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        AsterError,
    >(result)
    .map(aster_yggdrasil_model::types::task::StoredTaskResult)
    .map_err(AsterError::from)
}

pub(crate) mod runtime;

pub(crate) use runtime::SystemRuntimeTask;

#[cfg(test)]
mod tests {
    use aster_forge_tasks::{BackgroundTaskSpec, ErasedBackgroundTaskSpec};
    use chrono::Utc;

    use super::{TaskSpecAdapter, serialize_payload, serialize_result};
    use crate::services::ops::task::runtime::SystemRuntimeTaskKind;
    use crate::services::ops::task::spec::SystemRuntimeTask;
    use crate::services::ops::task::types::{
        RuntimeTaskName, RuntimeTaskPayload, RuntimeTaskResult, TaskPayload, TaskResult,
    };
    use aster_yggdrasil_model::entities::background_task;
    use aster_yggdrasil_model::types::{
        task::BackgroundTaskKind, task::BackgroundTaskStatus, task::StoredTaskPayload,
        task::StoredTaskResult,
    };
    fn task_model(
        kind: BackgroundTaskKind,
        payload_json: StoredTaskPayload,
        result_json: Option<StoredTaskResult>,
    ) -> background_task::Model {
        let now = Utc::now();
        background_task::Model {
            id: 7,
            kind,
            status: BackgroundTaskStatus::Succeeded,
            creator_user_id: None,
            display_name: "Task".to_string(),
            dedupe_key: None,
            payload_json,
            result_json,
            runtime_json: None,
            steps_json: None,
            progress_current: 1,
            progress_total: 1,
            status_text: None,
            attempt_count: 0,
            max_attempts: 1,
            next_run_at: now,
            processing_token: 0,
            processing_started_at: None,
            last_heartbeat_at: None,
            lease_expires_at: None,
            started_at: Some(now),
            finished_at: Some(now),
            last_error: None,
            failure_can_retry: None,
            expires_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn serialize_and_decode_system_runtime_payload_and_result() {
        let payload = RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::SystemHealthCheck),
        };
        let result = RuntimeTaskResult {
            duration_ms: 12,
            summary: Some("ok".to_string()),
            system_health: None,
        };
        let payload_json = serialize_payload::<SystemRuntimeTask>(&payload).unwrap();
        let result_json = serialize_result::<SystemRuntimeTask>(&result).unwrap();
        let task = task_model(
            BackgroundTaskKind::SystemRuntime,
            payload_json,
            Some(result_json),
        );

        let adapter = TaskSpecAdapter::<SystemRuntimeTask>::new();
        assert_eq!(
            adapter.decode_payload(&task).unwrap(),
            TaskPayload::SystemRuntime(payload)
        );
        assert_eq!(
            adapter.decode_result(&task).unwrap(),
            Some(TaskResult::SystemRuntime(RuntimeTaskResult {
                duration_ms: 12,
                summary: Some("ok".to_string()),
                system_health: None,
            }))
        );
    }

    #[test]
    fn decode_helpers_surface_kind_mismatch_and_invalid_json() {
        let bad_payload = task_model(
            BackgroundTaskKind::SystemRuntime,
            StoredTaskPayload("not json".to_string()),
            None,
        );
        let adapter = TaskSpecAdapter::<SystemRuntimeTask>::new();
        let error = adapter.decode_payload(&bad_payload).unwrap_err();
        assert!(error.to_string().contains("parse payload for task #7"));

        let bad_result = task_model(
            BackgroundTaskKind::SystemRuntime,
            serialize_payload::<SystemRuntimeTask>(&RuntimeTaskPayload {
                task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::TaskCleanup),
            })
            .unwrap(),
            Some(StoredTaskResult("not json".to_string())),
        );
        let error = adapter.decode_result(&bad_result).unwrap_err();
        assert!(error.to_string().contains("parse result for task #7"));
    }

    #[test]
    fn system_runtime_spec_contract_is_never_dispatched_by_regular_workers() {
        let runtime_config = crate::config::RuntimeConfig::new();
        let error = crate::errors::AsterError::database_connection("temporary");

        assert_eq!(SystemRuntimeTask::KIND, BackgroundTaskKind::SystemRuntime);
        assert!(SystemRuntimeTask::step_specs().is_empty());
        assert_eq!(SystemRuntimeTask::max_attempts(&runtime_config), 1);
        assert!(!SystemRuntimeTask::retry_class(&error).can_manual_retry());
    }
}
