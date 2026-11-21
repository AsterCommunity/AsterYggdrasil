//! User capability ban entity.

use crate::types::user::{UserBanScopes, UserBanStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_bans")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub scopes: UserBanScopes,
    pub status: UserBanStatus,
    pub reason: String,
    pub public_reason: Option<String>,
    pub admin_note: Option<String>,
    pub created_by_user_id: Option<i64>,
    pub starts_at: DateTimeUtc,
    pub expires_at: Option<DateTimeUtc>,
    pub revoked_at: Option<DateTimeUtc>,
    pub revoked_by_user_id: Option<i64>,
    pub revoke_note: Option<String>,
    pub created_at: DateTimeUtc,
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
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RevokedByUserId",
        to = "super::user::Column::Id"
    )]
    RevokedByUser,
    #[sea_orm(has_many = "super::user_ban_event::Entity")]
    Events,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::user_ban_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Events.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
