use sea_orm::{DeriveValueType, entity::prelude::*};
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
    #[sea_orm(string_value = "operator")]
    Operator,
    #[sea_orm(string_value = "user")]
    User,
}

impl UserRole {
    pub const fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum OperatorScope {
    #[sea_orm(string_value = "overview")]
    Overview,
    #[sea_orm(string_value = "users")]
    Users,
    #[sea_orm(string_value = "profiles")]
    Profiles,
    #[sea_orm(string_value = "texture_library")]
    TextureLibrary,
    #[sea_orm(string_value = "audit")]
    Audit,
    #[sea_orm(string_value = "tasks")]
    Tasks,
    #[sea_orm(string_value = "settings")]
    Settings,
    #[sea_orm(string_value = "external_auth")]
    ExternalAuth,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(48))")]
#[serde(rename_all = "snake_case")]
pub enum UserBanScope {
    #[sea_orm(string_value = "yggdrasil_access")]
    YggdrasilAccess,
    #[sea_orm(string_value = "yggdrasil_join")]
    YggdrasilJoin,
    #[sea_orm(string_value = "minecraft_profile_manage")]
    MinecraftProfileManage,
    #[sea_orm(string_value = "texture_upload")]
    TextureUpload,
    #[sea_orm(string_value = "texture_library_interact")]
    TextureLibraryInteract,
}

impl UserBanScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::YggdrasilAccess => "yggdrasil_access",
            Self::YggdrasilJoin => "yggdrasil_join",
            Self::MinecraftProfileManage => "minecraft_profile_manage",
            Self::TextureUpload => "texture_upload",
            Self::TextureLibraryInteract => "texture_library_interact",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct UserBanScopes(pub String);

impl UserBanScopes {
    pub fn new(scopes: Vec<UserBanScope>) -> Result<Self, UserBanScopesError> {
        let scopes = normalize_user_ban_scopes(scopes)?;
        let raw = serde_json::to_string(&scopes).map_err(|_| UserBanScopesError::Invalid)?;
        Ok(Self(raw))
    }

    pub fn from_stored(raw: impl Into<String>) -> Result<Self, UserBanScopesError> {
        let value = Self(raw.into());
        value.as_vec()?;
        Ok(value)
    }

    pub fn as_vec(&self) -> Result<Vec<UserBanScope>, UserBanScopesError> {
        let scopes = serde_json::from_str::<Vec<UserBanScope>>(&self.0)
            .map_err(|_| UserBanScopesError::Invalid)?;
        normalize_user_ban_scopes(scopes)
    }

    pub fn contains(&self, scope: UserBanScope) -> bool {
        self.as_vec()
            .map(|scopes| scopes.contains(&scope))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserBanScopesError {
    Empty,
    Invalid,
}

fn normalize_user_ban_scopes(
    mut scopes: Vec<UserBanScope>,
) -> Result<Vec<UserBanScope>, UserBanScopesError> {
    scopes.sort_unstable();
    scopes.dedup();
    if scopes.is_empty() {
        return Err(UserBanScopesError::Empty);
    }
    Ok(scopes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(24))")]
#[serde(rename_all = "snake_case")]
pub enum UserBanStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "revoked")]
    Revoked,
    #[sea_orm(string_value = "expired")]
    Expired,
}

impl UserBanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum UserBanEventType {
    #[sea_orm(string_value = "created")]
    Created,
    #[sea_orm(string_value = "updated")]
    Updated,
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

impl UserBanEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Revoked => "revoked",
        }
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
