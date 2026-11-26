//! Public external authentication API DTOs.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Request body for starting an external auth login flow in the browser.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct StartExternalAuthReq {
    /// In-app path to redirect to after the login completes; up to 2048
    /// bytes.
    #[validate(length(max = 2048, message = "return_path must not exceed 2048 bytes"))]
    pub return_path: Option<String>,
    /// Accepted for compatibility but unused; the callback redirect target is
    /// derived server-side from the public site URL. Up to 2048 bytes.
    #[validate(length(max = 2048, message = "redirect_uri must not exceed 2048 bytes"))]
    pub redirect_uri: Option<String>,
}

/// Query parameters the provider sends to the external auth callback URL.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct ExternalAuthCallbackQuery {
    /// Opaque flow state that ties the callback to the started login flow.
    pub state: Option<String>,
    /// Authorization code issued by the provider.
    pub code: Option<String>,
    /// Error code reported by the provider instead of a code.
    pub error: Option<String>,
    /// Human-readable provider error detail accompanying `error`.
    pub error_description: Option<String>,
}
