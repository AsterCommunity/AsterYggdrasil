//! Authentication API DTOs.

use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use aster_yggdrasil_model::types::user::AvatarSource;

/// Request body for first-time site setup, creating the initial admin account.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct SetupReq {
    /// Username of the initial administrator; 4-16 ASCII letters, digits,
    /// underscores or hyphens.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    /// Administrator email address; validated by the auth email policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
    /// Administrator password; validated by the auth password policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    /// Optional public site URL recorded during setup; the site default is
    /// used when omitted.
    pub public_site_url: Option<String>,
}

/// Request body for creating a new account via email registration.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RegisterReq {
    /// Desired username; 4-16 ASCII letters, digits, underscores or hyphens.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    /// Email address for the new account; validated by the auth email policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
    /// Initial password; validated by the auth password policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    /// Captcha challenge id issued by the captcha endpoint; required when the
    /// register flow enforces captcha.
    pub captcha_challenge_id: Option<String>,
    /// Answer for the captcha challenge paired with `captcha_challenge_id`.
    pub captcha_answer: Option<String>,
}

/// Request body for resending the registration activation email.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ResendRegisterActivationReq {
    /// Username or email address of the pending registration.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub identifier: String,
    /// Captcha challenge id issued by the captcha endpoint; required when the
    /// resend flow enforces captcha.
    pub captcha_challenge_id: Option<String>,
    /// Answer for the captcha challenge paired with `captcha_challenge_id`.
    pub captcha_answer: Option<String>,
}

/// Request body for requesting a change of the current user's email address.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RequestEmailChangeReq {
    /// New email address; validated by the auth email policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub new_email: String,
}

/// Request body for requesting a password reset email.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasswordResetRequestReq {
    /// Email address of the account to reset; unknown addresses are handled
    /// without revealing whether the account exists.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
}

/// Request body for completing a password reset with the emailed token.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasswordResetConfirmReq {
    /// One-time reset token from the password reset email.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub token: String,
    /// Replacement password; validated by the auth password policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub new_password: String,
}

/// Request body for changing the current user's password.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ChangePasswordReq {
    /// Current password for confirmation.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub current_password: String,
    /// Replacement password; validated by the auth password policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub new_password: String,
}

/// Request body for accepting a user invitation, claiming the invited email
/// address with a new account.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AcceptUserInvitationReq {
    /// Username for the new account; 4-16 ASCII letters, digits, underscores
    /// or hyphens.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    /// Initial password; validated by the auth password policy.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    /// Captcha challenge id issued by the captcha endpoint; required when the
    /// invitation flow enforces captcha.
    pub captcha_challenge_id: Option<String>,
    /// Answer for the captcha challenge paired with `captcha_challenge_id`.
    pub captcha_answer: Option<String>,
}

/// Query parameters for the email/contact verification confirm link.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct ContactVerificationConfirmQuery {
    /// One-time verification token from the confirmation email.
    pub token: Option<String>,
}

/// Request body for password login with username or email.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LoginReq {
    /// Username or email address of the account.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub identifier: String,
    /// Account password.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub password: String,
    /// Captcha challenge id issued by the captcha endpoint; required when the
    /// login flow enforces captcha.
    pub captcha_challenge_id: Option<String>,
    /// Answer for the captcha challenge paired with `captcha_challenge_id`.
    pub captcha_answer: Option<String>,
}

/// Request body for rotating an access token with a refresh token.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RefreshReq {
    /// Refresh token from login; consumed and replaced by a new token pair.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub refresh_token: String,
}

/// Request body for revoking a session by its refresh token.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LogoutReq {
    /// Refresh token of the session to revoke.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub refresh_token: String,
}

/// Request body for updating the current user's profile details.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateProfileReq {
    /// New display name, up to 64 characters; an empty value clears the
    /// display name and omitting the field keeps the current one.
    pub display_name: Option<String>,
}

/// Request body for switching the current user's avatar source.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateAvatarSourceReq {
    /// Avatar source to use (upload, Gravatar, or none).
    pub source: AvatarSource,
}

/// Response of the bootstrap check reporting whether the site is initialized.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CheckResp {
    /// True once initial setup has created the first administrator.
    pub initialized: bool,
}

/// Public captcha policy telling clients which flows currently require a
/// captcha challenge.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicCaptchaPolicyResp {
    /// Whether the captcha feature is enabled at all.
    pub enabled: bool,
    /// Whether the login flow requires a captcha.
    pub login_required: bool,
    /// Whether the registration flow requires a captcha.
    pub register_required: bool,
    /// Whether accepting an invitation requires a captcha.
    pub invitation_accept_required: bool,
    /// Whether resending the registration activation email requires a captcha.
    pub register_activation_resend_required: bool,
}

/// Response reporting whether a logout revoked the session.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LogoutResp {
    /// True when the refresh token matched a live session and it was revoked.
    pub revoked: bool,
}

/// Generic response carrying a single human-readable message.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ActionMessageResp {
    /// Message describing the completed action.
    pub message: String,
}

/// Request body for starting a passkey registration ceremony.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyRegisterStartReq {
    /// Optional friendly name for the new passkey, shown in the passkey list.
    pub name: Option<String>,
}

/// Request body for completing a passkey registration ceremony.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyRegisterFinishReq {
    /// Flow id returned by the registration start response.
    pub flow_id: String,
    /// WebAuthn client response for the new credential, as produced by
    /// `navigator.credentials.create()`.
    pub credential: serde_json::Value,
    /// Optional friendly name for the new passkey.
    pub name: Option<String>,
}

/// Request body for renaming an existing passkey.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PatchPasskeyReq {
    /// New friendly name for the passkey.
    pub name: String,
}

/// Request body for starting a passkey login ceremony.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyLoginStartReq {
    /// Optional username or email hint; discovery of the credential stays
    /// resident-key based.
    pub identifier: Option<String>,
    /// When true the challenge is issued for browser conditional mediation
    /// (autofill) instead of an explicit modal.
    pub conditional: Option<bool>,
}

/// Request body for completing a passkey login ceremony.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyLoginFinishReq {
    /// Flow id returned by the login start response.
    pub flow_id: String,
    /// WebAuthn client response asserting the credential, as produced by
    /// `navigator.credentials.get()`.
    pub credential: serde_json::Value,
}
