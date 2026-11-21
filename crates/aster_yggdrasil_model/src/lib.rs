//! AsterYggdrasil shared product models: SeaORM entities and domain types.
//!
//! This crate has no business logic and never depends on the application crate;
//! the binary depends on it, never the other way around (AsterDrive model crate
//! layout).

pub mod entities;
pub mod types;
