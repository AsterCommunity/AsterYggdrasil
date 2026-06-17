use tokio_util::io::ReaderStream;

use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_texture_repo, minecraft_texture_repo};
use crate::entities::{minecraft_profile, minecraft_texture};
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState};

use super::{
    MinecraftTextureMetadata, MinecraftTextureMetadataSource, MinecraftWardrobeTextureMetadata,
    TEXTURE_CACHE_CONTROL, TextureBlob, TextureDownload, default_skin, is_valid_texture_hash,
};

pub async fn texture_by_hash<S: DatabaseRuntimeState>(
    state: &S,
    hash: &str,
) -> Result<Option<minecraft_texture::Model>> {
    if !is_valid_texture_hash(hash) {
        tracing::debug!(hash, "texture lookup skipped invalid hash");
        return Ok(None);
    }
    let texture = minecraft_texture_repo::find_by_hash(state.reader_db(), hash).await?;
    tracing::debug!(hash, found = texture.is_some(), "looked up texture by hash");
    Ok(texture)
}

pub async fn texture_blob_by_hash<S: DatabaseRuntimeState>(
    state: &S,
    hash: &str,
) -> Result<Option<TextureBlob>> {
    if !is_valid_texture_hash(hash) {
        tracing::debug!(hash, "texture blob lookup skipped invalid hash");
        return Ok(None);
    }
    let blob = minecraft_texture_repo::find_by_hash(state.reader_db(), hash)
        .await?
        .map(|texture| TextureBlob {
            storage_key: texture.storage_key,
        });
    tracing::debug!(
        hash,
        found = blob.is_some(),
        "looked up texture blob by hash"
    );
    Ok(blob)
}

pub async fn download_texture<S: TextureStorageRuntimeState>(
    state: &S,
    texture: &minecraft_profile_texture_repo::ProfileTexture,
) -> Result<TextureDownload> {
    tracing::debug!(
        texture_id = texture.texture.id,
        profile_texture_id = texture.binding.id,
        "downloading profile texture"
    );
    download_texture_storage_key(state, &texture.texture.storage_key).await
}

pub async fn download_texture_blob<S: TextureStorageRuntimeState>(
    state: &S,
    texture: &TextureBlob,
) -> Result<TextureDownload> {
    tracing::debug!("downloading texture blob");
    download_texture_storage_key(state, &texture.storage_key).await
}

async fn download_texture_storage_key<S: TextureStorageRuntimeState>(
    state: &S,
    storage_key: &str,
) -> Result<TextureDownload> {
    let metadata = state.texture_storage().metadata(storage_key).await?;
    let stream = state.texture_storage().get_stream(storage_key).await?;
    tracing::debug!(
        content_type = %metadata.content_type,
        size = metadata.size,
        "opened texture storage stream"
    );
    Ok(TextureDownload {
        stream: ReaderStream::new(stream),
        content_type: metadata.content_type,
        cache_control: TEXTURE_CACHE_CONTROL,
        size: metadata.size,
    })
}

pub async fn textures_for_profile<S: DatabaseRuntimeState>(
    state: &S,
    profile_id: i64,
) -> Result<Vec<minecraft_profile_texture_repo::ProfileTexture>> {
    let textures =
        minecraft_profile_texture_repo::list_by_profile(state.reader_db(), profile_id).await?;
    tracing::debug!(
        profile_id,
        count = textures.len(),
        "listed textures for profile"
    );
    Ok(textures)
}

pub async fn texture_metadata_for_profile<S>(
    state: &S,
    profile: &minecraft_profile::Model,
) -> Result<Vec<MinecraftTextureMetadata>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let textures =
        minecraft_profile_texture_repo::list_by_profile(state.reader_db(), profile.id).await?;
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        count = textures.len(),
        "building texture metadata for profile"
    );
    let mut metadata = textures
        .iter()
        .map(|texture| texture_metadata(state, profile, texture))
        .collect::<Vec<_>>();
    if !metadata
        .iter()
        .any(|texture| texture.texture_type == crate::types::MinecraftTextureType::Skin)
    {
        metadata.push(default_skin_metadata(state, profile)?);
    }
    Ok(metadata)
}

pub fn texture_metadata<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    texture: &minecraft_profile_texture_repo::ProfileTexture,
) -> MinecraftTextureMetadata
where
    S: RuntimeConfigRuntimeState,
{
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    MinecraftTextureMetadata {
        id: texture.binding.id,
        profile_id: profile.id,
        profile_uuid: profile.uuid.clone(),
        profile_name: profile.name.clone(),
        hash: texture.texture.hash.clone(),
        texture_type: texture.binding.texture_type,
        texture_model: texture.texture.texture_model,
        visibility: texture.texture.visibility,
        width: texture.texture.width,
        height: texture.texture.height,
        file_size: texture.texture.file_size,
        mime_type: texture.texture.mime_type.clone(),
        url: crate::services::yggdrasil_signature::texture_base_url(&policy, &texture.texture.hash),
        source: MinecraftTextureMetadataSource::Bound,
        created_at: texture.binding.created_at,
        updated_at: texture.binding.updated_at,
    }
}

pub fn default_skin_metadata<S>(
    state: &S,
    profile: &minecraft_profile::Model,
) -> Result<MinecraftTextureMetadata>
where
    S: RuntimeConfigRuntimeState,
{
    let skin = default_skin::for_profile_uuid(&profile.uuid);
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let image = image::load_from_memory(skin.bytes).map_err(|error| {
        crate::errors::AsterError::internal_error(format!(
            "embedded default skin is not a valid PNG: {error}"
        ))
    })?;
    let width = crate::utils::numbers::u32_to_i32(image.width(), "default skin width")?;
    let height = crate::utils::numbers::u32_to_i32(image.height(), "default skin height")?;
    let file_size = crate::utils::numbers::usize_to_i64(skin.bytes.len(), "default skin size")?;
    Ok(MinecraftTextureMetadata {
        id: 0,
        profile_id: profile.id,
        profile_uuid: profile.uuid.clone(),
        profile_name: profile.name.clone(),
        hash: skin.hash.to_string(),
        texture_type: crate::types::MinecraftTextureType::Skin,
        texture_model: skin.model,
        visibility: crate::types::MinecraftTextureVisibility::Public,
        width,
        height,
        file_size,
        mime_type: "image/png".to_string(),
        url: crate::services::yggdrasil_signature::texture_base_url(&policy, skin.hash),
        source: MinecraftTextureMetadataSource::Default,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    })
}

pub fn wardrobe_texture_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> MinecraftWardrobeTextureMetadata
where
    S: RuntimeConfigRuntimeState,
{
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    MinecraftWardrobeTextureMetadata {
        id: texture.id,
        hash: texture.hash.clone(),
        texture_type: texture.texture_type,
        texture_model: texture.texture_model,
        visibility: texture.visibility,
        width: texture.width,
        height: texture.height,
        file_size: texture.file_size,
        mime_type: texture.mime_type.clone(),
        url: crate::services::yggdrasil_signature::texture_base_url(&policy, &texture.hash),
        created_at: texture.created_at,
        updated_at: texture.updated_at,
    }
}
