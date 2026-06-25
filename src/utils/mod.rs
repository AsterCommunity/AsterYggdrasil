//! General-purpose utilities.

pub mod paths;

pub const OUTBOUND_HTTP_USER_AGENT: &str = concat!("AsterYggdrasil/", env!("CARGO_PKG_VERSION"));
