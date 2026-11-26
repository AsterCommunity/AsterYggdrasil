//! Standalone CLI for running AsterYggdrasil database migrations.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::todo
    )
)]

use aster_yggdrasil_migration::Migrator;
use sea_orm_migration::cli;

#[tokio::main]
async fn main() {
    cli::run_cli(Migrator).await;
}
