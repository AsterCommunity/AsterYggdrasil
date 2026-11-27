//! Cross-flow query and normalization helpers shared by library, report, and
//! wardrobe moderation flows.

use super::shared::{object_storage_key, temporary_processed_path};
use aster_forge_utils::text::char_count;

use super::shared::{
    PNG_CONTENT_TYPE, invalidate_profile_texture_caches, invalidate_texture_asset_caches,
};
use crate::db::repository::{
    minecraft_profile_repo, minecraft_profile_texture_repo, minecraft_texture_repo,
    minecraft_texture_report_repo, minecraft_texture_tag_repo, user_repo,
};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState,
};
use crate::services::user as user_services;
use aster_forge_api::NullablePatch;
use aster_yggdrasil_model::entities::{
    minecraft_profile, minecraft_texture, minecraft_texture_report, user, yggdrasil_token,
};
use aster_yggdrasil_model::types::permission::capability;
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureModel, yggdrasil::MinecraftTextureReportStatus,
    yggdrasil::MinecraftTextureType, yggdrasil::MinecraftTextureVisibility,
};
use futures::StreamExt;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

use self::maintenance::cleanup_texture_blob_if_unreferenced;

use super::*;

pub(super) async fn user_wardrobe_texture<S>(
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

pub(super) async fn admin_wardrobe_texture<S>(
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

pub(super) async fn public_reportable_texture<S>(
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

pub(super) async fn admin_texture_report<S>(
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

pub(super) async fn pending_admin_texture_report<S>(
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

pub(super) async fn wardrobe_texture_with_current_tags<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let tags = minecraft_texture_tag_repo::list_for_texture(state.reader_db(), texture.id).await?;
    Ok(wardrobe_texture_metadata_with_tags(state, texture, tags))
}

pub(super) async fn replace_library_texture_tags_for_admin<S>(
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

pub(super) async fn texture_report_info<S>(
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

pub(super) async fn texture_report_infos_by_reports<S>(
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

pub(super) fn ensure_texture_library_enabled<S>(state: &S) -> Result<()>
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

pub(super) fn normalize_review_note_optional(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => normalize_review_note(&value),
        None => Ok(None),
    }
}

pub(super) fn normalize_review_note_required(value: Option<String>) -> Result<String> {
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
    if char_count(value) > 512 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryReviewNoteInvalid,
            "review note must not exceed 512 characters",
        ));
    }
    Ok(Some(value.to_string()))
}

pub(super) fn normalize_report_message(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if char_count(value) > 1000 {
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

pub(super) fn normalize_admin_report_note(value: Option<String>) -> Result<Option<String>> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if char_count(value) > 512 {
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

pub(super) fn normalize_texture_display_name_patch(
    value: NullablePatch<String>,
) -> Result<Option<String>> {
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
    if char_count(value) > 96 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::WardrobeTextureNameInvalid,
            "texture display name must not exceed 96 characters",
        ));
    }
    Ok(Some(value.to_string()))
}

pub(super) fn normalize_texture_tag_ids(tag_ids: &[i64]) -> Result<Vec<i64>> {
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

pub(super) fn normalize_texture_tag_name(value: &str) -> Result<(String, String)> {
    let name = value.trim();
    if name.is_empty() || char_count(name) > 64 {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::TextureLibraryTagNameInvalid,
            "texture library tag name must be 1-64 characters",
        ));
    }
    Ok((name.to_string(), name.to_lowercase()))
}

pub(super) fn normalize_texture_tag_color(value: &str) -> Result<String> {
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

/// Streams a multipart upload field into a temp file, enforcing
/// `max_upload_bytes` while writing so oversized payloads fail before hitting
/// the sanitizer.
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
        let chunk_len =
            aster_forge_utils::numbers::usize_to_u64(chunk.len(), "texture upload chunk")
                .map_err(AsterError::from)
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

/// Authenticates a Yggdrasil texture upload/delete request: the access token
/// must be active and have a selected profile, and that profile's UUID must
/// match the requested one. Returns the token and profile on success.
pub async fn authenticate_texture_access<S>(
    state: &S,
    access_token: &str,
    profile_uuid: &str,
) -> std::result::Result<(yggdrasil_token::Model, minecraft_profile::Model), TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    tracing::debug!(profile_uuid, "authenticating texture upload access");
    let token =
        crate::services::yggdrasil::session_flow::active_token_for_protocol(state, access_token)
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

/// Sanitizes, deduplicates, and binds an uploaded texture to the profile's
/// `texture_type` slot, replacing any previous binding and cleaning up the
/// orphaned asset. Checks ban status and the profile's upload allowlist first.
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
        + TextureStorageRuntimeState,
{
    user_services::ban::ensure_user_not_banned(state, profile.user_id, capability::texture::UPLOAD)
        .await
        .map_err(TextureError::from)?;
    user_services::ban::ensure_user_not_banned(state, profile.user_id, capability::profile::MANAGE)
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

/// Sanitizes and stores an uploaded texture as a wardrobe asset owned by
/// `user_id` without binding it to any profile, deduplicating by content
/// fingerprint when an identical asset already exists.
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
    user_services::ban::ensure_user_not_banned(state, user_id, capability::texture::UPLOAD)
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

/// Migrates legacy texture rows that predate the wardrobe model: converts
/// still-bound rows into wardrobe assets, re-points duplicates onto the
/// surviving asset, and removes duplicate rows. Returns per-outcome counts.
pub async fn register_bound_textures_in_wardrobe<S>(
    state: &S,
) -> std::result::Result<WardrobeRegistrationResult, TextureError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let bindings = minecraft_profile_texture_repo::list_all(state.reader_db())
        .await
        .map_err(TextureError::from)?;
    let scanned_bindings =
        aster_forge_utils::numbers::usize_to_u64(bindings.len(), "wardrobe registration scan")
            .map_err(AsterError::from)
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
        aster_forge_utils::fs::cleanup_temp_file(&processed_path).await;
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
    aster_forge_utils::fs::cleanup_temp_file(&processed_path).await;

    let file_size =
        aster_forge_utils::numbers::u64_to_i64(processing.file_size, "texture file size")
            .map_err(AsterError::from)
            .map_err(TextureError::from)?;
    let width = aster_forge_utils::numbers::u32_to_i32(processing.width, "texture width")
        .map_err(AsterError::from)
        .map_err(TextureError::from)?;
    let height = aster_forge_utils::numbers::u32_to_i32(processing.height, "texture height")
        .map_err(AsterError::from)
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

pub(super) async fn cleanup_texture_asset_if_unreferenced<S>(
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
