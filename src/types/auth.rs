//! Authentication domain types.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum VerificationChannel {
    #[sea_orm(string_value = "email")]
    Email,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum VerificationPurpose {
    #[sea_orm(string_value = "register_activation")]
    RegisterActivation,
    #[sea_orm(string_value = "contact_change")]
    ContactChange,
    #[sea_orm(string_value = "password_reset")]
    PasswordReset,
}
