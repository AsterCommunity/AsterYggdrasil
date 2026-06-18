//! Texture preview lookup, fingerprinting, cache, and rendering entrypoints.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use aster_texture_renderer::{
    DecodedSkin, RenderWorkspace, SkinModel, render_decoded_preview_with_workspace,
};
use image::{ExtendedColorType, ImageEncoder};
use serde::Serialize;
use sha2::Digest;
use tokio::io::AsyncReadExt;

use crate::api::error_code::AsterErrorCode;
use crate::config::texture_preview::{RuntimeTexturePreviewPolicy, TexturePreviewSpec};
use crate::entities::minecraft_texture;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{
    AppConfigRuntimeState, DatabaseRuntimeState, ObjectStorageRuntimeState,
    RuntimeConfigRuntimeState,
};
use crate::types::{MinecraftTextureModel, MinecraftTextureType};

use super::{default_skin, is_valid_texture_hash};

pub const TEXTURE_PREVIEW_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[derive(Debug, Clone)]
pub struct TexturePreviewBytes {
    pub bytes: Vec<u8>,
    pub storage_key: String,
}

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

pub async fn texture_preview_by_hash<S>(
    state: &S,
    hash: &str,
    file_name: &str,
) -> Result<TexturePreviewBytes>
where
    S: DatabaseRuntimeState
        + RuntimeConfigRuntimeState
        + ObjectStorageRuntimeState
        + AppConfigRuntimeState,
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
    if state.object_storage().exists(&storage_key).await? {
        let bytes = read_object_storage_key(state, &storage_key).await?;
        tracing::debug!(
            hash,
            storage_key,
            size = bytes.len(),
            "served cached texture preview"
        );
        return Ok(TexturePreviewBytes { bytes, storage_key });
    }

    let bytes = render_preview_png(source.bytes, source.texture_model, policy.spec).await?;
    let temp_path = preview_temp_path(state, hash, &expected_file_name);
    write_preview_temp_file(&temp_path, &bytes).await?;
    let put_result = state
        .object_storage()
        .put_file(&storage_key, &temp_path)
        .await;
    cleanup_preview_temp_file(&temp_path).await;
    put_result?;

    tracing::debug!(
        hash,
        storage_key,
        size = bytes.len(),
        "rendered and cached texture preview"
    );
    Ok(TexturePreviewBytes { bytes, storage_key })
}

struct PreviewSource {
    bytes: Vec<u8>,
    texture_model: MinecraftTextureModel,
}

async fn resolve_preview_source<S>(state: &S, hash: &str) -> Result<PreviewSource>
where
    S: DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    S: ObjectStorageRuntimeState,
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
    S: ObjectStorageRuntimeState,
{
    let mut stream = state.object_storage().get_stream(storage_key).await?;
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

fn render_error(error: aster_texture_renderer::RenderError) -> AsterError {
    AsterError::internal_error_code(
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

async fn cleanup_preview_temp_file(path: &Path) {
    if let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            path = %path.display(),
            error = %error,
            "failed to remove texture preview temp file"
        );
    }
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
