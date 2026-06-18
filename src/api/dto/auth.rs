//! Authentication API DTOs.

use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::types::AvatarSource;

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct SetupReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    pub public_site_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RegisterReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    pub captcha_challenge_id: Option<String>,
    pub captcha_answer: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ResendRegisterActivationReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub identifier: String,
    pub captcha_challenge_id: Option<String>,
    pub captcha_answer: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RequestEmailChangeReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub new_email: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasswordResetRequestReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_email"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasswordResetConfirmReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub token: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ChangePasswordReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub current_password: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AcceptUserInvitationReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_password"))]
    pub password: String,
    pub captcha_challenge_id: Option<String>,
    pub captcha_answer: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct ContactVerificationConfirmQuery {
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LoginReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub identifier: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub password: String,
    pub captcha_challenge_id: Option<String>,
    pub captcha_answer: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RefreshReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LogoutReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateProfileReq {
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateAvatarSourceReq {
    pub source: AvatarSource,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CheckResp {
    pub initialized: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PublicCaptchaPolicyResp {
    pub enabled: bool,
    pub login_required: bool,
    pub register_required: bool,
    pub invitation_accept_required: bool,
    pub register_activation_resend_required: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct LogoutResp {
    pub revoked: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ActionMessageResp {
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyRegisterStartReq {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyRegisterFinishReq {
    pub flow_id: String,
    pub credential: serde_json::Value,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PatchPasskeyReq {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyLoginStartReq {
    pub identifier: Option<String>,
    pub conditional: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PasskeyLoginFinishReq {
    pub flow_id: String,
    pub credential: serde_json::Value,
}
