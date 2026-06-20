//! Yggdrasil authentication service.

mod auth;
mod cache;
mod error;
mod login;
mod metadata;
mod minecraft_services;
mod profile;
mod properties;
mod session;
mod token;

pub use auth::{authenticate, invalidate, refresh, signout, validate};
pub use error::{YggdrasilError, YggdrasilErrorKind};
pub use metadata::metadata;
pub use minecraft_services::{
    minecraft_services_player_attributes, minecraft_services_privacy_blocklist,
    minecraft_services_privileges, profile_key_certificate,
};
pub use profile::{
    DeleteMinecraftProfileResult, MinecraftProfileInfo, RenameMinecraftProfileResult,
    create_profile, delete_profile_for_user, profile_info, profile_summary, rename_profile,
    rename_profile_for_user, validate_profile_name,
};
pub(crate) use properties::invalidate_profile_properties_cache;
pub(crate) use session::invalidate_session_forward_server_cache;
pub use session::{forwarded_texture_url, has_joined, join};
pub use token::{active_token_for_protocol, cleanup_expired_or_revoked_tokens};
pub(crate) use token::{invalidate_token_cache_for_user, invalidate_token_cache_hashes};

use crate::api::dto::yggdrasil::YggdrasilProfile;
use crate::db::repository::minecraft_profile_repo;
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState};

pub async fn profiles_by_names<S>(
    state: &S,
    names: &[String],
) -> std::result::Result<Vec<YggdrasilProfile>, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    tracing::debug!(
        requested_count = names.len(),
        "resolving yggdrasil profiles by names"
    );
    if names.len() > 100 {
        tracing::debug!(
            requested_count = names.len(),
            "yggdrasil profiles by names rejected because request exceeded limit"
        );
        return Err(YggdrasilError::new(
            YggdrasilErrorKind::TooManyProfilesRequested,
        ));
    }

    let mut summaries = Vec::new();
    let mut missed_names = Vec::new();
    for name in names {
        if let Some(profile) = cache::get_profile_name_summary(state, name).await {
            tracing::debug!(
                profile_name = name,
                "yggdrasil profile name summary cache hit"
            );
            summaries.push(profile);
        } else {
            missed_names.push(name.clone());
        }
    }

    if missed_names.is_empty() {
        tracing::debug!(
            requested_count = names.len(),
            matched_count = summaries.len(),
            "resolved yggdrasil profiles by names from cache"
        );
        return Ok(summaries);
    }

    let profiles = minecraft_profile_repo::list_by_names(state.reader_db(), &missed_names)
        .await
        .map_err(YggdrasilError::from)?;
    for profile in profiles.iter().map(profile_summary) {
        cache::set_profile_name_summary(state, &profile).await;
        summaries.push(profile);
    }
    tracing::debug!(
        requested_count = names.len(),
        cache_miss_count = missed_names.len(),
        matched_count = profiles.len(),
        "resolved yggdrasil profiles by names"
    );
    Ok(summaries)
}

pub async fn profile_by_uuid<S>(
    state: &S,
    uuid: &str,
    unsigned: bool,
) -> std::result::Result<Option<YggdrasilProfile>, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        profile_uuid = uuid,
        unsigned,
        "resolving yggdrasil profile by uuid"
    );
    let Some(profile) = minecraft_profile_repo::find_by_uuid(state.reader_db(), uuid)
        .await
        .map_err(YggdrasilError::from)?
    else {
        tracing::debug!(profile_uuid = uuid, "yggdrasil profile by uuid not found");
        return Ok(None);
    };
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        signed = !unsigned,
        "yggdrasil profile by uuid found"
    );
    Ok(Some(
        properties::profile_with_properties(state, &profile, !unsigned).await?,
    ))
}

pub(crate) async fn invalidate_profile_name_summary_cache<S>(state: &S, name: &str)
where
    S: CacheRuntimeState,
{
    cache::invalidate_profile_name_summary(state, name).await;
}
