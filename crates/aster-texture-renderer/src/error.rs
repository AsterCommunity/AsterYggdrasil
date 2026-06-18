use std::{error::Error, fmt};

/// Errors returned by texture preview rendering and encoding.
#[derive(Debug)]
pub enum RenderError {
    /// The provided input bytes could not be decoded as an image supported by `image`.
    InvalidPng(image::ImageError),
    /// The rendered preview could not be encoded to the requested output format.
    EncodeImage(image::ImageError),
    /// The decoded skin dimensions do not match Minecraft skin layout rules.
    InvalidDimensions { width: u32, height: u32 },
    /// The requested output canvas is empty or overflows during supersampling.
    InvalidOutputSize { width: u32, height: u32 },
    /// The 2D engine padding leaves no drawable area.
    InvalidPadding { padding: u32 },
    /// The 3D engine supersampling factor is outside the supported range.
    InvalidSupersampling { supersampling: u8 },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPng(error) => write!(formatter, "invalid PNG skin texture: {error}"),
            Self::EncodeImage(error) => {
                write!(formatter, "failed to encode preview image: {error}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "unsupported Minecraft skin dimensions: {width}x{height}"
                )
            }
            Self::InvalidOutputSize { width, height } => {
                write!(formatter, "invalid preview output size: {width}x{height}")
            }
            Self::InvalidPadding { padding } => {
                write!(formatter, "invalid preview padding: {padding}")
            }
            Self::InvalidSupersampling { supersampling } => {
                write!(formatter, "invalid supersampling factor: {supersampling}")
            }
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPng(error) | Self::EncodeImage(error) => Some(error),
            Self::InvalidDimensions { .. }
            | Self::InvalidOutputSize { .. }
            | Self::InvalidPadding { .. }
            | Self::InvalidSupersampling { .. } => None,
        }
    }
}
