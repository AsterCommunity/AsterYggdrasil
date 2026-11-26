//! User capability ban entity.

use crate::types::user::{UserBanScopes, UserBanStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_bans")]
/// Capability ban restricting which product abilities an account may use;
/// time-scoped and revocable.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Banned account.
    pub user_id: i64,
    /// Banned capability set as namespaced permission identifiers (for
    /// example `com.astercosm.yggdrasil.capability.yggdrasil.join`), stored
    /// as a sorted JSON array string.
    pub scopes: UserBanScopes,
    /// Ban lifecycle state (`active`, `revoked`, or `expired`).
    pub status: UserBanStatus,
    /// Reason recorded when the ban was issued.
    pub reason: String,
    /// Reason intended to be shown to the banned player.
    pub public_reason: Option<String>,
    /// Operator-only note about the ban.
    pub admin_note: Option<String>,
    /// Admin who issued the ban; cleared when that admin is deleted.
    pub created_by_user_id: Option<i64>,
    /// When the ban takes effect.
    pub starts_at: DateTimeUtc,
    /// When the ban lapses; `None` means the ban does not expire on its own.
    pub expires_at: Option<DateTimeUtc>,
    /// When the ban was revoked; `None` while it stands.
    pub revoked_at: Option<DateTimeUtc>,
    /// Admin who revoked the ban; cleared when that admin is deleted.
    pub revoked_by_user_id: Option<i64>,
    /// Note recorded together with the revocation.
    pub revoke_note: Option<String>,
    /// When the ban record was created.
    pub created_at: DateTimeUtc,
    /// When the ban record was last modified.
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the banned account, issuing and revoking admins, and history.
pub enum Relation {
    /// Banned account.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Admin who issued the ban.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedByUserId",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    /// Admin who revoked the ban.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RevokedByUserId",
        to = "super::user::Column::Id"
    )]
    RevokedByUser,
    /// Append-only change history of this ban.
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
