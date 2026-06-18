use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "user")]
    User,
}

impl UserRole {
    pub const fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

impl UserStatus {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum AvatarSource {
    #[sea_orm(string_value = "none")]
    None,
    #[sea_orm(string_value = "gravatar")]
    Gravatar,
    #[sea_orm(string_value = "upload")]
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum UserInvitationStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "accepted")]
    Accepted,
    #[sea_orm(string_value = "expired")]
    Expired,
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

impl UserInvitationStatus {
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }
}
