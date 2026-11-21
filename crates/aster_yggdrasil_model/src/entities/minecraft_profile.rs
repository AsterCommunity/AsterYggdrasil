//! Minecraft player profile entity.

use crate::types::yggdrasil::MinecraftTextureModel;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftProfileModel)
)]
#[sea_orm(table_name = "minecraft_profiles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    #[sea_orm(unique)]
    pub uuid: String,
    #[sea_orm(unique)]
    pub name: String,
    pub texture_model: MinecraftTextureModel,
    pub uploadable_textures: String,
    // TODO(ban-system): profile disabling should be modeled by the future ban system,
    // not by adding a quick disabled flag here. It must define login, join,
    // hasJoined, texture read, and admin policy semantics together.
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
    #[sea_orm(has_many = "super::yggdrasil_token::Entity")]
    YggdrasilTokens,
    #[sea_orm(has_many = "super::minecraft_profile_texture::Entity")]
    MinecraftProfileTextures,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::yggdrasil_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::YggdrasilTokens.def()
    }
}

impl Related<super::minecraft_profile_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftProfileTextures.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
