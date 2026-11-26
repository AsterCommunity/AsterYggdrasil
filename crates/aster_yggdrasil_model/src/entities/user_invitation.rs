//! User invitation entity.

use crate::types::user::UserInvitationStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "user_invitations")]
/// Invite-only registration invitation redeemable once through its signup link.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Email address the invitation was issued for.
    pub email: String,
    /// Hash of the single-use invitation token; the plaintext value exists only in the
    /// invitation link.
    pub token_hash: String,
    /// Invitation lifecycle state (`pending`, `accepted`, `expired`, or `revoked`).
    pub status: UserInvitationStatus,
    /// Account that issued the invitation.
    pub invited_by: i64,
    /// Account created by redeeming the invitation; `None` until acceptance.
    pub accepted_user_id: Option<i64>,
    /// When the invitation stops being redeemable.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: DateTimeUtc,
    /// When the invitation was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the invitation record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
    /// When the invitation was redeemed; `None` while unredeemed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub accepted_at: Option<DateTimeUtc>,
    /// When the invitation was revoked; `None` unless revoked before use.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub revoked_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the issuing and invited accounts.
pub enum Relation {
    /// Account that issued the invitation.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::InvitedBy",
        to = "super::user::Column::Id"
    )]
    InvitedByUser,
    /// Account created by redeeming the invitation.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::AcceptedUserId",
        to = "super::user::Column::Id"
    )]
    AcceptedUser,
}

impl ActiveModelBehavior for ActiveModel {}
