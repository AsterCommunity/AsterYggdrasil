//! Projected 3D Minecraft skin preview engine.
//!
//! The renderer is deliberately small and CPU-only. It does not implement a
//! general 3D pipeline; it knows just enough Minecraft geometry to make static
//! backend previews deterministic:
//!
//! 1. Decode and validate a Minecraft skin layout.
//! 2. Build player cuboids and attach vanilla skin UV rectangles.
//! 3. Project cuboid corners with yaw, pitch, and orthographic scale.
//! 4. Keep all cuboid faces, then depth-sort them from far to near.
//! 5. Split each quad into two triangles and rasterize by barycentric weights.
//! 6. Sample the source skin with nearest-neighbor lookup and alpha-blend.
//! 7. Optionally render at an integer supersampled size and downsample in
//!    premultiplied-alpha space to soften diagonal edges without black fringes.

use image::{ImageBuffer, Rgba, RgbaImage};

use crate::{
    decoded::decode_skin_png,
    engine::validate_skin_dimensions,
    error::RenderError,
    geometry::{Camera, DrawFace, EPSILON, ProjectedPoint, UvRect, Vec2},
    options::{SkinModel, SkinPreviewOptions},
    render::blend_pixel,
    skin::{TextureLayout, collect_player_faces},
    workspace::{RenderWorkspace, prepare_image},
};

const MAX_SUPERSAMPLING: u8 = 4;

/// Render a projected 3D dual-view skin preview.
pub fn render_skin_preview(
    skin_png: &[u8],
    model: SkinModel,
    options: &SkinPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    let texture = decode_skin_png(skin_png)?;
    validate_skin_dimensions(&texture)?;
    render_skin_preview_image(&texture, model, options)
}

pub(crate) fn render_skin_preview_image(
    texture: &RgbaImage,
    model: SkinModel,
    options: &SkinPreviewOptions,
) -> Result<RgbaImage, RenderError> {
    validate_output_size(options)?;
    if options.supersampling <= 1 {
        let mut output = new_output(options);
        render_skin_preview_rgba_into(texture, model, options, &mut output);
        return Ok(output);
    }

    let scaled_options = scaled_options(options)?;
    let mut high_res = new_output(&scaled_options);
    render_skin_preview_rgba_into(texture, model, &scaled_options, &mut high_res);
    let mut output = RgbaImage::new(options.output_width, options.output_height);
    downsample_premultiplied_into(
        &high_res,
        &mut output,
        options.output_width,
        options.output_height,
        u32::from(options.supersampling),
    );
    Ok(output)
}

/// Render a projected 3D dual-view skin preview into reusable workspace buffers.
pub fn render_skin_preview_with_workspace<'a>(
    skin_png: &[u8],
    model: SkinModel,
    options: &SkinPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    let texture = decode_skin_png(skin_png)?;
    validate_skin_dimensions(&texture)?;
    render_skin_preview_image_with_workspace(&texture, model, options, workspace)
}

pub(crate) fn render_skin_preview_image_with_workspace<'a>(
    texture: &RgbaImage,
    model: SkinModel,
    options: &SkinPreviewOptions,
    workspace: &'a mut RenderWorkspace,
) -> Result<&'a RgbaImage, RenderError> {
    validate_output_size(options)?;
    if options.supersampling <= 1 {
        prepare_image(
            &mut workspace.output,
            options.output_width,
            options.output_height,
            options.background.unwrap_or(Rgba([0, 0, 0, 0])),
        );
        render_skin_preview_rgba_into(texture, model, options, &mut workspace.output);
        return Ok(&workspace.output);
    }

    let scaled_options = scaled_options(options)?;
    prepare_image(
        &mut workspace.scratch,
        scaled_options.output_width,
        scaled_options.output_height,
        scaled_options.background.unwrap_or(Rgba([0, 0, 0, 0])),
    );
    render_skin_preview_rgba_into(texture, model, &scaled_options, &mut workspace.scratch);
    prepare_image(
        &mut workspace.output,
        options.output_width,
        options.output_height,
        Rgba([0, 0, 0, 0]),
    );
    downsample_premultiplied_into(
        &workspace.scratch,
        &mut workspace.output,
        options.output_width,
        options.output_height,
        u32::from(options.supersampling),
    );
    Ok(&workspace.output)
}

fn new_output(options: &SkinPreviewOptions) -> RgbaImage {
    ImageBuffer::from_pixel(
        options.output_width,
        options.output_height,
        options.background.unwrap_or(Rgba([0, 0, 0, 0])),
    )
}

fn render_skin_preview_rgba_into(
    texture: &RgbaImage,
    model: SkinModel,
    options: &SkinPreviewOptions,
    output: &mut RgbaImage,
) {
    let center_y = (options.output_height as f32 * options.center_y_ratio.clamp(0.0, 1.0))
        + options.vertical_offset;
    let texture_layout = TextureLayout {
        ratio: texture.width() as f32 / 64.0,
        legacy: texture.height() * 2 == texture.width(),
    };
    let back_faces = collect_view_faces(model, options, texture_layout, options.back_yaw_degrees);
    let front_faces = collect_view_faces(model, options, texture_layout, options.front_yaw_degrees);

    // Project each view around the origin first, then place the two bounds
    // together. That keeps layout stable when pitch, yaw, or scale changes.
    let Some(back_bounds) = FaceBounds::from_faces(&back_faces) else {
        return;
    };
    let Some(front_bounds) = FaceBounds::from_faces(&front_faces) else {
        return;
    };

    let spacing = options.view_spacing.max(0.0);
    let total_width = back_bounds.width() + spacing + front_bounds.width();
    let start_x =
        (options.output_width as f32 * 0.5) + options.horizontal_offset - total_width / 2.0;
    let back_offset = Vec2 {
        x: start_x - back_bounds.min_x,
        y: center_y - back_bounds.center_y(),
    };
    let front_offset = Vec2 {
        x: start_x + back_bounds.width() + spacing - front_bounds.min_x,
        y: center_y - front_bounds.center_y(),
    };

    draw_view(texture, output, texture_layout, &back_faces, back_offset);
    draw_view(texture, output, texture_layout, &front_faces, front_offset);
}

fn validate_output_size(options: &SkinPreviewOptions) -> Result<(), RenderError> {
    if options.output_width == 0 || options.output_height == 0 {
        return Err(RenderError::InvalidOutputSize {
            width: options.output_width,
            height: options.output_height,
        });
    }
    if options.supersampling == 0 || options.supersampling > MAX_SUPERSAMPLING {
        return Err(RenderError::InvalidSupersampling {
            supersampling: options.supersampling,
        });
    }
    let supersampling = u32::from(options.supersampling);
    if options.output_width.checked_mul(supersampling).is_none()
        || options.output_height.checked_mul(supersampling).is_none()
    {
        return Err(RenderError::InvalidOutputSize {
            width: options.output_width,
            height: options.output_height,
        });
    }
    Ok(())
}

fn scaled_options(options: &SkinPreviewOptions) -> Result<SkinPreviewOptions, RenderError> {
    let supersampling = u32::from(options.supersampling);
    Ok(SkinPreviewOptions {
        output_width: options.output_width.checked_mul(supersampling).ok_or(
            RenderError::InvalidOutputSize {
                width: options.output_width,
                height: options.output_height,
            },
        )?,
        output_height: options.output_height.checked_mul(supersampling).ok_or(
            RenderError::InvalidOutputSize {
                width: options.output_width,
                height: options.output_height,
            },
        )?,
        scale: options.scale * supersampling as f32,
        pitch_degrees: options.pitch_degrees,
        front_yaw_degrees: options.front_yaw_degrees,
        back_yaw_degrees: options.back_yaw_degrees,
        view_spacing: options.view_spacing * supersampling as f32,
        horizontal_offset: options.horizontal_offset * supersampling as f32,
        vertical_offset: options.vertical_offset * supersampling as f32,
        center_y_ratio: options.center_y_ratio,
        background: options.background,
        show_outer_layer: options.show_outer_layer,
        supersampling: 1,
    })
}

fn collect_view_faces(
    model: SkinModel,
    options: &SkinPreviewOptions,
    texture_layout: TextureLayout,
    yaw_degrees: f32,
) -> Vec<DrawFace> {
    let camera = Camera {
        yaw: yaw_degrees.to_radians(),
        pitch: options.pitch_degrees.to_radians(),
        scale: options.scale,
        center_x: 0.0,
        center_y: 0.0,
    };

    let mut faces = Vec::new();
    collect_player_faces(
        &mut faces,
        camera,
        model,
        texture_layout,
        options.show_outer_layer,
    );

    // We intentionally keep every cuboid face. Backface culling can open tiny
    // gaps where adjacent Minecraft cuboids meet, especially around head layers.
    faces.sort_by(|left, right| left.depth.total_cmp(&right.depth));
    faces
}

fn draw_view(
    texture: &RgbaImage,
    output: &mut RgbaImage,
    texture_layout: TextureLayout,
    faces: &[DrawFace],
    offset: Vec2,
) {
    for face in faces {
        if !uv_rect_has_visible_pixel(texture, texture_layout, face.uv) {
            continue;
        }
        draw_face(output, texture, texture_layout, face, offset);
    }
}

fn draw_face(
    output: &mut RgbaImage,
    texture: &RgbaImage,
    layout: TextureLayout,
    face: &DrawFace,
    offset: Vec2,
) {
    let uv_points = uv_corners(face.uv, face.mirror_u, face.rotate_uv_180);
    draw_triangle(
        output,
        texture,
        layout,
        [
            translate_point(face.points[0], offset),
            translate_point(face.points[1], offset),
            translate_point(face.points[2], offset),
        ],
        [uv_points[0], uv_points[1], uv_points[2]],
    );
    draw_triangle(
        output,
        texture,
        layout,
        [
            translate_point(face.points[0], offset),
            translate_point(face.points[2], offset),
            translate_point(face.points[3], offset),
        ],
        [uv_points[0], uv_points[2], uv_points[3]],
    );
}

fn translate_point(point: ProjectedPoint, offset: Vec2) -> Vec2 {
    Vec2 {
        x: point.position.x + offset.x,
        y: point.position.y + offset.y,
    }
}

fn uv_corners(rect: UvRect, mirror_u: bool, rotate_180: bool) -> [Vec2; 4] {
    let x0 = rect.x;
    let x1 = rect.x + rect.width;
    let (left, right) = if mirror_u { (x1, x0) } else { (x0, x1) };
    let corners = [
        Vec2 {
            x: left,
            y: rect.y + rect.height,
        },
        Vec2 {
            x: right,
            y: rect.y + rect.height,
        },
        Vec2 {
            x: right,
            y: rect.y,
        },
        Vec2 { x: left, y: rect.y },
    ];

    if rotate_180 {
        [corners[2], corners[3], corners[0], corners[1]]
    } else {
        corners
    }
}

#[derive(Debug, Clone, Copy)]
struct FaceBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl FaceBounds {
    fn from_faces(faces: &[DrawFace]) -> Option<Self> {
        let mut points = faces
            .iter()
            .flat_map(|face| face.points.iter().map(|point| point.position));
        let first = points.next()?;
        let mut bounds = Self {
            min_x: first.x,
            max_x: first.x,
            min_y: first.y,
            max_y: first.y,
        };

        for point in points {
            bounds.min_x = bounds.min_x.min(point.x);
            bounds.max_x = bounds.max_x.max(point.x);
            bounds.min_y = bounds.min_y.min(point.y);
            bounds.max_y = bounds.max_y.max(point.y);
        }

        Some(bounds)
    }

    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn center_y(self) -> f32 {
        (self.min_y + self.max_y) / 2.0
    }
}

fn draw_triangle(
    output: &mut RgbaImage,
    texture: &RgbaImage,
    layout: TextureLayout,
    points: [Vec2; 3],
    uvs: [Vec2; 3],
) {
    let min_x = floor_min(points[0].x.min(points[1].x).min(points[2].x));
    let max_x = ceil_max(
        points[0].x.max(points[1].x).max(points[2].x),
        output.width(),
    );
    let min_y = floor_min(points[0].y.min(points[1].y).min(points[2].y));
    let max_y = ceil_max(
        points[0].y.max(points[1].y).max(points[2].y),
        output.height(),
    );
    let area = edge(points[0], points[1], points[2]);
    if area.abs() <= EPSILON {
        return;
    }

    for y in min_y..max_y {
        for x in min_x..max_x {
            let point = Vec2 {
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
            };
            let w0 = edge(points[1], points[2], point) / area;
            let w1 = edge(points[2], points[0], point) / area;
            let w2 = edge(points[0], points[1], point) / area;
            if w0 < -EPSILON || w1 < -EPSILON || w2 < -EPSILON {
                continue;
            }

            // Barycentric interpolation maps the output pixel back to the
            // corresponding Minecraft skin texel.
            let uv = Vec2 {
                x: w0.mul_add(uvs[0].x, w1.mul_add(uvs[1].x, w2 * uvs[2].x)),
                y: w0.mul_add(uvs[0].y, w1.mul_add(uvs[1].y, w2 * uvs[2].y)),
            };
            let color = sample_texture(texture, layout, uv);
            if color[3] == 0 {
                continue;
            }
            blend_pixel(output.get_pixel_mut(x, y), color);
        }
    }
}

fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (c.x - a.x).mul_add(b.y - a.y, -((c.y - a.y) * (b.x - a.x)))
}

fn floor_min(value: f32) -> u32 {
    value.floor().max(0.0) as u32
}

fn ceil_max(value: f32, limit: u32) -> u32 {
    value.ceil().clamp(0.0, limit as f32) as u32
}

fn sample_texture(texture: &RgbaImage, layout: TextureLayout, uv: Vec2) -> Rgba<u8> {
    let max_x = texture.width().saturating_sub(1);
    let max_y = texture.height().saturating_sub(1);
    let x = ((uv.x * layout.ratio).floor() as u32).min(max_x);
    let y = ((uv.y * layout.ratio).floor() as u32).min(max_y);
    *texture.get_pixel(x, y)
}

fn uv_rect_has_visible_pixel(texture: &RgbaImage, layout: TextureLayout, uv: UvRect) -> bool {
    let x0 = ((uv.x * layout.ratio).floor() as u32).min(texture.width());
    let y0 = ((uv.y * layout.ratio).floor() as u32).min(texture.height());
    let x1 = (((uv.x + uv.width) * layout.ratio).ceil() as u32).min(texture.width());
    let y1 = (((uv.y + uv.height) * layout.ratio).ceil() as u32).min(texture.height());

    for y in y0..y1 {
        for x in x0..x1 {
            if texture.get_pixel(x, y)[3] > 0 {
                return true;
            }
        }
    }

    false
}

fn downsample_premultiplied_into(
    source: &RgbaImage,
    output: &mut RgbaImage,
    output_width: u32,
    output_height: u32,
    factor: u32,
) {
    let samples = factor * factor;

    for y in 0..output_height {
        for x in 0..output_width {
            let mut alpha_sum = 0_u32;
            let mut red_sum = 0_u32;
            let mut green_sum = 0_u32;
            let mut blue_sum = 0_u32;

            for sample_y in 0..factor {
                for sample_x in 0..factor {
                    let pixel = source.get_pixel(x * factor + sample_x, y * factor + sample_y);
                    let alpha = u32::from(pixel[3]);
                    alpha_sum += alpha;
                    red_sum += u32::from(pixel[0]) * alpha;
                    green_sum += u32::from(pixel[1]) * alpha;
                    blue_sum += u32::from(pixel[2]) * alpha;
                }
            }

            let alpha = div_round(alpha_sum, samples).min(255) as u8;
            let pixel = if alpha_sum == 0 {
                Rgba([0, 0, 0, 0])
            } else {
                // Premultiplied averaging prevents transparent black pixels from
                // bleeding into antialiased edges.
                Rgba([
                    div_round(red_sum, alpha_sum).min(255) as u8,
                    div_round(green_sum, alpha_sum).min(255) as u8,
                    div_round(blue_sum, alpha_sum).min(255) as u8,
                    alpha,
                ])
            };
            output.put_pixel(x, y, pixel);
        }
    }
}

fn div_round(value: u32, divisor: u32) -> u32 {
    (value + divisor / 2) / divisor
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use crate::geometry::uv;

    use super::*;

    fn downsample_premultiplied(
        source: &RgbaImage,
        output_width: u32,
        output_height: u32,
        factor: u32,
    ) -> RgbaImage {
        let mut output = RgbaImage::new(output_width, output_height);
        downsample_premultiplied_into(source, &mut output, output_width, output_height, factor);
        output
    }

    #[test]
    fn renders_dual_skin_preview() {
        let skin = fixture_skin(false);
        let options = SkinPreviewOptions {
            output_width: 320,
            output_height: 260,
            background: Some(Rgba([255, 255, 255, 255])),
            ..SkinPreviewOptions::default()
        };

        let preview = render_skin_preview(&skin, SkinModel::Default, &options).unwrap();

        assert_eq!(preview.dimensions(), (320, 260));
        let non_background = preview
            .pixels()
            .filter(|pixel| **pixel != Rgba([255, 255, 255, 255]))
            .count();
        assert!(non_background > 6_000);
    }

    #[test]
    fn accepts_slim_and_adjustable_camera() {
        let skin = fixture_skin(false);
        let mut options = SkinPreviewOptions {
            output_width: 260,
            output_height: 220,
            ..SkinPreviewOptions::default()
        };
        let first = render_skin_preview(&skin, SkinModel::Slim, &options).unwrap();

        options.pitch_degrees = -8.0;
        options.front_yaw_degrees = -70.0;
        let second = render_skin_preview(&skin, SkinModel::Slim, &options).unwrap();

        assert_ne!(first.as_raw(), second.as_raw());
    }

    #[test]
    fn accepts_legacy_skin_layout() {
        let skin = fixture_skin(true);
        let options = SkinPreviewOptions {
            output_width: 260,
            output_height: 220,
            ..SkinPreviewOptions::default()
        };

        let preview = render_skin_preview(&skin, SkinModel::Default, &options).unwrap();

        assert_eq!(preview.dimensions(), (260, 220));
        assert!(preview.pixels().any(|pixel| pixel[3] > 0));
    }

    #[test]
    fn rejects_invalid_supersampling() {
        let skin = fixture_skin(false);
        let options = SkinPreviewOptions {
            supersampling: 0,
            ..SkinPreviewOptions::default()
        };

        let error = render_skin_preview(&skin, SkinModel::Default, &options).unwrap_err();

        assert!(matches!(
            error,
            RenderError::InvalidSupersampling { supersampling: 0 }
        ));
    }

    #[test]
    fn keeps_all_cuboid_faces_for_depth_sorted_rendering() {
        let texture_layout = TextureLayout {
            ratio: 1.0,
            legacy: false,
        };
        let options_without_outer_layer = SkinPreviewOptions {
            show_outer_layer: false,
            ..SkinPreviewOptions::default()
        };
        let options_with_outer_layer = SkinPreviewOptions {
            show_outer_layer: true,
            ..SkinPreviewOptions::default()
        };

        let base_faces = collect_view_faces(
            SkinModel::Slim,
            &options_without_outer_layer,
            texture_layout,
            options_without_outer_layer.front_yaw_degrees,
        );
        let layered_faces = collect_view_faces(
            SkinModel::Slim,
            &options_with_outer_layer,
            texture_layout,
            options_with_outer_layer.front_yaw_degrees,
        );

        assert_eq!(base_faces.len(), 36);
        assert_eq!(layered_faces.len(), 72);
    }

    #[test]
    fn rotates_bottom_uv_corners_by_180_degrees() {
        let corners = uv_corners(uv(16.0, 0.0, 8.0, 8.0), false, true);

        assert_eq!(point_tuple(corners[0]), (24.0, 0.0));
        assert_eq!(point_tuple(corners[1]), (16.0, 0.0));
        assert_eq!(point_tuple(corners[2]), (16.0, 8.0));
        assert_eq!(point_tuple(corners[3]), (24.0, 8.0));
    }

    #[test]
    fn downsample_preserves_transparent_edge_color_without_black_fringe() {
        let mut source = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0]));
        source.put_pixel(0, 0, Rgba([240, 80, 20, 255]));

        let output = downsample_premultiplied(&source, 1, 1, 2);
        let pixel = output.get_pixel(0, 0);

        assert_eq!(pixel[0], 240);
        assert_eq!(pixel[1], 80);
        assert_eq!(pixel[2], 20);
        assert_eq!(pixel[3], 64);
    }

    #[test]
    fn skips_fully_transparent_uv_rectangles() {
        let mut texture = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        let layout = TextureLayout {
            ratio: 1.0,
            legacy: false,
        };

        assert!(!uv_rect_has_visible_pixel(
            &texture,
            layout,
            uv(8.0, 8.0, 8.0, 8.0)
        ));

        texture.put_pixel(12, 12, Rgba([255, 255, 255, 1]));

        assert!(uv_rect_has_visible_pixel(
            &texture,
            layout,
            uv(8.0, 8.0, 8.0, 8.0)
        ));
    }

    fn fixture_skin(legacy: bool) -> Vec<u8> {
        let height = if legacy { 32 } else { 64 };
        let mut image = RgbaImage::from_pixel(64, height, Rgba([0, 0, 0, 0]));
        paint_rect(&mut image, 0, 0, 64, height, Rgba([245, 156, 92, 255]));
        paint_rect(&mut image, 20, 20, 8, 12, Rgba([245, 245, 245, 255]));
        paint_rect(&mut image, 44, 20, 4, 12, Rgba([244, 178, 48, 255]));
        paint_rect(&mut image, 4, 20, 4, 12, Rgba([30, 30, 35, 255]));
        paint_rect(&mut image, 8, 8, 8, 8, Rgba([238, 128, 72, 255]));
        if !legacy {
            paint_rect(&mut image, 40, 8, 8, 8, Rgba([255, 178, 120, 150]));
            paint_rect(&mut image, 20, 52, 4, 12, Rgba([25, 25, 30, 255]));
        }

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

    fn point_tuple(point: Vec2) -> (f32, f32) {
        (point.x, point.y)
    }
}
