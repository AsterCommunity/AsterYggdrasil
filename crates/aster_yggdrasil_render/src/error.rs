/// Errors returned by texture preview rendering and encoding.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The provided input bytes could not be decoded as an image supported by `image`.
    #[error("invalid PNG skin texture: {0}")]
    InvalidPng(
        /// Underlying decoding failure from the `image` crate.
        #[source]
        image::ImageError,
    ),
    /// The rendered preview could not be encoded to the requested output format.
    #[error("failed to encode preview image: {0}")]
    EncodeImage(
        /// Underlying encoding failure from the `image` crate.
        #[source]
        image::ImageError,
    ),
    /// The decoded skin dimensions do not match Minecraft skin layout rules.
    #[error("unsupported Minecraft skin dimensions: {width}x{height}")]
    InvalidDimensions {
        /// Decoded texture width in pixels.
        width: u32,
        /// Decoded texture height in pixels.
        height: u32,
    },
    /// The requested output canvas is empty or overflows during supersampling.
    #[error("invalid preview output size: {width}x{height}")]
    InvalidOutputSize {
        /// Requested output width in pixels.
        width: u32,
        /// Requested output height in pixels.
        height: u32,
    },
    /// The 2D engine padding leaves no drawable area.
    #[error("invalid preview padding: {padding}")]
    InvalidPadding {
        /// Padding applied to each edge, in pixels.
        padding: u32,
    },
    /// The 3D engine supersampling factor is outside the supported range.
    #[error("invalid supersampling factor: {supersampling}")]
    InvalidSupersampling {
        /// Rejected supersampling factor.
        supersampling: u8,
    },
}
