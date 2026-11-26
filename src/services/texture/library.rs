//! Texture-library review, moderation flows, and admin texture lifecycle.

use super::moderation::{
    admin_texture_report, normalize_texture_tag_color, normalize_texture_tag_name,
};
use super::moderation::{
    admin_wardrobe_texture, ensure_texture_library_enabled, normalize_admin_report_note,
    normalize_report_message, normalize_review_note_optional, normalize_review_note_required,
    pending_admin_texture_report, public_reportable_texture,
    replace_library_texture_tags_for_admin, texture_report_info, texture_report_infos_by_reports,
    user_wardrobe_texture, wardrobe_texture_with_current_tags,
};
use super::shared::{
    invalidate_profile_texture_caches_for_bindings, invalidate_texture_asset_caches,
};
use crate::db::repository::{
    minecraft_profile_texture_repo, minecraft_texture_repo, minecraft_texture_report_repo,
    minecraft_texture_tag_repo,
};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState,
};
use crate::services::user;
use aster_forge_api::{CursorPage, DateTimeIdCursor, SortOrderNameIdCursor};
use aster_yggdrasil_model::entities::minecraft_texture;
use aster_yggdrasil_model::types::{
    user::UserBanScope, yggdrasil::MinecraftTextureLibraryStatus,
    yggdrasil::MinecraftTextureReportReason, yggdrasil::MinecraftTextureReportStatus,
    yggdrasil::MinecraftTextureVisibility,
};
use chrono::{DateTime, Utc};

use self::maintenance::cleanup_texture_blob_if_unreferenced;

use super::*;

/// Lists every texture-library tag in sort order.
pub async fn list_texture_library_tags<S: DatabaseRuntimeState>(
    state: &S,
) -> Result<Vec<MinecraftTextureTagInfo>> {
    let tags = minecraft_texture_tag_repo::list(state.reader_db()).await?;
    Ok(tags.into_iter().map(texture_tag_info).collect())
}

/// Lists texture-library tags as a cursor page ordered by sort order, name,
/// and id; `limit` is clamped to 1-100 and `keyword` filters by name.
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

/// Creates a texture-library tag; names are normalized (trimmed, 1-64 chars)
/// and must be unique ignoring case, colors must be `#RRGGBB`.
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

/// Partially updates a texture-library tag; only the supplied fields change,
/// and name uniqueness is enforced ignoring case against other tags.
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
            (Some(name.to_owned()), Some(normalized_name))
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

/// Deletes a texture-library tag by id; fails with not-found when the tag
/// does not exist.
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

/// Submits a user's public wardrobe texture to the public texture library.
/// Requires the library feature and a public-visibility texture; the entry
/// enters `PendingReview` when runtime policy requires review, otherwise it
/// is published immediately.
pub async fn submit_texture_library_review<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    user::ban::ensure_user_not_banned(state, user_id, UserBanScope::TextureLibraryInteract).await?;
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

/// Pulls a submitted texture back out of the public library, resetting its
/// library status to private and clearing submission/review timestamps.
pub async fn withdraw_texture_library_submission<S>(
    state: &S,
    user_id: i64,
    texture_id: i64,
) -> Result<MinecraftWardrobeTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    user::ban::ensure_user_not_banned(state, user_id, UserBanScope::TextureLibraryInteract).await?;
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

/// Admin cursor listing over library texture submissions, filtered by the
/// given status/filter set; `limit` is clamped to 1-100.
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

pub(super) fn texture_next_cursor(
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

/// Loads one wardrobe texture by id for admin review, with admin-audience
/// metadata.
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

/// Approves a pending library submission, publishing it and optionally
/// (re)setting its tags; requires the texture to be in `PendingReview`.
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

/// Rejects a pending library submission with a required review note; the
/// texture stays in the owner's wardrobe but is marked `Rejected`.
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

/// Takes a published library entry down: resets it to private, records the
/// reviewer note, and auto-accepts the texture's pending reports as handled.
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

/// Hard-deletes a wardrobe texture from the admin library view along with all
/// of its profile bindings and, when unreferenced, its stored blob. Returns a
/// pre-deletion metadata snapshot.
pub async fn delete_texture_library_texture<S>(
    state: &S,
    texture_id: i64,
) -> std::result::Result<DeletedTextureLibraryTexture, TextureError>
where
    S: CacheRuntimeState
        + DatabaseRuntimeState
        + TextureStorageRuntimeState
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
        deleted_profile_binding_count: aster_forge_utils::numbers::usize_to_u64(
            deleted_bindings.len(),
            "deleted profile texture binding count",
        )
        .map_err(AsterError::from)
        .map_err(TextureError::from)?,
    })
}

/// Files a report against a public library texture. Self-reports and a
/// second pending report for the same texture by the same reporter are
/// rejected.
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
    user::ban::ensure_user_not_banned(
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

/// Admin cursor listing over texture reports, filtered by the given filter
/// set; entries carry reporter, handler, and texture snapshots.
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

/// Loads a single texture report by id for the admin report view.
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

/// Upholds a pending report: unpublishes the reported texture (resetting it
/// to private with the admin note) and marks the report accepted.
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
