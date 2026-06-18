use std::{env, fs, path::PathBuf};

use aster_texture_renderer::{
    OutputFormat, PreviewEngine, RenderError, Skin2dPreviewOptions, SkinModel, SkinPreviewOptions,
    SkinPreviewProfile, TexturePreviewOptions, render_preview_bytes,
};
use image::Rgba;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(env::args().skip(1))?;
    let skin = fs::read(&args.input)?;
    let bytes = render_preview_bytes(&skin, args.model, &args.options, args.format)
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;
    fs::write(&args.output, bytes)?;
    Ok(())
}

struct Args {
    input: PathBuf,
    output: PathBuf,
    model: SkinModel,
    format: OutputFormat,
    options: TexturePreviewOptions,
}

impl Args {
    fn parse<I>(mut args: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut input = None;
        let mut output = None;
        let mut engine = PreviewEngine::Skin3d;
        let mut model = SkinModel::Default;
        let mut format = OutputFormat::Png;
        let mut skin_3d_options = SkinPreviewOptions::default();
        let mut skin_2d_options = Skin2dPreviewOptions::default();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input" => input = Some(PathBuf::from(next_value(&mut args, "--input")?)),
                "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
                "--engine" => {
                    engine = parse_engine(&next_value(&mut args, "--engine")?)?;
                }
                "--profile" => {
                    let (next_skin_3d_options, next_skin_2d_options) =
                        match parse_profile(&next_value(&mut args, "--profile")?)? {
                            SkinPreviewProfile::Fast => (
                                SkinPreviewOptions::from_profile(SkinPreviewProfile::Fast),
                                Skin2dPreviewOptions::default(),
                            ),
                            SkinPreviewProfile::Default => (
                                SkinPreviewOptions::from_profile(SkinPreviewProfile::Default),
                                Skin2dPreviewOptions::default(),
                            ),
                            SkinPreviewProfile::Quality => (
                                SkinPreviewOptions::from_profile(SkinPreviewProfile::Quality),
                                Skin2dPreviewOptions::default(),
                            ),
                        };
                    skin_3d_options = next_skin_3d_options;
                    skin_2d_options = next_skin_2d_options;
                }
                "--model" => {
                    model = match next_value(&mut args, "--model")?.as_str() {
                        "default" => SkinModel::Default,
                        "slim" => SkinModel::Slim,
                        value => return Err(format!("unsupported model: {value}")),
                    };
                }
                "--format" => {
                    format = match next_value(&mut args, "--format")?.as_str() {
                        "png" => OutputFormat::Png,
                        "webp" => OutputFormat::WebP,
                        value => return Err(format!("unsupported format: {value}")),
                    };
                }
                "--width" => {
                    let width = parse_u32(&mut args, "--width")?;
                    skin_3d_options.output_width = width;
                    skin_2d_options.output_width = width;
                }
                "--height" => {
                    let height = parse_u32(&mut args, "--height")?;
                    skin_3d_options.output_height = height;
                    skin_2d_options.output_height = height;
                }
                "--scale" => skin_3d_options.scale = parse_f32(&mut args, "--scale")?,
                "--pitch" => skin_3d_options.pitch_degrees = parse_f32(&mut args, "--pitch")?,
                "--front-yaw" => {
                    skin_3d_options.front_yaw_degrees = parse_f32(&mut args, "--front-yaw")?;
                }
                "--back-yaw" => {
                    skin_3d_options.back_yaw_degrees = parse_f32(&mut args, "--back-yaw")?;
                }
                "--spacing" => {
                    skin_3d_options.view_spacing = parse_f32(&mut args, "--spacing")?;
                    skin_2d_options.view_spacing = parse_u32_value(skin_3d_options.view_spacing)?;
                }
                "--supersampling" => {
                    skin_3d_options.supersampling = parse_u8(&mut args, "--supersampling")?;
                }
                "--padding" => {
                    skin_2d_options.padding = parse_u32(&mut args, "--padding")?;
                }
                "--x-offset" => {
                    skin_3d_options.horizontal_offset = parse_f32(&mut args, "--x-offset")?;
                }
                "--y-offset" => {
                    skin_3d_options.vertical_offset = parse_f32(&mut args, "--y-offset")?;
                }
                "--center-y" => {
                    skin_3d_options.center_y_ratio = parse_f32(&mut args, "--center-y")?;
                }
                "--background" => {
                    let background = parse_background(&next_value(&mut args, "--background")?)?;
                    skin_3d_options.background = background;
                    skin_2d_options.background = background;
                }
                "--transparent" => {
                    skin_3d_options.background = None;
                    skin_2d_options.background = None;
                }
                "--no-outer-layer" => {
                    skin_3d_options.show_outer_layer = false;
                    skin_2d_options.show_outer_layer = false;
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown argument: {value}\n\n{}", usage())),
            }
        }

        let options = match engine {
            PreviewEngine::Skin3d => TexturePreviewOptions::Skin3d(skin_3d_options),
            PreviewEngine::Skin2d => TexturePreviewOptions::Skin2d(skin_2d_options),
        };

        Ok(Self {
            input: input.ok_or_else(usage)?,
            output: output.ok_or_else(usage)?,
            model,
            format,
            options,
        })
    }
}

fn parse_engine(value: &str) -> Result<PreviewEngine, String> {
    match value {
        "skin-3d" | "3d" => Ok(PreviewEngine::Skin3d),
        "skin-2d" | "2d" => Ok(PreviewEngine::Skin2d),
        value => Err(format!("unsupported engine: {value}")),
    }
}

fn parse_profile(value: &str) -> Result<SkinPreviewProfile, String> {
    match value {
        "fast" => Ok(SkinPreviewProfile::Fast),
        "default" => Ok(SkinPreviewProfile::Default),
        "quality" => Ok(SkinPreviewProfile::Quality),
        value => Err(format!("unsupported profile: {value}")),
    }
}

fn next_value<I>(args: &mut I, name: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for argument {name}"))
}

fn parse_f32<I>(args: &mut I, name: &str) -> Result<f32, String>
where
    I: Iterator<Item = String>,
{
    next_value(args, name)?
        .parse()
        .map_err(|_| format!("invalid float for argument {name}"))
}

fn parse_u32<I>(args: &mut I, name: &str) -> Result<u32, String>
where
    I: Iterator<Item = String>,
{
    next_value(args, name)?
        .parse()
        .map_err(|_| format!("invalid integer for argument {name}"))
}

fn parse_u8<I>(args: &mut I, name: &str) -> Result<u8, String>
where
    I: Iterator<Item = String>,
{
    next_value(args, name)?
        .parse()
        .map_err(|_| format!("invalid integer for argument {name}"))
}

fn parse_u32_value(value: f32) -> Result<u32, String> {
    if value < 0.0 || value > u32::MAX as f32 {
        return Err("spacing must fit in u32".to_string());
    }
    Ok(value.round() as u32)
}

fn parse_background(value: &str) -> Result<Option<Rgba<u8>>, String> {
    match value {
        "transparent" | "none" => Ok(None),
        "white" => Ok(Some(Rgba([255, 255, 255, 255]))),
        "black" => Ok(Some(Rgba([0, 0, 0, 255]))),
        _ => parse_hex_background(value).map(Some),
    }
}

fn parse_hex_background(value: &str) -> Result<Rgba<u8>, String> {
    let hex = value.strip_prefix('#').ok_or_else(|| {
        "background must be transparent, white, black, #RRGGBB, or #RRGGBBAA".to_string()
    })?;
    if hex.len() != 6 && hex.len() != 8 {
        return Err("background hex must be #RRGGBB or #RRGGBBAA".to_string());
    }

    let red = parse_hex_byte(hex, 0)?;
    let green = parse_hex_byte(hex, 2)?;
    let blue = parse_hex_byte(hex, 4)?;
    let alpha = if hex.len() == 8 {
        parse_hex_byte(hex, 6)?
    } else {
        255
    };
    Ok(Rgba([red, green, blue, alpha]))
}

fn parse_hex_byte(hex: &str, start: usize) -> Result<u8, String> {
    u8::from_str_radix(&hex[start..start + 2], 16)
        .map_err(|_| "background hex contains invalid digits".to_string())
}

fn usage() -> String {
    "usage: render_skin_preview --input <skin.png> --output <preview.png> [--engine skin-3d|skin-2d] [--profile fast|default|quality] [--model default|slim] [--format png|webp] [--pitch deg] [--front-yaw deg] [--back-yaw deg] [--supersampling 1-4] [--padding px] [--background transparent|white|black|#RRGGBB|#RRGGBBAA]".to_string()
}

#[allow(dead_code)]
fn _assert_error_send_sync(error: RenderError) -> RenderError {
    error
}
