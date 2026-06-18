//! Flat 2D Minecraft player preview engine.
//!
//! This engine renders the same kind of flat front/back paper-doll preview used
//! by many skin sites:
//!
//! 1. Decode and validate a Minecraft skin layout.
//! 2. Build two flat player views, front on the left and back on the right.
//! 3. Copy vanilla UV rectangles for head, body, arms, and legs into those
//!    flat rectangles with nearest-neighbor sampling.
//! 4. Optionally draw second-layer regions over the base layer.
//! 5. Alpha-blend each copied texel over the optional background.

use image::{ImageBuffer, Rgba, RgbaImage};

use crate::{
    decoded::decode_skin_png,
    engine::validate_skin_dimensions,
    error::RenderError,
    options::{Skin2dPreviewOptions, SkinModel},
    render::blend_pixel,
    workspace::{RenderWorkspace, prepare_image},
};

const PLAYER_HEIGHT: u32 = 32;
const HEAD_WIDTH: u32 = 8;
const BODY_WIDTH: u32 = 8;
const BODY_HEIGHT: u32 = 12;
const LIMB_HEIGHT: u32 = 12;
const LEG_WIDTH: u32 = 4;
const LIMB_DEPTH: u32 = 4;

/// Render front/back flat player views from a Minecraft skin texture.
pub fn render_skin_2d_preview(
    skin_png: &[u8],
    model: SkinModel,
    options: &Skin2dPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    let texture = decode_skin_png(skin_png)?;
    validate_skin_dimensions(&texture)?;
    render_skin_2d_preview_image(&texture, model, options)
}

pub(crate) fn render_skin_2d_preview_image(
    texture: &RgbaImage,
    model: SkinModel,
    options: &Skin2dPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    validate_output_size(options)?;
    let mut output = ImageBuffer::from_pixel(
        options.output_width,
        options.output_height,
        options.background.unwrap_or(Rgba([0, 0, 0, 0])),
    );
    draw_player_views(&texture, &mut output, model, options);
    Ok(output)
}

/// Render front/back flat player views into reusable workspace buffers.
pub fn render_skin_2d_preview_with_workspace<'a>(
    skin_png: &[u8],
    model: SkinModel,
    options: &Skin2dPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    let texture = decode_skin_png(skin_png)?;
    validate_skin_dimensions(&texture)?;
    render_skin_2d_preview_image_with_workspace(&texture, model, options, workspace)
}

pub(crate) fn render_skin_2d_preview_image_with_workspace<'a>(
    texture: &RgbaImage,
    model: SkinModel,
    options: &Skin2dPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    validate_output_size(options)?;
    prepare_image(
        &mut workspace.output,
        options.output_width,
        options.output_height,
        options.background.unwrap_or(Rgba([0, 0, 0, 0])),
    );
    draw_player_views(&texture, &mut workspace.output, model, options);
    Ok(&workspace.output)
}

fn validate_output_size(options: &Skin2dPreviewOptions) -> Result<(), RenderError> {
    if options.output_width == 0 || options.output_height == 0 {
        return Err(RenderError::InvalidOutputSize {
            width: options.output_width,
            height: options.output_height,
        });
    }
    if options.padding.saturating_mul(2) >= options.output_width
        || options.padding.saturating_mul(2) >= options.output_height
    {
        return Err(RenderError::InvalidPadding {
            padding: options.padding,
        });
    }
    Ok(())
}

fn draw_player_views(
    texture: &RgbaImage,
    output: &mut RgbaImage,
    model: SkinModel,
    options: &Skin2dPreviewOptions,
) {
    let legacy = texture.height() * 2 == texture.width();
    let layout = FlatLayout::new(model);
    let available_width = options.output_width.saturating_sub(options.padding * 2);
    let available_height = options.output_height.saturating_sub(options.padding * 2);
    let available_width = available_width.saturating_sub(options.view_spacing);
    let total_model_width = layout.player_width * 2;
    let scale = (available_width as f32 / total_model_width as f32)
        .min(available_height as f32 / PLAYER_HEIGHT as f32)
        .floor()
        .max(1.0) as u32;
    let rendered_width = total_model_width * scale + options.view_spacing;
    let rendered_height = PLAYER_HEIGHT * scale;
    let start_x = (options.output_width - rendered_width) / 2;
    let start_y = (options.output_height - rendered_height) / 2;
    let back_x = start_x + layout.player_width * scale + options.view_spacing;

    draw_flat_player(
        texture,
        output,
        layout,
        View::Front,
        start_x,
        start_y,
        scale,
        legacy,
        false,
    );
    draw_flat_player(
        texture,
        output,
        layout,
        View::Back,
        back_x,
        start_y,
        scale,
        legacy,
        false,
    );

    if options.show_outer_layer && !legacy {
        draw_flat_player(
            texture,
            output,
            layout,
            View::Front,
            start_x,
            start_y,
            scale,
            false,
            true,
        );
        draw_flat_player(
            texture,
            output,
            layout,
            View::Back,
            back_x,
            start_y,
            scale,
            false,
            true,
        );
    }
}

fn draw_flat_player(
    texture: &RgbaImage,
    output: &mut RgbaImage,
    layout: FlatLayout,
    view: View,
    origin_x: u32,
    origin_y: u32,
    scale: u32,
    legacy: bool,
    layer: bool,
) {
    let arm = layout.arm_width;
    let body_x = arm;
    let head_x = arm;
    let right_leg_x = body_x;
    let left_leg_x = body_x + LEG_WIDTH;
    let right_arm_x = 0;
    let left_arm_x = body_x + BODY_WIDTH;

    let uvs = FlatUvs::new(view, layer, arm, legacy);
    let head_expand = if layer { scale / 2 } else { 0 };
    let body_expand = if layer { scale / 4 } else { 0 };

    // Draw from back to front. Arms and legs are the lowest layer in a flat
    // paper-doll preview, torso sits above them, and the head should cover the
    // neckline/hair edges last.
    draw_part(
        texture,
        output,
        uvs.right_arm,
        origin_x,
        origin_y,
        right_arm_x,
        8,
        arm,
        LIMB_HEIGHT,
        scale,
        body_expand,
        uvs.mirror_right_arm,
    );
    draw_part(
        texture,
        output,
        uvs.left_arm,
        origin_x,
        origin_y,
        left_arm_x,
        8,
        arm,
        LIMB_HEIGHT,
        scale,
        body_expand,
        uvs.mirror_left_arm,
    );
    draw_part(
        texture,
        output,
        uvs.right_leg,
        origin_x,
        origin_y,
        right_leg_x,
        20,
        LEG_WIDTH,
        LIMB_HEIGHT,
        scale,
        body_expand,
        uvs.mirror_right_leg,
    );
    draw_part(
        texture,
        output,
        uvs.left_leg,
        origin_x,
        origin_y,
        left_leg_x,
        20,
        LEG_WIDTH,
        LIMB_HEIGHT,
        scale,
        body_expand,
        uvs.mirror_left_leg,
    );
    draw_part(
        texture,
        output,
        uvs.body,
        origin_x,
        origin_y,
        body_x,
        8,
        BODY_WIDTH,
        BODY_HEIGHT,
        scale,
        body_expand,
        false,
    );
    draw_part(
        texture,
        output,
        uvs.head,
        origin_x,
        origin_y,
        head_x,
        0,
        HEAD_WIDTH,
        8,
        scale,
        head_expand,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_part(
    texture: &RgbaImage,
    output: &mut RgbaImage,
    uv: UvRect,
    origin_x: u32,
    origin_y: u32,
    model_x: u32,
    model_y: u32,
    model_width: u32,
    model_height: u32,
    scale: u32,
    expand: u32,
    mirror_u: bool,
) {
    let texture_scale = texture.width() / 64;
    let base_x = origin_x + model_x * scale;
    let base_y = origin_y + model_y * scale;
    let dest_x = base_x.saturating_sub(expand);
    let dest_y = base_y.saturating_sub(expand);
    let dest_width = model_width * scale + expand * 2;
    let dest_height = model_height * scale + expand * 2;
    let source_width = model_width * scale;
    let source_height = model_height * scale;

    for y in 0..dest_height {
        for x in 0..dest_width {
            let source_space_x = (x * source_width) / dest_width;
            let source_space_y = (y * source_height) / dest_height;
            let local_x = source_space_x / scale;
            let local_y = source_space_y / scale;
            let source_local_x = if mirror_u {
                model_width - 1 - local_x
            } else {
                local_x
            };
            let source_x = (uv.x + source_local_x.min(uv.width - 1)) * texture_scale;
            let source_y = (uv.y + local_y.min(uv.height - 1)) * texture_scale;
            let color = *texture.get_pixel(source_x, source_y);
            if color[3] == 0 {
                continue;
            }
            let output_x = dest_x + x;
            let output_y = dest_y + y;
            if output_x >= output.width() || output_y >= output.height() {
                continue;
            }
            blend_pixel(output.get_pixel_mut(output_x, output_y), color);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FlatLayout {
    arm_width: u32,
    player_width: u32,
}

impl FlatLayout {
    fn new(model: SkinModel) -> Self {
        let arm_width = match model {
            SkinModel::Default => 4,
            SkinModel::Slim => 3,
        };
        Self {
            arm_width,
            player_width: BODY_WIDTH + arm_width * 2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum View {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy)]
struct UvRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct FlatUvs {
    head: UvRect,
    body: UvRect,
    right_arm: UvRect,
    left_arm: UvRect,
    right_leg: UvRect,
    left_leg: UvRect,
    mirror_right_arm: bool,
    mirror_left_arm: bool,
    mirror_right_leg: bool,
    mirror_left_leg: bool,
}

impl FlatUvs {
    fn new(view: View, layer: bool, arm_width: u32, legacy: bool) -> Self {
        let head_x = if layer { 32 } else { 0 };
        let body_y = if layer { 32 } else { 16 };
        let right_arm_y = if layer { 32 } else { 16 };
        let right_leg_y = if layer { 32 } else { 16 };
        let left_arm_x = if layer { 48 } else { 32 };
        let left_leg_x = if layer { 0 } else { 16 };
        let right_arm_back_x = 44 + arm_width + LIMB_DEPTH;
        let left_arm_back_x = left_arm_x + LIMB_DEPTH + arm_width + LIMB_DEPTH;

        match view {
            View::Front => Self {
                head: uv(head_x + 8, 8, 8, 8),
                body: uv(20, body_y + 4, 8, 12),
                right_arm: uv(44, right_arm_y + 4, arm_width, 12),
                left_arm: if legacy {
                    uv(44, right_arm_y + 4, arm_width, 12)
                } else {
                    uv(left_arm_x + 4, 52, arm_width, 12)
                },
                right_leg: uv(4, right_leg_y + 4, 4, 12),
                left_leg: if legacy {
                    uv(4, right_leg_y + 4, 4, 12)
                } else {
                    uv(left_leg_x + 4, 52, 4, 12)
                },
                mirror_right_arm: false,
                mirror_left_arm: legacy,
                mirror_right_leg: false,
                mirror_left_leg: legacy,
            },
            View::Back => Self {
                head: uv(head_x + 24, 8, 8, 8),
                body: uv(32, body_y + 4, 8, 12),
                right_arm: if legacy {
                    uv(right_arm_back_x, right_arm_y + 4, arm_width, 12)
                } else {
                    uv(left_arm_back_x, 52, arm_width, 12)
                },
                left_arm: uv(right_arm_back_x, right_arm_y + 4, arm_width, 12),
                right_leg: if legacy {
                    uv(12, right_leg_y + 4, 4, 12)
                } else {
                    uv(left_leg_x + 12, 52, 4, 12)
                },
                left_leg: uv(12, right_leg_y + 4, 4, 12),
                mirror_right_arm: legacy,
                mirror_left_arm: false,
                mirror_right_leg: legacy,
                mirror_left_leg: false,
            },
        }
    }
}

const fn uv(x: u32, y: u32, width: u32, height: u32) -> UvRect {
    UvRect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use super::*;

    #[test]
    fn rejects_padding_that_leaves_no_drawable_area() {
        let options = Skin2dPreviewOptions {
            output_width: 32,
            output_height: 32,
            padding: 16,
            ..Skin2dPreviewOptions::default()
        };

        let error = validate_output_size(&options).unwrap_err();

        assert!(matches!(error, RenderError::InvalidPadding { padding: 16 }));
    }

    #[test]
    fn renders_front_and_back_player_views() {
        let skin = fixture_skin();
        let options = Skin2dPreviewOptions {
            output_width: 160,
            output_height: 160,
            padding: 16,
            view_spacing: 20,
            background: Some(Rgba([255, 255, 255, 255])),
            show_outer_layer: false,
        };

        let preview = render_skin_2d_preview(&skin, SkinModel::Default, &options).unwrap();

        assert_eq!(preview.dimensions(), (160, 160));
        assert!(
            preview
                .pixels()
                .any(|pixel| *pixel == Rgba([40, 80, 220, 255]))
        );
        assert!(
            preview
                .pixels()
                .any(|pixel| *pixel == Rgba([220, 80, 40, 255]))
        );
    }

    #[test]
    fn outer_layer_changes_output_when_enabled() {
        let skin = fixture_skin();
        let mut options = Skin2dPreviewOptions {
            output_width: 160,
            output_height: 160,
            padding: 16,
            view_spacing: 20,
            background: None,
            show_outer_layer: false,
        };
        let base = render_skin_2d_preview(&skin, SkinModel::Default, &options).unwrap();

        options.show_outer_layer = true;
        let layered = render_skin_2d_preview(&skin, SkinModel::Default, &options).unwrap();

        assert_ne!(base.as_raw(), layered.as_raw());
    }

    fn fixture_skin() -> Vec<u8> {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        paint_rect(&mut image, 8, 8, 8, 8, Rgba([220, 80, 40, 255]));
        paint_rect(&mut image, 24, 8, 8, 8, Rgba([40, 80, 220, 255]));
        paint_rect(&mut image, 20, 20, 8, 12, Rgba([220, 80, 40, 255]));
        paint_rect(&mut image, 32, 20, 8, 12, Rgba([40, 80, 220, 255]));
        paint_rect(&mut image, 40, 8, 8, 8, Rgba([255, 220, 40, 180]));
        paint_rect(&mut image, 20, 36, 8, 12, Rgba([255, 220, 40, 180]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
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
