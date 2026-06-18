//! Stable public API error codes.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

macro_rules! define_error_codes {
    ($($variant:ident => $value:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub enum AsterErrorCode {
            $(
                #[serde(rename = $value)]
                $variant,
            )+
        }

        impl AsterErrorCode {
            pub const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }

            pub fn parse(value: &str) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

#[cfg(all(debug_assertions, feature = "openapi"))]
impl utoipa::PartialSchema for AsterErrorCode {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(Self::ALL.iter().map(|code| code.as_str())))
            .into()
    }
}

#[cfg(all(debug_assertions, feature = "openapi"))]
impl utoipa::ToSchema for AsterErrorCode {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("AsterErrorCode")
    }
}

define_error_codes! {
    Success => "success",

    // Generic request and platform errors.
    BadRequest => "bad_request",
    ValidationFailed => "validation.failed",
    RequestMalformed => "request.malformed",
    RequestPayloadTooLarge => "request.payload_too_large",
    NotFound => "not_found",
    InternalServerError => "internal_server_error",
    DatabaseError => "database.error",
    CacheError => "cache.error",
    StorageError => "storage.error",
    ConfigError => "config.error",
    RuntimeUnavailable => "runtime.unavailable",
    EndpointNotFound => "endpoint.not_found",
    EndpointMethodNotAllowed => "endpoint.method_not_allowed",
    RateLimited => "rate_limited",

    // Authentication, session and request-security errors.
    AuthSetupRequired => "auth.setup_required",
    AuthSetupAlreadyCompleted => "auth.setup_already_completed",
    AuthRegistrationDisabled => "auth.registration_disabled",
    AuthPasswordPolicyFailed => "auth.password_policy_failed",
    AuthUsernameExists => "auth.username_exists",
    AuthEmailExists => "auth.email_exists",
    AuthEmailBlocked => "auth.email_blocked",
    AuthEmailNotAllowlisted => "auth.email_not_allowlisted",
    AuthUserDisabled => "auth.user_disabled",
    AuthPendingActivation => "auth.pending_activation",
    AuthPasswordChangeRequired => "auth.password_change_required",
    AuthPasskeyLoginDisabled => "auth.passkey_login_disabled",
    AuthCaptchaRequired => "auth.captcha_required",
    AuthCaptchaInvalid => "auth.captcha_invalid",
    AuthCaptchaExpired => "auth.captcha_expired",
    ContactVerificationInvalid => "auth.contact_verification_invalid",
    ContactVerificationExpired => "auth.contact_verification_expired",
    AuthInvitationInvalid => "auth.invitation_invalid",
    AuthInvitationExpired => "auth.invitation_expired",
    AuthInvitationAccepted => "auth.invitation_accepted",
    AuthInvitationRevoked => "auth.invitation_revoked",
    MailNotConfigured => "mail.not_configured",
    MailDeliveryFailed => "mail.delivery_failed",

    AuthCredentialsFailed => "auth.credentials_failed",
    AuthTokenExpired => "auth.token_expired",
    AuthTokenInvalid => "auth.token_invalid",
    AuthSessionNotFound => "auth.session_not_found",
    AuthSessionRevocationFailed => "auth.session_revocation_failed",
    AuthCsrfMissing => "auth.csrf_missing",
    AuthCsrfInvalid => "auth.csrf_invalid",
    AuthAdminRequired => "auth.admin_required",
    Forbidden => "forbidden",

    // External authentication provider and login-flow errors.
    ExternalAuthError => "external_auth.error",
    ExternalAuthProviderNotFound => "external_auth.provider_not_found",
    ExternalAuthProviderDisabled => "external_auth.provider_disabled",
    ExternalAuthProviderMisconfigured => "external_auth.provider_misconfigured",
    ExternalAuthStateInvalid => "external_auth.state_invalid",
    ExternalAuthStateExpired => "external_auth.state_expired",
    ExternalAuthCallbackFailed => "external_auth.callback_failed",
    ExternalAuthIdentityConflict => "external_auth.identity_conflict",
    ExternalAuthCallbackRedirectUriRequired => "external_auth.callback_redirect_uri_required",

    // Mail and outbox errors.
    MailTemplateInvalid => "mail.template_invalid",
    MailOutboxNotFound => "mail.outbox_not_found",

    // Runtime configuration and action errors.
    ConfigNotFound => "config.not_found",
    ConfigReadOnly => "config.read_only",
    ConfigValidationFailed => "config.validation_failed",
    ConfigActionNotFound => "config.action_not_found",
    ConfigActionInvalid => "config.action_invalid",
    ConfigActionFailed => "config.action_failed",

    // Audit-log and background-task errors.
    AuditLogInvalidFilter => "audit_log.invalid_filter",
    TaskNotFound => "task.not_found",
    TaskInvalidState => "task.invalid_state",
    TaskRetryNotAllowed => "task.retry_not_allowed",
    TaskCleanupFailed => "task.cleanup_failed",
    TaskLeaseConflict => "task.lease_conflict",

    // Minecraft profile errors exposed by project API endpoints.
    MinecraftProfileNotFound => "minecraft_profile.not_found",
    MinecraftProfileUuidInvalid => "minecraft_profile.uuid_invalid",
    MinecraftProfileNameInvalid => "minecraft_profile.name_invalid",
    MinecraftProfileNameTaken => "minecraft_profile.name_taken",
    MinecraftProfileLimitExceeded => "minecraft_profile.limit_exceeded",
    MinecraftProfileDeleteForbidden => "minecraft_profile.delete_forbidden",

    // Minecraft texture asset and binding errors.
    MinecraftTextureNotFound => "minecraft_texture.not_found",
    MinecraftTextureInvalidType => "minecraft_texture.invalid_type",
    MinecraftTextureUploadDisabled => "minecraft_texture.upload_disabled",
    MinecraftTextureInvalidPng => "minecraft_texture.invalid_png",
    MinecraftTextureInvalidDimensions => "minecraft_texture.invalid_dimensions",
    MinecraftTextureInvalidModel => "minecraft_texture.invalid_model",
    MinecraftTextureUnsupportedMime => "minecraft_texture.unsupported_mime",
    MinecraftTextureTooLarge => "minecraft_texture.too_large",
    MinecraftObjectStorageFailed => "minecraft_texture.storage_failed",
    MinecraftTextureBindConflict => "minecraft_texture.bind_conflict",

    // Wardrobe-specific texture library errors.
    WardrobeTextureNotFound => "wardrobe.texture_not_found",
    WardrobeTextureTypeMismatch => "wardrobe.texture_type_mismatch",
    WardrobeTextureDeleteConflict => "wardrobe.texture_delete_conflict",
    WardrobeTextureNameInvalid => "wardrobe.texture_name_invalid",
    WardrobeTextureNameTaken => "wardrobe.texture_name_taken",
    TextureLibraryTagNotFound => "texture_library.tag_not_found",
    TextureLibraryTagNameInvalid => "texture_library.tag_name_invalid",
    TextureLibraryTagColorInvalid => "texture_library.tag_color_invalid",
    TextureLibraryTagNameTaken => "texture_library.tag_name_taken",
    TextureLibraryTextureNotFound => "texture_library.texture_not_found",

    // Passkey / WebAuthn errors.
    PasskeyNameInvalid => "passkey.name_invalid",
    PasskeyNameTooLong => "passkey.name_too_long",
    PasskeyNotDiscoverable => "passkey.not_discoverable",

    // User profile and avatar errors.
    AvatarNotFound => "avatar.not_found",
    AvatarFileRequired => "avatar.file_required",
    AvatarUploadReadFailed => "avatar.upload_read_failed",
    AvatarEmptyImage => "avatar.empty_image",
    AvatarSourceInvalid => "avatar.source_invalid",
    AvatarSizeInvalid => "avatar.size_invalid",
    AvatarRenderFailed => "avatar.render_failed",
    AvatarOutputInvalid => "avatar.output_invalid",

    // Public frontend/bootstrap errors.
    ConfigPublicSiteUrlRequired => "config.public_site_url_required",
    ConfigPublicSiteUrlInvalid => "config.public_site_url_invalid",
    FrontendConfigUnavailable => "frontend_config.unavailable",
}

impl AsRef<str> for AsterErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for AsterErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseAsterErrorCodeError;

impl std::fmt::Display for ParseAsterErrorCodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid Aster error code")
    }
}

impl std::error::Error for ParseAsterErrorCodeError {}

impl FromStr for AsterErrorCode {
    type Err = ParseAsterErrorCodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(ParseAsterErrorCodeError)
    }
}

#[cfg(test)]
mod tests {
    use super::{AsterErrorCode, ParseAsterErrorCodeError};
    use std::collections::HashSet;
    use std::str::FromStr;

    #[test]
    fn serializes_as_stable_wire_value() {
        assert_eq!(
            serde_json::to_value(AsterErrorCode::AuthCredentialsFailed).unwrap(),
            serde_json::json!("auth.credentials_failed")
        );
    }

    #[test]
    fn parses_all_stable_wire_values() {
        for &code in AsterErrorCode::ALL {
            assert_eq!(AsterErrorCode::parse(code.as_str()), Some(code));
        }
        assert_eq!(AsterErrorCode::parse("AuthCredentialsFailed"), None);
    }

    #[test]
    fn display_as_ref_and_from_str_use_stable_wire_values() {
        let code = AsterErrorCode::RateLimited;

        assert_eq!(code.as_ref(), "rate_limited");
        assert_eq!(code.to_string(), "rate_limited");
        assert_eq!(
            AsterErrorCode::from_str("rate_limited").unwrap(),
            AsterErrorCode::RateLimited
        );
        assert!(AsterErrorCode::from_str("RATE_LIMITED").is_err());
    }

    #[test]
    fn stable_wire_values_are_unique() {
        let mut seen = HashSet::new();
        for &code in AsterErrorCode::ALL {
            assert!(
                seen.insert(code.as_str()),
                "duplicate AsterErrorCode wire value: {}",
                code.as_str()
            );
        }
    }

    #[test]
    fn project_api_domains_have_specific_error_codes() {
        for domain in [
            "auth.",
            "external_auth.",
            "mail.",
            "config.",
            "audit_log.",
            "task.",
            "minecraft_profile.",
            "minecraft_texture.",
            "wardrobe.",
            "frontend_config.",
        ] {
            assert!(
                AsterErrorCode::ALL
                    .iter()
                    .any(|code| code.as_str().starts_with(domain)),
                "missing domain-specific error code for {domain}"
            );
        }
    }

    #[test]
    fn deserializes_only_known_stable_wire_values() {
        assert_eq!(
            serde_json::from_str::<AsterErrorCode>(r#""database.error""#).unwrap(),
            AsterErrorCode::DatabaseError
        );
        assert!(serde_json::from_str::<AsterErrorCode>(r#""database_error""#).is_err());
    }

    #[test]
    fn parse_error_implements_display_and_error() {
        let error = ParseAsterErrorCodeError;
        assert_eq!(error.to_string(), "invalid Aster error code");
        let dyn_error: &dyn std::error::Error = &error;
        assert_eq!(dyn_error.to_string(), "invalid Aster error code");
    }
}
