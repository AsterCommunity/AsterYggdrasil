//! Runtime system configuration definitions.

use aster_forge_config::{
    ConfigCoreError, ConfigDefinition, ConfigRegistry, ConfigValueLookup, ConfigValueType,
    Result as ConfigCoreResult, normalize_bool_config_value,
};
use aster_forge_utils::bool_like::parse_bool_like;

use crate::config::{
    audit, auth_runtime, avatar, branding, cors, local_email_policy, mail, operations, site_url,
    texture_preview, yggdrasil,
};

pub const CONFIG_CATEGORY_SITE_PUBLIC: &str = "site.public";
pub const CONFIG_CATEGORY_SITE_BRANDING: &str = "site.branding";
pub const CONFIG_CATEGORY_AUTH_SESSION: &str = "auth.session";
pub const CONFIG_CATEGORY_AUTH_REGISTRATION: &str = "auth.registration";
pub const CONFIG_CATEGORY_AUTH_RECOVERY: &str = "auth.recovery";
pub const CONFIG_CATEGORY_AUTH_LOGIN: &str = "auth.login";
pub const CONFIG_CATEGORY_AUTH_CAPTCHA: &str = "auth.captcha";
pub const CONFIG_CATEGORY_AUTH_EMAIL_POLICY: &str = "auth.email_policy";
pub const CONFIG_CATEGORY_USER_AVATAR: &str = "user.avatar";
pub const CONFIG_CATEGORY_NETWORK_CORS: &str = "network.cors";
pub const CONFIG_CATEGORY_AUDIT_LOG: &str = "audit.log";
pub const CONFIG_CATEGORY_RUNTIME_TASKS: &str = "runtime.tasks";
pub const CONFIG_CATEGORY_RUNTIME_MAIL: &str = "runtime.mail";
pub const CONFIG_CATEGORY_RUNTIME_MAINTENANCE: &str = "runtime.maintenance";
pub const CONFIG_CATEGORY_MAIL_CONFIG: &str = "mail.config";
pub const CONFIG_CATEGORY_MAIL_TEMPLATE: &str = "mail.template";
pub const CONFIG_CATEGORY_YGGDRASIL_METADATA: &str = "yggdrasil.metadata";
pub const CONFIG_CATEGORY_YGGDRASIL_AUTH: &str = "yggdrasil.auth";
pub const CONFIG_CATEGORY_YGGDRASIL_TEXTURES: &str = "yggdrasil.textures";
pub const CONFIG_CATEGORY_YGGDRASIL_SIGNING: &str = "yggdrasil.signing";
pub const CONFIG_CATEGORY_TEXTURE_LIBRARY: &str = "texture.library";
pub const CONFIG_CATEGORY_TEXTURE_PREVIEW: &str = "texture.preview";

pub const SYSTEM_CONFIG_ALLOWED_CATEGORIES: &[&str] = &[
    CONFIG_CATEGORY_SITE_PUBLIC,
    CONFIG_CATEGORY_SITE_BRANDING,
    CONFIG_CATEGORY_AUTH_SESSION,
    CONFIG_CATEGORY_AUTH_REGISTRATION,
    CONFIG_CATEGORY_AUTH_RECOVERY,
    CONFIG_CATEGORY_AUTH_LOGIN,
    CONFIG_CATEGORY_AUTH_CAPTCHA,
    CONFIG_CATEGORY_AUTH_EMAIL_POLICY,
    CONFIG_CATEGORY_USER_AVATAR,
    CONFIG_CATEGORY_NETWORK_CORS,
    CONFIG_CATEGORY_AUDIT_LOG,
    CONFIG_CATEGORY_RUNTIME_TASKS,
    CONFIG_CATEGORY_RUNTIME_MAIL,
    CONFIG_CATEGORY_RUNTIME_MAINTENANCE,
    CONFIG_CATEGORY_MAIL_CONFIG,
    CONFIG_CATEGORY_MAIL_TEMPLATE,
    CONFIG_CATEGORY_YGGDRASIL_METADATA,
    CONFIG_CATEGORY_YGGDRASIL_AUTH,
    CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
    CONFIG_CATEGORY_YGGDRASIL_SIGNING,
    CONFIG_CATEGORY_TEXTURE_LIBRARY,
    CONFIG_CATEGORY_TEXTURE_PREVIEW,
];

pub const PUBLIC_SITE_URL_KEY: &str = "public_site_url";
pub const BRANDING_TITLE_KEY: &str = "branding_title";
pub const BRANDING_DESCRIPTION_KEY: &str = "branding_description";
pub const BRANDING_FAVICON_URL_KEY: &str = "branding_favicon_url";
pub const BRANDING_WORDMARK_DARK_URL_KEY: &str = "branding_wordmark_dark_url";
pub const BRANDING_WORDMARK_LIGHT_URL_KEY: &str = "branding_wordmark_light_url";

pub const AUTH_COOKIE_SECURE_KEY: &str = "auth_cookie_secure";
pub const AUTH_ACCESS_TOKEN_TTL_SECS_KEY: &str = "auth_access_token_ttl_secs";
pub const AUTH_REFRESH_TOKEN_TTL_SECS_KEY: &str = "auth_refresh_token_ttl_secs";
pub const AUTH_ALLOW_USER_REGISTRATION_KEY: &str = "auth_allow_user_registration";
pub const AUTH_REGISTER_ACTIVATION_ENABLED_KEY: &str = "auth_register_activation_enabled";
pub const AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY: &str = "auth_register_activation_ttl_secs";
pub const AUTH_USER_INVITATION_TTL_SECS_KEY: &str = "auth_user_invitation_ttl_secs";
pub const AUTH_CONTACT_CHANGE_TTL_SECS_KEY: &str = "auth_contact_change_ttl_secs";
pub const AUTH_PASSWORD_RESET_TTL_SECS_KEY: &str = "auth_password_reset_ttl_secs";
pub const AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY: &str =
    "auth_contact_verification_resend_cooldown_secs";
pub const AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY: &str =
    "auth_password_reset_request_cooldown_secs";
pub const AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY: &str = "auth_email_code_login_enabled";
pub const AUTH_PASSKEY_LOGIN_ENABLED_KEY: &str = "auth_passkey_login_enabled";
pub const AUTH_CAPTCHA_ENABLED_KEY: &str = "auth_captcha_enabled";
pub const AUTH_CAPTCHA_LOGIN_REQUIRED_KEY: &str = "auth_captcha_login_required";
pub const AUTH_CAPTCHA_REGISTER_REQUIRED_KEY: &str = "auth_captcha_register_required";
pub const AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY: &str =
    "auth_captcha_invitation_accept_required";
pub const AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY: &str =
    "auth_captcha_register_activation_resend_required";
pub const AUTH_CAPTCHA_PRESET_KEY: &str = "auth_captcha_preset";
pub const AUTH_CAPTCHA_TTL_SECS_KEY: &str = "auth_captcha_ttl_secs";
pub const AUTH_CAPTCHA_LENGTH_KEY: &str = "auth_captcha_length";
pub const AUTH_CAPTCHA_MAX_ATTEMPTS_KEY: &str = "auth_captcha_max_attempts";
pub const AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY: &str =
    "auth_email_code_login_allow_totp_fallback";
pub const AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY: &str = "auth_email_code_login_ttl_secs";
pub const AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY: &str =
    "auth_email_code_login_resend_cooldown_secs";
pub const AUTH_LOCAL_EMAIL_ALLOWLIST_KEY: &str = "auth_local_email_allowlist";
pub const AUTH_LOCAL_EMAIL_BLOCKLIST_KEY: &str = "auth_local_email_blocklist";

pub const GRAVATAR_BASE_URL_KEY: &str = "gravatar_base_url";

pub const CORS_ENABLED_KEY: &str = "cors_enabled";
pub const CORS_ALLOWED_ORIGINS_KEY: &str = "cors_allowed_origins";
pub const CORS_ALLOW_CREDENTIALS_KEY: &str = "cors_allow_credentials";
pub const CORS_MAX_AGE_SECS_KEY: &str = "cors_max_age_secs";

pub const AUDIT_LOG_ENABLED_KEY: &str = "audit_log_enabled";
pub const AUDIT_LOG_RETENTION_DAYS_KEY: &str = "audit_log_retention_days";
pub const AUDIT_LOG_RECORDED_ACTIONS_KEY: &str = "audit_log_recorded_actions";

pub const MAIL_SMTP_HOST_KEY: &str = "mail_smtp_host";
pub const MAIL_SMTP_PORT_KEY: &str = "mail_smtp_port";
pub const MAIL_SMTP_USERNAME_KEY: &str = "mail_smtp_username";
pub const MAIL_SMTP_PASSWORD_KEY: &str = "mail_smtp_password";
pub const MAIL_FROM_ADDRESS_KEY: &str = "mail_from_address";
pub const MAIL_FROM_NAME_KEY: &str = "mail_from_name";
pub const MAIL_SECURITY_KEY: &str = "mail_security";
pub const MAIL_TEMPLATE_REGISTER_ACTIVATION_SUBJECT_KEY: &str =
    "mail_template_register_activation_subject";
pub const MAIL_TEMPLATE_REGISTER_ACTIVATION_HTML_KEY: &str =
    "mail_template_register_activation_html";
pub const MAIL_TEMPLATE_CONTACT_CHANGE_CONFIRMATION_SUBJECT_KEY: &str =
    "mail_template_contact_change_confirmation_subject";
pub const MAIL_TEMPLATE_CONTACT_CHANGE_CONFIRMATION_HTML_KEY: &str =
    "mail_template_contact_change_confirmation_html";
pub const MAIL_TEMPLATE_PASSWORD_RESET_SUBJECT_KEY: &str = "mail_template_password_reset_subject";
pub const MAIL_TEMPLATE_PASSWORD_RESET_HTML_KEY: &str = "mail_template_password_reset_html";
pub const MAIL_TEMPLATE_PASSWORD_RESET_NOTICE_SUBJECT_KEY: &str =
    "mail_template_password_reset_notice_subject";
pub const MAIL_TEMPLATE_PASSWORD_RESET_NOTICE_HTML_KEY: &str =
    "mail_template_password_reset_notice_html";
pub const MAIL_TEMPLATE_CONTACT_CHANGE_NOTICE_SUBJECT_KEY: &str =
    "mail_template_contact_change_notice_subject";
pub const MAIL_TEMPLATE_CONTACT_CHANGE_NOTICE_HTML_KEY: &str =
    "mail_template_contact_change_notice_html";
pub const MAIL_TEMPLATE_EXTERNAL_AUTH_EMAIL_VERIFICATION_SUBJECT_KEY: &str =
    "mail_template_external_auth_email_verification_subject";
pub const MAIL_TEMPLATE_EXTERNAL_AUTH_EMAIL_VERIFICATION_HTML_KEY: &str =
    "mail_template_external_auth_email_verification_html";
pub const MAIL_TEMPLATE_LOGIN_EMAIL_CODE_SUBJECT_KEY: &str =
    "mail_template_login_email_code_subject";
pub const MAIL_TEMPLATE_LOGIN_EMAIL_CODE_HTML_KEY: &str = "mail_template_login_email_code_html";
pub const MAIL_TEMPLATE_USER_INVITATION_SUBJECT_KEY: &str = "mail_template_user_invitation_subject";
pub const MAIL_TEMPLATE_USER_INVITATION_HTML_KEY: &str = "mail_template_user_invitation_html";

pub const MAIL_OUTBOX_DISPATCH_INTERVAL_SECS_KEY: &str = "mail_outbox_dispatch_interval_secs";
pub const BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY: &str =
    "background_task_dispatch_interval_secs";
pub const BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY: &str =
    "background_task_dispatch_idle_max_interval_secs";
pub const BACKGROUND_TASK_MAX_CONCURRENCY_KEY: &str = "background_task_max_concurrency";
pub const BACKGROUND_TASK_MAX_ATTEMPTS_KEY: &str = "background_task_max_attempts";
pub const TASK_RETENTION_HOURS_KEY: &str = "task_retention_hours";
pub const TASK_LIST_MAX_LIMIT_KEY: &str = "task_list_max_limit";
pub const MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY: &str = "maintenance_cleanup_interval_secs";

pub const YGGDRASIL_SERVER_NAME_KEY: &str = "yggdrasil_server_name";
pub const YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY: &str = "yggdrasil_allow_profile_name_login";
pub const YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY: &str = "yggdrasil_allow_skin_upload";
pub const YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY: &str = "yggdrasil_allow_cape_upload";
pub const YGGDRASIL_ENABLE_PROFILE_KEY_KEY: &str = "yggdrasil_enable_profile_key";
pub const YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY: &str = "yggdrasil_enable_mojang_anti_features";
pub const YGGDRASIL_TOKEN_TTL_DAYS_KEY: &str = "yggdrasil_token_ttl_days";
pub const YGGDRASIL_MAX_ACTIVE_TOKENS_KEY: &str = "yggdrasil_max_active_tokens";
pub const YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY: &str = "yggdrasil_max_texture_upload_bytes";
pub const YGGDRASIL_MAX_TEXTURE_PIXELS_KEY: &str = "yggdrasil_max_texture_pixels";
pub const YGGDRASIL_SKIN_DOMAINS_KEY: &str = "yggdrasil_skin_domains";
pub const YGGDRASIL_PUBLIC_BASE_URL_KEY: &str = "yggdrasil_public_base_url";
pub const YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY: &str = "yggdrasil_texture_public_base_url";
pub const YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY: &str = "yggdrasil_signature_public_key";
pub const YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY: &str = "yggdrasil_signature_private_key";

pub const TEXTURE_LIBRARY_ENABLED_KEY: &str = "texture_library_enabled";
pub const TEXTURE_LIBRARY_REVIEW_REQUIRED_KEY: &str = "texture_library_review_required";
pub const TEXTURE_PREVIEW_ENGINE_KEY: &str = "texture_preview_engine";
pub const TEXTURE_PREVIEW_PROFILE_KEY: &str = "texture_preview_profile";
pub const TEXTURE_PREVIEW_WIDTH_KEY: &str = "texture_preview_width";
pub const TEXTURE_PREVIEW_HEIGHT_KEY: &str = "texture_preview_height";
pub const TEXTURE_PREVIEW_BACKGROUND_KEY: &str = "texture_preview_background";
pub const TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY: &str = "texture_preview_show_outer_layer";
pub const TEXTURE_PREVIEW_3D_SCALE_KEY: &str = "texture_preview_3d_scale";
pub const TEXTURE_PREVIEW_3D_PITCH_KEY: &str = "texture_preview_3d_pitch";
pub const TEXTURE_PREVIEW_3D_FRONT_YAW_KEY: &str = "texture_preview_3d_front_yaw";
pub const TEXTURE_PREVIEW_3D_BACK_YAW_KEY: &str = "texture_preview_3d_back_yaw";
pub const TEXTURE_PREVIEW_3D_SPACING_KEY: &str = "texture_preview_3d_spacing";
pub const TEXTURE_PREVIEW_3D_X_OFFSET_KEY: &str = "texture_preview_3d_x_offset";
pub const TEXTURE_PREVIEW_3D_Y_OFFSET_KEY: &str = "texture_preview_3d_y_offset";
pub const TEXTURE_PREVIEW_3D_CENTER_Y_KEY: &str = "texture_preview_3d_center_y";
pub const TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY: &str = "texture_preview_3d_supersampling";
pub const TEXTURE_PREVIEW_2D_PADDING_KEY: &str = "texture_preview_2d_padding";
pub const TEXTURE_PREVIEW_2D_SPACING_KEY: &str = "texture_preview_2d_spacing";

pub const DEPRECATED_AVATAR_DIR_KEY: &str = "avatar_dir";

pub const DEPRECATED_SYSTEM_CONFIG_KEYS: &[&str] = &[DEPRECATED_AVATAR_DIR_KEY];

pub type ConfigDef = ConfigDefinition;

fn empty_origin_list_default() -> String {
    "[]".to_string()
}

fn map_normalizer_result(result: crate::errors::Result<String>) -> ConfigCoreResult<String> {
    result.map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))
}

fn normalize_recorded_actions(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(audit::normalize_recorded_actions_config_value(value))
}

fn normalize_cookie_secure(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_cookie_secure_config_value(value))
}

fn normalize_allow_user_registration(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_allow_user_registration_config_value(value))
}

fn normalize_register_activation_enabled(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_register_activation_enabled_config_value(value))
}

fn normalize_auth_bool(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_auth_bool_config_value(key, value))
}

fn normalize_email_code_login_bool(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_email_code_login_bool_config_value(
        key, value,
    ))
}

fn normalize_token_ttl(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_token_ttl_config_value(key, value))
}

fn normalize_captcha_length(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_captcha_length_config_value(value))
}

fn normalize_captcha_preset(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_captcha_preset_config_value(value))
}

fn normalize_captcha_max_attempts(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(auth_runtime::normalize_captcha_max_attempts_config_value(
        value,
    ))
}

fn normalize_local_email_policy(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(local_email_policy::normalize_local_email_policy_config_value(key, value))
}

fn normalize_gravatar_base_url(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(avatar::normalize_gravatar_base_url_config_value(value))
}

fn normalize_smtp_host(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_smtp_host_config_value(value))
}

fn normalize_smtp_port(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_smtp_port_config_value(value))
}

fn normalize_mail_address(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_mail_address_config_value(value))
}

fn normalize_mail_name(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_mail_name_config_value(value))
}

fn normalize_mail_security(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_mail_security_config_value(value))
}

fn normalize_mail_template_subject(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_mail_template_subject_config_value(
        key, value,
    ))
}

fn normalize_mail_template_body(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(mail::normalize_mail_template_body_config_value(key, value))
}

fn normalize_operation_interval(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(operations::normalize_interval_config_value(key, value))
}

fn normalize_cors_enabled(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(cors::normalize_enabled_config_value(value))
}

fn normalize_cors_allowed_origins(
    lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    let normalized = cors::normalize_allowed_origins_config_value(value)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    let parsed = cors::parse_allowed_origins_value(&normalized)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    let allow_credentials = lookup
        .get_config_value(CORS_ALLOW_CREDENTIALS_KEY)
        .and_then(|raw| parse_bool_like(&raw))
        .unwrap_or(cors::DEFAULT_CORS_ALLOW_CREDENTIALS);
    cors::validate_runtime_cors_combination(&parsed, allow_credentials)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    Ok(normalized)
}

fn normalize_cors_allow_credentials(
    lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    let normalized = cors::normalize_allow_credentials_config_value(value)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    let allow_credentials = normalized == "true";
    let current_origins = lookup
        .get_config_value(CORS_ALLOWED_ORIGINS_KEY)
        .unwrap_or_default();
    let parsed = cors::parse_allowed_origins_value(&current_origins)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    cors::validate_runtime_cors_combination(&parsed, allow_credentials)
        .map_err(|error| ConfigCoreError::invalid_value(error.message().to_string()))?;
    Ok(normalized)
}

fn normalize_cors_max_age(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(cors::normalize_max_age_config_value(value))
}

fn normalize_public_site_url(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(site_url::normalize_public_site_url_config_value(value))
}

fn normalize_yggdrasil(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(yggdrasil::normalize_yggdrasil_config_value(key, value))
}

fn normalize_texture_preview(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(texture_preview::normalize_texture_preview_config_value(
        key, value,
    ))
}

fn normalize_texture_library_bool(
    _lookup: &dyn ConfigValueLookup,
    key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    normalize_bool_config_value(key, value)
}

fn normalize_branding_title(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(branding::normalize_title_config_value(value))
}

fn normalize_branding_description(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(branding::normalize_description_config_value(value))
}

fn normalize_branding_favicon_url(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(branding::normalize_favicon_url_config_value(value))
}

fn normalize_branding_wordmark_dark_url(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(branding::normalize_wordmark_dark_url_config_value(value))
}

fn normalize_branding_wordmark_light_url(
    _lookup: &dyn ConfigValueLookup,
    _key: &str,
    value: &str,
) -> ConfigCoreResult<String> {
    map_normalizer_result(branding::normalize_wordmark_light_url_config_value(value))
}

pub static ALL_CONFIGS: &[ConfigDefinition] = &[
    ConfigDefinition {
        key: PUBLIC_SITE_URL_KEY,
        label_i18n_key: "settings_item_public_site_url_label",
        description_i18n_key: "settings_item_public_site_url_desc",
        value_type: ConfigValueType::StringArray,
        default_fn: empty_origin_list_default,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_PUBLIC,
        description: "Public origins used to build externally visible application URLs",
        normalize_fn: Some(normalize_public_site_url),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BRANDING_TITLE_KEY,
        label_i18n_key: "settings_item_branding_title_label",
        description_i18n_key: "settings_item_branding_title_desc",
        value_type: ConfigValueType::String,
        default_fn: || "AsterYggdrasil".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_BRANDING,
        description: "Application title shown in the embedded frontend",
        normalize_fn: Some(normalize_branding_title),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BRANDING_DESCRIPTION_KEY,
        label_i18n_key: "settings_item_branding_description_label",
        description_i18n_key: "settings_item_branding_description_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            "Self-hosted Minecraft skin site and Yggdrasil authentication server.".to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_BRANDING,
        description: "Short application description shown in public UI contexts",
        normalize_fn: Some(normalize_branding_description),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BRANDING_FAVICON_URL_KEY,
        label_i18n_key: "settings_item_branding_favicon_url_label",
        description_i18n_key: "settings_item_branding_favicon_url_desc",
        value_type: ConfigValueType::String,
        default_fn: || "/favicon.svg".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_BRANDING,
        description: "Favicon URL for the embedded frontend",
        normalize_fn: Some(normalize_branding_favicon_url),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BRANDING_WORDMARK_DARK_URL_KEY,
        label_i18n_key: "settings_item_branding_wordmark_dark_url_label",
        description_i18n_key: "settings_item_branding_wordmark_dark_url_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_BRANDING,
        description: "Optional dark wordmark URL for branded frontend shells",
        normalize_fn: Some(normalize_branding_wordmark_dark_url),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BRANDING_WORDMARK_LIGHT_URL_KEY,
        label_i18n_key: "settings_item_branding_wordmark_light_url_label",
        description_i18n_key: "settings_item_branding_wordmark_light_url_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_SITE_BRANDING,
        description: "Optional light wordmark URL for branded frontend shells",
        normalize_fn: Some(normalize_branding_wordmark_light_url),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_COOKIE_SECURE_KEY,
        label_i18n_key: "settings_item_auth_cookie_secure_label",
        description_i18n_key: "settings_item_auth_cookie_secure_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_SESSION,
        description: "Whether authentication cookies require HTTPS",
        normalize_fn: Some(normalize_cookie_secure),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_ACCESS_TOKEN_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_access_token_ttl_secs_label",
        description_i18n_key: "settings_item_auth_access_token_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "900".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_SESSION,
        description: "Access token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_refresh_token_ttl_secs_label",
        description_i18n_key: "settings_item_auth_refresh_token_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "604800".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_SESSION,
        description: "Refresh token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_ALLOW_USER_REGISTRATION_KEY,
        label_i18n_key: "settings_item_auth_allow_user_registration_label",
        description_i18n_key: "settings_item_auth_allow_user_registration_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_REGISTRATION,
        description: "Allow users to register after the initial setup",
        normalize_fn: Some(normalize_allow_user_registration),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        label_i18n_key: "settings_item_auth_register_activation_enabled_label",
        description_i18n_key: "settings_item_auth_register_activation_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_REGISTRATION,
        description: "Require activation before newly registered users can sign in",
        normalize_fn: Some(normalize_register_activation_enabled),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_register_activation_ttl_secs_label",
        description_i18n_key: "settings_item_auth_register_activation_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "86400".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_REGISTRATION,
        description: "Registration activation token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_USER_INVITATION_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_user_invitation_ttl_secs_label",
        description_i18n_key: "settings_item_auth_user_invitation_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || {
            crate::config::auth_runtime::DEFAULT_AUTH_USER_INVITATION_TTL_SECS.to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_REGISTRATION,
        description: "User invitation token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CONTACT_CHANGE_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_contact_change_ttl_secs_label",
        description_i18n_key: "settings_item_auth_contact_change_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "86400".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_RECOVERY,
        description: "Contact change confirmation token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_PASSWORD_RESET_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_password_reset_ttl_secs_label",
        description_i18n_key: "settings_item_auth_password_reset_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "3600".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_RECOVERY,
        description: "Password reset token lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY,
        label_i18n_key: "settings_item_auth_contact_verification_resend_cooldown_secs_label",
        description_i18n_key: "settings_item_auth_contact_verification_resend_cooldown_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "60".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_RECOVERY,
        description: "Minimum cooldown between contact verification sends in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY,
        label_i18n_key: "settings_item_auth_password_reset_request_cooldown_secs_label",
        description_i18n_key: "settings_item_auth_password_reset_request_cooldown_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "60".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_RECOVERY,
        description: "Minimum cooldown between password reset requests in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY,
        label_i18n_key: "settings_item_auth_email_code_login_enabled_label",
        description_i18n_key: "settings_item_auth_email_code_login_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_LOGIN,
        description: "Enable email code login when mail plumbing is provided by the project",
        normalize_fn: Some(normalize_email_code_login_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_PASSKEY_LOGIN_ENABLED_KEY,
        label_i18n_key: "settings_item_auth_passkey_login_enabled_label",
        description_i18n_key: "settings_item_auth_passkey_login_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_LOGIN,
        description: "Enable passkey login when passkey plumbing is provided by the project",
        normalize_fn: Some(normalize_email_code_login_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_ENABLED_KEY,
        label_i18n_key: "settings_item_auth_captcha_enabled_label",
        description_i18n_key: "settings_item_auth_captcha_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Enable visual captcha challenges for selected public authentication flows",
        normalize_fn: Some(normalize_auth_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_LOGIN_REQUIRED_KEY,
        label_i18n_key: "settings_item_auth_captcha_login_required_label",
        description_i18n_key: "settings_item_auth_captcha_login_required_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Require captcha verification for local password login when captcha is enabled",
        normalize_fn: Some(normalize_auth_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_REGISTER_REQUIRED_KEY,
        label_i18n_key: "settings_item_auth_captcha_register_required_label",
        description_i18n_key: "settings_item_auth_captcha_register_required_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Require captcha verification for public self-registration when captcha is enabled",
        normalize_fn: Some(normalize_auth_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY,
        label_i18n_key: "settings_item_auth_captcha_invitation_accept_required_label",
        description_i18n_key: "settings_item_auth_captcha_invitation_accept_required_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Require captcha verification when accepting a user invitation",
        normalize_fn: Some(normalize_auth_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY,
        label_i18n_key: "settings_item_auth_captcha_register_activation_resend_required_label",
        description_i18n_key: "settings_item_auth_captcha_register_activation_resend_required_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Require captcha verification before resending registration activation email",
        normalize_fn: Some(normalize_auth_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_PRESET_KEY,
        label_i18n_key: "settings_item_auth_captcha_preset_label",
        description_i18n_key: "settings_item_auth_captcha_preset_desc",
        value_type: ConfigValueType::StringEnum,
        default_fn: || "balanced".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Captcha rendering preset controlling distortion and visual noise",
        normalize_fn: Some(normalize_captcha_preset),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_captcha_ttl_secs_label",
        description_i18n_key: "settings_item_auth_captcha_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "120".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Captcha challenge lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_LENGTH_KEY,
        label_i18n_key: "settings_item_auth_captcha_length_label",
        description_i18n_key: "settings_item_auth_captcha_length_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "5".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Number of characters in generated captcha challenges",
        normalize_fn: Some(normalize_captcha_length),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_CAPTCHA_MAX_ATTEMPTS_KEY,
        label_i18n_key: "settings_item_auth_captcha_max_attempts_label",
        description_i18n_key: "settings_item_auth_captcha_max_attempts_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "3".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_CAPTCHA,
        description: "Maximum answer attempts before a captcha challenge is consumed",
        normalize_fn: Some(normalize_captcha_max_attempts),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY,
        label_i18n_key: "settings_item_auth_email_code_login_allow_totp_fallback_label",
        description_i18n_key: "settings_item_auth_email_code_login_allow_totp_fallback_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_LOGIN,
        description: "Allow email code fallback for TOTP challenges",
        normalize_fn: Some(normalize_email_code_login_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY,
        label_i18n_key: "settings_item_auth_email_code_login_ttl_secs_label",
        description_i18n_key: "settings_item_auth_email_code_login_ttl_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "600".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_LOGIN,
        description: "Email login code lifetime in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY,
        label_i18n_key: "settings_item_auth_email_code_login_resend_cooldown_secs_label",
        description_i18n_key: "settings_item_auth_email_code_login_resend_cooldown_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "60".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_LOGIN,
        description: "Minimum cooldown between email login code sends in seconds",
        normalize_fn: Some(normalize_token_ttl),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
        label_i18n_key: "settings_item_auth_local_email_allowlist_label",
        description_i18n_key: "settings_item_auth_local_email_allowlist_desc",
        value_type: ConfigValueType::StringArray,
        default_fn: empty_origin_list_default,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_EMAIL_POLICY,
        description: "Allowed local-account email addresses and exact ASCII domains. Empty means no allowlist restriction",
        normalize_fn: Some(normalize_local_email_policy),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
        label_i18n_key: "settings_item_auth_local_email_blocklist_label",
        description_i18n_key: "settings_item_auth_local_email_blocklist_desc",
        value_type: ConfigValueType::StringArray,
        default_fn: empty_origin_list_default,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUTH_EMAIL_POLICY,
        description: "Blocked local-account email addresses and exact ASCII domains. Blocklist wins over allowlist",
        normalize_fn: Some(normalize_local_email_policy),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: GRAVATAR_BASE_URL_KEY,
        label_i18n_key: "settings_item_gravatar_base_url_label",
        description_i18n_key: "settings_item_gravatar_base_url_desc",
        value_type: ConfigValueType::String,
        default_fn: || "https://www.gravatar.com/avatar".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_USER_AVATAR,
        description: "Gravatar avatar base URL; change to a proxy or mirror if needed",
        normalize_fn: Some(normalize_gravatar_base_url),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: CORS_ENABLED_KEY,
        label_i18n_key: "settings_item_cors_enabled_label",
        description_i18n_key: "settings_item_cors_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_NETWORK_CORS,
        description: "Enable runtime CORS handling for cross-origin browser requests",
        normalize_fn: Some(normalize_cors_enabled),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: CORS_ALLOWED_ORIGINS_KEY,
        label_i18n_key: "settings_item_cors_allowed_origins_label",
        description_i18n_key: "settings_item_cors_allowed_origins_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_NETWORK_CORS,
        description: "Comma-separated CORS origin whitelist",
        normalize_fn: Some(normalize_cors_allowed_origins),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: CORS_ALLOW_CREDENTIALS_KEY,
        label_i18n_key: "settings_item_cors_allow_credentials_label",
        description_i18n_key: "settings_item_cors_allow_credentials_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "false".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_NETWORK_CORS,
        description: "Whether CORS responses include Access-Control-Allow-Credentials",
        normalize_fn: Some(normalize_cors_allow_credentials),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: CORS_MAX_AGE_SECS_KEY,
        label_i18n_key: "settings_item_cors_max_age_secs_label",
        description_i18n_key: "settings_item_cors_max_age_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "3600".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_NETWORK_CORS,
        description: "CORS preflight cache duration in seconds",
        normalize_fn: Some(normalize_cors_max_age),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUDIT_LOG_ENABLED_KEY,
        label_i18n_key: "settings_item_audit_log_enabled_label",
        description_i18n_key: "settings_item_audit_log_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUDIT_LOG,
        description: "Enable audit log recording",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUDIT_LOG_RETENTION_DAYS_KEY,
        label_i18n_key: "settings_item_audit_log_retention_days_label",
        description_i18n_key: "settings_item_audit_log_retention_days_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "180".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUDIT_LOG,
        description: "Audit log retention in days",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: AUDIT_LOG_RECORDED_ACTIONS_KEY,
        label_i18n_key: "settings_item_audit_log_recorded_actions_label",
        description_i18n_key: "settings_item_audit_log_recorded_actions_desc",
        value_type: ConfigValueType::StringEnumSet,
        default_fn: crate::config::audit::default_recorded_actions_value,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_AUDIT_LOG,
        description: "Audit action allowlist stored as a JSON string array",
        normalize_fn: Some(normalize_recorded_actions),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_SMTP_HOST_KEY,
        label_i18n_key: "settings_item_mail_smtp_host_label",
        description_i18n_key: "settings_item_mail_smtp_host_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "SMTP server hostname used for transactional email delivery",
        normalize_fn: Some(normalize_smtp_host),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_SMTP_PORT_KEY,
        label_i18n_key: "settings_item_mail_smtp_port_label",
        description_i18n_key: "settings_item_mail_smtp_port_desc",
        value_type: ConfigValueType::Number,
        default_fn: || crate::config::mail::DEFAULT_MAIL_SMTP_PORT.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "SMTP server port used for transactional email delivery",
        normalize_fn: Some(normalize_smtp_port),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_SMTP_USERNAME_KEY,
        label_i18n_key: "settings_item_mail_smtp_username_label",
        description_i18n_key: "settings_item_mail_smtp_username_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "SMTP username for authenticated mail delivery",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_SMTP_PASSWORD_KEY,
        label_i18n_key: "settings_item_mail_smtp_password_label",
        description_i18n_key: "settings_item_mail_smtp_password_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: true,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "SMTP password for authenticated mail delivery",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_FROM_ADDRESS_KEY,
        label_i18n_key: "settings_item_mail_from_address_label",
        description_i18n_key: "settings_item_mail_from_address_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "From address used for transactional email delivery",
        normalize_fn: Some(normalize_mail_address),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_FROM_NAME_KEY,
        label_i18n_key: "settings_item_mail_from_name_label",
        description_i18n_key: "settings_item_mail_from_name_desc",
        value_type: ConfigValueType::String,
        default_fn: || "AsterYggdrasil".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "Display name used for transactional email delivery",
        normalize_fn: Some(normalize_mail_name),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_SECURITY_KEY,
        label_i18n_key: "settings_item_mail_security_label",
        description_i18n_key: "settings_item_mail_security_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || crate::config::mail::DEFAULT_MAIL_SECURITY.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_CONFIG,
        description: "Whether SMTP uses encryption. Port 465 uses implicit TLS; other ports use STARTTLS when enabled",
        normalize_fn: Some(normalize_mail_security),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_REGISTER_ACTIVATION_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_register_activation_subject_label",
        description_i18n_key: "settings_item_mail_template_register_activation_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::RegisterActivation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for registration activation emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_REGISTER_ACTIVATION_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_register_activation_html_label",
        description_i18n_key: "settings_item_mail_template_register_activation_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::RegisterActivation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for registration activation emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_CONTACT_CHANGE_CONFIRMATION_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_contact_change_confirmation_subject_label",
        description_i18n_key: "settings_item_mail_template_contact_change_confirmation_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::ContactChangeConfirmation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for contact change confirmation emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_CONTACT_CHANGE_CONFIRMATION_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_contact_change_confirmation_html_label",
        description_i18n_key: "settings_item_mail_template_contact_change_confirmation_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::ContactChangeConfirmation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for contact change confirmation emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_PASSWORD_RESET_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_password_reset_subject_label",
        description_i18n_key: "settings_item_mail_template_password_reset_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::PasswordReset,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for password reset emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_PASSWORD_RESET_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_password_reset_html_label",
        description_i18n_key: "settings_item_mail_template_password_reset_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::PasswordReset,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for password reset emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_PASSWORD_RESET_NOTICE_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_password_reset_notice_subject_label",
        description_i18n_key: "settings_item_mail_template_password_reset_notice_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::PasswordResetNotice,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for password reset notice emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_PASSWORD_RESET_NOTICE_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_password_reset_notice_html_label",
        description_i18n_key: "settings_item_mail_template_password_reset_notice_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::PasswordResetNotice,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for password reset notice emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_CONTACT_CHANGE_NOTICE_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_contact_change_notice_subject_label",
        description_i18n_key: "settings_item_mail_template_contact_change_notice_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::ContactChangeNotice,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for contact change notice emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_CONTACT_CHANGE_NOTICE_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_contact_change_notice_html_label",
        description_i18n_key: "settings_item_mail_template_contact_change_notice_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::ContactChangeNotice,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for contact change notice emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_EXTERNAL_AUTH_EMAIL_VERIFICATION_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_external_auth_email_verification_subject_label",
        description_i18n_key: "settings_item_mail_template_external_auth_email_verification_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::ExternalAuthEmailVerification,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for external auth email verification emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_EXTERNAL_AUTH_EMAIL_VERIFICATION_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_external_auth_email_verification_html_label",
        description_i18n_key: "settings_item_mail_template_external_auth_email_verification_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::ExternalAuthEmailVerification,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for external auth email verification emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_LOGIN_EMAIL_CODE_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_login_email_code_subject_label",
        description_i18n_key: "settings_item_mail_template_login_email_code_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::LoginEmailCode,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for login email code messages",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_LOGIN_EMAIL_CODE_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_login_email_code_html_label",
        description_i18n_key: "settings_item_mail_template_login_email_code_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::LoginEmailCode,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for login email code messages",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_USER_INVITATION_SUBJECT_KEY,
        label_i18n_key: "settings_item_mail_template_user_invitation_subject_label",
        description_i18n_key: "settings_item_mail_template_user_invitation_subject_desc",
        value_type: ConfigValueType::String,
        default_fn: || {
            crate::config::mail::default_template_subject(
                crate::types::MailTemplateCode::UserInvitation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "Subject template for user invitation emails",
        normalize_fn: Some(normalize_mail_template_subject),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_TEMPLATE_USER_INVITATION_HTML_KEY,
        label_i18n_key: "settings_item_mail_template_user_invitation_html_label",
        description_i18n_key: "settings_item_mail_template_user_invitation_html_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: || {
            crate::config::mail::default_template_html(
                crate::types::MailTemplateCode::UserInvitation,
            )
            .to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_MAIL_TEMPLATE,
        description: "HTML template for user invitation emails",
        normalize_fn: Some(normalize_mail_template_body),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_SERVER_NAME_KEY,
        label_i18n_key: "settings_item_yggdrasil_server_name_label",
        description_i18n_key: "settings_item_yggdrasil_server_name_desc",
        value_type: ConfigValueType::String,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_SERVER_NAME.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_METADATA,
        description: "Server name exposed in authlib-injector metadata",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
        label_i18n_key: "settings_item_yggdrasil_allow_profile_name_login_label",
        description_i18n_key: "settings_item_yggdrasil_allow_profile_name_login_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || {
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN.to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_AUTH,
        description: "Allow launcher login using Minecraft profile names",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY,
        label_i18n_key: "settings_item_yggdrasil_allow_skin_upload_label",
        description_i18n_key: "settings_item_yggdrasil_allow_skin_upload_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_ALLOW_SKIN_UPLOAD.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Allow Minecraft profiles to upload skin textures",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY,
        label_i18n_key: "settings_item_yggdrasil_allow_cape_upload_label",
        description_i18n_key: "settings_item_yggdrasil_allow_cape_upload_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_ALLOW_CAPE_UPLOAD.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Allow Minecraft profiles to upload cape textures",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_ENABLE_PROFILE_KEY_KEY,
        label_i18n_key: "settings_item_yggdrasil_enable_profile_key_label",
        description_i18n_key: "settings_item_yggdrasil_enable_profile_key_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_ENABLE_PROFILE_KEY.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_METADATA,
        description: "Expose authlib-injector profile key support and serve Minecraft services player certificates",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY,
        label_i18n_key: "settings_item_yggdrasil_enable_mojang_anti_features_label",
        description_i18n_key: "settings_item_yggdrasil_enable_mojang_anti_features_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || {
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES.to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_METADATA,
        description: "Expose authlib-injector Minecraft services anti-feature policy endpoints",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_TOKEN_TTL_DAYS_KEY,
        label_i18n_key: "settings_item_yggdrasil_token_ttl_days_label",
        description_i18n_key: "settings_item_yggdrasil_token_ttl_days_desc",
        value_type: ConfigValueType::Number,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_TOKEN_TTL_DAYS.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_AUTH,
        description: "Launcher access token lifetime in days",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_MAX_ACTIVE_TOKENS_KEY,
        label_i18n_key: "settings_item_yggdrasil_max_active_tokens_label",
        description_i18n_key: "settings_item_yggdrasil_max_active_tokens_desc",
        value_type: ConfigValueType::Number,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_ACTIVE_TOKENS.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_AUTH,
        description: "Maximum active launcher tokens retained per user",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        label_i18n_key: "settings_item_yggdrasil_max_texture_upload_bytes_label",
        description_i18n_key: "settings_item_yggdrasil_max_texture_upload_bytes_desc",
        value_type: ConfigValueType::Number,
        default_fn: || {
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES.to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Maximum uploaded texture file size in bytes, enforced while streaming multipart data",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        label_i18n_key: "settings_item_yggdrasil_max_texture_pixels_label",
        description_i18n_key: "settings_item_yggdrasil_max_texture_pixels_desc",
        value_type: ConfigValueType::Number,
        default_fn: || crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS.to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Maximum uploaded texture pixel count checked from PNG dimensions before full decode",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_SKIN_DOMAINS_KEY,
        label_i18n_key: "settings_item_yggdrasil_skin_domains_label",
        description_i18n_key: "settings_item_yggdrasil_skin_domains_desc",
        value_type: ConfigValueType::StringArray,
        default_fn: crate::config::yggdrasil::default_skin_domains_config,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Texture domain whitelist exposed in authlib-injector metadata",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_PUBLIC_BASE_URL_KEY,
        label_i18n_key: "settings_item_yggdrasil_public_base_url_label",
        description_i18n_key: "settings_item_yggdrasil_public_base_url_desc",
        value_type: ConfigValueType::StringArray,
        default_fn: empty_origin_list_default,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Externally reachable base URL candidates used to build Yggdrasil texture URLs",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_TEXTURE_PUBLIC_BASE_URL_KEY,
        label_i18n_key: "settings_item_yggdrasil_texture_public_base_url_label",
        description_i18n_key: "settings_item_yggdrasil_texture_public_base_url_desc",
        value_type: ConfigValueType::String,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_TEXTURES,
        description: "Optional public object-storage or CDN base URL used for uploaded texture objects. When empty, texture URLs use the Yggdrasil API route",
        normalize_fn: Some(normalize_yggdrasil),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
        label_i18n_key: "settings_item_yggdrasil_signature_public_key_label",
        description_i18n_key: "settings_item_yggdrasil_signature_public_key_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_YGGDRASIL_SIGNING,
        description: "PEM public key exposed in authlib-injector metadata when no signing private key is configured; when a private key exists, metadata derives the public key from it",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        label_i18n_key: "settings_item_yggdrasil_signature_private_key_label",
        description_i18n_key: "settings_item_yggdrasil_signature_private_key_desc",
        value_type: ConfigValueType::Multiline,
        default_fn: String::new,
        requires_restart: false,
        is_sensitive: true,
        category: CONFIG_CATEGORY_YGGDRASIL_SIGNING,
        description: "PEM RSA private key used to sign Yggdrasil texture properties. Rotate via config action; new profile/hasJoined responses are signed with the current key",
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_LIBRARY_ENABLED_KEY,
        label_i18n_key: "settings_item_texture_library_enabled_label",
        description_i18n_key: "settings_item_texture_library_enabled_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_LIBRARY,
        description: "Enable the public texture library and related user submission controls",
        normalize_fn: Some(normalize_texture_library_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_LIBRARY_REVIEW_REQUIRED_KEY,
        label_i18n_key: "settings_item_texture_library_review_required_label",
        description_i18n_key: "settings_item_texture_library_review_required_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_LIBRARY,
        description: "Require administrator review before submitted wardrobe textures are published to the public library",
        normalize_fn: Some(normalize_texture_library_bool),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_ENGINE_KEY,
        label_i18n_key: "settings_item_texture_preview_engine_label",
        description_i18n_key: "settings_item_texture_preview_engine_desc",
        value_type: ConfigValueType::StringEnum,
        default_fn: || "skin-3d".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "Texture preview render engine: skin-3d or skin-2d",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_PROFILE_KEY,
        label_i18n_key: "settings_item_texture_preview_profile_label",
        description_i18n_key: "settings_item_texture_preview_profile_desc",
        value_type: ConfigValueType::StringEnum,
        default_fn: || "default".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview quality profile: fast, default, or quality",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_WIDTH_KEY,
        label_i18n_key: "settings_item_texture_preview_width_label",
        description_i18n_key: "settings_item_texture_preview_width_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "430".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "Rendered texture preview width in pixels",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_HEIGHT_KEY,
        label_i18n_key: "settings_item_texture_preview_height_label",
        description_i18n_key: "settings_item_texture_preview_height_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "430".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "Rendered texture preview height in pixels",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_BACKGROUND_KEY,
        label_i18n_key: "settings_item_texture_preview_background_label",
        description_i18n_key: "settings_item_texture_preview_background_desc",
        value_type: ConfigValueType::String,
        default_fn: || "transparent".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "Texture preview background: transparent, none, white, black, #RRGGBB, or #RRGGBBAA",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY,
        label_i18n_key: "settings_item_texture_preview_show_outer_layer_label",
        description_i18n_key: "settings_item_texture_preview_show_outer_layer_desc",
        value_type: ConfigValueType::Boolean,
        default_fn: || "true".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "Whether texture previews render skin second-layer regions",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_SCALE_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_scale_label",
        description_i18n_key: "settings_item_texture_preview_3d_scale_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "11.5".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview orthographic scale",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_PITCH_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_pitch_label",
        description_i18n_key: "settings_item_texture_preview_3d_pitch_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "30".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview camera pitch in degrees",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_FRONT_YAW_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_front_yaw_label",
        description_i18n_key: "settings_item_texture_preview_3d_front_yaw_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "-45".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview front view yaw in degrees",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_BACK_YAW_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_back_yaw_label",
        description_i18n_key: "settings_item_texture_preview_3d_back_yaw_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "135".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview back view yaw in degrees",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_SPACING_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_spacing_label",
        description_i18n_key: "settings_item_texture_preview_3d_spacing_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "35".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview spacing between front and back views",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_X_OFFSET_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_x_offset_label",
        description_i18n_key: "settings_item_texture_preview_3d_x_offset_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "0".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview horizontal offset in pixels",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_Y_OFFSET_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_y_offset_label",
        description_i18n_key: "settings_item_texture_preview_3d_y_offset_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "-24".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview vertical offset in pixels",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_CENTER_Y_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_center_y_label",
        description_i18n_key: "settings_item_texture_preview_3d_center_y_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "0.56".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview vertical center anchor ratio",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY,
        label_i18n_key: "settings_item_texture_preview_3d_supersampling_label",
        description_i18n_key: "settings_item_texture_preview_3d_supersampling_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "2".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "3D texture preview supersampling factor from 1 to 4",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_2D_PADDING_KEY,
        label_i18n_key: "settings_item_texture_preview_2d_padding_label",
        description_i18n_key: "settings_item_texture_preview_2d_padding_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "24".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "2D texture preview padding in pixels",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TEXTURE_PREVIEW_2D_SPACING_KEY,
        label_i18n_key: "settings_item_texture_preview_2d_spacing_label",
        description_i18n_key: "settings_item_texture_preview_2d_spacing_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "35".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_TEXTURE_PREVIEW,
        description: "2D texture preview spacing between front and back views",
        normalize_fn: Some(normalize_texture_preview),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAIL_OUTBOX_DISPATCH_INTERVAL_SECS_KEY,
        label_i18n_key: "settings_item_mail_outbox_dispatch_interval_secs_label",
        description_i18n_key: "settings_item_mail_outbox_dispatch_interval_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || {
            crate::config::operations::DEFAULT_MAIL_OUTBOX_DISPATCH_INTERVAL_SECS.to_string()
        },
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_MAIL,
        description: "Seconds between mail outbox dispatch polls",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BACKGROUND_TASK_DISPATCH_INTERVAL_SECS_KEY,
        label_i18n_key: "settings_item_background_task_dispatch_interval_secs_label",
        description_i18n_key: "settings_item_background_task_dispatch_interval_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "5".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "Default interval for project background task dispatch loops",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BACKGROUND_TASK_DISPATCH_IDLE_MAX_INTERVAL_SECS_KEY,
        label_i18n_key: "settings_item_background_task_dispatch_idle_max_interval_secs_label",
        description_i18n_key: "settings_item_background_task_dispatch_idle_max_interval_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "60".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "Maximum idle backoff interval for background task dispatch loops",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        label_i18n_key: "settings_item_background_task_max_concurrency_label",
        description_i18n_key: "settings_item_background_task_max_concurrency_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "4".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "Maximum number of generic background tasks processed concurrently",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: BACKGROUND_TASK_MAX_ATTEMPTS_KEY,
        label_i18n_key: "settings_item_background_task_max_attempts_label",
        description_i18n_key: "settings_item_background_task_max_attempts_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "3".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "Default max attempts for retryable project background tasks",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TASK_RETENTION_HOURS_KEY,
        label_i18n_key: "settings_item_task_retention_hours_label",
        description_i18n_key: "settings_item_task_retention_hours_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "24".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "How long completed background task records and artifacts are retained",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: TASK_LIST_MAX_LIMIT_KEY,
        label_i18n_key: "settings_item_task_list_max_limit_label",
        description_i18n_key: "settings_item_task_list_max_limit_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "100".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_TASKS,
        description: "Maximum page size accepted by background task list APIs",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
    ConfigDefinition {
        key: MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY,
        label_i18n_key: "settings_item_maintenance_cleanup_interval_secs_label",
        description_i18n_key: "settings_item_maintenance_cleanup_interval_secs_desc",
        value_type: ConfigValueType::Number,
        default_fn: || "3600".to_string(),
        requires_restart: false,
        is_sensitive: false,
        category: CONFIG_CATEGORY_RUNTIME_MAINTENANCE,
        description: "Default interval for project maintenance cleanup loops",
        normalize_fn: Some(normalize_operation_interval),
        ..ConfigDefinition::private_system()
    },
];

pub static CONFIG_REGISTRY: ConfigRegistry = ConfigRegistry::new(ALL_CONFIGS);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn every_config_category_is_allowed() {
        let allowed = SYSTEM_CONFIG_ALLOWED_CATEGORIES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(allowed.len(), SYSTEM_CONFIG_ALLOWED_CATEGORIES.len());

        for def in ALL_CONFIGS {
            assert!(
                allowed.contains(def.category),
                "{} uses unregistered category {}",
                def.key,
                def.category
            );
        }
    }

    #[test]
    fn deprecated_config_keys_do_not_overlap_active_definitions() {
        let active = ALL_CONFIGS
            .iter()
            .map(|def| def.key)
            .collect::<BTreeSet<_>>();
        let deprecated = DEPRECATED_SYSTEM_CONFIG_KEYS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            deprecated.len(),
            DEPRECATED_SYSTEM_CONFIG_KEYS.len(),
            "deprecated config keys must be unique"
        );

        for key in deprecated {
            assert!(!active.contains(key), "{key} is both active and deprecated");
        }
    }

    #[test]
    fn representative_configs_use_domain_subcategories() {
        let by_key = ALL_CONFIGS
            .iter()
            .map(|def| (def.key, def.category))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(by_key[PUBLIC_SITE_URL_KEY], CONFIG_CATEGORY_SITE_PUBLIC);
        assert_eq!(by_key[BRANDING_TITLE_KEY], CONFIG_CATEGORY_SITE_BRANDING);
        assert_eq!(
            by_key[AUTH_ACCESS_TOKEN_TTL_SECS_KEY],
            CONFIG_CATEGORY_AUTH_SESSION
        );
        assert_eq!(
            by_key[AUTH_ALLOW_USER_REGISTRATION_KEY],
            CONFIG_CATEGORY_AUTH_REGISTRATION
        );
        assert_eq!(
            by_key[AUTH_PASSWORD_RESET_TTL_SECS_KEY],
            CONFIG_CATEGORY_AUTH_RECOVERY
        );
        assert_eq!(
            by_key[AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY],
            CONFIG_CATEGORY_AUTH_LOGIN
        );
        assert_eq!(
            by_key[AUTH_LOCAL_EMAIL_ALLOWLIST_KEY],
            CONFIG_CATEGORY_AUTH_EMAIL_POLICY
        );
        assert_eq!(by_key[GRAVATAR_BASE_URL_KEY], CONFIG_CATEGORY_USER_AVATAR);
        assert_eq!(by_key[CORS_ENABLED_KEY], CONFIG_CATEGORY_NETWORK_CORS);
        assert_eq!(by_key[AUDIT_LOG_ENABLED_KEY], CONFIG_CATEGORY_AUDIT_LOG);
        assert_eq!(by_key[MAIL_SMTP_HOST_KEY], CONFIG_CATEGORY_MAIL_CONFIG);
        assert_eq!(
            by_key[MAIL_TEMPLATE_PASSWORD_RESET_HTML_KEY],
            CONFIG_CATEGORY_MAIL_TEMPLATE
        );
        assert_eq!(
            by_key[YGGDRASIL_SERVER_NAME_KEY],
            CONFIG_CATEGORY_YGGDRASIL_METADATA
        );
        assert_eq!(
            by_key[YGGDRASIL_TOKEN_TTL_DAYS_KEY],
            CONFIG_CATEGORY_YGGDRASIL_AUTH
        );
        assert_eq!(
            by_key[YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY],
            CONFIG_CATEGORY_YGGDRASIL_TEXTURES
        );
        assert_eq!(
            by_key[YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY],
            CONFIG_CATEGORY_YGGDRASIL_SIGNING
        );
        assert_eq!(
            by_key[TEXTURE_PREVIEW_ENGINE_KEY],
            CONFIG_CATEGORY_TEXTURE_PREVIEW
        );
        assert_eq!(
            by_key[MAIL_OUTBOX_DISPATCH_INTERVAL_SECS_KEY],
            CONFIG_CATEGORY_RUNTIME_MAIL
        );
        assert_eq!(
            by_key[BACKGROUND_TASK_MAX_CONCURRENCY_KEY],
            CONFIG_CATEGORY_RUNTIME_TASKS
        );
        assert_eq!(
            by_key[MAINTENANCE_CLEANUP_INTERVAL_SECS_KEY],
            CONFIG_CATEGORY_RUNTIME_MAINTENANCE
        );
    }

    #[test]
    fn registry_can_build_normalized_default_seed_records() {
        let seeds = CONFIG_REGISTRY
            .default_seed_records()
            .expect("system config registry defaults should seed");
        let by_key = seeds
            .iter()
            .map(|seed| (seed.key.as_str(), seed))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(seeds.len(), ALL_CONFIGS.len());
        assert_eq!(
            by_key[PUBLIC_SITE_URL_KEY].value,
            empty_origin_list_default()
        );
        assert_eq!(by_key[AUTH_COOKIE_SECURE_KEY].value, "true");
        assert_eq!(by_key[YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY].value, "");
        assert_eq!(by_key[YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY].value, "");
    }

    #[test]
    fn registry_definitions_attach_representative_normalizers() {
        for key in [
            PUBLIC_SITE_URL_KEY,
            BRANDING_TITLE_KEY,
            AUTH_COOKIE_SECURE_KEY,
            AUTH_ACCESS_TOKEN_TTL_SECS_KEY,
            AUTH_CAPTCHA_PRESET_KEY,
            AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
            CORS_ALLOWED_ORIGINS_KEY,
            AUDIT_LOG_RECORDED_ACTIONS_KEY,
            MAIL_SMTP_PORT_KEY,
            MAIL_TEMPLATE_PASSWORD_RESET_SUBJECT_KEY,
            YGGDRASIL_TOKEN_TTL_DAYS_KEY,
            TEXTURE_PREVIEW_ENGINE_KEY,
            BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        ] {
            let definition = CONFIG_REGISTRY
                .require(key)
                .expect("representative config key should exist");
            assert!(
                definition.normalize_fn.is_some(),
                "{key} should register a normalizer"
            );
        }

        assert!(
            CONFIG_REGISTRY
                .require(YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY)
                .expect("signature private key config should exist")
                .normalize_fn
                .is_none()
        );
    }
}
