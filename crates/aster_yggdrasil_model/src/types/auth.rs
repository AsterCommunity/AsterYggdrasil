//! Authentication domain types.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Which web-login token in the pair a value refers to.
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Short-lived access token presented on each request.
    Access,
    /// Long-lived refresh token used to obtain new access tokens.
    Refresh,
}

impl TokenType {
    /// Canonical string form (`access` or `refresh`).
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
/// Delivery channel a verification message is sent through.
#[serde(rename_all = "snake_case")]
pub enum VerificationChannel {
    /// Verification code delivered by email.
    #[sea_orm(string_value = "email")]
    Email,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
/// Account flow a verification code was issued for.
#[serde(rename_all = "snake_case")]
pub enum VerificationPurpose {
    /// Activating a freshly registered account.
    #[sea_orm(string_value = "register_activation")]
    RegisterActivation,
    /// Confirming a change to the account's contact email.
    #[sea_orm(string_value = "contact_change")]
    ContactChange,
    /// Confirming a password reset request.
    #[sea_orm(string_value = "password_reset")]
    PasswordReset,
}
