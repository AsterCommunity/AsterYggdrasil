//! User profile and avatar service.

mod avatar;
mod avatar_image;
mod avatar_storage;
mod info;
mod profile;
mod shared;

pub use avatar::{
    AVATAR_CACHE_CONTROL, AVATAR_CONTENT_TYPE, get_avatar_bytes, set_avatar_source, upload_avatar,
};
pub use info::{AvatarAudience, AvatarInfo, UserProfileInfo, get_profile_info_map};
pub use profile::{get_profile_info, update_profile};
