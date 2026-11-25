//! Root command parsing and operational command dispatch.

use super::CommandOutcome;
use super::app::{RootCli, RootCommand};
use std::io;

/// Parses the resolved command into server startup or a finished exit status.
pub(super) async fn execute(cli: RootCli) -> io::Result<CommandOutcome> {
    if let Some(RootCommand::HashPassword) = cli.command {
        return Ok(match super::commands::hash_password::execute().await {
            Ok(hash) => {
                println!("{hash}");
                CommandOutcome::Exit(0)
            }
            Err(super::common::error::CliError::Canceled) => {
                eprintln!("\nCanceled.");
                CommandOutcome::Exit(130)
            }
            Err(super::common::error::CliError::Failed(error)) => {
                eprintln!("{error}");
                CommandOutcome::Exit(1)
            }
        });
    }

    let role = match cli.command {
        Some(RootCommand::Serve { role }) => role,
        _ => crate::runtime::role::ServeRole::All,
    };
    let location = crate::config::loader::ConfigLocation::resolve(
        cli.config_file.as_deref(),
        cli.data_dir.as_deref(),
    )
    .map_err(io::Error::other)?;

    Ok(CommandOutcome::Serve { role, location })
}
