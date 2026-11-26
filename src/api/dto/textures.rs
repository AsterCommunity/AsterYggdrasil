//! Texture management DTOs.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use validator::Validate;

use aster_forge_api::NullablePatch;

use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureModel, yggdrasil::MinecraftTextureReportReason,
    yggdrasil::MinecraftTextureVisibility,
};

/// Request body binding an existing wardrobe texture to a profile texture
/// slot.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct BindMinecraftTextureReq {
    /// Id of the wardrobe texture to bind; must be a positive id owned by the
    /// current user.
    #[validate(range(min = 1))]
    pub texture_id: i64,
}

/// Request body for updating one texture in the current user's wardrobe.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateWardrobeTextureReq {
    /// New display name, `null` to clear, or omit to keep.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub display_name: Option<NullablePatch<String>>,
    /// Player model the texture targets (default or slim); omit to keep.
    pub texture_model: Option<MinecraftTextureModel>,
    /// New visibility; omit to keep.
    pub visibility: Option<MinecraftTextureVisibility>,
}

/// Request body for copying a published texture-library texture into the
/// current user's wardrobe.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CopyPublicTextureReq {
    /// Wardrobe display name for the copy; `null` clears it, omit to reuse
    /// the source name.
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

/// Request body replacing the tag set of a texture-library texture.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ReplaceWardrobeTextureTagsReq {
    /// Complete replacement tag id list; at most 16 tags.
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Vec<i64>,
}

/// Request body for creating a texture-library tag.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateMinecraftTextureTagReq {
    /// Tag name; 1-64 characters.
    #[validate(length(min = 1, max = 64, message = "tag name must be 1-64 characters"))]
    pub name: String,
    /// Tag color as a `#RRGGBB` hex value.
    #[validate(length(min = 4, max = 16, message = "tag color must be 4-16 characters"))]
    pub color: String,
    /// Optional sort position; omit to append after existing tags.
    pub sort_order: Option<i32>,
}

/// Request body for partially updating a texture-library tag. Omitted fields
/// keep their current value.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateMinecraftTextureTagReq {
    /// New tag name; 1-64 characters when provided.
    #[validate(length(min = 1, max = 64, message = "tag name must be 1-64 characters"))]
    pub name: Option<String>,
    /// New tag color as a `#RRGGBB` hex value when provided.
    #[validate(length(min = 4, max = 16, message = "tag color must be 4-16 characters"))]
    pub color: Option<String>,
    /// New sort position when provided.
    pub sort_order: Option<i32>,
}

/// Request body for reviewing a pending texture-library submission; shared by
/// the approve and reject endpoints.
#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ReviewTextureLibraryTextureReq {
    /// Reviewer note; up to 512 characters. Optional on approval, required on
    /// rejection.
    #[validate(length(max = 512, message = "review note must not exceed 512 characters"))]
    pub review_note: Option<String>,
    /// Replacement tag set applied on approval; at most 16 tags.
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Option<Vec<i64>>,
}

/// Request body for reporting a published texture-library texture to the
/// moderators.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateTextureReportReq {
    /// Report reason category (e.g. inappropriate, broken, copyright).
    pub reason: MinecraftTextureReportReason,
    /// Optional free-text detail; up to 1000 characters.
    #[validate(length(max = 1000, message = "report message must not exceed 1000 characters"))]
    pub message: Option<String>,
}

/// Request body for handling a texture report; shared by the resolve and
/// reject endpoints.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct HandleTextureReportReq {
    /// Optional moderator note recorded on the handled report; up to 512
    /// characters.
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
