//! AsterYggdrasil shared product models: SeaORM entities and domain types.
//!
//! This crate has no business logic and never depends on the application crate;
//! the binary depends on it, never the other way around (AsterDrive model crate
//! layout).

#![forbid(unsafe_code)]
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

pub mod entities;
pub mod types;
