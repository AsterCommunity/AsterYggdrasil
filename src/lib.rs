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

pub mod alloc;
pub mod api;
pub mod cache;
pub mod config;
pub mod db;
pub mod entities;
pub mod errors;
pub mod external_auth;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod object_storage;
pub mod runtime;
pub mod services;
pub mod types;
pub mod utils;
