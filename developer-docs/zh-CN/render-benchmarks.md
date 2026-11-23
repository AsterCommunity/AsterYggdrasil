# 渲染基准基线

皮肤预览由 `crates/aster_yggdrasil_render` 渲染，位于 profile 和 wardrobe 预览的请求路径上，渲染引擎、投影参数或 PNG 编码的改动都可能拖慢用户可见延迟。本文记录一次参考运行，以及后续改动如何与它对照。

基准入口是 `tests/benchmarks/render_skin_preview.rs`。每轮迭代渲染一张确定性的 64x64 slim 皮肤并编码为 PNG，因此计时包含 PNG 解码、投影/光栅化、降采样和 PNG 编码。

| 基准 | 测量内容 |
| --- | --- |
| `render_skin_preview/skin_3d_png` | 3D 双视角投影预览，`SkinPreviewOptions::default()`（430x430，`SkinPreviewProfile::Default`，2x 超采样） |
| `render_skin_preview/skin_2d_png` | 2D 平铺正反面预览，`Skin2dPreviewOptions::default()`（430x430） |
| `render_skin_preview_profiles/skin_3d_png_fast` | 同一 3D 预览，`SkinPreviewProfile::Fast`（1x） |
| `render_skin_preview_profiles/skin_3d_png_default` | 同一 3D 预览，`SkinPreviewProfile::Default`（2x） |
| `render_skin_preview_profiles/skin_3d_png_quality` | 同一 3D 预览，`SkinPreviewProfile::Quality`（3x） |

## 如何运行

```bash
# 记录一次运行；本文使用的基线名为 2026-11-23。
cargo bench --features benchmarks --bench render_skin_preview -- --save-baseline 2026-11-23

# 后续改动与基线对照。
cargo bench --features benchmarks --bench render_skin_preview -- --baseline 2026-11-23
```

必须带 `--bench render_skin_preview`。直接跑 `cargo bench` 会同时把 lib target 的单元测试 harness 当作 bench 目标，criterion 参数会被传给 libtest，报 `error: Unrecognized option: 'save-baseline'`。

criterion 把原始测量和图表写到 `target/criterion/`。输入皮肤使用固定 RNG 种子，重复运行会渲染完全相同的 PNG 字节（当前种子下为 16,516 字节）；不要再改回随机输入，否则历史基线不再可比。

## 参考运行环境

| 项 | 值 |
| --- | --- |
| 日期 | 2026-11-23 |
| 版本 | `1d80b94`（`master`）；本次基线没有改动 render crate |
| 机器 | ASUS TUF Gaming FA608UM（笔记本） |
| CPU | AMD Ryzen 7 H 260，8 核 / 16 线程，标称 3.8 GHz |
| 内存 | 31.3 GiB |
| 系统 | Windows 11（10.0.26300）x86_64 |
| 工具链 | `rustc 1.100.0 (0ef1bc38)`、`cargo 1.100.0` |
| Cargo profile | `bench`（继承 `release`）：`opt-level = 2`、启用 LTO、`codegen-units = 1` |
| 电源方案 | Windows 电源方案 "Silent"（ASUS 静音降频档） |
| 输入 | 64x64 RGBA slim 皮肤，像素确定，PNG，16,516 字节 |

## 结果

| 基准 | 中位数（95% CI） | 均值 | 吞吐 |
| --- | --- | --- | --- |
| `render_skin_preview/skin_3d_png` | 75.42 ms（74.64 - 76.07 ms） | 75.63 ms | 213.25 KiB/s |
| `render_skin_preview/skin_2d_png` | 1.87 ms（1.85 - 1.89 ms） | 1.91 ms | 8.19 MiB/s |
| `render_skin_preview_profiles/skin_3d_png_fast` | 20.51 ms（20.18 - 20.76 ms） | 20.70 ms | - |
| `render_skin_preview_profiles/skin_3d_png_default` | 76.41 ms（75.76 - 76.87 ms） | 77.13 ms | - |
| `render_skin_preview_profiles/skin_3d_png_quality` | 165.90 ms（164.87 - 166.83 ms） | 167.11 ms | - |

吞吐按输入 PNG 大小计算。同样输出 430x430 时，3D 路径约为 2D 路径的 40 倍，符合“投影 + 光栅化十二个立方体面并渲染两遍”的额外工作量。

## 超采样档位缩放

| 档位 | 超采样 | 中位数 | 相对 `Fast` 成本 |
| --- | --- | --- | --- |
| `Fast` | 1x | 20.51 ms | 1.0x |
| `Default` | 2x | 76.41 ms | 3.7x |
| `Quality` | 3x | 165.90 ms | 8.1x |

光栅化像素量按倍率平方增长（1x、4x、9x），实测耗时增长是 1x、3.7x、8.1x：PNG 解码/编码和最后一次降采样是固定成本，所以略低于平方增长。`Quality` 在同样 430x430 输出下约为 `Default` 的 2.2 倍，服务端默认仍应保持 `Default`。

## 可比性规则

- 只在同机器、同电源方案、同工具链下用 `--baseline 2026-11-23` 对照；绝对数字不能跨机器比较。
- 参考机器的重复运行波动约在 ±4% 以内（多数用例 ±2%），小于 5% 的差异视为噪声。
- 参考运行用的是笔记本的 `Silent` 降频档，这批数字是下界。做容量规划前应在高性能电源方案下复测，并把电源方案写进任何新基线。
- 基准只覆盖一张固定输入。真实皮肤的压缩特征不同，PNG 解码/编码成本会随内容变化。