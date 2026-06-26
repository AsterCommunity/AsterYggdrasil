//! Avatar runtime configuration.
//!
//! This module keeps the product-specific config binding for avatar base URLs.
//! The shared normalization and defaulting logic lives in Forge config, while
//! this crate still owns the runtime key, local schema wiring, and tests around
//! product state.

use crate::config::RuntimeConfig;
use crate::errors::Result;
use aster_forge_config::avatar::{
    gravatar_base_url_or_default as forge_gravatar_base_url_or_default,
    normalize_gravatar_base_url_config_value as forge_normalize_gravatar_base_url_config_value,
};

pub use crate::config::definitions::GRAVATAR_BASE_URL_KEY;

pub fn normalize_gravatar_base_url_config_value(value: &str) -> Result<String> {
    forge_normalize_gravatar_base_url_config_value(value).map_err(Into::into)
}

pub fn gravatar_base_url_or_default(runtime_config: &RuntimeConfig) -> String {
    let configured = runtime_config.get(GRAVATAR_BASE_URL_KEY);
    forge_gravatar_base_url_or_default(configured.as_deref())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{gravatar_base_url_or_default, normalize_gravatar_base_url_config_value};
    use crate::config::RuntimeConfig;
    use crate::entities::system_config;
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};
    #[test]
    fn gravatar_base_url_normalization_rejects_query_and_bad_scheme() {
        assert_eq!(
            normalize_gravatar_base_url_config_value(" https://mirror.example/avatar/ ").unwrap(),
            "https://mirror.example/avatar"
        );
        assert!(normalize_gravatar_base_url_config_value("ftp://example.com/avatar").is_err());
        assert!(
            normalize_gravatar_base_url_config_value("https://example.com/avatar?x=1").is_err()
        );
        assert!(
            normalize_gravatar_base_url_config_value("https://example.com/avatar#frag").is_err()
        );
    }

    #[test]
    fn gravatar_base_url_defaults_when_runtime_value_is_blank() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(system_config::Model {
            id: 1,
            key: super::GRAVATAR_BASE_URL_KEY.to_string(),
            value: "   ".to_string(),
            value_type: ConfigValueType::String,
            source: ConfigSource::System,
            visibility: ConfigVisibility::Private,
            requires_restart: false,
            is_sensitive: false,
            category: crate::config::definitions::CONFIG_CATEGORY_USER_AVATAR.to_string(),
            namespace: "system".to_string(),
            description: String::new(),
            updated_at: Utc::now(),
            updated_by: None,
        });

        assert_eq!(
            gravatar_base_url_or_default(&runtime_config),
            "https://www.gravatar.com/avatar"
        );
    }
}
