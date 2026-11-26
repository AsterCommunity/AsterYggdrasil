//! Protocol request bodies.

use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use validator::Validate;

use crate::profile::YggdrasilProfile;

/// The `agent` object identifying the client product in auth requests.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAgentReq {
    /// Client product name; the protocol only accepts `"Minecraft"`.
    #[validate(custom(function = "crate::validation::validate_yggdrasil_agent_name"))]
    pub name: String,
    /// Client product version; the current protocol generation is fixed at 1.
    #[validate(range(min = 1, max = 1, message = "agent version must be 1"))]
    pub version: i32,
}

/// `POST /authserver/authenticate` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAuthenticateReq {
    /// Account login identifier supplied by the player; hosts decide whether it
    /// carries an email address or a profile/account name.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub username: String,
    /// Account password used to prove the credentials.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub password: String,
    /// Client token from a previous login; echoed back when provided,
    /// otherwise the host generates a fresh one.
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    /// When true, the response also carries the authenticated `user` object.
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    /// Optional launcher identity (`Minecraft`, version 1).
    #[validate(nested)]
    pub agent: Option<YggdrasilAgentReq>,
}

/// `POST /authserver/refresh` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilRefreshReq {
    /// Access token issued by an earlier authenticate/refresh call.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Client token paired with the access token; the request is rejected as
    /// `InvalidToken` when it does not match the stored pairing.
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    /// When true, the response also carries the authenticated `user` object.
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    /// Profile to rebind the token to; required only when the token is not
    /// bound yet, otherwise the binding is kept.
    #[validate(nested)]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<YggdrasilProfile>,
}

/// Shared access-token request body for `/validate` and `/invalidate`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilTokenReq {
    /// Access token being checked or revoked.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Client token paired with the access token; a mismatch invalidates the
    /// token instead of serving it.
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
}

/// `POST /authserver/signout` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilSignoutReq {
    /// Account login identifier; must match the account that holds the tokens.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub username: String,
    /// Account password; invalidates all access tokens issued to the account.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub password: String,
}

/// `POST /sessionserver/session/minecraft/join` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilJoinReq {
    /// Access token authenticating the joining player.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// Profile UUID in unsigned (dash-less) form that is joining the server.
    #[validate(custom(function = "crate::validation::validate_unsigned_uuid"))]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: String,
    /// Server identifier derived from the server's encryption handshake.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
}

/// Query parameters for `GET /sessionserver/session/minecraft/hasJoined`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilHasJoinedQuery {
    /// Player name whose join is being verified by the game server.
    #[validate(custom(function = "crate::validation::validate_minecraft_profile_name"))]
    pub username: String,
    /// Server identifier the player reported during the join handshake.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
    /// Optional player IP; hosts that bind joins to IPs enforce it here.
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    pub ip: Option<String>,
}

/// Query parameters for profile lookup (`unsigned` toggle).
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfileQuery {
    /// When true, the profile is returned without signatures; false requests
    /// the signed `textures` property.
    pub unsigned: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::YggdrasilAuthenticateReq;

    #[test]
    fn authenticate_request_uses_camel_case_keys() {
        let request: YggdrasilAuthenticateReq = serde_json::from_value(serde_json::json!({
            "agent": { "name": "Minecraft", "version": 1 },
            "username": "notch@example.com",
            "password": "hunter2",
            "clientToken": "client-token-value",
            "requestUser": true
        }))
        .unwrap();
        assert_eq!(request.client_token.as_deref(), Some("client-token-value"));
        assert!(request.request_user);
        assert_eq!(request.agent.unwrap().version, 1);
    }
}
