use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureModel {
    #[sea_orm(string_value = "default")]
    Default,
    #[sea_orm(string_value = "slim")]
    Slim,
}

impl MinecraftTextureModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Slim => "slim",
        }
    }

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
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureType {
    #[sea_orm(string_value = "skin")]
    Skin,
    #[sea_orm(string_value = "cape")]
    Cape,
}

impl MinecraftTextureType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skin => "skin",
            Self::Cape => "cape",
        }
    }

    pub const fn textures_property_key(self) -> &'static str {
        match self {
            Self::Skin => "SKIN",
            Self::Cape => "CAPE",
        }
    }

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
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureVisibility {
    #[sea_orm(string_value = "private")]
    Private,
    #[sea_orm(string_value = "public")]
    Public,
}

impl MinecraftTextureVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
        }
    }

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
#[serde(rename_all = "snake_case")]
pub enum MinecraftTextureLibraryStatus {
    #[sea_orm(string_value = "private")]
    Private,
    #[sea_orm(string_value = "pending_review")]
    PendingReview,
    #[sea_orm(string_value = "published")]
    Published,
    #[sea_orm(string_value = "rejected")]
    Rejected,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TextureTagSearchMethod {
    #[default]
    All,
    Any,
}

impl TextureTagSearchMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }
}

impl MinecraftTextureLibraryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Rejected => "rejected",
        }
    }
}
