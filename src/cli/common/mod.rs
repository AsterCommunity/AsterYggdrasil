//! Shared CLI helpers: error mapping, help layout, and — under
//! `operations-cli` — bounded input, output envelopes, and instance context
//! resolution.

#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod error;
pub(in crate::cli) mod help;
#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod input;
#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod output;
#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod context;
