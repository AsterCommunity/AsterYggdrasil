//! Runtime system configuration service.

mod actions;
mod public;
pub mod runtime;
mod schema;
mod system;

pub use crate::services::mail_template::list_template_variable_groups;
pub(crate) use actions::execute_action_with_audit;
pub use actions::{
    ConfigActionResult, ConfigActionType, ExecuteConfigActionInput, MAIL_CONFIG_ACTION_KEY,
    YGGDRASIL_CONFIG_ACTION_KEY,
};
pub use public::{
    PUBLIC_CONFIG_CACHE_CONTROL, PublicBranding, PublicCaptchaConfig, PublicFrontendConfig,
    PublicTextureLibraryConfig, PublicYggdrasilConfig, get_public_branding,
    get_public_captcha_config, get_public_frontend_config, get_public_texture_library_config,
    get_public_yggdrasil_config,
};
pub use schema::{ConfigSchemaItem, ConfigSchemaOption, get_schema};
pub use system::{
    SystemConfig, SystemConfigUpdateResult, SystemConfigWarning, bootstrap_insecure_cookies,
    ensure_defaults,
};
pub(crate) use system::{
    delete_with_audit, get_by_key, list_cursor, set_with_audit_and_visibility_result,
};
