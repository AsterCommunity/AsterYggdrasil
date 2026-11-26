//! User account domain types: roles, operator scopes, bans, and invitations.

use sea_orm::{DeriveValueType, entity::prelude::*};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// Platform role of an account, stored on the user record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// Full administrator; bypasses per-scope operator permission checks.
    #[sea_orm(string_value = "admin")]
    Admin,
    /// Operator with a fixed subset of admin-panel scopes.
    #[sea_orm(string_value = "operator")]
    Operator,
    /// Regular player account with no admin-panel access.
    #[sea_orm(string_value = "user")]
    User,
}

impl UserRole {
    /// Whether this role is the full administrator, which gates admin-only
    /// routes independently of [`OperatorScope`] grants.
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
/// Admin-panel permission scope granted to an operator account; full admins
/// pass every scope gate without holding any of these.
#[serde(rename_all = "snake_case")]
pub enum OperatorScope {
    /// Dashboard and server overview pages.
    #[sea_orm(string_value = "overview")]
    Overview,
    /// User account management.
    #[sea_orm(string_value = "users")]
    Users,
    /// Minecraft profile management.
    #[sea_orm(string_value = "profiles")]
    Profiles,
    /// Texture library moderation.
    #[sea_orm(string_value = "texture_library")]
    TextureLibrary,
    /// Audit log access.
    #[sea_orm(string_value = "audit")]
    Audit,
    /// Background task management.
    #[sea_orm(string_value = "tasks")]
    Tasks,
    /// Runtime and system configuration.
    #[sea_orm(string_value = "settings")]
    Settings,
    /// External authentication provider management.
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
/// One independently enforceable punishment scope of a user ban.
#[serde(rename_all = "snake_case")]
pub enum UserBanScope {
    /// Blocks Yggdrasil authenticate/refresh/token operations.
    #[sea_orm(string_value = "yggdrasil_access")]
    YggdrasilAccess,
    /// Blocks the multiplayer join handshake for this account.
    #[sea_orm(string_value = "yggdrasil_join")]
    YggdrasilJoin,
    /// Blocks creating, renaming, or deleting Minecraft profiles.
    #[sea_orm(string_value = "minecraft_profile_manage")]
    MinecraftProfileManage,
    /// Blocks uploading new textures.
    #[sea_orm(string_value = "texture_upload")]
    TextureUpload,
    /// Blocks interacting with the shared texture library.
    #[sea_orm(string_value = "texture_library_interact")]
    TextureLibraryInteract,
}

impl UserBanScope {
    /// Canonical storage/serialization string for this scope.
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

/// DB wrapper storing a ban's scope set as a sorted, deduplicated JSON array
/// of [`UserBanScope`] strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct UserBanScopes(pub String);

impl UserBanScopes {
    /// Builds the wrapper from a scope list, normalized (sorted and
    /// deduplicated) before serialization; rejects an empty list.
    pub fn new(scopes: Vec<UserBanScope>) -> Result<Self, UserBanScopesError> {
        let scopes = normalize_user_ban_scopes(scopes)?;
        let raw = serde_json::to_string(&scopes).map_err(|_| UserBanScopesError::Invalid)?;
        Ok(Self(raw))
    }

    /// Re-validates a raw JSON array read from the database, guaranteeing
    /// every read goes through the same normalization as [`Self::new`].
    pub fn from_stored(raw: impl Into<String>) -> Result<Self, UserBanScopesError> {
        let value = Self(raw.into());
        value.as_vec()?;
        Ok(value)
    }

    /// Decodes and re-normalizes the stored JSON into a scope list.
    pub fn as_vec(&self) -> Result<Vec<UserBanScope>, UserBanScopesError> {
        let scopes = serde_json::from_str::<Vec<UserBanScope>>(&self.0)
            .map_err(|_| UserBanScopesError::Invalid)?;
        normalize_user_ban_scopes(scopes)
    }

    /// Whether the ban covers `scope`; an unparseable stored value treats as
    /// not containing it rather than failing the check path.
    pub fn contains(&self, scope: UserBanScope) -> bool {
        self.as_vec()
            .map(|scopes| scopes.contains(&scope))
            .unwrap_or(false)
    }
}

/// Failure modes when constructing or decoding [`UserBanScopes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserBanScopesError {
    /// The scope list was empty after normalization.
    Empty,
    /// The stored value was not a valid JSON array of known scopes.
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
/// Lifecycle state of a single user ban record.
#[serde(rename_all = "snake_case")]
pub enum UserBanStatus {
    /// Currently enforced.
    #[sea_orm(string_value = "active")]
    Active,
    /// Lifted early by an administrator.
    #[sea_orm(string_value = "revoked")]
    Revoked,
    /// Lapsed because its end time passed.
    #[sea_orm(string_value = "expired")]
    Expired,
}

impl UserBanStatus {
    /// Canonical storage string for this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    /// Whether the ban still restricts the account right now.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
/// Kind of audit event recorded for a user ban.
#[serde(rename_all = "snake_case")]
pub enum UserBanEventType {
    /// Ban was created.
    #[sea_orm(string_value = "created")]
    Created,
    /// Ban record was edited (scopes or duration).
    #[sea_orm(string_value = "updated")]
    Updated,
    /// Ban was lifted before its end time.
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

impl UserBanEventType {
    /// Canonical storage string for this event type.
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
/// Account lifecycle state stored on the user record.
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// Account can log in and use the service.
    #[sea_orm(string_value = "active")]
    Active,
    /// Account is disabled and rejected at login.
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

impl UserStatus {
    /// Whether the account may log in.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
/// Where the account's avatar image comes from.
#[serde(rename_all = "snake_case")]
pub enum AvatarSource {
    /// No avatar; clients render a fallback.
    #[sea_orm(string_value = "none")]
    None,
    /// Avatar resolved from the account email's Gravatar.
    #[sea_orm(string_value = "gravatar")]
    Gravatar,
    /// Avatar uploaded by the user.
    #[sea_orm(string_value = "upload")]
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Lifecycle state of a registration invitation.
#[serde(rename_all = "snake_case")]
pub enum UserInvitationStatus {
    /// Unused and still within its validity window.
    #[sea_orm(string_value = "pending")]
    Pending,
    /// Consumed by a completed registration.
    #[sea_orm(string_value = "accepted")]
    Accepted,
    /// No longer usable because its expiry passed.
    #[sea_orm(string_value = "expired")]
    Expired,
    /// Invalidated by an administrator before use.
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

impl UserInvitationStatus {
    /// Whether the invitation can still be redeemed.
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }
}
