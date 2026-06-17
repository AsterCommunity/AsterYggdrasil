use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::MinecraftTextureType;
use image::{GenericImageView, ImageDecoder, ImageEncoder};
use sha2::Digest;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TextureProcessingResult {
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub hash: String,
}

pub async fn process_texture_file(
    source_path: &Path,
    output_path: &Path,
    texture_type: MinecraftTextureType,
    max_pixels: u64,
) -> Result<TextureProcessingResult> {
    let source_path = source_path.to_path_buf();
    let output_path = output_path.to_path_buf();
    tracing::debug!(
        texture_type = ?texture_type,
        max_pixels,
        "processing texture file"
    );
    tokio::task::spawn_blocking(move || {
        let source = File::open(source_path)
            .map_aster_err_ctx("open uploaded texture", AsterError::validation_error)?;
        let output = File::create(output_path)
            .map_aster_err_ctx("create processed texture", AsterError::internal_error)?;
        sanitize_png_texture(source, output, texture_type, max_pixels)
    })
    .await
    .map_aster_err_ctx("process texture task", AsterError::internal_error)?
}

pub fn sanitize_png_texture<R, W>(
    reader: R,
    writer: W,
    texture_type: MinecraftTextureType,
    max_pixels: u64,
) -> Result<TextureProcessingResult>
where
    R: Read + Seek,
    W: Write,
{
    tracing::debug!(
        texture_type = ?texture_type,
        max_pixels,
        "sanitizing png texture"
    );
    let mut reader = BufReader::new(reader);
    let decoder = image::codecs::png::PngDecoder::new(&mut reader)
        .map_aster_err_ctx("decode png header", AsterError::validation_error)?;
    let (width, height) = decoder.dimensions();
    tracing::debug!(
        width,
        height,
        texture_type = ?texture_type,
        "decoded png texture header"
    );
    validate_texture_pixel_limit(width, height, max_pixels)?;
    super::validate_texture_dimensions(texture_type, width, height)?;

    reader
        .seek(SeekFrom::Start(0))
        .map_aster_err_ctx("rewind png texture", AsterError::validation_error)?;
    let image = image::ImageReader::with_format(reader, image::ImageFormat::Png)
        .decode()
        .map_aster_err_ctx("decode png texture", AsterError::validation_error)?;
    let (decoded_width, decoded_height) = image.dimensions();
    tracing::debug!(
        width = decoded_width,
        height = decoded_height,
        texture_type = ?texture_type,
        "decoded png texture image"
    );
    validate_texture_pixel_limit(decoded_width, decoded_height, max_pixels)?;
    super::validate_texture_dimensions(texture_type, decoded_width, decoded_height)?;

    let rgba = image.to_rgba8();
    let (rgba, output_width, output_height) =
        normalize_texture_canvas(texture_type, rgba, decoded_width, decoded_height);
    let mut hasher = sha2::Sha256::new();
    let mut hashing_writer = HashingWriter {
        inner: BufWriter::new(writer),
        hasher: &mut hasher,
        written: 0,
    };
    let encoder = image::codecs::png::PngEncoder::new(&mut hashing_writer);
    encoder
        .write_image(
            rgba.as_raw(),
            output_width,
            output_height,
            image::ExtendedColorType::Rgba8,
        )
        .map_aster_err_ctx("encode sanitized png texture", AsterError::internal_error)?;
    hashing_writer
        .inner
        .flush()
        .map_aster_err_ctx("flush sanitized png texture", AsterError::internal_error)?;
    let file_size = hashing_writer.written;
    let hash = hex::encode(hasher.finalize());
    tracing::debug!(
        width = output_width,
        height = output_height,
        file_size,
        hash = %hash,
        texture_type = ?texture_type,
        "sanitized png texture"
    );

    Ok(TextureProcessingResult {
        width: output_width,
        height: output_height,
        file_size,
        hash,
    })
}

fn validate_texture_pixel_limit(width: u32, height: u32, max_pixels: u64) -> Result<()> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| AsterError::validation_error("texture dimensions are too large"))?;
    // authlib-injector explicitly requires checking PNG dimensions before
    // reading the full image; this prevents small PNG bombs from allocating
    // huge RGBA buffers during decode.
    if pixels > max_pixels {
        tracing::debug!(
            width,
            height,
            pixels,
            max_pixels,
            "texture rejected because pixel limit was exceeded"
        );
        return Err(AsterError::validation_error(format!(
            "texture dimensions exceed {max_pixels} pixels: {width}x{height}"
        )));
    }
    Ok(())
}

fn normalize_texture_canvas(
    texture_type: MinecraftTextureType,
    rgba: image::RgbaImage,
    width: u32,
    height: u32,
) -> (image::RgbaImage, u32, u32) {
    if texture_type != MinecraftTextureType::Cape
        || !super::is_multiple_texture_size(width, height, 22, 17)
    {
        tracing::debug!(
            texture_type = ?texture_type,
            width,
            height,
            "texture canvas normalization not required"
        );
        return (rgba, width, height);
    }

    let scale = width / 22;
    let output_width = 64 * scale;
    let output_height = 32 * scale;
    let mut padded =
        image::RgbaImage::from_pixel(output_width, output_height, image::Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            padded.put_pixel(x, y, *rgba.get_pixel(x, y));
        }
    }
    tracing::debug!(
        width,
        height,
        output_width,
        output_height,
        "normalized legacy cape texture canvas"
    );
    (padded, output_width, output_height)
}

struct HashingWriter<'a, W: Write> {
    inner: BufWriter<W>,
    hasher: &'a mut sha2::Sha256,
    written: u64,
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.written = self.written.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn png(width: u32, height: u32) -> Vec<u8> {
        png_with_color(width, height, image::Rgba([0, 0, 0, 0]))
    }

    fn png_with_color(width: u32, height: u32, color: image::Rgba<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let image = image::RgbaImage::from_pixel(width, height, color);
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn sanitize_png_texture_accepts_skin_dimensions_with_read_seek() {
        let mut output = Vec::new();
        let result = sanitize_png_texture(
            Cursor::new(png(64, 64)),
            Cursor::new(&mut output),
            MinecraftTextureType::Skin,
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
        )
        .unwrap();

        assert_eq!((result.width, result.height), (64, 64));
        assert_eq!(result.hash.len(), 64);
        assert!(!output.is_empty());
    }

    #[test]
    fn sanitize_png_texture_accepts_dimensions_at_pixel_limit() {
        let mut output = Vec::new();
        let result = sanitize_png_texture(
            Cursor::new(png(64, 64)),
            Cursor::new(&mut output),
            MinecraftTextureType::Skin,
            64 * 64,
        )
        .unwrap();

        assert_eq!((result.width, result.height), (64, 64));
        assert!(!output.is_empty());
    }

    #[test]
    fn sanitize_png_texture_pads_legacy_cape_to_standard_canvas() {
        let mut output = Vec::new();
        let result = sanitize_png_texture(
            Cursor::new(png_with_color(22, 17, image::Rgba([8, 9, 10, 255]))),
            Cursor::new(&mut output),
            MinecraftTextureType::Cape,
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
        )
        .unwrap();

        assert_eq!((result.width, result.height), (64, 32));
        let decoded = image::load_from_memory(&output).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (64, 32));
        assert_eq!(*decoded.get_pixel(0, 0), image::Rgba([8, 9, 10, 255]));
        assert_eq!(*decoded.get_pixel(21, 16), image::Rgba([8, 9, 10, 255]));
        assert_eq!(*decoded.get_pixel(22, 17), image::Rgba([0, 0, 0, 0]));
        assert_eq!(*decoded.get_pixel(63, 31), image::Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn sanitize_png_texture_rejects_invalid_dimensions() {
        let mut output = Vec::new();
        let error = sanitize_png_texture(
            Cursor::new(png(63, 64)),
            Cursor::new(&mut output),
            MinecraftTextureType::Skin,
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
        )
        .unwrap_err();

        assert!(error.message().contains("invalid skin texture dimensions"));
    }

    #[test]
    fn sanitize_png_texture_enforces_texture_type_specific_dimensions() {
        let mut output = Vec::new();
        let skin_error = sanitize_png_texture(
            Cursor::new(png(22, 17)),
            Cursor::new(&mut output),
            MinecraftTextureType::Skin,
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
        )
        .unwrap_err();
        assert!(
            skin_error
                .message()
                .contains("invalid skin texture dimensions")
        );

        let mut output = Vec::new();
        let cape_error = sanitize_png_texture(
            Cursor::new(png(64, 64)),
            Cursor::new(&mut output),
            MinecraftTextureType::Cape,
            crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
        )
        .unwrap_err();
        assert!(
            cape_error
                .message()
                .contains("invalid cape texture dimensions")
        );
    }

    #[test]
    fn sanitize_png_texture_rejects_oversized_header_before_full_decode() {
        let mut output = Vec::new();
        let error = sanitize_png_texture(
            Cursor::new(png(128, 128)),
            Cursor::new(&mut output),
            MinecraftTextureType::Skin,
            128 * 128 - 1,
        )
        .unwrap_err();

        assert!(error.message().contains("texture dimensions exceed"));
        assert!(output.is_empty());
    }
}
