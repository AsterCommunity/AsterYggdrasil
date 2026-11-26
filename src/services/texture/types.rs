use serde::Serialize;
use tokio_util::io::ReaderStream;

use crate::db::repository::minecraft_profile_texture_repo;
use crate::services::user::profile::AvatarInfo;
use aster_yggdrasil_model::entities::{minecraft_profile, minecraft_texture};
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureLibraryStatus, yggdrasil::MinecraftTextureModel,
    yggdrasil::MinecraftTextureReportReason, yggdrasil::MinecraftTextureReportStatus,
    yggdrasil::MinecraftTextureType, yggdrasil::MinecraftTextureVisibility,
};

/// Where a profile's texture metadata came from: a texture actually bound to
/// the profile, or the built-in default skin substituted for a missing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureMetadataSource {
    /// A texture binding stored on the profile itself.
    Bound,
    /// The embedded default skin standing in for an unbound slot.
    Default,
}

/// A profile texture binding joined with its owning profile and wardrobe asset.
#[derive(Debug, Clone)]
pub struct StoredTexture {
    /// The profile-to-texture binding row (which slot this texture fills).
    pub texture: minecraft_profile_texture_repo::ProfileTexture,
    /// The Minecraft profile the binding belongs to.
    pub profile: minecraft_profile::Model,
    /// The wardrobe texture asset the binding points at.
    pub wardrobe_texture: minecraft_texture::Model,
}

/// A wardrobe-owned texture asset without any profile binding attached.
#[derive(Debug, Clone)]
pub struct StoredWardrobeTexture {
    /// The stored texture asset row.
    pub texture: minecraft_texture::Model,
}

/// Serialized metadata for the texture currently bound to a profile slot.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureMetadata {
    /// Texture binding row id.
    pub id: i64,
    /// Wardrobe texture asset id the binding points at.
    pub texture_id: i64,
    /// Internal asset name assigned at upload time.
    pub name: String,
    /// Optional user-facing name; falls back to [`Self::name`] when unset.
    pub display_name: Option<String>,
    /// Owning Minecraft profile row id.
    pub profile_id: i64,
    /// Owning profile's UUID in the unhyphenated 32-char form Yggdrasil uses
    /// as the player identifier.
    pub profile_uuid: String,
    /// Profile name at the time this metadata was built.
    pub profile_name: String,
    /// SHA-256 hash of the texture PNG, lowercase hex (64 chars); doubles as
    /// the content address used in texture URLs and de-duplication.
    pub hash: String,
    /// Which body slot this texture fills (skin or cape).
    pub texture_type: MinecraftTextureType,
    /// Skin arm model (`default`/`slim`); irrelevant for capes.
    pub texture_model: MinecraftTextureModel,
    /// Whether the asset is private to the owner or shared in the wardrobe.
    pub visibility: MinecraftTextureVisibility,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Encoded PNG size in bytes.
    pub file_size: i64,
    /// MIME type; always `image/png` for accepted uploads.
    pub mime_type: String,
    /// Public URL the texture bytes can be fetched from.
    pub url: String,
    /// Rendered preview image URL, if a preview has been generated.
    pub preview_url: Option<String>,
    /// Whether this metadata describes a real binding or a default-skin
    /// stand-in.
    pub source: MinecraftTextureMetadataSource,
    /// When the binding was created (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the binding was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Metadata for a texture sitting in a user's wardrobe, including its public
/// library submission state and tags.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftWardrobeTextureMetadata {
    /// Wardrobe texture asset row id.
    pub id: i64,
    /// Internal asset name assigned at upload time.
    pub name: String,
    /// Optional user-facing name; falls back to [`Self::name`] when unset.
    pub display_name: Option<String>,
    /// SHA-256 hash of the texture PNG, lowercase hex (64 chars).
    pub hash: String,
    /// Which body slot this texture fills (skin or cape).
    pub texture_type: MinecraftTextureType,
    /// Skin arm model (`default`/`slim`); irrelevant for capes.
    pub texture_model: MinecraftTextureModel,
    /// Whether the asset is private to the owner or publicly visible.
    pub visibility: MinecraftTextureVisibility,
    /// Lifecycle state of the asset's submission to the public texture library.
    pub library_status: MinecraftTextureLibraryStatus,
    /// Reviewer note attached when the library submission was approved or
    /// rejected; `None` while unreviewed.
    pub library_review_note: Option<String>,
    /// When the asset was submitted to the library, if it was.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When a moderator decided on the library submission, if one has.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Library tags attached to this texture.
    pub tags: Vec<MinecraftTextureTagInfo>,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Encoded PNG size in bytes.
    pub file_size: i64,
    /// MIME type; always `image/png` for accepted uploads.
    pub mime_type: String,
    /// Public URL the texture bytes can be fetched from.
    pub url: String,
    /// Rendered preview image URL, if a preview has been generated.
    pub preview_url: Option<String>,
    /// When the asset was uploaded (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the asset was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Publicly displayable information about the user who uploaded a library
/// texture; omits internal identifiers.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureUploaderInfo {
    /// Uploader's user row id.
    pub id: i64,
    /// Uploader's account username.
    pub username: String,
    /// Random public UUID (unhyphenated 32-char hex) identifying the user in
    /// external contexts instead of the internal row id.
    pub public_uuid: String,
    /// Display name shown on the site.
    pub name: String,
    /// Avatar image metadata for the uploader.
    pub avatar: AvatarInfo,
}

/// Metadata for a texture listed in the public texture library.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct PublicTextureLibraryTextureMetadata {
    /// Wardrobe texture asset row id.
    pub id: i64,
    /// Internal asset name assigned at upload time.
    pub name: String,
    /// Optional user-facing name; falls back to [`Self::name`] when unset.
    pub display_name: Option<String>,
    /// SHA-256 hash of the texture PNG, lowercase hex (64 chars).
    pub hash: String,
    /// Which body slot this texture fills (skin or cape).
    pub texture_type: MinecraftTextureType,
    /// Skin arm model (`default`/`slim`); irrelevant for capes.
    pub texture_model: MinecraftTextureModel,
    /// Whether the asset is private to the owner or publicly visible.
    pub visibility: MinecraftTextureVisibility,
    /// Lifecycle state of the asset's submission to the public texture library.
    pub library_status: MinecraftTextureLibraryStatus,
    /// Reviewer note attached when the library submission was approved or
    /// rejected; `None` while unreviewed.
    pub library_review_note: Option<String>,
    /// When the asset was submitted to the library, if it was.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When a moderator decided on the library submission, if one has.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Library tags attached to this texture.
    pub tags: Vec<MinecraftTextureTagInfo>,
    /// Who uploaded the texture, when the uploader should be shown publicly.
    pub uploader: Option<MinecraftTextureUploaderInfo>,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Encoded PNG size in bytes.
    pub file_size: i64,
    /// MIME type; always `image/png` for accepted uploads.
    pub mime_type: String,
    /// Public URL the texture bytes can be fetched from.
    pub url: String,
    /// Rendered preview image URL, if a preview has been generated.
    pub preview_url: Option<String>,
    /// When the asset was uploaded (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the asset was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Minimal public identity of a user involved in a texture report (reporter
/// or handling admin).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct TextureReportUserInfo {
    /// Random public UUID (unhyphenated 32-char hex) identifying the user.
    pub public_uuid: String,
    /// Display name shown on the site.
    pub name: String,
}

/// A public-library texture report with its moderation state and participants.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct TextureReportInfo {
    /// Report row id.
    pub id: i64,
    /// Reported wardrobe texture asset id.
    pub texture_id: i64,
    /// Categorized reason the reporter selected.
    pub reason: MinecraftTextureReportReason,
    /// Free-form explanation supplied by the reporter, if any.
    pub message: Option<String>,
    /// Moderation lifecycle state (pending/handled).
    pub status: MinecraftTextureReportStatus,
    /// Note left by the handling admin; hidden from the reporter unless the
    /// API chooses to expose it.
    pub admin_note: Option<String>,
    /// Snapshot of the reported texture, if it still exists.
    pub texture: Option<PublicTextureLibraryTextureMetadata>,
    /// Who filed the report, when resolvable.
    pub reporter: Option<TextureReportUserInfo>,
    /// Admin who handled the report, once handled.
    pub handler: Option<TextureReportUserInfo>,
    /// When the report was handled (UTC), if handled.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub handled_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the report was filed (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the report was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A library tag that can be attached to public texture library entries.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureTagInfo {
    /// Tag row id.
    pub id: i64,
    /// Tag label shown to users.
    pub name: String,
    /// CSS color used when rendering the tag.
    pub color: String,
    /// Position of the tag in ordering; lower values sort first.
    pub sort_order: i32,
    /// When the tag was created (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the tag was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Reference to a texture blob in object storage, addressed by content hash.
#[derive(Debug, Clone)]
pub struct TextureBlob {
    /// Object-storage key of the blob (hash-prefix path, `.png` extension).
    pub storage_key: String,
}

/// A profile texture binding that was deleted, together with its owner.
#[derive(Debug, Clone)]
pub struct DeletedMinecraftTexture {
    /// The binding row as it looked right before deletion.
    pub texture: minecraft_profile_texture_repo::ProfileTexture,
    /// The profile the deleted binding belonged to.
    pub profile: minecraft_profile::Model,
}

/// A texture payload ready to be streamed to an HTTP client.
pub struct TextureDownload {
    /// Byte stream of the texture content.
    pub stream: ReaderStream<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
    /// Response `Content-Type` (always `image/png` for textures).
    pub content_type: &'static str,
    /// Response `Cache-Control`; textures are content-addressed and immutable.
    pub cache_control: &'static str,
    /// Exact body size in bytes, taken from stored metadata.
    pub size: u64,
}

/// Counters produced when migrating legacy per-profile texture rows into
/// wardrobe-owned textures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WardrobeRegistrationResult {
    /// Number of existing profile texture bindings inspected.
    pub scanned_bindings: u64,
    /// Bindings whose texture row was newly converted into a wardrobe texture.
    pub converted_textures: u64,
    /// Bindings re-pointed at the wardrobe texture after conversion.
    pub rebound_bindings: u64,
    /// Duplicate texture rows (same hash) that were merged away.
    pub removed_duplicate_textures: u64,
}
