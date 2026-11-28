//! Static Clap command tree and command-line parsing.

use super::common::help;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// `--help` footer for builds including the operational commands.
const AFTER_HELP_WITH_OPERATIONS: &str = "Examples:\n  aster_yggdrasil serve\n  aster_yggdrasil serve api\n  aster_yggdrasil --data-dir /srv/asteryggdrasil serve worker\n  printf '%s' \"$PW\" | aster_yggdrasil hash-password\n\nRunning without a command starts all server roles.";
/// `--help` footer for server-only builds without operational commands.
const AFTER_HELP_SERVER_ONLY: &str = "Examples:\n  aster_yggdrasil serve\n  aster_yggdrasil serve api\n  aster_yggdrasil --data-dir /srv/asteryggdrasil serve worker\n\nRunning without a command starts all server roles.";

#[derive(Debug, Parser)]
#[command(
    name = "aster_yggdrasil",
    display_name = "AsterYggdrasil",
    version,
    propagate_version = true,
    about = env!("CARGO_PKG_DESCRIPTION"),
    after_help = if cfg!(feature = "operations-cli") {
        AFTER_HELP_WITH_OPERATIONS
    } else {
        AFTER_HELP_SERVER_ONLY
    },
    styles = help::cli_styles()
)]
pub(super) struct RootCli {
    /// Instance configuration file
    #[arg(
        long,
        global = true,
        env = "ASTER_CLI_CONFIG_FILE",
        value_name = "PATH",
        conflicts_with = "data_dir"
    )]
    pub(super) config_file: Option<PathBuf>,
    /// Instance root directory (contains the data/ directory)
    #[arg(
        long,
        global = true,
        env = "ASTER_CLI_DATA_DIR",
        value_name = "PATH",
        conflicts_with = "config_file"
    )]
    pub(super) data_dir: Option<PathBuf>,
    #[command(subcommand)]
    pub(super) command: Option<RootCommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub(super) enum RootCommand {
    /// Run the server with all, API, or worker responsibilities
    Serve {
        /// Process responsibilities
        #[arg(value_enum, default_value = "all")]
        role: crate::runtime::role::ServeRole,
    },
    #[cfg(feature = "operations-cli")]
    /// Generate an authentication verifier (masked terminal input, or stdin until EOF)
    HashPassword,
    #[cfg(feature = "operations-cli")]
    /// Migrate data between database backends
    DatabaseMigrate {
        #[arg(
            long,
            env = "ASTER_CLI_OUTPUT_FORMAT",
            default_value = "auto",
            value_enum
        )]
        output_format: crate::cli::common::output::OutputFormat,
        #[command(flatten)]
        args: crate::cli::commands::database_migration::DatabaseMigrateArgs,
    },
    /// Show build information
    Version {
        /// Output build information as JSON
        #[arg(long)]
        json: bool,
    },
}

#[cfg(test)]
fn parsed(args: &[&str]) -> RootCli {
    let full = std::iter::once("aster_yggdrasil").chain(args.iter().copied());
    RootCli::try_parse_from(full).unwrap_or_else(|error| panic!("args should parse: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::role::ServeRole;

    fn parsed_role(args: &[&str]) -> ServeRole {
        match parsed(args).command {
            None => ServeRole::All,
            Some(RootCommand::Serve { role }) => role,
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn default_and_explicit_role_commands() {
        for (args, expected) in [
            (vec![], ServeRole::All),
            (vec!["serve"], ServeRole::All),
            (vec!["serve", "all"], ServeRole::All),
            (vec!["serve", "api"], ServeRole::Api),
            (vec!["serve", "worker"], ServeRole::Worker),
        ] {
            assert_eq!(parsed_role(&args), expected, "args: {args:?}");
        }
    }

    #[test]
    fn location_arguments_parse_globally_and_reject_conflicts() {
        assert_eq!(
            parsed(&["--data-dir", "/srv/ygg/data"]).data_dir,
            Some(PathBuf::from("/srv/ygg/data"))
        );
        assert_eq!(
            parsed(&["serve", "--config-file", "/etc/ygg/config.toml"]).config_file,
            Some(PathBuf::from("/etc/ygg/config.toml"))
        );
        assert_eq!(
            parsed(&["serve", "api", "--data-dir", "/srv/ygg/data"]).data_dir,
            Some(PathBuf::from("/srv/ygg/data"))
        );
        assert!(
            RootCli::try_parse_from([
                "aster_yggdrasil",
                "--config-file",
                "a.toml",
                "--data-dir",
                "/srv",
            ])
            .is_err(),
            "config-file and data-dir are mutually exclusive"
        );
    }

    #[test]
    fn invalid_or_conflicting_roles_are_rejected() {
        for args in [
            vec!["serve", "follower"],
            vec!["serve", "API"],
            vec!["serve", "api", "worker"],
            vec!["serve", "--role", "worker"],
            vec!["serve", ""],
        ] {
            let full = std::iter::once("aster_yggdrasil").chain(args.iter().copied());
            assert!(
                RootCli::try_parse_from(full).is_err(),
                "args should be rejected: {args:?}"
            );
        }
    }

    #[test]
    fn role_values_survive_roundtrip_through_value_parser() {
        for role in [ServeRole::All, ServeRole::Api, ServeRole::Worker] {
            assert_eq!(parsed_role(&["serve", role.as_str()]), role);
        }
    }

    #[test]
    fn version_subcommand_parses_with_and_without_json_flag() {
        let plain = parsed(&["version"]);
        assert!(matches!(
            plain.command,
            Some(RootCommand::Version { json: false })
        ));
        let json = parsed(&["version", "--json"]);
        assert!(matches!(
            json.command,
            Some(RootCommand::Version { json: true })
        ));
        assert!(
            RootCli::try_parse_from(["aster_yggdrasil", "version", "extra"]).is_err(),
            "version must not accept positional arguments"
        );
    }
}

#[cfg(all(test, feature = "operations-cli"))]
mod hash_password_cli_tests {
    use super::*;

    #[test]
    fn hash_password_subcommand_parses_without_extra_arguments() {
        let cli = parsed(&["hash-password"]);
        assert!(matches!(cli.command, Some(RootCommand::HashPassword)));
        assert!(
            RootCli::try_parse_from(["aster_yggdrasil", "hash-password", "extra"]).is_err(),
            "hash-password must not accept positional arguments"
        );
    }
}
