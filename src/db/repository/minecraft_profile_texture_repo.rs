//! Minecraft profile current texture binding repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_yggdrasil_model::entities::{
    minecraft_profile_texture::{self, Entity as MinecraftProfileTexture},
    minecraft_texture::{self, Entity as MinecraftTexture},
};
use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureType;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
};

/// A profile's current texture binding paired with the bound texture record.
#[derive(Debug, Clone)]
pub struct ProfileTexture {
    /// The binding row connecting a profile to a texture of one slot type.
    pub binding: minecraft_profile_texture::Model,
    /// The texture record the binding points at.
    pub texture: minecraft_texture::Model,
}

/// Input for binding a texture into one of a profile's texture slots.
#[derive(Debug, Clone)]
pub struct UpsertMinecraftProfileTexture {
    /// Profile receiving the texture.
    pub profile_id: i64,
    /// Texture to bind into the slot.
    pub texture_id: i64,
    /// Slot type (skin, cape, ...) the texture is bound to; one binding per type per profile.
    pub texture_type: MinecraftTextureType,
}

/// Binds a texture into a profile's slot, replacing any existing binding for that slot type.
///
/// Inserts when the slot is empty, otherwise rewrites the existing binding row's texture id.
/// Returns the updated binding with its texture; a dangling `texture_id` is a
/// `record_not_found` error. Not transactional — wrap in a transaction when atomicity matters.
pub async fn upsert_for_profile<C: ConnectionTrait>(
    db: &C,
    input: UpsertMinecraftProfileTexture,
) -> Result<ProfileTexture> {
    let now = chrono::Utc::now();
    let binding = if let Some(existing) =
        find_binding_by_profile_and_type(db, input.profile_id, input.texture_type).await?
    {
        let mut active: minecraft_profile_texture::ActiveModel = existing.into();
        active.texture_id = Set(input.texture_id);
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_aster_err(AsterError::database_operation)?
    } else {
        minecraft_profile_texture::ActiveModel {
            profile_id: Set(input.profile_id),
            texture_id: Set(input.texture_id),
            texture_type: Set(input.texture_type),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)?
    };

    let texture = MinecraftTexture::find_by_id(binding.texture_id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("texture '{}'", binding.texture_id)))?;
    Ok(ProfileTexture { binding, texture })
}

/// Loads a profile's current binding for one texture slot type.
///
/// Returns `None` when the slot is empty.
pub async fn find_by_profile_and_type<C: ConnectionTrait>(
    db: &C,
    profile_id: i64,
    texture_type: MinecraftTextureType,
) -> Result<Option<ProfileTexture>> {
    let Some(binding) = find_binding_by_profile_and_type(db, profile_id, texture_type).await?
    else {
        return Ok(None);
    };
    let texture = MinecraftTexture::find_by_id(binding.texture_id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("texture '{}'", binding.texture_id)))?;
    Ok(Some(ProfileTexture { binding, texture }))
}

/// Lists every texture binding of a profile, ordered by texture type ascending.
pub async fn list_by_profile<C: ConnectionTrait>(
    db: &C,
    profile_id: i64,
) -> Result<Vec<ProfileTexture>> {
    let rows = MinecraftProfileTexture::find()
        .filter(minecraft_profile_texture::Column::ProfileId.eq(profile_id))
        .join(
            JoinType::InnerJoin,
            minecraft_profile_texture::Relation::MinecraftTexture.def(),
        )
        .order_by_asc(minecraft_profile_texture::Column::TextureType)
        .select_also(MinecraftTexture)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(binding, texture)| match texture {
            Some(texture) => Ok(ProfileTexture { binding, texture }),
            None => Err(AsterError::record_not_found(format!(
                "texture '{}'",
                binding.texture_id
            ))),
        })
        .collect()
}

/// Lists every profile texture binding across all profiles, ordered by binding id ascending.
pub async fn list_all<C: ConnectionTrait>(db: &C) -> Result<Vec<ProfileTexture>> {
    let rows = MinecraftProfileTexture::find()
        .join(
            JoinType::InnerJoin,
            minecraft_profile_texture::Relation::MinecraftTexture.def(),
        )
        .order_by_asc(minecraft_profile_texture::Column::Id)
        .select_also(MinecraftTexture)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(binding, texture)| match texture {
            Some(texture) => Ok(ProfileTexture { binding, texture }),
            None => Err(AsterError::record_not_found(format!(
                "texture '{}'",
                binding.texture_id
            ))),
        })
        .collect()
}

/// Lists every profile binding whose texture content hash matches `hash`, ordered by binding
/// id ascending.
///
/// Used to find which profiles reference a texture before deletion or invalidation.
pub async fn list_by_hash<C: ConnectionTrait>(db: &C, hash: &str) -> Result<Vec<ProfileTexture>> {
    let rows = MinecraftProfileTexture::find()
        .join(
            JoinType::InnerJoin,
            minecraft_profile_texture::Relation::MinecraftTexture.def(),
        )
        .filter(minecraft_texture::Column::Hash.eq(hash))
        .order_by_asc(minecraft_profile_texture::Column::Id)
        .select_also(MinecraftTexture)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(binding, texture)| match texture {
            Some(texture) => Ok(ProfileTexture { binding, texture }),
            None => Err(AsterError::record_not_found(format!(
                "texture '{}'",
                binding.texture_id
            ))),
        })
        .collect()
}

/// Counts how many profiles currently bind the given texture.
pub async fn count_by_texture_id<C: ConnectionTrait>(db: &C, texture_id: i64) -> Result<u64> {
    MinecraftProfileTexture::find()
        .filter(minecraft_profile_texture::Column::TextureId.eq(texture_id))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Lists every profile binding pointing at the given texture, ordered by binding id ascending.
pub async fn list_by_texture_id<C: ConnectionTrait>(
    db: &C,
    texture_id: i64,
) -> Result<Vec<ProfileTexture>> {
    let rows = MinecraftProfileTexture::find()
        .join(
            JoinType::InnerJoin,
            minecraft_profile_texture::Relation::MinecraftTexture.def(),
        )
        .filter(minecraft_profile_texture::Column::TextureId.eq(texture_id))
        .order_by_asc(minecraft_profile_texture::Column::Id)
        .select_also(MinecraftTexture)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(binding, texture)| match texture {
            Some(texture) => Ok(ProfileTexture { binding, texture }),
            None => Err(AsterError::record_not_found(format!(
                "texture '{}'",
                binding.texture_id
            ))),
        })
        .collect()
}

/// Removes every profile binding pointing at the given texture, returning the removed
/// bindings.
///
/// Not transactional: each binding is deleted in its own statement, so callers should wrap
/// this in a transaction when atomicity matters.
pub async fn delete_by_texture_id<C: ConnectionTrait>(
    db: &C,
    texture_id: i64,
) -> Result<Vec<ProfileTexture>> {
    let bindings = list_by_texture_id(db, texture_id).await?;
    for binding in &bindings {
        MinecraftProfileTexture::delete_by_id(binding.binding.id)
            .exec(db)
            .await
            .map_aster_err(AsterError::database_operation)?;
    }
    Ok(bindings)
}

/// Clears one texture slot of a profile, returning the removed binding.
///
/// Returns `None` when the slot was already empty.
pub async fn delete_for_profile<C: ConnectionTrait>(
    db: &C,
    profile_id: i64,
    texture_type: MinecraftTextureType,
) -> Result<Option<ProfileTexture>> {
    let Some(existing) = find_by_profile_and_type(db, profile_id, texture_type).await? else {
        return Ok(None);
    };
    MinecraftProfileTexture::delete_by_id(existing.binding.id)
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(existing))
}

async fn find_binding_by_profile_and_type<C: ConnectionTrait>(
    db: &C,
    profile_id: i64,
    texture_type: MinecraftTextureType,
) -> Result<Option<minecraft_profile_texture::Model>> {
    MinecraftProfileTexture::find()
        .filter(minecraft_profile_texture::Column::ProfileId.eq(profile_id))
        .filter(minecraft_profile_texture::Column::TextureType.eq(texture_type))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}
