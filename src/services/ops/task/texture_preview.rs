//! `texture_preview_generate` enqueue contract.
//!
//! Mirrors the AsterDrive thumbnail pattern: enqueuing is idempotent against
//! the latest task with the same target, an active task only wakes the
//! dispatcher, and the request path learns whether to answer 202 or 404.

use crate::db::repository::background_task_repo;
use crate::errors::Result;
use crate::runtime::{RuntimeConfigRuntimeState, TaskRuntimeState};
use aster_yggdrasil_model::types::task::BackgroundTaskStatus;

use super::create::{TypedTaskCreate, insert_typed_task_record};
use super::types::TexturePreviewGenerateTaskPayload;

/// Request-path view of the generation task for one preview target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TexturePreviewTaskState {
    /// A task is queued or executing (possibly just enqueued by this call).
    Pending,
    /// The latest task failed terminally; the endpoint reports the preview as
    /// unavailable instead of letting clients retry forever.
    Failed,
}

/// Idempotently ensures a preview generation task for the hash+model target.
///
/// State handling follows the Drive thumbnail contract:
/// - `Pending` / `Processing` / `Retry`: keep the single active task, wake the
///   dispatcher, answer `Pending`.
/// - `Failed`: do not rebuild — a failed render is deterministic (corrupt
///   source) or needs operator attention, and unbounded re-enqueue from
///   request traffic would churn task records.
/// - `Succeeded` / `Canceled` / no task: enqueue a fresh run, so a preview
///   whose cached object was removed regenerates on the next request.
///
/// Concurrent first requests may race past the latest-task check and enqueue
/// two tasks for the same preview; that is accepted (Drive parity): both
/// render the same immutable storage key, so the loser is one redundant
/// render, not a divergence. A dedupe key must NOT be used here — the unique
/// index has no state predicate, so a terminal task would hold the key
/// forever and silently block regeneration after cache removal.
pub(crate) async fn ensure_texture_preview_task<S>(
    state: &S,
    payload: &TexturePreviewGenerateTaskPayload,
    file_name: &str,
) -> Result<TexturePreviewTaskState>
where
    S: TaskRuntimeState + RuntimeConfigRuntimeState,
{
    // The display name carries the full variant identity: hash plus the
    // spec-derived file name. Without the file name, a failed task under an
    // old render spec would permanently block enqueueing a new variant of the
    // same texture after the spec changes.
    let display_name = format!("texture preview {} ({})", payload.texture_hash, file_name);
    let existing = background_task_repo::find_latest_by_kind_and_display_name(
        state.writer_db(),
        aster_yggdrasil_model::types::task::BackgroundTaskKind::TexturePreviewGenerate,
        &display_name,
    )
    .await?;

    if let Some(existing) = existing {
        match existing.status {
            BackgroundTaskStatus::Pending
            | BackgroundTaskStatus::Processing
            | BackgroundTaskStatus::Retry => {
                tracing::debug!(
                    task_id = existing.id,
                    hash = payload.texture_hash,
                    "texture preview task already active; waking dispatcher"
                );
                state.wake_background_task_dispatcher();
                return Ok(TexturePreviewTaskState::Pending);
            }
            BackgroundTaskStatus::Failed => {
                tracing::debug!(
                    task_id = existing.id,
                    hash = payload.texture_hash,
                    error = existing.last_error.as_deref().unwrap_or("unknown"),
                    "texture preview task previously failed; refusing to re-enqueue"
                );
                return Ok(TexturePreviewTaskState::Failed);
            }
            BackgroundTaskStatus::Succeeded | BackgroundTaskStatus::Canceled => {}
        }
    }

    let task = insert_typed_task_record(
        state,
        state.writer_db(),
        TypedTaskCreate::<super::spec::TexturePreviewGenerateTask>::new(
            display_name,
            payload.clone(),
        ),
    )
    .await?;
    tracing::debug!(
        task_id = task.id,
        hash = payload.texture_hash,
        "texture preview generation enqueued"
    );
    state.wake_background_task_dispatcher();
    Ok(TexturePreviewTaskState::Pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_yggdrasil_model::entities::background_task;
    use aster_yggdrasil_model::types::task::BackgroundTaskStatus;
    use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set, Unchanged};

    #[tokio::test]
    async fn regenerated_after_terminal_state_when_cache_is_missing() {
        let state = crate::runtime::test_support::setup_state().await;
        let payload = TexturePreviewGenerateTaskPayload {
            texture_hash: "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133"
                .to_string(),
            texture_model: MinecraftTextureModel::Default,
        };
        let file_name = "skin-3d-default-fingerprint.png";

        // First request enqueues; simulate the task reaching Succeeded.
        assert_eq!(
            ensure_texture_preview_task(&state, &payload, file_name)
                .await
                .expect("first ensure should succeed"),
            TexturePreviewTaskState::Pending
        );
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert_eq!(tasks.len(), 1);
        let mut done = background_task::ActiveModel {
            id: Unchanged(tasks[0].id),
            ..Default::default()
        };
        done.status = Set(BackgroundTaskStatus::Succeeded);
        done.update(state.writer_db())
            .await
            .expect("terminal update should succeed");

        // Cache removal scenario: the next request must enqueue a fresh task
        // instead of answering Pending forever against the dead record.
        assert_eq!(
            ensure_texture_preview_task(&state, &payload, file_name)
                .await
                .expect("regeneration ensure should succeed"),
            TexturePreviewTaskState::Pending
        );
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert_eq!(
            tasks.len(),
            2,
            "a terminal task must not block regeneration"
        );
        assert!(
            tasks
                .iter()
                .any(|task| task.status == BackgroundTaskStatus::Pending),
            "the fresh task must be pending"
        );
    }
}
