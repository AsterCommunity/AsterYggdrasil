//! Command-line process boundary for the AsterYggdrasil executable.
//!
//! The CLI owns argument parsing and operational subcommands. Running without a
//! subcommand (or with [`CommandOutcome::Serve`]) hands control back to the
//! process entry, which keeps server startup in the product entrypoint.

mod app;
mod commands;
mod common;
mod dispatch;

use common::help;

/// CLI result consumed by the process entry point.
pub enum CommandOutcome {
    Serve {
        role: crate::runtime::role::ServeRole,
        location: crate::config::loader::ConfigLocation,
    },
    Exit(i32),
}

/// Parses arguments and runs an operational command or selects server startup.
pub async fn run() -> std::io::Result<CommandOutcome> {
    use clap::{CommandFactory, FromArgMatches};
    let command = help::configure_cli_help(app::RootCli::command());
    let cli = app::RootCli::from_arg_matches(&command.clone().get_matches())
        .unwrap_or_else(|error| error.exit());
    dispatch::execute(cli).await
}
