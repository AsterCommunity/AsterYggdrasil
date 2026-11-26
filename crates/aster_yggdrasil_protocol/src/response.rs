//! Protocol response bodies.
//!
//! Field names and omission rules here are wire contract, not style: launchers
//! and authlib-injector depend on the exact camelCase keys, and metadata
//! feature flags use dotted keys inside `meta`.

use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::profile::{YggdrasilProfile, YggdrasilUser};

/// Protocol error body used by authserver and sessionserver error responses.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilErrorBody {
    /// Stable protocol error name, for example `ForbiddenOperationException`.
    pub error: &'static str,
    /// Human-readable protocol message for the error.
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    /// Optional machine-readable cause string (legacy Mojang extension).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

/// `POST /authserver/authenticate` response body.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAuthenticateResp {
    /// Session access token; launchers send it back on refresh/validate/join.
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Client token echoed from the request, or newly generated when absent.
    #[serde(rename = "clientToken")]
    pub client_token: String,
    /// All game profiles owned by the account.
    #[serde(rename = "availableProfiles")]
    pub available_profiles: Vec<YggdrasilProfile>,
    /// Profile bound to this token; present only when exactly one profile is
    /// available (or the request forced a specific profile).
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<YggdrasilProfile>,
    /// Authenticated account, present when the request set `requestUser`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<YggdrasilUser>,
}

/// `POST /authserver/refresh` response body.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilRefreshResp {
    /// Replacement access token; the old token no longer validates.
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Client token carried over from the token being refreshed.
    #[serde(rename = "clientToken")]
    pub client_token: String,
    /// Profile bound to the refreshed token; omitted when the token is not
    /// bound to any profile yet.
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<YggdrasilProfile>,
    /// Authenticated account, present when the request set `requestUser`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<YggdrasilUser>,
}

/// `GET /api/profiles/metadata` (authlib-injector) response body.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMetaResp {
    /// Metadata block describing this server and its feature flags.
    pub meta: YggdrasilMeta,
    /// Domain suffixes the client may accept texture URLs from.
    #[serde(rename = "skinDomains")]
    pub skin_domains: Vec<String>,
    /// PEM public key used to verify signed `textures` properties.
    #[serde(rename = "signaturePublickey")]
    pub signature_publickey: String,
}

/// Authlib-injector metadata block served under `meta`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMeta {
    /// Display name of this server shown by launchers.
    #[serde(rename = "serverName")]
    pub server_name: String,
    /// Server implementation identifier, for example `AsterYggdrasil`.
    #[serde(rename = "implementationName")]
    pub implementation_name: String,
    /// Server implementation version string.
    #[serde(rename = "implementationVersion")]
    pub implementation_version: String,
    /// Homepage and registration links; omitted when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<YggdrasilMetaLinks>,
    // authlib-injector's metadata feature flags are dotted keys inside `meta`,
    // not nested objects. Keep this wire shape to match the third-party protocol.
    /// `feature.non_email_login`: accounts can log in with a non-email name.
    #[serde(rename = "feature.non_email_login")]
    pub feature_non_email_login: bool,
    /// `feature.enable_profile_key`: profile key signing is supported.
    #[serde(rename = "feature.enable_profile_key")]
    pub feature_enable_profile_key: bool,
    /// `feature.enable_mojang_anti_features`: Mojang-services compatibility
    /// endpoints (privileges, profanity filter, blocklist, bans) are served.
    #[serde(rename = "feature.enable_mojang_anti_features")]
    pub feature_enable_mojang_anti_features: bool,
    /// `feature.username_check`: the server enforces the profile name charset.
    #[serde(rename = "feature.username_check")]
    pub feature_username_check: bool,
}

/// Server links advertised inside the metadata `links` object.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilMetaLinks {
    /// Public homepage URL of this server.
    pub homepage: String,
    /// Registration page URL; omitted when self-registration is closed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register: Option<String>,
}

/// `GET /minecraftservices/certificate` compatibility response body.
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

/// Key-pair payload body for the Mojang Minecraft-services certificate flow.
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

/// Minecraft services error body naming the rejecting path (Mojang 401 shape).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPathError {
    /// Minecraft services path that rejected the request, matching Mojang's 401 body shape.
    pub path: String,
}

/// Privileges body for the Mojang Minecraft-services privilege check.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivilegesResp {
    /// Effective service privileges for the authenticated account/profile.
    pub privileges: MinecraftServicesPrivileges,
}

/// Player attributes body combining privileges and client feature preferences.
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

/// Per-service privilege flags for the authenticated account/profile.
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

/// Single Mojang Minecraft-services privilege flag.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivilege {
    /// Boolean state for a single Minecraft services privilege.
    pub enabled: bool,
}

/// Profanity filter preference served through the player-attributes endpoint.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesProfanityFilterPreferences {
    /// Whether the client should enable the profanity filter.
    #[serde(rename = "profanityFilterOn")]
    pub profanity_filter_on: bool,
}

/// Social (friends) feature preferences served through player attributes.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesFriendsPreferences {
    /// Friend-list feature state, for example `ENABLED` or `DISABLED`.
    pub friends: MinecraftServicesPreferenceState,
    /// Friend invitation feature state, for example `ENABLED` or `DISABLED`.
    #[serde(rename = "acceptInvites")]
    pub accept_invites: MinecraftServicesPreferenceState,
}

/// Chat feature preferences served through player attributes.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesChatPreferences {
    /// Text chat feature state, for example `ENABLED` or `DISABLED`.
    #[serde(rename = "textCommunication")]
    pub text_communication: MinecraftServicesPreferenceState,
}

/// Wire state string for a client-side feature preference.
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

/// Ban status body served through the Mojang compatibility endpoints.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesBanStatus {
    /// Map of banned service scopes. Empty means no active Minecraft services ban.
    #[serde(rename = "bannedScopes")]
    pub banned_scopes: MinecraftServicesBannedScopes,
}

/// Banned service scopes; an empty object means no active ban.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesBannedScopes {
    /// Multiplayer/server-join access is banned.
    #[serde(rename = "MULTIPLAYER", skip_serializing_if = "is_false")]
    pub multiplayer: bool,
}

/// Privacy blocklist body for the Mojang blocklist compatibility endpoint.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MinecraftServicesPrivacyBlocklistResp {
    /// UUID list of profiles blocked by the authenticated user.
    #[serde(rename = "blockedProfiles")]
    pub blocked_profiles: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::{
        MinecraftServicesBanStatus, MinecraftServicesBannedScopes, YggdrasilErrorBody,
        YggdrasilMeta, YggdrasilMetaLinks, YggdrasilMetaResp,
    };

    #[test]
    fn error_body_serializes_camel_case_with_optional_cause() {
        let body = YggdrasilErrorBody {
            error: "ForbiddenOperationException",
            error_message: "Invalid token.".to_string(),
            cause: None,
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["error"], "ForbiddenOperationException");
        assert_eq!(json["errorMessage"], "Invalid token.");
        assert!(json.get("cause").is_none());
    }

    #[test]
    fn meta_feature_flags_are_dotted_keys() {
        let resp = YggdrasilMetaResp {
            meta: YggdrasilMeta {
                server_name: "Aster".to_string(),
                implementation_name: "AsterYggdrasil".to_string(),
                implementation_version: "0.1.0".to_string(),
                links: Some(YggdrasilMetaLinks {
                    homepage: "https://example.test/".to_string(),
                    register: None,
                }),
                feature_non_email_login: true,
                feature_enable_profile_key: false,
                feature_enable_mojang_anti_features: false,
                feature_username_check: true,
            },
            skin_domains: vec!["textures.example.test".to_string()],
            signature_publickey: "-----BEGIN PUBLIC KEY-----".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["meta"]["feature.non_email_login"], true);
        assert_eq!(json["meta"]["feature.enable_profile_key"], false);
        assert_eq!(json["meta"]["feature.username_check"], true);
        assert_eq!(json["skinDomains"][0], "textures.example.test");
        assert_eq!(json["signaturePublickey"], "-----BEGIN PUBLIC KEY-----");
        assert_eq!(json["meta"]["links"]["homepage"], "https://example.test/");
        assert!(json["meta"]["links"].get("register").is_none());
    }

    #[test]
    fn banned_scopes_omit_multiplayer_key_when_not_banned() {
        let status = MinecraftServicesBanStatus {
            banned_scopes: MinecraftServicesBannedScopes::default(),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert!(json["bannedScopes"].get("MULTIPLAYER").is_none());
    }
}
