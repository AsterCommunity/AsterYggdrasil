//! User avatar service.

use actix_multipart::Multipart;
use chrono::Utc;
use sea_orm::Set;

use crate::api::error_code::AsterErrorCode;
use crate::config::avatar;
use crate::db::repository::{user_profile_repo, user_repo};
use crate::entities::user_profile;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::types::AvatarSource;

use super::avatar_image::{process_avatar_upload, read_avatar_upload};
use super::avatar_storage::{
    avatar_variant_file_path, cleanup_local_avatar_prefix, delete_upload_objects,
    resolve_stored_avatar_variant_path, user_avatar_dir, user_avatar_prefix,
};
use super::info::{AvatarAudience, UserProfileInfo, build_profile_info, resolve_gravatar_base_url};
use super::shared::{
    AVATAR_SIZE_LG, AVATAR_SIZE_SM, default_profile_active_model, stored_avatar_prefix,
};

pub const AVATAR_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
pub const AVATAR_CONTENT_TYPE: &str = "image/webp";

async fn write_local_avatar(path: &std::path::Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_aster_err_ctx("create avatar directory", AsterError::internal_error)?;
    }

    tracing::debug!(bytes = data.len(), "writing local avatar variant");
    tokio::fs::write(path, data)
        .await
        .map_aster_err_ctx("write avatar file", AsterError::internal_error)?;
    Ok(())
}

pub async fn upload_avatar<S>(
    state: &S,
    user_id: i64,
    payload: &mut Multipart,
) -> Result<UserProfileInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(user_id, "uploading user avatar");
    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    let existing = user_profile_repo::find_by_user_id(state.writer_db(), user_id).await?;
    let upload_data = read_avatar_upload(payload).await?;
    tracing::debug!(
        user_id,
        bytes = upload_data.bytes.len(),
        existing_profile = existing.is_some(),
        "read user avatar upload"
    );
    let processed_avatar = process_avatar_upload(upload_data.bytes).await?;
    let version = existing
        .as_ref()
        .map(|profile| profile.avatar_version.saturating_add(1))
        .unwrap_or(1);
    let avatar_root_dir = avatar::resolve_local_avatar_root_dir(state.runtime_config())?;
    let prefix_key = user_avatar_prefix(user_id, version);
    let prefix = user_avatar_dir(&avatar_root_dir, user_id, version);
    let small_path = avatar_variant_file_path(&prefix, AVATAR_SIZE_SM);
    let large_path = avatar_variant_file_path(&prefix, AVATAR_SIZE_LG);

    write_local_avatar(&small_path, &processed_avatar.small_bytes).await?;
    if let Err(error) = write_local_avatar(&large_path, &processed_avatar.large_bytes).await {
        cleanup_local_avatar_prefix(&prefix, &avatar_root_dir).await;
        return Err(error);
    }
    tracing::debug!(
        user_id,
        version,
        small_bytes = processed_avatar.small_bytes.len(),
        large_bytes = processed_avatar.large_bytes.len(),
        "wrote user avatar variants"
    );

    let now = Utc::now();
    let saved = match existing.clone() {
        Some(current) => {
            let mut active: user_profile::ActiveModel = current.into();
            active.avatar_source = Set(AvatarSource::Upload);
            active.avatar_key = Set(Some(prefix_key));
            active.avatar_version = Set(version);
            active.updated_at = Set(now);
            user_profile_repo::update(state.writer_db(), active).await
        }
        None => {
            let mut active = default_profile_active_model(user_id, now);
            active.avatar_source = Set(AvatarSource::Upload);
            active.avatar_key = Set(Some(prefix_key));
            active.avatar_version = Set(version);
            user_profile_repo::create(state.writer_db(), active).await
        }
    };

    let saved = match saved {
        Ok(model) => model,
        Err(error) => {
            cleanup_local_avatar_prefix(&prefix, &avatar_root_dir).await;
            return Err(error);
        }
    };

    if let Some(previous) = existing.as_ref() {
        delete_upload_objects(state, previous).await;
    }

    let gravatar_base_url = resolve_gravatar_base_url(state);
    tracing::debug!(
        user_id,
        version = saved.avatar_version,
        "user avatar upload completed"
    );
    Ok(build_profile_info(
        &user,
        Some(&saved),
        AvatarAudience::SelfUser,
        &gravatar_base_url,
    ))
}

pub async fn set_avatar_source<S>(
    state: &S,
    user_id: i64,
    source: AvatarSource,
) -> Result<UserProfileInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(user_id, source = ?source, "setting user avatar source");
    if source == AvatarSource::Upload {
        tracing::debug!(
            user_id,
            "avatar source update rejected because upload source requires upload endpoint"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AvatarSourceInvalid,
            "upload avatar source must use the upload endpoint",
        ));
    }

    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    let existing = user_profile_repo::find_by_user_id(state.writer_db(), user_id).await?;
    let gravatar_base_url = resolve_gravatar_base_url(state);

    if existing.is_none() && source == AvatarSource::None {
        tracing::debug!(
            user_id,
            "avatar source update skipped empty profile and none source"
        );
        return Ok(build_profile_info(
            &user,
            None,
            AvatarAudience::SelfUser,
            &gravatar_base_url,
        ));
    }

    let now = Utc::now();
    let saved = match existing.clone() {
        Some(current) => {
            let next_version = current.avatar_version.saturating_add(1);
            let mut active: user_profile::ActiveModel = current.into();
            active.avatar_source = Set(source);
            active.avatar_key = Set(None);
            active.avatar_version = Set(next_version);
            active.updated_at = Set(now);
            user_profile_repo::update(state.writer_db(), active).await?
        }
        None => {
            let mut active = default_profile_active_model(user_id, now);
            active.avatar_source = Set(source);
            user_profile_repo::create(state.writer_db(), active).await?
        }
    };

    if let Some(previous) = existing.as_ref() {
        delete_upload_objects(state, previous).await;
    }

    tracing::debug!(
        user_id,
        source = ?saved.avatar_source,
        version = saved.avatar_version,
        "user avatar source updated"
    );
    Ok(build_profile_info(
        &user,
        Some(&saved),
        AvatarAudience::SelfUser,
        &gravatar_base_url,
    ))
}

fn validate_avatar_size(size: u32) -> Result<u32> {
    match size {
        AVATAR_SIZE_SM | AVATAR_SIZE_LG => Ok(size),
        _ => Err(AsterError::validation_error_code(
            AsterErrorCode::AvatarSizeInvalid,
            "avatar size must be 512 or 1024",
        )),
    }
}

pub async fn get_avatar_bytes<S>(state: &S, user_id: i64, size: u32) -> Result<Vec<u8>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let size = validate_avatar_size(size)?;
    tracing::debug!(user_id, size, "loading user avatar bytes");
    user_repo::find_by_id(state.reader_db(), user_id).await?;
    let profile = user_profile_repo::find_by_user_id(state.reader_db(), user_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(
                user_id,
                size,
                "user avatar not found because profile is missing"
            );
            AsterError::record_not_found_code(
                AsterErrorCode::AvatarNotFound,
                format!("profile for user #{user_id}"),
            )
        })?;

    if profile.avatar_source != AvatarSource::Upload {
        tracing::debug!(
            user_id,
            size,
            source = ?profile.avatar_source,
            "user avatar not found because avatar source is not upload"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::AvatarNotFound,
            format!("user #{user_id} does not have an uploaded avatar"),
        ));
    }

    stored_avatar_prefix(Some(&profile)).ok_or_else(|| {
        AsterError::record_not_found_code(AsterErrorCode::AvatarNotFound, "avatar key missing")
    })?;
    let avatar_root_dir = avatar::resolve_local_avatar_root_dir(state.runtime_config())?;
    let path =
        resolve_stored_avatar_variant_path(&avatar_root_dir, &profile, size).ok_or_else(|| {
            tracing::warn!(
                user_id = profile.user_id,
                avatar_version = profile.avatar_version,
                "reject invalid stored avatar key"
            );
            AsterError::record_not_found_code(AsterErrorCode::AvatarNotFound, "avatar key invalid")
        })?;
    let bytes = tokio::fs::read(&path).await.map_aster_err_with(|| {
        AsterError::record_not_found_code(
            AsterErrorCode::AvatarNotFound,
            format!("avatar object {}", path.display()),
        )
    })?;
    tracing::debug!(
        user_id,
        size,
        bytes = bytes.len(),
        "loaded user avatar bytes"
    );
    Ok(bytes)
}
