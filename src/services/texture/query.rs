use std::collections::{BTreeSet, HashMap};

use tokio_util::io::ReaderStream;

use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{
    minecraft_profile_texture_repo, minecraft_texture_repo, minecraft_texture_tag_repo,
    user_profile_repo, user_repo,
};
use crate::errors::Result;
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState,
};
use crate::services::user::profile::{self, AvatarAudience};
use aster_yggdrasil_model::entities::{
    minecraft_profile, minecraft_texture, minecraft_texture_tag, user, user_profile,
};

use super::{
    MinecraftTextureMetadata, MinecraftTextureMetadataSource, MinecraftTextureTagInfo,
    MinecraftTextureUploaderInfo, MinecraftWardrobeTextureMetadata,
    PublicTextureLibraryTextureMetadata, TEXTURE_CACHE_CONTROL, TextureBlob, TextureDownload,
    default_skin, is_valid_texture_hash,
};

/// Looks up a wardrobe texture asset by its SHA-256 hash; `None` for invalid
/// hashes or unknown content.
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

/// Resolves a hash to the object-storage key of its texture blob, caching the
/// lookup so repeated reads skip the database. Invalid hashes yield `None`.
pub async fn texture_blob_by_hash<S>(state: &S, hash: &str) -> Result<Option<TextureBlob>>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    if !is_valid_texture_hash(hash) {
        tracing::debug!(hash, "texture blob lookup skipped invalid hash");
        return Ok(None);
    }
    if let Some(blob) = super::cache::get_texture_blob_lookup(state, hash).await {
        tracing::debug!(hash, "texture blob lookup cache hit");
        return Ok(Some(blob));
    }
    let blob = minecraft_texture_repo::find_by_hash(state.reader_db(), hash)
        .await?
        .map(|texture| TextureBlob {
            storage_key: texture.storage_key,
        });
    if let Some(blob) = blob.as_ref() {
        super::cache::set_texture_blob_lookup(state, hash, blob).await;
    }
    tracing::debug!(
        hash,
        found = blob.is_some(),
        "looked up texture blob by hash"
    );
    Ok(blob)
}

pub(crate) async fn invalidate_texture_blob_lookup_cache<S>(state: &S, hash: &str)
where
    S: CacheRuntimeState,
{
    super::cache::invalidate_texture_blob_lookup(state, hash).await;
}

/// Opens a streaming download of a bound profile texture from object storage,
/// with the texture content type and immutable cache header attached.
pub async fn download_texture<S: TextureStorageRuntimeState>(
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

/// Opens a streaming download of a texture blob previously resolved via
/// [`texture_blob_by_hash`], with the same content type and cache headers.
pub async fn download_texture_blob<S: TextureStorageRuntimeState>(
    state: &S,
    texture: &TextureBlob,
) -> Result<TextureDownload> {
    tracing::debug!("downloading texture blob");
    download_object_storage_key(state, &texture.storage_key).await
}

async fn download_object_storage_key<S: TextureStorageRuntimeState>(
    state: &S,
    storage_key: &str,
) -> Result<TextureDownload> {
    let metadata = state.texture_storage().metadata(storage_key).await?;
    let stream = state.texture_storage().get_stream(storage_key).await?;
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

/// Lists every texture binding on the profile, one entry per occupied slot.
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

/// Builds serialized metadata for every texture bound to the profile,
/// appending the UUID-derived default skin entry when the skin slot is empty
/// so clients always see a wearable skin.
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
    if !metadata.iter().any(|texture| {
        texture.texture_type == aster_yggdrasil_model::types::yggdrasil::MinecraftTextureType::Skin
    }) {
        metadata.push(default_skin_metadata(state, profile)?);
    }
    Ok(metadata)
}

/// Maps one profile texture binding into [`MinecraftTextureMetadata`],
/// deriving the texture and preview URLs from the runtime Yggdrasil policy.
/// Pure projection; no database access.
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
        texture_id: texture.texture.id,
        name: texture_display_name(&texture.texture),
        display_name: texture.texture.display_name.clone(),
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
        url: crate::services::yggdrasil::signature::texture_object_url(
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

/// Builds [`MinecraftTextureMetadata`] for the built-in default skin chosen
/// from the profile UUID; used as the stand-in entry when no skin is bound.
pub fn default_skin_metadata<S>(
    state: &S,
    profile: &minecraft_profile::Model,
) -> Result<MinecraftTextureMetadata>
where
    S: RuntimeConfigRuntimeState,
{
    let skin = default_skin::for_profile_uuid(&profile.uuid);
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    Ok(MinecraftTextureMetadata {
        id: 0,
        texture_id: 0,
        name: "Default skin".to_string(),
        display_name: None,
        profile_id: profile.id,
        profile_uuid: profile.uuid.clone(),
        profile_name: profile.name.clone(),
        hash: skin.hash.to_string(),
        texture_type: aster_yggdrasil_model::types::yggdrasil::MinecraftTextureType::Skin,
        texture_model: skin.model,
        visibility: aster_yggdrasil_model::types::yggdrasil::MinecraftTextureVisibility::Public,
        width: skin.width,
        height: skin.height,
        file_size: skin.file_size,
        mime_type: "image/png".to_string(),
        url: crate::services::yggdrasil::signature::texture_base_url(&policy, skin.hash),
        preview_url: super::current_texture_preview_url(
            state.runtime_config(),
            skin.hash,
            aster_yggdrasil_model::types::yggdrasil::MinecraftTextureType::Skin,
            skin.model,
        ),
        source: MinecraftTextureMetadataSource::Default,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    })
}

/// Maps a wardrobe texture asset into [`MinecraftWardrobeTextureMetadata`]
/// with no library tags attached.
pub fn wardrobe_texture_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> MinecraftWardrobeTextureMetadata
where
    S: RuntimeConfigRuntimeState,
{
    wardrobe_texture_metadata_with_tags(state, texture, Vec::new())
}

/// Same as [`wardrobe_texture_metadata`], with the texture's library tags
/// rendered into the output.
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
        library_review_note: texture.library_review_note.clone(),
        library_submitted_at: texture.library_submitted_at,
        library_reviewed_at: texture.library_reviewed_at,
        tags: tags.into_iter().map(texture_tag_info).collect(),
        width: texture.width,
        height: texture.height,
        file_size: texture.file_size,
        mime_type: texture.mime_type.clone(),
        url: crate::services::yggdrasil::signature::texture_object_url(
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

/// Batch variant of [`wardrobe_texture_metadata_with_tags`]: fetches tags for
/// all given textures in one query before projecting each one.
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

/// Batch variant of [`public_texture_library_metadata`]; uploader avatars use
/// the public audience rendering.
pub async fn public_texture_library_metadata_by_texture_ids<S>(
    state: &S,
    textures: &[minecraft_texture::Model],
) -> Result<Vec<PublicTextureLibraryTextureMetadata>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    texture_library_metadata_by_texture_ids(state, textures, None).await
}

/// Batch variant of [`admin_texture_library_metadata`]; uploader avatars point
/// at the admin avatar API path instead of the self-service one.
pub async fn admin_texture_library_metadata_by_texture_ids<S>(
    state: &S,
    textures: &[minecraft_texture::Model],
) -> Result<Vec<PublicTextureLibraryTextureMetadata>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    texture_library_metadata_by_texture_ids(state, textures, Some(AvatarAudience::AdminUser)).await
}

async fn texture_library_metadata_by_texture_ids<S>(
    state: &S,
    textures: &[minecraft_texture::Model],
    avatar_audience: Option<AvatarAudience>,
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
                avatar_audience,
            )
        })
        .collect())
}

/// Builds the publicly shown [`PublicTextureLibraryTextureMetadata`] for a
/// library texture, resolving uploader identity and avatar when available.
pub async fn public_texture_library_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    texture_library_metadata(state, texture, None).await
}

/// Like [`public_texture_library_metadata`], but uploader avatars point at
/// the admin avatar API path (`/admin/avatars/...`) instead of the
/// self-service profile path.
pub async fn admin_texture_library_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
) -> Result<PublicTextureLibraryTextureMetadata>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    texture_library_metadata(state, texture, Some(AvatarAudience::AdminUser)).await
}

async fn texture_library_metadata<S>(
    state: &S,
    texture: &minecraft_texture::Model,
    avatar_audience: Option<AvatarAudience>,
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
        avatar_audience,
    ))
}

fn public_texture_library_metadata_with_tags<S>(
    state: &S,
    texture: &minecraft_texture::Model,
    tags: Vec<minecraft_texture_tag::Model>,
    uploader: Option<&user::Model>,
    uploader_profile: Option<&user_profile::Model>,
    avatar_audience: Option<AvatarAudience>,
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
        library_review_note: texture.library_review_note.clone(),
        library_submitted_at: texture.library_submitted_at,
        library_reviewed_at: texture.library_reviewed_at,
        tags: tags.into_iter().map(texture_tag_info).collect(),
        uploader: uploader
            .map(|user| texture_uploader_info(state, user, uploader_profile, avatar_audience)),
        width: texture.width,
        height: texture.height,
        file_size: texture.file_size,
        mime_type: texture.mime_type.clone(),
        url: crate::services::yggdrasil::signature::texture_object_url(
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

fn texture_uploader_info<S>(
    state: &S,
    user: &user::Model,
    profile: Option<&user_profile::Model>,
    avatar_audience: Option<AvatarAudience>,
) -> MinecraftTextureUploaderInfo
where
    S: RuntimeConfigRuntimeState,
{
    let name = profile
        .and_then(|profile| profile.display_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&user.username)
        .to_owned();
    let avatar = avatar_audience
        .map(|avatar_audience| {
            profile::build_profile_info(
                user,
                profile,
                avatar_audience,
                &profile::resolve_gravatar_base_url(state),
            )
            .avatar
        })
        .unwrap_or_else(|| profile::AvatarInfo {
            source: aster_yggdrasil_model::types::user::AvatarSource::None,
            url_512: None,
            url_1024: None,
            version: 0,
        });
    MinecraftTextureUploaderInfo {
        id: user.id,
        username: user.username.clone(),
        public_uuid: user.public_uuid.clone(),
        name,
        avatar,
    }
}

/// Maps a tag row into its serialized [`MinecraftTextureTagInfo`] form.
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
