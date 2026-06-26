//! Process bootstrap for the HTTP runtime.
//!
//! This module owns process-level startup steps that happen before runtime
//! components are assembled: panic hook installation, environment loading,
//! static configuration loading, CSRF middleware naming, logging setup, and
//! product runtime state preparation.

/// Prepared process state needed by the runtime entrypoint.
pub struct BootstrappedRuntime {
    /// Prepared product runtime state and startup report.
    pub prepared: crate::runtime::startup::PreparedRuntime,
    _logging: aster_forge_logging::LoggingInitResult,
}

/// Installs process hooks, loads configuration, initializes logging, and prepares runtime state.
pub async fn bootstrap() -> crate::errors::Result<BootstrappedRuntime> {
    install_panic_hook();
    dotenvy::dotenv().ok();

    let config = crate::config::init_config()?;
    crate::api::middleware::csrf::init_token_names_from_auth_config(&config.auth)?;

    let logging = aster_forge_logging::init_logging(&config.logging);
    if let Some(warning) = logging.warning.as_ref() {
        tracing::warn!("{warning}");
    }

    let prepared = crate::runtime::startup::prepare(config).await?;

    Ok(BootstrappedRuntime {
        prepared,
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
