//! Software renderer for Minecraft texture previews.
//!
//! This crate intentionally avoids GPU and windowing dependencies. It provides
//! small, deterministic preview engines that can run in a backend process:
//!
//! - `Skin3d` projects Minecraft skin cuboids into a static dual-view preview.
//! - `Skin2d` renders the raw skin texture as a scaled, centered 2D preview.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::unreachable, clippy::expect_used)
)]

mod decoded;
mod engine;
mod error;
mod geometry;
mod options;
mod render;
mod skin;
mod workspace;

pub use decoded::DecodedSkin;
pub use error::RenderError;
pub use options::{
    OutputFormat, PreviewEngine, Skin2dPreviewOptions, SkinModel, SkinPreviewOptions,
    SkinPreviewProfile, TexturePreviewOptions,
};
pub use render::{
    render_decoded_preview, render_decoded_preview_with_workspace, render_decoded_skin_2d_preview,
    render_decoded_skin_2d_preview_with_workspace, render_decoded_skin_preview,
    render_decoded_skin_preview_with_workspace, render_preview, render_preview_bytes,
    render_preview_with_workspace, render_skin_2d_preview, render_skin_2d_preview_bytes,
    render_skin_2d_preview_with_workspace, render_skin_preview, render_skin_preview_bytes,
    render_skin_preview_with_workspace,
};
pub use workspace::RenderWorkspace;
