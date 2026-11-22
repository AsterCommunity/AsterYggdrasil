/// Errors returned by texture preview rendering and encoding.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The provided input bytes could not be decoded as an image supported by `image`.
    #[error("invalid PNG skin texture: {0}")]
    InvalidPng(#[source] image::ImageError),
    /// The rendered preview could not be encoded to the requested output format.
    #[error("failed to encode preview image: {0}")]
    EncodeImage(#[source] image::ImageError),
    /// The decoded skin dimensions do not match Minecraft skin layout rules.
    #[error("unsupported Minecraft skin dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
    /// The requested output canvas is empty or overflows during supersampling.
    #[error("invalid preview output size: {width}x{height}")]
    InvalidOutputSize { width: u32, height: u32 },
    /// The 2D engine padding leaves no drawable area.
    #[error("invalid preview padding: {padding}")]
    InvalidPadding { padding: u32 },
    /// The 3D engine supersampling factor is outside the supported range.
    #[error("invalid supersampling factor: {supersampling}")]
    InvalidSupersampling { supersampling: u8 },
}
