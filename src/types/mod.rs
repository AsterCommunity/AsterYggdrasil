//! Shared domain types.
//!
//! Concrete submodules make type ownership explicit. The root facade preserves
//! the stable `crate::types::{...}` compatibility entry for cross-boundary
//! imports.

pub mod audit;
pub mod auth;
pub mod config;
pub mod external_auth;
mod facade;
pub mod mail;
pub mod passkey;
pub mod task;
pub mod user;
pub mod yggdrasil;

pub use facade::*;
