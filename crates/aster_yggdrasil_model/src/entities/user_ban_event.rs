//! Append-only user ban event entity.

use crate::types::user::{UserBanEventType, UserBanScopes, UserBanStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_ban_events")]
/// Append-only audit entry recording one state change of a user ban.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Ban this event belongs to.
    pub ban_id: i64,
    /// Admin who performed the change; `None` when the event is recorded without an
    /// acting admin, and cleared when that admin is deleted.
    pub actor_user_id: Option<i64>,
    /// Kind of change (`created`, `updated`, or `revoked`).
    pub event_type: UserBanEventType,
    /// Ban status before the change, when it changed.
    pub previous_status: Option<UserBanStatus>,
    /// Ban status after the change, when it changed.
    pub next_status: Option<UserBanStatus>,
    /// Banned scope set before the change, when it changed; a sorted JSON array string.
    pub previous_scopes: Option<UserBanScopes>,
    /// Banned scope set after the change, when it changed; a sorted JSON array string.
    pub next_scopes: Option<UserBanScopes>,
    /// Expiry before the change, when it changed.
    pub previous_expires_at: Option<DateTimeUtc>,
    /// Expiry after the change, when it changed.
    pub next_expires_at: Option<DateTimeUtc>,
    /// Free-form note recorded with the change.
    pub note: Option<String>,
    /// When the change happened.
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the changed ban and acting admin.
pub enum Relation {
    /// Ban this event belongs to.
    #[sea_orm(
        belongs_to = "super::user_ban::Entity",
        from = "Column::BanId",
        to = "super::user_ban::Column::Id"
    )]
    Ban,
    /// Admin who performed the change.
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
