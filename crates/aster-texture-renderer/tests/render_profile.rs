use std::time::{Duration, Instant};

use aster_texture_renderer::{
    DecodedSkin, OutputFormat, RenderWorkspace, Skin2dPreviewOptions, SkinModel,
    SkinPreviewOptions, TexturePreviewOptions, render_decoded_preview_with_workspace,
    render_preview, render_preview_bytes, render_preview_with_workspace,
};
use image::Rgba;
use sysinfo::{Pid, ProcessesToUpdate, System};

const BLUE_SLIM_SKIN: &[u8] = include_bytes!("fixtures/skins/blue.png");
const ORANGE_SLIM_SKIN: &[u8] = include_bytes!("fixtures/skins/orange.png");
const STEVE_SKIN: &[u8] = include_bytes!("fixtures/skins/steve.png");

const WARMUP_ITERATIONS: usize = 20;
const DEFAULT_PROFILE_ITERATIONS: usize = 500;

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_rgba_default() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions {
        background: Some(Rgba([255, 255, 255, 255])),
        ..SkinPreviewOptions::default()
    });

    profile_render(
        "skin-3d rgba default aa2",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_rgba_no_antialiasing() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions {
        supersampling: 1,
        background: Some(Rgba([255, 255, 255, 255])),
        ..SkinPreviewOptions::default()
    });

    profile_render(
        "skin-3d rgba aa1",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_rgba_supersampling_3x() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions {
        supersampling: 3,
        background: Some(Rgba([255, 255, 255, 255])),
        ..SkinPreviewOptions::default()
    });

    profile_render(
        "skin-3d rgba aa3",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_rgba_default() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions {
        background: Some(Rgba([255, 255, 255, 255])),
        ..Skin2dPreviewOptions::default()
    });

    profile_render(
        "skin-2d rgba default",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_rgba_without_outer_layer() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions {
        show_outer_layer: false,
        background: Some(Rgba([255, 255, 255, 255])),
        ..Skin2dPreviewOptions::default()
    });

    profile_render(
        "skin-2d rgba no outer layer",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_png_encode_default() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

    profile_encode(
        "skin-3d png render+encode",
        STEVE_SKIN,
        SkinModel::Default,
        &options,
        OutputFormat::Png,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_webp_encode_default() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

    profile_encode(
        "skin-3d webp render+encode",
        STEVE_SKIN,
        SkinModel::Default,
        &options,
        OutputFormat::WebP,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_png_encode_default() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions::default());

    profile_encode(
        "skin-2d png render+encode",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
        OutputFormat::Png,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_webp_encode_default() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions::default());

    profile_encode(
        "skin-2d webp render+encode",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
        OutputFormat::WebP,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_memory_growth() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

    profile_memory_growth(
        "skin-3d memory growth",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_workspace_memory_growth() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

    profile_workspace_memory_growth(
        "skin-3d workspace memory growth",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin3d_decoded_workspace_memory_growth() {
    let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::default());

    profile_decoded_workspace_memory_growth(
        "skin-3d decoded workspace memory growth",
        BLUE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_memory_growth() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions::default());

    profile_memory_growth(
        "skin-2d memory growth",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_workspace_memory_growth() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions::default());

    profile_workspace_memory_growth(
        "skin-2d workspace memory growth",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

#[test]
#[ignore = "profiling test; run manually with --ignored --nocapture"]
fn profile_skin2d_decoded_workspace_memory_growth() {
    let options = TexturePreviewOptions::Skin2d(Skin2dPreviewOptions::default());

    profile_decoded_workspace_memory_growth(
        "skin-2d decoded workspace memory growth",
        ORANGE_SLIM_SKIN,
        SkinModel::Slim,
        &options,
    );
}

fn profile_render(name: &str, skin: &[u8], model: SkinModel, options: &TexturePreviewOptions) {
    let iterations = profile_iterations();
    warmup(skin, model, options);

    let started_at = Instant::now();
    let mut rendered_pixels = 0_u64;
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let preview = render_preview(skin, model, options).unwrap();
        rendered_pixels += u64::from(preview.width()) * u64::from(preview.height());
        checksum = checksum.wrapping_add(u64::from(preview.as_raw()[index % preview.len()]));
    }
    let elapsed = started_at.elapsed();

    print_timing_report(name, iterations, elapsed);
    println!(
        "rendered_pixels={rendered_pixels}, checksum_guard={checksum}, options={}",
        describe_options(options)
    );
}

fn profile_encode(
    name: &str,
    skin: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
    format: OutputFormat,
) {
    let iterations = (profile_iterations().max(1) / 5).max(1);
    warmup(skin, model, options);

    let started_at = Instant::now();
    let mut encoded_bytes = 0_usize;
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        let bytes = render_preview_bytes(skin, model, options, format).unwrap();
        checksum = checksum.wrapping_add(u64::from(bytes[0]));
        encoded_bytes += bytes.len();
    }
    let elapsed = started_at.elapsed();

    print_timing_report(name, iterations, elapsed);
    println!(
        "encoded_bytes={encoded_bytes}, checksum_guard={checksum}, options={}",
        describe_options(options)
    );
}

fn profile_memory_growth(
    name: &str,
    skin: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
) {
    let iterations = profile_iterations();
    let mut rss = RssSampler::new();

    warmup(skin, model, options);
    let before = rss.current_bytes();

    let mut peak = before;
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let preview = render_preview(skin, model, options).unwrap();
        checksum = checksum.wrapping_add(u64::from(preview.as_raw()[index % preview.len()]));

        if index % 25 == 0 {
            peak = peak.max(rss.current_bytes());
        }
    }
    let after = rss.current_bytes();
    peak = peak.max(after);

    println!(
        "{name}: iterations={iterations}, before={}, after={}, peak={}, delta={}, peak_delta={}, checksum_guard={}, options={}",
        format_bytes(before),
        format_bytes(after),
        format_bytes(peak),
        format_signed_bytes(after as i128 - before as i128),
        format_signed_bytes(peak as i128 - before as i128),
        checksum,
        describe_options(options),
    );
}

fn profile_workspace_memory_growth(
    name: &str,
    skin: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
) {
    let iterations = profile_iterations();
    let mut rss = RssSampler::new();
    let mut workspace = RenderWorkspace::new();

    warmup_workspace(skin, model, options, &mut workspace);
    let before = rss.current_bytes();

    let mut peak = before;
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let preview = render_preview_with_workspace(skin, model, options, &mut workspace).unwrap();
        checksum = checksum.wrapping_add(u64::from(preview.as_raw()[index % preview.len()]));

        if index % 25 == 0 {
            peak = peak.max(rss.current_bytes());
        }
    }
    let after = rss.current_bytes();
    peak = peak.max(after);

    println!(
        "{name}: iterations={iterations}, before={}, after={}, peak={}, delta={}, peak_delta={}, workspace_capacity={}, checksum_guard={}, options={}",
        format_bytes(before),
        format_bytes(after),
        format_bytes(peak),
        format_signed_bytes(after as i128 - before as i128),
        format_signed_bytes(peak as i128 - before as i128),
        format_bytes(workspace.total_capacity_bytes() as u64),
        checksum,
        describe_options(options),
    );
}

fn profile_decoded_workspace_memory_growth(
    name: &str,
    skin: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
) {
    let iterations = profile_iterations();
    let mut rss = RssSampler::new();
    let decoded = DecodedSkin::from_png_bytes(skin).unwrap();
    let mut workspace = RenderWorkspace::new();

    warmup_decoded_workspace(&decoded, model, options, &mut workspace);
    let before = rss.current_bytes();

    let mut peak = before;
    let mut checksum = 0_u64;
    for index in 0..iterations {
        let preview =
            render_decoded_preview_with_workspace(&decoded, model, options, &mut workspace)
                .unwrap();
        checksum = checksum.wrapping_add(u64::from(preview.as_raw()[index % preview.len()]));

        if index % 25 == 0 {
            peak = peak.max(rss.current_bytes());
        }
    }
    let after = rss.current_bytes();
    peak = peak.max(after);

    println!(
        "{name}: iterations={iterations}, before={}, after={}, peak={}, delta={}, peak_delta={}, workspace_capacity={}, decoded_skin={}x{}, checksum_guard={}, options={}",
        format_bytes(before),
        format_bytes(after),
        format_bytes(peak),
        format_signed_bytes(after as i128 - before as i128),
        format_signed_bytes(peak as i128 - before as i128),
        format_bytes(workspace.total_capacity_bytes() as u64),
        decoded.dimensions().0,
        decoded.dimensions().1,
        checksum,
        describe_options(options),
    );
}

fn warmup(skin: &[u8], model: SkinModel, options: &TexturePreviewOptions) {
    for _ in 0..WARMUP_ITERATIONS {
        let preview = render_preview(skin, model, options).unwrap();
        std::hint::black_box(preview);
    }
}

fn warmup_workspace(
    skin: &[u8],
    model: SkinModel,
    options: &TexturePreviewOptions,
    workspace: &mut RenderWorkspace,
) {
    for _ in 0..WARMUP_ITERATIONS {
        let preview = render_preview_with_workspace(skin, model, options, workspace).unwrap();
        std::hint::black_box(preview);
    }
}

fn warmup_decoded_workspace(
    skin: &DecodedSkin,
    model: SkinModel,
    options: &TexturePreviewOptions,
    workspace: &mut RenderWorkspace,
) {
    for _ in 0..WARMUP_ITERATIONS {
        let preview =
            render_decoded_preview_with_workspace(skin, model, options, workspace).unwrap();
        std::hint::black_box(preview);
    }
}

fn profile_iterations() -> usize {
    std::env::var("ASTER_TEXTURE_RENDERER_PROFILE_ITERS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|iterations| *iterations > 0)
        .unwrap_or(DEFAULT_PROFILE_ITERATIONS)
}

fn print_timing_report(name: &str, iterations: usize, elapsed: Duration) {
    let average = elapsed.as_secs_f64() * 1_000.0 / iterations as f64;
    let per_second = iterations as f64 / elapsed.as_secs_f64();
    println!(
        "{name}: iterations={iterations}, total={elapsed:?}, average={average:.3}ms, throughput={per_second:.1}/s"
    );
}

fn describe_options(options: &TexturePreviewOptions) -> String {
    match options {
        TexturePreviewOptions::Skin3d(options) => format!(
            "engine=skin-3d, size={}x{}, scale={}, pitch={}, front_yaw={}, back_yaw={}, spacing={}, supersampling={}, outer_layer={}",
            options.output_width,
            options.output_height,
            options.scale,
            options.pitch_degrees,
            options.front_yaw_degrees,
            options.back_yaw_degrees,
            options.view_spacing,
            options.supersampling,
            options.show_outer_layer,
        ),
        TexturePreviewOptions::Skin2d(options) => format!(
            "engine=skin-2d, size={}x{}, padding={}, spacing={}, outer_layer={}",
            options.output_width,
            options.output_height,
            options.padding,
            options.view_spacing,
            options.show_outer_layer,
        ),
    }
}

struct RssSampler {
    system: System,
    pid: Pid,
}

impl RssSampler {
    fn new() -> Self {
        Self {
            system: System::new(),
            pid: Pid::from_u32(std::process::id()),
        }
    }

    fn current_bytes(&mut self) -> u64 {
        self.system
            .refresh_processes(ProcessesToUpdate::Some(&[self.pid]), false);
        self.system
            .process(self.pid)
            .map(|process| process.memory())
            .unwrap_or(0)
    }
}

fn format_bytes(bytes: u64) -> String {
    format!("{:.2} MiB", bytes as f64 / 1024.0 / 1024.0)
}

fn format_signed_bytes(bytes: i128) -> String {
    let sign = if bytes < 0 { "-" } else { "" };
    format!("{sign}{:.2} MiB", bytes.abs() as f64 / 1024.0 / 1024.0)
}
