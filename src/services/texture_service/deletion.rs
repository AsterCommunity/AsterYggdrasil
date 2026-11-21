//! Texture deletion across wardrobe, profiles, and hash-level admin cleanup.

use super::moderation::cleanup_texture_asset_if_unreferenced;
use super::shared::{
    invalidate_profile_texture_caches, invalidate_texture_asset_caches, is_valid_texture_hash,
};
use crate::db::repository::{
    minecraft_profile_repo, minecraft_profile_texture_repo, minecraft_texture_repo,
};
use crate::errors::{AsterError, Result};
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState, TextureStorageRuntimeState};
use crate::services::ban_service;
use aster_yggdrasil_model::entities::minecraft_profile;
use aster_yggdrasil_model::types::{user::UserBanScope, yggdrasil::MinecraftTextureType};

use self::maintenance::cleanup_texture_blob_if_unreferenced;

use super::*;

pub async fn delete_texture<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<Option<minecraft_profile_texture_repo::ProfileTexture>, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    ban_service::ensure_user_not_banned(state, profile.user_id, UserBanScope::TextureUpload)
        .await
        .map_err(TextureError::from)?;
    ban_service::ensure_user_not_banned(
        state,
        profile.user_id,
        UserBanScope::MinecraftProfileManage,
    )
    .await
    .map_err(TextureError::from)?;
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
        invalidate_texture_asset_caches(state, &texture.texture).await;
    }
    if deleted.is_some() {
        invalidate_profile_texture_caches(state, profile.id).await;
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
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
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
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
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
        invalidate_texture_asset_caches(state, &texture.texture).await;
    }
    if deleted.is_some() {
        invalidate_profile_texture_caches(state, profile.id).await;
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
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
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
        invalidate_profile_texture_caches(state, profile.id).await;
        invalidate_texture_asset_caches(state, &deleted_texture.texture).await;
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
        invalidate_texture_asset_caches(state, &texture).await;
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
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
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
        invalidate_texture_asset_caches(state, &deleted_texture.texture).await;
        deleted.push(deleted_texture);
    }
    if !deleted.is_empty() {
        invalidate_profile_texture_caches(state, profile.id).await;
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
