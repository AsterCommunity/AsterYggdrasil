//! Authentication runtime configuration helpers.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};
use aster_forge_config::{
    normalize_bool_config_value, normalize_bounded_u64_config_value,
    normalize_positive_u64_config_value, parse_single_string_enum_selection,
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

/// Whether session cookies get the `Secure` flag when `auth_cookie_secure` is
/// unset; defaults to true so production deployments do not ship insecure
/// cookies.
pub const DEFAULT_AUTH_COOKIE_SECURE: bool = true;
/// Whether self-registration is open when `auth_allow_user_registration` is
/// unset; defaults to true for self-hosted single-tenant setups.
pub const DEFAULT_AUTH_ALLOW_USER_REGISTRATION: bool = true;
/// Whether new accounts must activate via emailed link when
/// `auth_register_activation_enabled` is unset; defaults to true so
/// sign-ups prove email ownership out of the box.
pub const DEFAULT_AUTH_REGISTER_ACTIVATION_ENABLED: bool = true;
/// Access token lifetime, in seconds, when `auth_access_token_ttl_secs` is
/// unset; short lived (15 minutes) so stolen access tokens expire quickly.
pub const DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS: u64 = 900;
/// Refresh token lifetime, in seconds, when `auth_refresh_token_ttl_secs` is
/// unset; one week, balancing convenience against long-lived credentials.
pub const DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS: u64 = 604800;
/// Register activation link lifetime, in seconds, when
/// `auth_register_activation_ttl_secs` is unset; one day.
pub const DEFAULT_AUTH_REGISTER_ACTIVATION_TTL_SECS: u64 = 86_400;
/// User invitation acceptance link lifetime, in seconds, when
/// `auth_user_invitation_ttl_secs` is unset; one week so invited users are
/// not rushed.
pub const DEFAULT_AUTH_USER_INVITATION_TTL_SECS: u64 = 7 * 86_400;
/// Contact (email) change verification link lifetime, in seconds, when
/// `auth_contact_change_ttl_secs` is unset; one day.
pub const DEFAULT_AUTH_CONTACT_CHANGE_TTL_SECS: u64 = 86_400;
/// Password reset link lifetime, in seconds, when
/// `auth_password_reset_ttl_secs` is unset; one hour.
pub const DEFAULT_AUTH_PASSWORD_RESET_TTL_SECS: u64 = 3_600;
/// Minimum interval, in seconds, between resends of contact-change/register
/// activation verification mail when the corresponding key is unset.
pub const DEFAULT_AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS: u64 = 60;
/// Minimum interval, in seconds, between password reset requests for the
/// same account when `auth_password_reset_request_cooldown_secs` is unset.
pub const DEFAULT_AUTH_PASSWORD_RESET_REQUEST_COOLDOWN_SECS: u64 = 60;
/// Whether email-code login is available when `auth_email_code_login_enabled`
/// is unset; defaults to false so password login remains the only path until
/// the operator opts in.
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_ENABLED: bool = false;
/// Whether passkey (WebAuthn) login is available when
/// `auth_passkey_login_enabled` is unset; defaults to true.
pub const DEFAULT_AUTH_PASSKEY_LOGIN_ENABLED: bool = true;
/// Whether users with TOTP may sign in with a TOTP code when email-code login
/// cannot be used and `auth_email_code_login_allow_totp_fallback` is unset;
/// defaults to false.
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_ALLOW_TOTP_FALLBACK: bool = false;
/// Email login code lifetime, in seconds, when
/// `auth_email_code_login_ttl_secs` is unset; ten minutes.
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_TTL_SECS: u64 = 600;
/// Minimum interval, in seconds, between email login code resends when
/// `auth_email_code_login_resend_cooldown_secs` is unset.
pub const DEFAULT_AUTH_EMAIL_CODE_LOGIN_RESEND_COOLDOWN_SECS: u64 = 60;
/// Whether the CAPTCHA challenge is enabled at all when
/// `auth_captcha_enabled` is unset; defaults to false so fresh installs work
/// without image rendering configured.
pub const DEFAULT_AUTH_CAPTCHA_ENABLED: bool = false;
/// Whether login requires a CAPTCHA when the master switch is on and
/// `auth_captcha_login_required` is unset.
pub const DEFAULT_AUTH_CAPTCHA_LOGIN_REQUIRED: bool = true;
/// Whether registration requires a CAPTCHA when the master switch is on and
/// `auth_captcha_register_required` is unset.
pub const DEFAULT_AUTH_CAPTCHA_REGISTER_REQUIRED: bool = true;
/// Whether accepting a user invitation requires a CAPTCHA when the master
/// switch is on and `auth_captcha_invitation_accept_required` is unset.
pub const DEFAULT_AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED: bool = true;
/// Whether resending a register activation mail requires a CAPTCHA when the
/// master switch is on and the matching key is unset.
pub const DEFAULT_AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED: bool = true;
/// CAPTCHA answer lifetime, in seconds, when `auth_captcha_ttl_secs` is
/// unset; two minutes.
pub const DEFAULT_AUTH_CAPTCHA_TTL_SECS: u64 = 120;
/// CAPTCHA image difficulty preset used when `auth_captcha_preset` is unset.
pub const DEFAULT_AUTH_CAPTCHA_PRESET: CaptchaRenderPreset = CaptchaRenderPreset::Balanced;
/// CAPTCHA code length, in characters, when `auth_captcha_length` is unset.
pub const DEFAULT_AUTH_CAPTCHA_LENGTH: u64 = 5;
/// CAPTCHA verification attempts allowed per challenge when
/// `auth_captcha_max_attempts` is unset.
pub const DEFAULT_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 3;
/// Minimum accepted CAPTCHA code length; shorter codes are too easy to guess.
pub const MIN_AUTH_CAPTCHA_LENGTH: u64 = 4;
/// Maximum accepted CAPTCHA code length; longer codes hurt usability without
/// meaningfully slowing automated solving.
pub const MAX_AUTH_CAPTCHA_LENGTH: u64 = 8;
/// Minimum accepted CAPTCHA attempts per challenge.
pub const MIN_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 1;
/// Maximum accepted CAPTCHA attempts per challenge.
pub const MAX_AUTH_CAPTCHA_MAX_ATTEMPTS: u64 = 10;

/// Difficulty preset for CAPTCHA image rendering; maps to concrete render
/// parameters via [`CaptchaRenderPreset::render_params`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaRenderPreset {
    /// Low distortion, generous letter spacing: easiest for humans.
    Readable,
    /// Middle ground between readability and bot resistance (the default).
    Balanced,
    /// Heavy distortion, noise, and compression: hardest to solve.
    Hardened,
}

impl CaptchaRenderPreset {
    /// Every preset in canonical order, used for validation and schema docs.
    pub const ALL: [Self; 3] = [Self::Readable, Self::Balanced, Self::Hardened];

    /// Lowercase name as stored in the runtime config value.
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

    /// Concrete image render parameters (height, noise, distortion, JPEG
    /// compression) for this preset.
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

/// Concrete CAPTCHA image render parameters selected by a preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptchaRenderParams {
    /// Captcha image height in pixels.
    pub height: u32,
    /// Text complexity level (letter style/overlap) used by the renderer.
    pub complexity: u32,
    /// JPEG compression quality (0-100); lower quality adds noise.
    pub compression: u32,
    /// Number of interference lines drawn across the image.
    pub interference_lines: u32,
    /// Number of interference ellipses drawn across the image.
    pub interference_ellipses: u32,
    /// Text distortion intensity level applied by the renderer.
    pub distortion: u32,
}

/// Core account/token auth switches resolved from the runtime config store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeAuthPolicy {
    /// Whether session cookies carry the `Secure` flag.
    pub cookie_secure: bool,
    /// Whether self-registration is open.
    pub allow_user_registration: bool,
    /// Whether passkey (WebAuthn) login is available.
    pub passkey_login_enabled: bool,
    /// Whether new accounts must activate via emailed link.
    pub register_activation_enabled: bool,
    /// Access token lifetime, in seconds.
    pub access_token_ttl_secs: u64,
    /// Refresh token lifetime, in seconds.
    pub refresh_token_ttl_secs: u64,
}

/// Email verification budgets for contact-change, register activation, and
/// password reset flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeContactVerificationPolicy {
    /// Register activation link lifetime, in seconds.
    pub register_activation_ttl_secs: u64,
    /// Contact (email) change verification link lifetime, in seconds.
    pub contact_change_ttl_secs: u64,
    /// Minimum interval between verification mail resends, in seconds.
    pub resend_cooldown_secs: u64,
    /// Password reset link lifetime, in seconds.
    pub password_reset_ttl_secs: u64,
    /// Minimum interval between password reset requests, in seconds.
    pub password_reset_request_cooldown_secs: u64,
}

/// Email-code login settings resolved from the runtime config store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEmailCodeLoginPolicy {
    /// Whether email-code login is available at all.
    pub enabled: bool,
    /// Whether users with TOTP may fall back to a TOTP code when email
    /// delivery is unavailable.
    pub allow_totp_fallback: bool,
    /// Login code lifetime, in seconds.
    pub ttl_secs: u64,
    /// Minimum interval between code resends, in seconds.
    pub resend_cooldown_secs: u64,
}

/// CAPTCHA challenge settings resolved from the runtime config store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCaptchaPolicy {
    /// Whether the CAPTCHA master switch is on.
    pub enabled: bool,
    /// Whether login should require a CAPTCHA (when enabled).
    pub login_required: bool,
    /// Whether registration should require a CAPTCHA (when enabled).
    pub register_required: bool,
    /// Whether invitation acceptance should require a CAPTCHA (when enabled).
    pub invitation_accept_required: bool,
    /// Whether resending register activation mail should require a CAPTCHA
    /// (when enabled).
    pub register_activation_resend_required: bool,
    /// CAPTCHA answer lifetime, in seconds.
    pub ttl_secs: u64,
    /// Image difficulty preset.
    pub preset: CaptchaRenderPreset,
    /// CAPTCHA code length, in characters.
    pub length: u64,
    /// Verification attempts allowed per challenge.
    pub max_attempts: u64,
}

impl RuntimeAuthPolicy {
    /// Resolves the core auth switches and token TTLs from the runtime config
    /// store, applying documented defaults for missing or invalid values.
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
    /// Resolves the email verification budgets from the runtime config store,
    /// applying documented defaults for missing or non-positive values.
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
    /// Resolves the email-code login settings from the runtime config store,
    /// applying documented defaults for missing or invalid values.
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
    /// Resolves the CAPTCHA settings from the runtime config store.
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        Self::from_runtime_config_with_overrides(runtime_config, &BTreeMap::new())
    }

    /// Like [`RuntimeCaptchaPolicy::from_runtime_config`], but values in
    /// `overrides` (pending edits being previewed) take precedence over the
    /// stored snapshot.
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

    /// Whether login should present a CAPTCHA: requires the master switch.
    pub fn login_required(&self) -> bool {
        self.enabled && self.login_required
    }

    /// Whether registration should present a CAPTCHA: requires the master
    /// switch.
    pub fn register_required(&self) -> bool {
        self.enabled && self.register_required
    }

    /// Whether accepting a user invitation should present a CAPTCHA: requires
    /// the master switch.
    pub fn invitation_accept_required(&self) -> bool {
        self.enabled && self.invitation_accept_required
    }

    /// Whether resending register activation mail should present a CAPTCHA:
    /// requires the master switch.
    pub fn register_activation_resend_required(&self) -> bool {
        self.enabled && self.register_activation_resend_required
    }
}

/// Validates the stored `auth_cookie_secure` boolean value.
pub fn normalize_cookie_secure_config_value(value: &str) -> Result<String> {
    normalize_auth_bool_config_value(AUTH_COOKIE_SECURE_KEY, value)
}

/// Validates the stored `auth_allow_user_registration` boolean value.
pub fn normalize_allow_user_registration_config_value(value: &str) -> Result<String> {
    normalize_auth_bool_config_value(AUTH_ALLOW_USER_REGISTRATION_KEY, value)
}

/// Validates the stored `auth_register_activation_enabled` boolean value.
pub fn normalize_register_activation_enabled_config_value(value: &str) -> Result<String> {
    normalize_auth_bool_config_value(AUTH_REGISTER_ACTIVATION_ENABLED_KEY, value)
}

/// Validates a stored email-code-login boolean value for the given config key.
pub fn normalize_email_code_login_bool_config_value(key: &str, value: &str) -> Result<String> {
    normalize_auth_bool_config_value(key, value)
}

/// Validates a stored auth boolean value ("true"/"false" and common aliases).
pub fn normalize_auth_bool_config_value(key: &str, value: &str) -> Result<String> {
    normalize_bool_config_value(key, value).map_err(Into::into)
}

/// Validates a stored auth TTL value as a positive integer number of seconds.
pub fn normalize_token_ttl_config_value(key: &str, value: &str) -> Result<String> {
    normalize_positive_u64_config_value(key, value).map_err(Into::into)
}

/// Validates a stored `auth_captcha_length` value within
/// [`MIN_AUTH_CAPTCHA_LENGTH`]..=[`MAX_AUTH_CAPTCHA_LENGTH`].
pub fn normalize_captcha_length_config_value(value: &str) -> Result<String> {
    normalize_bounded_u64_config_value(
        AUTH_CAPTCHA_LENGTH_KEY,
        value,
        MIN_AUTH_CAPTCHA_LENGTH,
        MAX_AUTH_CAPTCHA_LENGTH,
    )
    .map_err(Into::into)
}

/// Validates a stored `auth_captcha_preset` value against
/// [`CaptchaRenderPreset::ALL`], normalizing to the canonical lowercase name.
pub fn normalize_captcha_preset_config_value(value: &str) -> Result<String> {
    let selected = parse_captcha_preset_selection(value)?;
    Ok(selected.as_str().to_string())
}

/// Validates a stored `auth_captcha_max_attempts` value within
/// [`MIN_AUTH_CAPTCHA_MAX_ATTEMPTS`]..=[`MAX_AUTH_CAPTCHA_MAX_ATTEMPTS`].
pub fn normalize_captcha_max_attempts_config_value(value: &str) -> Result<String> {
    normalize_bounded_u64_config_value(
        AUTH_CAPTCHA_MAX_ATTEMPTS_KEY,
        value,
        MIN_AUTH_CAPTCHA_MAX_ATTEMPTS,
        MAX_AUTH_CAPTCHA_MAX_ATTEMPTS,
    )
    .map_err(Into::into)
}

/// Returns the invitation acceptance link lifetime in seconds, falling back to
/// [`DEFAULT_AUTH_USER_INVITATION_TTL_SECS`] for missing or non-positive
/// values.
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
    use aster_forge_db::system_config;
    use chrono::Utc;

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: aster_forge_config::ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: aster_forge_config::ConfigSource::System,
            visibility: aster_forge_config::ConfigVisibility::Private,
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
