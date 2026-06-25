//! Authentication runtime configuration helpers.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};
use aster_forge_config::{
    parse_bool_like_value, parse_positive_u64, parse_single_string_enum_selection,
    read_bool as forge_read_bool, read_bounded_u64, read_positive_u64 as forge_read_positive_u64,
};

pub use crate::config::definitions::{
    AUTH_ACCESS_TOKEN_TTL_SECS_KEY, AUTH_ALLOW_USER_REGISTRATION_KEY, AUTH_CAPTCHA_ENABLED_KEY,
    AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY, AUTH_CAPTCHA_LENGTH_KEY,
    AUTH_CAPTCHA_LOGIN_REQUIRED_KEY, AUTH_CAPTCHA_MAX_ATTEMPTS_KEY, AUTH_CAPTCHA_PRESET_KEY,
    AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY, AUTH_CAPTCHA_REGISTER_REQUIRED_KEY,
    AUTH_CAPTCHA_TTL_SECS_KEY, AUTH_CONTACT_CHANGE_TTL_SECS_KEY,
    AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY, AUTH_COOKIE_SECURE_KEY,
    AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY, AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY,
    AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY, AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY,
    AUTH_PASSKEY_LOGIN_ENABLED_KEY, AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY,
    AUTH_PASSWORD_RESET_TTL_SECS_KEY, AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
    AUTH_REGISTER_ACTIVATION_ENABLED_KEY, AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY,
    AUTH_USER_INVITATION_TTL_SECS_KEY,
};

pub const DEFAULT_AUTH_COOKIE_SECURE: bool = true;
pub const DEFAULT_AUTH_ALLOW_USER_REGISTRATION: bool = true;
pub const DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED: bool = true;
pub const DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS: u64 = 900;
pub const DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS: u64 = 604800;
pub const DEFAULT_AUTH_REGISTER_ACTIVATION_TTL_SECS: u64 = 86_400;
pub const DEFAULT_AUTH_USER_INVITATION_TTL_SECS: u64 = 7 * 86_400;
pub const DEFAULT_AUTH_CONTACT_CHANGE_TTL_SECS: u64 = 86_400;
pub const DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS: u64 = 3_600;
pub const DEFAULT_AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS: u64 = 60;
pub const DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS: u64 = 60;
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_ENABLED: bool = false;
pub const DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED: bool = true;
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK: bool = false;
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS: u64 = 600;
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS: u64 = 60;
pub const DEFAULT_AUTH_CAPTCHA_ENABLED: bool = false;
pub const DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED: bool = true;
pub const DEFAULT_AUTH_CAPTCHA_REGISTER_REQUIRED: bool = true;
pub const DEFAULT_AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED: bool = true;
pub const DEFAULT_AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED: bool = true;
pub const DEFAULT_AUTH_CAPTCHA_TTL_SECS: u64 = 120;
pub const DEFAULT_AUTH_CAPTCHA_PRESET: CaptchaRenderPreset = CaptchaRenderPreset::Balanced;
pub const DEFAULT_AUTH_CAPTCHA_LENGTH: u64 = 5;
pub const DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 3;
pub const MIN_AUTH_CAPTCHA_LENGTH: u64 = 4;
pub const MAX_AUTH_CAPTCHA_LENGTH: u64 = 8;
pub const MIN_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 1;
pub const MAX_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaRenderPreset {
    Readable,
    Balanced,
    Hardened,
}

impl CaptchaRenderPreset {
    pub const ALL: [Self; 3] = [Self::Readable, Self::Balanced, Self::Hardened];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Readable => "readable",
            Self::Balanced => "balanced",
            Self::Hardened => "hardened",
        }
    }

    fn parse_value(value: &str) -> Option<Self> {
        match value.trim() {
            "readable" => Some(Self::Readable),
            "balanced" => Some(Self::Balanced),
            "hardened" => Some(Self::Hardened),
            _ => None,
        }
    }

    pub const fn render_params(self) -> CaptchaRenderParams {
        match self {
            Self::Readable => CaptchaRenderParams {
                height: 54,
                complexity: 1,
                compression: 86,
                interference_lines: 1,
                interference_ellipses: 0,
                distortion: 1,
            },
            Self::Balanced => CaptchaRenderParams {
                height: 58,
                complexity: 2,
                compression: 78,
                interference_lines: 2,
                interference_ellipses: 1,
                distortion: 2,
            },
            Self::Hardened => CaptchaRenderParams {
                height: 64,
                complexity: 4,
                compression: 58,
                interference_lines: 4,
                interference_ellipses: 3,
                distortion: 4,
            },
        }
    }
}

impl FromStr for CaptchaRenderPreset {
    type Err = AsterError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse_value(value).ok_or_else(|| {
            AsterError::validation_error(
                "captcha render preset must be one of: readable, balanced, hardened",
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptchaRenderParams {
    pub height: u32,
    pub complexity: u32,
    pub compression: u32,
    pub interference_lines: u32,
    pub interference_ellipses: u32,
    pub distortion: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeAuthPolicy {
    pub cookie_secure: bool,
    pub allow_user_registration: bool,
    pub passkey_login_enabled: bool,
    pub register_activation_enabled: bool,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeContactVerificationPolicy {
    pub register_activation_ttl_secs: u64,
    pub contact_change_ttl_secs: u64,
    pub resend_cooldown_secs: u64,
    pub password_reset_ttl_secs: u64,
    pub password_reset_request_cooldown_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEmailCodeLoginPolicy {
    pub enabled: bool,
    pub allow_totp_fallback: bool,
    pub ttl_secs: u64,
    pub resend_cooldown_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCaptchaPolicy {
    pub enabled: bool,
    pub login_required: bool,
    pub register_required: bool,
    pub invitation_accept_required: bool,
    pub register_activation_resend_required: bool,
    pub ttl_secs: u64,
    pub preset: CaptchaRenderPreset,
    pub length: u64,
    pub max_attempts: u64,
}

impl RuntimeAuthPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let cookie_secure = forge_read_bool(
            runtime_config,
            AUTH_COOKIE_SECURE_KEY,
            DEFAULT_AUTH_COOKIE_SECURE,
        );
        let allow_user_registration = forge_read_bool(
            runtime_config,
            AUTH_ALLOW_USER_REGISTRATION_KEY,
            DEFAULT_AUTH_ALLOW_USER_REGISTRATION,
        );
        let register_activation_enabled = forge_read_bool(
            runtime_config,
            AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
            DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED,
        );
        let passkey_login_enabled = forge_read_bool(
            runtime_config,
            AUTH_PASSKEY_LOGIN_ENABLED_KEY,
            DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED,
        );

        let access_token_ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_ACCESS_TOKEN_TTL_SECS_KEY,
            DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS,
        );

        let refresh_token_ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
            DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS,
        );

        Self {
            cookie_secure,
            allow_user_registration,
            passkey_login_enabled,
            register_activation_enabled,
            access_token_ttl_secs,
            refresh_token_ttl_secs,
        }
    }
}

impl RuntimeContactVerificationPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let register_activation_ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY,
            DEFAULT_AUTH_REGISTER_ACTIVATION_TTL_SECS,
        );
        let contact_change_ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_CONTACT_CHANGE_TTL_SECS_KEY,
            DEFAULT_AUTH_CONTACT_CHANGE_TTL_SECS,
        );
        let password_reset_ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_PASSWORD_RESET_TTL_SECS_KEY,
            DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS,
        );
        let resend_cooldown_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY,
            DEFAULT_AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS,
        );
        let password_reset_request_cooldown_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY,
            DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS,
        );

        Self {
            register_activation_ttl_secs,
            contact_change_ttl_secs,
            resend_cooldown_secs,
            password_reset_ttl_secs,
            password_reset_request_cooldown_secs,
        }
    }
}

impl RuntimeEmailCodeLoginPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let enabled = forge_read_bool(
            runtime_config,
            AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_ENABLED,
        );
        let allow_totp_fallback = forge_read_bool(
            runtime_config,
            AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK,
        );
        let ttl_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS,
        );
        let resend_cooldown_secs = forge_read_positive_u64(
            runtime_config,
            AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS,
        );

        Self {
            enabled,
            allow_totp_fallback,
            ttl_secs,
            resend_cooldown_secs,
        }
    }
}

impl RuntimeCaptchaPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        Self::from_runtime_config_with_overrides(runtime_config, &BTreeMap::new())
    }

    pub fn from_runtime_config_with_overrides(
        runtime_config: &RuntimeConfig,
        overrides: &BTreeMap<String, String>,
    ) -> Self {
        let get = |key: &str| {
            overrides
                .get(key)
                .cloned()
                .or_else(|| runtime_config.get(key))
        };
        let enabled = forge_read_bool(&get, AUTH_CAPTCHA_ENABLED_KEY, DEFAULT_AUTH_CAPTCHA_ENABLED);
        let login_required = forge_read_bool(
            &get,
            AUTH_CAPTCHA_LOGIN_REQUIRED_KEY,
            DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED,
        );
        let register_required = forge_read_bool(
            &get,
            AUTH_CAPTCHA_REGISTER_REQUIRED_KEY,
            DEFAULT_AUTH_CAPTCHA_REGISTER_REQUIRED,
        );
        let invitation_accept_required = forge_read_bool(
            &get,
            AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY,
            DEFAULT_AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED,
        );
        let register_activation_resend_required = forge_read_bool(
            &get,
            AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY,
            DEFAULT_AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED,
        );
        let ttl_secs = forge_read_positive_u64(
            &get,
            AUTH_CAPTCHA_TTL_SECS_KEY,
            DEFAULT_AUTH_CAPTCHA_TTL_SECS,
        );
        let preset = read_captcha_render_preset_from(&get);
        let length = read_bounded_u64(
            &get,
            AUTH_CAPTCHA_LENGTH_KEY,
            DEFAULT_AUTH_CAPTCHA_LENGTH,
            MIN_AUTH_CAPTCHA_LENGTH,
            MAX_AUTH_CAPTCHA_LENGTH,
        );
        let max_attempts = read_bounded_u64(
            &get,
            AUTH_CAPTCHA_MAX_ATTEMPTS_KEY,
            DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS,
            MIN_AUTH_CAPTCHA_MAX_ATTEMPTS,
            MAX_AUTH_CAPTCHA_MAX_ATTEMPTS,
        );

        Self {
            enabled,
            login_required,
            register_required,
            invitation_accept_required,
            register_activation_resend_required,
            ttl_secs,
            preset,
            length,
            max_attempts,
        }
    }

    pub fn login_required(&self) -> bool {
        self.enabled && self.login_required
    }

    pub fn register_required(&self) -> bool {
        self.enabled && self.register_required
    }

    pub fn invitation_accept_required(&self) -> bool {
        self.enabled && self.invitation_accept_required
    }

    pub fn register_activation_resend_required(&self) -> bool {
        self.enabled && self.register_activation_resend_required
    }
}

pub fn normalize_cookie_secure_config_value(value: &str) -> Result<String> {
    match parse_bool_like_value(value) {
        Some(value) => Ok(if value { "true" } else { "false" }.to_string()),
        None => Err(AsterError::validation_error(
            "auth_cookie_secure must be 'true' or 'false'",
        )),
    }
}

pub fn normalize_allow_user_registration_config_value(value: &str) -> Result<String> {
    match parse_bool_like_value(value) {
        Some(value) => Ok(if value { "true" } else { "false" }.to_string()),
        None => Err(AsterError::validation_error(
            "auth_allow_user_registration must be 'true' or 'false'",
        )),
    }
}

pub fn normalize_register_activation_enabled_config_value(value: &str) -> Result<String> {
    match parse_bool_like_value(value) {
        Some(value) => Ok(if value { "true" } else { "false" }.to_string()),
        None => Err(AsterError::validation_error(
            "auth_register_activation_enabled must be 'true' or 'false'",
        )),
    }
}

pub fn normalize_email_code_login_bool_config_value(key: &str, value: &str) -> Result<String> {
    match parse_bool_like_value(value) {
        Some(value) => Ok(if value { "true" } else { "false" }.to_string()),
        None => Err(AsterError::validation_error(format!(
            "{key} must be 'true' or 'false'",
        ))),
    }
}

pub fn normalize_auth_bool_config_value(key: &str, value: &str) -> Result<String> {
    match parse_bool_like_value(value) {
        Some(value) => Ok(if value { "true" } else { "false" }.to_string()),
        None => Err(AsterError::validation_error(format!(
            "{key} must be 'true' or 'false'",
        ))),
    }
}

pub fn normalize_token_ttl_config_value(key: &str, value: &str) -> Result<String> {
    let Some(ttl) = parse_positive_u64(value) else {
        return Err(AsterError::validation_error(format!(
            "{key} must be a positive integer",
        )));
    };
    Ok(ttl.to_string())
}

pub fn normalize_captcha_length_config_value(value: &str) -> Result<String> {
    let Some(length) = parse_positive_u64(value) else {
        return Err(AsterError::validation_error(format!(
            "{AUTH_CAPTCHA_LENGTH_KEY} must be a positive integer",
        )));
    };
    if !(MIN_AUTH_CAPTCHA_LENGTH..=MAX_AUTH_CAPTCHA_LENGTH).contains(&length) {
        return Err(AsterError::validation_error(format!(
            "{AUTH_CAPTCHA_LENGTH_KEY} must be between {MIN_AUTH_CAPTCHA_LENGTH} and {MAX_AUTH_CAPTCHA_LENGTH}",
        )));
    }
    Ok(length.to_string())
}

pub fn normalize_captcha_preset_config_value(value: &str) -> Result<String> {
    let selected = parse_captcha_preset_selection(value)?;
    Ok(selected.as_str().to_string())
}

pub fn normalize_captcha_max_attempts_config_value(value: &str) -> Result<String> {
    let Some(attempts) = parse_positive_u64(value) else {
        return Err(AsterError::validation_error(format!(
            "{AUTH_CAPTCHA_MAX_ATTEMPTS_KEY} must be a positive integer",
        )));
    };
    if !(MIN_AUTH_CAPTCHA_MAX_ATTEMPTS..=MAX_AUTH_CAPTCHA_MAX_ATTEMPTS).contains(&attempts) {
        return Err(AsterError::validation_error(format!(
            "{AUTH_CAPTCHA_MAX_ATTEMPTS_KEY} must be between {MIN_AUTH_CAPTCHA_MAX_ATTEMPTS} and {MAX_AUTH_CAPTCHA_MAX_ATTEMPTS}",
        )));
    }
    Ok(attempts.to_string())
}

pub fn user_invitation_ttl_secs(runtime_config: &RuntimeConfig) -> u64 {
    forge_read_positive_u64(
        runtime_config,
        AUTH_USER_INVITATION_TTL_SECS_KEY,
        DEFAULT_AUTH_USER_INVITATION_TTL_SECS,
    )
}

fn read_captcha_render_preset_from<F>(get: &F) -> CaptchaRenderPreset
where
    F: Fn(&str) -> Option<String>,
{
    match get(AUTH_CAPTCHA_PRESET_KEY) {
        Some(raw) => match parse_captcha_preset_selection(&raw) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(
                    key = AUTH_CAPTCHA_PRESET_KEY,
                    value = %raw,
                    error = %error,
                    "invalid runtime captcha preset config; using default"
                );
                DEFAULT_AUTH_CAPTCHA_PRESET
            }
        },
        None => DEFAULT_AUTH_CAPTCHA_PRESET,
    }
}

fn parse_captcha_preset_selection(value: &str) -> Result<CaptchaRenderPreset> {
    parse_single_string_enum_selection(
        value,
        AUTH_CAPTCHA_PRESET_KEY,
        "readable, balanced, hardened",
        |value| value.parse::<CaptchaRenderPreset>().ok(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        AUTH_ACCESS_TOKEN_TTL_SECS_KEY, AUTH_ALLOW_USER_REGISTRATION_KEY, AUTH_CAPTCHA_ENABLED_KEY,
        AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY, AUTH_CAPTCHA_LENGTH_KEY,
        AUTH_CAPTCHA_LOGIN_REQUIRED_KEY, AUTH_CAPTCHA_MAX_ATTEMPTS_KEY, AUTH_CAPTCHA_PRESET_KEY,
        AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY, AUTH_CAPTCHA_REGISTER_REQUIRED_KEY,
        AUTH_CAPTCHA_TTL_SECS_KEY, AUTH_CONTACT_CHANGE_TTL_SECS_KEY,
        AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY, AUTH_COOKIE_SECURE_KEY,
        AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY, AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY,
        AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY, AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY,
        AUTH_PASSKEY_LOGIN_ENABLED_KEY, AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY,
        AUTH_PASSWORD_RESET_TTL_SECS_KEY, AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
        AUTH_REGISTER_ACTIVATION_ENABLED_KEY, AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY,
        CaptchaRenderPreset, DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS,
        DEFAULT_AUTH_ALLOW_USER_REGISTRATION, DEFAULT_AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED,
        DEFAULT_AUTH_CAPTCHA_LENGTH, DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED,
        DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS, DEFAULT_AUTH_CAPTCHA_PRESET,
        DEFAULT_AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED,
        DEFAULT_AUTH_CAPTCHA_REGISTER_REQUIRED, DEFAULT_AUTH_CAPTCHA_TTL_SECS,
        DEFAULT_AUTH_CONTACT_CHANGE_TTL_SECS,
        DEFAULT_AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS, DEFAULT_AUTH_COOKIE_SECURE,
        DEFAULT_AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK, DEFAULT_AUTH_EMAIL_CODE_LOGIN_ENABLED,
        DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS, DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS,
        DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED, DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS,
        DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS, DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS,
        DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED, DEFAULT_AUTH_REGISTER_ACTIVATION_TTL_SECS,
        MAX_AUTH_CAPTCHA_LENGTH, MAX_AUTH_CAPTCHA_MAX_ATTEMPTS, MIN_AUTH_CAPTCHA_LENGTH,
        MIN_AUTH_CAPTCHA_MAX_ATTEMPTS, RuntimeAuthPolicy, RuntimeCaptchaPolicy,
        RuntimeContactVerificationPolicy, RuntimeEmailCodeLoginPolicy,
        normalize_auth_bool_config_value, normalize_captcha_length_config_value,
        normalize_captcha_max_attempts_config_value, normalize_captcha_preset_config_value,
        normalize_email_code_login_bool_config_value, normalize_token_ttl_config_value,
    };
    use crate::config::RuntimeConfig;
    use crate::config::definitions::CONFIG_CATEGORY_AUTH_SESSION;
    use crate::entities::system_config;
    use chrono::Utc;

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: crate::types::SystemConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: crate::types::SystemConfigSource::System,
            visibility: crate::types::SystemConfigVisibility::Private,
            namespace: String::new(),
            category: CONFIG_CATEGORY_AUTH_SESSION.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn runtime_auth_policy_uses_defaults_when_config_missing() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeAuthPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.cookie_secure, DEFAULT_AUTH_COOKIE_SECURE);
        assert_eq!(
            policy.allow_user_registration,
            DEFAULT_AUTH_ALLOW_USER_REGISTRATION
        );
        assert_eq!(
            policy.register_activation_enabled,
            DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED
        );
        assert_eq!(
            policy.passkey_login_enabled,
            DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED
        );
        assert_eq!(
            policy.access_token_ttl_secs,
            DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS
        );
        assert_eq!(
            policy.refresh_token_ttl_secs,
            DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS
        );
    }

    #[test]
    fn runtime_auth_policy_reads_runtime_values() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_COOKIE_SECURE_KEY, "false"));
        runtime_config.apply(config_model(AUTH_ALLOW_USER_REGISTRATION_KEY, "false"));
        runtime_config.apply(config_model(AUTH_REGISTER_ACTIVATION_ENABLED_KEY, "false"));
        runtime_config.apply(config_model(AUTH_PASSKEY_LOGIN_ENABLED_KEY, "false"));
        runtime_config.apply(config_model(AUTH_ACCESS_TOKEN_TTL_SECS_KEY, "120"));
        runtime_config.apply(config_model(AUTH_REFRESH_TOKEN_TTL_SECS_KEY, "3600"));

        let policy = RuntimeAuthPolicy::from_runtime_config(&runtime_config);

        assert!(!policy.cookie_secure);
        assert!(!policy.allow_user_registration);
        assert!(!policy.register_activation_enabled);
        assert!(!policy.passkey_login_enabled);
        assert_eq!(policy.access_token_ttl_secs, 120);
        assert_eq!(policy.refresh_token_ttl_secs, 3600);
    }

    #[test]
    fn runtime_auth_policy_rejects_invalid_bool_values_to_defaults() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_COOKIE_SECURE_KEY, "maybe"));
        runtime_config.apply(config_model(AUTH_ALLOW_USER_REGISTRATION_KEY, "unknown"));
        runtime_config.apply(config_model(
            AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
            "sometimes",
        ));
        runtime_config.apply(config_model(AUTH_PASSKEY_LOGIN_ENABLED_KEY, "maybe"));

        let policy = RuntimeAuthPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.cookie_secure, DEFAULT_AUTH_COOKIE_SECURE);
        assert_eq!(
            policy.allow_user_registration,
            DEFAULT_AUTH_ALLOW_USER_REGISTRATION
        );
        assert_eq!(
            policy.register_activation_enabled,
            DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED
        );
        assert_eq!(
            policy.passkey_login_enabled,
            DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED
        );
    }

    #[test]
    fn runtime_contact_verification_policy_reads_values_and_defaults_invalid_input() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_REGISTER_ACTIVATION_TTL_SECS_KEY, "120"));
        runtime_config.apply(config_model(AUTH_CONTACT_CHANGE_TTL_SECS_KEY, "240"));
        runtime_config.apply(config_model(AUTH_PASSWORD_RESET_TTL_SECS_KEY, "0"));
        runtime_config.apply(config_model(
            AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY,
            "30",
        ));
        runtime_config.apply(config_model(
            AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS_KEY,
            "bad",
        ));

        let policy = RuntimeContactVerificationPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.register_activation_ttl_secs, 120);
        assert_eq!(policy.contact_change_ttl_secs, 240);
        assert_eq!(
            policy.password_reset_ttl_secs,
            DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS
        );
        assert_eq!(policy.resend_cooldown_secs, 30);
        assert_eq!(
            policy.password_reset_request_cooldown_secs,
            DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS
        );
    }

    #[test]
    fn runtime_contact_verification_policy_uses_defaults_when_config_missing() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeContactVerificationPolicy::from_runtime_config(&runtime_config);

        assert_eq!(
            policy.register_activation_ttl_secs,
            DEFAULT_AUTH_REGISTER_ACTIVATION_TTL_SECS
        );
        assert_eq!(
            policy.contact_change_ttl_secs,
            DEFAULT_AUTH_CONTACT_CHANGE_TTL_SECS
        );
        assert_eq!(
            policy.password_reset_ttl_secs,
            DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS
        );
        assert_eq!(
            policy.resend_cooldown_secs,
            DEFAULT_AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS
        );
        assert_eq!(
            policy.password_reset_request_cooldown_secs,
            DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS
        );
    }

    #[test]
    fn runtime_email_code_login_policy_uses_safe_defaults() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeEmailCodeLoginPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.enabled, DEFAULT_AUTH_EMAIL_CODE_LOGIN_ENABLED);
        assert_eq!(
            policy.allow_totp_fallback,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK
        );
        assert_eq!(policy.ttl_secs, DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS);
        assert_eq!(
            policy.resend_cooldown_secs,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS
        );
    }

    #[test]
    fn runtime_email_code_login_policy_reads_valid_values() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY, "on"));
        runtime_config.apply(config_model(
            AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY,
            "1",
        ));
        runtime_config.apply(config_model(AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY, "1"));
        runtime_config.apply(config_model(
            AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY,
            "60",
        ));

        let policy = RuntimeEmailCodeLoginPolicy::from_runtime_config(&runtime_config);

        assert!(policy.enabled);
        assert!(policy.allow_totp_fallback);
        assert_eq!(policy.ttl_secs, 1);
        assert_eq!(policy.resend_cooldown_secs, 60);
    }

    #[test]
    fn runtime_email_code_login_policy_rejects_invalid_values_to_defaults() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY, "maybe"));
        runtime_config.apply(config_model(
            AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY,
            "perhaps",
        ));
        runtime_config.apply(config_model(AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY, "0"));
        runtime_config.apply(config_model(
            AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS_KEY,
            "not-a-number",
        ));

        let policy = RuntimeEmailCodeLoginPolicy::from_runtime_config(&runtime_config);

        assert!(!policy.enabled);
        assert!(!policy.allow_totp_fallback);
        assert_eq!(policy.ttl_secs, DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS);
        assert_eq!(
            policy.resend_cooldown_secs,
            DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS
        );
    }

    #[test]
    fn runtime_email_code_login_normalizers_enforce_boolean_and_positive_ttl() {
        assert_eq!(
            normalize_email_code_login_bool_config_value(AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY, "yes")
                .unwrap(),
            "true"
        );
        assert_eq!(
            normalize_email_code_login_bool_config_value(
                AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK_KEY,
                "off"
            )
            .unwrap(),
            "false"
        );
        assert!(
            normalize_email_code_login_bool_config_value(
                AUTH_EMAIL_CODE_LOGIN_ENABLED_KEY,
                "sometimes"
            )
            .is_err()
        );
        assert_eq!(
            normalize_token_ttl_config_value(AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY, "60").unwrap(),
            "60"
        );
        assert!(normalize_token_ttl_config_value(AUTH_EMAIL_CODE_LOGIN_TTL_SECS_KEY, "0").is_err());
    }

    #[test]
    fn runtime_captcha_policy_uses_safe_defaults() {
        let runtime_config = RuntimeConfig::new();
        let policy = RuntimeCaptchaPolicy::from_runtime_config(&runtime_config);

        assert!(!policy.enabled);
        assert_eq!(policy.login_required, DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED);
        assert_eq!(
            policy.register_required,
            DEFAULT_AUTH_CAPTCHA_REGISTER_REQUIRED
        );
        assert_eq!(
            policy.invitation_accept_required,
            DEFAULT_AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED
        );
        assert_eq!(
            policy.register_activation_resend_required,
            DEFAULT_AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED
        );
        assert_eq!(policy.ttl_secs, DEFAULT_AUTH_CAPTCHA_TTL_SECS);
        assert_eq!(policy.preset, DEFAULT_AUTH_CAPTCHA_PRESET);
        assert_eq!(policy.length, DEFAULT_AUTH_CAPTCHA_LENGTH);
        assert_eq!(policy.max_attempts, DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS);
        assert!(!policy.login_required());
        assert!(!policy.register_required());
        assert!(!policy.invitation_accept_required());
        assert!(!policy.register_activation_resend_required());
    }

    #[test]
    fn runtime_captcha_policy_reads_valid_values() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_CAPTCHA_ENABLED_KEY, "on"));
        runtime_config.apply(config_model(AUTH_CAPTCHA_LOGIN_REQUIRED_KEY, "off"));
        runtime_config.apply(config_model(AUTH_CAPTCHA_REGISTER_REQUIRED_KEY, "yes"));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY,
            "0",
        ));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY,
            "1",
        ));
        runtime_config.apply(config_model(AUTH_CAPTCHA_TTL_SECS_KEY, "300"));
        runtime_config.apply(config_model(AUTH_CAPTCHA_PRESET_KEY, "hardened"));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_LENGTH_KEY,
            &MAX_AUTH_CAPTCHA_LENGTH.to_string(),
        ));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_MAX_ATTEMPTS_KEY,
            &MAX_AUTH_CAPTCHA_MAX_ATTEMPTS.to_string(),
        ));

        let policy = RuntimeCaptchaPolicy::from_runtime_config(&runtime_config);

        assert!(policy.enabled);
        assert!(!policy.login_required);
        assert!(policy.register_required);
        assert!(!policy.invitation_accept_required);
        assert!(policy.register_activation_resend_required);
        assert_eq!(policy.ttl_secs, 300);
        assert_eq!(policy.preset, CaptchaRenderPreset::Hardened);
        assert_eq!(policy.length, MAX_AUTH_CAPTCHA_LENGTH);
        assert_eq!(policy.max_attempts, MAX_AUTH_CAPTCHA_MAX_ATTEMPTS);
        assert!(!policy.login_required());
        assert!(policy.register_required());
        assert!(!policy.invitation_accept_required());
        assert!(policy.register_activation_resend_required());
    }

    #[test]
    fn runtime_captcha_policy_rejects_invalid_values_to_defaults() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_CAPTCHA_ENABLED_KEY, "maybe"));
        runtime_config.apply(config_model(AUTH_CAPTCHA_LOGIN_REQUIRED_KEY, "sometimes"));
        runtime_config.apply(config_model(AUTH_CAPTCHA_TTL_SECS_KEY, "0"));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_PRESET_KEY,
            r#"["readable","hardened"]"#,
        ));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_LENGTH_KEY,
            &(MIN_AUTH_CAPTCHA_LENGTH - 1).to_string(),
        ));
        runtime_config.apply(config_model(
            AUTH_CAPTCHA_MAX_ATTEMPTS_KEY,
            &(MAX_AUTH_CAPTCHA_MAX_ATTEMPTS + 1).to_string(),
        ));

        let policy = RuntimeCaptchaPolicy::from_runtime_config(&runtime_config);

        assert!(!policy.enabled);
        assert_eq!(policy.login_required, DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED);
        assert_eq!(policy.ttl_secs, DEFAULT_AUTH_CAPTCHA_TTL_SECS);
        assert_eq!(policy.preset, DEFAULT_AUTH_CAPTCHA_PRESET);
        assert_eq!(policy.length, DEFAULT_AUTH_CAPTCHA_LENGTH);
        assert_eq!(policy.max_attempts, DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS);
    }

    #[test]
    fn runtime_captcha_normalizers_enforce_boolean_and_ranges() {
        assert_eq!(
            normalize_auth_bool_config_value(AUTH_CAPTCHA_ENABLED_KEY, "yes").unwrap(),
            "true"
        );
        assert_eq!(
            normalize_auth_bool_config_value(AUTH_CAPTCHA_LOGIN_REQUIRED_KEY, "off").unwrap(),
            "false"
        );
        assert!(
            normalize_auth_bool_config_value(AUTH_CAPTCHA_REGISTER_REQUIRED_KEY, "sometimes")
                .is_err()
        );
        assert_eq!(
            normalize_captcha_preset_config_value(r#"["readable"]"#).unwrap(),
            "readable"
        );
        assert_eq!(
            normalize_captcha_preset_config_value("balanced").unwrap(),
            "balanced"
        );
        assert!(normalize_captcha_preset_config_value(r#"[]"#).is_err());
        assert!(normalize_captcha_preset_config_value(r#"["readable","hardened"]"#).is_err());
        assert!(normalize_captcha_preset_config_value(r#"["nope"]"#).is_err());
        assert_eq!(
            normalize_captcha_length_config_value(&MIN_AUTH_CAPTCHA_LENGTH.to_string()).unwrap(),
            MIN_AUTH_CAPTCHA_LENGTH.to_string()
        );
        assert_eq!(
            normalize_captcha_length_config_value(&MAX_AUTH_CAPTCHA_LENGTH.to_string()).unwrap(),
            MAX_AUTH_CAPTCHA_LENGTH.to_string()
        );
        assert!(normalize_captcha_length_config_value("0").is_err());
        assert!(
            normalize_captcha_length_config_value(&(MAX_AUTH_CAPTCHA_LENGTH + 1).to_string())
                .is_err()
        );
        assert_eq!(
            normalize_captcha_max_attempts_config_value(&MIN_AUTH_CAPTCHA_MAX_ATTEMPTS.to_string())
                .unwrap(),
            MIN_AUTH_CAPTCHA_MAX_ATTEMPTS.to_string()
        );
        assert_eq!(
            normalize_captcha_max_attempts_config_value(&MAX_AUTH_CAPTCHA_MAX_ATTEMPTS.to_string())
                .unwrap(),
            MAX_AUTH_CAPTCHA_MAX_ATTEMPTS.to_string()
        );
        assert!(normalize_captcha_max_attempts_config_value("0").is_err());
        assert!(
            normalize_captcha_max_attempts_config_value(
                &(MAX_AUTH_CAPTCHA_MAX_ATTEMPTS + 1).to_string()
            )
            .is_err()
        );
    }
}
