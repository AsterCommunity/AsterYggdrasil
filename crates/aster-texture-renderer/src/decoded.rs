//! Decoded Minecraft skin texture type.

use image::{ImageFormat, RgbaImage};

use crate::{engine::validate_skin_dimensions, error::RenderError};

/// Decoded and validated Minecraft skin texture.
///
/// Byte-oriented APIs are convenient, but they decode PNG data on every render.
/// Batch renderers can decode once into `DecodedSkin`, then reuse it with
/// workspace rendering APIs to avoid repeated decoder and source-image
/// allocations.
#[derive(Debug, Clone)]
pub struct DecodedSkin {
    image: RgbaImage,
}

impl DecodedSkin {
    /// Decode PNG bytes and validate that the dimensions match a Minecraft skin
    /// layout: 64x64, 64x32 legacy, or an HD multiple of those layouts.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, RenderError> {
        let image = decode_skin_png(bytes)?;
        validate_skin_dimensions(&image)?;
        Ok(Self { image })
    }

    /// Borrow the decoded RGBA texture.
    pub fn image(&self) -> &RgbaImage {
        &self.image
    }

    /// Return decoded texture dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        self.image.dimensions()
    }

    /// Whether the skin uses the legacy 64x32 layout.
    pub fn is_legacy_layout(&self) -> bool {
        self.image.height() * 2 == self.image.width()
    }
}

pub(crate) fn decode_skin_png(bytes: &[u8]) -> Result<RgbaImage, RenderError> {
    image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(RenderError::InvalidPng)
        .map(|image| image.to_rgba8())
}
