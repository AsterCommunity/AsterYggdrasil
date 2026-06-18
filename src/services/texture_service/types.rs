use serde::Serialize;
use tokio_util::io::ReaderStream;

use crate::db::repository::minecraft_profile_texture_repo;
use crate::entities::{minecraft_profile, minecraft_texture};
use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureType,
    MinecraftTextureVisibility,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureMetadataSource {
    Bound,
    Default,
}

#[derive(Debug, Clone)]
pub struct StoredTexture {
    pub texture: minecraft_profile_texture_repo::ProfileTexture,
    pub profile: minecraft_profile::Model,
    pub wardrobe_texture: minecraft_texture::Model,
}

#[derive(Debug, Clone)]
pub struct StoredWardrobeTexture {
    pub texture: minecraft_texture::Model,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureMetadata {
    pub id: i64,
    pub profile_id: i64,
    pub profile_uuid: String,
    pub profile_name: String,
    pub hash: String,
    pub texture_type: MinecraftTextureType,
    pub texture_model: MinecraftTextureModel,
    pub visibility: MinecraftTextureVisibility,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub mime_type: String,
    pub url: String,
    pub preview_url: Option<String>,
    pub source: MinecraftTextureMetadataSource,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftWardrobeTextureMetadata {
    pub id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub hash: String,
    pub texture_type: MinecraftTextureType,
    pub texture_model: MinecraftTextureModel,
    pub visibility: MinecraftTextureVisibility,
    pub library_status: MinecraftTextureLibraryStatus,
    pub tags: Vec<MinecraftTextureTagInfo>,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub mime_type: String,
    pub url: String,
    pub preview_url: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureUploaderInfo {
    pub public_uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct PublicTextureLibraryTextureMetadata {
    pub id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub hash: String,
    pub texture_type: MinecraftTextureType,
    pub texture_model: MinecraftTextureModel,
    pub visibility: MinecraftTextureVisibility,
    pub library_status: MinecraftTextureLibraryStatus,
    pub tags: Vec<MinecraftTextureTagInfo>,
    pub uploader: Option<MinecraftTextureUploaderInfo>,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub mime_type: String,
    pub url: String,
    pub preview_url: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureTagInfo {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i32,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct TextureBlob {
    pub storage_key: String,
}

#[derive(Debug, Clone)]
pub struct DeletedMinecraftTexture {
    pub texture: minecraft_profile_texture_repo::ProfileTexture,
    pub profile: minecraft_profile::Model,
}

pub struct TextureDownload {
    pub stream: ReaderStream<Box<dyn tokio::io::AsyncRead + Unpin + Send>>,
    pub content_type: &'static str,
    pub cache_control: &'static str,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WardrobeRegistrationResult {
    pub scanned_bindings: u64,
    pub converted_textures: u64,
    pub rebound_bindings: u64,
    pub removed_duplicate_textures: u64,
}
