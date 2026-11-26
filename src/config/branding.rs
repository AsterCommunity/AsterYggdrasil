//! Branding runtime configuration helpers.

use crate::config::RuntimeConfig;
use crate::errors::Result;
use aster_forge_validation::display::{
    display_text_or_default, normalize_bounded_display_text, normalize_public_asset_url,
    public_asset_url_or_default,
};

pub use crate::config::definitions::{
    BRANDING_DESCRIPTION_KEY, BRANDING_FAVICON_URL_KEY, BRANDING_TITLE_KEY,
    BRANDING_WORDMARK_DARK_URL_KEY, BRANDING_WORDMARK_LIGHT_URL_KEY,
};

/// Site title shown when `branding_title` is unset or blank.
pub const DEFAULT_BRANDING_TITLE: &str = "AsterYggdrasil";
/// Site description shown when `branding_description` is unset, blank, or still
/// holds a legacy template placeholder value.
pub const DEFAULT_BRANDING_DESCRIPTION: &str =
    "Self-hosted Minecraft skin site and Yggdrasil authentication server.";
/// Favicon URL shown when `branding_favicon_url` is unset, blank, or invalid;
/// points at the asset bundled with the embedded frontend.
pub const DEFAULT_BRANDING_FAVICON_URL: &str = "/favicon.svg";
/// Dark-theme wordmark URL shown when `branding_wordmark_dark_url` is unset,
/// blank, or invalid; empty means the UI falls back to text-only branding.
pub const DEFAULT_BRANDING_WORDMARK_DARK_URL: &str = "";
/// Light-theme wordmark URL shown when `branding_wordmark_light_url` is unset,
/// blank, or invalid; empty means the UI falls back to text-only branding.
pub const DEFAULT_BRANDING_WORDMARK_LIGHT_URL: &str = "";

const MAX_BRANDING_TITLE_LEN: usize = 120;
const MAX_BRANDING_DESCRIPTION_LEN: usize = 300;
const MAX_BRANDING_ASSET_URL_LEN: usize = 2048;

/// Validates and trims a stored `branding_title` value, rejecting text beyond
/// 120 characters; empty input is allowed and means "use the default".
pub fn normalize_title_config_value(value: &str) -> Result<String> {
    normalize_bounded_display_text("branding_title", value, MAX_BRANDING_TITLE_LEN)
        .map_err(Into::into)
}

/// Validates and trims a stored `branding_description` value, rejecting text
/// beyond 300 characters; empty input is allowed and means "use the default".
pub fn normalize_description_config_value(value: &str) -> Result<String> {
    normalize_bounded_display_text("branding_description", value, MAX_BRANDING_DESCRIPTION_LEN)
        .map_err(Into::into)
}

/// Validates a stored `branding_favicon_url` value: absolute http(s) URLs or
/// root-relative paths, up to 2048 characters, no whitespace or script schemes.
pub fn normalize_favicon_url_config_value(value: &str) -> Result<String> {
    normalize_public_asset_url(BRANDING_FAVICON_URL_KEY, value, MAX_BRANDING_ASSET_URL_LEN)
        .map_err(Into::into)
}

/// Validates a stored `branding_wordmark_dark_url` value with the same rules
/// as [`normalize_favicon_url_config_value`].
pub fn normalize_wordmark_dark_url_config_value(value: &str) -> Result<String> {
    normalize_public_asset_url(
        BRANDING_WORDMARK_DARK_URL_KEY,
        value,
        MAX_BRANDING_ASSET_URL_LEN,
    )
    .map_err(Into::into)
}

/// Validates a stored `branding_wordmark_light_url` value with the same rules
/// as [`normalize_favicon_url_config_value`].
pub fn normalize_wordmark_light_url_config_value(value: &str) -> Result<String> {
    normalize_public_asset_url(
        BRANDING_WORDMARK_LIGHT_URL_KEY,
        value,
        MAX_BRANDING_ASSET_URL_LEN,
    )
    .map_err(Into::into)
}

/// Returns the configured site title, or [`DEFAULT_BRANDING_TITLE`] when the
/// runtime value is missing, blank, or invalid.
pub fn title_or_default(runtime_config: &RuntimeConfig) -> String {
    string_or_default(
        runtime_config.get(BRANDING_TITLE_KEY),
        DEFAULT_BRANDING_TITLE,
        "branding_title",
        MAX_BRANDING_TITLE_LEN,
    )
}

/// Returns the configured site description, or [`DEFAULT_BRANDING_DESCRIPTION`]
/// when the runtime value is missing, blank, invalid, or a legacy template
/// placeholder left over from before this project had its own identity.
pub fn description_or_default(runtime_config: &RuntimeConfig) -> String {
    let description = string_or_default(
        runtime_config.get(BRANDING_DESCRIPTION_KEY),
        DEFAULT_BRANDING_DESCRIPTION,
        "branding_description",
        MAX_BRANDING_DESCRIPTION_LEN,
    );
    if is_legacy_template_description(&description) {
        DEFAULT_BRANDING_DESCRIPTION.to_string()
    } else {
        description
    }
}

/// Returns the configured favicon URL, or [`DEFAULT_BRANDING_FAVICON_URL`]
/// when the runtime value is missing, blank, or invalid.
pub fn favicon_url_or_default(runtime_config: &RuntimeConfig) -> String {
    asset_url_or_default(
        runtime_config,
        BRANDING_FAVICON_URL_KEY,
        DEFAULT_BRANDING_FAVICON_URL,
    )
}

/// Returns the configured dark-theme wordmark URL, or
/// [`DEFAULT_BRANDING_WORDMARK_DARK_URL`] (empty) when unset, blank, or invalid.
pub fn wordmark_dark_url_or_default(runtime_config: &RuntimeConfig) -> String {
    asset_url_or_default(
        runtime_config,
        BRANDING_WORDMARK_DARK_URL_KEY,
        DEFAULT_BRANDING_WORDMARK_DARK_URL,
    )
}

/// Returns the configured light-theme wordmark URL, or
/// [`DEFAULT_BRANDING_WORDMARK_LIGHT_URL`] (empty) when unset, blank, or invalid.
pub fn wordmark_light_url_or_default(runtime_config: &RuntimeConfig) -> String {
    asset_url_or_default(
        runtime_config,
        BRANDING_WORDMARK_LIGHT_URL_KEY,
        DEFAULT_BRANDING_WORDMARK_LIGHT_URL,
    )
}

fn asset_url_or_default(runtime_config: &RuntimeConfig, key: &str, default: &str) -> String {
    public_asset_url_or_default(runtime_config.get(key), default)
}

fn string_or_default(
    value: Option<String>,
    default: &str,
    field_name: &str,
    max_len: usize,
) -> String {
    display_text_or_default(value, default, field_name, max_len)
}

fn is_legacy_template_description(value: &str) -> bool {
    matches!(
        value.trim(),
        "Reusable Aster service foundation"
            | "Reusable Rust and React service foundation"
            | "Reusable Rust + React service foundation for Aster projects"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BRANDING_DESCRIPTION_KEY, BRANDING_FAVICON_URL_KEY, BRANDING_TITLE_KEY,
        BRANDING_WORDMARK_DARK_URL_KEY, BRANDING_WORDMARK_LIGHT_URL_KEY,
        DEFAULT_BRANDING_DESCRIPTION, DEFAULT_BRANDING_FAVICON_URL, DEFAULT_BRANDING_TITLE,
        DEFAULT_BRANDING_WORDMARK_DARK_URL, DEFAULT_BRANDING_WORDMARK_LIGHT_URL,
        description_or_default, favicon_url_or_default, normalize_favicon_url_config_value,
        normalize_title_config_value, normalize_wordmark_dark_url_config_value,
        normalize_wordmark_light_url_config_value, title_or_default, wordmark_dark_url_or_default,
        wordmark_light_url_or_default,
    };
    use crate::config::RuntimeConfig;
    use aster_forge_db::system_config;
    use chrono::Utc;

    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: aster_forge_config::ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source: aster_forge_config::ConfigSource::System,
            visibility: aster_forge_config::ConfigVisibility::Private,
            namespace: String::new(),
            category: crate::config::definitions::CONFIG_CATEGORY_SITE_BRANDING.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn title_and_description_trim_and_allow_empty_for_default_reset() {
        assert_eq!(
            normalize_title_config_value("  My Foundation  ").unwrap(),
            "My Foundation"
        );
        assert_eq!(
            super::normalize_description_config_value("  Shared base  ").unwrap(),
            "Shared base"
        );
        assert_eq!(normalize_title_config_value("   ").unwrap(), "");
    }

    #[test]
    fn branding_asset_urls_reject_whitespace_and_trim() {
        assert_eq!(
            normalize_favicon_url_config_value("  /assets/icon.svg?v=1  ").unwrap(),
            "/assets/icon.svg?v=1"
        );
        assert_eq!(
            normalize_wordmark_dark_url_config_value("  /assets/wordmark-dark.svg  ").unwrap(),
            "/assets/wordmark-dark.svg"
        );
        assert_eq!(
            normalize_wordmark_light_url_config_value("  /assets/wordmark-light.svg  ").unwrap(),
            "/assets/wordmark-light.svg"
        );
        assert!(normalize_favicon_url_config_value("https://cdn.example.com/icon 1.svg").is_err());
        assert!(normalize_favicon_url_config_value("javascript:alert(1)").is_err());
        assert!(normalize_favicon_url_config_value("icons/favicon.svg").is_err());
        assert!(normalize_wordmark_dark_url_config_value("assets/wordmark-dark.svg").is_err());
        assert!(normalize_wordmark_light_url_config_value("javascript:alert(1)").is_err());
    }

    #[test]
    fn effective_branding_values_fall_back_when_missing_or_blank() {
        let runtime_config = RuntimeConfig::new();
        assert_eq!(title_or_default(&runtime_config), DEFAULT_BRANDING_TITLE);
        assert_eq!(
            description_or_default(&runtime_config),
            DEFAULT_BRANDING_DESCRIPTION
        );
        assert_eq!(
            favicon_url_or_default(&runtime_config),
            DEFAULT_BRANDING_FAVICON_URL
        );
        assert_eq!(
            wordmark_dark_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_DARK_URL
        );
        assert_eq!(
            wordmark_light_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_LIGHT_URL
        );

        runtime_config.apply(config_model(BRANDING_TITLE_KEY, "  "));
        runtime_config.apply(config_model(BRANDING_DESCRIPTION_KEY, "  "));
        runtime_config.apply(config_model(BRANDING_FAVICON_URL_KEY, " "));
        runtime_config.apply(config_model(BRANDING_WORDMARK_DARK_URL_KEY, " "));
        runtime_config.apply(config_model(BRANDING_WORDMARK_LIGHT_URL_KEY, " "));

        assert_eq!(title_or_default(&runtime_config), DEFAULT_BRANDING_TITLE);
        assert_eq!(
            description_or_default(&runtime_config),
            DEFAULT_BRANDING_DESCRIPTION
        );

        runtime_config.apply(config_model(
            BRANDING_DESCRIPTION_KEY,
            "Reusable Aster service foundation",
        ));
        assert_eq!(
            description_or_default(&runtime_config),
            DEFAULT_BRANDING_DESCRIPTION
        );
        assert_eq!(
            favicon_url_or_default(&runtime_config),
            DEFAULT_BRANDING_FAVICON_URL
        );
        assert_eq!(
            wordmark_dark_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_DARK_URL
        );
        assert_eq!(
            wordmark_light_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_LIGHT_URL
        );

        runtime_config.apply(config_model(
            BRANDING_FAVICON_URL_KEY,
            "javascript:alert(1)",
        ));
        runtime_config.apply(config_model(
            BRANDING_WORDMARK_DARK_URL_KEY,
            "javascript:alert(1)",
        ));
        runtime_config.apply(config_model(
            BRANDING_WORDMARK_LIGHT_URL_KEY,
            "wordmark-light.svg",
        ));
        assert_eq!(
            favicon_url_or_default(&runtime_config),
            DEFAULT_BRANDING_FAVICON_URL
        );
        assert_eq!(
            wordmark_dark_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_DARK_URL
        );
        assert_eq!(
            wordmark_light_url_or_default(&runtime_config),
            DEFAULT_BRANDING_WORDMARK_LIGHT_URL
        );
    }

    #[test]
    fn effective_branding_title_strips_control_chars_from_runtime_value() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            BRANDING_TITLE_KEY,
            "  My\r\n\tFoundation\u{7}  ",
        ));

        assert_eq!(title_or_default(&runtime_config), "MyFoundation");
    }
}
