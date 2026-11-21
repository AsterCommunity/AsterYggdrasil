//! Avatar object storage key helpers.

use crate::runtime::ObjectStorageRuntimeState;
use aster_yggdrasil_model::entities::user_profile;
use aster_yggdrasil_model::types::user::AvatarSource;

use super::shared::{AVATAR_SIZE_LG, AVATAR_SIZE_SM, stored_avatar_prefix};

const AVATAR_OBJECT_ROOT: &str = "avatar";

pub(super) fn user_avatar_prefix(user_id: i64, version: i32) -> String {
    format!("{AVATAR_OBJECT_ROOT}/user/{user_id}/v{version}")
}

pub(super) fn user_avatar_variant_key(prefix: &str, size: u32) -> String {
    format!("{prefix}/{size}.webp")
}

fn expected_user_avatar_prefix(user_id: i64, version: i32) -> String {
    user_avatar_prefix(user_id, version)
}

pub(super) fn resolve_stored_avatar_prefix(profile: &user_profile::Model) -> Option<&str> {
    let stored_prefix = stored_avatar_prefix(Some(profile))?;
    let expected = expected_user_avatar_prefix(profile.user_id, profile.avatar_version);
    if stored_prefix == expected {
        Some(stored_prefix)
    } else {
        None
    }
}

pub(super) fn resolve_stored_avatar_variant_key(
    profile: &user_profile::Model,
    size: u32,
) -> Option<String> {
    resolve_stored_avatar_prefix(profile).map(|prefix| user_avatar_variant_key(prefix, size))
}

pub(super) async fn delete_avatar_variant_objects<S: ObjectStorageRuntimeState>(
    state: &S,
    prefix: &str,
) {
    for size in [AVATAR_SIZE_SM, AVATAR_SIZE_LG] {
        let key = user_avatar_variant_key(prefix, size);
        if let Err(error) = state.object_storage().delete(&key).await {
            tracing::warn!(key, "failed to delete avatar object: {error}");
        }
    }
}

pub(super) async fn delete_upload_objects<S: ObjectStorageRuntimeState>(
    state: &S,
    profile: &user_profile::Model,
) {
    if profile.avatar_source != AvatarSource::Upload {
        return;
    }

    let Some(prefix) = resolve_stored_avatar_prefix(profile) else {
        if stored_avatar_prefix(Some(profile)).is_some() {
            tracing::warn!(
                user_id = profile.user_id,
                avatar_version = profile.avatar_version,
                "skip avatar cleanup for invalid stored avatar key"
            );
        }
        return;
    };

    delete_avatar_variant_objects(state, prefix).await;
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn profile_with_key(user_id: i64, version: i32, avatar_key: &str) -> user_profile::Model {
        user_profile::Model {
            user_id,
            display_name: None,
            avatar_source: AvatarSource::Upload,
            avatar_key: Some(avatar_key.to_string()),
            avatar_version: version,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn resolve_stored_avatar_prefix_accepts_expected_object_key() {
        let profile = profile_with_key(42, 3, "avatar/user/42/v3");

        assert_eq!(
            resolve_stored_avatar_prefix(&profile),
            Some("avatar/user/42/v3")
        );
        assert_eq!(
            resolve_stored_avatar_variant_key(&profile, 512),
            Some("avatar/user/42/v3/512.webp".to_string())
        );
    }

    #[test]
    fn resolve_stored_avatar_prefix_rejects_legacy_or_wrong_keys() {
        let legacy = profile_with_key(42, 3, "user/42/v3");
        let wrong_user = profile_with_key(42, 3, "avatar/user/43/v3");
        let wrong_version = profile_with_key(42, 3, "avatar/user/42/v4");
        let absolute = profile_with_key(42, 3, "/avatar/user/42/v3");

        assert!(resolve_stored_avatar_prefix(&legacy).is_none());
        assert!(resolve_stored_avatar_prefix(&wrong_user).is_none());
        assert!(resolve_stored_avatar_prefix(&wrong_version).is_none());
        assert!(resolve_stored_avatar_prefix(&absolute).is_none());
    }
}
