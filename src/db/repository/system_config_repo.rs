//! Product-bound system config repository.
//!
//! Forge owns the shared `system_config` table contract and CRUD mechanics.
//! This module binds those mechanics to Yggdrasil's config registry,
//! deprecated-key list, cursor API type, and public error semantics.

use crate::config::definitions::{CONFIG_REGISTRY, DEPRECATED_SYSTEM_CONFIG_KEYS};
use crate::errors::{AsterError, Result};
use aster_forge_api::CursorSlice;
use aster_forge_config::ConfigVisibility;
use aster_forge_db::system_config::{self, SystemConfigDbBinding, SystemConfigUpsert};
use sea_orm::ConnectionTrait;

static STORE: SystemConfigDbBinding =
    SystemConfigDbBinding::new(&CONFIG_REGISTRY, DEPRECATED_SYSTEM_CONFIG_KEYS);

fn map_store_error(error: aster_forge_db::DbError) -> AsterError {
    let message = error.to_string();
    if message.contains("cannot delete system configuration") {
        return AsterError::auth_forbidden("cannot delete system configuration");
    }
    if let Some(key) = config_key_from_message(&message) {
        return AsterError::record_not_found(format!("config key '{key}'"));
    }
    AsterError::from(error)
}

fn config_key_from_message(message: &str) -> Option<&str> {
    let prefix = "config key '";
    let start = message.find(prefix)? + prefix.len();
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

fn map_store_result<T>(result: aster_forge_db::Result<T>) -> Result<T> {
    result.map_err(map_store_error)
}

/// Loads every system config row, including locked system entries and custom keys.
pub async fn find_all<C: ConnectionTrait>(db: &C) -> Result<Vec<system_config::Model>> {
    map_store_result(STORE.find_all(db).await)
}

/// Paginates config rows by ascending id.
///
/// `after_id` continues from the last id of the previous page; the returned [`CursorSlice`]
/// carries the total row count and a `has_more` flag.
pub async fn find_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after_id: Option<i64>,
) -> Result<CursorSlice<system_config::Model>> {
    let page = map_store_result(STORE.find_cursor(db, limit, after_id).await)?;
    Ok(CursorSlice {
        items: page.items,
        total: page.total,
        has_more: page.has_more,
    })
}

/// Loads a single config entry by its key.
///
/// Returns `None` when the key is not configured.
pub async fn find_by_key<C: ConnectionTrait>(
    db: &C,
    key: &str,
) -> Result<Option<system_config::Model>> {
    map_store_result(STORE.find_by_key(db, key).await)
}

/// Lists custom config entries visible to the requesting audience.
///
/// `include_authenticated` widens visibility from public-only to public plus
/// authenticated-only entries; system-source rows are always excluded.
pub async fn find_visible_custom<C: ConnectionTrait>(
    db: &C,
    include_authenticated: bool,
) -> Result<Vec<system_config::Model>> {
    map_store_result(STORE.find_visible_custom(db, include_authenticated).await)
}

/// Locks a config key back to the system-managed source, discarding any custom override.
///
/// Returns `Err(record_not_found)` when the key does not exist.
pub async fn lock_by_key<C: ConnectionTrait>(db: &C, key: &str) -> Result<()> {
    map_store_result(STORE.lock_by_key(db, key).await)
}

/// Upserts a config value attributed to the acting user, keeping the entry's current visibility.
pub async fn upsert<C: ConnectionTrait>(
    db: &C,
    key: &str,
    value: &str,
    updated_by: i64,
) -> Result<system_config::Model> {
    upsert_with_options(db, key, value, None, Some(updated_by)).await
}

/// Upserts a config value with an explicit actor, allowing `None` for system-driven writes.
pub async fn upsert_with_actor<C: ConnectionTrait>(
    db: &C,
    key: &str,
    value: &str,
    updated_by: Option<i64>,
) -> Result<system_config::Model> {
    upsert_with_options(db, key, value, None, updated_by).await
}

/// Upserts a config value with optional visibility override and actor attribution.
///
/// The value is validated against the Yggdrasil config registry before writing; keys unknown
/// to the registry or values failing validation return an error without modifying the row.
pub async fn upsert_with_options<C: ConnectionTrait>(
    db: &C,
    key: &str,
    value: &str,
    visibility: Option<ConfigVisibility>,
    updated_by: Option<i64>,
) -> Result<system_config::Model> {
    map_store_result(
        STORE
            .upsert_prevalidated(
                db,
                SystemConfigUpsert {
                    key,
                    value,
                    visibility,
                    updated_by,
                },
            )
            .await,
    )
}

/// Deletes a config entry by key.
///
/// Deleting system-managed entries is rejected with an auth-forbidden error.
pub async fn delete_by_key<C: ConnectionTrait>(db: &C, key: &str) -> Result<()> {
    map_store_result(STORE.delete_by_key(db, key).await)
}

/// Writes a system-sourced value for a registry key only when the key is not configured yet.
///
/// Existing values are left untouched. Returns whether a row was inserted; unknown keys are
/// rejected as a validation error.
pub async fn ensure_system_value_if_missing<C: ConnectionTrait>(
    db: &C,
    key: &str,
    value: &str,
) -> Result<bool> {
    map_store_result(STORE.ensure_system_value_if_missing(db, key, value).await)
}

/// Deletes config rows whose keys appear in the deprecated-key list, returning the number of
/// rows removed.
pub async fn delete_deprecated_keys<C: ConnectionTrait>(db: &C) -> Result<u64> {
    map_store_result(STORE.delete_deprecated_keys(db).await)
}

/// Seeds missing registry defaults and repairs metadata drift on existing rows.
///
/// Returns the number of rows inserted; running it again is a no-op that also prunes
/// deprecated keys introduced between releases.
pub async fn ensure_defaults<C: ConnectionTrait>(db: &C) -> Result<usize> {
    map_store_result(STORE.ensure_defaults(db).await)
}

#[cfg(test)]
mod tests {
    use sea_orm::{ActiveModelTrait, Set};

    use super::{
        delete_by_key, delete_deprecated_keys, ensure_defaults, ensure_system_value_if_missing,
        find_all, find_by_key, find_cursor, lock_by_key, upsert_with_actor,
    };
    use crate::config::{
        DatabaseConfig,
        definitions::{
            ALL_CONFIGS, AUTH_COOKIE_SECURE_KEY, BRANDING_TITLE_KEY, DEPRECATED_AVATAR_DIR_KEY,
            PUBLIC_SITE_URL_KEY,
        },
    };
    use aster_forge_config::{ConfigSource, ConfigValueType};
    use aster_forge_db::system_config;

    async fn build_test_db() -> sea_orm::DatabaseConnection {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("system config repo test DB should connect");
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .expect("system config repo test migrations should succeed");
        db
    }

    #[tokio::test]
    async fn ensure_defaults_uses_yggdrasil_registry_and_repairs_metadata() {
        let db = build_test_db().await;

        let inserted = ensure_defaults(&db).await.unwrap();
        assert_eq!(inserted, ALL_CONFIGS.len());
        assert_eq!(ensure_defaults(&db).await.unwrap(), 0);

        let mut active: system_config::ActiveModel = find_by_key(&db, BRANDING_TITLE_KEY)
            .await
            .unwrap()
            .unwrap()
            .into();
        active.source = Set(ConfigSource::Custom);
        active.value_type = Set(ConfigValueType::Number);
        active.requires_restart = Set(true);
        active.is_sensitive = Set(true);
        active.category = Set("wrong".to_string());
        active.description = Set("wrong".to_string());
        active.update(&db).await.unwrap();

        assert_eq!(ensure_defaults(&db).await.unwrap(), 0);
        let repaired = find_by_key(&db, BRANDING_TITLE_KEY).await.unwrap().unwrap();
        assert_eq!(repaired.source, ConfigSource::System);
        assert_eq!(repaired.value_type, ConfigValueType::String);
        assert!(!repaired.requires_restart);
        assert!(!repaired.is_sensitive);
        assert_eq!(repaired.category, "site.branding");
        assert_eq!(
            repaired.description,
            "Application title shown in the embedded frontend"
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn ensure_defaults_deletes_yggdrasil_deprecated_config_keys() {
        let db = build_test_db().await;

        upsert_with_actor(&db, DEPRECATED_AVATAR_DIR_KEY, "/tmp/avatars", None)
            .await
            .unwrap();
        upsert_with_actor(&db, "custom_keep_me", "value", None)
            .await
            .unwrap();

        let inserted = ensure_defaults(&db).await.unwrap();
        assert_eq!(inserted, ALL_CONFIGS.len());
        assert!(
            find_by_key(&db, DEPRECATED_AVATAR_DIR_KEY)
                .await
                .unwrap()
                .is_none()
        );
        assert!(find_by_key(&db, "custom_keep_me").await.unwrap().is_some());
        assert_eq!(delete_deprecated_keys(&db).await.unwrap(), 0);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn repository_preserves_product_error_semantics_and_cursor_api() {
        let db = build_test_db().await;
        ensure_defaults(&db).await.unwrap();

        let all = find_all(&db).await.unwrap();
        assert_eq!(all.len(), ALL_CONFIGS.len());
        let page = find_cursor(&db, 2, Some(all[0].id)).await.unwrap();
        assert_eq!(page.total, ALL_CONFIGS.len() as u64);
        assert!(page.has_more);
        assert_eq!(page.items.len(), 2);

        lock_by_key(&db, BRANDING_TITLE_KEY).await.unwrap();
        let missing = lock_by_key(&db, "missing_lock_key").await.unwrap_err();
        assert!(missing.message().contains("config key 'missing_lock_key'"));

        let system_error = delete_by_key(&db, BRANDING_TITLE_KEY).await.unwrap_err();
        assert!(
            system_error
                .message()
                .contains("cannot delete system configuration")
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn ensure_system_value_if_missing_uses_product_registry() {
        let db = build_test_db().await;

        assert!(
            ensure_system_value_if_missing(&db, PUBLIC_SITE_URL_KEY, r#"["https://example.com"]"#)
                .await
                .unwrap()
        );
        assert!(
            !ensure_system_value_if_missing(&db, PUBLIC_SITE_URL_KEY, r#"["https://ignored.com"]"#)
                .await
                .unwrap()
        );
        let stored = find_by_key(&db, PUBLIC_SITE_URL_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.value, r#"["https://example.com"]"#);
        assert_eq!(stored.value_type, ConfigValueType::StringArray);

        let unknown = ensure_system_value_if_missing(&db, "unknown_config_key", "value")
            .await
            .unwrap_err();
        assert!(unknown.message().contains("unknown_config_key"));

        assert!(
            ensure_system_value_if_missing(&db, AUTH_COOKIE_SECURE_KEY, "true")
                .await
                .unwrap()
        );

        db.close().await.unwrap();
    }
}
