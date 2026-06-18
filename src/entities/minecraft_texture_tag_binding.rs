//! Minecraft texture to administrator-managed tag binding entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftTextureTagBindingEntity)
)]
#[sea_orm(table_name = "minecraft_texture_tag_bindings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub texture_id: i64,
    pub tag_id: i64,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::minecraft_texture::Entity",
        from = "Column::TextureId",
        to = "super::minecraft_texture::Column::Id"
    )]
    MinecraftTexture,
    #[sea_orm(
        belongs_to = "super::minecraft_texture_tag::Entity",
        from = "Column::TagId",
        to = "super::minecraft_texture_tag::Column::Id"
    )]
    MinecraftTextureTag,
}

impl Related<super::minecraft_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTexture.def()
    }
}

impl Related<super::minecraft_texture_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTextureTag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
