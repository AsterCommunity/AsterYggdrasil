//! Root command parsing and operational command dispatch.

use super::CommandOutcome;
use super::app::{RootCli, RootCommand};
use std::io;

#[cfg(feature = "operations-cli")]
use super::commands::hash_password;
#[cfg(feature = "operations-cli")]
use super::common::error::CliError;

/// Parses the resolved command into server startup or a finished exit status.
pub(super) async fn execute(cli: RootCli) -> io::Result<CommandOutcome> {
    let serve_role = match cli.command {
        #[cfg(feature = "operations-cli")]
        Some(RootCommand::HashPassword) => {
            return Ok(match hash_password::execute().await {
                Ok(hash) => {
                    println!("{hash}");
                    CommandOutcome::Exit(0)
                }
                Err(CliError::Canceled) => {
                    eprintln!("\nCanceled.");
                    CommandOutcome::Exit(130)
                }
                Err(CliError::Failed(error)) => {
                    eprintln!("{error}");
                    CommandOutcome::Exit(1)
                }
            });
        }
        Some(RootCommand::Version { json }) => {
            println!("{}", super::commands::version::render_build_info(json));
            return Ok(CommandOutcome::Exit(0));
        }
        Some(RootCommand::Serve { role }) => role,
        None => crate::runtime::role::ServeRole::All,
    };

    let location = crate::config::loader::ConfigLocation::resolve(
        cli.config_file.as_deref(),
        cli.data_dir.as_deref(),
    )
    .map_err(io::Error::other)?;

    Ok(CommandOutcome::Serve {
        role: serve_role,
        location,
    })
}
