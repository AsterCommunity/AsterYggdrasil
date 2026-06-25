//! User profile metadata service.

use chrono::Utc;
use sea_orm::Set;

use crate::db::repository::{user_profile_repo, user_repo};
use crate::entities::{user, user_profile};
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};

use super::info::{AvatarAudience, UserProfileInfo, build_profile_info, resolve_gravatar_base_url};
use super::shared::default_profile_active_model;

fn normalize_display_name(value: &str) -> Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if aster_forge_utils::text::char_count(trimmed) > 64 {
        return Err(AsterError::validation_error(
            "display name must be 64 characters or fewer",
        ));
    }

    Ok(Some(trimmed.to_string()))
}

pub async fn get_profile_info<S>(
    state: &S,
    user: &user::Model,
    audience: AvatarAudience,
) -> Result<UserProfileInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let profile = user_profile_repo::find_by_user_id(state.reader_db(), user.id).await?;
    let gravatar_base_url = resolve_gravatar_base_url(state);
    tracing::debug!(
        user_id = user.id,
        audience = ?audience,
        has_profile = profile.is_some(),
        "loaded user profile info"
    );
    Ok(build_profile_info(
        user,
        profile.as_ref(),
        audience,
        &gravatar_base_url,
    ))
}

pub async fn update_profile<S>(
    state: &S,
    user_id: i64,
    display_name: Option<String>,
) -> Result<UserProfileInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        user_id,
        display_name_provided = display_name.is_some(),
        "updating user profile"
    );
    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    let existing = user_profile_repo::find_by_user_id(state.writer_db(), user_id).await?;
    let gravatar_base_url = resolve_gravatar_base_url(state);

    let Some(display_name) = display_name else {
        tracing::debug!(user_id, "profile update had no display name changes");
        return Ok(build_profile_info(
            &user,
            existing.as_ref(),
            AvatarAudience::SelfUser,
            &gravatar_base_url,
        ));
    };

    let normalized = normalize_display_name(&display_name)?;
    let now = Utc::now();
    tracing::debug!(
        user_id,
        existing_profile = existing.is_some(),
        display_name_set = normalized.is_some(),
        "normalized user profile update"
    );

    let saved = match existing {
        Some(current) => {
            if current.display_name == normalized {
                tracing::debug!(
                    user_id,
                    "profile update skipped because display name was unchanged"
                );
                current
            } else {
                let mut active: user_profile::ActiveModel = current.into();
                active.display_name = Set(normalized);
                active.updated_at = Set(now);
                let saved = user_profile_repo::update(state.writer_db(), active).await?;
                tracing::debug!(user_id, "updated existing user profile");
                saved
            }
        }
        None => {
            if normalized.is_none() {
                tracing::debug!(
                    user_id,
                    "profile update skipped empty display name without existing profile"
                );
                return Ok(build_profile_info(
                    &user,
                    None,
                    AvatarAudience::SelfUser,
                    &gravatar_base_url,
                ));
            }

            let mut active = default_profile_active_model(user_id, now);
            active.display_name = Set(normalized);
            let saved = user_profile_repo::create(state.writer_db(), active).await?;
            tracing::debug!(user_id, "created user profile");
            saved
        }
    };

    Ok(build_profile_info(
        &user,
        Some(&saved),
        AvatarAudience::SelfUser,
        &gravatar_base_url,
    ))
}
