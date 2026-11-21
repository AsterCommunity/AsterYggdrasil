//! Append-only user ban event entity.

use crate::types::user::{UserBanEventType, UserBanScopes, UserBanStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_ban_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub ban_id: i64,
    pub actor_user_id: Option<i64>,
    pub event_type: UserBanEventType,
    pub previous_status: Option<UserBanStatus>,
    pub next_status: Option<UserBanStatus>,
    pub previous_scopes: Option<UserBanScopes>,
    pub next_scopes: Option<UserBanScopes>,
    pub previous_expires_at: Option<DateTimeUtc>,
    pub next_expires_at: Option<DateTimeUtc>,
    pub note: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_ban::Entity",
        from = "Column::BanId",
        to = "super::user_ban::Column::Id"
    )]
    Ban,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ActorUserId",
        to = "super::user::Column::Id"
    )]
    ActorUser,
}

impl Related<super::user_ban::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ban.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
