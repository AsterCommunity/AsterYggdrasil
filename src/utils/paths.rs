//! Path constants and configuration path adapters for AsterYggdrasil.
//!
//! Generic path rendering lives in `aster_forge_utils::paths`. This module keeps the
//! Yggdrasil-specific defaults and maps shared utility errors into the local configuration error
//! type at the application boundary.

use crate::errors::{AsterError, Result};
use std::path::Path;

/// Default data directory, resolved relative to the process working directory.
pub const DEFAULT_DATA_DIR: &str = "data";
/// Default config file path inside the data directory.
pub const DEFAULT_CONFIG_PATH: &str = "data/config.toml";
/// Default SQLite database file path inside the data directory.
pub const DEFAULT_SQLITE_DATABASE_PATH: &str = "data/asteryggdrasil.db";
/// Default SQLite URL used when writing the config file; relative to the data directory.
pub const DEFAULT_CONFIG_SQLITE_DATABASE_URL: &str = "sqlite://asteryggdrasil.db?mode=rwc";
/// Default SQLite URL used when opening the database at runtime; relative to the process working directory.
pub const DEFAULT_SQLITE_DATABASE_URL: &str = "sqlite://data/asteryggdrasil.db?mode=rwc";
/// Default temp directory written into the config file; relative to the data directory.
pub const DEFAULT_CONFIG_TEMP_DIR: &str = ".tmp";
/// Default temp directory used at runtime; relative to the process working directory.
pub const DEFAULT_TEMP_DIR: &str = "data/.tmp";

fn map_utils_config(error: aster_forge_utils::UtilsError) -> AsterError {
    AsterError::config_error(error.to_string())
}

/// Resolves a configured path value against the config directory, keeping it inside `base_dir`.
///
/// Plain values are interpreted relative to `config_dir`; values already prefixed with the
/// data directory name (or absolute values) are kept as-is. Returns an error when the result
/// escapes `base_dir`.
pub fn resolve_config_relative_path(
    base_dir: &Path,
    config_dir: &Path,
    value: &str,
) -> Result<String> {
    aster_forge_utils::paths::resolve_config_relative_path(base_dir, config_dir, value)
        .map_err(map_utils_config)
}

/// Resolves a configured `sqlite://` URL so its database file stays inside `base_dir`.
///
/// Plain relative file paths are re-rooted under `config_dir`; URLs already prefixed with the
/// data directory name or pointing to an absolute path are kept unchanged. Returns an error
/// when the resolved database path escapes `base_dir`.
pub fn resolve_config_relative_sqlite_url(
    base_dir: &Path,
    config_dir: &Path,
    value: &str,
) -> Result<String> {
    aster_forge_utils::paths::resolve_config_relative_sqlite_url(base_dir, config_dir, value)
        .map_err(map_utils_config)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CONFIG_SQLITE_DATABASE_URL, resolve_config_relative_path,
        resolve_config_relative_sqlite_url,
    };
    use std::path::Path;

    #[test]
    fn resolve_config_relative_path_accepts_plain_and_data_prefixed_relative_values() {
        let base_dir = Path::new("/srv/asteryggdrasil");
        let config_dir = Path::new("/srv/asteryggdrasil/data");

        assert_eq!(
            resolve_config_relative_path(base_dir, config_dir, ".tmp").unwrap(),
            "data/.tmp"
        );
        assert_eq!(
            resolve_config_relative_path(base_dir, config_dir, "data/.tmp").unwrap(),
            "data/.tmp"
        );
        assert_eq!(
            resolve_config_relative_path(base_dir, config_dir, "../shared").unwrap(),
            "shared"
        );
    }

    #[test]
    fn resolve_config_relative_sqlite_url_accepts_plain_and_data_prefixed_relative_values() {
        let base_dir = Path::new("/srv/asteryggdrasil");
        let config_dir = Path::new("/srv/asteryggdrasil/data");

        assert_eq!(
            resolve_config_relative_sqlite_url(
                base_dir,
                config_dir,
                DEFAULT_CONFIG_SQLITE_DATABASE_URL
            )
            .unwrap(),
            "sqlite://data/asteryggdrasil.db?mode=rwc"
        );
        assert_eq!(
            resolve_config_relative_sqlite_url(
                base_dir,
                config_dir,
                "sqlite://data/asteryggdrasil.db?mode=rwc"
            )
            .unwrap(),
            "sqlite://data/asteryggdrasil.db?mode=rwc"
        );
        assert_eq!(
            resolve_config_relative_sqlite_url(
                base_dir,
                config_dir,
                "sqlite:///var/lib/asteryggdrasil/custom.db?mode=rwc"
            )
            .unwrap(),
            "sqlite:///var/lib/asteryggdrasil/custom.db?mode=rwc"
        );
    }

    #[test]
    fn resolve_config_relative_path_rejects_values_outside_base_dir() {
        let base_dir = Path::new("/srv/asteryggdrasil");
        let config_dir = Path::new("/srv/asteryggdrasil/data");

        let error = resolve_config_relative_path(base_dir, config_dir, "../../shared")
            .expect_err("path outside base_dir should be rejected");
        assert!(error.to_string().contains("outside data base_dir"));
    }

    #[test]
    fn resolve_config_relative_sqlite_url_rejects_values_outside_base_dir() {
        let base_dir = Path::new("/srv/asteryggdrasil");
        let config_dir = Path::new("/srv/asteryggdrasil/data");

        let error = resolve_config_relative_sqlite_url(
            base_dir,
            config_dir,
            "sqlite://../../shared/asteryggdrasil.db?mode=rwc",
        )
        .expect_err("sqlite path outside base_dir should be rejected");
        assert!(error.to_string().contains("outside data base_dir"));
    }
}
