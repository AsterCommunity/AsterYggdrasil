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
/// Join row attaching one administrator-managed tag to one texture.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Tagged texture.
    pub texture_id: i64,
    /// Tag attached to the texture.
    pub tag_id: i64,
    /// When the tag was attached.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the bound texture and tag.
pub enum Relation {
    /// Tagged texture.
    #[sea_orm(
        belongs_to = "super::minecraft_texture::Entity",
        from = "Column::TextureId",
        to = "super::minecraft_texture::Column::Id"
    )]
    MinecraftTexture,
    /// Tag attached to the texture.
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
