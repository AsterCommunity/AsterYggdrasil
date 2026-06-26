//! Runtime-editable system configuration cache.

use aster_forge_config::{RuntimeConfigRecord, SyncConfigSnapshot, SyncRuntimeConfig};
use parking_lot::RwLock;
use sea_orm::ConnectionTrait;

use crate::config::audit::{self, AuditLogRuntimeSettings};
use crate::db::repository::system_config_repo;
use crate::entities::system_config;
use crate::errors::Result;

pub struct RuntimeConfig {
    snapshot: SyncRuntimeConfig<system_config::Model>,
    audit_log_settings: RwLock<AuditLogRuntimeSettings>,
}

impl RuntimeConfigRecord for system_config::Model {
    fn config_key(&self) -> &str {
        &self.key
    }

    fn config_value(&self) -> &str {
        &self.value
    }

    fn config_requires_restart(&self) -> bool {
        self.requires_restart
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        Self {
            snapshot: SyncRuntimeConfig::new(),
            audit_log_settings: RwLock::new(AuditLogRuntimeSettings::default()),
        }
    }

    pub async fn reload<C: ConnectionTrait>(&self, db: &C) -> Result<()> {
        let configs = system_config_repo::find_all(db).await?;
        let next_snapshot = SyncConfigSnapshot::from_configs(configs.clone());
        let audit_log_settings = build_audit_log_settings(&next_snapshot);
        self.snapshot.replace(configs);
        *self.audit_log_settings.write() = audit_log_settings;
        Ok(())
    }

    pub fn get_model(&self, key: &str) -> Option<system_config::Model> {
        self.snapshot.get_model(key)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.snapshot.get(key)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.snapshot.get_bool(key)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.snapshot.get_i64(key)
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.snapshot.get_u64(key)
    }

    pub fn get_string_or(&self, key: &str, default: &str) -> String {
        self.snapshot.get_string_or(key, default)
    }

    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.snapshot.get_bool_or(key, default)
    }

    pub fn should_record_audit_action(&self, action: crate::types::audit::AuditAction) -> bool {
        self.audit_log_settings.read().should_record(action)
    }

    pub fn get_i64_or(&self, key: &str, default: i64) -> i64 {
        self.snapshot.get_i64_or(key, default)
    }

    pub fn get_u64_or(&self, key: &str, default: u64) -> u64 {
        self.snapshot.get_u64_or(key, default)
    }

    pub fn apply(&self, config: system_config::Model) {
        let is_audit_runtime_key = audit::is_audit_runtime_key(&config.key);
        let changed = self.snapshot.apply(config).is_some();
        if changed && is_audit_runtime_key {
            *self.audit_log_settings.write() = build_audit_log_settings(&self.snapshot.snapshot());
        }
    }

    pub fn remove(&self, key: &str) {
        let removed = self.snapshot.remove(key).is_some();
        if removed && audit::is_audit_runtime_key(key) {
            *self.audit_log_settings.write() = build_audit_log_settings(&self.snapshot.snapshot());
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn build_audit_log_settings(
    snapshot: &SyncConfigSnapshot<system_config::Model>,
) -> AuditLogRuntimeSettings {
    let enabled = snapshot.get(audit::AUDIT_LOG_ENABLED_KEY);
    let actions = snapshot.get(audit::AUDIT_LOG_RECORDED_ACTIONS_KEY);
    AuditLogRuntimeSettings::from_raw_values(enabled, actions)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::RuntimeConfig;
    use crate::config::DatabaseConfig;
    use crate::config::audit;
    use crate::config::definitions::{
        AUTH_ACCESS_TOKEN_TTL_SECS_KEY, AUTH_COOKIE_SECURE_KEY, BRANDING_TITLE_KEY,
        CONFIG_CATEGORY_SITE_BRANDING,
    };
    use crate::db;
    use crate::db::repository::system_config_repo;
    use crate::entities::system_config;
    use crate::types::audit::AuditAction;
    use aster_forge_config::{ConfigSource, ConfigValueType};
    async fn setup_db() -> sea_orm::DatabaseConnection {
        let db = db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        system_config_repo::ensure_defaults(&db).await.unwrap();
        db
    }

    fn model(key: &str, value: &str, requires_restart: bool) -> system_config::Model {
        system_config::Model {
            id: 1,
            key: key.to_string(),
            value: value.to_string(),
            value_type: ConfigValueType::String,
            requires_restart,
            is_sensitive: false,
            source: ConfigSource::System,
            visibility: aster_forge_config::ConfigVisibility::Private,
            namespace: String::new(),
            category: CONFIG_CATEGORY_SITE_BRANDING.to_string(),
            description: "test".to_string(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }

    #[tokio::test]
    async fn reload_loads_defaults_and_remove_hides_values() {
        let db = setup_db().await;
        let runtime_config = RuntimeConfig::new();

        runtime_config.reload(&db).await.unwrap();
        assert_eq!(runtime_config.get_bool(AUTH_COOKIE_SECURE_KEY), Some(true));
        assert_eq!(
            runtime_config.get_i64(AUTH_ACCESS_TOKEN_TTL_SECS_KEY),
            Some(900)
        );

        runtime_config.remove(AUTH_COOKIE_SECURE_KEY);
        assert_eq!(runtime_config.get(AUTH_COOKIE_SECURE_KEY), None);
    }

    #[tokio::test]
    async fn apply_updates_existing_runtime_values() {
        let db = setup_db().await;
        let runtime_config = RuntimeConfig::new();
        runtime_config.reload(&db).await.unwrap();

        let mut updated = system_config_repo::find_by_key(&db, BRANDING_TITLE_KEY)
            .await
            .unwrap()
            .unwrap();
        updated.value = "Custom Foundation".to_string();

        runtime_config.apply(updated);

        assert_eq!(
            runtime_config.get(BRANDING_TITLE_KEY).as_deref(),
            Some("Custom Foundation")
        );
    }

    #[tokio::test]
    async fn reload_and_apply_keep_precompiled_audit_scope_current() {
        let db = setup_db().await;
        let runtime_config = RuntimeConfig::new();
        runtime_config.reload(&db).await.unwrap();

        assert!(runtime_config.should_record_audit_action(AuditAction::ConfigUpdate));

        runtime_config.apply(model(
            audit::AUDIT_LOG_RECORDED_ACTIONS_KEY,
            r#"["user_login"]"#,
            false,
        ));
        assert!(runtime_config.should_record_audit_action(AuditAction::UserLogin));
        assert!(!runtime_config.should_record_audit_action(AuditAction::ConfigUpdate));

        runtime_config.apply(model(audit::AUDIT_LOG_ENABLED_KEY, "false", false));
        assert!(!runtime_config.should_record_audit_action(AuditAction::UserLogin));
    }

    #[tokio::test]
    async fn apply_keeps_existing_value_when_config_requires_restart() {
        let runtime_config = RuntimeConfig::new();
        runtime_config.apply(model("test.requires_restart", "old", false));
        runtime_config.apply(model("test.requires_restart", "new", true));

        assert_eq!(
            runtime_config.get("test.requires_restart").as_deref(),
            Some("old")
        );
    }
}
