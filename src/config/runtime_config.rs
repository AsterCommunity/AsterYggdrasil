//! Runtime-editable system configuration cache.

use std::sync::Arc;

use aster_forge_auth::{JwtPolicy, JwtSigner, JwtVerifier, PasswordHashRuntime};
use aster_forge_config::{SyncConfigSnapshot, SyncRuntimeConfig};
use parking_lot::RwLock;
use sea_orm::ConnectionTrait;

use crate::config::audit::{self, AuditLogRuntimeSettings};
use crate::db::repository::system_config_repo;
use crate::errors::{AsterError, Result};
use aster_forge_db::system_config;

/// Fixed `iss` claim stamped into every JWT issued by this service.
pub const JWT_ISSUER: &str = "aster_yggdrasil";
/// Fixed `aud` claim stamped into every JWT issued by this service.
pub const JWT_AUDIENCE: &str = "aster_yggdrasil";

/// Forge auth runtimes built from the static JWT secret, the startup runtime
/// snapshot, and the static password-hash concurrency limit.
///
/// The JWT signer/verifier policy and password hash runtime are installed once
/// during startup.
pub struct AuthRuntimes {
    jwt_signer: JwtSigner,
    jwt_verifier: JwtVerifier,
    password_hash_runtime: PasswordHashRuntime,
    dummy_password_hash: Arc<String>,
}

impl AuthRuntimes {
    /// Builds the auth runtimes for a JWT secret, TTL snapshot, and Argon2
    /// concurrency limit.
    ///
    /// # Errors
    /// Returns an error when the secret or TTLs are invalid for Forge JWT,
    /// when Forge rejects the password hash concurrency, or when the dummy hash
    /// cannot be produced.
    pub async fn new(
        jwt_secret: &str,
        access_ttl_secs: u64,
        refresh_ttl_secs: u64,
        password_hash_max_concurrency: usize,
    ) -> Result<Arc<Self>> {
        use aster_forge_auth::JwtKey;

        let key = JwtKey::hmac(jwt_secret)
            .map_err(|error| AsterError::internal_error(format!("jwt key: {error}")))?;
        let policy = JwtPolicy::new(JWT_ISSUER, JWT_AUDIENCE, access_ttl_secs, refresh_ttl_secs)
            .map_err(|error| AsterError::internal_error(format!("jwt policy: {error}")))?;
        let jwt_signer = JwtSigner::with_system_clock(key.clone(), policy.clone());
        let jwt_verifier = JwtVerifier::with_system_clock(key, policy);
        let password_hash_runtime = PasswordHashRuntime::new(password_hash_max_concurrency)
            .map_err(|error| AsterError::config_error(format!("password hash runtime: {error}")))?;
        // Burned against unknown identifiers so login timing does not reveal
        // whether an account exists.
        let dummy_password = format!("dummy:{}", uuid::Uuid::new_v4());
        let dummy_password_hash = Arc::new(
            password_hash_runtime
                .hash_password(&dummy_password)
                .await
                .map_err(|error| {
                    AsterError::internal_error(format!("dummy password hash: {error}"))
                })?,
        );
        Ok(Arc::new(Self {
            jwt_signer,
            jwt_verifier,
            password_hash_runtime,
            dummy_password_hash,
        }))
    }

    pub fn jwt_signer(&self) -> &JwtSigner {
        &self.jwt_signer
    }

    pub fn jwt_verifier(&self) -> &JwtVerifier {
        &self.jwt_verifier
    }

    pub fn password_hash_runtime(&self) -> &PasswordHashRuntime {
        &self.password_hash_runtime
    }

    pub fn dummy_password_hash(&self) -> &str {
        &self.dummy_password_hash
    }
}

/// Process-wide cache of the `system_config` table plus the auth runtimes and
/// precompiled audit settings derived from it. Values are refreshed by
/// [`RuntimeConfig::reload`] or updated in place via [`RuntimeConfig::apply`].
pub struct RuntimeConfig {
    snapshot: SyncRuntimeConfig<system_config::Model>,
    audit_log_settings: RwLock<AuditLogRuntimeSettings>,
    auth_runtimes: RwLock<Option<Arc<AuthRuntimes>>>,
}

impl RuntimeConfig {
    /// Creates an empty store holding only defaults; call
    /// [`RuntimeConfig::reload`] (server path) or [`RuntimeConfig::apply`]
    /// (tests) to populate it.
    pub fn new() -> Self {
        Self {
            snapshot: SyncRuntimeConfig::new(),
            audit_log_settings: RwLock::new(AuditLogRuntimeSettings::default()),
            auth_runtimes: RwLock::new(None),
        }
    }

    /// Installs the auth runtimes after the runtime snapshot has been loaded.
    ///
    /// JWT TTLs are read from the runtime snapshot; `password_hash_max_concurrency`
    /// comes from static configuration. Both are installed at startup.
    ///
    /// # Errors
    /// Returns an error when the JWT secret, TTL snapshot values, or password
    /// hash concurrency are invalid.
    pub async fn install_auth_runtimes(
        &self,
        jwt_secret: &str,
        password_hash_max_concurrency: usize,
    ) -> Result<()> {
        use crate::config::auth_runtime::{
            AUTH_ACCESS_TOKEN_TTL_SECS_KEY, AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
            DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS, DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS,
        };
        let access_ttl_secs = u64::try_from(self.get_i64_or(
            AUTH_ACCESS_TOKEN_TTL_SECS_KEY,
            i64::try_from(DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS).unwrap_or(900),
        ))
        .unwrap_or(DEFAULT_AUTH_ACCESS_TOKEN_TTL_SECS);
        let refresh_ttl_secs = u64::try_from(self.get_i64_or(
            AUTH_REFRESH_TOKEN_TTL_SECS_KEY,
            i64::try_from(DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS).unwrap_or(604_800),
        ))
        .unwrap_or(DEFAULT_AUTH_REFRESH_TOKEN_TTL_SECS);
        *self.auth_runtimes.write() = Some(
            AuthRuntimes::new(
                jwt_secret,
                access_ttl_secs,
                refresh_ttl_secs,
                password_hash_max_concurrency,
            )
            .await?,
        );
        Ok(())
    }

    /// Returns the installed Forge auth runtimes.
    ///
    /// # Errors
    /// Returns an error when [`RuntimeConfig::install_auth_runtimes`] has not
    /// run yet (miswired startup or test fixture).
    pub fn auth_runtimes(&self) -> Result<Arc<AuthRuntimes>> {
        self.auth_runtimes.read().clone().ok_or_else(|| {
            AsterError::internal_error("auth runtimes are not installed in runtime config")
        })
    }

    /// Convenience accessor returning the installed auth runtimes, which own
    /// the JWT signer. See [`RuntimeConfig::auth_runtimes`] for error cases.
    pub fn jwt_signer(&self) -> Result<Arc<AuthRuntimes>> {
        self.auth_runtimes()
    }

    /// Replaces the whole snapshot with the current `system_config` rows and
    /// rebuilds the precompiled audit settings.
    pub async fn reload<C: ConnectionTrait>(&self, db: &C) -> Result<()> {
        let configs = system_config_repo::find_all(db).await?;
        let next_snapshot = SyncConfigSnapshot::from_configs(configs.clone());
        let audit_log_settings = build_audit_log_settings(&next_snapshot);
        self.snapshot.replace(configs);
        *self.audit_log_settings.write() = audit_log_settings;
        Ok(())
    }

    /// Returns the full stored row for `key`, including metadata such as
    /// value type and source.
    pub fn get_model(&self, key: &str) -> Option<system_config::Model> {
        self.snapshot.get_model(key)
    }

    /// Returns the stored string value for `key`.
    pub fn get(&self, key: &str) -> Option<String> {
        self.snapshot.get(key)
    }

    /// Returns the stored boolean value for `key`, or `None` when the key is
    /// missing or does not parse as a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.snapshot.get_bool(key)
    }

    /// Returns the stored numeric value for `key` as `i64`, or `None` when the
    /// key is missing or does not parse as a number.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.snapshot.get_i64(key)
    }

    /// Returns the stored numeric value for `key` as `u64`, or `None` when the
    /// key is missing, does not parse as a number, or is negative.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.snapshot.get_u64(key)
    }

    /// Returns the stored string value for `key`, or `default` when missing.
    pub fn get_string_or(&self, key: &str, default: &str) -> String {
        self.snapshot.get_string_or(key, default)
    }

    /// Returns the stored boolean value for `key`, or `default` when missing
    /// or not a valid boolean.
    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.snapshot.get_bool_or(key, default)
    }

    /// Returns whether the precompiled audit settings record `action`.
    pub fn should_record_audit_action(
        &self,
        action: aster_yggdrasil_model::types::audit::AuditAction,
    ) -> bool {
        self.audit_log_settings.read().should_record(action)
    }

    /// Returns the stored numeric value for `key` as `i64`, or `default` when
    /// missing or not a valid number.
    pub fn get_i64_or(&self, key: &str, default: i64) -> i64 {
        self.snapshot.get_i64_or(key, default)
    }

    /// Returns the stored numeric value for `key` as `u64`, or `default` when
    /// missing, invalid, or negative.
    pub fn get_u64_or(&self, key: &str, default: u64) -> u64 {
        self.snapshot.get_u64_or(key, default)
    }

    /// Applies a single stored row to the snapshot. Rows flagged
    /// `requires_restart` keep their previous value until restart; audit keys
    /// additionally rebuild the precompiled audit settings.
    pub fn apply(&self, config: system_config::Model) {
        let is_audit_runtime_key = audit::is_audit_runtime_key(&config.key);
        let changed = self.snapshot.apply(config).is_some();
        if changed && is_audit_runtime_key {
            *self.audit_log_settings.write() = build_audit_log_settings(&self.snapshot.snapshot());
        }
    }

    /// Removes `key` from the snapshot so reads fall back to defaults; audit
    /// keys additionally rebuild the precompiled audit settings.
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
    use std::task::Poll;

    use chrono::Utc;

    use super::{AuthRuntimes, RuntimeConfig};
    use crate::config::DatabaseConfig;
    use crate::config::audit;
    use crate::config::definitions::{
        AUTH_ACCESS_TOKEN_TTL_SECS_KEY, AUTH_COOKIE_SECURE_KEY, BRANDING_TITLE_KEY,
        CONFIG_CATEGORY_SITE_BRANDING,
    };
    use crate::db;
    use crate::db::repository::system_config_repo;
    use aster_forge_config::{ConfigSource, ConfigValueType};
    use aster_forge_db::system_config;
    use aster_yggdrasil_model::types::audit::AuditAction;
    async fn setup_db() -> sea_orm::DatabaseConnection {
        let db = db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .unwrap();
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

    #[tokio::test]
    async fn third_password_job_waits_for_a_permit() {
        let runtimes = AuthRuntimes::new(crate::config::TEST_JWT_SECRET, 900, 604_800, 2)
            .await
            .unwrap();

        let runtime = runtimes.password_hash_runtime();
        let hash = runtimes.dummy_password_hash();

        // Two in-flight verifications hold both permits; the probe must then
        // observe a saturated pool immediately. Saturation is re-confirmed in
        // the same synchronous poll sequence as the probe because a long
        // test-thread suspension can let the in-flight Argon2 jobs complete
        // and release permits between separately awaited steps.
        let mut first = Box::pin(runtime.try_verify_password("probe", hash));
        let mut second = Box::pin(runtime.try_verify_password("probe", hash));
        let mut probe = Box::pin(runtime.try_verify_password("probe", hash));
        let busy = loop {
            // Drive both placeholders into permit-holding state; the results
            // are re-confirmed in the synchronous window below.
            let _ = futures::poll!(first.as_mut());
            let _ = futures::poll!(second.as_mut());

            // Synchronous window: re-confirm both permits are still held and
            // probe Busy without any await in between, so the only way the
            // pool drains here is an Argon2 job completing inside this
            // microsecond-scale sequence.
            if !futures::poll!(first.as_mut()).is_pending()
                || !futures::poll!(second.as_mut()).is_pending()
            {
                // A suspension longer than one Argon2 verification released a
                // permit: rebuild the placeholders instead of asserting
                // against stale saturation.
                first = Box::pin(runtime.try_verify_password("probe", hash));
                second = Box::pin(runtime.try_verify_password("probe", hash));
                continue;
            }

            match futures::poll!(probe.as_mut()) {
                Poll::Ready(Err(aster_forge_auth::PasswordWorkError::Busy)) => break true,
                other => {
                    // The probe slipped in and holds a permit (`Pending`) or
                    // already finished (`Ready(Ok)`). Drain it so its job
                    // releases the permit before rebuilding saturation.
                    if matches!(other, Poll::Pending) {
                        let _ = probe.as_mut().await;
                    }
                    first = Box::pin(runtime.try_verify_password("probe", hash));
                    second = Box::pin(runtime.try_verify_password("probe", hash));
                    probe = Box::pin(runtime.try_verify_password("probe", hash));
                }
            }
        };
        assert!(busy, "saturated pool must reject try_verify immediately");

        // A normal hash queues instead of failing.
        let mut third = Box::pin(runtime.hash_password("queued-password"));
        assert!(futures::poll!(third.as_mut()).is_pending());

        assert!(!first.await.unwrap().is_valid);
        assert!(!second.await.unwrap().is_valid);
        assert!(
            third
                .await
                .unwrap()
                .starts_with("$argon2id$v=19$m=65536,t=3,p=4$")
        );
    }

    #[tokio::test]
    async fn auth_runtimes_rejects_zero_password_hash_concurrency() {
        assert!(
            AuthRuntimes::new(crate::config::TEST_JWT_SECRET, 900, 604_800, 0)
                .await
                .is_err()
        );
    }
}
