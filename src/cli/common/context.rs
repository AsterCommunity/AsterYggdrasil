//! Resolves the instance configuration once for operational CLI commands.
//!
//! Operational reads must never bootstrap a deployment: a missing config file
//! is only acceptable when an explicit database target was provided.
#![expect(
    dead_code,
    reason = "plumbing lands first; consumers arrive with the config, doctor, \
              and database-migrate command sub-issues"
)]

use super::output::OutputFormat;
use crate::config::loader::{ConfigLocation, load_read_only_at};
use crate::config::{Config, DatabaseConfig};
use crate::errors::{AsterError, Result};

/// Shared inputs for an operational command: resolved config, effective
/// database target, and the requested output format.
#[derive(Debug)]
pub struct CliContext {
    pub config: Config,
    pub database: DatabaseConfig,
    pub output_format: OutputFormat,
}

impl CliContext {
    /// Loads the instance read-only and applies the explicit database URL
    /// override. When neither `--database-url` nor `ASTER__DATABASE__URL` is
    /// present, the config file must already exist.
    pub fn existing(
        location: &ConfigLocation,
        database_url: Option<&str>,
        output_format: OutputFormat,
    ) -> Result<Self> {
        let env_database_url = std::env::var("ASTER__DATABASE__URL").ok();
        let allow_missing =
            database_url.is_some() || env_database_url.as_deref().is_some_and(|url| !url.is_empty());
        let mut config = load_read_only_at(location, allow_missing)?;
        if let Some(url) = database_url {
            if url.trim().is_empty() {
                return Err(AsterError::validation_error(
                    "--database-url must not be empty",
                ));
            }
            config.database.url = aster_forge_db::DatabaseUrl::Url(url.to_string());
        }
        // Operational commands run short, focused queries; a single pooled
        // connection without retries keeps failures loud and local.
        let database = DatabaseConfig {
            pool_size: 1,
            retry_count: 0,
            ..config.database.clone()
        };
        Ok(Self {
            config,
            database,
            output_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_database_url_is_rejected() {
        let location = ConfigLocation::resolve(Some(std::path::Path::new(
            "/nonexistent/config.toml",
        )), None)
        .expect("resolve should succeed");
        let error = CliContext::existing(&location, Some("   "), OutputFormat::Json)
            .expect_err("empty url must be rejected");
        assert!(error.message().contains("--database-url must not be empty"));
    }

    #[test]
    fn missing_config_without_database_url_is_an_error() {
        let location = ConfigLocation::resolve(Some(std::path::Path::new(
            "/nonexistent/asteryggdrasil/config.toml",
        )), None)
        .expect("resolve should succeed");
        let error = CliContext::existing(&location, None, OutputFormat::Json)
            .expect_err("missing config must not bootstrap");
        assert!(error.message().contains("does not exist"));
    }

    #[test]
    fn missing_config_with_explicit_url_falls_back_to_defaults() {
        let location = ConfigLocation::resolve(Some(std::path::Path::new(
            "/nonexistent/asteryggdrasil/config.toml",
        )), None)
        .expect("resolve should succeed");
        let context = CliContext::existing(&location, Some("sqlite://memory.db"), OutputFormat::Json)
            .expect("explicit url allows missing config");
        assert_eq!(
            context.database.url,
            aster_forge_db::DatabaseUrl::Url("sqlite://memory.db".to_string())
        );
        assert_eq!(context.database.pool_size, 1);
        assert_eq!(context.database.retry_count, 0);
    }
}
