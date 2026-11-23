//! Render-engine throughput baselines (`cargo bench --features benchmarks`).

use std::hint::black_box;

use aster_yggdrasil_render::{
    OutputFormat, Skin2dPreviewOptions, SkinModel, SkinPreviewOptions, SkinPreviewProfile,
    render_skin_2d_preview_bytes, render_skin_preview_bytes,
};
use criterion::{Criterion, criterion_group, criterion_main};
use image::{ImageFormat, Rgba, RgbaImage};
use rand::{RngExt, SeedableRng};

/// Build a deterministic 64x64 slim-model skin: layered colors so the
/// 3D projection and the 2D scaler both touch many regions.
fn synthetic_skin_png() -> Vec<u8> {
    let mut image = RgbaImage::new(64, 64);
    // Fixed seed: every benchmark run has to render the same PNG bytes, or
    // recorded baselines stop being comparable across runs.
    let mut rng = rand::rngs::StdRng::seed_from_u64(0x5EED_1547);
    for (_x, _y, pixel) in image.enumerate_pixels_mut() {
        let base = rng.random_range(0..=255u8);
        *pixel = Rgba([base, base / 2, 255 - base, 255]);
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("synthetic skin should encode");
    bytes.into_inner()
}

fn bench_render_engines(criterion: &mut Criterion) {
    let skin = synthetic_skin_png();
    let model = SkinModel::Slim;
    let three_d = SkinPreviewOptions::default();
    let two_d = Skin2dPreviewOptions::default();

    let mut group = criterion.benchmark_group("render_skin_preview");
    group.throughput(criterion::Throughput::Bytes(skin.len() as u64));
    group.bench_function("skin_3d_png", |b| {
        b.iter(|| {
            let bytes = render_skin_preview_bytes(
                black_box(&skin),
                model,
                black_box(&three_d),
                OutputFormat::Png,
            )
            .expect("3D render should succeed");
            black_box(bytes.len())
        })
    });
    group.bench_function("skin_2d_png", |b| {
        b.iter(|| {
            let bytes = render_skin_2d_preview_bytes(
                black_box(&skin),
                model,
                black_box(&two_d),
                OutputFormat::Png,
            )
            .expect("2D render should succeed");
            black_box(bytes.len())
        })
    });
    group.finish();
}

/// Cost of the built-in supersampling profiles on the same input skin, so
/// preview-size and sampling-strategy decisions have a reference point.
fn bench_preview_profiles(criterion: &mut Criterion) {
    let skin = synthetic_skin_png();
    let model = SkinModel::Slim;

    let mut group = criterion.benchmark_group("render_skin_preview_profiles");
    for (name, profile) in [
        ("skin_3d_png_fast", SkinPreviewProfile::Fast),
        ("skin_3d_png_default", SkinPreviewProfile::Default),
        ("skin_3d_png_quality", SkinPreviewProfile::Quality),
    ] {
        let options = SkinPreviewOptions::from_profile(profile);
        group.bench_function(name, |b| {
            b.iter(|| {
                let bytes = render_skin_preview_bytes(
                    black_box(&skin),
                    model,
                    black_box(&options),
                    OutputFormat::Png,
                )
                .expect("3D render should succeed");
                black_box(bytes.len())
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_render_engines, bench_preview_profiles);
criterion_main!(benches);
