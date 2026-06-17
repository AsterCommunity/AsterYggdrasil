//! Minecraft texture validation, storage and lookup.

mod default_skin;
mod error;
mod maintenance;
mod processing;
mod query;
mod types;

#[cfg(test)]
mod tests;

pub use default_skin::{
    DefaultSkin as EmbeddedDefaultSkin, by_hash as embedded_default_skin_by_hash,
    for_profile_uuid as default_skin_for_profile_uuid,
};
pub use error::{TextureError, TextureErrorKind};
pub use maintenance::{
    OrphanTextureCleanupResult, TextureStorageConsistencyIssue, TextureStorageConsistencyIssueKind,
    TextureStorageConsistencyReport, check_texture_storage_consistency,
    cleanup_orphan_texture_blobs,
};
pub use processing::{TextureProcessingResult, process_texture_file, sanitize_png_texture};
pub use query::{
    default_skin_metadata, download_texture, download_texture_blob, texture_blob_by_hash,
    texture_by_hash, texture_metadata, texture_metadata_for_profile, textures_for_profile,
    wardrobe_texture_metadata,
};
pub use types::{
    DeletedMinecraftTexture, MinecraftTextureMetadata, MinecraftTextureMetadataSource,
    MinecraftWardrobeTextureMetadata, StoredTexture, StoredWardrobeTexture, TextureBlob,
    TextureDownload, WardrobeRegistrationResult,
};

use crate::api::pagination::OffsetPage;
use crate::db::repository::{
    minecraft_profile_repo, minecraft_profile_texture_repo, minecraft_texture_repo,
};
use crate::entities::{minecraft_profile, minecraft_texture, yggdrasil_token};
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState};
use crate::types::{MinecraftTextureModel, MinecraftTextureType, MinecraftTextureVisibility};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

use self::maintenance::cleanup_texture_blob_if_unreferenced;

const PNG_CONTENT_TYPE: &str = "image/png";
pub(crate) const TEXTURE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

pub fn parse_texture_type(value: &str) -> std::result::Result<MinecraftTextureType, TextureError> {
    MinecraftTextureType::parse_path(value)
        .ok_or_else(|| TextureError::new(TextureErrorKind::InvalidTextureType))
}

pub fn parse_skin_model(
    value: Option<&str>,
) -> std::result::Result<MinecraftTextureModel, TextureError> {
    match value.unwrap_or_default().trim() {
        "" | "default" => Ok(MinecraftTextureModel::Default),
        "slim" => Ok(MinecraftTextureModel::Slim),
        _ => Err(TextureError::with_detail(
            TextureErrorKind::InvalidDimensions,
            "Invalid skin model.",
        )),
    }
}

pub async fn write_multipart_texture_field_to_file(
    field: &mut actix_multipart::Field,
    path: &Path,
    max_upload_bytes: u64,
) -> std::result::Result<(), TextureError> {
    tracing::debug!(
        max_upload_bytes,
        "writing multipart texture upload to temp file"
    );
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            TextureError::with_detail(
                TextureErrorKind::Storage,
                format!("Failed to create upload temp dir: {error}"),
            )
        })?;
    }
    let mut file = tokio::fs::File::create(path).await.map_err(|error| {
        TextureError::with_detail(
            TextureErrorKind::Storage,
            format!("Failed to create upload temp file: {error}"),
        )
    })?;
    let mut written: u64 = 0;
    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|error| {
            TextureError::with_detail(
                TextureErrorKind::InvalidPng,
                format!("Invalid multipart file field: {error}"),
            )
        })?;
        let chunk_len = crate::utils::numbers::usize_to_u64(chunk.len(), "texture upload chunk")
            .map_err(TextureError::from)?;
        written = written.checked_add(chunk_len).ok_or_else(|| {
            TextureError::with_detail(
                TextureErrorKind::InvalidDimensions,
                "Texture upload is too large.",
            )
        })?;
        if written > max_upload_bytes {
            return Err(TextureError::with_detail(
                TextureErrorKind::InvalidDimensions,
                format!("Texture upload exceeds {max_upload_bytes} bytes."),
            ));
        }
        file.write_all(&chunk).await.map_err(|error| {
            TextureError::with_detail(
                TextureErrorKind::Storage,
                format!("Failed to write upload temp file: {error}"),
            )
        })?;
    }
    tracing::debug!(written, "finished writing multipart texture upload");
    file.flush().await.map_err(|error| {
        TextureError::with_detail(
            TextureErrorKind::Storage,
            format!("Failed to flush upload temp file: {error}"),
        )
    })
}

pub async fn authenticate_texture_access<S: DatabaseRuntimeState>(
    state: &S,
    access_token: &str,
    profile_uuid: &str,
) -> std::result::Result<(yggdrasil_token::Model, minecraft_profile::Model), TextureError> {
    tracing::debug!(profile_uuid, "authenticating texture upload access");
    let token = crate::services::yggdrasil_service::active_token_for_protocol(state, access_token)
        .await
        .map_err(|_| TextureError::new(TextureErrorKind::InvalidToken))?;
    let Some(selected_profile_id) = token.selected_profile_id else {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "texture access rejected because token has no selected profile"
        );
        return Err(TextureError::new(TextureErrorKind::InvalidToken));
    };
    let profile = minecraft_profile_repo::find_by_id(state.reader_db(), selected_profile_id)
        .await
        .map_err(TextureError::from)?;
    if profile.uuid != profile_uuid {
        tracing::debug!(
            token_id = token.id,
            profile_id = profile.id,
            expected_profile_uuid = %profile.uuid,
            requested_profile_uuid = profile_uuid,
            "texture access rejected because profile uuid did not match selected token profile"
        );
        return Err(TextureError::new(TextureErrorKind::ForbiddenProfile));
    }
    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        "texture upload access authenticated"
    );
    Ok((token, profile))
}

pub async fn store_texture<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
    texture_model: MinecraftTextureModel,
    source_path: PathBuf,
) -> std::result::Result<StoredTexture, TextureError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        user_id = profile.user_id,
        texture_type = ?texture_type,
        texture_model = ?texture_model,
        "storing profile texture"
    );
    ensure_upload_allowed(profile, texture_type)?;
    let wardrobe_texture = store_or_reuse_wardrobe_texture(
        state,
        StoreTextureAssetInput {
            user_id: profile.user_id,
            texture_type,
            texture_model,
            source_path,
            visibility: MinecraftTextureVisibility::Private,
            cleanup_reason: "launcher texture wardrobe registration failure",
        },
    )
    .await?;
    let previous = minecraft_profile_texture_repo::find_by_profile_and_type(
        state.reader_db(),
        profile.id,
        texture_type,
    )
    .await
    .map_err(TextureError::from)?;

    let texture = minecraft_profile_texture_repo::upsert_for_profile(
        state.writer_db(),
        minecraft_profile_texture_repo::UpsertMinecraftProfileTexture {
            profile_id: profile.id,
            texture_id: wardrobe_texture.id,
            texture_type,
        },
    )
    .await;
    let texture = match texture {
        Ok(texture) => texture,
        Err(error) => {
            cleanup_texture_asset_if_unreferenced(state, &wardrobe_texture, "texture bind failure")
                .await;
            return Err(TextureError::from(error));
        }
    };

    if let Some(previous) = previous.as_ref()
        && previous.texture.id != texture.texture.id
    {
        cleanup_texture_asset_if_unreferenced(state, &previous.texture, "texture reupload").await;
    }

    tracing::debug!(
        profile_id = profile.id,
        profile_texture_id = texture.binding.id,
        texture_id = texture.texture.id,
        wardrobe_texture_id = wardrobe_texture.id,
        replaced_texture_id = previous.as_ref().map(|previous| previous.texture.id),
        "stored profile texture"
    );
    Ok(StoredTexture {
        texture,
        profile: profile.clone(),
        wardrobe_texture,
    })
}

pub async fn store_wardrobe_texture<S>(
    state: &S,
    user_id: i64,
    texture_type: MinecraftTextureType,
    texture_model: MinecraftTextureModel,
    visibility: MinecraftTextureVisibility,
    source_path: PathBuf,
) -> std::result::Result<StoredWardrobeTexture, TextureError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        user_id,
        texture_type = ?texture_type,
        texture_model = ?texture_model,
        visibility = ?visibility,
        "storing wardrobe texture"
    );
    let texture = store_or_reuse_wardrobe_texture(
        state,
        StoreTextureAssetInput {
            user_id,
            texture_type,
            texture_model,
            source_path,
            visibility,
            cleanup_reason: "wardrobe texture insert failure",
        },
    )
    .await?;
    tracing::debug!(
        user_id,
        texture_id = texture.id,
        hash = %texture.hash,
        "stored wardrobe texture"
    );
    Ok(StoredWardrobeTexture { texture })
}

pub async fn register_bound_textures_in_wardrobe<S>(
    state: &S,
) -> std::result::Result<WardrobeRegistrationResult, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let bindings = minecraft_profile_texture_repo::list_all(state.reader_db())
        .await
        .map_err(TextureError::from)?;
    let scanned_bindings =
        crate::utils::numbers::usize_to_u64(bindings.len(), "wardrobe registration scan")
            .map_err(TextureError::from)?;
    let mut result = WardrobeRegistrationResult {
        scanned_bindings,
        converted_textures: 0,
        rebound_bindings: 0,
        removed_duplicate_textures: 0,
    };
    tracing::debug!(scanned_bindings, "registering bound textures in wardrobe");

    for binding in bindings {
        if binding.texture.is_wardrobe_item {
            continue;
        }

        let Some(existing) = minecraft_texture_repo::find_wardrobe_by_fingerprint(
            state.reader_db(),
            binding.texture.user_id,
            binding.texture.texture_type,
            &binding.texture.hash,
            binding.texture.texture_model,
        )
        .await
        .map_err(TextureError::from)?
        else {
            minecraft_texture_repo::mark_as_wardrobe_item(state.writer_db(), binding.texture)
                .await
                .map_err(TextureError::from)?;
            result.converted_textures += 1;
            continue;
        };

        minecraft_profile_texture_repo::upsert_for_profile(
            state.writer_db(),
            minecraft_profile_texture_repo::UpsertMinecraftProfileTexture {
                profile_id: binding.binding.profile_id,
                texture_id: existing.id,
                texture_type: binding.binding.texture_type,
            },
        )
        .await
        .map_err(TextureError::from)?;
        result.rebound_bindings += 1;

        if let Some(deleted) = minecraft_texture_repo::delete_by_id_for_user(
            state.writer_db(),
            binding.texture.id,
            binding.texture.user_id,
        )
        .await
        .map_err(TextureError::from)?
        {
            cleanup_texture_blob_if_unreferenced(
                state,
                &deleted.storage_key,
                "wardrobe registration duplicate texture",
            )
            .await;
            result.removed_duplicate_textures += 1;
        }
    }

    tracing::debug!(
        scanned_bindings = result.scanned_bindings,
        converted_textures = result.converted_textures,
        rebound_bindings = result.rebound_bindings,
        removed_duplicate_textures = result.removed_duplicate_textures,
        "finished registering bound textures in wardrobe"
    );
    Ok(result)
}

struct StoreTextureAssetInput<'a> {
    user_id: i64,
    texture_type: MinecraftTextureType,
    texture_model: MinecraftTextureModel,
    source_path: PathBuf,
    visibility: MinecraftTextureVisibility,
    cleanup_reason: &'a str,
}

async fn store_or_reuse_wardrobe_texture<S>(
    state: &S,
    input: StoreTextureAssetInput<'_>,
) -> std::result::Result<minecraft_texture::Model, TextureError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + TextureStorageRuntimeState,
{
    let StoreTextureAssetInput {
        user_id,
        texture_type,
        texture_model,
        source_path,
        visibility,
        cleanup_reason,
    } = input;
    let policy = crate::config::yggdrasil::RuntimeYggdrasilPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let processed_path = temporary_processed_path(&source_path);
    tracing::debug!(
        user_id,
        texture_type = ?texture_type,
        texture_model = ?texture_model,
        visibility = ?visibility,
        max_texture_pixels = policy.max_texture_pixels,
        "processing texture asset"
    );
    let processing = process_texture_file(
        &source_path,
        &processed_path,
        texture_type,
        policy.max_texture_pixels,
    )
    .await
    .map_err(|error| TextureError::with_detail(TextureErrorKind::InvalidPng, error.message()))?;
    let storage_key = texture_storage_key(&processing.hash);
    tracing::debug!(
        user_id,
        texture_type = ?texture_type,
        hash = %processing.hash,
        width = processing.width,
        height = processing.height,
        file_size = processing.file_size,
        "processed texture asset"
    );

    if let Some(existing) = minecraft_texture_repo::find_wardrobe_by_fingerprint(
        state.reader_db(),
        user_id,
        texture_type,
        &processing.hash,
        texture_model,
    )
    .await
    .map_err(TextureError::from)?
    {
        cleanup_temp_file(&processed_path).await;
        tracing::debug!(
            user_id,
            texture_id = existing.id,
            hash = %existing.hash,
            "reusing existing wardrobe texture asset"
        );
        return Ok(existing);
    }

    state
        .texture_storage()
        .put_file(&storage_key, &processed_path)
        .await
        .map_err(TextureError::from)?;
    tracing::debug!(user_id, hash = %processing.hash, "stored texture blob");
    cleanup_temp_file(&processed_path).await;

    let file_size = crate::utils::numbers::u64_to_i64(processing.file_size, "texture file size")
        .map_err(TextureError::from)?;
    let width = crate::utils::numbers::u32_to_i32(processing.width, "texture width")
        .map_err(TextureError::from)?;
    let height = crate::utils::numbers::u32_to_i32(processing.height, "texture height")
        .map_err(TextureError::from)?;
    let texture = minecraft_texture_repo::create(
        state.writer_db(),
        minecraft_texture_repo::CreateMinecraftTexture {
            user_id,
            texture_type,
            hash: &processing.hash,
            storage_key: &storage_key,
            mime_type: PNG_CONTENT_TYPE,
            file_size,
            width,
            height,
            texture_model,
            visibility,
            is_wardrobe_item: true,
        },
    )
    .await;
    match texture {
        Ok(texture) => {
            tracing::debug!(
                user_id,
                texture_id = texture.id,
                hash = %texture.hash,
                "created wardrobe texture asset record"
            );
            Ok(texture)
        }
        Err(error) => {
            cleanup_texture_blob_if_unreferenced(state, &storage_key, cleanup_reason).await;
            Err(TextureError::from(error))
        }
    }
}

async fn cleanup_texture_asset_if_unreferenced<S>(
    state: &S,
    texture: &minecraft_texture::Model,
    reason: &str,
) where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    if texture.is_wardrobe_item {
        tracing::debug!(
            texture_id = texture.id,
            reason,
            "skipping wardrobe texture asset cleanup"
        );
        return;
    }
    match minecraft_profile_texture_repo::count_by_texture_id(state.reader_db(), texture.id).await {
        Ok(0) => {
            match minecraft_texture_repo::delete_by_id_for_user(
                state.writer_db(),
                texture.id,
                texture.user_id,
            )
            .await
            {
                Ok(_) => {
                    cleanup_texture_blob_if_unreferenced(state, &texture.storage_key, reason).await;
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        texture_id = texture.id,
                        reason,
                        "failed to delete unreferenced texture asset"
                    );
                }
            }
        }
        Ok(ref_count) => {
            tracing::debug!(
                texture_id = texture.id,
                ref_count,
                reason,
                "skipping texture asset cleanup because it is still bound"
            );
        }
        Err(error) => {
            tracing::warn!(
                error = %error,
                texture_id = texture.id,
                reason,
                "failed to count profile texture bindings before cleanup"
            );
        }
    }
}

pub async fn list_wardrobe_textures<S>(
    state: &S,
    user_id: i64,
) -> Result<Vec<minecraft_texture::Model>>
where
    S: DatabaseRuntimeState,
{
    let textures = minecraft_texture_repo::list_by_user(state.reader_db(), user_id).await?;
    tracing::debug!(user_id, count = textures.len(), "listed wardrobe textures");
    Ok(textures)
}

pub async fn list_wardrobe_textures_paginated<S>(
    state: &S,
    user_id: i64,
    limit: u64,
    offset: u64,
    filter: minecraft_texture_repo::WardrobeTextureListFilter,
) -> Result<OffsetPage<minecraft_texture::Model>>
where
    S: DatabaseRuntimeState,
{
    let page = minecraft_texture_repo::list_by_user_paginated(
        state.reader_db(),
        user_id,
        limit,
        offset,
        filter,
    )
    .await?;
    tracing::debug!(
        user_id,
        returned = page.items.len(),
        total = page.total,
        limit = page.limit,
        offset = page.offset,
        "listed wardrobe textures page"
    );
    Ok(page)
}

pub async fn delete_wardrobe_texture<S>(
    state: &S,
    user_id: i64,
    wardrobe_texture_id: i64,
) -> std::result::Result<minecraft_texture::Model, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        user_id,
        texture_id = wardrobe_texture_id,
        "deleting wardrobe texture"
    );
    let Some(deleted) = minecraft_texture_repo::delete_by_id_for_user(
        state.writer_db(),
        wardrobe_texture_id,
        user_id,
    )
    .await
    .map_err(TextureError::from)?
    else {
        return Err(TextureError::with_detail(
            TextureErrorKind::NotFound,
            format!("wardrobe texture #{wardrobe_texture_id}"),
        ));
    };

    cleanup_texture_blob_if_unreferenced(state, &deleted.storage_key, "wardrobe texture delete")
        .await;
    tracing::debug!(
        user_id,
        texture_id = deleted.id,
        hash = %deleted.hash,
        "deleted wardrobe texture"
    );
    Ok(deleted)
}

pub async fn bind_wardrobe_texture_to_profile<S>(
    state: &S,
    user_id: i64,
    profile: &minecraft_profile::Model,
    wardrobe_texture_id: i64,
    texture_type: MinecraftTextureType,
) -> std::result::Result<StoredTexture, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        user_id,
        profile_id = profile.id,
        texture_id = wardrobe_texture_id,
        texture_type = ?texture_type,
        "binding wardrobe texture to profile"
    );
    ensure_upload_allowed(profile, texture_type)?;
    let Some(wardrobe_texture) = minecraft_texture_repo::find_by_id_for_user(
        state.reader_db(),
        wardrobe_texture_id,
        user_id,
    )
    .await
    .map_err(TextureError::from)?
    else {
        return Err(TextureError::with_detail(
            TextureErrorKind::NotFound,
            "wardrobe texture #{wardrobe_texture_id}",
        ));
    };
    if wardrobe_texture.texture_type != texture_type {
        return Err(TextureError::with_detail(
            TextureErrorKind::InvalidTextureType,
            "Wardrobe texture type does not match the target slot.",
        ));
    }

    let previous = minecraft_profile_texture_repo::find_by_profile_and_type(
        state.reader_db(),
        profile.id,
        texture_type,
    )
    .await
    .map_err(TextureError::from)?;
    let texture = minecraft_profile_texture_repo::upsert_for_profile(
        state.writer_db(),
        minecraft_profile_texture_repo::UpsertMinecraftProfileTexture {
            profile_id: profile.id,
            texture_id: wardrobe_texture.id,
            texture_type,
        },
    )
    .await
    .map_err(TextureError::from)?;

    if let Some(previous) = previous.as_ref()
        && previous.texture.id != texture.texture.id
    {
        cleanup_texture_asset_if_unreferenced(state, &previous.texture, "wardrobe bind").await;
    }

    tracing::debug!(
        user_id,
        profile_id = profile.id,
        profile_texture_id = texture.binding.id,
        texture_id = texture.texture.id,
        replaced_texture_id = previous.as_ref().map(|previous| previous.texture.id),
        "bound wardrobe texture to profile"
    );
    Ok(StoredTexture {
        texture,
        profile: profile.clone(),
        wardrobe_texture,
    })
}

pub async fn delete_texture<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<Option<minecraft_profile_texture_repo::ProfileTexture>, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        profile_id = profile.id,
        texture_type = ?texture_type,
        "deleting profile texture"
    );
    ensure_upload_allowed(profile, texture_type)?;
    let deleted = minecraft_profile_texture_repo::delete_for_profile(
        state.writer_db(),
        profile.id,
        texture_type,
    )
    .await
    .map_err(TextureError::from)?;
    if let Some(texture) = deleted.as_ref() {
        cleanup_texture_asset_if_unreferenced(state, &texture.texture, "texture delete").await;
    }
    tracing::debug!(
        profile_id = profile.id,
        texture_type = ?texture_type,
        deleted = deleted.is_some(),
        "profile texture delete completed"
    );
    Ok(deleted)
}

pub async fn delete_texture_for_profile<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<Option<DeletedMinecraftTexture>, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let deleted = delete_texture(state, profile, texture_type).await?;
    Ok(deleted.map(|texture| DeletedMinecraftTexture {
        texture,
        profile: profile.clone(),
    }))
}

pub async fn delete_texture_for_profile_unchecked<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<Option<DeletedMinecraftTexture>, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        profile_id = profile.id,
        texture_type = ?texture_type,
        "deleting profile texture without upload permission check"
    );
    let deleted = minecraft_profile_texture_repo::delete_for_profile(
        state.writer_db(),
        profile.id,
        texture_type,
    )
    .await
    .map_err(TextureError::from)?;
    if let Some(texture) = deleted.as_ref() {
        cleanup_texture_asset_if_unreferenced(state, &texture.texture, "admin texture delete")
            .await;
    }
    tracing::debug!(
        profile_id = profile.id,
        texture_type = ?texture_type,
        deleted = deleted.is_some(),
        "unchecked profile texture delete completed"
    );
    Ok(deleted.map(|texture| DeletedMinecraftTexture {
        texture,
        profile: profile.clone(),
    }))
}

pub async fn delete_textures_by_hash<S>(
    state: &S,
    hash: &str,
) -> std::result::Result<Vec<DeletedMinecraftTexture>, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    if !is_valid_texture_hash(hash) {
        tracing::debug!(hash, "delete textures by hash skipped invalid hash");
        return Ok(Vec::new());
    }
    tracing::debug!(hash, "deleting textures by hash");
    let textures = minecraft_profile_texture_repo::list_by_hash(state.reader_db(), hash)
        .await
        .map_err(TextureError::from)?;
    let mut deleted = Vec::new();
    for texture in textures {
        let profile =
            minecraft_profile_repo::find_by_id(state.reader_db(), texture.binding.profile_id)
                .await
                .map_err(TextureError::from)?;
        let Some(deleted_texture) = minecraft_profile_texture_repo::delete_for_profile(
            state.writer_db(),
            profile.id,
            texture.binding.texture_type,
        )
        .await
        .map_err(TextureError::from)?
        else {
            continue;
        };
        cleanup_texture_asset_if_unreferenced(
            state,
            &deleted_texture.texture,
            "hash texture delete",
        )
        .await;
        deleted.push(DeletedMinecraftTexture {
            texture: deleted_texture,
            profile,
        });
    }
    let deleted_assets = minecraft_texture_repo::delete_by_hash(state.writer_db(), hash)
        .await
        .map_err(TextureError::from)?;
    for texture in deleted_assets {
        cleanup_texture_blob_if_unreferenced(state, &texture.storage_key, "hash texture delete")
            .await;
    }
    tracing::debug!(
        hash,
        deleted_bindings = deleted.len(),
        "delete textures by hash completed"
    );
    Ok(deleted)
}

pub async fn delete_all_textures_for_profile<S>(
    state: &S,
    profile: &minecraft_profile::Model,
) -> std::result::Result<Vec<minecraft_profile_texture_repo::ProfileTexture>, TextureError>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(profile_id = profile.id, "deleting all textures for profile");
    let textures = minecraft_profile_texture_repo::list_by_profile(state.reader_db(), profile.id)
        .await
        .map_err(TextureError::from)?;
    let mut deleted = Vec::new();
    for texture in textures {
        let Some(deleted_texture) = minecraft_profile_texture_repo::delete_for_profile(
            state.writer_db(),
            profile.id,
            texture.binding.texture_type,
        )
        .await
        .map_err(TextureError::from)?
        else {
            continue;
        };
        cleanup_texture_asset_if_unreferenced(
            state,
            &deleted_texture.texture,
            "profile texture delete",
        )
        .await;
        deleted.push(deleted_texture);
    }
    tracing::debug!(
        profile_id = profile.id,
        deleted = deleted.len(),
        "deleted all textures for profile"
    );
    Ok(deleted)
}

pub(super) fn validate_texture_dimensions(
    texture_type: MinecraftTextureType,
    width: u32,
    height: u32,
) -> Result<()> {
    let valid = match texture_type {
        MinecraftTextureType::Skin => {
            is_multiple_texture_size(width, height, 64, 32)
                || is_multiple_texture_size(width, height, 64, 64)
        }
        MinecraftTextureType::Cape => {
            is_multiple_texture_size(width, height, 64, 32)
                || is_multiple_texture_size(width, height, 22, 17)
        }
    };
    if !valid {
        return Err(AsterError::validation_error(format!(
            "invalid {} texture dimensions: {}x{}",
            texture_type.as_str(),
            width,
            height
        )));
    }
    Ok(())
}

pub(super) fn is_multiple_texture_size(
    width: u32,
    height: u32,
    unit_width: u32,
    unit_height: u32,
) -> bool {
    width >= unit_width
        && height >= unit_height
        && width.is_multiple_of(unit_width)
        && height.is_multiple_of(unit_height)
        && width / unit_width == height / unit_height
}

fn ensure_upload_allowed(
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<(), TextureError> {
    let allowed = profile
        .uploadable_textures
        .split(',')
        .map(str::trim)
        .any(|item| item == texture_type.as_str());
    if allowed {
        Ok(())
    } else {
        Err(TextureError::new(TextureErrorKind::UploadDisabled))
    }
}

fn texture_storage_key(hash: &str) -> String {
    let prefix = &hash[..2];
    format!("{prefix}/{hash}.png")
}

fn temporary_processed_path(source_path: &Path) -> PathBuf {
    let mut path = source_path.to_path_buf();
    path.set_extension("processed.png");
    path
}

async fn cleanup_temp_file(path: &Path) {
    if let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), error = %error, "failed to remove temp texture file");
    }
}

fn is_valid_texture_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
