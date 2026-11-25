//! AsterYggdrasil backend crate.
#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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

pub use aster_yggdrasil_model::{entities, types};

pub mod api;
pub mod cli;
pub mod config;
pub mod db;
pub mod errors;
pub mod external_auth;
pub mod metrics;
pub mod runtime;
pub mod services;
pub mod texture_storage;
pub mod utils;
