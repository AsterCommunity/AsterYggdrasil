use crate::config::{auth_runtime, branding, site_url, texture_library, yggdrasil};
use crate::runtime::{RuntimeConfigRuntimeState, SharedRuntimeState};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// Cache-Control header served with public config responses; values may lag
/// up to a minute behind admin changes.
pub const PUBLIC_CONFIG_CACHE_CONTROL: &str = "public, max-age=60";

/// Site branding and registration entry points exposed to unauthenticated
/// visitors; every value falls back to its default when unset.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicBranding {
    /// Site title shown in the UI.
    pub title: String,
    /// Site description text.
    pub description: String,
    /// URL of the favicon image.
    pub favicon_url: String,
    /// URL of the dark-theme wordmark image.
    pub wordmark_dark_url: String,
    /// URL of the light-theme wordmark image.
    pub wordmark_light_url: String,
    /// Public site URLs advertised to visitors.
    pub site_urls: Vec<String>,
    /// Whether open registration is currently allowed.
    pub allow_user_registration: bool,
    /// Whether passkey login is offered on sign-in surfaces.
    pub passkey_login_enabled: bool,
}

/// Captcha policy as applied to each unauthenticated flow.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicCaptchaConfig {
    /// Whether any captcha backend is configured at all.
    pub enabled: bool,
    /// Whether the login flow requires a captcha token.
    pub login_required: bool,
    /// Whether the registration flow requires a captcha token.
    pub register_required: bool,
    /// Whether accepting an invitation requires a captcha token.
    pub invitation_accept_required: bool,
    /// Whether resending a registration activation email requires a captcha.
    pub register_activation_resend_required: bool,
}

/// Yggdrasil server metadata that is safe to expose publicly; drives
/// launcher-facing defaults and upload form limits.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicYggdrasilConfig {
    /// Display name of the authentication server.
    pub server_name: String,
    /// Domains whose owners may use the skin host, per Yggdrasil metadata.
    pub skin_domains: Vec<String>,
    /// Public API root URLs advertised by the server.
    pub public_base_urls: Vec<String>,
    /// Whether profile-name login is allowed alongside account login.
    pub allow_profile_name_login: bool,
    /// Whether users may upload skins through the service.
    pub allow_skin_upload: bool,
    /// Whether users may upload capes through the service.
    pub allow_cape_upload: bool,
    /// Maximum accepted texture upload size in bytes.
    pub max_texture_upload_bytes: u64,
    /// Maximum accepted texture image area in pixels.
    pub max_texture_pixels: u64,
}

/// Texture library visibility policy as seen by visitors.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicTextureLibraryConfig {
    /// Whether the public texture library exists at all.
    pub enabled: bool,
    /// Whether submissions appear only after moderator approval.
    pub review_required: bool,
}

/// The full public runtime configuration payload served to the frontend;
/// `version` lets clients detect schema changes.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicFrontendConfig {
    /// Payload schema version; bumped on breaking shape changes.
    pub version: i32,
    /// Site branding and registration policy.
    pub branding: PublicBranding,
    /// Captcha requirements per flow.
    pub captcha: PublicCaptchaConfig,
    /// Yggdrasil server metadata and upload limits.
    pub yggdrasil: PublicYggdrasilConfig,
    /// Texture library visibility policy.
    pub texture_library: PublicTextureLibraryConfig,
}

/// Projects the runtime config into the public branding view, applying
/// defaults for unset values.
pub fn get_public_branding(state: &impl RuntimeConfigRuntimeState) -> PublicBranding {
    let runtime_config = state.runtime_config();
    let auth_policy = auth_runtime::RuntimeAuthPolicy::from_runtime_config(runtime_config);
    PublicBranding {
        title: branding::title_or_default(runtime_config),
        description: branding::description_or_default(runtime_config),
        favicon_url: branding::favicon_url_or_default(runtime_config),
        wordmark_dark_url: branding::wordmark_dark_url_or_default(runtime_config),
        wordmark_light_url: branding::wordmark_light_url_or_default(runtime_config),
        site_urls: site_url::public_site_urls(runtime_config),
        allow_user_registration: auth_policy.allow_user_registration,
        passkey_login_enabled: auth_policy.passkey_login_enabled,
    }
}

/// Projects the runtime config into the per-flow captcha requirements.
pub fn get_public_captcha_config(state: &impl RuntimeConfigRuntimeState) -> PublicCaptchaConfig {
    let policy = auth_runtime::RuntimeCaptchaPolicy::from_runtime_config(state.runtime_config());
    PublicCaptchaConfig {
        enabled: policy.enabled,
        login_required: policy.login_required(),
        register_required: policy.register_required(),
        invitation_accept_required: policy.invitation_accept_required(),
        register_activation_resend_required: policy.register_activation_resend_required(),
    }
}

/// Projects the runtime config into the public Yggdrasil metadata and
/// upload limits.
pub fn get_public_yggdrasil_config(
    state: &impl RuntimeConfigRuntimeState,
) -> PublicYggdrasilConfig {
    let policy = yggdrasil::RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    PublicYggdrasilConfig {
        server_name: policy.server_name,
        skin_domains: policy.skin_domains,
        public_base_urls: policy.public_base_urls,
        allow_profile_name_login: policy.allow_profile_name_login,
        allow_skin_upload: policy.allow_skin_upload,
        allow_cape_upload: policy.allow_cape_upload,
        max_texture_upload_bytes: policy.max_texture_upload_bytes,
        max_texture_pixels: policy.max_texture_pixels,
    }
}

/// Projects the runtime config into the public texture library policy.
pub fn get_public_texture_library_config(
    state: &impl RuntimeConfigRuntimeState,
) -> PublicTextureLibraryConfig {
    let policy =
        texture_library::RuntimeTextureLibraryPolicy::from_runtime_config(state.runtime_config());
    PublicTextureLibraryConfig {
        enabled: policy.enabled,
        review_required: policy.review_required,
    }
}

/// Assembles the complete public config payload served to the frontend.
pub fn get_public_frontend_config(state: &impl SharedRuntimeState) -> PublicFrontendConfig {
    PublicFrontendConfig {
        version: 1,
        branding: get_public_branding(state),
        captcha: get_public_captcha_config(state),
        yggdrasil: get_public_yggdrasil_config(state),
        texture_library: get_public_texture_library_config(state),
    }
}
