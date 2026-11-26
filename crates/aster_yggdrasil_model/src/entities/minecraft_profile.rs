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
/// Minecraft player profile owned by a user account; the identity exposed
/// through the Yggdrasil API.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Owning user account.
    pub user_id: i64,
    /// Profile UUID as a 32-character undashed hex string; unique and stable across renames.
    #[sea_orm(unique)]
    pub uuid: String,
    /// Player-visible profile name (at most 16 characters); unique across profiles and
    /// changeable only through the controlled rename flow.
    #[sea_orm(unique)]
    pub name: String,
    /// Arm model skins are rendered with (`default` wide or `slim`).
    pub texture_model: MinecraftTextureModel,
    /// Comma-separated texture types this profile accepts (default `skin,cape`).
    pub uploadable_textures: String,
    // TODO(ban-system): profile disabling should be modeled by the future ban system,
    // not by adding a quick disabled flag here. It must define login, join,
    // hasJoined, texture read, and admin policy semantics together.
    /// When the profile was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the profile was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations from a profile to its owner, tokens, and texture slots.
pub enum Relation {
    /// Owning user account.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Launcher tokens that selected this profile at login.
    #[sea_orm(has_many = "super::yggdrasil_token::Entity")]
    YggdrasilTokens,
    /// Current texture bindings of this profile.
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
