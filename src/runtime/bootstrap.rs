//! Process bootstrap for the HTTP runtime.
//!
//! This module owns process-level startup steps that happen before runtime
//! components are assembled: panic hook installation, environment loading,
//! static configuration loading, CSRF middleware naming, logging setup, and
//! product runtime state preparation.

use std::io;
use std::path::Path;
use std::sync::Arc;

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
    let writer = prepare_report_writer(Path::new(crate::utils::paths::DEFAULT_DATA_DIR));
    aster_forge_panic::install_panic_hook(
        aster_forge_panic::PanicHookConfig::new(
            "AsterYggdrasil",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY"),
        )
        .with_report_writer(writer),
    );
}

/// Prepares the private crash-report writer before hook installation.
///
/// Forge panic hooks no longer create ancestor directories, so the product
/// bootstrap owns the crash-log parent and fails closed to a no-op writer when
/// the destination cannot be prepared; a panic during startup must not mask the
/// original panic with filesystem errors.
fn prepare_report_writer(data_dir: &Path) -> Arc<dyn aster_forge_panic::ReportWriter> {
    let path = data_dir.join("crash.log");
    match std::fs::create_dir_all(data_dir)
        .and_then(|()| aster_forge_panic::PrivateReportWriter::try_new(&path))
    {
        Ok(writer) => Arc::new(writer),
        Err(error) => {
            eprintln!(
                "warning: private crash reports are unavailable at {}: {error}",
                path.display()
            );
            Arc::new(UnavailableReportWriter)
        }
    }
}

struct UnavailableReportWriter;

impl aster_forge_panic::ReportWriter for UnavailableReportWriter {
    fn write_report(&self, _report: &str) -> io::Result<()> {
        Err(io::Error::other(
            "private crash report storage is unavailable",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_report_writer;
    use std::path::Path;

    #[test]
    fn prepare_creates_the_crash_log_parent_before_hook_installation() {
        let root = aster_forge_test::temp::TestTempDir::new("panic-report-bootstrap");
        let data_dir = root.path().join("data");
        let path = data_dir.join("crash.log");
        let writer = prepare_report_writer(&data_dir);

        assert!(path.parent().is_some_and(Path::exists));
        // Linux has the private-file mechanics; other platforms fail closed by
        // design (Forge's private writer rejects every path with Unsupported).
        #[cfg(target_os = "linux")]
        {
            writer
                .write_report("panic report")
                .expect("prepared report writer should accept reports");
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "panic report");
        }
        #[cfg(not(target_os = "linux"))]
        assert!(writer.write_report("panic report").is_err());
    }

    #[test]
    fn unavailable_crash_log_does_not_replace_the_blocking_directory() {
        let root = aster_forge_test::temp::TestTempDir::new("panic-report-unavailable");
        let blocked = root.path().join("data");
        std::fs::create_dir(&blocked).unwrap();

        let writer = prepare_report_writer(&blocked);
        assert!(writer.write_report("panic report").is_err());
        assert_eq!(std::fs::read_dir(&blocked).unwrap().count(), 0);
    }
}
