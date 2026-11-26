//! Admin API DTOs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::services::auth::external::{
    CreateExternalAuthProviderInput, ExternalAuthProviderTestParamsInput,
    UpdateExternalAuthProviderInput,
};
use crate::services::ops::config::ConfigActionType;
use crate::services::yggdrasil::session_forward::{
    CreateYggdrasilSessionForwardServerInput, UpdateYggdrasilSessionForwardServerInput,
};
use aster_forge_api::NullablePatch;
use aster_forge_config::ConfigValue;
use aster_forge_config::ConfigVisibility;
use aster_yggdrasil_model::types::{
    external_auth::ExternalAuthProviderKind, external_auth::ExternalAuthProviderOptions,
    task::BackgroundTaskKind, task::BackgroundTaskStatus, user::OperatorScope, user::UserBanScope,
    user::UserBanStatus, user::UserRole, user::UserStatus,
    yggdrasil::YggdrasilSessionForwardEndpointKind, yggdrasil::YggdrasilSessionForwardServerSortBy,
};

/// Admin request body for setting a runtime config value.
#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct SetConfigReq {
    /// New value to store for the config key; validated against the key's
    /// configured value type.
    pub value: ConfigValue,
    /// Optional visibility override; when omitted the stored visibility is
    /// kept.
    pub visibility: Option<ConfigVisibility>,
}

/// Admin request body for running a config action, such as a test email or a
/// Yggdrasil signature key rotation.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExecuteConfigActionReq {
    /// Action to execute; accepted actions depend on the targeted config
    /// action key.
    pub action: ConfigActionType,
    /// Optional action parameters keyed by config key (e.g. the test-email
    /// recipient or captcha preview overrides).
    pub values: Option<BTreeMap<String, ConfigValue>>,
}

/// Admin response describing the outcome of a config action.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExecuteConfigActionResp {
    /// Human-readable result message suitable for display in the admin UI.
    pub message: String,
    /// Optional output produced by the action; absent when the action has no
    /// return value.
    pub value: Option<String>,
}

/// Response carrying the number of records removed by a bulk operation.
#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RemovedCountResponse {
    /// Count of records the operation deleted.
    pub removed: u64,
}

/// Cursor query for the admin background task list, ordered by update time
/// with id as tie breaker.
#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminTaskListQuery {
    /// Optional task kind filter.
    pub kind: Option<BackgroundTaskKind>,
    /// Optional task status filter.
    pub status: Option<BackgroundTaskStatus>,
    /// Cursor: update timestamp of the last item on the previous page.
    pub after_updated_at: Option<DateTime<Utc>>,
    /// Cursor: id of the last item, used as a stable tie breaker.
    pub after_id: Option<i64>,
}

/// Admin request body for bulk-deleting finished background tasks.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AdminTaskCleanupReq {
    /// Deletes only tasks that finished before this timestamp.
    pub finished_before: DateTime<Utc>,
    /// Optional task kind filter; all kinds are cleaned when omitted.
    pub kind: Option<BackgroundTaskKind>,
    /// Optional task status filter.
    pub status: Option<BackgroundTaskStatus>,
}

/// Cursor query for the admin user list, ordered by creation time with id as
/// tie breaker.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminUserListQuery {
    /// Optional search term matched case-insensitively against username and
    /// email; up to 96 characters.
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    /// Optional role filter.
    pub role: Option<UserRole>,
    /// Optional account status filter.
    pub status: Option<UserStatus>,
    /// Cursor: creation timestamp of the last item on the previous page.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Cursor: id of the last item, used as a stable tie breaker.
    pub after_id: Option<i64>,
}

/// Cursor query for the admin ban list, ordered by creation time with id as
/// tie breaker.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminUserBanListQuery {
    /// Optional filter restricting results to bans of this user.
    pub user_id: Option<i64>,
    /// Optional ban scope filter.
    pub scope: Option<UserBanScope>,
    /// Optional stored ban status filter.
    pub status: Option<UserBanStatus>,
    /// When true, only bans whose effective status is currently active are
    /// returned.
    #[serde(default)]
    pub effective_only: bool,
    /// Cursor: creation timestamp of the last item on the previous page.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Cursor: id of the last item, used as a stable tie breaker.
    pub after_id: Option<i64>,
}

/// Admin request body for creating a new capability ban on a user.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateUserBanReq {
    /// Capability scopes the user loses while the ban is effective; must be
    /// non-empty.
    pub scopes: Vec<UserBanScope>,
    /// Internal reason recorded for moderators; 1-128 characters.
    #[validate(length(min = 1, max = 128, message = "reason must be 1-128 characters"))]
    pub reason: String,
    /// Optional reason shown to the banned user; up to 1000 characters.
    #[validate(length(max = 1000, message = "public_reason must not exceed 1000 characters"))]
    pub public_reason: Option<String>,
    /// Optional moderator-only note; up to 1000 characters.
    #[validate(length(max = 1000, message = "admin_note must not exceed 1000 characters"))]
    pub admin_note: Option<String>,
    /// Optional ban start time; defaults to now when omitted.
    pub starts_at: Option<DateTime<Utc>>,
    /// Optional expiry time; omit for a ban that lasts until revoked.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Admin request body for partially updating an existing ban. Omitted fields
/// keep their stored value; nullable fields accept explicit `null` to clear.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateUserBanReq {
    /// Replacement scope set; omitted leaves the scopes unchanged.
    pub scopes: Option<Vec<UserBanScope>>,
    /// Replacement internal reason; 1-128 characters when provided.
    #[validate(length(min = 1, max = 128, message = "reason must be 1-128 characters"))]
    pub reason: Option<String>,
    /// New reason shown to the banned user, `null` to clear, or omit to keep.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub public_reason: Option<NullablePatch<String>>,
    /// New moderator-only note, `null` to clear, or omit to keep.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub admin_note: Option<NullablePatch<String>>,
    /// Replacement start time; omit to keep.
    pub starts_at: Option<DateTime<Utc>>,
    /// New expiry time, `null` to make the ban permanent, or omit to keep.
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub expires_at: Option<NullablePatch<DateTime<Utc>>>,
}

/// Admin request body for revoking an active ban.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RevokeUserBanReq {
    /// Optional note recorded with the revocation; up to 1000 characters.
    #[validate(length(max = 1000, message = "revoke_note must not exceed 1000 characters"))]
    pub revoke_note: Option<String>,
}

/// Admin request body for creating a user account.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateAdminUserReq {
    /// Account username; 4-16 ASCII letters, digits, underscores or hyphens.
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    /// Account email address; must be valid and not already registered.
    #[validate(email(message = "email must be a valid email address"))]
    pub email: String,
    /// Initial password; when omitted the server generates a temporary
    /// password, returns it once, and forces a change at next login.
    pub password: Option<String>,
    /// Force a password change at next login; forced on when the server
    /// generated the password.
    pub must_change_password: Option<bool>,
    /// Account role; falls back to the service default when omitted.
    pub role: Option<UserRole>,
    /// Operator scopes for limited staff accounts.
    pub operator_scopes: Option<Vec<OperatorScope>>,
    /// Initial account status; falls back to the service default when omitted.
    pub status: Option<UserStatus>,
}

/// Admin request body for inviting a new user by email.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateUserInvitationReq {
    /// Recipient email address; must be valid and not already registered.
    #[validate(email(message = "email must be a valid email address"))]
    pub email: String,
}

/// Admin request body for partially updating a user account. Omitted fields
/// keep their current value.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateAdminUserReq {
    /// New username when provided; validated like registration.
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_auth_username"))]
    pub username: Option<String>,
    /// New email address when provided.
    #[validate(email(message = "email must be a valid email address"))]
    pub email: Option<String>,
    /// Replacement password; changing it invalidates the user's existing
    /// sessions.
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_auth_password"))]
    pub password: Option<String>,
    /// New role when provided.
    pub role: Option<UserRole>,
    /// Replacement operator scopes when provided.
    pub operator_scopes: Option<Vec<OperatorScope>>,
    /// New account status when provided.
    pub status: Option<UserStatus>,
    /// Force a password change at next login when provided.
    pub must_change_password: Option<bool>,
}

/// Cursor query for the admin Minecraft profile list, ordered by id.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminMinecraftProfileListQuery {
    /// Optional filter restricting results to profiles owned by this user.
    pub user_id: Option<i64>,
    /// Optional exact profile name filter; must be a valid profile name.
    #[validate(custom(
        function = "crate::api::dto::validation::validate_optional_minecraft_profile_name"
    ))]
    pub name: Option<String>,
    /// Optional exact profile UUID filter; 32-character unsigned hexadecimal.
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_unsigned_uuid"))]
    pub uuid: Option<String>,
    /// Optional search term matched case-insensitively against profile name
    /// and UUID; up to 64 characters.
    #[validate(length(max = 64, message = "query must not exceed 64 characters"))]
    pub query: Option<String>,
    /// Cursor: id of the last item on the previous page.
    pub after_id: Option<i64>,
}

/// Admin request body for creating an external auth provider. Absent optional
/// flags fall back to per-kind defaults.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateExternalAuthProviderReq {
    /// Provider kind selecting the protocol and endpoint defaults.
    pub provider_kind: ExternalAuthProviderKind,
    /// Human-readable name shown on the login page; non-blank, up to 128
    /// bytes.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: String,
    /// Optional logo URL shown next to the provider button.
    pub icon_url: Option<String>,
    /// Kind-specific options (e.g. the Microsoft tenant); kind defaults apply
    /// when omitted.
    pub options: Option<ExternalAuthProviderOptions>,
    /// Issuer/discovery base URL; required by kinds that discover endpoints.
    pub issuer_url: Option<String>,
    /// Manual authorization endpoint; only honored by kinds supporting manual
    /// endpoint configuration.
    pub authorization_url: Option<String>,
    /// Legacy alias for `authorization_url`; ignored when both are provided.
    pub authorize_url: Option<String>,
    /// Manual token endpoint; only honored by kinds supporting manual endpoint
    /// configuration.
    pub token_url: Option<String>,
    /// Manual userinfo endpoint; only honored by kinds supporting manual
    /// endpoint configuration.
    pub userinfo_url: Option<String>,
    /// OAuth client id registered at the provider; non-blank, up to 255 bytes.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: String,
    /// OAuth client secret stored server-side; never echoed back in full.
    pub client_secret: Option<String>,
    /// Space-separated OAuth scopes; falls back to the kind default when
    /// omitted.
    pub scopes: Option<String>,
    /// Whether logins through this provider are accepted; defaults to enabled.
    pub enabled: Option<bool>,
    /// Whether unknown external identities create local users automatically.
    pub auto_provision_enabled: Option<bool>,
    /// Whether a verified provider email may auto-link an existing local
    /// account with the same email.
    pub auto_link_verified_email_enabled: Option<bool>,
    /// Whether logins are rejected unless the provider asserts a verified
    /// email; defaults per kind.
    pub require_email_verified: Option<bool>,
    /// Claim name override for the subject; driver default when omitted.
    pub subject_claim: Option<String>,
    /// Claim name override for the preferred username; driver default when
    /// omitted.
    pub username_claim: Option<String>,
    /// Claim name override for the display name; driver default when omitted.
    pub display_name_claim: Option<String>,
    /// Claim name override for the email; driver default when omitted.
    pub email_claim: Option<String>,
    /// Claim name override for the email-verified flag; driver default when
    /// omitted.
    pub email_verified_claim: Option<String>,
    /// Claim name override for group memberships; driver default when omitted.
    pub groups_claim: Option<String>,
    /// Claim name override for the avatar URL; driver default when omitted.
    pub avatar_url_claim: Option<String>,
    /// Restrict logins to these email domains; absent or empty allows all.
    pub allowed_domains: Option<Vec<String>>,
}

impl From<CreateExternalAuthProviderReq> for CreateExternalAuthProviderInput {
    fn from(value: CreateExternalAuthProviderReq) -> Self {
        Self {
            provider_kind: value.provider_kind,
            display_name: value.display_name,
            icon_url: value.icon_url,
            options: value.options,
            issuer_url: value.issuer_url,
            authorization_url: value.authorization_url.or(value.authorize_url),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_enabled: value.auto_provision_enabled,
            auto_link_verified_email_enabled: value.auto_link_verified_email_enabled,
            require_email_verified: value.require_email_verified,
            subject_claim: value.subject_claim,
            username_claim: value.username_claim,
            display_name_claim: value.display_name_claim,
            email_claim: value.email_claim,
            email_verified_claim: value.email_verified_claim,
            groups_claim: value.groups_claim,
            avatar_url_claim: value.avatar_url_claim,
            allowed_domains: value.allowed_domains,
        }
    }
}

/// Admin request body for partially updating an external auth provider.
/// Omitted fields keep their stored value; nullable fields accept explicit
/// `null` to clear.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateExternalAuthProviderReq {
    /// New display name; up to 128 bytes when provided.
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: Option<String>,
    /// New logo URL, `null` to clear, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub icon_url: Option<NullablePatch<String>>,
    /// New issuer/discovery base URL, `null` to clear, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub issuer_url: Option<NullablePatch<String>>,
    /// New manual authorization endpoint, `null` to clear, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub authorization_url: Option<NullablePatch<String>>,
    /// Legacy alias for `authorization_url`; ignored when both are provided.
    pub authorize_url: Option<String>,
    /// New manual token endpoint, `null` to clear, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub token_url: Option<NullablePatch<String>>,
    /// New manual userinfo endpoint, `null` to clear, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub userinfo_url: Option<NullablePatch<String>>,
    /// Replacement kind-specific options when provided.
    pub options: Option<ExternalAuthProviderOptions>,
    /// New OAuth client id; up to 255 bytes when provided.
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: Option<String>,
    /// New client secret, `null` to remove, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub client_secret: Option<NullablePatch<String>>,
    /// New space-separated OAuth scopes when provided.
    pub scopes: Option<String>,
    /// New login acceptance flag when provided.
    pub enabled: Option<bool>,
    /// New auto-provision flag when provided.
    pub auto_provision_enabled: Option<bool>,
    /// New verified-email auto-link flag when provided.
    pub auto_link_verified_email_enabled: Option<bool>,
    /// New verified-email requirement when provided.
    pub require_email_verified: Option<bool>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub subject_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub username_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub display_name_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub groups_claim: Option<NullablePatch<String>>,
    /// New claim name override, `null` to fall back to the driver default, or
    /// omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub avatar_url_claim: Option<NullablePatch<String>>,
    /// New domain allowlist, `null` to allow all domains, or omit to keep.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<Vec<String>>))]
    pub allowed_domains: Option<NullablePatch<Vec<String>>>,
}

impl From<UpdateExternalAuthProviderReq> for UpdateExternalAuthProviderInput {
    fn from(value: UpdateExternalAuthProviderReq) -> Self {
        Self {
            display_name: value.display_name,
            icon_url: value.icon_url,
            issuer_url: value.issuer_url,
            authorization_url: value
                .authorization_url
                .or_else(|| value.authorize_url.map(NullablePatch::Value)),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            options: value.options,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_enabled: value.auto_provision_enabled,
            auto_link_verified_email_enabled: value.auto_link_verified_email_enabled,
            require_email_verified: value.require_email_verified,
            subject_claim: value.subject_claim,
            username_claim: value.username_claim,
            display_name_claim: value.display_name_claim,
            email_claim: value.email_claim,
            email_verified_claim: value.email_verified_claim,
            groups_claim: value.groups_claim,
            avatar_url_claim: value.avatar_url_claim,
            allowed_domains: value.allowed_domains,
        }
    }
}

/// Admin request body for testing external auth provider parameters against
/// the live provider without persisting them.
#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExternalAuthProviderTestParamsReq {
    /// Provider kind; `provider_kind` wins, `kind` is a legacy alias, and OIDC
    /// is assumed when both are absent.
    pub kind: Option<ExternalAuthProviderKind>,
    /// Provider kind selecting the protocol and endpoint defaults.
    pub provider_kind: Option<ExternalAuthProviderKind>,
    /// Kind-specific options (e.g. the Microsoft tenant).
    pub options: Option<ExternalAuthProviderOptions>,
    /// Issuer/discovery base URL for discovery-capable kinds.
    pub issuer_url: Option<String>,
    /// Manual authorization endpoint for kinds that support it.
    pub authorization_url: Option<String>,
    /// Legacy alias for `authorization_url`; ignored when both are provided.
    pub authorize_url: Option<String>,
    /// Manual token endpoint for kinds that support it.
    pub token_url: Option<String>,
    /// Manual userinfo endpoint for kinds that support it.
    pub userinfo_url: Option<String>,
    /// OAuth client id registered at the provider; non-blank, up to 255 bytes.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: String,
    /// OAuth client secret used for this test call only.
    pub client_secret: Option<String>,
    /// Space-separated OAuth scopes; falls back to the kind default when
    /// omitted.
    pub scopes: Option<String>,
}

impl From<ExternalAuthProviderTestParamsReq> for ExternalAuthProviderTestParamsInput {
    fn from(value: ExternalAuthProviderTestParamsReq) -> Self {
        Self {
            provider_kind: value
                .provider_kind
                .or(value.kind)
                .unwrap_or(ExternalAuthProviderKind::Oidc),
            options: value.options,
            issuer_url: value.issuer_url,
            authorization_url: value.authorization_url.or(value.authorize_url),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
        }
    }
}

/// Cursor query for the admin session-forward server list. The cursor fields
/// used depend on the selected sort order.
#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminYggdrasilSessionForwardServerListQuery {
    /// Page size; clamped to `1..=max` by the handler.
    pub limit: Option<u64>,
    /// Sort order; defaults to call order.
    pub sort_by: Option<YggdrasilSessionForwardServerSortBy>,
    /// Cursor: id of the last item on the previous page.
    pub after_id: Option<i64>,
    /// Cursor: enablement flag of the last item; only used for call-order
    /// sorting.
    pub after_enabled: Option<bool>,
    /// Cursor: priority of the last item; only used for call-order sorting.
    pub after_priority: Option<i32>,
}

impl AdminYggdrasilSessionForwardServerListQuery {
    /// Returns the page size clamped to `1..=max`, or `default` when unset.
    pub fn limit_or(&self, default: u64, max: u64) -> u64 {
        self.limit
            .map(|value| value.clamp(1, max))
            .unwrap_or(default)
    }

    /// Returns the requested sort order or the default call order.
    pub fn sort_by(&self) -> YggdrasilSessionForwardServerSortBy {
        self.sort_by.unwrap_or_default()
    }
}

/// Admin request body for registering an upstream session-forward server.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateYggdrasilSessionForwardServerReq {
    /// Human-readable name; non-blank, up to 128 bytes.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: String,
    /// Base URL of the upstream Yggdrasil API; non-blank and unique across
    /// servers.
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub base_url: String,
    /// Upstream endpoint layout; defaults to authlib-injector when omitted.
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    /// Whether the server participates in forwarding; defaults to enabled.
    pub enabled: Option<bool>,
    /// Call-order position; enabled servers with lower values are tried
    /// first. Defaults to 100.
    pub priority: Option<i32>,
    /// Weight among servers sharing the same priority; defaults to 1.
    pub weight: Option<i32>,
    /// Upstream request timeout in milliseconds; defaults to 1500.
    pub timeout_ms: Option<i32>,
    /// Whether texture requests are also forwarded; defaults to false.
    pub texture_forward_enabled: Option<bool>,
}

impl From<CreateYggdrasilSessionForwardServerReq> for CreateYggdrasilSessionForwardServerInput {
    fn from(value: CreateYggdrasilSessionForwardServerReq) -> Self {
        Self {
            display_name: value.display_name,
            base_url: value.base_url,
            endpoint_kind: value.endpoint_kind,
            enabled: value.enabled,
            priority: value.priority,
            weight: value.weight,
            timeout_ms: value.timeout_ms,
            texture_forward_enabled: value.texture_forward_enabled,
        }
    }
}

/// Admin request body for partially updating a session-forward server.
/// Omitted fields keep their stored value.
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateYggdrasilSessionForwardServerReq {
    /// New display name; up to 128 bytes when provided.
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: Option<String>,
    /// New upstream base URL; must stay unique when provided.
    pub base_url: Option<String>,
    /// New upstream endpoint layout when provided.
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    /// New forwarding participation flag when provided.
    pub enabled: Option<bool>,
    /// New call-order position when provided.
    pub priority: Option<i32>,
    /// New same-priority weight when provided.
    pub weight: Option<i32>,
    /// New upstream request timeout in milliseconds when provided.
    pub timeout_ms: Option<i32>,
    /// New texture-forwarding flag when provided.
    pub texture_forward_enabled: Option<bool>,
}

impl From<UpdateYggdrasilSessionForwardServerReq> for UpdateYggdrasilSessionForwardServerInput {
    fn from(value: UpdateYggdrasilSessionForwardServerReq) -> Self {
        Self {
            display_name: value.display_name,
            base_url: value.base_url,
            endpoint_kind: value.endpoint_kind,
            enabled: value.enabled,
            priority: value.priority,
            weight: value.weight,
            timeout_ms: value.timeout_ms,
            texture_forward_enabled: value.texture_forward_enabled,
        }
    }
}
