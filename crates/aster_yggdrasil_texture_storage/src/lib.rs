//! Project-agnostic texture object-storage contract crate.
//!
//! This crate owns the storage contract for Minecraft texture blobs:
//!
//! - [`TextureStore`]: the backend-neutral store trait (put, stream, delete,
//!   exists, metadata, key listing) with normalized `textures/aa/bb.png`-style
//!   keys.
//! - [`TextureStoreError`]: the structured error state machine store
//!   implementations report to hosts.
//! - [`TextureStorageConfig`] / [`S3TextureStorageConfig`]: the
//!   `[object_storage]` configuration section types.
//!
//! The crate is intentionally free of database, HTTP-server, cache, and
//! application concerns: concrete filesystem and S3-compatible
//! implementations, backend selection, and application error mapping remain
//! product infrastructure in the application crate.
//!
//! The shape mirrors `aster_drive_storage` (AsterDrive): contracts and
//! descriptors live here, SDK-backed drivers live next to the application.

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

mod config;
mod error;
mod store;

pub use config::{S3TextureStorageConfig, TextureStorageConfig};
pub use error::TextureStoreError;
pub use store::{ObjectBlobMetadata, TextureStore, TextureStoreResult};
