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
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub color: String,
    pub sort_order: i32,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::minecraft_texture_tag_binding::Entity")]
    MinecraftTextureTagBindings,
}

impl Related<super::minecraft_texture_tag_binding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTextureTagBindings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
