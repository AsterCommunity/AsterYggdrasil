//! Local authentication and session service.
//!
//! Split into AsterDrive-style subdomains; the facade keeps every historical
//! `local::` path stable.

pub mod shared;

mod contact_verification;
mod login;
mod password_change;
mod registration;
mod session;
mod token_scope;
mod tokens;

pub use contact_verification::{
    ContactVerificationConfirmResult, PasswordResetRequestResult, RegisterActivationResendOutcome,
    cleanup_expired_contact_verification_tokens, confirm_contact_verification,
    confirm_password_reset, request_email_change, request_password_reset, resend_email_change,
    resend_register_activation,
};
pub use login::{login, logout, refresh};
pub use password_change::{change_password, change_password_with_audit};
pub use registration::{RegisterOutcome, RegisterResponse, register, setup_first_admin};
pub use session::{
    AuthSessionInfo, AuthTokenBundle, AuthTokenResponse, AuthTokenStatus,
    cleanup_expired_auth_sessions, current_user, current_user_from_token,
    issue_tokens_for_authenticated_user, issue_tokens_for_user, issue_tokens_for_user_id,
    list_sessions, list_sessions_cursor, revoke_other_sessions, revoke_session,
};
pub use shared::{
    AuthUserInfo, auth_request_user_info, auth_user_info, hash_password_with_runtime,
    is_email_verified, validate_email, validate_password, validate_username, verify_user_password,
};
pub use tokens::{AuthenticatedClaims, verify_token, verify_typed_token};
pub(crate) use tokens::{peer_ip, user_agent};
