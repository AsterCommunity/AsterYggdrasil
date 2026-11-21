//! Preview engine implementations.
//!
//! Engines are split by rendering strategy instead of by output format. The 3D
//! engine projects Minecraft player cuboids, while the 2D engine draws the raw
//! texture layout. Both engines share input validation and image encoding from
//! `render.rs`.

pub(crate) mod skin2d;
pub(crate) mod skin3d;

use image::RgbaImage;

use crate::error::RenderError;

pub(crate) fn validate_skin_dimensions(texture: &RgbaImage) -> Result<(), RenderError> {
    let (width, height) = texture.dimensions();
    let ratio_valid = width % 64 == 0 && (height == width || height * 2 == width);
    if width == 0 || height == 0 || !ratio_valid {
        return Err(RenderError::InvalidDimensions { width, height });
    }
    Ok(())
}
