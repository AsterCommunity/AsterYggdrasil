//! Instance resolution for operator commands that must never bootstrap a deployment.

use crate::config::{Config, DatabaseConfig, loader::ConfigLocation};
use crate::errors::Result;

pub(in crate::cli) struct CliContext {
    pub config: Config,
    pub database: DatabaseConfig,
}

impl CliContext {
    /// Loads configuration without writes and prepares a single-connection,
    /// zero-retry database configuration for short-lived commands.
    pub(in crate::cli) fn existing(
        location: &ConfigLocation,
        database_url: Option<&str>,
    ) -> Result<Self> {
        let config = location.load_read_only(database_url)?;
        let database = DatabaseConfig {
            pool_size: 1,
            retry_count: 0,
            ..config.database.clone()
        };
        Ok(Self { config, database })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_preserves_instance_settings_with_a_single_connection_and_no_writes() {
        let root =
            std::env::temp_dir().join(format!("aster-operator-context-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let _cleanup =
            aster_forge_utils::raii::TempDirGuard::new(root.clone(), "operator context fixture");
        let path = root.join("instance.conf");
        let original =
            "[database]\npool_size = 7\nretry_count = 5\nurl = \"sqlite://persisted.db?mode=rw\"\n";
        std::fs::write(&path, original).unwrap();
        let location = ConfigLocation::resolve(Some(&path), None).unwrap();
        let context =
            CliContext::existing(&location, Some("sqlite://override.db?mode=rw")).unwrap();
        let expected = format!("sqlite://{}?mode=rw", root.join("override.db").display());
        assert_eq!(
            context.config.database.url.as_url(),
            Some(expected.as_str())
        );
        assert_eq!(context.database.url, context.config.database.url);
        assert_eq!(
            (
                context.config.database.pool_size,
                context.config.database.retry_count
            ),
            (7, 5)
        );
        assert_eq!(
            (context.database.pool_size, context.database.retry_count),
            (1, 0)
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }
}
