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
            // Policy items are emails or domains: personal data, so neither the
            // item nor the forge error text (which embeds the input) may reach
            // logs. Position and size locate the entry in the stored value;
            // the admin re-reads the config value to fix it.
            tracing::warn!(
                key,
                item_kind = email_policy_item_kind(item),
                item_len = item.len(),
                error_kind = email_policy_error_kind(&error.to_string()),
                "invalid local email policy item; ignoring"
            );
        }),
        Err(error) => {
            // serde_json's invalid-type messages embed fragments of the raw
            // value; log only the parse-failure class and size, never `value`.
            tracing::warn!(
                key,
                value_len = raw.len(),
                error_kind = email_policy_error_kind(&error.to_string()),
                "invalid local email policy config JSON; using an empty list"
            );
            EmailPolicyList::default()
        }
    }
}

/// Coarse structural shape of a policy item: domain vs exact email. Never the
/// value itself.
fn email_policy_item_kind(item: &str) -> &'static str {
    if item.starts_with('@') {
        "domain"
    } else if item.contains('@') {
        "email"
    } else {
        "domain"
    }
}

/// Bounded classification of validation/parse errors, derived from the forge
/// error text without copying it. The enum set is deliberately closed so
/// downstream aggregation stays meaningful.
fn email_policy_error_kind(error_text: &str) -> &'static str {
    let text = error_text.to_ascii_lowercase();
    if text.contains("json array") || text.contains("expected") {
        "parse"
    } else if text.contains("empty") {
        "empty"
    } else if text.contains("too long") || text.contains("length") {
        "length"
    } else if text.contains("non-ascii") {
        "non_ascii"
    } else if text.contains("invalid") {
        "malformed"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::config::RuntimeConfig;
    use aster_forge_config::{ConfigSource, ConfigValueType, ConfigVisibility};
    use aster_forge_db::system_config;
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

#[cfg(test)]
mod redaction_tests {
    use super::*;
    use crate::config::RuntimeConfig;
    use aster_forge_test::logs::CapturedLogs;

    fn config_model(key: &str, value: &str) -> aster_forge_db::system_config::Model {
        aster_forge_db::system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: aster_forge_config::ConfigValueType::StringArray,
            requires_restart: false,
            is_sensitive: false,
            source: aster_forge_config::ConfigSource::System,
            visibility: aster_forge_config::ConfigVisibility::Private,
            namespace: String::new(),
            category: crate::config::definitions::CONFIG_CATEGORY_AUTH_EMAIL_POLICY.to_string(),
            description: "test".to_string(),
            updated_at: chrono::Utc::now(),
            updated_by: None,
        }
    }

    fn captured_policy_logs(raw_value: &str) -> String {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(config_model(AUTH_LOCAL_EMAIL_BLOCKLIST_KEY, raw_value));
        let logs = CapturedLogs::default();
        logs.capture(|| {
            let _ = LocalEmailPolicy::from_runtime_config(&runtime_config);
        });
        logs.text()
    }

    #[test]
    fn invalid_item_logs_never_contain_the_email_or_the_forge_error_text() {
        // The invalid entry is a personal email; the forge ValidationError
        // embeds it verbatim ("invalid email policy domain '{value}'"), so the
        // captured log must contain neither the address nor that sentence.
        let log = captured_policy_logs(r##"["person@example.com","not an email!","#ok.test"]"##);

        assert!(log.contains("invalid local email policy item"), "{log}");
        assert!(
            !log.contains("person@example.com"),
            "log must not contain the email address: {log}"
        );
        assert!(
            !log.contains("invalid email policy domain"),
            "log must not copy the forge error text: {log}"
        );
        assert!(
            !log.contains("not an email!"),
            "log must not contain the raw item: {log}"
        );
        // Structural breadcrumbs survive: size and closed-set kinds.
        assert!(log.contains("item_len"), "{log}");
        assert!(log.contains("item_kind"), "{log}");
    }

    #[test]
    fn invalid_json_logs_never_contain_the_raw_value_nor_value_fragments() {
        // A type-mismatch JSON body embeds fragments in serde errors; neither
        // the whole value nor such fragments may reach the log.
        let log = captured_policy_logs(r#"["blocked@example.com", 42]"#);

        assert!(
            log.contains("invalid local email policy config JSON"),
            "{log}"
        );
        assert!(
            !log.contains("blocked@example.com"),
            "log must not contain any address from the raw value: {log}"
        );
        assert!(
            !log.contains(r#""blocked@example.com""#),
            "log must not contain the raw JSON: {log}"
        );
        assert!(log.contains("value_len"), "{log}");
    }
}
