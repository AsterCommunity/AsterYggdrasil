use crate::api::error_code::AsterErrorCode;
use crate::config::RuntimeConfig;
use crate::errors::{AsterError, Result};
use aster_forge_config::parse_string_array_config_value;
use aster_forge_validation::email_policy::{
    EmailPolicyList, normalize_email_policy_items, normalized_email_and_domain,
};

pub use crate::config::definitions::{
    AUTH_LOCAL_EMAIL_ALLOWLIST_KEY, AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEmailPolicy {
    allowlist: EmailPolicyList,
    blocklist: EmailPolicyList,
}

impl LocalEmailPolicy {
    pub fn from_runtime_config(runtime_config: &RuntimeConfig) -> Self {
        let allowlist = read_policy_list(runtime_config, AUTH_LOCAL_EMAIL_ALLOWLIST_KEY);
        let blocklist = read_policy_list(runtime_config, AUTH_LOCAL_EMAIL_BLOCKLIST_KEY);
        Self {
            allowlist,
            blocklist,
        }
    }

    pub fn check(&self, email: &str) -> Result<()> {
        let (normalized, domain) = normalized_email_and_domain(email)?;

        if self.blocklist.matches(&normalized, &domain) {
            return Err(email_blocked_error());
        }

        if !self.allowlist.is_empty() && !self.allowlist.matches(&normalized, &domain) {
            return Err(AsterError::validation_error_code(
                AsterErrorCode::AuthEmailNotAllowlisted,
                "email address is not allowed by local account policy",
            ));
        }

        Ok(())
    }

    pub fn check_not_blocked(&self, email: &str) -> Result<()> {
        let (normalized, domain) = normalized_email_and_domain(email)?;

        if self.blocklist.matches(&normalized, &domain) {
            return Err(email_blocked_error());
        }

        Ok(())
    }
}

fn email_blocked_error() -> AsterError {
    AsterError::validation_error_code(
        AsterErrorCode::AuthEmailBlocked,
        "email address is blocked by local account policy",
    )
}

pub fn normalize_local_email_policy_config_value(key: &str, value: &str) -> Result<String> {
    let raw_items = parse_string_array_config_value(value, key)?;
    let normalized = normalize_email_policy_items(raw_items)?;
    serde_json::to_string(&normalized).map_err(|error| {
        AsterError::internal_error(format!("failed to serialize {key} config value: {error}"))
    })
}

fn read_policy_list(runtime_config: &RuntimeConfig, key: &str) -> EmailPolicyList {
    let Some(raw) = runtime_config.get(key) else {
        return EmailPolicyList::default();
    };
    match parse_string_array_config_value(&raw, key) {
        Ok(items) => EmailPolicyList::from_items_lossy(items, |item, error| {
            tracing::warn!(
                key,
                item,
                error = %error,
                "invalid local email policy item; ignoring"
            );
        }),
        Err(error) => {
            tracing::warn!(
                key,
                value = %raw,
                error = %error,
                "invalid local email policy config JSON; using an empty list"
            );
            EmailPolicyList::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::config::RuntimeConfig;
    use crate::entities::system_config;
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};
    fn config_model(key: &str, value: &str) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: ConfigValueType::StringArray,
            requires_restart: false,
            is_sensitive: false,
            source: ConfigSource::System,
            visibility: ConfigVisibility::Private,
            namespace: String::new(),
            category: crate::config::definitions::CONFIG_CATEGORY_AUTH_EMAIL_POLICY.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[test]
    fn normalizes_policy_items_with_trimming_lowercase_dedupe_and_sort() {
        let normalized = normalize_local_email_policy_config_value(
            AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
            r#"[" Example.COM ","alice@Example.com","example.com"," ALICE@example.COM ","@Team.Example"]"#,
        )
        .unwrap();

        assert_eq!(
            normalized,
            r#"["alice@example.com","example.com","team.example"]"#
        );
    }

    #[test]
    fn rejects_invalid_policy_items() {
        assert!(
            normalize_local_email_policy_config_value(
                AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
                r#"["localhost"]"#
            )
            .is_err()
        );
        assert!(
            normalize_local_email_policy_config_value(
                AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
                r#"["alice@example"]"#
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unicode_domains_and_accepts_punycode_domains() {
        assert!(
            normalize_local_email_policy_config_value(
                AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
                r#"["用户.中国"]"#
            )
            .is_err()
        );
        assert_eq!(
            normalize_local_email_policy_config_value(
                AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
                r#"["xn--fiq228c.xn--fiqs8s"]"#,
            )
            .unwrap(),
            r#"["xn--fiq228c.xn--fiqs8s"]"#
        );
    }

    #[test]
    fn local_email_policy_allows_when_lists_are_empty() {
        let policy = LocalEmailPolicy::from_runtime_config(&RuntimeConfig::new());

        assert!(policy.check("user@example.com").is_ok());
    }

    #[test]
    fn local_email_policy_enforces_allowlist_blocklist_and_exact_domain_matching() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
            r#"["example.com","alice@other.test"]"#,
        ));
        runtime_config.apply(config_model(
            AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
            r#"["blocked@example.com","tempmail.test"]"#,
        ));
        let policy = LocalEmailPolicy::from_runtime_config(&runtime_config);

        assert!(policy.check("ALICE@example.com").is_ok());
        assert!(policy.check("alice@other.test").is_ok());
        assert!(policy.check("blocked@example.com").is_err());
        assert!(policy.check("user@tempmail.test").is_err());
        assert!(policy.check("user@sub.example.com").is_err());
        assert!(policy.check("bob@other.test").is_err());
    }

    #[test]
    fn local_email_policy_check_not_blocked_ignores_allowlist() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
            r#"["example.com"]"#,
        ));
        runtime_config.apply(config_model(
            AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
            r#"["blocked@example.com"]"#,
        ));
        let policy = LocalEmailPolicy::from_runtime_config(&runtime_config);

        assert!(policy.check_not_blocked("bob@other.test").is_ok());
        assert!(policy.check_not_blocked("blocked@example.com").is_err());
    }

    #[test]
    fn runtime_policy_list_keeps_valid_items_when_one_item_is_invalid() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(
            AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
            r#"["blocked@example.com","invalid-domain","tempmail.test"]"#,
        ));
        let policy = LocalEmailPolicy::from_runtime_config(&runtime_config);

        assert!(policy.check("blocked@example.com").is_err());
        assert!(policy.check("user@tempmail.test").is_err());
        assert!(policy.check("user@example.com").is_ok());
    }
}
