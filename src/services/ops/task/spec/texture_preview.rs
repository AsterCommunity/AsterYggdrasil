//! `texture_preview_generate` background task: renders a cached skin preview
//! PNG in the worker process so the preview endpoint never renders inline.

use super::super::types::{
    TaskPayload, TaskResult, TexturePreviewGenerateTaskPayload, TexturePreviewGenerateTaskResult,
};
use super::TaskProcessFuture;
use crate::config::RuntimeConfig;
use crate::errors::AsterError;
use crate::runtime::AppState;
use crate::services::ops::task::dispatch::TaskLane;
use aster_forge_tasks::TaskExecutionContext;
use aster_yggdrasil_model::entities::background_task;
use aster_yggdrasil_model::types::task::BackgroundTaskKind;

const NO_STEPS: &[aster_forge_tasks::TaskStepSpec] = &[];
const MAX_ATTEMPTS: i32 = 3;

pub(crate) struct TexturePreviewGenerateTask;

impl
    aster_forge_tasks::BackgroundTaskSpec<
        AppState,
        background_task::Model,
        RuntimeConfig,
        TaskExecutionContext,
        AsterError,
    > for TexturePreviewGenerateTask
{
    type Kind = BackgroundTaskKind;
    type Lane = TaskLane;
    type Payload = TexturePreviewGenerateTaskPayload;
    type Result = TexturePreviewGenerateTaskResult;
    type PayloadEnvelope = TaskPayload;
    type ResultEnvelope = TaskResult;

    const KIND: BackgroundTaskKind = BackgroundTaskKind::TexturePreviewGenerate;

    fn step_specs() -> &'static [aster_forge_tasks::TaskStepSpec] {
        NO_STEPS
    }

    fn lane() -> TaskLane {
        TaskLane::Fallback
    }

    fn max_attempts(_runtime_config: &RuntimeConfig) -> i32 {
        MAX_ATTEMPTS
    }

    fn wrap_payload(payload: Self::Payload) -> TaskPayload {
        TaskPayload::TexturePreviewGenerate(payload)
    }

    fn wrap_result(result: Self::Result) -> TaskResult {
        TaskResult::TexturePreviewGenerate(result)
    }

    fn process<'a>(
        state: &'a AppState,
        task: &'a background_task::Model,
        context: TaskExecutionContext,
    ) -> TaskProcessFuture<'a> {
        Box::pin(async move {
            let payload = super::decode_payload_as::<Self>(task)?;
            let outcome =
                crate::services::texture::generate_preview_object(state, &payload).await?;
            let result = TexturePreviewGenerateTaskResult {
                file_name: outcome.file_name,
                reused_existing: outcome.reused_existing,
            };
            let result_json = aster_yggdrasil_model::types::task::StoredTaskResult(
                super::serialize_result::<Self>(&result)?.into(),
            );
            super::super::mark_task_succeeded(
                state,
                context.lease_guard(),
                Some(&result_json),
                1,
                1,
                None,
                &[],
            )
            .await?;
            Ok(())
        })
    }

    fn retry_class(error: &AsterError) -> aster_forge_tasks::TaskRetryClass {
        match error {
            // Transient infrastructure failures may resolve on a later attempt;
            // a deterministic render failure (corrupt source, invalid PNG) will
            // fail again and must not burn retries. The classified variant is
            // what SeaORM transaction paths raise for ordinary operation
            // failures, so it retries like its DatabaseOperation sibling.
            AsterError::DatabaseConnection(_)
            | AsterError::DatabaseOperation(_)
            | AsterError::DatabaseOperationClassified { .. } => {
                aster_forge_tasks::TaskRetryClass::Auto
            }
            _ => aster_forge_tasks::TaskRetryClass::Never,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ops::task::types::TaskPresentationCode;
    use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;

    fn payload() -> TexturePreviewGenerateTaskPayload {
        TexturePreviewGenerateTaskPayload {
            texture_hash: "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133"
                .to_string(),
            texture_model: MinecraftTextureModel::Default,
        }
    }

    #[test]
    fn payload_roundtrips_through_the_typed_envelope() {
        let encoded = serde_json::to_string(&TaskPayload::TexturePreviewGenerate(payload()))
            .expect("payload should serialize");
        assert!(encoded.contains(r#""kind":"texture_preview_generate""#));
        assert!(encoded.contains(r#""texture_model":"default""#));
        let decoded: TaskPayload = serde_json::from_str(&encoded).expect("payload should decode");
        assert_eq!(decoded, TaskPayload::TexturePreviewGenerate(payload()));
    }

    #[test]
    fn result_roundtrips_and_marks_reuse() {
        let result = TexturePreviewGenerateTaskResult {
            file_name: "skin3d-default-fingerprint.png".to_string(),
            reused_existing: true,
        };
        let encoded = serde_json::to_string(&TaskResult::TexturePreviewGenerate(result.clone()))
            .expect("result should serialize");
        assert!(encoded.contains(r#""reused_existing":true"#));
        let decoded: TaskResult = serde_json::from_str(&encoded).expect("result should decode");
        assert_eq!(decoded, TaskResult::TexturePreviewGenerate(result));
    }

    #[test]
    fn retry_class_matches_the_error_taxonomy() {
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::retry_class(&AsterError::database_connection("down")),
            aster_forge_tasks::TaskRetryClass::Auto
        );
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::retry_class(&AsterError::internal_error("corrupt png")),
            aster_forge_tasks::TaskRetryClass::Never
        );
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::retry_class(&AsterError::DatabaseOperationClassified {
                message: "write failed".to_string(),
                kind: aster_forge_db::DatabaseErrorKind::Deadlock,
            }),
            aster_forge_tasks::TaskRetryClass::Auto
        );
    }

    #[test]
    fn spec_metadata_binds_kind_lane_and_attempts() {
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::KIND,
            BackgroundTaskKind::TexturePreviewGenerate
        );
        assert_eq!(
            BackgroundTaskKind::TexturePreviewGenerate.as_str(),
            "texture_preview_generate"
        );
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::lane(),
            TaskLane::Fallback
        );
        assert_eq!(
            <TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
                AppState,
                background_task::Model,
                RuntimeConfig,
                TaskExecutionContext,
                AsterError,
            >>::max_attempts(&RuntimeConfig::new()),
            3
        );
        // Presentation keeps a stable title code so admin UI text stays pinned.
        let title = TaskPayload::TexturePreviewGenerate(payload());
        let encoded = serde_json::to_string(&title).unwrap();
        assert!(!encoded.is_empty());
        let _ = TaskPresentationCode::TexturePreviewGenerateHash;
    }
}
