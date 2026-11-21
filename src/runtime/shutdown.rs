//! Runtime shutdown tail.
//!
//! Owns the ordered drain that runs after [`crate::runtime::assembly::run`]
//! returns: the metrics provider closes last so in-flight exports finish, and
//! the original runtime error always wins over shutdown failures. When Forge
//! audit recovery lands upstream (AsterCommunity/AsterForge#74), its
//! `finish_audit` call belongs here, before the metrics drain.

use std::io;
use std::time::Duration;

/// Drains the metrics provider and returns the runtime result unchanged unless
/// the runtime succeeded and the drain failed.
pub async fn finish_runtime(
    result: io::Result<()>,
    metrics: &aster_forge_metrics::SharedMetricsRecorder,
) -> io::Result<()> {
    if let Some(provider) = metrics.provider() {
        let shutdown = provider.shutdown(Duration::from_secs(10)).await;
        if let Err(error) = &shutdown {
            tracing::warn!(%error, "metrics shutdown failed");
        }
        if result.is_ok() {
            shutdown.map_err(io::Error::other)?;
        }
    }
    result
}
