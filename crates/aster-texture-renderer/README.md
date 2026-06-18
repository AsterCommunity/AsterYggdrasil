# aster-texture-renderer

Static Minecraft texture preview renderer for AsterYggdrasil.

This crate is intentionally standalone. It does not depend on the main server
crate and does not know about storage, HTTP, database rows, cache keys, or
Yggdrasil response shapes.

## Scope

- Pure software rendering with the `image` crate.
- Minecraft skin preview as two projected player models.
- Minecraft skin preview as a front/back 2D paper-doll image.
- Adjustable output size, scale, pitch, front/back yaw, spacing, offsets, and
  background.
- Integer supersampling for softer projected edges without GPU dependencies.
- Transparent output by default, with optional solid or RGBA background.
- Supports default/slim arms, 64x64 skins, 64x32 legacy skins, and HD skins
  whose dimensions are multiples of the vanilla layout.
- Encodes PNG or WebP bytes.

## Non-goals for this crate

- Interactive 3D preview.
- GPU, OpenGL, WebGPU, OSMesa, or native windowing dependencies.
- Backend route/service integration.
- Texture storage or cache policy.

## Examples

The default profile renders a transparent 430x430 dual-view preview with 2x
supersampling:

```bash
cargo run -p aster-texture-renderer --example render_skin_preview -- \
  --input textures/example.png \
  --output /tmp/preview.png \
  --engine skin-3d \
  --profile default \
  --model slim
```

3D profiles trade CPU cost for antialiasing quality while keeping the same
camera and framing:

| Profile | Supersampling | Use case |
| --- | ---: | --- |
| `fast` | 1x | high-throughput background jobs |
| `default` | 2x | balanced backend previews |
| `quality` | 3x | slower high-quality exports |

Use the 2D engine when you need a flat front/back player preview instead of the
projected 3D preview:

```bash
cargo run -p aster-texture-renderer --example render_skin_preview -- \
  --input textures/example.png \
  --output /tmp/preview-2d.png \
  --engine skin-2d \
  --profile default \
  --background white \
  --padding 24
```

Use `--background` when a solid preview is easier to inspect:

```bash
cargo run -p aster-texture-renderer --example render_skin_preview -- \
  --input textures/example.png \
  --output /tmp/preview-white.png \
  --engine skin-3d \
  --profile default \
  --model slim \
  --background white
```

Supported background values are `transparent`, `none`, `white`, `black`,
`#RRGGBB`, and `#RRGGBBAA`.

## Recommended Backend API

For backend batch rendering, prefer `DecodedSkin` plus `RenderWorkspace`.
Decode each source skin once and reuse one workspace per worker. This avoids
repeated PNG decoding and keeps the reusable output/supersampling buffers hot:

```rust
use aster_texture_renderer::{
    DecodedSkin, RenderWorkspace, SkinModel, SkinPreviewOptions,
    SkinPreviewProfile, TexturePreviewOptions, render_decoded_preview_with_workspace,
};

let decoded = DecodedSkin::from_png_bytes(skin_png)?;
let options = TexturePreviewOptions::Skin3d(SkinPreviewOptions::from_profile(
    SkinPreviewProfile::Default,
));
let mut workspace = RenderWorkspace::new();

let preview =
    render_decoded_preview_with_workspace(&decoded, SkinModel::Slim, &options, &mut workspace)?;
```

The profile can be overridden with explicit render options. Put `--profile`
before overrides so later flags win:

```bash
cargo run -p aster-texture-renderer --example render_skin_preview -- \
  --input textures/example.png \
  --output /tmp/preview-custom.webp \
  --engine skin-3d \
  --profile default \
  --model slim \
  --format webp \
  --width 512 \
  --height 512 \
  --scale 12 \
  --pitch 30 \
  --front-yaw -45 \
  --back-yaw 135 \
  --spacing 35 \
  --x-offset 0 \
  --y-offset -24 \
  --center-y 0.56 \
  --supersampling 2 \
  --background '#ffffffff'
```

Supported example flags:

- `--engine skin-3d|skin-2d`
- `--profile fast|default|quality`
- `--model default|slim`
- `--format png|webp`
- `--width <px>` / `--height <px>`
- `--scale <number>`
- `--pitch <degrees>`
- `--front-yaw <degrees>` / `--back-yaw <degrees>`
- `--spacing <px>`
- `--x-offset <px>` / `--y-offset <px>`
- `--center-y <ratio>`
- `--supersampling 1..4`
- `--padding <px>`
- `--background transparent|none|white|black|#RRGGBB|#RRGGBBAA`
- `--transparent`
- `--no-outer-layer`

Profiling tests are ignored by default. Run them manually with output enabled:

```bash
cargo test -p aster-texture-renderer --test render_profile -- --ignored --nocapture
```
