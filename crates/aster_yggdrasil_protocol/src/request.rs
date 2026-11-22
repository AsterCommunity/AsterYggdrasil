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
    #[validate(custom(function = "crate::validation::validate_yggdrasil_agent_name"))]
    pub name: String,
    #[validate(range(min = 1, max = 1, message = "agent version must be 1"))]
    pub version: i32,
}

/// `POST /authserver/authenticate` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilAuthenticateReq {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub username: String,
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub password: String,
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    #[validate(nested)]
    pub agent: Option<YggdrasilAgentReq>,
}

/// `POST /authserver/refresh` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilRefreshReq {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser", default)]
    pub request_user: bool,
    #[validate(nested)]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<YggdrasilProfile>,
}

/// Shared access-token request body for `/validate` and `/invalidate`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilTokenReq {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
}

/// `POST /authserver/signout` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilSignoutReq {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub username: String,
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub password: String,
}

/// `POST /sessionserver/session/minecraft/join` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilJoinReq {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[validate(custom(function = "crate::validation::validate_unsigned_uuid"))]
    #[serde(rename = "selectedProfile")]
    pub selected_profile: String,
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
}

/// Query parameters for `GET /sessionserver/session/minecraft/hasJoined`.
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilHasJoinedQuery {
    #[validate(custom(function = "crate::validation::validate_minecraft_profile_name"))]
    pub username: String,
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[serde(rename = "serverId")]
    pub server_id: String,
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    pub ip: Option<String>,
}

/// Query parameters for profile lookup (`unsigned` toggle).
#[derive(Debug, Clone, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfileQuery {
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
