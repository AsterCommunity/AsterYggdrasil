//! Project Minecraft profile API DTOs.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Filter query for the current user's Minecraft profile list, ordered by id.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct CurrentMinecraftProfileListQuery {
    /// Optional search term matched case-insensitively against profile name;
    /// up to 64 characters.
    #[validate(length(max = 64, message = "query must not exceed 64 characters"))]
    pub query: Option<String>,
    /// Cursor: id of the last profile on the previous page.
    pub after_id: Option<i64>,
}

/// Request body for creating a Minecraft profile for the current user.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateMinecraftProfileReq {
    /// Desired profile name; 3-16 ASCII letters, digits or underscores, unique
    /// among live profiles.
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub name: String,
}

/// Request body for renaming a Minecraft profile; the UUID, texture bindings
/// and launcher sessions are preserved while bound tokens are invalidated.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RenameMinecraftProfileReq {
    /// New profile name; 3-16 ASCII letters, digits or underscores, unique
    /// among live profiles.
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub name: String,
}
