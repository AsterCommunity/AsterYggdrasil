//! User profile presentation helpers.

use std::collections::HashMap;

use md5::{Digest, Md5};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::config::avatar;
use crate::db::repository::user_profile_repo;
use crate::entities::{user, user_profile};
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::types::AvatarSource;

use super::shared::{AVATAR_SIZE_LG, AVATAR_SIZE_SM, stored_avatar_prefix};

#[derive(Debug, Clone, Copy)]
pub enum AvatarAudience {
    SelfUser,
    AdminUser,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AvatarInfo {
    pub source: AvatarSource,
    pub url_512: Option<String>,
    pub url_1024: Option<String>,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UserProfileInfo {
    pub display_name: Option<String>,
    pub avatar: AvatarInfo,
}

pub fn resolve_gravatar_base_url(state: &impl RuntimeConfigRuntimeState) -> String {
    avatar::gravatar_base_url_or_default(state.runtime_config())
}

fn gravatar_hash(email: &str) -> String {
    let normalized = email.trim().to_lowercase();
    let mut hasher = Md5::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

fn gravatar_url(email: &str, size: u32, base_url: &str) -> String {
    let hash = gravatar_hash(email);
    let base = base_url.trim_end_matches('/');
    format!("{base}/{hash}?d=identicon&s={size}&r=g")
}

fn avatar_api_path(user_id: i64, version: i32, size: u32, audience: AvatarAudience) -> String {
    match audience {
        AvatarAudience::SelfUser => format!("/auth/profile/avatar/{size}?v={version}"),
        AvatarAudience::AdminUser => format!("/admin/avatars/users/{user_id}/{size}?v={version}"),
    }
}

fn build_avatar_info(
    user: &user::Model,
    profile: Option<&user_profile::Model>,
    audience: AvatarAudience,
    gravatar_base_url: &str,
) -> AvatarInfo {
    let source = profile
        .map(|profile| profile.avatar_source)
        .unwrap_or(AvatarSource::None);
    let version = profile.map(|profile| profile.avatar_version).unwrap_or(0);

    match source {
        AvatarSource::None => AvatarInfo {
            source,
            url_512: None,
            url_1024: None,
            version,
        },
        AvatarSource::Gravatar => AvatarInfo {
            source,
            url_512: Some(gravatar_url(&user.email, AVATAR_SIZE_SM, gravatar_base_url)),
            url_1024: Some(gravatar_url(&user.email, AVATAR_SIZE_LG, gravatar_base_url)),
            version,
        },
        AvatarSource::Upload => {
            let has_upload = stored_avatar_prefix(profile).is_some();

            AvatarInfo {
                source,
                url_512: has_upload
                    .then(|| avatar_api_path(user.id, version, AVATAR_SIZE_SM, audience)),
                url_1024: has_upload
                    .then(|| avatar_api_path(user.id, version, AVATAR_SIZE_LG, audience)),
                version,
            }
        }
    }
}

pub fn build_profile_info(
    user: &user::Model,
    profile: Option<&user_profile::Model>,
    audience: AvatarAudience,
    gravatar_base_url: &str,
) -> UserProfileInfo {
    UserProfileInfo {
        display_name: profile.and_then(|profile| profile.display_name.clone()),
        avatar: build_avatar_info(user, profile, audience, gravatar_base_url),
    }
}

pub async fn get_profile_info_map<S>(
    state: &S,
    users: &[user::Model],
    audience: AvatarAudience,
) -> Result<HashMap<i64, UserProfileInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let user_ids: Vec<i64> = users.iter().map(|user| user.id).collect();
    let profiles = user_profile_repo::find_by_user_ids(state.reader_db(), &user_ids).await?;
    let gravatar_base_url = resolve_gravatar_base_url(state);

    Ok(users
        .iter()
        .map(|user| {
            (
                user.id,
                build_profile_info(user, profiles.get(&user.id), audience, &gravatar_base_url),
            )
        })
        .collect())
}
