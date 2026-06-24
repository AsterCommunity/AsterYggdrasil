//! Minecraft texture validation, storage and lookup.

mod cache;
mod default_skin;
mod error;
mod maintenance;
mod preview;
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
    ObjectStorageConsistencyIssue, ObjectStorageConsistencyIssueKind,
    ObjectStorageConsistencyReport, OrphanTextureCleanupResult, check_object_storage_consistency,
    cleanup_orphan_texture_blobs,
};
pub use preview::{
    TEXTURE_PREVIEW_CACHE_CONTROL, TexturePreviewBytes, current_texture_preview_url,
    texture_preview_by_hash,
};
pub use processing::{TextureProcessingResult, process_texture_file, sanitize_png_texture};
pub use query::{
    admin_texture_library_metadata, admin_texture_library_metadata_by_texture_ids,
    default_skin_metadata, download_texture, download_texture_blob,
    public_texture_library_metadata, public_texture_library_metadata_by_texture_ids,
    texture_blob_by_hash, texture_by_hash, texture_metadata, texture_metadata_for_profile,
    texture_tag_info, textures_for_profile, wardrobe_texture_metadata,
    wardrobe_texture_metadata_by_texture_ids, wardrobe_texture_metadata_with_tags,
};
pub use types::{
    DeletedMinecraftTexture, MinecraftTextureMetadata, MinecraftTextureMetadataSource,
    MinecraftTextureTagInfo, MinecraftTextureUploaderInfo, MinecraftWardrobeTextureMetadata,
    PublicTextureLibraryTextureMetadata, StoredTexture, StoredWardrobeTexture, TextureBlob,
    TextureDownload, TextureReportInfo, TextureReportUserInfo, WardrobeRegistrationResult,
};

use crate::db::repository::{
    minecraft_profile_repo, minecraft_profile_texture_repo, minecraft_texture_repo,
    minecraft_texture_report_repo, minecraft_texture_tag_repo, user_repo,
};
use crate::entities::{
    minecraft_profile, minecraft_texture, minecraft_texture_report, user, yggdrasil_token,
};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, ObjectStorageRuntimeState, RuntimeConfigRuntimeState,
};
use crate::services::ban_service;
use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureReportReason,
    MinecraftTextureReportStatus, MinecraftTextureType, MinecraftTextureVisibility, NullablePatch,
    UserBanScope,
};
use aster_forge_api::{CursorPage, DateTimeIdCursor, SortOrderNameIdCursor};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::Serialize;
use std::collections::BTreeSet;
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

async fn invalidate_profile_texture_caches<S>(state: &S, profile_id: i64)
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    crate::services::yggdrasil_service::invalidate_profile_properties_cache(state, profile_id)
        .await;
    if let Err(error) = minecraft_profile_repo::touch_by_id(state.writer_db(), profile_id).await {
        tracing::warn!(
            error = %error,
            profile_id,
            "failed to touch minecraft profile after texture cache invalidation"
        );
    }
}

async fn invalidate_profile_texture_caches_for_bindings<S>(
    state: &S,
    bindings: &[minecraft_profile_texture_repo::ProfileTexture],
) where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    for profile_id in bindings
        .iter()
        .map(|binding| binding.binding.profile_id)
        .collect::<BTreeSet<_>>()
    {
        invalidate_profile_texture_caches(state, profile_id).await;
    }
}

async fn invalidate_texture_asset_caches<S>(state: &S, texture: &minecraft_texture::Model)
where
    S: CacheRuntimeState,
{
    query::invalidate_texture_blob_lookup_cache(state, &texture.hash).await;
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureTagMutationResult {
    pub tag: MinecraftTextureTagInfo,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct DeletedTextureLibraryTexture {
    pub texture: PublicTextureLibraryTextureMetadata,
    pub deleted_profile_binding_count: u64,
}

pub async fn list_texture_library_tags<S: DatabaseRuntimeState>(
    state: &S,
) -> Result<Vec<MinecraftTextureTagInfo>> {
    let tags = minecraft_texture_tag_repo::list(state.reader_db()).await?;
    Ok(tags.into_iter().map(texture_tag_info).collect())
}

pub async fn list_texture_library_tags_paginated<S: DatabaseRuntimeState>(
    state: &S,
    limit: u64,
    after: Option<(i32, String, i64)>,
    keyword: Option<String>,
) -> Result<CursorPage<MinecraftTextureTagInfo, SortOrderNameIdCursor>> {
    let limit = limit.clamp(1, 100);
    let page = minecraft_texture_tag_repo::list_cursor(
        state.reader_db(),
        limit,
        after,
        minecraft_texture_tag_repo::MinecraftTextureTagListFilter { keyword },
    )
    .await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|tag| SortOrderNameIdCursor {
            sort_order: tag.sort_order,
            name: tag.name.clone(),
            id: tag.id,
        })
    } else {
        None
    };
    Ok(CursorPage::new(
        page.items.into_iter().map(texture_tag_info).collect(),
        page.total,
        limit,
        next_cursor,
    ))
}

pub async fn create_texture_library_tag<S: DatabaseRuntimeState>(
    state: &S,
    name: &str,
    color: &str,
    sort_order: Option<i32>,
) -> Result<MinecraftTextureTagInfo> {
    let (name, normalized_name) = normalize_texture_tag_name(name)?;
    let color = normalize_texture_tag_color(color)?;
    if minecraft_texture_tag_repo::find_by_normalized_name(state.reader_db(), &normalized_name)
        .await?
        .is_some()
    {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNameTaken,
            "texture library tag name already exists",
        ));
    }
    let tag = minecraft_texture_tag_repo::create(
        state.writer_db(),
        minecraft_texture_tag_repo::CreateMinecraftTextureTag {
            name: &name,
            normalized_name: &normalized_name,
            color: &color,
            sort_order: sort_order.unwrap_or(0),
        },
    )
    .await?;
    Ok(texture_tag_info(tag))
}

pub async fn update_texture_library_tag<S: DatabaseRuntimeState>(
    state: &S,
    tag_id: i64,
    name: Option<&str>,
    color: Option<&str>,
    sort_order: Option<i32>,
) -> Result<MinecraftTextureTagInfo> {
    let tag = minecraft_texture_tag_repo::find_by_id(state.reader_db(), tag_id)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryTagNotFound,
                format!("texture library tag '{tag_id}'"),
            )
        })?;
    let (name, normalized_name) = match name {
        Some(name) => {
            let (name, normalized_name) = normalize_texture_tag_name(name)?;
            if let Some(existing) = minecraft_texture_tag_repo::find_by_normalized_name(
                state.reader_db(),
                &normalized_name,
            )
            .await?
                && existing.id != tag.id
            {
                return Err(AsterError::validation_error_code(
                    crate::api::error_code::AsterErrorCode::TextureLibraryTagNameTaken,
                    "texture library tag name already exists",
                ));
            }
            (Some(name), Some(normalized_name))
        }
        None => (None, None),
    };
    let color = color.map(normalize_texture_tag_color).transpose()?;
    let tag = minecraft_texture_tag_repo::update(
        state.writer_db(),
        tag,
        minecraft_texture_tag_repo::UpdateMinecraftTextureTag {
            name,
            normalized_name,
            color,
            sort_order,
        },
    )
    .await?;
    Ok(texture_tag_info(tag))
}

pub async fn delete_texture_library_tag<S: DatabaseRuntimeState>(
    state: &S,
    tag_id: i64,
) -> Result<()> {
    if !minecraft_texture_tag_repo::delete(state.writer_db(), tag_id).await? {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNotFound,
            format!("texture library tag '{tag_id}'"),
        ));
    }
    Ok(())
}

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
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureUpload).await?;
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

pub async fn replace_wardrobe_texture_tags<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
    tag_ids: &[i64],
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureUpload).await?;
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

pub async fn submit_texture_library_review<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureLibraryInteract)
        .await?;
    let texture = user_wardrobe_texture(state, user_id, texture_id).await?;
    if texture.visibility != MinecraftTextureVisibility::Public {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotPublic,
            "texture must be public before it can be submitted to the library",
        ));
    }

    let now = chrono::Utc::now();
    let policy = crate::config::texture_library::RuntimeTextureLibraryPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let next_status = if policy.review_required {
        MinecraftTextureLibraryStatus::PendingReview
    } else {
        MinecraftTextureLibraryStatus::Published
    };
    let updated = minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: next_status,
            library_submitted_at: Some(Some(now)),
            library_reviewed_at: Some(if policy.review_required {
                None
            } else {
                Some(now)
            }),
            library_reviewer_user_id: Some(None),
            library_review_note: Some(None),
        },
    )
    .await?;
    wardrobe_texture_with_current_tags(state, &updated).await
}

pub async fn withdraw_texture_library_submission<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureLibraryInteract)
        .await?;
    let texture = user_wardrobe_texture(state, user_id, texture_id).await?;
    let updated = minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: MinecraftTextureLibraryStatus::Private,
            library_submitted_at: Some(None),
            library_reviewed_at: Some(None),
            library_reviewer_user_id: Some(None),
            library_review_note: Some(None),
        },
    )
    .await?;
    wardrobe_texture_with_current_tags(state, &updated).await
}

pub async fn list_admin_texture_library_textures_cursor<S>(
    state: &S,
    limit: u64,
    after: Option<(chrono::DateTime<chrono::Utc>, i64)>,
    filter: minecraft_texture_repo::AdminTextureLibraryListFilter,
) -> Result<CursorPage<PublicTextureLibraryTextureMetadata, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let limit = limit.clamp(1, 100);
    let slice = minecraft_texture_repo::list_admin_library_textures_cursor(
        state.reader_db(),
        limit,
        after,
        filter,
    )
    .await?;
    let next_cursor = texture_next_cursor(&slice.items, slice.has_more);
    let textures = admin_texture_library_metadata_by_texture_ids(state, &slice.items).await?;
    Ok(CursorPage::new(textures, slice.total, limit, next_cursor))
}

fn texture_next_cursor(
    textures: &[minecraft_texture::Model],
    has_more: bool,
) -> Option<DateTimeIdCursor> {
    if !has_more {
        return None;
    }
    textures.last().map(|texture| DateTimeIdCursor {
        value: texture.updated_at,
        id: texture.id,
    })
}

pub async fn get_admin_texture_library_texture<S>(
    state: &S,
    texture_id: i64,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let texture = admin_wardrobe_texture(state, texture_id).await?;
    admin_texture_library_metadata(state, &texture).await
}

pub async fn approve_texture_library_texture<S>(
    state: &S,
    reviewer_user_id: i64,
    texture_id: i64,
    review_note: Option<String>,
    tag_ids: Option<&[i64]>,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let texture = admin_wardrobe_texture(state, texture_id).await?;
    if texture.library_status != MinecraftTextureLibraryStatus::PendingReview {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotPending,
            "only pending texture library submissions can be approved",
        ));
    }
    if texture.visibility != MinecraftTextureVisibility::Public {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotPublic,
            "texture must be public before it can be published",
        ));
    }
    if let Some(tag_ids) = tag_ids {
        replace_library_texture_tags_for_admin(state, texture.id, tag_ids).await?;
    }
    let updated = minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: MinecraftTextureLibraryStatus::Published,
            library_submitted_at: None,
            library_reviewed_at: Some(Some(chrono::Utc::now())),
            library_reviewer_user_id: Some(Some(reviewer_user_id)),
            library_review_note: Some(normalize_review_note_optional(review_note)?),
        },
    )
    .await?;
    admin_texture_library_metadata(state, &updated).await
}

pub async fn reject_texture_library_texture<S>(
    state: &S,
    reviewer_user_id: i64,
    texture_id: i64,
    review_note: Option<String>,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let texture = admin_wardrobe_texture(state, texture_id).await?;
    if texture.library_status != MinecraftTextureLibraryStatus::PendingReview {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotPending,
            "only pending texture library submissions can be rejected",
        ));
    }
    let note = normalize_review_note_required(review_note)?;
    let updated = minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: MinecraftTextureLibraryStatus::Rejected,
            library_submitted_at: None,
            library_reviewed_at: Some(Some(chrono::Utc::now())),
            library_reviewer_user_id: Some(Some(reviewer_user_id)),
            library_review_note: Some(Some(note)),
        },
    )
    .await?;
    admin_texture_library_metadata(state, &updated).await
}

pub async fn unpublish_texture_library_texture<S>(
    state: &S,
    reviewer_user_id: i64,
    texture_id: i64,
    review_note: Option<String>,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let texture = admin_wardrobe_texture(state, texture_id).await?;
    if texture.library_status != MinecraftTextureLibraryStatus::Published {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotPublished,
            "only published texture library entries can be unpublished",
        ));
    }
    let note = normalize_review_note_optional(review_note)?;
    let now = chrono::Utc::now();
    let updated = minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: MinecraftTextureLibraryStatus::Private,
            library_submitted_at: Some(None),
            library_reviewed_at: Some(Some(now)),
            library_reviewer_user_id: Some(Some(reviewer_user_id)),
            library_review_note: Some(note.clone()),
        },
    )
    .await?;
    minecraft_texture_report_repo::handle_pending_for_texture(
        state.writer_db(),
        texture_id,
        minecraft_texture_report_repo::HandleTextureReport {
            status: MinecraftTextureReportStatus::Accepted,
            admin_note: note,
            handled_by_user_id: reviewer_user_id,
            handled_at: now,
        },
    )
    .await?;
    admin_texture_library_metadata(state, &updated).await
}

pub async fn delete_texture_library_texture<S>(
    state: &S,
    texture_id: i64,
) -> std::result::Result<DeletedTextureLibraryTexture, TextureError>
where
    S: CacheRuntimeState
        + DatabaseRuntimeState
        + ObjectStorageRuntimeState
        + RuntimeConfigRuntimeState,
{
    if texture_id <= 0 {
        return Err(TextureError::with_detail(
            TextureErrorKind::NotFound,
            format!("texture library texture #{texture_id}"),
        ));
    }
    tracing::debug!(texture_id, "deleting admin texture library texture");
    let texture = minecraft_texture_repo::find_by_id(state.reader_db(), texture_id)
        .await
        .map_err(TextureError::from)?
        .filter(|texture| texture.is_wardrobe_item)
        .ok_or_else(|| {
            TextureError::with_detail(
                TextureErrorKind::NotFound,
                format!("texture library texture #{texture_id}"),
            )
        })?;
    let metadata = admin_texture_library_metadata(state, &texture)
        .await
        .map_err(TextureError::from)?;
    let deleted_bindings =
        minecraft_profile_texture_repo::delete_by_texture_id(state.writer_db(), texture_id)
            .await
            .map_err(TextureError::from)?;
    let Some(deleted_texture) =
        minecraft_texture_repo::delete_wardrobe_by_id(state.writer_db(), texture_id)
            .await
            .map_err(TextureError::from)?
    else {
        return Err(TextureError::with_detail(
            TextureErrorKind::NotFound,
            format!("texture library texture #{texture_id}"),
        ));
    };

    cleanup_texture_blob_if_unreferenced(
        state,
        &deleted_texture.storage_key,
        "admin texture library texture delete",
    )
    .await;
    invalidate_profile_texture_caches_for_bindings(state, &deleted_bindings).await;
    invalidate_texture_asset_caches(state, &deleted_texture).await;
    tracing::debug!(
        texture_id = deleted_texture.id,
        hash = %deleted_texture.hash,
        deleted_profile_binding_count = deleted_bindings.len(),
        "deleted admin texture library texture"
    );
    Ok(DeletedTextureLibraryTexture {
        texture: metadata,
        deleted_profile_binding_count: crate::utils::numbers::usize_to_u64(
            deleted_bindings.len(),
            "deleted profile texture binding count",
        )
        .map_err(TextureError::from)?,
    })
}

pub async fn create_texture_library_report<S>(
    state: &S,
    reporter_user_id: i64,
    texture_id: i64,
    reason: MinecraftTextureReportReason,
    message: Option<String>,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    ban_service::ensure_user_not_banned(
        state,
        reporter_user_id,
        UserBanScope::TextureLibraryInteract,
    )
    .await?;
    let texture = public_reportable_texture(state, texture_id).await?;
    if texture.user_id == reporter_user_id {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureReportSelfReportNotAllowed,
            "users cannot report their own texture library entries",
        ));
    }
    if minecraft_texture_report_repo::find_pending_for_reporter_and_texture(
        state.reader_db(),
        reporter_user_id,
        texture_id,
    )
    .await?
    .is_some()
    {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureReportPendingExists,
            "a pending report already exists for this texture",
        ));
    }

    let message = normalize_report_message(message)?;
    let report = minecraft_texture_report_repo::create(
        state.writer_db(),
        minecraft_texture_report_repo::CreateTextureReport {
            texture_id,
            reporter_user_id,
            reason,
            message,
        },
    )
    .await?;
    texture_report_info(state, &report).await
}

pub async fn list_admin_texture_library_reports_cursor<S>(
    state: &S,
    limit: u64,
    filter: minecraft_texture_report_repo::AdminTextureReportListFilter,
    cursor: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorPage<TextureReportInfo, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let page = minecraft_texture_report_repo::list_cursor(state.reader_db(), limit, filter, cursor)
        .await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|report| DateTimeIdCursor {
            value: report.created_at,
            id: report.id,
        })
    } else {
        None
    };
    let items = texture_report_infos_by_reports(state, &page.items).await?;
    Ok(CursorPage::new(
        items,
        page.total,
        limit.clamp(1, 100),
        next_cursor,
    ))
}

pub async fn get_admin_texture_library_report<S>(
    state: &S,
    report_id: i64,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let report = admin_texture_report(state, report_id).await?;
    texture_report_info(state, &report).await
}

pub async fn accept_texture_library_report<S>(
    state: &S,
    handler_user_id: i64,
    report_id: i64,
    admin_note: Option<String>,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let report = pending_admin_texture_report(state, report_id).await?;
    let texture = admin_wardrobe_texture(state, report.texture_id).await?;
    let note = normalize_admin_report_note(admin_note)?;
    let now = chrono::Utc::now();
    minecraft_texture_repo::update_library_review(
        state.writer_db(),
        texture,
        minecraft_texture_repo::UpdateTextureLibraryReview {
            library_status: MinecraftTextureLibraryStatus::Private,
            library_submitted_at: Some(None),
            library_reviewed_at: Some(Some(now)),
            library_reviewer_user_id: Some(Some(handler_user_id)),
            library_review_note: Some(note.clone()),
        },
    )
    .await?;
    let report = minecraft_texture_report_repo::handle(
        state.writer_db(),
        report,
        minecraft_texture_report_repo::HandleTextureReport {
            status: MinecraftTextureReportStatus::Accepted,
            admin_note: note,
            handled_by_user_id: handler_user_id,
            handled_at: now,
        },
    )
    .await?;
    texture_report_info(state, &report).await
}

pub async fn reject_texture_library_report<S>(
    state: &S,
    handler_user_id: i64,
    report_id: i64,
    admin_note: Option<String>,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let report = pending_admin_texture_report(state, report_id).await?;
    let report = minecraft_texture_report_repo::handle(
        state.writer_db(),
        report,
        minecraft_texture_report_repo::HandleTextureReport {
            status: MinecraftTextureReportStatus::Rejected,
            admin_note: normalize_admin_report_note(admin_note)?,
            handled_by_user_id: handler_user_id,
            handled_at: chrono::Utc::now(),
        },
    )
    .await?;
    texture_report_info(state, &report).await
}

async fn user_wardrobe_texture<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
) -> Result<minecraft_texture::Model>
where
    S: DatabaseRuntimeState,
{
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
            "invalid wardrobe texture id",
        ));
    }
    minecraft_texture_repo::find_by_id_for_user(state.reader_db(), texture_id, user_id)
        .await?
        .filter(|texture| texture.is_wardrobe_item)
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::WardrobeTextureNotFound,
                format!("wardrobe texture '{texture_id}'"),
            )
        })
}

async fn admin_wardrobe_texture<S>(state: &S, texture_id: i64) -> Result<minecraft_texture::Model>
where
    S: DatabaseRuntimeState,
{
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
            "invalid texture library texture id",
        ));
    }
    minecraft_texture_repo::find_by_id(state.reader_db(), texture_id)
        .await?
        .filter(|texture| texture.is_wardrobe_item)
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
                format!("texture library texture '{texture_id}'"),
            )
        })
}

async fn public_reportable_texture<S>(
    state: &S,
    texture_id: i64,
) -> Result<minecraft_texture::Model>
where
    S: DatabaseRuntimeState,
{
    if texture_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTextureNotFound,
            "invalid texture library texture id",
        ));
    }
    minecraft_texture_repo::find_public_wardrobe_by_id(state.reader_db(), texture_id)
        .await?
        .ok_or_else(|| {
            AsterError::validation_error_code(
                crate::api::error_code::AsterErrorCode::TextureReportTextureNotReportable,
                "only published public texture library entries can be reported",
            )
        })
}

async fn admin_texture_report<S>(
    state: &S,
    report_id: i64,
) -> Result<minecraft_texture_report::Model>
where
    S: DatabaseRuntimeState,
{
    if report_id <= 0 {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureReportNotFound,
            "invalid texture report id",
        ));
    }
    minecraft_texture_report_repo::find_by_id(state.reader_db(), report_id)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureReportNotFound,
                format!("texture report '{report_id}'"),
            )
        })
}

async fn pending_admin_texture_report<S>(
    state: &S,
    report_id: i64,
) -> Result<minecraft_texture_report::Model>
where
    S: DatabaseRuntimeState,
{
    let report = admin_texture_report(state, report_id).await?;
    if report.status != MinecraftTextureReportStatus::Pending {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureReportNotPending,
            "only pending texture reports can be handled",
        ));
    }
    Ok(report)
}

async fn wardrobe_texture_with_current_tags<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let tags = minecraft_texture_tag_repo::list_for_texture(state.reader_db(), texture.id).await?;
    Ok(wardrobe_texture_metadata_with_tags(state, texture, tags))
}

async fn replace_library_texture_tags_for_admin<S>(
    state: &S,
    texture_id: i64,
    tag_ids: &[i64],
) -> Result<()>
where
    S: DatabaseRuntimeState,
{
    let unique_tag_ids = normalize_texture_tag_ids(tag_ids)?;
    let tags = minecraft_texture_tag_repo::find_by_ids(state.reader_db(), &unique_tag_ids).await?;
    if tags.len() != unique_tag_ids.len() {
        return Err(AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNotFound,
            "one or more texture library tags were not found",
        ));
    }
    minecraft_texture_tag_repo::replace_texture_tags(state.writer_db(), texture_id, &unique_tag_ids)
        .await
}

async fn texture_report_info<S>(
    state: &S,
    report: &minecraft_texture_report::Model,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let infos = texture_report_infos_by_reports(state, std::slice::from_ref(report)).await?;
    infos.into_iter().next().ok_or_else(|| {
        AsterError::record_not_found_code(
            crate::api::error_code::AsterErrorCode::TextureReportNotFound,
            format!("texture report '{}'", report.id),
        )
    })
}

async fn texture_report_infos_by_reports<S>(
    state: &S,
    reports: &[minecraft_texture_report::Model],
) -> Result<Vec<TextureReportInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    use std::collections::HashMap;

    if reports.is_empty() {
        return Ok(Vec::new());
    }

    let texture_ids = reports
        .iter()
        .map(|report| report.texture_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut user_ids = BTreeSet::new();
    for report in reports {
        user_ids.insert(report.reporter_user_id);
        if let Some(handler_user_id) = report.handled_by_user_id {
            user_ids.insert(handler_user_id);
        }
    }
    let textures = minecraft_texture_repo::find_by_ids(state.reader_db(), &texture_ids).await?;
    let users =
        user_repo::find_by_ids(state.reader_db(), &user_ids.into_iter().collect::<Vec<_>>())
            .await?;

    let textures_by_id = textures
        .into_iter()
        .map(|texture| (texture.id, texture))
        .collect::<HashMap<_, _>>();
    let users_by_id = users
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();

    let existing_textures = reports
        .iter()
        .filter_map(|report| textures_by_id.get(&report.texture_id).cloned())
        .collect::<Vec<_>>();
    let texture_metadata =
        admin_texture_library_metadata_by_texture_ids(state, &existing_textures).await?;
    let texture_metadata_by_id = texture_metadata
        .into_iter()
        .map(|texture| (texture.id, texture))
        .collect::<HashMap<_, _>>();

    Ok(reports
        .iter()
        .map(|report| TextureReportInfo {
            id: report.id,
            texture_id: report.texture_id,
            reason: report.reason,
            message: report.message.clone(),
            status: report.status,
            admin_note: report.admin_note.clone(),
            texture: texture_metadata_by_id.get(&report.texture_id).cloned(),
            reporter: users_by_id
                .get(&report.reporter_user_id)
                .map(texture_report_user_info),
            handler: report
                .handled_by_user_id
                .and_then(|user_id| users_by_id.get(&user_id))
                .map(texture_report_user_info),
            handled_at: report.handled_at,
            created_at: report.created_at,
            updated_at: report.updated_at,
        })
        .collect())
}

fn texture_report_user_info(user: &user::Model) -> TextureReportUserInfo {
    TextureReportUserInfo {
        public_uuid: user.public_uuid.clone(),
        name: user.username.clone(),
    }
}

fn ensure_texture_library_enabled<S>(state: &S) -> Result<()>
where
    S: RuntimeConfigRuntimeState,
{
    let policy = crate::config::texture_library::RuntimeTextureLibraryPolicy::from_runtime_config(
        state.runtime_config(),
    );
    if !policy.enabled {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryDisabled,
            "public texture library is disabled",
        ));
    }
    Ok(())
}

fn normalize_review_note_optional(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => normalize_review_note(&value),
        None => Ok(None),
    }
}

fn normalize_review_note_required(value: Option<String>) -> Result<String> {
    normalize_review_note_optional(value)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AsterError::validation_error_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryReviewNoteInvalid,
                "review note is required when rejecting a texture library submission",
            )
        })
}

fn normalize_review_note(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 512 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryReviewNoteInvalid,
            "review note must not exceed 512 characters",
        ));
    }
    Ok(Some(value.to_string()))
}

fn normalize_report_message(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > 1000 {
                return Err(AsterError::validation_error_code(
                    crate::api::error_code::AsterErrorCode::TextureReportMessageInvalid,
                    "report message must not exceed 1000 characters",
                ));
            }
            Ok(Some(value.to_string()))
        }
        None => Ok(None),
    }
}

fn normalize_admin_report_note(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > 512 {
                return Err(AsterError::validation_error_code(
                    crate::api::error_code::AsterErrorCode::TextureReportMessageInvalid,
                    "admin note must not exceed 512 characters",
                ));
            }
            Ok(Some(value.to_string()))
        }
        None => Ok(None),
    }
}

fn normalize_texture_display_name_patch(value: NullablePatch<String>) -> Result<Option<String>> {
    match value {
        NullablePatch::Absent => Ok(None),
        NullablePatch::Null => Ok(None),
        NullablePatch::Value(value) => normalize_texture_display_name(&value),
    }
}

fn normalize_texture_display_name(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > 96 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNameInvalid,
            "texture display name must not exceed 96 characters",
        ));
    }
    Ok(Some(value.to_string()))
}

fn normalize_texture_tag_ids(tag_ids: &[i64]) -> Result<Vec<i64>> {
    let mut unique = BTreeSet::new();
    for tag_id in tag_ids {
        if *tag_id <= 0 {
            return Err(AsterError::record_not_found_code(
                crate::api::error_code::AsterErrorCode::TextureLibraryTagNotFound,
                "invalid texture library tag id",
            ));
        }
        unique.insert(*tag_id);
    }
    Ok(unique.into_iter().collect())
}

fn normalize_texture_tag_name(value: &str) -> Result<(String, String)> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNameInvalid,
            "texture library tag name must be 1-64 characters",
        ));
    }
    Ok((name.to_string(), name.to_lowercase()))
}

fn normalize_texture_tag_color(value: &str) -> Result<String> {
    let color = value.trim();
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    if !valid {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagColorInvalid,
            "texture library tag color must be a #RRGGBB value",
        ));
    }
    Ok(color.to_ascii_lowercase())
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

pub async fn authenticate_texture_access<S>(
    state: &S,
    access_token: &str,
    profile_uuid: &str,
) -> std::result::Result<(yggdrasil_token::Model, minecraft_profile::Model), TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
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
    S: CacheRuntimeState
        + DatabaseRuntimeState
        + RuntimeConfigRuntimeState
        + ObjectStorageRuntimeState,
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
    invalidate_profile_texture_caches(state, profile.id).await;
    invalidate_texture_asset_caches(state, &texture.texture).await;
    if let Some(previous) = previous.as_ref() {
        invalidate_texture_asset_caches(state, &previous.texture).await;
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
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + ObjectStorageRuntimeState,
{
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureUpload)
        .await
        .map_err(TextureError::from)?;
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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
            invalidate_profile_texture_caches(state, binding.binding.profile_id).await;
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
        invalidate_profile_texture_caches(state, binding.binding.profile_id).await;
        invalidate_texture_asset_caches(state, &existing).await;
        invalidate_texture_asset_caches(state, &binding.texture).await;
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
            invalidate_texture_asset_caches(state, &deleted).await;
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
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + ObjectStorageRuntimeState,
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
    let storage_key = object_storage_key(&processing.hash);
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
        .object_storage()
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
            display_name: None,
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
    S: DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureLibraryInteract)
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

pub async fn delete_wardrobe_texture<S>(
    state: &S,
    user_id: i64,
    wardrobe_texture_id: i64,
) -> std::result::Result<minecraft_texture::Model, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
{
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureUpload)
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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

pub async fn delete_all_wardrobe_textures_for_user<S>(
    state: &S,
    user_id: i64,
) -> std::result::Result<Vec<minecraft_texture::Model>, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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

pub async fn bind_wardrobe_texture_to_profile<S>(
    state: &S,
    user_id: i64,
    profile: &minecraft_profile::Model,
    wardrobe_texture_id: i64,
    texture_type: MinecraftTextureType,
) -> std::result::Result<StoredTexture, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
{
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::TextureUpload)
        .await
        .map_err(TextureError::from)?;
    ban_service::ensure_user_not_banned(state, user_id, UserBanScope::MinecraftProfileManage)
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

pub async fn delete_texture<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<Option<minecraft_profile_texture_repo::ProfileTexture>, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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
    S: CacheRuntimeState + DatabaseRuntimeState + ObjectStorageRuntimeState,
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

fn object_storage_key(hash: &str) -> String {
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
