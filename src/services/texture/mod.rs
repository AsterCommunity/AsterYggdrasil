//! Minecraft texture validation, storage and lookup.

mod deletion;
mod library;
mod moderation;
mod reports;
mod shared;
mod wardrobe;

mod cache;
mod default_skin;
mod error;
mod maintenance;
mod preview;
mod processing;
mod query;
mod types;

#[cfg(test)]
mod tests;

pub use default_skin::{
    DefaultSkin as EmbeddedDefaultSkin, by_hash as embedded_default_skin_by_hash,
    for_profile_uuid as default_skin_for_profile_uuid,
};
pub use error::{TextureError, TextureErrorKind};
pub use maintenance::{
    OrphanTextureCleanupResult, TextureStoreConsistencyIssue, TextureStoreConsistencyIssueKind,
    TextureStoreConsistencyReport, check_texture_storage_consistency, cleanup_orphan_texture_blobs,
};
pub use preview::{
    TEXTURE_PREVIEW_CACHE_CONTROL, TexturePreviewBytes, current_texture_preview_url,
    texture_preview_by_hash,
};
pub use processing::{TextureProcessingResult, process_texture_file, sanitize_png_texture};
pub use query::{
    admin_texture_library_metadata, admin_texture_library_metadata_by_texture_ids,
    default_skin_metadata, download_texture, download_texture_blob,
    public_texture_library_metadata, public_texture_library_metadata_by_texture_ids,
    texture_blob_by_hash, texture_by_hash, texture_metadata, texture_metadata_for_profile,
    texture_tag_info, textures_for_profile, wardrobe_texture_metadata,
    wardrobe_texture_metadata_by_texture_ids, wardrobe_texture_metadata_with_tags,
};
pub use types::{
    DeletedMinecraftTexture, MinecraftTextureMetadata, MinecraftTextureMetadataSource,
    MinecraftTextureTagInfo, MinecraftTextureUploaderInfo, MinecraftWardrobeTextureMetadata,
    PublicTextureLibraryTextureMetadata, StoredTexture, StoredWardrobeTexture, TextureBlob,
    TextureDownload, TextureReportInfo, TextureReportUserInfo, WardrobeRegistrationResult,
};

pub(crate) use shared::{TEXTURE_CACHE_CONTROL, ensure_upload_allowed, is_valid_texture_hash};

// Re-export the moved public surface so external `crate::services::texture::…`
// paths are unchanged.
pub use deletion::{
    delete_all_textures_for_profile, delete_texture, delete_texture_for_profile,
    delete_texture_for_profile_unchecked, delete_textures_by_hash,
};
pub use library::{
    accept_texture_library_report, create_texture_library_report, get_admin_texture_library_report,
    list_admin_texture_library_reports_cursor,
};
pub use library::{approve_texture_library_texture, withdraw_texture_library_submission};
pub use library::{
    create_texture_library_tag, delete_texture_library_tag, delete_texture_library_texture,
    get_admin_texture_library_texture, list_admin_texture_library_textures_cursor,
    list_texture_library_tags, list_texture_library_tags_paginated, reject_texture_library_texture,
    submit_texture_library_review, unpublish_texture_library_texture, update_texture_library_tag,
};
pub use moderation::{
    authenticate_texture_access, register_bound_textures_in_wardrobe, store_texture,
    store_wardrobe_texture, write_multipart_texture_field_to_file,
};
pub use reports::reject_texture_library_report;
pub use shared::DeletedTextureLibraryTexture;
pub use shared::MinecraftTextureTagMutationResult;
pub use shared::{parse_skin_model, parse_texture_type};
pub use wardrobe::{
    bind_wardrobe_texture_to_profile, copy_public_texture_to_wardrobe,
    delete_all_wardrobe_textures_for_user, delete_wardrobe_texture,
    get_public_texture_library_texture, list_public_texture_library_cursor,
    list_wardrobe_textures_cursor, replace_wardrobe_texture_tags, update_wardrobe_texture_metadata,
};
