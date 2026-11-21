//! Process bootstrap for the HTTP runtime.
//!
//! This module owns process-level startup steps that happen before runtime
//! components are assembled: panic hook installation, environment loading,
//! static configuration loading, CSRF middleware naming, logging setup, and
//! product runtime state preparation.

/// Prepared process state needed by the runtime entrypoint.
pub struct BootstrappedRuntime {
    /// Prepared product runtime state.
    pub state: crate::runtime::AppState,
    _logging: aster_forge_logging::LoggingInitResult,
}

/// Installs process hooks, loads configuration, initializes logging, and prepares runtime state.
pub async fn bootstrap() -> crate::errors::Result<BootstrappedRuntime> {
    install_panic_hook();
    dotenvy::dotenv().ok();

    // Forge JWT signing requires the crypto backend to be initialized before the
    // first token is issued or verified. A second initialization is a no-op.
    if let Err(error) = aster_forge_auth::initialize_jwt_crypto_provider() {
        tracing::warn!(%error, "JWT crypto provider was already initialized");
    }

    let config = crate::config::init_config()?;
    crate::config::deployment::validate_static(config.as_ref())?;
    crate::api::middleware::csrf::init_token_names_from_auth_config(&config.auth)?;

    let logging = aster_forge_logging::init_logging(&config.logging);
    if let Some(warning) = logging.warning.as_ref() {
        tracing::warn!("{warning}");
    }

    let state = crate::runtime::startup::common::prepare_common_state(config).await?;

    Ok(BootstrappedRuntime {
        state,
        _logging: logging,
    })
}

fn install_panic_hook() {
    aster_forge_panic::install_panic_hook(aster_forge_panic::PanicHookConfig::new(
        "AsterYggdrasil",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
    ));
}
