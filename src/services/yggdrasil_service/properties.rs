use super::YggdrasilError;
use crate::api::dto::yggdrasil::{YggdrasilProfile, YggdrasilProfileProperty};
use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::minecraft_profile_texture_repo;
use crate::entities::minecraft_profile;
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::{texture_service, yggdrasil_signature};
use chrono::Utc;
use serde::Serialize;
use std::collections::BTreeMap;

pub(super) async fn profile_with_properties<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    signed: bool,
) -> std::result::Result<YggdrasilProfile, YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let mut properties = Vec::new();
    let textures = minecraft_profile_texture_repo::list_by_profile(state.reader_db(), profile.id)
        .await
        .map_err(YggdrasilError::from)?;
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        texture_count = textures.len(),
        signed,
        uploadable_textures_present = !profile.uploadable_textures.trim().is_empty(),
        "building yggdrasil profile properties"
    );
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let value =
        texture_property_value(&policy, profile, &textures).map_err(YggdrasilError::from)?;
    let signature = if signed {
        yggdrasil_signature::sign_texture_property(&policy, &value).map_err(YggdrasilError::from)?
    } else {
        None
    };
    properties.push(YggdrasilProfileProperty {
        name: "textures".to_string(),
        value,
        signature,
    });
    tracing::debug!(
        profile_id = profile.id,
        signed,
        "added yggdrasil textures property"
    );
    if !profile.uploadable_textures.trim().is_empty() {
        let signature = if signed {
            yggdrasil_signature::sign_texture_property(&policy, &profile.uploadable_textures)
                .map_err(YggdrasilError::from)?
        } else {
            None
        };
        properties.push(YggdrasilProfileProperty {
            name: "uploadableTextures".to_string(),
            value: profile.uploadable_textures.clone(),
            signature,
        });
        tracing::debug!(
            profile_id = profile.id,
            signed,
            uploadable_textures = %profile.uploadable_textures,
            "added yggdrasil uploadableTextures property"
        );
    }

    Ok(YggdrasilProfile {
        id: profile.uuid.clone(),
        name: profile.name.clone(),
        properties: Some(properties),
    })
}

#[derive(Debug, Serialize)]
struct TexturesProperty<'a> {
    timestamp: i64,
    #[serde(rename = "profileId")]
    profile_id: &'a str,
    #[serde(rename = "profileName")]
    profile_name: &'a str,
    textures: BTreeMap<&'static str, TexturePropertyItem<'a>>,
}

#[derive(Debug, Serialize)]
struct TexturePropertyItem<'a> {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<TexturePropertyMetadata<'a>>,
}

#[derive(Debug, Serialize)]
struct TexturePropertyMetadata<'a> {
    model: &'a str,
}

fn texture_property_value(
    policy: &RuntimeYggdrasilPolicy,
    profile: &minecraft_profile::Model,
    textures: &[minecraft_profile_texture_repo::ProfileTexture],
) -> Result<String> {
    let mut entries = BTreeMap::new();
    for texture in textures {
        let metadata = (texture.binding.texture_type == crate::types::MinecraftTextureType::Skin)
            .then_some(texture.texture.texture_model)
            .and_then(crate::types::MinecraftTextureModel::as_metadata_value)
            .map(|model| TexturePropertyMetadata { model });
        entries.insert(
            texture.binding.texture_type.textures_property_key(),
            TexturePropertyItem {
                url: yggdrasil_signature::required_texture_public_url(
                    policy,
                    &texture.texture.hash,
                )?,
                metadata,
            },
        );
    }
    if !entries.contains_key(crate::types::MinecraftTextureType::Skin.textures_property_key()) {
        let skin = texture_service::default_skin_for_profile_uuid(&profile.uuid);
        let metadata = skin
            .model
            .as_metadata_value()
            .map(|model| TexturePropertyMetadata { model });
        entries.insert(
            crate::types::MinecraftTextureType::Skin.textures_property_key(),
            TexturePropertyItem {
                url: yggdrasil_signature::required_texture_public_url(policy, skin.hash)?,
                metadata,
            },
        );
        tracing::debug!(
            profile_id = profile.id,
            profile_uuid = %profile.uuid,
            default_skin_hash = skin.hash,
            default_skin_model = ?skin.model,
            "added default yggdrasil skin texture property"
        );
    }
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        texture_count = entries.len(),
        "serializing yggdrasil textures property"
    );

    let payload = TexturesProperty {
        timestamp: Utc::now().timestamp_millis(),
        profile_id: &profile.uuid,
        profile_name: &profile.name,
        textures: entries,
    };
    use base64::Engine;
    let encoded = serde_json::to_vec(&payload)
        .map(|payload| base64::engine::general_purpose::STANDARD.encode(payload))
        .map_err(|error| {
            AsterError::internal_error(format!("failed to serialize textures property: {error}"))
        })?;
    Ok(encoded)
}
