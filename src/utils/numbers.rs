//! Checked numeric conversion helpers.

use std::num::{NonZeroU32, NonZeroU64};

use crate::errors::{AsterError, Result};

fn map_utils_internal(error: aster_forge_utils::UtilsError) -> AsterError {
    AsterError::internal_error(error.to_string())
}

pub fn non_zero_u32(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap_or(NonZeroU32::MIN)
}

pub fn non_zero_u64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap_or(NonZeroU64::MIN)
}

pub fn bytes_to_usize(bytes: i64, value_name: &str) -> Result<usize> {
    aster_forge_utils::numbers::bytes_to_usize(bytes, value_name).map_err(map_utils_internal)
}

pub fn i32_to_usize(value: i32, value_name: &str) -> Result<usize> {
    aster_forge_utils::numbers::i32_to_usize(value, value_name).map_err(map_utils_internal)
}

pub fn i64_to_i32(value: i64, value_name: &str) -> Result<i32> {
    aster_forge_utils::numbers::i64_to_i32(value, value_name).map_err(map_utils_internal)
}

pub fn i64_to_usize(value: i64, value_name: &str) -> Result<usize> {
    aster_forge_utils::numbers::i64_to_usize(value, value_name).map_err(map_utils_internal)
}

pub fn i64_to_u64(value: i64, value_name: &str) -> Result<u64> {
    aster_forge_utils::numbers::i64_to_u64(value, value_name).map_err(map_utils_internal)
}

pub fn u128_to_u64(value: u128, value_name: &str) -> Result<u64> {
    aster_forge_utils::numbers::u128_to_u64(value, value_name).map_err(map_utils_internal)
}

pub fn f64_seconds_to_u64_millis(seconds: f64, value_name: &str) -> Result<u64> {
    if !seconds.is_finite() {
        return Err(AsterError::internal_error(format!(
            "{value_name} must be finite: {seconds}"
        )));
    }
    if seconds < 0.0 {
        return Err(AsterError::internal_error(format!(
            "{value_name} cannot be negative: {seconds}"
        )));
    }

    let duration = std::time::Duration::try_from_secs_f64(seconds).map_err(|_| {
        AsterError::internal_error(format!("{value_name} exceeds duration range: {seconds}"))
    })?;
    let rounded_duration = duration
        .checked_add(std::time::Duration::from_micros(500))
        .ok_or_else(|| {
            AsterError::internal_error(format!("{value_name} exceeds duration range: {seconds}"))
        })?;

    u128_to_u64(rounded_duration.as_millis(), value_name)
}

pub fn u32_to_usize(value: u32, value_name: &str) -> Result<usize> {
    aster_forge_utils::numbers::u32_to_usize(value, value_name).map_err(map_utils_internal)
}

pub fn u32_to_i64(value: u32, value_name: &str) -> Result<i64> {
    aster_forge_utils::numbers::u32_to_i64(value, value_name).map_err(map_utils_internal)
}

pub fn u32_to_i32(value: u32, value_name: &str) -> Result<i32> {
    i32::try_from(value)
        .map_err(|_| AsterError::internal_error(format!("{value_name} exceeds i32 range: {value}")))
}

pub fn u64_to_i64(value: u64, value_name: &str) -> Result<i64> {
    aster_forge_utils::numbers::u64_to_i64(value, value_name).map_err(map_utils_internal)
}

pub fn u64_to_usize(value: u64, value_name: &str) -> Result<usize> {
    aster_forge_utils::numbers::u64_to_usize(value, value_name).map_err(map_utils_internal)
}

pub fn usize_to_i32(value: usize, value_name: &str) -> Result<i32> {
    aster_forge_utils::numbers::usize_to_i32(value, value_name).map_err(map_utils_internal)
}

/// Converts `usize` values such as `Vec::len()` or byte-slice lengths to `i64`.
pub fn usize_to_i64(value: usize, value_name: &str) -> Result<i64> {
    aster_forge_utils::numbers::usize_to_i64(value, value_name).map_err(map_utils_internal)
}

pub fn usize_to_u32(value: usize, value_name: &str) -> Result<u32> {
    aster_forge_utils::numbers::usize_to_u32(value, value_name).map_err(map_utils_internal)
}

pub fn usize_to_u64(value: usize, value_name: &str) -> Result<u64> {
    aster_forge_utils::numbers::usize_to_u64(value, value_name).map_err(map_utils_internal)
}

pub fn calc_total_chunks(total_size: i64, chunk_size: i64, context: &str) -> Result<i32> {
    if total_size < 0 {
        return Err(AsterError::validation_error(format!(
            "{context} total_size cannot be negative: {total_size}"
        )));
    }
    if chunk_size <= 0 {
        return Err(AsterError::internal_error(format!(
            "{context} chunk_size must be positive, got {chunk_size}"
        )));
    }

    let adjusted = total_size.checked_add(chunk_size - 1).ok_or_else(|| {
        AsterError::validation_error(format!("{context} total_size is too large: {total_size}"))
    })?;
    let chunks = adjusted / chunk_size;

    i32::try_from(chunks).map_err(|_| {
        AsterError::validation_error(format!("{context} requires too many chunks: {chunks}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_to_usize_accepts_positive_values() {
        assert_eq!(bytes_to_usize(5_242_880, "chunk_size").unwrap(), 5_242_880);
    }

    #[test]
    fn non_zero_helpers_preserve_positive_values() {
        assert_eq!(non_zero_u32(7).get(), 7);
        assert_eq!(non_zero_u64(9).get(), 9);
    }

    #[test]
    fn non_zero_helpers_fallback_to_min_for_zero() {
        assert_eq!(non_zero_u32(0), NonZeroU32::MIN);
        assert_eq!(non_zero_u64(0), NonZeroU64::MIN);
    }

    #[test]
    fn bytes_to_usize_rejects_negative_values() {
        let err = bytes_to_usize(-1, "chunk_size").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn i32_to_usize_rejects_negative_values() {
        let err = i32_to_usize(-1, "total_chunks").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn i64_to_u64_accepts_positive_values() {
        assert_eq!(i64_to_u64(42, "content_length").unwrap(), 42);
    }

    #[test]
    fn i64_to_u64_rejects_negative_values() {
        let err = i64_to_u64(-1, "content_length").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn f64_seconds_to_u64_millis_rounds_to_nearest_millisecond() {
        assert_eq!(f64_seconds_to_u64_millis(1.2344, "duration").unwrap(), 1234);
        assert_eq!(f64_seconds_to_u64_millis(1.2345, "duration").unwrap(), 1235);
        assert_eq!(f64_seconds_to_u64_millis(0.0004, "duration").unwrap(), 0);
        assert_eq!(f64_seconds_to_u64_millis(0.0005, "duration").unwrap(), 1);
    }

    #[test]
    fn f64_seconds_to_u64_millis_accepts_zero() {
        assert_eq!(f64_seconds_to_u64_millis(0.0, "duration").unwrap(), 0);
    }

    #[test]
    fn f64_seconds_to_u64_millis_rejects_invalid_values() {
        let negative = f64_seconds_to_u64_millis(-1.0, "duration").unwrap_err();
        assert_eq!(negative.code(), "E004");

        let nan = f64_seconds_to_u64_millis(f64::NAN, "duration").unwrap_err();
        assert_eq!(nan.code(), "E004");

        let infinity = f64_seconds_to_u64_millis(f64::INFINITY, "duration").unwrap_err();
        assert_eq!(infinity.code(), "E004");
    }

    #[test]
    fn f64_seconds_to_u64_millis_rejects_u64_millis_overflow() {
        let overflow_seconds = "18446744073709552".parse::<f64>().unwrap();
        let err = f64_seconds_to_u64_millis(overflow_seconds, "duration").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn usize_to_i32_rejects_overflow() {
        let overflow = usize::try_from(i32::MAX)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let err = usize_to_i32(overflow, "uploaded_part_count").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn usize_to_i64_accepts_small_values() {
        assert_eq!(usize_to_i64(1024, "body_len").unwrap(), 1024);
    }

    #[test]
    fn usize_to_u64_accepts_common_values() {
        assert_eq!(usize_to_u64(0, "test").unwrap(), 0);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(usize_to_u64(usize::MAX, "test").unwrap(), u64::MAX);
    }

    #[test]
    fn u64_to_i64_accepts_within_i64_range() {
        assert_eq!(u64_to_i64(0, "test").unwrap(), 0);
        let max_i64_as_u64 = u64::try_from(i64::MAX).unwrap_or(u64::MAX);
        assert_eq!(u64_to_i64(max_i64_as_u64, "test").unwrap(), i64::MAX);
    }

    #[test]
    fn u64_to_i64_rejects_overflow() {
        let overflow = u64::try_from(i64::MAX)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let err = u64_to_i64(overflow, "test").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn u64_to_usize_accepts_within_platform_range() {
        assert_eq!(u64_to_usize(0, "test").unwrap(), 0);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(u64_to_usize(u64::MAX, "test").unwrap(), usize::MAX);
        // on 32-bit this would reject overflow
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn u64_to_usize_rejects_overflow() {
        // u64::MAX won't fit in usize on 32-bit targets
        let err = u64_to_usize(u64::MAX, "cursor_value").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn calc_total_chunks_rounds_up() {
        assert_eq!(
            calc_total_chunks(10_485_761, 5_242_880, "multipart upload").unwrap(),
            3
        );
    }

    #[test]
    fn calc_total_chunks_handles_exact_division() {
        assert_eq!(
            calc_total_chunks(10_485_760, 5_242_880, "multipart upload").unwrap(),
            2
        );
    }

    #[test]
    fn calc_total_chunks_allows_zero_size() {
        assert_eq!(calc_total_chunks(0, 5, "multipart upload").unwrap(), 0);
    }

    #[test]
    fn calc_total_chunks_rejects_negative_total_size() {
        let err = calc_total_chunks(-1, 5, "multipart upload").unwrap_err();
        assert_eq!(err.code(), "E005");
    }

    #[test]
    fn calc_total_chunks_rejects_non_positive_chunk_size() {
        let err = calc_total_chunks(10, 0, "multipart upload").unwrap_err();
        assert_eq!(err.code(), "E004");
    }

    #[test]
    fn calc_total_chunks_rejects_i32_overflow() {
        let overflow_total_size = (i64::from(i32::MAX) + 1) * 5;
        let err = calc_total_chunks(overflow_total_size, 1, "multipart upload").unwrap_err();
        assert_eq!(err.code(), "E005");
    }
}
