//! Single-point numeric conversion helpers for render rasterization.
//!
//! `aster_forge_utils::numbers` has no f32 pixel helpers yet; these wrappers
//! centralize the saturating f32 -> integer casts used by the render engines so
//! call sites stay cast-free. Candidates for promotion into Forge utils.

/// Floors and clamps into `[0, u32::MAX]` before converting; NaN yields 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Clamped into range first; Rust's float -> int cast then saturates."
)]
pub fn floor_to_u32(value: f32) -> u32 {
    value.floor().clamp(0.0, u32::MAX as f32) as u32
}

/// Ceils and clamps into `[0, u32::MAX]` before converting; NaN yields 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Clamped into range first; Rust's float -> int cast then saturates."
)]
pub fn ceil_to_u32(value: f32) -> u32 {
    value.ceil().clamp(0.0, u32::MAX as f32) as u32
}

/// Rounds and clamps into `[0, 255]` before converting; NaN yields 0.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Clamped into range first; Rust's float -> int cast then saturates."
)]
pub fn round_clamp_to_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::{ceil_to_u32, floor_to_u32, round_clamp_to_u8};

    #[test]
    fn floor_to_u32_clamps_negative_and_nan_to_zero() {
        assert_eq!(floor_to_u32(-1.5), 0);
        assert_eq!(floor_to_u32(f32::NAN), 0);
        assert_eq!(floor_to_u32(2.9), 2);
        assert_eq!(floor_to_u32(u32::MAX as f32 * 4.0), u32::MAX);
    }

    #[test]
    fn ceil_to_u32_clamps_negative_and_nan_to_zero() {
        assert_eq!(ceil_to_u32(-0.1), 0);
        assert_eq!(ceil_to_u32(f32::NAN), 0);
        assert_eq!(ceil_to_u32(2.1), 3);
        assert_eq!(ceil_to_u32(u32::MAX as f32 * 4.0), u32::MAX);
    }

    #[test]
    fn round_clamp_to_u8_stays_in_byte_range() {
        assert_eq!(round_clamp_to_u8(-3.0), 0);
        assert_eq!(round_clamp_to_u8(f32::NAN), 0);
        assert_eq!(round_clamp_to_u8(127.6), 128);
        assert_eq!(round_clamp_to_u8(300.0), 255);
    }
}
