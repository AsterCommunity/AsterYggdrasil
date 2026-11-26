//! Minecraft profile current texture binding entity.

use crate::types::yggdrasil::MinecraftTextureType;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftProfileTextureModelEntity)
)]
#[sea_orm(table_name = "minecraft_profile_textures")]
/// Texture currently equipped in one slot of a profile; a profile holds at most
/// one binding per texture type.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Profile the texture is equipped on.
    pub profile_id: i64,
    /// Texture asset occupying the slot.
    pub texture_id: i64,
    /// Slot kind (`skin` or `cape`); unique together with the profile.
    pub texture_type: MinecraftTextureType,
    /// When the binding was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the binding was last changed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the equipped profile and texture.
pub enum Relation {
    /// Profile the texture is equipped on.
    #[sea_orm(
        belongs_to = "super::minecraft_profile::Entity",
        from = "Column::ProfileId",
        to = "super::minecraft_profile::Column::Id"
    )]
    MinecraftProfile,
    /// Texture asset occupying the slot.
    #[sea_orm(
        belongs_to = "super::minecraft_texture::Entity",
        from = "Column::TextureId",
        to = "super::minecraft_texture::Column::Id"
    )]
    MinecraftTexture,
}

impl Related<super::minecraft_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftProfile.def()
    }
}

impl Related<super::minecraft_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTexture.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
