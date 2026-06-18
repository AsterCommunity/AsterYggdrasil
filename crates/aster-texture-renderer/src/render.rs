//! Public rendering entry points and shared image helpers.

use std::io::Cursor;

use image::{DynamicImage, Rgba, RgbaImage};

use crate::{
    decoded::DecodedSkin,
    engine::{skin2d, skin3d},
    error::RenderError,
    geometry::EPSILON,
    options::{
        OutputFormat, Skin2dPreviewOptions, SkinModel, SkinPreviewOptions, TexturePreviewOptions,
    },
    workspace::RenderWorkspace,
};

/// Render a preview with the selected engine and encode it as PNG or WebP bytes.
///
/// `model` selects Steve or Alex arm width for both 3D and 2D player previews.
pub fn render_preview_bytes(
    skin_png: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
    format: OutputFormat,
) -> Result<Vec<u8>, RenderError> {
    let image = render_preview(skin_png, model, options)?;
    encode_image(image, format)
}

/// Render a preview with the selected engine and return an RGBA image.
///
/// Use this entry point when callers want to choose the engine dynamically.
pub fn render_preview(
    skin_png: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
) -> Result<RgbaImage, RenderError> {
    match options {
        TexturePreviewOptions::Skin3d(options) => render_skin_preview(skin_png, model, options),
        TexturePreviewOptions::Skin2d(options) => render_skin_2d_preview(skin_png, model, options),
    }
}

/// Render a preview from an already decoded skin texture.
pub fn render_decoded_preview(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &TexturePreviewOptions,
) -> Result<RgbaImage, RenderError> {
    match options {
        TexturePreviewOptions::Skin3d(options) => render_decoded_skin_preview(skin, model, options),
        TexturePreviewOptions::Skin2d(options) => {
            render_decoded_skin_2d_preview(skin, model, options)
        }
    }
}

/// Render a preview into reusable workspace buffers and return the final image.
///
/// The returned image reference is valid until the next render call using the
/// same workspace. Use this for batch jobs where repeatedly allocating 430x430
/// and supersampled buffers would otherwise dominate memory churn.
pub fn render_preview_with_workspace<'a>(
    skin_png: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    match options {
        TexturePreviewOptions::Skin3d(options) => {
            render_skin_preview_with_workspace(skin_png, model, options, workspace)
        }
        TexturePreviewOptions::Skin2d(options) => {
            render_skin_2d_preview_with_workspace(skin_png, model, options, workspace)
        }
    }
}

/// Render an already decoded skin into reusable workspace buffers.
pub fn render_decoded_preview_with_workspace<'a>(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &TexturePreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    match options {
        TexturePreviewOptions::Skin3d(options) => {
            render_decoded_skin_preview_with_workspace(skin, model, options, workspace)
        }
        TexturePreviewOptions::Skin2d(options) => {
            render_decoded_skin_2d_preview_with_workspace(skin, model, options, workspace)
        }
    }
}

/// Render a projected 3D skin preview and encode it as PNG or WebP bytes.
pub fn render_skin_preview_bytes(
    skin_png: &[u8],
    model: SkinModel,
    options: &SkinPreviewOptions,
    format: OutputFormat,
) -> Result<Vec<u8>, RenderError> {
    let image = render_skin_preview(skin_png, model, options)?;
    encode_image(image, format)
}

/// Render a flat 2D skin texture preview and encode it as PNG or WebP bytes.
pub fn render_skin_2d_preview_bytes(
    skin_png: &[u8],
    model: SkinModel,
    options: &Skin2dPreviewOptions,
    format: OutputFormat,
) -> Result<Vec<u8>, RenderError> {
    let image = render_skin_2d_preview(skin_png, model, options)?;
    encode_image(image, format)
}

/// Render a projected 3D dual-view skin preview.
pub fn render_skin_preview(
    skin_png: &[u8],
    model: SkinModel,
    options: &SkinPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    skin3d::render_skin_preview(skin_png, model, options)
}

/// Render a projected 3D dual-view skin preview from a decoded skin texture.
pub fn render_decoded_skin_preview(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &SkinPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    skin3d::render_skin_preview_image(skin.image(), model, options)
}

/// Render a projected 3D dual-view skin preview into reusable workspace buffers.
pub fn render_skin_preview_with_workspace<'a>(
    skin_png: &[u8],
    model: SkinModel,
    options: &SkinPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    skin3d::render_skin_preview_with_workspace(skin_png, model, options, workspace)
}

/// Render a decoded 3D skin preview into reusable workspace buffers.
pub fn render_decoded_skin_preview_with_workspace<'a>(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &SkinPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    skin3d::render_skin_preview_image_with_workspace(skin.image(), model, options, workspace)
}

/// Render the raw Minecraft skin texture as a scaled, centered 2D preview.
pub fn render_skin_2d_preview(
    skin_png: &[u8],
    model: SkinModel,
    options: &Skin2dPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    skin2d::render_skin_2d_preview(skin_png, model, options)
}

/// Render a flat 2D player preview from a decoded skin texture.
pub fn render_decoded_skin_2d_preview(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &Skin2dPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    skin2d::render_skin_2d_preview_image(skin.image(), model, options)
}

/// Render a flat 2D player preview into reusable workspace buffers.
pub fn render_skin_2d_preview_with_workspace<'a>(
    skin_png: &[u8],
    model: SkinModel,
    options: &Skin2dPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    skin2d::render_skin_2d_preview_with_workspace(skin_png, model, options, workspace)
}

/// Render a decoded flat 2D player preview into reusable workspace buffers.
pub fn render_decoded_skin_2d_preview_with_workspace<'a>(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &Skin2dPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    skin2d::render_skin_2d_preview_image_with_workspace(skin.image(), model, options, workspace)
}

fn encode_image(image: RgbaImage, format: OutputFormat) -> Result<Vec<u8>, RenderError> {
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), format.image_format())
        .map_err(RenderError::EncodeImage)?;
    Ok(bytes)
}

pub(crate) fn blend_pixel(target: &mut Rgba<u8>, source: Rgba<u8>) {
    let source_alpha = f32::from(source[3]) / 255.0;
    if source_alpha >= 1.0 {
        *target = source;
        return;
    }

    let target_alpha = f32::from(target[3]) / 255.0;
    let out_alpha = source_alpha + target_alpha * (1.0 - source_alpha);
    if out_alpha <= EPSILON {
        *target = Rgba([0, 0, 0, 0]);
        return;
    }

    let mut out = [0_u8; 4];
    for channel in 0..3 {
        let source_value = f32::from(source[channel]) / 255.0;
        let target_value = f32::from(target[channel]) / 255.0;
        let value = (source_value * source_alpha
            + target_value * target_alpha * (1.0 - source_alpha))
            / out_alpha;
        out[channel] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    *target = Rgba(out);
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat};

    use super::*;

    #[test]
    fn workspace_preview_api_reuses_image_buffers() {
        let skin = fixture_skin();
        let mut workspace = RenderWorkspace::new();
        let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

        {
            let preview =
                render_preview_with_workspace(&skin, SkinModel::Slim, &options, &mut workspace)
                    .unwrap();
            assert_eq!(preview.dimensions(), (430, 430));
        }
        let capacity_after_first_render = workspace.total_capacity_bytes();

        {
            let preview =
                render_preview_with_workspace(&skin, SkinModel::Slim, &options, &mut workspace)
                    .unwrap();
            assert_eq!(preview.dimensions(), (430, 430));
        }

        assert_eq!(
            workspace.total_capacity_bytes(),
            capacity_after_first_render
        );
        assert!(workspace.output_capacity_bytes() > 0);
        assert!(workspace.scratch_capacity_bytes() > 0);
    }

    #[test]
    fn decoded_workspace_preview_api_avoids_redecode() {
        let skin = DecodedSkin::from_png_bytes(&fixture_skin()).unwrap();
        let mut workspace = RenderWorkspace::new();
        let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

        {
            let preview = render_decoded_preview_with_workspace(
                &skin,
                SkinModel::Slim,
                &options,
                &mut workspace,
            )
            .unwrap();
            assert_eq!(preview.dimensions(), (430, 430));
        }
        let capacity_after_first_render = workspace.total_capacity_bytes();

        {
            let preview = render_decoded_preview_with_workspace(
                &skin,
                SkinModel::Slim,
                &options,
                &mut workspace,
            )
            .unwrap();
            assert_eq!(preview.dimensions(), (430, 430));
        }

        assert_eq!(
            workspace.total_capacity_bytes(),
            capacity_after_first_render
        );
        assert_eq!(skin.dimensions(), (64, 64));
    }

    fn fixture_skin() -> Vec<u8> {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        paint_rect(&mut image, 8, 8, 8, 8, Rgba([240, 130, 80, 255]));
        paint_rect(&mut image, 20, 20, 8, 12, Rgba([80, 120, 240, 255]));
        paint_rect(&mut image, 44, 20, 4, 12, Rgba([40, 45, 50, 255]));
        paint_rect(&mut image, 20, 52, 4, 12, Rgba([245, 245, 245, 255]));
        paint_rect(&mut image, 40, 8, 8, 8, Rgba([255, 210, 80, 180]));

        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn paint_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
        for next_y in y..(y + height).min(image.height()) {
            for next_x in x..(x + width).min(image.width()) {
                image.put_pixel(next_x, next_y, color);
            }
        }
    }
}
