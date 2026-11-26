use crate::config::auth_runtime::CaptchaRenderPreset;
use crate::config::definitions::{
    ALL_CONFIGS, AUDIT_LOG_RECORDED_ACTIONS_KEY, AUTH_CAPTCHA_PRESET_KEY,
    TEXTURE_PREVIEW_ENGINE_KEY, TEXTURE_PREVIEW_PROFILE_KEY,
};
use crate::config::texture_preview::{TexturePreviewEngine, TexturePreviewQualityProfile};
use aster_forge_config::ConfigValueType;
use aster_yggdrasil_model::types::audit::AuditAction;
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// Schema descriptor for one runtime config key, letting the admin UI render
/// forms without hardcoding the registry.
#[derive(Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ConfigSchemaItem {
    /// Stable config key this descriptor describes.
    pub key: String,
    /// I18n key for the field label.
    pub label_i18n_key: String,
    /// I18n key for the field description.
    pub description_i18n_key: String,
    /// Value type that governs parsing and validation.
    pub value_type: ConfigValueType,
    /// Admin UI grouping category.
    pub category: String,
    /// Human-readable English description.
    pub description: String,
    /// Whether changes only apply after a process restart.
    pub requires_restart: bool,
    /// Whether the value must be redacted in listings and audit records.
    pub is_sensitive: bool,
    /// Enumerated value choices for enum-typed keys; empty otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ConfigSchemaOption>,
}

/// One selectable choice for an enum-typed config key.
#[derive(Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ConfigSchemaOption {
    /// Wire value to store when this option is chosen.
    pub value: String,
    /// I18n key for the option label.
    pub label_i18n_key: String,
    /// Sub-group the option renders under in the admin UI.
    pub group: String,
}

/// Renders the full config registry into schema descriptors for the admin
/// config UI, attaching backend-authored enum options where they exist.
pub fn get_schema() -> Vec<ConfigSchemaItem> {
    ALL_CONFIGS
        .iter()
        .map(|def| ConfigSchemaItem {
            key: def.key.to_string(),
            label_i18n_key: def.label_i18n_key.to_string(),
            description_i18n_key: def.description_i18n_key.to_string(),
            value_type: def.value_type,
            category: def.category.to_string(),
            description: def.description.to_string(),
            requires_restart: def.requires_restart,
            is_sensitive: def.is_sensitive,
            options: config_schema_options(def.key),
        })
        .collect()
}

fn config_schema_options(key: &str) -> Vec<ConfigSchemaOption> {
    match key {
        // Keep enum-set options backend-authored so the UI cannot drift from AuditAction.
        AUDIT_LOG_RECORDED_ACTIONS_KEY => AuditAction::ALL
            .iter()
            .map(|action| ConfigSchemaOption {
                value: action.as_str().to_string(),
                label_i18n_key: format!("audit_action_{}", action.as_str()),
                group: action.group().to_string(),
            })
            .collect(),
        AUTH_CAPTCHA_PRESET_KEY => CaptchaRenderPreset::ALL
            .iter()
            .map(|preset| ConfigSchemaOption {
                value: preset.as_str().to_string(),
                label_i18n_key: format!("settings_auth_captcha_preset_{}", preset.as_str()),
                group: "captcha".to_string(),
            })
            .collect(),
        TEXTURE_PREVIEW_ENGINE_KEY => TexturePreviewEngine::ALL
            .iter()
            .map(|engine| ConfigSchemaOption {
                value: engine.as_str().to_string(),
                label_i18n_key: format!(
                    "settings_texture_preview_engine_{}",
                    engine.as_str().replace('-', "_")
                ),
                group: "texture_preview".to_string(),
            })
            .collect(),
        TEXTURE_PREVIEW_PROFILE_KEY => TexturePreviewQualityProfile::ALL
            .iter()
            .map(|profile| ConfigSchemaOption {
                value: profile.as_str().to_string(),
                label_i18n_key: format!("settings_texture_preview_profile_{}", profile.as_str()),
                group: "texture_preview".to_string(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_recorded_actions_schema_options_cover_all_actions() {
        let item = get_schema()
            .into_iter()
            .find(|item| item.key == AUDIT_LOG_RECORDED_ACTIONS_KEY)
            .expect("audit action scope config should be in schema");

        assert_eq!(item.value_type, ConfigValueType::StringEnumSet);
        assert_eq!(item.options.len(), AuditAction::COUNT);

        for (option, action) in item.options.iter().zip(AuditAction::ALL) {
            assert_eq!(option.value, action.as_str());
            assert_eq!(
                option.label_i18n_key,
                format!("audit_action_{}", action.as_str())
            );
            assert_eq!(option.group, action.group());
        }
    }

    #[test]
    fn captcha_preset_schema_options_cover_supported_presets() {
        let item = get_schema()
            .into_iter()
            .find(|item| item.key == AUTH_CAPTCHA_PRESET_KEY)
            .expect("captcha preset config should be in schema");

        assert_eq!(item.value_type, ConfigValueType::StringEnum);
        assert_eq!(item.options.len(), CaptchaRenderPreset::ALL.len());

        for (option, preset) in item.options.iter().zip(CaptchaRenderPreset::ALL) {
            assert_eq!(option.value, preset.as_str());
            assert_eq!(
                option.label_i18n_key,
                format!("settings_auth_captcha_preset_{}", preset.as_str())
            );
            assert_eq!(option.group, "captcha");
        }
    }

    #[test]
    fn texture_preview_engine_schema_options_cover_supported_engines() {
        let item = get_schema()
            .into_iter()
            .find(|item| item.key == TEXTURE_PREVIEW_ENGINE_KEY)
            .expect("texture preview engine config should be in schema");

        assert_eq!(item.value_type, ConfigValueType::StringEnum);
        assert_eq!(item.options.len(), TexturePreviewEngine::ALL.len());

        for (option, engine) in item.options.iter().zip(TexturePreviewEngine::ALL) {
            assert_eq!(option.value, engine.as_str());
            assert_eq!(
                option.label_i18n_key,
                format!(
                    "settings_texture_preview_engine_{}",
                    engine.as_str().replace('-', "_")
                )
            );
            assert_eq!(option.group, "texture_preview");
        }
    }

    #[test]
    fn texture_preview_profile_schema_options_cover_supported_profiles() {
        let item = get_schema()
            .into_iter()
            .find(|item| item.key == TEXTURE_PREVIEW_PROFILE_KEY)
            .expect("texture preview profile config should be in schema");

        assert_eq!(item.value_type, ConfigValueType::StringEnum);
        assert_eq!(item.options.len(), TexturePreviewQualityProfile::ALL.len());

        for (option, profile) in item.options.iter().zip(TexturePreviewQualityProfile::ALL) {
            assert_eq!(option.value, profile.as_str());
            assert_eq!(
                option.label_i18n_key,
                format!("settings_texture_preview_profile_{}", profile.as_str())
            );
            assert_eq!(option.group, "texture_preview");
        }
    }
}
