//! Product entrypoint assembly for the HTTP service.
//!
//! This module keeps the binary `main.rs` thin while preserving product-owned
//! wiring: configuration loading, Actix application construction, background
//! task spawning, and runtime component registration. Forge supplies the shared
//! process mechanics such as panic hooks, termination signal handling,
//! dependency-aware shutdown ordering, and shutdown report logging.

use std::io;

/// Runs the AsterYggdrasil HTTP service until it receives a shutdown signal.
pub async fn run() -> io::Result<()> {
    let bootstrap = crate::runtime::bootstrap::bootstrap()
        .await
        .map_err(to_io_error)?;
    crate::runtime::assembly::run(bootstrap.prepared).await
}

fn to_io_error(error: impl ToString) -> io::Error {
    io::Error::other(error.to_string())
}
