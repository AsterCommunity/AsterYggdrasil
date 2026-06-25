//! Runtime texture preview rendering configuration.

use aster_texture_renderer::{
    Skin2dPreviewOptions, SkinPreviewOptions, SkinPreviewProfile, TexturePreviewOptions,
};
use image::Rgba;
use serde::Serialize;
use std::str::FromStr;

use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};
use aster_forge_config::{
    normalize_bool_config_value, normalize_bounded_u8_config_value,
    normalize_finite_f32_config_value, normalize_positive_u32_config_value,
    parse_single_string_enum_selection, read_finite_f32, read_positive_u32,
};

pub use crate::config::definitions::{
    TEXTURE_PREVIEW_2D_PADDING_KEY, TEXTURE_PREVIEW_2D_SPACING_KEY,
    TEXTURE_PREVIEW_3D_BACK_YAW_KEY, TEXTURE_PREVIEW_3D_CENTER_Y_KEY,
    TEXTURE_PREVIEW_3D_FRONT_YAW_KEY, TEXTURE_PREVIEW_3D_PITCH_KEY, TEXTURE_PREVIEW_3D_SCALE_KEY,
    TEXTURE_PREVIEW_3D_SPACING_KEY, TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY,
    TEXTURE_PREVIEW_3D_X_OFFSET_KEY, TEXTURE_PREVIEW_3D_Y_OFFSET_KEY,
    TEXTURE_PREVIEW_BACKGROUND_KEY, TEXTURE_PREVIEW_ENGINE_KEY, TEXTURE_PREVIEW_HEIGHT_KEY,
    TEXTURE_PREVIEW_PROFILE_KEY, TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY, TEXTURE_PREVIEW_WIDTH_KEY,
};

pub const DEFAULT_TEXTURE_PREVIEW_ENGINE: TexturePreviewEngine = TexturePreviewEngine::Skin3d;
pub const DEFAULT_TEXTURE_PREVIEW_PROFILE: TexturePreviewQualityProfile =
    TexturePreviewQualityProfile::Default;
pub const DEFAULT_TEXTURE_PREVIEW_WIDTH: u32 = 430;
pub const DEFAULT_TEXTURE_PREVIEW_HEIGHT: u32 = 430;
pub const DEFAULT_TEXTURE_PREVIEW_BACKGROUND: &str = "transparent";
pub const DEFAULT_TEXTURE_PREVIEW_SHOW_OUTER_LAYER: bool = true;
pub const DEFAULT_TEXTURE_PREVIEW_3D_SCALE: f32 = 11.5;
pub const DEFAULT_TEXTURE_PREVIEW_3D_PITCH: f32 = 30.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_FRONT_YAW: f32 = -45.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_BACK_YAW: f32 = 135.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_SPACING: f32 = 35.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_X_OFFSET: f32 = 0.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_Y_OFFSET: f32 = -24.0;
pub const DEFAULT_TEXTURE_PREVIEW_3D_CENTER_Y: f32 = 0.56;
pub const DEFAULT_TEXTURE_PREVIEW_3D_SUPERSAMPLING: u8 = 2;
pub const DEFAULT_TEXTURE_PREVIEW_2D_PADDING: u32 = 24;
pub const DEFAULT_TEXTURE_PREVIEW_2D_SPACING: u32 = 35;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TexturePreviewEngine {
    Skin3d,
    Skin2d,
}

impl TexturePreviewEngine {
    pub const ALL: [Self; 2] = [Self::Skin3d, Self::Skin2d];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skin3d => "skin-3d",
            Self::Skin2d => "skin-2d",
        }
    }

    fn parse_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "skin-3d" | "3d" => Some(Self::Skin3d),
            "skin-2d" | "2d" => Some(Self::Skin2d),
            _ => None,
        }
    }
}

impl FromStr for TexturePreviewEngine {
    type Err = AsterError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse_value(value).ok_or_else(|| {
            AsterError::validation_error("texture preview engine must be one of: skin-3d, skin-2d")
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TexturePreviewQualityProfile {
    Fast,
    Default,
    Quality,
}

impl TexturePreviewQualityProfile {
    pub const ALL: [Self; 3] = [Self::Fast, Self::Default, Self::Quality];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Default => "default",
            Self::Quality => "quality",
        }
    }

    fn parse_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fast" => Some(Self::Fast),
            "default" => Some(Self::Default),
            "quality" => Some(Self::Quality),
            _ => None,
        }
    }
}

impl FromStr for TexturePreviewQualityProfile {
    type Err = AsterError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse_value(value).ok_or_else(|| {
            AsterError::validation_error(
                "texture preview quality profile must be one of: fast, default, quality",
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeTexturePreviewPolicy {
    pub spec: TexturePreviewSpec,
}

impl RuntimeTexturePreviewPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let engine = read_engine(runtime_config);
        let profile = read_profile(runtime_config);
        let width = read_u32(
            runtime_config,
            TEXTURE_PREVIEW_WIDTH_KEY,
            DEFAULT_TEXTURE_PREVIEW_WIDTH,
        );
        let height = read_u32(
            runtime_config,
            TEXTURE_PREVIEW_HEIGHT_KEY,
            DEFAULT_TEXTURE_PREVIEW_HEIGHT,
        );
        let background = read_background(runtime_config);
        let show_outer_layer = runtime_config.get_bool_or(
            TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY,
            DEFAULT_TEXTURE_PREVIEW_SHOW_OUTER_LAYER,
        );

        Self {
            spec: TexturePreviewSpec {
                engine,
                profile,
                width,
                height,
                background,
                show_outer_layer,
                scale: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_SCALE_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_SCALE,
                ),
                pitch: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_PITCH_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_PITCH,
                ),
                front_yaw: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_FRONT_YAW_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_FRONT_YAW,
                ),
                back_yaw: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_BACK_YAW_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_BACK_YAW,
                ),
                spacing_3d: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_SPACING_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_SPACING,
                ),
                x_offset: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_X_OFFSET_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_X_OFFSET,
                ),
                y_offset: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_Y_OFFSET_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_Y_OFFSET,
                ),
                center_y: read_f32(
                    runtime_config,
                    TEXTURE_PREVIEW_3D_CENTER_Y_KEY,
                    DEFAULT_TEXTURE_PREVIEW_3D_CENTER_Y,
                ),
                supersampling: read_supersampling(runtime_config, profile),
                padding_2d: read_u32(
                    runtime_config,
                    TEXTURE_PREVIEW_2D_PADDING_KEY,
                    DEFAULT_TEXTURE_PREVIEW_2D_PADDING,
                ),
                spacing_2d: read_u32(
                    runtime_config,
                    TEXTURE_PREVIEW_2D_SPACING_KEY,
                    DEFAULT_TEXTURE_PREVIEW_2D_SPACING,
                ),
            },
        }
    }
}

/// Canonical preview parameters used for URL fingerprints and renderer options.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TexturePreviewSpec {
    pub engine: TexturePreviewEngine,
    pub profile: TexturePreviewQualityProfile,
    pub width: u32,
    pub height: u32,
    pub background: TexturePreviewBackground,
    pub show_outer_layer: bool,
    pub scale: f32,
    pub pitch: f32,
    pub front_yaw: f32,
    pub back_yaw: f32,
    pub spacing_3d: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub center_y: f32,
    pub supersampling: u8,
    pub padding_2d: u32,
    pub spacing_2d: u32,
}

impl TexturePreviewSpec {
    pub fn renderer_options(&self) -> TexturePreviewOptions {
        match self.engine {
            TexturePreviewEngine::Skin3d => {
                let mut options = SkinPreviewOptions::from_profile(match self.profile {
                    TexturePreviewQualityProfile::Fast => SkinPreviewProfile::Fast,
                    TexturePreviewQualityProfile::Default => SkinPreviewProfile::Default,
                    TexturePreviewQualityProfile::Quality => SkinPreviewProfile::Quality,
                });
                options.output_width = self.width;
                options.output_height = self.height;
                options.scale = self.scale;
                options.pitch_degrees = self.pitch;
                options.front_yaw_degrees = self.front_yaw;
                options.back_yaw_degrees = self.back_yaw;
                options.view_spacing = self.spacing_3d;
                options.horizontal_offset = self.x_offset;
                options.vertical_offset = self.y_offset;
                options.center_y_ratio = self.center_y;
                options.background = self.background.rgba();
                options.show_outer_layer = self.show_outer_layer;
                options.supersampling = self.supersampling;
                TexturePreviewOptions::Skin3d(options)
            }
            TexturePreviewEngine::Skin2d => TexturePreviewOptions::Skin2d(Skin2dPreviewOptions {
                output_width: self.width,
                output_height: self.height,
                padding: self.padding_2d,
                view_spacing: self.spacing_2d,
                background: self.background.rgba(),
                show_outer_layer: self.show_outer_layer,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "rgba", rename_all = "snake_case")]
pub enum TexturePreviewBackground {
    Transparent,
    Solid([u8; 4]),
}

impl TexturePreviewBackground {
    pub fn rgba(self) -> Option<Rgba<u8>> {
        match self {
            Self::Transparent => None,
            Self::Solid(rgba) => Some(Rgba(rgba)),
        }
    }

    pub fn wire_value(self) -> String {
        match self {
            Self::Transparent => "transparent".to_string(),
            Self::Solid([red, green, blue, alpha]) => {
                format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
            }
        }
    }
}

pub fn normalize_texture_preview_config_value(key: &str, value: &str) -> Result<String> {
    match key {
        TEXTURE_PREVIEW_ENGINE_KEY => Ok(parse_engine(value)?.as_str().to_string()),
        TEXTURE_PREVIEW_PROFILE_KEY => Ok(parse_profile(value)?.as_str().to_string()),
        TEXTURE_PREVIEW_BACKGROUND_KEY => Ok(parse_background(value)?.wire_value()),
        TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY => {
            normalize_bool_config_value(TEXTURE_PREVIEW_SHOW_OUTER_LAYER_KEY, value)
                .map_err(Into::into)
        }
        TEXTURE_PREVIEW_WIDTH_KEY
        | TEXTURE_PREVIEW_HEIGHT_KEY
        | TEXTURE_PREVIEW_2D_PADDING_KEY
        | TEXTURE_PREVIEW_2D_SPACING_KEY => normalize_u32(value),
        TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY => normalize_supersampling(value),
        TEXTURE_PREVIEW_3D_SCALE_KEY
        | TEXTURE_PREVIEW_3D_PITCH_KEY
        | TEXTURE_PREVIEW_3D_FRONT_YAW_KEY
        | TEXTURE_PREVIEW_3D_BACK_YAW_KEY
        | TEXTURE_PREVIEW_3D_SPACING_KEY
        | TEXTURE_PREVIEW_3D_X_OFFSET_KEY
        | TEXTURE_PREVIEW_3D_Y_OFFSET_KEY
        | TEXTURE_PREVIEW_3D_CENTER_Y_KEY => normalize_f32(value),
        _ => Ok(value.to_string()),
    }
}

fn read_engine(runtime_config: &RuntimeConfig) -> TexturePreviewEngine {
    runtime_config
        .get(TEXTURE_PREVIEW_ENGINE_KEY)
        .and_then(|value| parse_engine(&value).ok())
        .unwrap_or(DEFAULT_TEXTURE_PREVIEW_ENGINE)
}

fn read_profile(runtime_config: &RuntimeConfig) -> TexturePreviewQualityProfile {
    runtime_config
        .get(TEXTURE_PREVIEW_PROFILE_KEY)
        .and_then(|value| parse_profile(&value).ok())
        .unwrap_or(DEFAULT_TEXTURE_PREVIEW_PROFILE)
}

fn read_background(runtime_config: &RuntimeConfig) -> TexturePreviewBackground {
    runtime_config
        .get(TEXTURE_PREVIEW_BACKGROUND_KEY)
        .and_then(|value| parse_background(&value).ok())
        .unwrap_or(TexturePreviewBackground::Transparent)
}

fn read_u32(runtime_config: &RuntimeConfig, key: &str, default: u32) -> u32 {
    read_positive_u32(runtime_config, key, default)
}

fn read_supersampling(runtime_config: &RuntimeConfig, profile: TexturePreviewQualityProfile) -> u8 {
    let profile_value = match profile {
        TexturePreviewQualityProfile::Fast => 1,
        TexturePreviewQualityProfile::Default => DEFAULT_TEXTURE_PREVIEW_3D_SUPERSAMPLING,
        TexturePreviewQualityProfile::Quality => 3,
    };
    let configured = runtime_config
        .get(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY)
        .and_then(|value| aster_forge_config::parse_bounded_u8(&value, 1, 4));

    match configured {
        Some(value) if value != DEFAULT_TEXTURE_PREVIEW_3D_SUPERSAMPLING => value,
        _ => profile_value,
    }
}

fn read_f32(runtime_config: &RuntimeConfig, key: &str, default: f32) -> f32 {
    read_finite_f32(runtime_config, key, default)
}

fn parse_engine(value: &str) -> Result<TexturePreviewEngine> {
    parse_single_string_enum_selection(
        value,
        TEXTURE_PREVIEW_ENGINE_KEY,
        "skin-3d or skin-2d",
        |value| value.parse::<TexturePreviewEngine>().ok(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))
}

fn parse_profile(value: &str) -> Result<TexturePreviewQualityProfile> {
    parse_single_string_enum_selection(
        value,
        TEXTURE_PREVIEW_PROFILE_KEY,
        "fast, default, or quality",
        |value| value.parse::<TexturePreviewQualityProfile>().ok(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))
}

fn parse_background(value: &str) -> Result<TexturePreviewBackground> {
    match value.trim().to_ascii_lowercase().as_str() {
        "transparent" | "none" => Ok(TexturePreviewBackground::Transparent),
        "white" => Ok(TexturePreviewBackground::Solid([255, 255, 255, 255])),
        "black" => Ok(TexturePreviewBackground::Solid([0, 0, 0, 255])),
        _ => parse_hex_background(value).map(TexturePreviewBackground::Solid),
    }
}

fn parse_hex_background(value: &str) -> Result<[u8; 4]> {
    let hex = value.trim().strip_prefix('#').ok_or_else(|| {
        AsterError::validation_error(
            "texture preview background must be transparent, none, white, black, #RRGGBB, or #RRGGBBAA",
        )
    })?;
    if hex.len() != 6 && hex.len() != 8 {
        return Err(AsterError::validation_error(
            "texture preview background hex must be #RRGGBB or #RRGGBBAA",
        ));
    }
    let red = parse_hex_byte(hex, 0)?;
    let green = parse_hex_byte(hex, 2)?;
    let blue = parse_hex_byte(hex, 4)?;
    let alpha = if hex.len() == 8 {
        parse_hex_byte(hex, 6)?
    } else {
        255
    };
    Ok([red, green, blue, alpha])
}

fn parse_hex_byte(hex: &str, start: usize) -> Result<u8> {
    u8::from_str_radix(&hex[start..start + 2], 16).map_err(|_| {
        AsterError::validation_error("texture preview background contains invalid hex digits")
    })
}

fn normalize_u32(value: &str) -> Result<String> {
    normalize_positive_u32_config_value("texture preview value", value).map_err(Into::into)
}

fn normalize_supersampling(value: &str) -> Result<String> {
    normalize_bounded_u8_config_value(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY, value, 1, 4)
        .map_err(Into::into)
}

fn normalize_f32(value: &str) -> Result<String> {
    normalize_finite_f32_config_value("texture preview value", value).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::system_config;
    use crate::types::{SystemConfigSource, SystemConfigValueType, SystemConfigVisibility};
    use chrono::Utc;

    #[test]
    fn normalizes_background_aliases_and_hex() {
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_BACKGROUND_KEY, "none").unwrap(),
            "transparent"
        );
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_BACKGROUND_KEY, "#AABBCC")
                .unwrap(),
            "#aabbccff"
        );
    }

    #[test]
    fn rejects_invalid_supersampling() {
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY, "0")
                .is_err()
        );
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY, "5")
                .is_err()
        );
    }

    #[test]
    fn normalizes_engine_and_profile_as_string_enums() {
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_ENGINE_KEY, "skin-3d").unwrap(),
            "skin-3d"
        );
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_ENGINE_KEY, r#"["skin-2d"]"#)
                .unwrap(),
            "skin-2d"
        );
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_PROFILE_KEY, "quality").unwrap(),
            "quality"
        );
        assert_eq!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_PROFILE_KEY, r#"["default"]"#)
                .unwrap(),
            "default"
        );
    }

    #[test]
    fn rejects_invalid_engine_and_profile_string_enums() {
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_ENGINE_KEY, r#"[]"#).is_err()
        );
        assert!(
            normalize_texture_preview_config_value(
                TEXTURE_PREVIEW_ENGINE_KEY,
                r#"["skin-3d","skin-2d"]"#
            )
            .is_err()
        );
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_ENGINE_KEY, r#"["unknown"]"#)
                .is_err()
        );
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_PROFILE_KEY, r#"[]"#).is_err()
        );
        assert!(
            normalize_texture_preview_config_value(
                TEXTURE_PREVIEW_PROFILE_KEY,
                r#"["fast","quality"]"#
            )
            .is_err()
        );
        assert!(
            normalize_texture_preview_config_value(TEXTURE_PREVIEW_PROFILE_KEY, r#"["unknown"]"#)
                .is_err()
        );
    }

    #[test]
    fn profile_controls_supersampling_until_value_is_explicitly_changed() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(model(TEXTURE_PREVIEW_PROFILE_KEY, "fast"));
        runtime_config.apply(model(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY, "2"));

        let policy = RuntimeTexturePreviewPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.spec.supersampling, 1);

        runtime_config.apply(model(TEXTURE_PREVIEW_3D_SUPERSAMPLING_KEY, "4"));
        let policy = RuntimeTexturePreviewPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.spec.supersampling, 4);
    }

    #[test]
    fn runtime_policy_reads_string_enum_engine_and_profile_values() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(model(TEXTURE_PREVIEW_ENGINE_KEY, "skin-2d"));
        runtime_config.apply(model(TEXTURE_PREVIEW_PROFILE_KEY, "quality"));

        let policy = RuntimeTexturePreviewPolicy::from_runtime_config(&runtime_config);

        assert_eq!(policy.spec.engine, TexturePreviewEngine::Skin2d);
        assert_eq!(policy.spec.profile, TexturePreviewQualityProfile::Quality);
    }

    fn model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: SystemConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: SystemConfigSource::System,
            visibility: SystemConfigVisibility::Private,
            namespace: String::new(),
            category: crate::config::definitions::CONFIG_CATEGORY_TEXTURE_PREVIEW.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }
}
