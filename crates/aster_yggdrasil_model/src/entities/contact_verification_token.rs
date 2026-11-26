//! Contact verification token entity.

use crate::types::auth::{VerificationChannel, VerificationPurpose};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "contact_verification_tokens")]
/// Single-use verification token confirming ownership of a contact address
/// (currently email) for registration, contact change, or password reset.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Account the verification is for.
    pub user_id: i64,
    /// Delivery channel; only email exists today.
    pub channel: VerificationChannel,
    /// What the verification grants (`register_activation`, `contact_change`, or `password_reset`).
    pub purpose: VerificationPurpose,
    /// Address being verified.
    pub target: String,
    /// Hash of the one-time token; the plaintext value exists only in the verification link.
    pub token_hash: String,
    /// When the token stops being redeemable.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: DateTimeUtc,
    /// When the token was redeemed; `None` while outstanding.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub consumed_at: Option<DateTimeUtc>,
    /// When the token was issued.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the verified account.
pub enum Relation {
    /// Account the verification belongs to.
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
