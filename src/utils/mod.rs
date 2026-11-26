//! General-purpose utilities.

pub mod paths;

/// User-Agent string sent with all outbound HTTP requests, e.g. for webhook or mail delivery.
pub const OUTBOUND_HTTP_USER_AGENT: &str = concat!("AsterYggdrasil/", env!("CARGO_PKG_VERSION"));
