//! User profile entity.

use crate::types::user::AvatarSource;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = UserProfileModel))]
#[sea_orm(table_name = "user_profiles")]
/// Display-oriented extension of a user account: optional display name and avatar.
pub struct Model {
    /// Owning account; shares the `users` id as a non-auto-incrementing primary key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    /// Optional name shown in place of the login username.
    pub display_name: Option<String>,
    /// Avatar mode (`none`, `gravatar`, or `upload`).
    pub avatar_source: AvatarSource,
    /// Storage key prefix of the uploaded avatar variants; set only for uploaded avatars.
    pub avatar_key: Option<String>,
    /// Counter bumped on every avatar change; versioned storage keys keep clients
    /// from serving stale cached images.
    pub avatar_version: i32,
    /// When the profile row was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the profile row was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the owning account.
pub enum Relation {
    /// Account this display profile belongs to.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
