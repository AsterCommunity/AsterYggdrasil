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
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub texture_id: i64,
    pub texture_type: MinecraftTextureType,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::minecraft_profile::Entity",
        from = "Column::ProfileId",
        to = "super::minecraft_profile::Column::Id"
    )]
    MinecraftProfile,
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
