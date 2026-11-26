//! Player wardrobe: metadata editing, listing, copy-in, binding, and deletion.

use super::library::texture_next_cursor;
use super::moderation::{
    cleanup_texture_asset_if_unreferenced, ensure_texture_library_enabled,
    normalize_texture_display_name_patch, normalize_texture_tag_ids,
};
use super::shared::{
    invalidate_profile_texture_caches, invalidate_profile_texture_caches_for_bindings,
    invalidate_texture_asset_caches,
};
use crate::db::repository::{
    minecraft_profile_texture_repo, minecraft_texture_repo, minecraft_texture_tag_repo,
};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState,
};
use crate::services::user;
use aster_forge_api::{CursorPage, DateTimeIdCursor, NullablePatch};
use aster_yggdrasil_model::entities::{minecraft_profile, minecraft_texture};
use aster_yggdrasil_model::types::permission::capability;
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureModel, yggdrasil::MinecraftTextureType,
    yggdrasil::MinecraftTextureVisibility,
};

use self::maintenance::cleanup_texture_blob_if_unreferenced;

use super::*;

/// Updates a wardrobe texture's display name, skin model, or visibility on
/// behalf of its owner. `NullablePatch` fields distinguish "clear the value"
/// from "leave unchanged".
pub async fn update_wardrobe_texture_metadata<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
    display_name: Option<NullablePatch<String>>,
    texture_model: Option<MinecraftTextureModel>,
    visibility: Option<MinecraftTextureVisibility>,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: CacheRuntimeState + DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    user::ban::ensure_user_not_banned(state, user_id, capability::texture::UPLOAD).await?;
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
            "invalid wardrobe texture id",
        ));
    }
    let texture =
        minecraft_texture_repo::find_by_id_for_user(state.reader_db(), texture_id, user_id)
            .await?
            .filter(|texture| texture.is_wardrobe_item)
            .ok_or_else(|| {
                AsterError::record_not_found_code(
                    crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
                    format!("wardrobe texture '{texture_id}'"),
                )
            })?;
    let display_name = display_name
        .map(normalize_texture_display_name_patch)
        .transpose()?;
    let texture_model = texture_model.map(|model| {
        if texture.texture_type == MinecraftTextureType::Skin {
            model
        } else {
            MinecraftTextureModel::Default
        }
    });
    let texture_model_changed =
        texture_model.is_some_and(|texture_model| texture_model != texture.texture_model);
    let updated = minecraft_texture_repo::update_wardrobe_metadata_for_user(
        state.writer_db(),
        texture,
        user_id,
        minecraft_texture_repo::UpdateWardrobeTextureMetadata {
            display_name,
            texture_model,
            visibility,
        },
    )
    .await?
    .ok_or_else(|| {
        AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
            format!("wardrobe texture '{texture_id}'"),
        )
    })?;
    if texture_model_changed {
        let bindings =
            minecraft_profile_texture_repo::list_by_texture_id(state.reader_db(), texture_id)
                .await?;
        invalidate_profile_texture_caches_for_bindings(state, &bindings).await;
    }
    let tags = minecraft_texture_tag_repo::list_for_texture(state.reader_db(), updated.id).await?;
    Ok(wardrobe_texture_metadata_with_tags(state, &updated, tags))
}

/// Replaces the full set of library tags on a wardrobe texture with the given
/// ids; duplicates are collapsed and every id must exist.
pub async fn replace_wardrobe_texture_tags<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
    tag_ids: &[i64],
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    user::ban::ensure_user_not_banned(state, user_id, capability::texture::UPLOAD).await?;
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
            "invalid wardrobe texture id",
        ));
    }
    let texture =
        minecraft_texture_repo::find_by_id_for_user(state.reader_db(), texture_id, user_id)
            .await?
            .filter(|texture| texture.is_wardrobe_item)
            .ok_or_else(|| {
                AsterError::record_not_found_code(
                    crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
                    format!("wardrobe texture '{texture_id}'"),
                )
            })?;
    let unique_tag_ids = normalize_texture_tag_ids(tag_ids)?;
    let tags = minecraft_texture_tag_repo::find_by_ids(state.reader_db(), &unique_tag_ids).await?;
    if tags.len() != unique_tag_ids.len() {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNotFound,
            "one or more texture library tags were not found",
        ));
    }
    minecraft_texture_tag_repo::replace_texture_tags(
        state.writer_db(),
        texture.id,
        &unique_tag_ids,
    )
    .await?;
    Ok(wardrobe_texture_metadata_with_tags(state, &texture, tags))
}

/// Cursor page over one user's wardrobe textures with tags included;
/// `limit` is clamped to 1-100.
pub async fn list_wardrobe_textures_cursor<S>(
    state: &S,
    user_id: i64,
    limit: u64,
    after: Option<(chrono::DateTime<chrono::Utc>, i64)>,
    filter: minecraft_texture_repo::WardrobeTextureListFilter,
) -> Result<CursorPage<MinecraftWardrobeTextureMetadata, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let limit = limit.clamp(1, 100);
    let slice = minecraft_texture_repo::list_by_user_cursor(
        state.reader_db(),
        user_id,
        limit,
        after,
        filter,
    )
    .await?;
    let next_cursor = texture_next_cursor(&slice.items, slice.has_more);
    let textures = wardrobe_texture_metadata_by_texture_ids(state, &slice.items).await?;
    Ok(CursorPage::new(textures, slice.total, limit, next_cursor))
}

/// Cursor page over the published public texture library; requires the
/// library feature and clamps `limit` to 1-100.
pub async fn list_public_texture_library_cursor<S>(
    state: &S,
    limit: u64,
    after: Option<(chrono::DateTime<chrono::Utc>, i64)>,
    filter: minecraft_texture_repo::WardrobeTextureListFilter,
) -> Result<CursorPage<PublicTextureLibraryTextureMetadata, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let limit = limit.clamp(1, 100);
    let slice = minecraft_texture_repo::list_public_wardrobe_cursor(
        state.reader_db(),
        limit,
        after,
        filter,
    )
    .await?;
    let next_cursor = texture_next_cursor(&slice.items, slice.has_more);
    let textures = public_texture_library_metadata_by_texture_ids(state, &slice.items).await?;
    Ok(CursorPage::new(textures, slice.total, limit, next_cursor))
}

/// Fetches a single published library texture with public metadata; only
/// textures whose library status is published are visible here.
pub async fn get_public_texture_library_texture<S>(
    state: &S,
    texture_id: i64,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
            "invalid public texture id",
        ));
    }
    let texture = minecraft_texture_repo::find_public_wardrobe_by_id(state.reader_db(), texture_id)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
                format!("public texture '{texture_id}'"),
            )
        })?;
    public_texture_library_metadata(state, &texture).await
}

/// Copies a published library texture into the caller's wardrobe as a private
/// asset, reusing the caller's existing asset with the same content
/// fingerprint when one exists. Source tags are carried over.
pub async fn copy_public_texture_to_wardrobe<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
    display_name: Option<NullablePatch<String>>,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    user::ban::ensure_user_not_banned(state, user_id, capability::texture_library::INTERACT)
        .await?;
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
            "invalid public texture id",
        ));
    }
    let source = minecraft_texture_repo::find_public_wardrobe_by_id(state.reader_db(), texture_id)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
                format!("public texture '{texture_id}'"),
            )
        })?;

    if let Some(existing) = minecraft_texture_repo::find_wardrobe_by_fingerprint(
        state.reader_db(),
        user_id,
        source.texture_type,
        &source.hash,
        source.texture_model,
    )
    .await?
    {
        let tags =
            minecraft_texture_tag_repo::list_for_texture(state.reader_db(), existing.id).await?;
        return Ok(wardrobe_texture_metadata_with_tags(state, &existing, tags));
    }

    let display_name = match display_name {
        Some(value) => normalize_texture_display_name_patch(value)?,
        None => source.display_name.clone(),
    };
    if let Some(display_name) = display_name.as_deref()
        && minecraft_texture_repo::find_wardrobe_by_display_name(
            state.reader_db(),
            user_id,
            display_name,
        )
        .await?
        .is_some()
    {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNameTaken,
            "wardrobe texture name already exists",
        ));
    }

    let copied = minecraft_texture_repo::create_wardrobe_copy(
        state.writer_db(),
        &source,
        user_id,
        MinecraftTextureVisibility::Private,
        display_name.as_deref(),
    )
    .await?;
    let source_tags =
        minecraft_texture_tag_repo::list_for_texture(state.reader_db(), source.id).await?;
    let tag_ids = source_tags.iter().map(|tag| tag.id).collect::<Vec<_>>();
    if !tag_ids.is_empty() {
        minecraft_texture_tag_repo::replace_texture_tags(state.writer_db(), copied.id, &tag_ids)
            .await?;
    }
    tracing::debug!(
        user_id,
        source_texture_id = source.id,
        copied_texture_id = copied.id,
        "copied public texture to current user wardrobe"
    );
    Ok(wardrobe_texture_metadata_with_tags(
        state,
        &copied,
        source_tags,
    ))
}

/// Deletes one of the user's wardrobe textures (with blob cleanup when
/// unreferenced); fails after the ban check if the texture is not owned by
/// the user.
pub async fn delete_wardrobe_texture<S>(
    state: &S,
    user_id: i64,
    wardrobe_texture_id: i64,
) -> std::result::Result<minecraft_texture::Model, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    user::ban::ensure_user_not_banned(state, user_id, capability::texture::UPLOAD)
        .await
        .map_err(TextureError::from)?;
    tracing::debug!(
        user_id,
        texture_id = wardrobe_texture_id,
        "deleting wardrobe texture"
    );
    delete_wardrobe_texture_unchecked(state, user_id, wardrobe_texture_id).await
}

async fn delete_wardrobe_texture_unchecked<S>(
    state: &S,
    user_id: i64,
    wardrobe_texture_id: i64,
) -> std::result::Result<minecraft_texture::Model, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let deleted_bindings =
        minecraft_profile_texture_repo::list_by_texture_id(state.reader_db(), wardrobe_texture_id)
            .await
            .map_err(TextureError::from)?;
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
    invalidate_profile_texture_caches_for_bindings(state, &deleted_bindings).await;
    invalidate_texture_asset_caches(state, &deleted).await;
    tracing::debug!(
        user_id,
        texture_id = deleted.id,
        hash = %deleted.hash,
        "deleted wardrobe texture"
    );
    Ok(deleted)
}

/// Deletes every wardrobe texture owned by the user, skipping textures that
/// vanish concurrently; returns the rows that were removed.
pub async fn delete_all_wardrobe_textures_for_user<S>(
    state: &S,
    user_id: i64,
) -> std::result::Result<Vec<minecraft_texture::Model>, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let textures = minecraft_texture_repo::list_by_user(state.reader_db(), user_id)
        .await
        .map_err(TextureError::from)?;
    let mut deleted = Vec::with_capacity(textures.len());
    for texture in textures {
        let texture_id = texture.id;
        match delete_wardrobe_texture_unchecked(state, user_id, texture_id).await {
            Ok(texture) => deleted.push(texture),
            Err(error) if error.kind() == TextureErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(deleted)
}

/// Binds one of the user's wardrobe textures into a profile's `texture_type`
/// slot, replacing any previous binding. The texture's type must match the
/// slot, and caches are invalidated so launchers see the new texture.
pub async fn bind_wardrobe_texture_to_profile<S>(
    state: &S,
    user_id: i64,
    profile: &minecraft_profile::Model,
    wardrobe_texture_id: i64,
    texture_type: MinecraftTextureType,
) -> std::result::Result<StoredTexture, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    user::ban::ensure_user_not_banned(state, user_id, capability::texture::UPLOAD)
        .await
        .map_err(TextureError::from)?;
    user::ban::ensure_user_not_banned(state, user_id, capability::profile::MANAGE)
        .await
        .map_err(TextureError::from)?;
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
    invalidate_profile_texture_caches(state, profile.id).await;
    invalidate_texture_asset_caches(state, &texture.texture).await;
    if let Some(previous) = previous.as_ref() {
        invalidate_texture_asset_caches(state, &previous.texture).await;
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
