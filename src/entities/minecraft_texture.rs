//! User-owned Minecraft texture asset entity.

use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureType,
    MinecraftTextureVisibility,
};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftTextureModelEntity)
)]
#[sea_orm(table_name = "minecraft_textures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub texture_type: MinecraftTextureType,
    pub hash: String,
    pub storage_key: String,
    pub mime_type: String,
    pub file_size: i64,
    pub width: i32,
    pub height: i32,
    pub texture_model: MinecraftTextureModel,
    pub visibility: MinecraftTextureVisibility,
    pub is_wardrobe_item: bool,
    pub display_name: Option<String>,
    pub library_status: MinecraftTextureLibraryStatus,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::minecraft_profile_texture::Entity")]
    MinecraftProfileTextures,
    #[sea_orm(has_many = "super::minecraft_texture_tag_binding::Entity")]
    MinecraftTextureTagBindings,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::minecraft_profile_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftProfileTextures.def()
    }
}

impl Related<super::minecraft_texture_tag_binding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTextureTagBindings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
