//! Reusable render buffers for batch preview generation.

use image::{ImageBuffer, Rgba, RgbaImage};

/// Reusable buffers used by workspace rendering APIs.
///
/// The default rendering functions return owned images and allocate their output
/// every call. Backend jobs that render many previews can instead keep one
/// `RenderWorkspace` per worker and call `*_with_workspace` APIs to reuse the
/// final output buffer and the 3D supersampling scratch buffer.
#[derive(Debug, Default)]
pub struct RenderWorkspace {
    pub(crate) output: RgbaImage,
    pub(crate) scratch: RgbaImage,
}

impl RenderWorkspace {
    /// Create an empty workspace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current reusable output buffer capacity in bytes.
    pub fn output_capacity_bytes(&self) -> usize {
        self.output.as_raw().capacity()
    }

    /// Current reusable scratch buffer capacity in bytes.
    pub fn scratch_capacity_bytes(&self) -> usize {
        self.scratch.as_raw().capacity()
    }

    /// Total reusable image buffer capacity in bytes.
    pub fn total_capacity_bytes(&self) -> usize {
        self.output_capacity_bytes() + self.scratch_capacity_bytes()
    }
}

pub(crate) fn prepare_image(image: &mut RgbaImage, width: u32, height: u32, color: Rgba<u8>) {
    if image.dimensions() != (width, height) {
        *image = ImageBuffer::from_pixel(width, height, color);
        return;
    }

    for pixel in image.pixels_mut() {
        *pixel = color;
    }
}
