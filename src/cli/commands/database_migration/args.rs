//! Command-line arguments for the database migration command.

#[derive(Debug, Clone, clap::Args)]
pub(in crate::cli) struct DatabaseMigrateArgs {
    /// Source database URL
    #[arg(long, env = "ASTER_CLI_SOURCE_DATABASE_URL", hide_env_values = true)]
    pub source_database_url: String,
    /// Target database URL
    #[arg(long, env = "ASTER_CLI_TARGET_DATABASE_URL", hide_env_values = true)]
    pub target_database_url: String,
    /// Preview the migration without writing to the target
    #[arg(long, env = "ASTER_CLI_DRY_RUN", default_value_t = false)]
    pub dry_run: bool,
    /// Verify an already-migrated target without copying
    #[arg(long, env = "ASTER_CLI_VERIFY_ONLY", default_value_t = false)]
    pub verify_only: bool,
    /// Confirm target schema and data writes without an interactive prompt
    #[arg(long, env = "ASTER_CLI_YES", default_value_t = false)]
    pub yes: bool,
}
