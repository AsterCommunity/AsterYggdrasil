//! Yggdrasil protocol DTOs.

use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilErrorBody {
    pub error: &'static str,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfileProperty {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub name: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub value: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_non_blank"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfile {
    #[validate(custom(function = "crate::api::dto::validation::validate_unsigned_uuid"))]
    pub id: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub name: String,
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<YggdrasilProfileProperty>>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct CurrentMinecraftProfileListQuery {
    #[validate(length(max = 64, message = "query must not exceed 64 characters"))]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilUser {
    pub id: String,
    pub properties: Vec<YggdrasilProfileProperty>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAgentReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_yggdrasil_agent_name"))]
    pub name: String,
    #[validate(range(min = 1, max = 1, message = "agent version must be 1"))]
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAuthenticateReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub password: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    #[validate(nested)]
    pub agent: Option<YggdrasilAgentReq>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAuthenticateResp {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
    #[serde(rename = "availableProfiles")]
    pub available_profiles: Vec<YggdrasilProfile>,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<YggdrasilProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<YggdrasilUser>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilRefreshReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    #[validate(nested)]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<YggdrasilProfile>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilRefreshResp {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<YggdrasilProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<YggdrasilUser>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilTokenReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilSignoutReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilJoinReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_unsigned_uuid"))]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilHasJoinedQuery {
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub username: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_non_blank"))]
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfileQuery {
    pub unsigned: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMetaResp {
    pub meta: YggdrasilMeta,
    #[serde(rename = "skinDomains")]
    pub skin_domains: Vec<String>,
    #[serde(rename = "signaturePublickey")]
    pub signature_publickey: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMeta {
    #[serde(rename = "serverName")]
    pub server_name: String,
    #[serde(rename = "implementationName")]
    pub implementation_name: String,
    #[serde(rename = "implementationVersion")]
    pub implementation_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<YggdrasilMetaLinks>,
    // authlib-injector's metadata feature flags are dotted keys inside `meta`,
    // not nested objects. Keep this wire shape to match the third-party protocol.
    #[serde(rename = "feature.non_email_login")]
    pub feature_non_email_login: bool,
    #[serde(rename = "feature.enable_profile_key")]
    pub feature_enable_profile_key: bool,
    #[serde(rename = "feature.enable_mojang_anti_features")]
    pub feature_enable_mojang_anti_features: bool,
    #[serde(rename = "feature.username_check")]
    pub feature_username_check: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMetaLinks {
    pub homepage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesCertificateResp {
    /// Ephemeral RSA key pair used by the client for signed chat/profile-key flows.
    #[serde(rename = "keyPair")]
    pub key_pair: MinecraftServicesKeyPair,
    /// Signature over the public key. Self-hosted authlib-injector compatibility uses a dummy value.
    #[serde(rename = "publicKeySignature")]
    pub public_key_signature: String,
    /// Newer signature field used by recent clients. Self-hosted compatibility uses a dummy value.
    #[serde(rename = "publicKeySignatureV2")]
    pub public_key_signature_v2: String,
    /// RFC3339 timestamp after which the certificate should no longer be used.
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
    /// RFC3339 timestamp after which the client should refresh the certificate.
    #[serde(rename = "refreshedAfter")]
    pub refreshed_after: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesKeyPair {
    /// PKCS#1 PEM private key returned to the authenticated client.
    #[serde(rename = "privateKey")]
    pub private_key: String,
    /// PKCS#1 PEM public key paired with `privateKey`.
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPathError {
    /// Minecraft services path that rejected the request, matching Mojang's 401 body shape.
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivilegesResp {
    /// Effective service privileges for the authenticated account/profile.
    pub privileges: MinecraftServicesPrivileges,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPlayerAttributesResp {
    /// Effective service privileges for the authenticated account/profile.
    pub privileges: MinecraftServicesPrivileges,
    /// Client-side profanity filter preference.
    #[serde(rename = "profanityFilterPreferences")]
    pub profanity_filter_preferences: MinecraftServicesProfanityFilterPreferences,
    /// Social feature preferences exposed by Minecraft services.
    #[serde(rename = "friendsPreferences")]
    pub friends_preferences: MinecraftServicesFriendsPreferences,
    /// Chat feature preferences exposed by Minecraft services.
    #[serde(rename = "chatPreferences")]
    pub chat_preferences: MinecraftServicesChatPreferences,
    /// Ban scopes currently active for this account/profile.
    #[serde(rename = "banStatus")]
    pub ban_status: MinecraftServicesBanStatus,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivileges {
    /// Whether online chat is allowed.
    #[serde(rename = "onlineChat")]
    pub online_chat: MinecraftServicesPrivilege,
    /// Whether joining multiplayer servers is allowed.
    #[serde(rename = "multiplayerServer")]
    pub multiplayer_server: MinecraftServicesPrivilege,
    /// Whether Realms multiplayer is allowed.
    #[serde(rename = "multiplayerRealms")]
    pub multiplayer_realms: MinecraftServicesPrivilege,
    /// Whether required telemetry is allowed/enabled from the service policy perspective.
    pub telemetry: MinecraftServicesPrivilege,
    /// Whether optional telemetry is allowed/enabled from the service policy perspective.
    #[serde(rename = "optionalTelemetry")]
    pub optional_telemetry: MinecraftServicesPrivilege,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivilege {
    /// Boolean state for a single Minecraft services privilege.
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesProfanityFilterPreferences {
    /// Whether the client should enable the profanity filter.
    #[serde(rename = "profanityFilterOn")]
    pub profanity_filter_on: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesFriendsPreferences {
    /// Friend-list feature state, for example `ENABLED` or `DISABLED`.
    pub friends: MinecraftServicesPreferenceState,
    /// Friend invitation feature state, for example `ENABLED` or `DISABLED`.
    #[serde(rename = "acceptInvites")]
    pub accept_invites: MinecraftServicesPreferenceState,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesChatPreferences {
    /// Text chat feature state, for example `ENABLED` or `DISABLED`.
    #[serde(rename = "textCommunication")]
    pub text_communication: MinecraftServicesPreferenceState,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub enum MinecraftServicesPreferenceState {
    /// Feature is enabled for the authenticated account/profile.
    #[serde(rename = "ENABLED")]
    Enabled,
    /// Feature is disabled for the authenticated account/profile.
    #[serde(rename = "DISABLED")]
    Disabled,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesBanStatus {
    /// Map of banned service scopes. Empty means no active Minecraft services ban.
    #[serde(rename = "bannedScopes")]
    pub banned_scopes: MinecraftServicesBannedScopes,
}

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesBannedScopes {}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivacyBlocklistResp {
    /// UUID list of profiles blocked by the authenticated user.
    #[serde(rename = "blockedProfiles")]
    pub blocked_profiles: Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateMinecraftProfileReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RenameMinecraftProfileReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_minecraft_profile_name"))]
    pub name: String,
}
