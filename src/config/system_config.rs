//! Runtime system configuration helpers.

use crate::config::RuntimeConfig;
use crate::config::definitions::{CONFIG_REGISTRY, ConfigDef};
use crate::entities::system_config;
use crate::errors::Result;
use aster_forge_config::{ConfigSource, ConfigValueType};
use aster_forge_config::{ConfigValueLookup, StoredConfig};

impl ConfigValueLookup for RuntimeConfig {
    fn get_config_value(&self, key: &str) -> Option<String> {
        self.get(key)
    }
}

pub fn get_definition(key: &str) -> Option<&'static ConfigDef> {
    CONFIG_REGISTRY.get(key)
}

pub fn validate_value_type(value_type: ConfigValueType, value: &str) -> Result<()> {
    aster_forge_config::validate_storage_value(value_type, value).map_err(Into::into)
}

pub fn normalize_system_value<L>(lookup: &L, key: &str, value: &str) -> Result<String>
where
    L: ConfigValueLookup,
{
    CONFIG_REGISTRY
        .normalize_value(lookup, key, value)
        .map_err(Into::into)
}

pub fn apply_definition(mut config: system_config::Model) -> system_config::Model {
    if config.source != ConfigSource::System {
        return config;
    }

    let stored = CONFIG_REGISTRY.apply_definition(model_to_stored_config(&config));
    config.value_type = stored.value_type;
    config.requires_restart = stored.requires_restart;
    config.is_sensitive = stored.is_sensitive;
    config.visibility = stored.visibility;
    config.category = stored.category;
    config.description = stored.description;
    config
}

fn model_to_stored_config(config: &system_config::Model) -> StoredConfig {
    StoredConfig {
        id: config.id,
        key: config.key.clone(),
        value: config.value.clone(),
        value_type: config.value_type,
        requires_restart: config.requires_restart,
        is_sensitive: config.is_sensitive,
        source: config.source,
        visibility: config.visibility,
        category: config.category.clone(),
        description: config.description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::{apply_definition, normalize_system_value, validate_value_type};
    use crate::config::definitions::{CONFIG_CATEGORY_SITE_PUBLIC, PUBLIC_SITE_URL_KEY};
    use crate::config::yggdrasil::{YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, YGGDRASIL_TOKEN_TTL_DAYS_KEY};
    use crate::config::{audit, cors, operations};
    use crate::entities::system_config;
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};
    fn model(key: &str, value: &str, source: ConfigSource) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: ConfigValueType::String,
            requires_restart: false,
            is_sensitive: false,
            source,
            visibility: ConfigVisibility::Private,
            namespace: String::new(),
            category: String::new(),
            description: String::new(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn validate_value_type_enforces_declared_types() {
        assert!(validate_value_type(ConfigValueType::Boolean, "true").is_ok());
        assert!(validate_value_type(ConfigValueType::Boolean, "false").is_ok());
        assert!(validate_value_type(ConfigValueType::Boolean, " yes ").is_err());

        assert!(validate_value_type(ConfigValueType::Number, "42").is_ok());
        assert!(validate_value_type(ConfigValueType::Number, "1.5").is_ok());
        assert!(validate_value_type(ConfigValueType::Number, "nope").is_err());

        assert!(validate_value_type(ConfigValueType::StringArray, r#"["a"]"#).is_ok());
        assert!(validate_value_type(ConfigValueType::StringArray, r#""a""#).is_err());
        assert!(validate_value_type(ConfigValueType::StringEnumSet, r#"["a"]"#).is_ok());
        assert!(validate_value_type(ConfigValueType::StringEnumSet, r#""a""#).is_err());
        assert!(validate_value_type(ConfigValueType::StringEnum, "a").is_ok());
        assert!(validate_value_type(ConfigValueType::String, "anything").is_ok());
        assert!(validate_value_type(ConfigValueType::Multiline, "line\nline").is_ok());
    }

    #[test]
    fn normalize_system_value_validates_audit_action_scope() {
        let lookup = HashMap::new();

        assert_eq!(
            normalize_system_value(
                &lookup,
                audit::AUDIT_LOG_RECORDED_ACTIONS_KEY,
                r#"["user_login","config_update"]"#,
            )
            .unwrap(),
            r#"["config_update","user_login"]"#
        );
        assert!(
            normalize_system_value(
                &lookup,
                audit::AUDIT_LOG_RECORDED_ACTIONS_KEY,
                r#"["unknown_action"]"#,
            )
            .is_err()
        );
        assert!(
            normalize_system_value(
                &lookup,
                audit::AUDIT_LOG_RECORDED_ACTIONS_KEY,
                r#"["user_login","user_login"]"#,
            )
            .is_err()
        );
        assert_eq!(
            normalize_system_value(&lookup, audit::AUDIT_LOG_RECORDED_ACTIONS_KEY, "[]").unwrap(),
            "[]"
        );
    }

    #[test]
    fn normalize_system_value_uses_lookup_for_cors_cross_field_validation() {
        let lookup = HashMap::from([(
            cors::CORS_ALLOW_CREDENTIALS_KEY.to_string(),
            "true".to_string(),
        )]);

        let err = normalize_system_value(&lookup, cors::CORS_ALLOWED_ORIGINS_KEY, "*").unwrap_err();
        assert!(
            err.message()
                .contains("cors_allow_credentials cannot be true when cors_allowed_origins is '*'")
        );

        let lookup = HashMap::from([(cors::CORS_ALLOWED_ORIGINS_KEY.to_string(), "*".to_string())]);
        let err =
            normalize_system_value(&lookup, cors::CORS_ALLOW_CREDENTIALS_KEY, "true").unwrap_err();
        assert!(
            err.message()
                .contains("cors_allow_credentials cannot be true when cors_allowed_origins is '*'")
        );
    }

    #[test]
    fn normalize_system_value_rejects_non_positive_yggdrasil_token_limits() {
        let lookup = HashMap::new();

        assert_eq!(
            normalize_system_value(&lookup, YGGDRASIL_TOKEN_TTL_DAYS_KEY, "15").unwrap(),
            "15"
        );
        assert_eq!(
            normalize_system_value(&lookup, YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, "2").unwrap(),
            "2"
        );
        assert!(normalize_system_value(&lookup, YGGDRASIL_TOKEN_TTL_DAYS_KEY, "0").is_err());
        assert!(normalize_system_value(&lookup, YGGDRASIL_MAX_ACTIVE_TOKENS_KEY, "1.5").is_err());
    }

    #[test]
    fn normalize_system_value_routes_generic_operation_and_site_keys() {
        let lookup = HashMap::new();

        assert_eq!(
            normalize_system_value(
                &lookup,
                operations::BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
                " 8 ",
            )
            .unwrap(),
            "8"
        );
        assert!(
            normalize_system_value(
                &lookup,
                operations::BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
                "0",
            )
            .is_err()
        );
        assert_eq!(
            normalize_system_value(
                &lookup,
                PUBLIC_SITE_URL_KEY,
                r#"["https://example.com/"," https://admin.example.com "]"#,
            )
            .unwrap(),
            r#"["https://example.com","https://admin.example.com"]"#
        );
    }

    #[test]
    fn apply_definition_overlays_schema_metadata_for_system_rows() {
        let config = apply_definition(model(
            PUBLIC_SITE_URL_KEY,
            r#"["https://forge.example.com"]"#,
            ConfigSource::System,
        ));
        assert_eq!(config.value_type, ConfigValueType::StringArray);
        assert_eq!(config.category, CONFIG_CATEGORY_SITE_PUBLIC);
        assert_eq!(
            config.description,
            "Public origins used to build externally visible application URLs"
        );

        let custom = apply_definition(model("custom.demo", "value", ConfigSource::Custom));
        assert_eq!(custom.category, "");
        assert_eq!(custom.description, "");
    }
}
