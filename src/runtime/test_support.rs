//! Shared test fixtures for runtime state construction.
//!
//! Service tests build the full `AppState` through [`setup_state`]/[`setup_state_from_db`]
//! instead of hand-assembling `AppStateParts`, so a new runtime dependency only needs one
//! fixture update.

use std::sync::Arc;

use aster_forge_audit::AuditWriter;

use super::state::{AppState, AppStateParts};
use crate::config::{Config, DatabaseConfig, RuntimeConfig};

/// Makes `crate::api::configure` safe in unit tests that do not run bootstrap:
/// `routes/mod.rs` reads rate-limit and network-trust settings from the
/// process-wide static config at route registration time (pre-existing debt;
/// the routes should take those as explicit parameters instead).
pub fn ensure_static_config() {
    let _ = crate::config::set_config_for_test(Arc::new(Config::default()));
}

/// Builds a fully migrated `AppState` on an in-memory SQLite database.
pub async fn setup_state() -> AppState {
    let db = connect_in_memory().await;
    setup_state_from_db(db).await
}

/// Builds an `AppState` from an already-connected database. Migrations are the
/// caller's responsibility; they have usually just run on this connection.
pub async fn setup_state_from_db(db: sea_orm::DatabaseConnection) -> AppState {
    setup_state_with_config(db, Arc::new(Config::default())).await
}

/// Builds an `AppState` from an already-connected database and an explicit config.
/// Used by tests that need a customized `Config` (e.g. a temp local storage root).
pub async fn setup_state_with_config(
    db: sea_orm::DatabaseConnection,
    config: Arc<Config>,
) -> AppState {
    let runtime_config = Arc::new(RuntimeConfig::new());
    runtime_config
        .reload(&db)
        .await
        .expect("runtime config should load in test state");
    runtime_config
        .install_auth_runtimes(
            crate::config::TEST_JWT_SECRET,
            config.auth.password_hash_max_concurrency,
        )
        .await
        .expect("auth runtimes should install in test state");
    let cache = aster_forge_cache::create_cache(&config.cache).await;

    let state = AppState::from_parts(AppStateParts {
        db_handles: aster_forge_db::DbHandles::single(db.clone()),
        config: config.clone(),
        runtime_config,
        cache,
        texture_storage: crate::texture_storage::create_texture_store(&config.texture_storage)
            .expect("test object storage should initialize"),
        mail_sender: aster_forge_mail::memory_sender(),
        config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test("aster_yggdrasil"),
        metrics: aster_forge_metrics::NoopMetrics::arc(),
        audit_writer: Arc::new(AuditWriter::new(db)),
    })
    .expect("test AppState should build");

    // Integration-style fixtures never run the runtime component bundle, so the
    // writer is started here; the component path in these test processes never
    // reaches its own start phase.
    state
        .audit_writer()
        .start()
        .await
        .expect("test audit writer should start");

    state
}

/// Connects an in-memory SQLite database and runs all migrations.
pub async fn connect_in_memory() -> sea_orm::DatabaseConnection {
    let db = crate::db::connect_with_metrics(
        &DatabaseConfig {
            url: "sqlite::memory:".into(),
            pool_size: 1,
            retry_count: 0,
        },
        aster_forge_metrics::NoopMetrics::arc(),
    )
    .await
    .expect("test database should connect");
    aster_yggdrasil_migration::Migrator::up(&db, None)
        .await
        .expect("test database migration should run");
    db
}
