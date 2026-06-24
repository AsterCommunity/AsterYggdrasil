//! Texture management DTOs.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use validator::Validate;

use aster_forge_api::NullablePatch;

use crate::types::{
    MinecraftTextureModel, MinecraftTextureReportReason, MinecraftTextureVisibility,
};

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
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub display_name: Option<NullablePatch<String>>,
    pub texture_model: Option<MinecraftTextureModel>,
    pub visibility: Option<MinecraftTextureVisibility>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CopyPublicTextureReq {
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
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

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ReviewTextureLibraryTextureReq {
    #[validate(length(max = 512, message = "review note must not exceed 512 characters"))]
    pub review_note: Option<String>,
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateTextureReportReq {
    pub reason: MinecraftTextureReportReason,
    #[validate(length(max = 1000, message = "report message must not exceed 1000 characters"))]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct HandleTextureReportReq {
    #[validate(length(max = 512, message = "admin note must not exceed 512 characters"))]
    pub admin_note: Option<String>,
}

#[cfg(test)]
mod tests {
    use aster_forge_api::NullablePatch;

    use super::UpdateWardrobeTextureReq;

    #[test]
    fn update_wardrobe_texture_display_name_preserves_nullable_patch_state() {
        let omitted: UpdateWardrobeTextureReq = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.display_name, None);

        let cleared: UpdateWardrobeTextureReq =
            serde_json::from_str(r#"{"display_name":null}"#).unwrap();
        assert_eq!(cleared.display_name, Some(NullablePatch::Null));

        let updated: UpdateWardrobeTextureReq =
            serde_json::from_str(r#"{"display_name":"Forge"}"#).unwrap();
        assert_eq!(
            updated.display_name,
            Some(NullablePatch::Value("Forge".to_string()))
        );
    }
}
