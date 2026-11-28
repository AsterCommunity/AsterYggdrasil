//! Root command parsing and operational command dispatch.

use super::CommandOutcome;
use super::app::{RootCli, RootCommand};
use std::io;

fn hash_password_outcome(
    result: super::common::error::CliResult<String>,
) -> (CommandOutcome, Option<String>, Option<String>) {
    match result {
        Ok(hash) => (CommandOutcome::Exit(0), Some(hash), None),
        #[cfg(feature = "operator-cli")]
        Err(super::common::error::CliError::Canceled) => (
            CommandOutcome::Exit(130),
            None,
            Some("\nCanceled.".to_owned()),
        ),
        Err(super::common::error::CliError::Failed(error)) => {
            (CommandOutcome::Exit(1), None, Some(error.to_string()))
        }
    }
}

fn resolve_server_startup(
    cli: &RootCli,
) -> io::Result<(
    crate::runtime::role::ServeRole,
    crate::config::loader::ConfigLocation,
)> {
    let role = match cli.command {
        Some(RootCommand::Serve { role }) => role,
        _ => crate::runtime::role::ServeRole::All,
    };
    let location = crate::config::loader::ConfigLocation::resolve(
        cli.config_file.as_deref(),
        cli.data_dir.as_deref(),
    )
    .map_err(io::Error::other)?;
    Ok((role, location))
}

/// Parses the resolved command into server startup or a finished exit status.
pub(super) async fn execute(cli: RootCli) -> io::Result<CommandOutcome> {
    match cli.command {
        Some(RootCommand::Version { json }) => {
            let info =
                super::commands::build_info::render_build_info(json).map_err(io::Error::other)?;
            use std::io::Write;
            writeln!(io::stdout().lock(), "{info}")?;
            return Ok(CommandOutcome::Exit(0));
        }
        Some(RootCommand::HashPassword) => {
            let (outcome, stdout, stderr) =
                hash_password_outcome(super::commands::hash_password::execute().await);
            if let Some(stdout) = stdout {
                println!("{stdout}");
            }
            if let Some(stderr) = stderr {
                eprintln!("{stderr}");
            }
            return Ok(outcome);
        }
        Some(RootCommand::Serve { .. }) | None => {}
    }
    let (role, location) = resolve_server_startup(&cli)?;
    Ok(CommandOutcome::Serve { role, location })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn hash_password_results_map_to_exit_status_and_non_leaking_output() {
        let result = hash_password_outcome(Ok("secret-hash".into()));
        assert_eq!(exit_code(result.0), Some(0));
        assert_eq!(result.1, Some("secret-hash".into()));
        assert_eq!(result.2, None);

        let error = crate::errors::AsterError::validation_error("invalid input");
        let result = hash_password_outcome(Err(error.into()));
        assert_eq!(exit_code(result.0), Some(1));
        assert_eq!(result.1, None);
        assert!(
            result
                .2
                .as_deref()
                .is_some_and(|text| text.contains("invalid input"))
        );
        assert!(!result.2.as_deref().unwrap_or_default().contains("secret"));

        #[cfg(feature = "operator-cli")]
        {
            let result =
                hash_password_outcome(Err(super::super::common::error::CliError::Canceled));
            assert_eq!(exit_code(result.0), Some(130));
            assert_eq!(result.1, None);
            assert_eq!(result.2.as_deref(), Some("\nCanceled."));
        }
    }

    #[test]
    fn server_role_and_implicit_startup_preserve_the_resolved_location() {
        for (args, expected) in [
            (
                vec!["aster_yggdrasil", "serve", "api"],
                crate::runtime::role::ServeRole::Api,
            ),
            (
                vec!["aster_yggdrasil", "serve", "worker"],
                crate::runtime::role::ServeRole::Worker,
            ),
        ] {
            let cli = RootCli::try_parse_from(args).unwrap();
            let (role, location) = resolve_server_startup(&cli).unwrap();
            assert_eq!(role, expected);
            assert!(location.path().ends_with("data/config.toml"));
        }

        let cli = RootCli::try_parse_from(["aster_yggdrasil"]).unwrap();
        let (role, location) = resolve_server_startup(&cli).unwrap();
        assert_eq!(role, crate::runtime::role::ServeRole::All);
        assert!(location.path().ends_with("data/config.toml"));
        assert_eq!(
            exit_code(CommandOutcome::Serve {
                role: crate::runtime::role::ServeRole::All,
                location,
            }),
            None
        );
    }

    fn exit_code(outcome: CommandOutcome) -> Option<i32> {
        match outcome {
            CommandOutcome::Exit(code) => Some(code),
            CommandOutcome::Serve { .. } => None,
        }
    }
}
