//! Shared texture-service helpers: parsing, cache invalidation, and storage guards.

use crate::db::repository::{minecraft_profile_repo, minecraft_profile_texture_repo};
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState};
use aster_yggdrasil_model::entities::{minecraft_profile, minecraft_texture};
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureModel, yggdrasil::MinecraftTextureType,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::*;

pub(crate) const PNG_CONTENT_TYPE: &str = "image/png";
pub(crate) const TEXTURE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Parses a texture type from a request path segment (`skin`/`cape`), mapping
/// unknown values to [`TextureErrorKind::InvalidTextureType`].
pub fn parse_texture_type(value: &str) -> std::result::Result<MinecraftTextureType, TextureError> {
    MinecraftTextureType::parse_path(value)
        .ok_or_else(|| TextureError::new(TextureErrorKind::InvalidTextureType))
}

/// Parses the optional skin model parameter; empty or `default` maps to the
/// classic model, `slim` to the slim model, anything else is rejected.
pub fn parse_skin_model(
    value: Option<&str>,
) -> std::result::Result<MinecraftTextureModel, TextureError> {
    match value.unwrap_or_default().trim() {
        "" | "default" => Ok(MinecraftTextureModel::Default),
        "slim" => Ok(MinecraftTextureModel::Slim),
        _ => Err(TextureError::with_detail(
            TextureErrorKind::InvalidDimensions,
            "Invalid skin model.",
        )),
    }
}

pub(super) async fn invalidate_profile_texture_caches<S>(state: &S, profile_id: i64)
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    crate::services::yggdrasil::session_flow::invalidate_profile_properties_cache(
        state, profile_id,
    )
    .await;
    if let Err(error) = minecraft_profile_repo::touch_by_id(state.writer_db(), profile_id).await {
        tracing::warn!(
            error = %error,
            profile_id,
            "failed to touch minecraft profile after texture cache invalidation"
        );
    }
}

pub(super) async fn invalidate_profile_texture_caches_for_bindings<S>(
    state: &S,
    bindings: &[minecraft_profile_texture_repo::ProfileTexture],
) where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    for profile_id in bindings
        .iter()
        .map(|binding| binding.binding.profile_id)
        .collect::<BTreeSet<_>>()
    {
        invalidate_profile_texture_caches(state, profile_id).await;
    }
}

pub(super) async fn invalidate_texture_asset_caches<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) where
    S: CacheRuntimeState,
{
    query::invalidate_texture_blob_lookup_cache(state, &texture.hash).await;
}

/// Response payload after creating, updating, or deleting a texture-library tag.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftTextureTagMutationResult {
    /// Final state of the affected tag.
    pub tag: MinecraftTextureTagInfo,
}

/// Result of deleting a texture from the public texture library.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct DeletedTextureLibraryTexture {
    /// Snapshot of the library texture as it looked right before deletion.
    pub texture: PublicTextureLibraryTextureMetadata,
    /// Number of profile texture bindings that referenced the deleted asset
    /// and were removed along with it.
    pub deleted_profile_binding_count: u64,
}

pub(crate) fn ensure_upload_allowed(
    profile: &minecraft_profile::Model,
    texture_type: MinecraftTextureType,
) -> std::result::Result<(), TextureError> {
    let allowed = profile
        .uploadable_textures
        .split(',')
        .map(str::trim)
        .any(|item| item == texture_type.as_str());
    if allowed {
        Ok(())
    } else {
        Err(TextureError::new(TextureErrorKind::UploadDisabled))
    }
}

pub(super) fn object_storage_key(hash: &str) -> String {
    let prefix = &hash[..2];
    format!("{prefix}/{hash}.png")
}

pub(super) fn temporary_processed_path(source_path: &Path) -> PathBuf {
    let mut path = source_path.to_path_buf();
    path.set_extension("processed.png");
    path
}

pub(crate) fn is_valid_texture_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
