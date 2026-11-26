//! Product entrypoint assembly for the HTTP service.
//!
//! This module keeps the binary `main.rs` thin while preserving product-owned
//! wiring: configuration loading, Actix application construction, background
//! task spawning, and runtime component registration. Forge supplies the shared
//! process mechanics such as panic hooks, termination signal handling,
//! dependency-aware shutdown ordering, and shutdown report logging.

use std::io;

use crate::runtime::role::ServeRole;

/// Runs the AsterYggdrasil service with the given process role and
/// configuration location until it receives a shutdown signal.
pub async fn run(
    role: ServeRole,
    location: &crate::config::loader::ConfigLocation,
) -> io::Result<()> {
    let bootstrap = crate::runtime::bootstrap::bootstrap(location)
        .await
        .map_err(|error| io::Error::other(error.to_string()))?;
    let metrics = bootstrap.state.metrics().clone();
    let audit_writer = bootstrap.state.audit_writer().clone();
    let result = crate::runtime::assembly::run(bootstrap.state, role).await;
    crate::runtime::shutdown::finish_runtime(result, &audit_writer, &metrics).await
}
