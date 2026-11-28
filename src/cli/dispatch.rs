//! Root command parsing and operational command dispatch.

use super::CommandOutcome;
use super::app::{RootCli, RootCommand};
use std::io;

/// Parses the resolved command into server startup or a finished exit status.
pub(super) async fn execute(cli: RootCli) -> io::Result<CommandOutcome> {
    let role = match cli.command {
        Some(RootCommand::Version { json }) => {
            let info =
                super::commands::build_info::render_build_info(json).map_err(io::Error::other)?;
            use std::io::Write;
            writeln!(io::stdout().lock(), "{info}")?;
            return Ok(CommandOutcome::Exit(0));
        }
        Some(RootCommand::HashPassword) => {
            return Ok(match super::commands::hash_password::execute().await {
                Ok(hash) => {
                    println!("{hash}");
                    CommandOutcome::Exit(0)
                }
                #[cfg(feature = "operator-cli")]
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
        Some(RootCommand::Serve { role }) => role,
        None => crate::runtime::role::ServeRole::All,
    };
    let location = crate::config::loader::ConfigLocation::resolve(
        cli.config_file.as_deref(),
        cli.data_dir.as_deref(),
    )
    .map_err(io::Error::other)?;

    Ok(CommandOutcome::Serve { role, location })
}
