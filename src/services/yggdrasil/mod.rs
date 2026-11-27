//! Yggdrasil 协议域：协议流程、会话转发上游基础设施和签名密钥管理。

pub mod session_flow;
pub mod session_forward;
pub mod signature;

pub use session_flow::{
    DeleteMinecraftProfileResult, MinecraftProfileInfo, RenameMinecraftProfileResult,
    YggdrasilError, YggdrasilErrorKind, active_token_for_protocol, authenticate,
    cleanup_expired_or_revoked_tokens, create_profile, delete_profile_for_user,
    forwarded_texture_url, has_joined, invalidate, join, metadata,
    minecraft_services_player_attributes, minecraft_services_privacy_blocklist,
    minecraft_services_privileges, profile_by_uuid, profile_info, profile_key_certificate,
    profile_summary, profiles_by_names, refresh, rename_profile, rename_profile_for_user, signout,
    validate, validate_profile_name,
};
