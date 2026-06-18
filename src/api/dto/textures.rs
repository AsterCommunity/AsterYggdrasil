//! Texture management DTOs.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use validator::Validate;

use crate::types::{MinecraftTextureVisibility, NullablePatch};

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct BindMinecraftTextureReq {
    #[validate(range(min = 1))]
    pub texture_id: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateWardrobeTextureReq {
    #[serde(
        default,
        deserialize_with = "crate::types::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub display_name: Option<NullablePatch<String>>,
    pub visibility: Option<MinecraftTextureVisibility>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CopyPublicTextureReq {
    #[serde(
        default,
        deserialize_with = "crate::types::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub display_name: Option<NullablePatch<String>>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ReplaceWardrobeTextureTagsReq {
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateMinecraftTextureTagReq {
    #[validate(length(min = 1, max = 64, message = "tag name must be 1-64 characters"))]
    pub name: String,
    #[validate(length(min = 4, max = 16, message = "tag color must be 4-16 characters"))]
    pub color: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateMinecraftTextureTagReq {
    #[validate(length(min = 1, max = 64, message = "tag name must be 1-64 characters"))]
    pub name: Option<String>,
    #[validate(length(min = 4, max = 16, message = "tag color must be 4-16 characters"))]
    pub color: Option<String>,
    pub sort_order: Option<i32>,
}
