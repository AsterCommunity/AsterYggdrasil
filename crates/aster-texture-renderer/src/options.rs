use image::Rgba;

/// Rendering backend used by the high-level preview API and CLI example.
///
/// The crate keeps engines explicit because the projected 3D preview and the
/// flat 2D texture preview have different option sets and different quality
/// tradeoffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewEngine {
    /// Project the skin onto Minecraft player cuboids and render two views.
    Skin3d,
    /// Render front/back flat player views from the Minecraft skin texture.
    Skin2d,
}

/// Minecraft player geometry variant used by the 3D skin engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinModel {
    /// Steve geometry: 4-pixel-wide arms in the vanilla 64x64 layout.
    Default,
    /// Alex geometry: 3-pixel-wide arms in the vanilla 64x64 layout.
    Slim,
}

/// Built-in quality profiles for the projected 3D skin preview engine.
///
/// Profiles keep the same camera and framing defaults, and only adjust the
/// supersampling level. This makes switching profiles predictable for backend
/// callers that cache preview dimensions or compare images visually.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinPreviewProfile {
    /// Fastest profile. Renders at final resolution without supersampling.
    Fast,
    /// Balanced default profile. Renders internally at 2x and downsamples.
    Default,
    /// Higher quality profile. Renders internally at 3x and downsamples.
    Quality,
}

/// Encoded byte format for preview output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// PNG output, usually preferred for transparent previews.
    Png,
    /// WebP output, useful when smaller previews are more important.
    WebP,
}

impl OutputFormat {
    pub(crate) fn image_format(self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::WebP => image::ImageFormat::WebP,
        }
    }
}

/// Options for the projected 3D skin preview engine.
///
/// The default profile is tuned for a 430x430 transparent dual-view preview:
/// back view on the left, front view on the right, 2x supersampling, and a
/// slight top-down camera angle.
#[derive(Debug, Clone)]
pub struct SkinPreviewOptions {
    /// Final output width in pixels.
    pub output_width: u32,
    /// Final output height in pixels.
    pub output_height: u32,
    /// Orthographic projection scale. Larger values make both players bigger.
    pub scale: f32,
    /// Camera pitch in degrees. Positive values look down at the player.
    pub pitch_degrees: f32,
    /// Yaw in degrees for the right/front player view.
    pub front_yaw_degrees: f32,
    /// Yaw in degrees for the left/back player view.
    pub back_yaw_degrees: f32,
    /// Horizontal spacing in pixels between the two projected views.
    pub view_spacing: f32,
    /// Horizontal offset in pixels applied after automatic centering.
    pub horizontal_offset: f32,
    /// Vertical offset in pixels applied after automatic centering.
    pub vertical_offset: f32,
    /// Vertical center anchor as a ratio of output height.
    pub center_y_ratio: f32,
    /// Optional background color. `None` keeps the output transparent.
    pub background: Option<Rgba<u8>>,
    /// Whether to render second-layer skin cuboids such as hat, jacket, and sleeves.
    pub show_outer_layer: bool,
    /// Integer supersampling factor. `2` renders internally at 2x and downsamples.
    pub supersampling: u8,
}

/// Options for the flat 2D player preview engine.
#[derive(Debug, Clone)]
pub struct Skin2dPreviewOptions {
    /// Final output width in pixels.
    pub output_width: u32,
    /// Final output height in pixels.
    pub output_height: u32,
    /// Minimum empty margin around the centered texture.
    pub padding: u32,
    /// Horizontal spacing in pixels between the front and back views.
    pub view_spacing: u32,
    /// Optional background color. `None` keeps the output transparent.
    pub background: Option<Rgba<u8>>,
    /// Whether to render second-layer skin regions such as hat, jacket, and sleeves.
    pub show_outer_layer: bool,
}

/// Engine-specific options for the high-level preview API.
#[derive(Debug, Clone)]
pub enum TexturePreviewOptions {
    /// Options for [`PreviewEngine::Skin3d`].
    Skin3d(SkinPreviewOptions),
    /// Options for [`PreviewEngine::Skin2d`].
    Skin2d(Skin2dPreviewOptions),
}

impl Default for SkinPreviewOptions {
    fn default() -> Self {
        Self::from_profile(SkinPreviewProfile::Default)
    }
}

impl SkinPreviewOptions {
    /// Build 3D preview options from a built-in quality profile.
    pub fn from_profile(profile: SkinPreviewProfile) -> Self {
        Self {
            output_width: 430,
            output_height: 430,
            scale: 11.5,
            pitch_degrees: 30.0,
            front_yaw_degrees: -45.0,
            back_yaw_degrees: 135.0,
            view_spacing: 35.0,
            horizontal_offset: 0.0,
            vertical_offset: -24.0,
            center_y_ratio: 0.56,
            background: None,
            show_outer_layer: true,
            supersampling: match profile {
                SkinPreviewProfile::Fast => 1,
                SkinPreviewProfile::Default => 2,
                SkinPreviewProfile::Quality => 3,
            },
        }
    }
}

impl Default for Skin2dPreviewOptions {
    fn default() -> Self {
        Self {
            output_width: 430,
            output_height: 430,
            padding: 24,
            view_spacing: 35,
            background: None,
            show_outer_layer: true,
        }
    }
}

impl Default for TexturePreviewOptions {
    fn default() -> Self {
        Self::Skin3d(SkinPreviewOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skin_preview_profiles_only_change_supersampling() {
        let fast = SkinPreviewOptions::from_profile(SkinPreviewProfile::Fast);
        let default = SkinPreviewOptions::from_profile(SkinPreviewProfile::Default);
        let quality = SkinPreviewOptions::from_profile(SkinPreviewProfile::Quality);

        assert_eq!(SkinPreviewOptions::default().supersampling, 2);
        assert_eq!(fast.supersampling, 1);
        assert_eq!(default.supersampling, 2);
        assert_eq!(quality.supersampling, 3);

        assert_eq!(fast.output_width, default.output_width);
        assert_eq!(quality.output_height, default.output_height);
        assert_eq!(fast.pitch_degrees, default.pitch_degrees);
        assert_eq!(quality.front_yaw_degrees, default.front_yaw_degrees);
        assert_eq!(fast.background, default.background);
    }
}
