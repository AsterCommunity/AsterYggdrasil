//! External authentication service entrypoint.
//!
//! These modules own provider administration, login callbacks, email verification,
//! account linking, and cleanup tasks. The public surface stays aggregated through
//! `crate::services::auth::external::*` so route code does not need to know
//! the internal module split.

mod links;
mod login;
mod normalize;
mod password_link;
mod providers;
mod resolution;
mod verification;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use aster_forge_api::NullablePatch;
use aster_yggdrasil_model::entities::user;
use aster_yggdrasil_model::types::{
    external_auth::ExternalAuthProtocol, external_auth::ExternalAuthProviderKind,
    external_auth::ExternalAuthProviderOptions,
};

pub use links::{cleanup_expired_flows, delete_link, list_links, list_links_paginated};
pub use login::{finish_callback, start_login};
pub use normalize::callback_redirect_uri;
pub use password_link::link_with_password;
pub use providers::{
    create_provider, delete_provider, get_admin_provider, list_admin_providers,
    list_provider_kinds, list_public_providers, list_public_providers_by_kind,
    list_public_providers_by_kind_paginated, list_public_providers_paginated, test_provider,
    test_provider_params, update_provider,
};
pub use verification::{confirm_email_verification, start_email_verification};

const DEFAULT_SCOPES: &str = "openid email profile";
const FLOW_TTL_SECS: u64 = 300;
const EMAIL_VERIFICATION_FLOW_TTL_SECS: u64 = 1_800;
const REDACTED_SECRET: &str = "***REDACTED***";
const EXTERNAL_AUTH_USER_PASSWORD_BYTES: usize = 48;
const EXTERNAL_AUTH_IDENTITY_NAMESPACE_MAX_LEN: usize = 512;
const EXTERNAL_AUTH_URL_MAX_LEN: usize = 2048;
const USERNAME_MAX_LEN: usize = 16;
const USERNAME_MIN_LEN: usize = 4;

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Non-sensitive view of an enabled external auth provider, safe for the public
/// login page. Secrets and endpoint configuration are never included here.
pub struct ExternalAuthPublicProvider {
    /// Stable public identifier of the provider; part of the login and callback
    /// route paths and matched case-insensitively against the stored key.
    pub key: String,
    /// Protocol family the provider speaks (OIDC, GitHub, Google, Microsoft, QQ).
    pub kind: ExternalAuthProviderKind,
    /// Human-readable name shown on the login page.
    pub display_name: String,
    /// Optional logo URL rendered next to the provider button.
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Capability metadata for one provider kind, built from its registry
/// descriptor. Drives which fields the admin provider form must show.
pub struct ExternalAuthProviderKindInfo {
    /// Provider kind this descriptor describes.
    pub kind: ExternalAuthProviderKind,
    /// Underlying protocol (OIDC or generic OAuth 2.0).
    pub protocol: ExternalAuthProtocol,
    /// Human-readable kind name for admin UI.
    pub display_name: String,
    /// Short explanation of the kind shown in admin UI.
    pub description: String,
    /// Space-separated OAuth scopes applied when the admin leaves scopes empty.
    pub default_scopes: String,
    /// Whether `issuer_url` must be configured for this kind.
    pub issuer_url_required: bool,
    /// Whether authorization/token/userinfo endpoints can be set manually
    /// instead of being derived from discovery or fixed per kind.
    pub manual_endpoint_configuration_supported: bool,
    /// Whether `authorization_url` must be configured for this kind.
    pub authorization_url_required: bool,
    /// Whether `token_url` must be configured for this kind.
    pub token_url_required: bool,
    /// Whether `userinfo_url` must be configured for this kind.
    pub userinfo_url_required: bool,
    /// Whether the driver performs OIDC discovery against `issuer_url`.
    pub supports_discovery: bool,
    /// Whether the driver uses PKCE for the authorization code flow.
    pub supports_pkce: bool,
    /// Whether the provider exposes an email-verified claim that can be
    /// honored by `require_email_verified`.
    pub supports_email_verified_claim: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Body for starting an external auth login flow.
pub struct ExternalAuthStartLoginRequest {
    /// In-app path to redirect to after the login completes; normalized and
    /// persisted on the login flow record.
    pub return_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Response carrying the provider URL the browser must be redirected to.
pub struct ExternalAuthStartLoginResponse {
    /// Absolute authorization endpoint URL including client id, state, and
    /// (when supported) PKCE challenge and nonce.
    pub authorization_url: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Body for submitting the email address to verify inside a pending
/// external auth email verification flow.
pub struct ExternalAuthEmailVerificationStartRequest {
    /// Opaque `oev_` flow token issued with the
    /// [`ExternalAuthCallbackOutcome::EmailVerificationRequired`] outcome.
    pub flow_token: String,
    /// Email address to verify; must be allowed by the provider's domain
    /// allowlist when one is configured.
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Body for linking a pending external identity to an existing local account
/// by proving the account password instead of going through email verification.
pub struct ExternalAuthPasswordLinkRequest {
    /// Opaque `oev_` flow token issued with the
    /// [`ExternalAuthCallbackOutcome::EmailVerificationRequired`] outcome.
    pub flow_token: String,
    /// Local username or email of the account to link the identity to.
    pub identifier: String,
    /// Plaintext account password; verified against the stored Argon2 hash and
    /// never persisted.
    pub password: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Response confirming the verification email was queued.
pub struct ExternalAuthEmailVerificationStartResponse {
    /// Human-readable status message for the client.
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Query parameters accepted by the email verification confirm endpoint
/// (the target of the link sent in the verification email).
pub struct ExternalAuthEmailVerificationConfirmQuery {
    /// Single-use verification token from the email link; consumed on first
    /// successful confirm.
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Query parameters the provider appends to the OAuth2 redirect callback.
pub struct ExternalAuthCallbackQuery {
    /// Authorization code to exchange for tokens; required on success.
    pub code: Option<String>,
    /// Opaque state value; hashed and matched against the persisted login flow.
    pub state: Option<String>,
    /// Error code returned by the provider when authorization failed.
    pub error: Option<String>,
    /// Optional human-readable detail accompanying `error`.
    pub error_description: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// One external identity linked to a user, as returned by the account
/// "connected accounts" listing endpoints.
pub struct ExternalAuthLinkInfo {
    /// Identity row id, used when deleting the link.
    pub id: i64,
    /// Provider row the identity belongs to.
    pub provider_id: i64,
    /// Public key of the provider at link time.
    pub provider_key: String,
    /// Protocol family of the provider at link time.
    pub provider_kind: ExternalAuthProviderKind,
    /// Provider display name at link time.
    pub provider_display_name: String,
    /// Provider logo URL at link time.
    pub provider_icon_url: Option<String>,
    /// Issuer namespace that scoped the subject (identity_namespace column).
    pub issuer: String,
    /// Stable subject identifier the provider assigned to the account.
    pub subject: String,
    /// Email captured from the most recent login claims, if any.
    pub email_snapshot: Option<String>,
    /// Display name captured from the most recent login claims, if any.
    pub display_name_snapshot: Option<String>,
    /// When the link was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<Utc>,
    /// When the link row was last updated (snapshot refresh or rename).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<Utc>,
    /// Last successful login through this identity; null until first use.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_login_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone)]
/// Result of a resolved external auth login: the authenticated user plus the
/// context routes and audit need to record the login.
pub struct ExternalAuthPrimaryLogin {
    /// Authenticated local user account.
    pub user: user::Model,
    /// Normalized in-app path to redirect to; defaults to `/` when the flow
    /// was started without one.
    pub return_path: String,
    /// Provider key the login went through.
    pub provider_key: String,
    /// Issuer namespace that scoped the subject.
    pub issuer: String,
    /// Subject identifier the provider assigned.
    pub subject: String,
    /// True when this login created a new identity link (first link, email
    /// auto-link, or auto-provision); false when an existing link matched.
    pub linked: bool,
    /// True when the local user account itself was created during this login
    /// by auto-provisioning, not just the identity link.
    pub auto_provisioned: bool,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Admin view of a configured external auth provider. The client secret is
/// always redacted to [`REDACTED_SECRET`]; use `client_secret_configured` to
/// know whether a real secret is stored.
pub struct AdminExternalAuthProviderInfo {
    /// Provider row id used in admin update/delete/test routes.
    pub id: i64,
    /// Stable public identifier used in login and callback routes.
    pub key: String,
    /// Provider kind (OIDC, GitHub, Google, Microsoft, QQ).
    pub provider_kind: ExternalAuthProviderKind,
    /// Protocol family the kind implements.
    pub protocol: ExternalAuthProtocol,
    /// Human-readable name shown on the login page.
    pub display_name: String,
    /// Optional logo URL shown next to the provider button.
    pub icon_url: Option<String>,
    /// Kind-specific options; for Microsoft providers this carries the tenant.
    pub options: ExternalAuthProviderOptions,
    /// Issuer/discovery base URL; `None` for kinds with fixed endpoints or
    /// when Microsoft options carry the tenant instead.
    pub issuer_url: Option<String>,
    /// Manually configured authorization endpoint; only for kinds that
    /// support manual endpoint configuration.
    pub authorization_url: Option<String>,
    /// Manually configured token endpoint; only for kinds that support
    /// manual endpoint configuration.
    pub token_url: Option<String>,
    /// Manually configured userinfo endpoint; only for kinds that support
    /// manual endpoint configuration.
    pub userinfo_url: Option<String>,
    /// OAuth client id registered at the provider.
    pub client_id: String,
    /// Redacted placeholder; the real secret is never returned to clients.
    pub client_secret: Option<String>,
    /// Whether a non-empty client secret is stored.
    pub client_secret_configured: bool,
    /// Space-separated OAuth scopes requested during login.
    pub scopes: String,
    /// Whether logins through this provider are accepted.
    pub enabled: bool,
    /// Whether unknown identities create a new local user automatically.
    pub auto_provision_enabled: bool,
    /// Whether a verified provider email may auto-link to an existing local
    /// account with the same email.
    pub auto_link_verified_email_enabled: bool,
    /// Whether logins are rejected unless the provider asserts a verified email.
    pub require_email_verified: bool,
    /// Claim name override for the subject; `None` uses the driver default.
    pub subject_claim: Option<String>,
    /// Claim name override for the preferred username.
    pub username_claim: Option<String>,
    /// Claim name override for the display name.
    pub display_name_claim: Option<String>,
    /// Claim name override for the email.
    pub email_claim: Option<String>,
    /// Claim name override for the email-verified flag.
    pub email_verified_claim: Option<String>,
    /// Claim name override for group memberships.
    pub groups_claim: Option<String>,
    /// Claim name override for the avatar URL.
    pub avatar_url_claim: Option<String>,
    /// Restrict logins to these email domains; empty means all domains allowed.
    pub allowed_domains: Vec<String>,
    /// When the provider was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<Utc>,
    /// When the provider was last updated (config change or successful test).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Payload for creating an external auth provider. Absent optional flags fall
/// back to the defaults in `create_provider`.
pub struct CreateExternalAuthProviderInput {
    /// Kind the new provider will use; picks protocol and endpoint defaults.
    pub provider_kind: ExternalAuthProviderKind,
    /// Human-readable name shown on the login page.
    pub display_name: String,
    /// Optional logo URL shown next to the provider button.
    pub icon_url: Option<String>,
    /// Kind-specific options (Microsoft tenant); defaults per kind when absent.
    pub options: Option<ExternalAuthProviderOptions>,
    /// Issuer/discovery base URL; required for OIDC and generic OAuth2.
    pub issuer_url: Option<String>,
    /// Manual authorization endpoint; only honored by kinds that support it.
    pub authorization_url: Option<String>,
    /// Manual token endpoint; only honored by kinds that support it.
    pub token_url: Option<String>,
    /// Manual userinfo endpoint; only honored by kinds that support it.
    pub userinfo_url: Option<String>,
    /// OAuth client id registered at the provider.
    pub client_id: String,
    /// OAuth client secret stored server-side; never echoed back in full.
    pub client_secret: Option<String>,
    /// Space-separated scopes; falls back to the kind default when absent.
    pub scopes: Option<String>,
    /// Whether logins are accepted; defaults to enabled.
    pub enabled: Option<bool>,
    /// Whether unknown identities create local users automatically.
    pub auto_provision_enabled: Option<bool>,
    /// Whether verified provider emails may auto-link existing accounts.
    pub auto_link_verified_email_enabled: Option<bool>,
    /// Whether unverified-email logins are rejected; defaults per kind.
    pub require_email_verified: Option<bool>,
    /// Claim name override for the subject.
    pub subject_claim: Option<String>,
    /// Claim name override for the preferred username.
    pub username_claim: Option<String>,
    /// Claim name override for the display name.
    pub display_name_claim: Option<String>,
    /// Claim name override for the email.
    pub email_claim: Option<String>,
    /// Claim name override for the email-verified flag.
    pub email_verified_claim: Option<String>,
    /// Claim name override for group memberships.
    pub groups_claim: Option<String>,
    /// Claim name override for the avatar URL.
    pub avatar_url_claim: Option<String>,
    /// Restrict logins to these email domains; empty/absent allows all.
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Partial update payload for an external auth provider. Nullable fields use
/// `NullablePatch` so JSON `null` clears a value while omission keeps it.
pub struct UpdateExternalAuthProviderInput {
    /// New display name; unchanged when absent.
    pub display_name: Option<String>,
    /// Logo URL; `null` clears it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub icon_url: Option<NullablePatch<String>>,
    /// Issuer/discovery URL; `null` clears it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub issuer_url: Option<NullablePatch<String>>,
    /// Manual authorization endpoint; `null` clears it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub authorization_url: Option<NullablePatch<String>>,
    /// Manual token endpoint; `null` clears it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub token_url: Option<NullablePatch<String>>,
    /// Manual userinfo endpoint; `null` clears it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub userinfo_url: Option<NullablePatch<String>>,
    /// Kind-specific options (Microsoft tenant); replaces the stored options.
    pub options: Option<ExternalAuthProviderOptions>,
    /// New OAuth client id; unchanged when absent.
    pub client_id: Option<String>,
    /// Client secret; `null` clears it, the redacted placeholder keeps the
    /// stored secret, a new value replaces it.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub client_secret: Option<NullablePatch<String>>,
    /// New space-separated scope list; unchanged when absent.
    pub scopes: Option<String>,
    /// Whether logins are accepted; unchanged when absent.
    pub enabled: Option<bool>,
    /// Whether unknown identities auto-create local users.
    pub auto_provision_enabled: Option<bool>,
    /// Whether verified emails auto-link existing accounts.
    pub auto_link_verified_email_enabled: Option<bool>,
    /// Whether unverified-email logins are rejected.
    pub require_email_verified: Option<bool>,
    /// Subject claim override; `null` restores the driver default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub subject_claim: Option<NullablePatch<String>>,
    /// Preferred-username claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub username_claim: Option<NullablePatch<String>>,
    /// Display-name claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub display_name_claim: Option<NullablePatch<String>>,
    /// Email claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub email_claim: Option<NullablePatch<String>>,
    /// Email-verified claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub email_verified_claim: Option<NullablePatch<String>>,
    /// Groups claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub groups_claim: Option<NullablePatch<String>>,
    /// Avatar-URL claim override; `null` restores the default.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub avatar_url_claim: Option<NullablePatch<String>>,
    /// Domain allowlist; `null` clears the restriction.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<Vec<String>>)
    )]
    pub allowed_domains: Option<NullablePatch<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Draft provider configuration for the connectivity test endpoint, letting
/// admins validate settings before persisting a provider row.
pub struct ExternalAuthProviderTestParamsInput {
    /// Kind the draft provider would use.
    pub provider_kind: ExternalAuthProviderKind,
    /// Kind-specific options (Microsoft tenant).
    pub options: Option<ExternalAuthProviderOptions>,
    /// Issuer/discovery URL; for Microsoft drafts this may carry the tenant.
    pub issuer_url: Option<String>,
    /// Draft manual authorization endpoint.
    pub authorization_url: Option<String>,
    /// Draft manual token endpoint.
    pub token_url: Option<String>,
    /// Draft manual userinfo endpoint.
    pub userinfo_url: Option<String>,
    /// OAuth client id to test with.
    pub client_id: String,
    /// OAuth client secret to test with; never persisted.
    pub client_secret: Option<String>,
    /// Space-separated scopes; falls back to the kind default when absent.
    pub scopes: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// One step of the provider connectivity test result.
pub struct ExternalAuthProviderTestCheck {
    /// Stable check name (e.g. discovery, token endpoint, JWKS fetch).
    pub name: String,
    /// Whether this individual check passed.
    pub success: bool,
    /// Human-readable outcome detail, including failure cause.
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Aggregated result of running the provider connectivity test.
pub struct ExternalAuthProviderTestResult {
    /// Provider label the driver reported (stored key or "draft").
    pub provider: String,
    /// Issuer discovered from the configuration, if discovery succeeded.
    pub issuer: Option<String>,
    /// Discovered authorization endpoint, if any.
    pub authorization_endpoint: Option<String>,
    /// Discovered token endpoint, if any.
    pub token_endpoint: Option<String>,
    /// Discovered userinfo endpoint, if any.
    pub userinfo_endpoint: Option<String>,
    /// Number of keys in the fetched JWKS, if it was fetched.
    pub jwks_key_count: Option<usize>,
    /// Individual probe outcomes; inspect `success` per check.
    pub checks: Vec<ExternalAuthProviderTestCheck>,
}

#[derive(Clone, Debug, Serialize)]
/// Audit-log details recorded for a completed external auth login.
pub struct ExternalAuthLoginAuditDetails<'a> {
    /// Provider key the login went through.
    pub provider_key: &'a str,
    /// Issuer namespace that scoped the subject.
    pub issuer: &'a str,
    /// Subject identifier the provider assigned.
    pub subject: &'a str,
    /// Whether the login created a new identity link.
    pub linked: bool,
    /// Whether the login created the local user account itself.
    pub auto_provisioned: bool,
}

#[derive(Clone, Debug, Serialize)]
/// Audit-log details recorded when a provider is created, updated, or deleted.
pub struct ExternalAuthProviderAuditDetails<'a> {
    /// Provider key affected by the change.
    pub key: &'a str,
    /// Provider kind affected by the change.
    pub kind: ExternalAuthProviderKind,
    /// Logo URL after the change.
    pub icon_url: Option<&'a str>,
    /// Issuer URL after the change.
    pub issuer_url: Option<&'a str>,
    /// Whether logins were accepted after the change.
    pub enabled: bool,
    /// Auto-provisioning flag after the change.
    pub auto_provision_enabled: bool,
    /// Verified-email auto-link flag after the change.
    pub auto_link_verified_email_enabled: bool,
    /// Verified-email requirement after the change.
    pub require_email_verified: bool,
}

/// Returned when a callback cannot complete without the user proving a local
/// email address. The flow token stays valid until the flow expires
/// (`EMAIL_VERIFICATION_FLOW_TTL_SECS`, 30 minutes).
pub struct PendingExternalAuthEmailVerification {
    /// Opaque `oev_` flow token identifying the pending verification flow;
    /// only its hash is stored server-side.
    pub flow_token: String,
    /// In-app path to redirect to once verification completes.
    pub return_path: String,
}

/// Successful outcome of confirming the emailed verification token.
pub struct ExternalAuthEmailVerificationConfirmResult {
    /// Login resolved from the verified flow; links or provisions as needed.
    pub primary_login: ExternalAuthPrimaryLogin,
}

/// Successful outcome of linking a pending identity via account password.
pub struct ExternalAuthPasswordLinkResult {
    /// Login resolved from the consumed flow; the identity is linked to the
    /// authenticated account.
    pub primary_login: ExternalAuthPrimaryLogin,
}

#[expect(
    clippy::large_enum_variant,
    reason = "one-shot service-to-route result; boxing would add a heap allocation without shrinking retained state"
)]
/// Terminal state of an OAuth2 callback: either a resolved login or a
/// request to verify a local email address before the login can complete.
pub enum ExternalAuthCallbackOutcome {
    /// Claims resolved to a user; the route layer issues a session.
    Login(ExternalAuthCallbackResult),
    /// No unambiguous user: an email verification flow was created and its
    /// token must be handed back to the client.
    EmailVerificationRequired(PendingExternalAuthEmailVerification),
}

/// Payload of [`ExternalAuthCallbackOutcome::Login`].
pub struct ExternalAuthCallbackResult {
    /// Resolved login to turn into a session.
    pub primary_login: ExternalAuthPrimaryLogin,
}

#[cfg(test)]
mod tests {
    use aster_forge_api::NullablePatch;

    use super::UpdateExternalAuthProviderInput;

    #[test]
    fn update_provider_input_preserves_nullable_patch_state() {
        let omitted: UpdateExternalAuthProviderInput = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.icon_url, None);
        assert_eq!(omitted.allowed_domains, None);

        let patched: UpdateExternalAuthProviderInput = serde_json::from_str(
            r#"{"icon_url":null,"allowed_domains":["example.com","forge.example.com"]}"#,
        )
        .unwrap();
        assert_eq!(patched.icon_url, Some(NullablePatch::Null));
        assert_eq!(
            patched.allowed_domains,
            Some(NullablePatch::Value(vec![
                "example.com".to_string(),
                "forge.example.com".to_string(),
            ]))
        );
    }
}
