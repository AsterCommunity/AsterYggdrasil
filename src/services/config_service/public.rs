use crate::config::{auth_runtime, branding, site_url, yggdrasil};
use crate::runtime::{RuntimeConfigRuntimeState, SharedRuntimeState};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

pub const PUBLIC_CONFIG_CACHE_CONTROL: &str = "public, max-age=60";

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicBranding {
    pub title: String,
    pub description: String,
    pub favicon_url: String,
    pub wordmark_dark_url: String,
    pub wordmark_light_url: String,
    pub site_urls: Vec<String>,
    pub allow_user_registration: bool,
    pub passkey_login_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicYggdrasilConfig {
    pub server_name: String,
    pub skin_domains: Vec<String>,
    pub public_base_urls: Vec<String>,
    pub allow_profile_name_login: bool,
    pub allow_skin_upload: bool,
    pub allow_cape_upload: bool,
    pub max_texture_upload_bytes: u64,
    pub max_texture_pixels: u64,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicFrontendConfig {
    pub version: i32,
    pub branding: PublicBranding,
    pub yggdrasil: PublicYggdrasilConfig,
}

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

pub fn get_public_frontend_config(state: &impl SharedRuntimeState) -> PublicFrontendConfig {
    PublicFrontendConfig {
        version: 1,
        branding: get_public_branding(state),
        yggdrasil: get_public_yggdrasil_config(state),
    }
}
