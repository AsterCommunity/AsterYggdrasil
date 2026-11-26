//! Administrator-managed Minecraft texture library tag entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftTextureTagEntity)
)]
#[sea_orm(table_name = "minecraft_texture_tags")]
/// Administrator-managed tag used to categorize public texture-library entries.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Tag name as entered by the admin.
    pub name: String,
    /// Lowercase comparison form of the name; unique, so duplicate tags cannot be created.
    pub normalized_name: String,
    /// Display color for the tag chip (hex, default `#64748b`).
    pub color: String,
    /// Sort position among tags in the admin panel (lower comes first).
    pub sort_order: i32,
    /// When the tag was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the tag was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations from a tag to the textures carrying it.
pub enum Relation {
    /// Textures this tag is attached to.
    #[sea_orm(has_many = "super::minecraft_texture_tag_binding::Entity")]
    MinecraftTextureTagBindings,
}

impl Related<super::minecraft_texture_tag_binding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTextureTagBindings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
