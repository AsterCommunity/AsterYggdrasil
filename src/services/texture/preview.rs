//! Texture preview lookup, fingerprinting, cache, and rendering entrypoints.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use aster_yggdrasil_render::{
    DecodedSkin, RenderWorkspace, SkinModel, render_decoded_preview_with_workspace,
};
use image::{ExtendedColorType, ImageEncoder};
use serde::Serialize;
use sha2::Digest;
use tokio::io::AsyncReadExt;

use super::{default_skin, is_valid_texture_hash};
use crate::api::error_code::AsterErrorCode;
use crate::config::texture_preview::{RuntimeTexturePreviewPolicy, TexturePreviewSpec};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{
    AppConfigRuntimeState, AppState, DatabaseRuntimeState, RuntimeConfigRuntimeState,
    TaskRuntimeState, TextureStorageRuntimeState,
};
use crate::services::ops::task::types::TexturePreviewGenerateTaskPayload;
use aster_yggdrasil_model::entities::minecraft_texture;
use aster_yggdrasil_model::types::yggdrasil::{MinecraftTextureModel, MinecraftTextureType};

/// `Cache-Control` for preview responses; previews are content-addressed by
/// texture hash plus render fingerprint, so they never change once written.
pub const TEXTURE_PREVIEW_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// A rendered preview PNG together with the object-storage key it is cached
/// under.
#[derive(Debug, Clone)]
pub struct TexturePreviewBytes {
    /// Encoded PNG payload of the preview image.
    pub bytes: Vec<u8>,
    /// Object-storage key the preview is persisted to.
    pub storage_key: String,
}

/// Returns the canonical preview URL path for the given skin hash and model,
/// or `None` when previews are disabled for this texture type (only skins get
/// previews) or the hash is not valid.
pub fn current_texture_preview_url(
    runtime_config: &crate::config::RuntimeConfig,
    hash: &str,
    texture_type: MinecraftTextureType,
    texture_model: MinecraftTextureModel,
) -> Option<String> {
    if texture_type != MinecraftTextureType::Skin || !is_valid_texture_hash(hash) {
        return None;
    }
    let policy = RuntimeTexturePreviewPolicy::from_runtime_config(runtime_config);
    Some(format!(
        "/api/v1/texture-previews/{hash}/{}",
        preview_file_name(&policy.spec, texture_model)
    ))
}

/// Outcome of a preview lookup on the request path.
///
/// `Ready` carries the cached PNG; `Accepted` means the request should come
/// back after the worker finished (or started) generating the preview task.
#[derive(Debug, Clone)]
pub enum TexturePreviewLookup {
    /// Cached preview found; serve [`TexturePreviewBytes`] directly.
    Ready(TexturePreviewBytes),
    /// No cached preview yet; a generation task has been queued and the
    /// client should retry after the worker finishes.
    Accepted,
}

/// Endpoint-side preview lookup: serves cached previews and routes cache
/// misses into the `texture_preview_generate` background task. Rendering
/// never happens inline — the api process stays render-free and the worker
/// process owns GPU-free projection work.
pub async fn texture_preview_by_hash<S>(
    state: &S,
    hash: &str,
    file_name: &str,
) -> Result<TexturePreviewLookup>
where
    S: DatabaseRuntimeState
        + RuntimeConfigRuntimeState
        + TextureStorageRuntimeState
        + AppConfigRuntimeState
        + TaskRuntimeState,
{
    if !is_valid_texture_hash(hash) {
        return Err(texture_preview_not_found("invalid texture hash"));
    }

    let source = resolve_preview_source(state, hash).await?;
    let policy = RuntimeTexturePreviewPolicy::from_runtime_config(state.runtime_config());
    let expected_file_name = preview_file_name(&policy.spec, source.texture_model);
    if file_name != expected_file_name {
        tracing::debug!(
            hash,
            file_name,
            expected_file_name,
            "texture preview filename does not match current render spec"
        );
        return Err(texture_preview_not_found("texture preview variant"));
    }

    let storage_key = preview_storage_key(hash, &expected_file_name);
    if state.texture_storage().exists(&storage_key).await? {
        let bytes = read_object_storage_key(state, &storage_key).await?;
        tracing::debug!(
            hash,
            storage_key,
            size = bytes.len(),
            "served cached texture preview"
        );
        return Ok(TexturePreviewLookup::Ready(TexturePreviewBytes {
            bytes,
            storage_key,
        }));
    }

    let task_state = crate::services::ops::task::texture_preview::ensure_texture_preview_task(
        state,
        &TexturePreviewGenerateTaskPayload {
            texture_hash: hash.to_string(),
            texture_model: source.texture_model,
        },
        &expected_file_name,
    )
    .await?;
    match task_state {
        crate::services::ops::task::texture_preview::TexturePreviewTaskState::Pending => {
            Ok(TexturePreviewLookup::Accepted)
        }
        crate::services::ops::task::texture_preview::TexturePreviewTaskState::Failed => {
            Err(texture_preview_not_found("texture preview"))
        }
    }
}

/// Outcome of the worker-side generation.
#[derive(Debug)]
pub struct GeneratedPreviewOutcome {
    pub file_name: String,
    pub reused_existing: bool,
}

/// Renders the preview PNG and writes it to object storage. Runs on the
/// worker process as the `texture_preview_generate` task body.
pub async fn generate_preview_object(
    state: &AppState,
    payload: &TexturePreviewGenerateTaskPayload,
) -> Result<GeneratedPreviewOutcome> {
    let source = resolve_preview_source(state, &payload.texture_hash).await?;
    if source.texture_model != payload.texture_model {
        // The texture row changed model between enqueue and execution; the
        // endpoint re-derives the file name per request, so this run is stale
        // and must not publish a preview under the old model's name.
        tracing::warn!(
            hash = payload.texture_hash,
            enqueued = ?payload.texture_model,
            found = ?source.texture_model,
            "texture model changed before preview generation; skipping stale render"
        );
        return Err(texture_preview_not_found("texture variant"));
    }

    let policy = RuntimeTexturePreviewPolicy::from_runtime_config(state.runtime_config());
    let expected_file_name = preview_file_name(&policy.spec, payload.texture_model);
    let storage_key = preview_storage_key(&payload.texture_hash, &expected_file_name);
    if state.texture_storage().exists(&storage_key).await? {
        return Ok(GeneratedPreviewOutcome {
            file_name: expected_file_name,
            reused_existing: true,
        });
    }

    let bytes = render_preview_png(source.bytes, payload.texture_model, policy.spec).await?;
    let temp_path = preview_temp_path(state, &payload.texture_hash, &expected_file_name);
    write_preview_temp_file(&temp_path, &bytes).await?;
    let put_result = state
        .texture_storage()
        .put_file(&storage_key, &temp_path)
        .await;
    aster_forge_utils::fs::cleanup_temp_file(&temp_path).await;
    put_result?;

    tracing::info!(
        hash = payload.texture_hash,
        storage_key,
        size = bytes.len(),
        "rendered and cached texture preview"
    );
    Ok(GeneratedPreviewOutcome {
        file_name: expected_file_name,
        reused_existing: false,
    })
}

struct PreviewSource {
    bytes: Vec<u8>,
    texture_model: MinecraftTextureModel,
}

async fn resolve_preview_source<S>(state: &S, hash: &str) -> Result<PreviewSource>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    if let Some(texture) = super::texture_by_hash(state, hash).await? {
        return preview_source_from_texture(state, texture).await;
    }
    if let Some(default_skin) = default_skin::by_hash(hash) {
        return Ok(PreviewSource {
            bytes: default_skin.bytes.to_vec(),
            texture_model: default_skin.model,
        });
    }
    Err(texture_preview_not_found("texture"))
}

async fn preview_source_from_texture<S>(
    state: &S,
    texture: minecraft_texture::Model,
) -> Result<PreviewSource>
where
    S: TextureStorageRuntimeState,
{
    if texture.texture_type != MinecraftTextureType::Skin {
        return Err(texture_preview_not_found("non-skin texture"));
    }
    let bytes = read_object_storage_key(state, &texture.storage_key).await?;
    Ok(PreviewSource {
        bytes,
        texture_model: texture.texture_model,
    })
}

async fn read_object_storage_key<S>(state: &S, storage_key: &str) -> Result<Vec<u8>>
where
    S: TextureStorageRuntimeState,
{
    let mut stream = state.texture_storage().get_stream(storage_key).await?;
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .await
        .map_aster_err_ctx("read texture preview object", AsterError::internal_error)?;
    Ok(bytes)
}

async fn render_preview_png(
    source: Vec<u8>,
    texture_model: MinecraftTextureModel,
    spec: TexturePreviewSpec,
) -> Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        let decoded = DecodedSkin::from_png_bytes(&source).map_err(render_error)?;
        let options = spec.renderer_options();
        let model = renderer_skin_model(texture_model);
        let mut workspace = RenderWorkspace::new();
        let preview =
            render_decoded_preview_with_workspace(&decoded, model, &options, &mut workspace)
                .map_err(render_error)?;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(Cursor::new(&mut bytes))
            .write_image(
                preview.as_raw(),
                preview.width(),
                preview.height(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|error| {
                AsterError::internal_error_code(
                    AsterErrorCode::MinecraftTextureInvalidPng,
                    format!("failed to encode texture preview PNG: {error}"),
                )
            })?;
        Ok(bytes)
    })
    .await
    .map_aster_err_ctx("render texture preview task", AsterError::internal_error)?
}

fn render_error(error: aster_yggdrasil_render::RenderError) -> AsterError {
    // A render failure is deterministic (the source PNG is corrupt or
    // unsupported): mapping it to validation keeps the background task's
    // retry policy from burning attempts on data that will fail again.
    AsterError::validation_error_code(
        AsterErrorCode::MinecraftTextureInvalidPng,
        format!("failed to render texture preview: {error}"),
    )
}

async fn write_preview_temp_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_aster_err_ctx(
            "create texture preview temp dir",
            AsterError::internal_error,
        )?;
    }
    tokio::fs::write(path, bytes).await.map_aster_err_ctx(
        "write texture preview temp file",
        AsterError::internal_error,
    )
}

fn preview_temp_path<S>(state: &S, hash: &str, file_name: &str) -> PathBuf
where
    S: AppConfigRuntimeState,
{
    PathBuf::from(&state.config().server.temp_dir)
        .join("texture-previews")
        .join(format!("{hash}-{file_name}"))
}

fn renderer_skin_model(model: MinecraftTextureModel) -> SkinModel {
    match model {
        MinecraftTextureModel::Default => SkinModel::Default,
        MinecraftTextureModel::Slim => SkinModel::Slim,
    }
}

fn preview_file_name(spec: &TexturePreviewSpec, model: MinecraftTextureModel) -> String {
    format!(
        "{}-{}-{}.png",
        spec.engine.as_str(),
        model.as_str(),
        preview_fingerprint(spec, model)
    )
}

fn preview_storage_key(hash: &str, file_name: &str) -> String {
    format!("texture-previews/{hash}/{file_name}")
}

fn preview_fingerprint(spec: &TexturePreviewSpec, model: MinecraftTextureModel) -> String {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        version: u8,
        format: &'static str,
        model: &'static str,
        spec: &'a TexturePreviewSpec,
    }

    let payload = match serde_json::to_vec(&Fingerprint {
        version: 1,
        format: "png",
        model: model.as_str(),
        spec,
    }) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to serialize texture preview fingerprint payload"
            );
            format!("v1:png:{}:{spec:?}", model.as_str()).into_bytes()
        }
    };
    let digest = sha2::Sha256::digest(payload);
    hex::encode(digest).chars().take(16).collect()
}

fn texture_preview_not_found(message: impl Into<String>) -> AsterError {
    AsterError::record_not_found_code(AsterErrorCode::MinecraftTextureNotFound, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::texture_preview::{
        TexturePreviewBackground, TexturePreviewEngine, TexturePreviewQualityProfile,
    };

    #[test]
    fn file_name_changes_when_render_parameters_change() {
        let mut spec = TexturePreviewSpec {
            engine: TexturePreviewEngine::Skin3d,
            profile: TexturePreviewQualityProfile::Default,
            width: 430,
            height: 430,
            background: TexturePreviewBackground::Transparent,
            show_outer_layer: true,
            scale: 11.5,
            pitch: 30.0,
            front_yaw: -45.0,
            back_yaw: 135.0,
            spacing_3d: 35.0,
            x_offset: 0.0,
            y_offset: -24.0,
            center_y: 0.56,
            supersampling: 2,
            padding_2d: 24,
            spacing_2d: 35,
        };
        let first = preview_file_name(&spec, MinecraftTextureModel::Slim);
        spec.pitch = 25.0;
        let second = preview_file_name(&spec, MinecraftTextureModel::Slim);

        assert_ne!(first, second);
        assert!(first.starts_with("skin-3d-slim-"));
        assert!(first.ends_with(".png"));
    }
}

#[cfg(test)]
mod task_and_lookup_tests {
    use super::*;
    use crate::runtime::test_support;
    use crate::services::ops::task::types::TexturePreviewGenerateTaskPayload;
    use aster_yggdrasil_model::entities::background_task;
    use aster_yggdrasil_model::types::task::BackgroundTaskStatus;
    use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;
    use sea_orm::EntityTrait;

    /// Isolated state: each test gets its own temp storage root so preview
    /// objects never leak between tests or into the repository's data dir.
    async fn test_state() -> crate::runtime::AppState {
        let root = std::env::temp_dir().join(format!(
            "asteryggdrasil-preview-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("preview test root should create");
        let mut config = crate::config::Config::default();
        config.texture_storage.local_root = root.join("storage").to_string_lossy().into_owned();
        config.server.temp_dir = root.join("tmp").to_string_lossy().into_owned();
        let db = test_support::connect_in_memory().await;
        test_support::setup_state_with_config(db, std::sync::Arc::new(config)).await
    }

    fn steve_payload(model: MinecraftTextureModel) -> TexturePreviewGenerateTaskPayload {
        TexturePreviewGenerateTaskPayload {
            texture_hash: default_skin::STEVE_HASH.to_string(),
            texture_model: model,
        }
    }

    fn expected_file_name(
        state: &crate::runtime::AppState,
        model: MinecraftTextureModel,
    ) -> String {
        let policy = RuntimeTexturePreviewPolicy::from_runtime_config(state.runtime_config());
        preview_file_name(&policy.spec, model)
    }

    #[tokio::test]
    async fn generate_renders_then_reports_reuse_for_default_skins() {
        let state = test_state().await;
        let payload = steve_payload(MinecraftTextureModel::Default);

        let first = generate_preview_object(&state, &payload)
            .await
            .expect("default skin generation should succeed");
        assert!(!first.reused_existing);
        let storage_key = preview_storage_key(&payload.texture_hash, &first.file_name);
        assert!(
            state
                .texture_storage()
                .exists(&storage_key)
                .await
                .expect("storage exists check should succeed"),
            "first generation must publish the cached object"
        );

        let second = generate_preview_object(&state, &payload)
            .await
            .expect("second generation should succeed");
        assert!(second.reused_existing, "second run must reuse the object");
        assert_eq!(second.file_name, first.file_name);
    }

    #[tokio::test]
    async fn generate_renders_each_skin_model_under_its_own_file_name() {
        let state = test_state().await;

        // Each embedded default skin carries its own model: Steve is classic,
        // Alex is slim.
        let default =
            generate_preview_object(&state, &steve_payload(MinecraftTextureModel::Default))
                .await
                .expect("default-model render should succeed");
        let slim = generate_preview_object(
            &state,
            &TexturePreviewGenerateTaskPayload {
                texture_hash: default_skin::ALEX_HASH.to_string(),
                texture_model: MinecraftTextureModel::Slim,
            },
        )
        .await
        .expect("slim-model render should succeed");
        assert_ne!(default.file_name, slim.file_name);
        assert!(default.file_name.contains("default"));
        assert!(slim.file_name.contains("slim"));
    }

    #[tokio::test]
    async fn generate_skips_when_the_texture_model_drifted_after_enqueue() {
        let state = test_state().await;
        // Alex's embedded bytes resolve as `Slim`, so a `Default`-model payload
        // mismatches the resolved source model and the run must refuse instead
        // of publishing a preview under the wrong model's file name.
        let mismatched = TexturePreviewGenerateTaskPayload {
            texture_hash: default_skin::ALEX_HASH.to_string(),
            texture_model: MinecraftTextureModel::Default,
        };

        let outcome = generate_preview_object(&state, &mismatched).await;
        assert!(
            outcome.is_err(),
            "alex (slim) bytes must not render under a default-model payload"
        );
    }

    #[tokio::test]
    async fn unknown_hash_fails_resolution_without_creating_tasks() {
        let state = test_state().await;
        let lookup = texture_preview_by_hash(
            &state,
            "1111111111111111111111111111111111111111111111111111111111111111",
            "whatever.png",
        )
        .await;
        assert!(lookup.is_err(), "unknown hashes must 404 before enqueueing");
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert!(
            tasks.is_empty(),
            "unknown-hash requests must not enqueue tasks"
        );
    }

    #[tokio::test]
    async fn lookup_accepts_on_miss_and_ready_after_worker_generation() {
        let state = test_state().await;
        let model = MinecraftTextureModel::Default;
        let file_name = expected_file_name(&state, model);
        let payload = steve_payload(model);

        let lookup = texture_preview_by_hash(&state, default_skin::STEVE_HASH, &file_name)
            .await
            .expect("valid lookup should not error");
        assert!(
            matches!(lookup, TexturePreviewLookup::Accepted),
            "cache miss must hand off to the background task, not render inline"
        );

        // Worker-side execution flips the endpoint to Ready.
        generate_preview_object(&state, &payload)
            .await
            .expect("worker generation should succeed");
        let lookup = texture_preview_by_hash(&state, default_skin::STEVE_HASH, &file_name)
            .await
            .expect("second lookup should not error");
        let TexturePreviewLookup::Ready(preview) = lookup else {
            panic!("cached preview must be Ready, got Accepted");
        };
        assert!(preview.bytes.starts_with(b"\x89PNG"));
        assert_eq!(
            preview.storage_key,
            preview_storage_key(default_skin::STEVE_HASH, &file_name)
        );
    }

    #[tokio::test]
    async fn filename_mismatch_returns_not_found_without_enqueueing() {
        let state = test_state().await;
        let lookup =
            texture_preview_by_hash(&state, default_skin::STEVE_HASH, "stale-fingerprint.png")
                .await;
        assert!(lookup.is_err(), "spec-mismatched file names must 404");
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert!(tasks.is_empty(), "mismatched variants must not enqueue");
    }

    #[tokio::test]
    async fn ensure_is_idempotent_and_refuses_after_failure() {
        let state = test_state().await;
        let model = MinecraftTextureModel::Default;
        let file_name = expected_file_name(&state, model);
        let payload = steve_payload(model);

        use crate::services::ops::task::texture_preview::{
            TexturePreviewTaskState, ensure_texture_preview_task,
        };
        let first = ensure_texture_preview_task(&state, &payload, &file_name)
            .await
            .expect("first ensure should enqueue");
        assert_eq!(first, TexturePreviewTaskState::Pending);
        let second = ensure_texture_preview_task(&state, &payload, &file_name)
            .await
            .expect("second ensure should dedupe");
        assert_eq!(second, TexturePreviewTaskState::Pending);

        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert_eq!(
            tasks.len(),
            1,
            "the second ensure must reuse the active task, not enqueue a duplicate"
        );

        // Terminal failure: the endpoint must learn the preview is unavailable
        // instead of answering 202 forever.
        use sea_orm::{ActiveModelTrait, Set, Unchanged};
        let task = &tasks[0];
        let mut update = background_task::ActiveModel {
            id: Unchanged(task.id),
            ..Default::default()
        };
        update.status = Set(BackgroundTaskStatus::Failed);
        update.last_error = Set(Some("render failed".to_string()));
        update
            .update(state.writer_db())
            .await
            .expect("task failure update should succeed");

        let refused = ensure_texture_preview_task(&state, &payload, &file_name)
            .await
            .expect("ensure should not error on a failed task");
        assert_eq!(refused, TexturePreviewTaskState::Failed);
        let lookup = texture_preview_by_hash(&state, default_skin::STEVE_HASH, &file_name).await;
        assert!(lookup.is_err(), "a failed generation must 404, not 202");
    }
}

#[cfg(test)]
mod variant_identity_tests {
    use super::*;
    use crate::runtime::test_support;
    use crate::services::ops::task::texture_preview::{
        TexturePreviewTaskState, ensure_texture_preview_task,
    };
    use crate::services::ops::task::types::TexturePreviewGenerateTaskPayload;
    use aster_yggdrasil_model::entities::background_task;
    use aster_yggdrasil_model::types::task::BackgroundTaskStatus;
    use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set, Unchanged};

    async fn test_state() -> crate::runtime::AppState {
        let root = std::env::temp_dir().join(format!(
            "asteryggdrasil-preview-variant-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("variant test root should create");
        let mut config = crate::config::Config::default();
        config.texture_storage.local_root = root.join("storage").to_string_lossy().into_owned();
        config.server.temp_dir = root.join("tmp").to_string_lossy().into_owned();
        let db = test_support::connect_in_memory().await;
        test_support::setup_state_with_config(db, std::sync::Arc::new(config)).await
    }

    fn expected_file_name(
        state: &crate::runtime::AppState,
        model: MinecraftTextureModel,
    ) -> String {
        let policy = RuntimeTexturePreviewPolicy::from_runtime_config(state.runtime_config());
        preview_file_name(&policy.spec, model)
    }

    #[tokio::test]
    async fn a_failed_variant_does_not_block_a_new_variant_after_a_spec_change() {
        let state = test_state().await;
        let payload = TexturePreviewGenerateTaskPayload {
            texture_hash: default_skin::STEVE_HASH.to_string(),
            texture_model: MinecraftTextureModel::Default,
        };

        // Variant A (current spec) fails terminally.
        let file_a = expected_file_name(&state, MinecraftTextureModel::Default);
        assert_eq!(
            ensure_texture_preview_task(&state, &payload, &file_a)
                .await
                .expect("variant A ensure should succeed"),
            TexturePreviewTaskState::Pending
        );
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert_eq!(tasks.len(), 1);
        let mut failed = background_task::ActiveModel {
            id: Unchanged(tasks[0].id),
            ..Default::default()
        };
        failed.status = Set(BackgroundTaskStatus::Failed);
        failed.last_error = Set(Some("render failed".to_string()));
        failed
            .update(state.writer_db())
            .await
            .expect("failure update should succeed");

        // Same variant stays refused.
        assert_eq!(
            ensure_texture_preview_task(&state, &payload, &file_a)
                .await
                .expect("ensure should not error"),
            TexturePreviewTaskState::Failed
        );

        // A new variant (different spec fingerprint -> different file name)
        // must be enqueuable: the old failure belongs to a variant that will
        // never be requested again.
        let file_b = file_a.replace(".png", "-newspec.png");
        assert_eq!(
            ensure_texture_preview_task(&state, &payload, &file_b)
                .await
                .expect("variant B ensure should succeed"),
            TexturePreviewTaskState::Pending,
            "a new variant must not inherit the old variant's failure"
        );
        let tasks = background_task::Entity::find()
            .all(state.writer_db())
            .await
            .expect("task query should succeed");
        assert_eq!(tasks.len(), 2, "variant B must get its own task record");
    }
}

#[cfg(test)]
mod execution_paths_tests {
    use super::*;
    use crate::runtime::{AppState, AppStateParts, test_support};
    use crate::services::ops::task::types::TexturePreviewGenerateTaskPayload;
    use aster_forge_tasks::TaskRetryClass;
    use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;
    use std::path::Path as StdPath;

    const STEVE_HASH: &str = "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133";

    async fn isolated_state() -> AppState {
        let root = std::env::temp_dir().join(format!(
            "asteryggdrasil-preview-exec-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("execution test root should create");
        let mut config = crate::config::Config::default();
        config.texture_storage.local_root = root.join("storage").to_string_lossy().into_owned();
        config.server.temp_dir = root.join("tmp").to_string_lossy().into_owned();
        let db = test_support::connect_in_memory().await;
        test_support::setup_state_with_config(db, std::sync::Arc::new(config)).await
    }

    struct FailingUploadStorage;

    #[async_trait::async_trait]
    impl crate::texture_storage::TextureStore for FailingUploadStorage {
        fn backend_name(&self) -> &'static str {
            "failing-upload"
        }

        async fn put_file(
            &self,
            _storage_key: &str,
            _local_path: &StdPath,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<String> {
            Err(aster_yggdrasil_texture_storage::TextureStoreError::backend(
                "S3 object upload failed: endpoint=https://s3.internal, bucket=private, source=connection refused",
            ))
        }

        async fn get_stream(
            &self,
            _storage_key: &str,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<
            Box<dyn tokio::io::AsyncRead + Unpin + Send>,
        > {
            Err(aster_yggdrasil_texture_storage::TextureStoreError::backend(
                "failing upload storage",
            ))
        }

        async fn delete(
            &self,
            _storage_key: &str,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<()> {
            Ok(())
        }

        async fn exists(
            &self,
            _storage_key: &str,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<bool> {
            Ok(false)
        }

        async fn metadata(
            &self,
            _storage_key: &str,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<
            aster_yggdrasil_texture_storage::ObjectBlobMetadata,
        > {
            Err(aster_yggdrasil_texture_storage::TextureStoreError::backend(
                "failing upload storage",
            ))
        }

        async fn list_keys(
            &self,
            _prefix: &str,
        ) -> aster_yggdrasil_texture_storage::TextureStoreResult<Vec<String>> {
            Ok(Vec::new())
        }
    }

    fn preview_payload() -> TexturePreviewGenerateTaskPayload {
        TexturePreviewGenerateTaskPayload {
            texture_hash: STEVE_HASH.to_string(),
            texture_model: MinecraftTextureModel::Default,
        }
    }

    fn retry_class_for(error: &AsterError) -> TaskRetryClass {
        <crate::services::ops::task::TexturePreviewGenerateTask as aster_forge_tasks::BackgroundTaskSpec<
            AppState,
            aster_yggdrasil_model::entities::background_task::Model,
            crate::config::RuntimeConfig,
            aster_forge_tasks::TaskExecutionContext,
            AsterError,
        >>::retry_class(error)
    }

    #[tokio::test]
    async fn storage_upload_failure_is_retryable_internal_error() {
        let state = isolated_state().await;
        let rebuilt = AppState::from_parts(AppStateParts {
            db_handles: state.db_handles.clone(),
            config: state.config().clone(),
            runtime_config: state.runtime_config().clone(),
            cache: state.cache().clone(),
            texture_storage: std::sync::Arc::new(FailingUploadStorage),
            mail_sender: state.mail_sender().clone(),
            config_sync: state.config_sync().clone(),
            metrics: state.metrics().clone(),
            audit_writer: state.audit_writer().clone(),
        })
        .expect("failing-upload AppState should build");

        let outcome = generate_preview_object(&rebuilt, &preview_payload()).await;
        let error = outcome.expect_err("upload failure must surface as an error");
        assert!(
            error.to_string().contains("upload failed"),
            "error should carry the storage failure: {error}"
        );
        assert!(
            matches!(error, AsterError::InternalError(_)),
            "storage upload maps to InternalError, got {error:?}"
        );
        assert_eq!(
            retry_class_for(&error),
            TaskRetryClass::Auto,
            "transient storage IO must be retryable"
        );
    }

    #[tokio::test]
    async fn corrupt_source_png_maps_to_never_retry_validation_error() {
        let state = isolated_state().await;
        let corrupt_key = "textures/corrupt-source.png";
        let temp_png =
            std::env::temp_dir().join(format!("corrupt-source-{}.png", uuid::Uuid::new_v4()));
        std::fs::write(&temp_png, b"definitely not a png payload")
            .expect("corrupt fixture should write");
        state
            .texture_storage()
            .put_file(corrupt_key, &temp_png)
            .await
            .expect("corrupt object should store");
        let _ = std::fs::remove_file(&temp_png);

        let hash = "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aabb7788ccdd99ee";
        // minecraft_texture.user_id -> users.id (FK): seed the owner row.
        use aster_yggdrasil_model::entities::user;
        use sea_orm::{ActiveModelTrait as UserActive, Set as UserSet, Unchanged as UserUnchanged};
        user::ActiveModel {
            id: UserUnchanged(1),
            public_uuid: UserSet("exec-paths-owner-uuid".to_string()),
            username: UserSet("exec-paths-owner".to_string()),
            email: UserSet("exec-paths-owner@example.test".to_string()),
            password_hash: UserSet("not-a-real-hash".to_string()),
            role: UserSet(aster_yggdrasil_model::types::user::UserRole::Admin),
            status: UserSet(aster_yggdrasil_model::types::user::UserStatus::Active),
            must_change_password: UserSet(false),
            session_version: UserSet(1),
            email_verified_at: UserSet(None),
            pending_email: UserSet(None),
            created_at: UserSet(chrono::Utc::now()),
            updated_at: UserSet(chrono::Utc::now()),
        }
        .insert(state.writer_db())
        .await
        .expect("owner user should insert");
        crate::db::repository::minecraft_texture_repo::create(
            state.writer_db(),
            crate::db::repository::minecraft_texture_repo::CreateMinecraftTexture {
                user_id: 1,
                texture_type: aster_yggdrasil_model::types::yggdrasil::MinecraftTextureType::Skin,
                hash,
                storage_key: corrupt_key,
                mime_type: "image/png",
                file_size: 28,
                width: 64,
                height: 64,
                texture_model: MinecraftTextureModel::Default,
                visibility:
                    aster_yggdrasil_model::types::yggdrasil::MinecraftTextureVisibility::Private,
                is_wardrobe_item: false,
                display_name: None,
            },
        )
        .await
        .expect("corrupt texture row should insert");

        let outcome = generate_preview_object(
            &state,
            &TexturePreviewGenerateTaskPayload {
                texture_hash: hash.to_string(),
                texture_model: MinecraftTextureModel::Default,
            },
        )
        .await;
        let error = outcome.expect_err("corrupt source must fail the render");
        assert!(
            matches!(
                error,
                AsterError::Public {
                    code: crate::api::error_code::AsterErrorCode::MinecraftTextureInvalidPng,
                    ..
                }
            ),
            "corrupt source maps to the invalid-png validation error, got {error:?}"
        );
        assert_eq!(
            retry_class_for(&error),
            TaskRetryClass::Never,
            "deterministic render failure must never retry"
        );
    }
}
