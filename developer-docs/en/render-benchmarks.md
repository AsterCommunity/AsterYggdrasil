# Render benchmark baseline

Skin previews are rendered by `crates/aster_yggdrasil_render` and sit on the
request path for profile and wardrobe previews, so engine, projection, or PNG
encoding changes can regress user-visible latency. This page records one
reference run and the rules for comparing a later change against it.

The benchmark entry point is `tests/benchmarks/render_skin_preview.rs`. Every
iteration renders a deterministic 64x64 slim skin and encodes the result to
PNG, so the timings include PNG decode, projection/rasterization,
downsampling, and PNG encode.

| Benchmark | What it measures |
| --- | --- |
| `render_skin_preview/skin_3d_png` | Projected dual-view 3D preview, `SkinPreviewOptions::default()` (430x430, `SkinPreviewProfile::Default`, 2x supersampling) |
| `render_skin_preview/skin_2d_png` | Flat front/back 2D preview, `Skin2dPreviewOptions::default()` (430x430) |
| `render_skin_preview_profiles/skin_3d_png_fast` | Same 3D preview with `SkinPreviewProfile::Fast` (1x) |
| `render_skin_preview_profiles/skin_3d_png_default` | Same 3D preview with `SkinPreviewProfile::Default` (2x) |
| `render_skin_preview_profiles/skin_3d_png_quality` | Same 3D preview with `SkinPreviewProfile::Quality` (3x) |

## How to run

```bash
# Record a run; the baseline used on this page is named 2026-11-23.
cargo bench --features benchmarks --bench render_skin_preview -- --save-baseline 2026-11-23

# Compare a later change against it.
cargo bench --features benchmarks --bench render_skin_preview -- --baseline 2026-11-23
```

`--bench render_skin_preview` is required. Plain `cargo bench` also benches the
library target's unit-test harness and passes the criterion flags to libtest,
which fails with `error: Unrecognized option: 'save-baseline'`.

Criterion writes raw measurements and plots to `target/criterion/`. The input
skin uses a fixed RNG seed, so reruns render identical PNG bytes (16,516 bytes
for the current seed); keep it that way or recorded baselines stop being
comparable.

## Reference run

| Item | Value |
| --- | --- |
| Date | 2026-11-23 |
| Revision | `1d80b94` (`master`); the render crate was not modified by this baseline |
| Machine | ASUS TUF Gaming FA608UM (laptop) |
| CPU | AMD Ryzen 7 H 260, 8 cores / 16 threads, 3.8 GHz nominal |
| RAM | 31.3 GiB |
| OS | Windows 11 (10.0.26300) x86_64 |
| Toolchain | `rustc 1.100.0 (0ef1bc38)`, `cargo 1.100.0` |
| Cargo profile | `bench` (inherits `release`): `opt-level = 2`, LTO enabled, `codegen-units = 1` |
| Power plan | Windows power scheme "Silent" (ASUS throttled preset) |
| Input | 64x64 RGBA slim skin, deterministic pixels, PNG, 16,516 bytes |

## Results

| Benchmark | Median (95% CI) | Mean | Throughput |
| --- | --- | --- | --- |
| `render_skin_preview/skin_3d_png` | 75.42 ms (74.64 - 76.07 ms) | 75.63 ms | 213.25 KiB/s |
| `render_skin_preview/skin_2d_png` | 1.87 ms (1.85 - 1.89 ms) | 1.91 ms | 8.19 MiB/s |
| `render_skin_preview_profiles/skin_3d_png_fast` | 20.51 ms (20.18 - 20.76 ms) | 20.70 ms | - |
| `render_skin_preview_profiles/skin_3d_png_default` | 76.41 ms (75.76 - 76.87 ms) | 77.13 ms | - |
| `render_skin_preview_profiles/skin_3d_png_quality` | 165.90 ms (164.87 - 166.83 ms) | 167.11 ms | - |

Throughput is reported against the PNG input size. The 3D path costs about 40x
the 2D path at the same 430x430 output size, which matches the extra work of
projecting and rasterizing twelve cuboid faces twice.

## Supersampling profile scaling

| Profile | Supersampling | Median | Cost vs `Fast` |
| --- | --- | --- | --- |
| `Fast` | 1x | 20.51 ms | 1.0x |
| `Default` | 2x | 76.41 ms | 3.7x |
| `Quality` | 3x | 165.90 ms | 8.1x |

Rasterized pixel count grows with the square of the factor (1x, 4x, 9x), but
measured time grows 1x, 3.7x, 8.1x because PNG decode/encode and the final
downsample are fixed costs. Scaling is therefore consistent with expectations,
slightly below quadratic. `Quality` costs about 2.2x `Default` for the same
430x430 output, so `Default` remains the sensible server-side default.

## Comparability rules

- Compare only on the same machine, power plan, and toolchain, using
  `--baseline 2026-11-23`. Absolute numbers do not transfer between machines.
- Run-to-run noise on the reference machine stayed within about +/-4% (most
  cases +/-2%), so treat differences below 5% as noise.
- A single run is not a verdict: two consecutive runs on the same setup that are
  both at least 10% slower than this baseline is the regression signal. That
  threshold sits more than twice above the measured noise, so laptop jitter does
  not turn into a bug report.
- The reference run used the laptop's throttled `Silent` preset, so these
  numbers are a lower bound. Re-run on a performance plan before using them for
  capacity planning, and record the plan next to any new baseline.
- The benchmark covers one fixed input. Real skins compress differently, so
  PNG decode/encode cost can shift with texture content.

## Relation to AsterDrive and AsterForge

AsterDrive keeps the same criterion harness shape for isolated hotspots
(`tests/benchmarks/`, `harness = false`, `required-features = ["benchmarks"]`)
and records dated prose numbers next to the harness. Its machine-readable
baseline files (`tests/performance/baselines/*.json`, matched by provider and
payload, maintained by a validating updater, with an explicit p95/throughput
policy) belong to the provider-range artifact suite, not to criterion hotspots.
AsterForge only benches `aster_forge_cache` inside that crate; it has no render
benchmark and nothing here to delegate upward.

This page follows the dated-prose convention because #17 asks for the baseline
in `developer-docs` and keeps CI wiring out of scope. No versioned baseline file
is committed yet: with no runner comparing against it, it would be dead data.
Adopt the AsterDrive shape (versioned profile, validating updater, explicit
regression policy) if the render bench is ever gated in CI or a second
micro-benchmark appears.