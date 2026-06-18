use std::collections::{BTreeSet, HashMap};

use tokio_util::io::ReaderStream;

use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{
    minecraft_profile_texture_repo, minecraft_texture_repo, minecraft_texture_tag_repo,
    user_profile_repo, user_repo,
};
use crate::entities::{
    minecraft_profile, minecraft_texture, minecraft_texture_tag, user, user_profile,
};
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, ObjectStorageRuntimeState, RuntimeConfigRuntimeState};

use super::{
    MinecraftTextureMetadata, MinecraftTextureMetadataSource, MinecraftTextureTagInfo,
    MinecraftTextureUploaderInfo, MinecraftWardrobeTextureMetadata,
    PublicTextureLibraryTextureMetadata, TEXTURE_CACHE_CONTROL, TextureBlob, TextureDownload,
    default_skin, is_valid_texture_hash,
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

pub async fn download_texture<S: ObjectStorageRuntimeState>(
    state: &S,
    texture: &minecraft_profile_texture_repo::ProfileTexture,
) -> Result<TextureDownload> {
    tracing::debug!(
        texture_id = texture.texture.id,
        profile_texture_id = texture.binding.id,
        "downloading profile texture"
    );
    download_object_storage_key(state, &texture.texture.storage_key).await
}

pub async fn download_texture_blob<S: ObjectStorageRuntimeState>(
    state: &S,
    texture: &TextureBlob,
) -> Result<TextureDownload> {
    tracing::debug!("downloading texture blob");
    download_object_storage_key(state, &texture.storage_key).await
}

async fn download_object_storage_key<S: ObjectStorageRuntimeState>(
    state: &S,
    storage_key: &str,
) -> Result<TextureDownload> {
    let metadata = state.object_storage().metadata(storage_key).await?;
    let stream = state.object_storage().get_stream(storage_key).await?;
    tracing::debug!(
        content_type = %metadata.content_type,
        size = metadata.size,
        "opened object storage stream"
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
        url: crate::services::yggdrasil_signature::texture_object_url(
            &policy,
            &texture.texture.hash,
            &texture.texture.storage_key,
        ),
        preview_url: super::current_texture_preview_url(
            state.runtime_config(),
            &texture.texture.hash,
            texture.binding.texture_type,
            texture.texture.texture_model,
        ),
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
        preview_url: super::current_texture_preview_url(
            state.runtime_config(),
            skin.hash,
            crate::types::MinecraftTextureType::Skin,
            skin.model,
        ),
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
    wardrobe_texture_metadata_with_tags(state, texture, Vec::new())
}

pub fn wardrobe_texture_metadata_with_tags<S>(
    state: &S,
    texture: &minecraft_texture::Model,
    tags: Vec<minecraft_texture_tag::Model>,
) -> MinecraftWardrobeTextureMetadata
where
    S: RuntimeConfigRuntimeState,
{
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    MinecraftWardrobeTextureMetadata {
        id: texture.id,
        name: texture_display_name(texture),
        display_name: texture.display_name.clone(),
        hash: texture.hash.clone(),
        texture_type: texture.texture_type,
        texture_model: texture.texture_model,
        visibility: texture.visibility,
        library_status: texture.library_status,
        tags: tags.into_iter().map(texture_tag_info).collect(),
        width: texture.width,
        height: texture.height,
        file_size: texture.file_size,
        mime_type: texture.mime_type.clone(),
        url: crate::services::yggdrasil_signature::texture_object_url(
            &policy,
            &texture.hash,
            &texture.storage_key,
        ),
        preview_url: super::current_texture_preview_url(
            state.runtime_config(),
            &texture.hash,
            texture.texture_type,
            texture.texture_model,
        ),
        created_at: texture.created_at,
        updated_at: texture.updated_at,
    }
}

fn texture_display_name(texture: &minecraft_texture::Model) -> String {
    texture
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| texture.hash.chars().take(16).collect())
}

pub async fn wardrobe_texture_metadata_by_texture_ids<S>(
    state: &S,
    textures: &[minecraft_texture::Model],
) -> Result<Vec<MinecraftWardrobeTextureMetadata>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let texture_ids = textures
        .iter()
        .map(|texture| texture.id)
        .collect::<Vec<_>>();
    let mut tags_by_texture =
        minecraft_texture_tag_repo::list_for_texture_ids(state.reader_db(), &texture_ids).await?;
    Ok(textures
        .iter()
        .map(|texture| {
            wardrobe_texture_metadata_with_tags(
                state,
                texture,
                tags_by_texture.remove(&texture.id).unwrap_or_default(),
            )
        })
        .collect())
}

pub async fn public_texture_library_metadata_by_texture_ids<S>(
    state: &S,
    textures: &[minecraft_texture::Model],
) -> Result<Vec<PublicTextureLibraryTextureMetadata>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let texture_ids = textures
        .iter()
        .map(|texture| texture.id)
        .collect::<Vec<_>>();
    let user_ids = textures
        .iter()
        .map(|texture| texture.user_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut tags_by_texture =
        minecraft_texture_tag_repo::list_for_texture_ids(state.reader_db(), &texture_ids).await?;
    let users = user_repo::find_by_ids(state.reader_db(), &user_ids).await?;
    let profiles = user_profile_repo::find_by_user_ids(state.reader_db(), &user_ids).await?;
    let users_by_id = users
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();

    Ok(textures
        .iter()
        .map(|texture| {
            public_texture_library_metadata_with_tags(
                state,
                texture,
                tags_by_texture.remove(&texture.id).unwrap_or_default(),
                users_by_id.get(&texture.user_id),
                profiles.get(&texture.user_id),
            )
        })
        .collect())
}

pub async fn public_texture_library_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let tags = minecraft_texture_tag_repo::list_for_texture(state.reader_db(), texture.id).await?;
    let uploader = user_repo::find_by_id(state.reader_db(), texture.user_id).await?;
    let uploader_profile =
        user_profile_repo::find_by_user_id(state.reader_db(), texture.user_id).await?;
    Ok(public_texture_library_metadata_with_tags(
        state,
        texture,
        tags,
        Some(&uploader),
        uploader_profile.as_ref(),
    ))
}

fn public_texture_library_metadata_with_tags<S>(
    state: &S,
    texture: &minecraft_texture::Model,
    tags: Vec<minecraft_texture_tag::Model>,
    uploader: Option<&user::Model>,
    uploader_profile: Option<&user_profile::Model>,
) -> PublicTextureLibraryTextureMetadata
where
    S: RuntimeConfigRuntimeState,
{
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    PublicTextureLibraryTextureMetadata {
        id: texture.id,
        name: texture_display_name(texture),
        display_name: texture.display_name.clone(),
        hash: texture.hash.clone(),
        texture_type: texture.texture_type,
        texture_model: texture.texture_model,
        visibility: texture.visibility,
        library_status: texture.library_status,
        tags: tags.into_iter().map(texture_tag_info).collect(),
        uploader: uploader.map(|user| texture_uploader_info(user, uploader_profile)),
        width: texture.width,
        height: texture.height,
        file_size: texture.file_size,
        mime_type: texture.mime_type.clone(),
        url: crate::services::yggdrasil_signature::texture_object_url(
            &policy,
            &texture.hash,
            &texture.storage_key,
        ),
        preview_url: super::current_texture_preview_url(
            state.runtime_config(),
            &texture.hash,
            texture.texture_type,
            texture.texture_model,
        ),
        created_at: texture.created_at,
        updated_at: texture.updated_at,
    }
}

fn texture_uploader_info(
    user: &user::Model,
    profile: Option<&user_profile::Model>,
) -> MinecraftTextureUploaderInfo {
    let name = profile
        .and_then(|profile| profile.display_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&user.username)
        .to_owned();
    MinecraftTextureUploaderInfo {
        public_uuid: user.public_uuid.clone(),
        name,
    }
}

pub fn texture_tag_info(tag: minecraft_texture_tag::Model) -> MinecraftTextureTagInfo {
    MinecraftTextureTagInfo {
        id: tag.id,
        name: tag.name,
        color: tag.color,
        sort_order: tag.sort_order,
        created_at: tag.created_at,
        updated_at: tag.updated_at,
    }
}
