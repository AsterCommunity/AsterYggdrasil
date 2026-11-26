//! Profile and user wire types.

use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;
use validator::Validate;

/// A single profile property, for example the `textures` property.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfileProperty {
    /// Property name, for example `textures`.
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub name: String,
    /// Property payload interpreted by the client per `name` (base64 JSON for
    /// `textures`).
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub value: String,
    /// Yggdrasil signature over `name` and `value` (base64); omitted when the
    /// property is served unsigned.
    #[validate(custom(function = "crate::validation::validate_optional_non_blank"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// A Yggdrasil game profile as it appears in lookups and auth responses.
///
/// `id` is the unsigned (dash-less) lowercase profile UUID required by the
/// protocol.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilProfile {
    /// Unsigned (dash-less) lowercase profile UUID, 32 hex characters.
    #[validate(custom(function = "crate::validation::validate_unsigned_uuid"))]
    pub id: String,
    /// Player name, 3-16 ASCII letters, numbers, or underscores.
    #[validate(custom(function = "crate::validation::validate_minecraft_profile_name"))]
    pub name: String,
    /// Signed profile properties; omitted when the lookup was made unsigned.
    #[validate(nested)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<YggdrasilProfileProperty>>,
}

/// The authenticated user entry returned when a request sets `requestUser`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct YggdrasilUser {
    /// Account identifier in unhyphenated UUID form, distinct from profile ids.
    pub id: String,
    /// Account-level properties; hosts that keep none serialize an empty list.
    pub properties: Vec<YggdrasilProfileProperty>,
}

#[cfg(test)]
mod tests {
    use super::{YggdrasilProfile, YggdrasilProfileProperty};

    #[test]
    fn profile_omits_properties_when_absent() {
        let profile = YggdrasilProfile {
            id: "d8f78c67e4c64b6cae5c6e21687ad818".to_string(),
            name: "Notch".to_string(),
            properties: None,
        };
        let json = serde_json::to_value(&profile).unwrap();
        assert_eq!(json["id"], "d8f78c67e4c64b6cae5c6e21687ad818");
        assert_eq!(json["name"], "Notch");
        assert!(json.get("properties").is_none());
    }

    #[test]
    fn property_signature_is_omitted_when_unsigned() {
        let property = YggdrasilProfileProperty {
            name: "textures".to_string(),
            value: "eyJ0aW1lc3RhbXAiOjF9".to_string(),
            signature: None,
        };
        let json = serde_json::to_value(&property).unwrap();
        assert!(json.get("signature").is_none());
        let signed = YggdrasilProfileProperty {
            signature: Some("c2ln".to_string()),
            ..property
        };
        let json = serde_json::to_value(&signed).unwrap();
        assert_eq!(json["signature"], "c2ln");
    }
}
