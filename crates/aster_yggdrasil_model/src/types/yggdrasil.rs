//! Yggdrasil session forwarding and Minecraft texture domain enums.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Where a session-forward server entry resolves its checks from.
#[serde(rename_all = "snake_case")]
pub enum YggdrasilSessionForwardProviderKind {
    /// Built-in entry served by this deployment's own session store.
    #[sea_orm(string_value = "local")]
    Local,
    /// Upstream Yggdrasil server queried over HTTP.
    #[sea_orm(string_value = "remote")]
    Remote,
}

impl YggdrasilSessionForwardProviderKind {
    /// Canonical storage string for this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
/// Wire dialect a remote forward target speaks on its session endpoints.
#[serde(rename_all = "snake_case")]
pub enum YggdrasilSessionForwardEndpointKind {
    /// authlib-injector style session API root.
    #[sea_orm(string_value = "authlib_injector")]
    AuthlibInjector,
    /// Legacy Mojang session server URL shape.
    #[sea_orm(string_value = "mojang_session")]
    MojangSession,
}

impl YggdrasilSessionForwardEndpointKind {
    /// Canonical storage string for this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthlibInjector => "authlib_injector",
            Self::MojangSession => "mojang_session",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Sort key for admin listings of session-forward servers.
#[serde(rename_all = "snake_case")]
pub enum YggdrasilSessionForwardServerSortBy {
    /// Effective call order (enabled state, priority, weight) used for lookups.
    #[default]
    CallOrder,
    /// Stable database id.
    Id,
}

impl YggdrasilSessionForwardServerSortBy {
    /// Query-parameter string for this sort key.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CallOrder => "call_order",
            Self::Id => "id",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Player skin model variants the protocol distinguishes.
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureModel {
    /// Classic 4-pixel-arm model; the protocol default that omits metadata.
    #[sea_orm(string_value = "default")]
    Default,
    /// Slim 3-pixel-arm ("Alex") model.
    #[sea_orm(string_value = "slim")]
    Slim,
}

impl MinecraftTextureModel {
    /// Canonical storage string for this model.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Slim => "slim",
        }
    }

    /// Value written into the `textures` metadata block; `Default` returns
    /// `None` so the metadata object is omitted on the wire.
    pub const fn as_metadata_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Slim => Some("slim"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Texture slot type a player profile can bind.
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureType {
    /// Player skin texture.
    #[sea_orm(string_value = "skin")]
    Skin,
    /// Player cape texture.
    #[sea_orm(string_value = "cape")]
    Cape,
}

impl MinecraftTextureType {
    /// Canonical storage string for this type.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skin => "skin",
            Self::Cape => "cape",
        }
    }

    /// Uppercase key used inside the signed `textures` property payload.
    pub const fn textures_property_key(self) -> &'static str {
        match self {
            Self::Skin => "SKIN",
            Self::Cape => "CAPE",
        }
    }

    /// Parses a texture slot name from a URL path segment (`skin`, `cape`).
    pub fn parse_path(value: &str) -> Option<Self> {
        match value {
            "skin" => Some(Self::Skin),
            "cape" => Some(Self::Cape),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Whether a bound texture is served to other players or kept owner-only.
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureVisibility {
    /// Only the owner can fetch the texture.
    #[sea_orm(string_value = "private")]
    Private,
    /// Publicly served through the Yggdrasil texture endpoints.
    #[sea_orm(string_value = "public")]
    Public,
}

impl MinecraftTextureVisibility {
    /// Canonical storage string for this visibility.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
        }
    }

    /// Parses an upload-form value; an empty field defaults to `Private`.
    pub fn parse_form_value(value: &str) -> Option<Self> {
        match value.trim() {
            "" | "private" => Some(Self::Private),
            "public" => Some(Self::Public),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(24))")]
/// Review lifecycle of a texture submitted to the shared library.
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureLibraryStatus {
    /// Not submitted for review; visible only to the owner.
    #[sea_orm(string_value = "private")]
    Private,
    /// Submitted and waiting for moderator review.
    #[sea_orm(string_value = "pending_review")]
    PendingReview,
    /// Approved and listed in the shared library.
    #[sea_orm(string_value = "published")]
    Published,
    /// Denied during review.
    #[sea_orm(string_value = "rejected")]
    Rejected,
}

/// Reason a player gave when reporting a library texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(24))")]
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureReportReason {
    /// Content judged unsuitable for the community.
    #[sea_orm(string_value = "inappropriate")]
    Inappropriate,
    /// Insulting or hateful content.
    #[sea_orm(string_value = "offensive")]
    Offensive,
    /// Uses assets the reporter believes are copied without permission.
    #[sea_orm(string_value = "copyright")]
    Copyright,
    /// Impersonates another player or misrepresents its content.
    #[sea_orm(string_value = "misleading")]
    Misleading,
    /// Broken, unreadable, or corrupted texture.
    #[sea_orm(string_value = "broken")]
    Broken,
    /// Advertising or repeated unwanted submissions.
    #[sea_orm(string_value = "spam")]
    Spam,
    /// Anything not covered by the fixed reason list.
    #[sea_orm(string_value = "other")]
    Other,
}

/// Moderator outcome for a submitted library texture report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(24))")]
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureReportStatus {
    /// Reported and waiting for moderator action.
    #[sea_orm(string_value = "pending")]
    Pending,
    /// Upheld; the reported texture was actioned on.
    #[sea_orm(string_value = "accepted")]
    Accepted,
    /// Declined; the texture stays as it is.
    #[sea_orm(string_value = "rejected")]
    Rejected,
}

/// How tag filters combine in library searches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TextureTagSearchMethod {
    /// Result must carry every selected tag.
    #[default]
    All,
    /// Result must carry at least one selected tag.
    Any,
}

impl TextureTagSearchMethod {
    /// Query-parameter string for this method.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }
}

impl MinecraftTextureLibraryStatus {
    /// Canonical storage string for this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Rejected => "rejected",
        }
    }
}

impl MinecraftTextureReportReason {
    /// Canonical storage string for this reason.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inappropriate => "inappropriate",
            Self::Offensive => "offensive",
            Self::Copyright => "copyright",
            Self::Misleading => "misleading",
            Self::Broken => "broken",
            Self::Spam => "spam",
            Self::Other => "other",
        }
    }
}

impl MinecraftTextureReportStatus {
    /// Canonical storage string for this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}
