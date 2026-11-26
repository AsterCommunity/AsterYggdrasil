//! The base64-encoded `textures` profile property value.
//!
//! Hosts gather texture entries (URLs, models, the default skin fallback)
//! from their own storage and policy, then hand the assembled payload to
//! [`encode_textures_property_value`]. The wire shape and deterministic key
//! ordering are owned here.

use std::collections::BTreeMap;

use base64::Engine;
use serde::Serialize;

use crate::error::{YggdrasilError, YggdrasilErrorKind};

/// Payload for a profile's `textures` property value, before encoding.
///
/// `textures` keys are the protocol texture slots (`skin`, `cape`, `elytra`);
/// a `BTreeMap` keeps the serialized key order deterministic.
#[derive(Debug, Serialize)]
pub struct TexturesPropertyValue<'a> {
    /// Unix epoch milliseconds embedded in the property for cache freshness.
    pub timestamp: i64,
    /// UUID of the profile owning these textures, in unsigned (dash-less) form.
    #[serde(rename = "profileId")]
    pub profile_id: &'a str,
    /// Player name recorded at signing time.
    #[serde(rename = "profileName")]
    pub profile_name: &'a str,
    /// Texture slots keyed by slot name (`skin`, `cape`, `elytra`).
    pub textures: BTreeMap<&'static str, TexturesPropertyValueItem<'a>>,
}

/// One texture slot entry inside the `textures` object.
#[derive(Debug, Serialize)]
pub struct TexturesPropertyValueItem<'a> {
    /// Texture image URL the client downloads and caches.
    pub url: String,
    /// Slot metadata; present only when the skin model is not the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TexturesPropertyValueMetadata<'a>>,
}

/// Metadata block attached to a texture slot.
#[derive(Debug, Serialize)]
pub struct TexturesPropertyValueMetadata<'a> {
    /// Skin model name, for example `slim`; the default model omits metadata.
    pub model: &'a str,
}

/// Serialize and base64-encode the payload into the property `value` string.
pub fn encode_textures_property_value(
    value: &TexturesPropertyValue<'_>,
) -> Result<String, YggdrasilError> {
    let payload = serde_json::to_vec(value).map_err(|error| {
        YggdrasilError::with_detail(
            YggdrasilErrorKind::Internal,
            format!("failed to serialize textures property: {error}"),
        )
    })?;
    Ok(base64::engine::general_purpose::STANDARD.encode(payload))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        TexturesPropertyValue, TexturesPropertyValueItem, TexturesPropertyValueMetadata,
        encode_textures_property_value,
    };
    use base64::Engine;

    #[test]
    fn encoded_value_decodes_to_the_documented_json_shape() {
        let mut textures = BTreeMap::new();
        textures.insert(
            "cape",
            TexturesPropertyValueItem {
                url: "https://textures.example.test/capes/abc.png".to_string(),
                metadata: None,
            },
        );
        textures.insert(
            "skin",
            TexturesPropertyValueItem {
                url: "https://textures.example.test/skins/def.png".to_string(),
                metadata: Some(TexturesPropertyValueMetadata { model: "slim" }),
            },
        );
        let value = TexturesPropertyValue {
            timestamp: 1_700_000_000_000,
            profile_id: "0dace2b93c2b4aa994668b50979fc425",
            profile_name: "Notch",
            textures,
        };
        let encoded = encode_textures_property_value(&value).unwrap();
        let json: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::STANDARD
                .decode(&encoded)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["timestamp"], 1_700_000_000_000i64);
        assert_eq!(json["profileId"], "0dace2b93c2b4aa994668b50979fc425");
        assert_eq!(json["profileName"], "Notch");
        // Keys serialize in sorted order regardless of insertion order.
        let keys: Vec<&str> = json["textures"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["cape", "skin"]);
        assert_eq!(json["textures"]["skin"]["metadata"]["model"], "slim");
        assert!(json["textures"]["cape"].get("metadata").is_none());
    }
}
