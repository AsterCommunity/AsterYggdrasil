//! Shared domain types.
//!
//! Concrete submodules make type ownership explicit. Callers import domain
//! types from their owning modules, while shared infrastructure types come
//! directly from the owning Forge crates.

pub mod audit;
pub mod auth;
pub mod external_auth;
pub mod passkey;
pub mod task;
pub mod user;
pub mod yggdrasil;
