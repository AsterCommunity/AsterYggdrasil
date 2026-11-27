//! User profile and avatar service.

mod avatar;
mod avatar_image;
mod avatar_storage;
mod details;
mod info;
mod minecraft;
mod shared;

pub use avatar::{
    AVATAR_CACHE_CONTROL, AVATAR_CONTENT_TYPE, delete_uploaded_avatar_for_user, get_avatar_bytes,
    set_avatar_source, upload_avatar,
};
pub use details::{get_profile_info, update_profile};
pub use info::{
    AvatarAudience, AvatarInfo, UserProfileInfo, build_profile_info, get_profile_info_map,
    resolve_gravatar_base_url,
};
pub use minecraft::{
    MinecraftProfileFilters, count_profiles_for_users, find_any_profile, find_owned_profile,
    list_profiles, list_profiles_for_user, validate_profile_uuid,
};
