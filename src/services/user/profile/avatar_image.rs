//! Avatar upload decoding and rendering helpers.

use std::io::Cursor;

use actix_multipart::Multipart;
use futures::StreamExt;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Limits};

use crate::api::error_code::AsterErrorCode;
use crate::errors::{AsterError, MapAsterErr, Result};

use super::shared::{AVATAR_SIZE_LG, AVATAR_SIZE_SM, MAX_AVATAR_DECODE_ALLOC};

pub(super) const AVATAR_MAX_UPLOAD_SIZE: usize = 5 * 1024 * 1024;

pub(super) struct AvatarUploadData {
    pub bytes: Vec<u8>,
}

pub(super) struct ProcessedAvatar {
    pub small_bytes: Vec<u8>,
    pub large_bytes: Vec<u8>,
}

pub(super) async fn read_avatar_upload(payload: &mut Multipart) -> Result<AvatarUploadData> {
    let mut bytes = Vec::new();
    let mut saw_file = false;
    let mut field_count = 0_u64;
    tracing::debug!("reading avatar upload multipart payload");

    while let Some(field) = payload.next().await {
        field_count += 1;
        let mut field = field.map_aster_err(|message| {
            AsterError::validation_error_code(AsterErrorCode::AvatarUploadReadFailed, message)
        })?;
        let has_file_name = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());

        if !has_file_name {
            tracing::debug!(field_count, "draining non-file avatar multipart field");
            while let Some(chunk) = field.next().await {
                chunk.map_aster_err(|message| {
                    AsterError::validation_error_code(
                        AsterErrorCode::AvatarUploadReadFailed,
                        message,
                    )
                })?;
            }
            continue;
        }

        saw_file = true;
        tracing::debug!(field_count, "reading avatar file multipart field");
        while let Some(chunk) = field.next().await {
            let chunk = chunk.map_aster_err(|message| {
                AsterError::validation_error_code(AsterErrorCode::AvatarUploadReadFailed, message)
            })?;
            if bytes.len() + chunk.len() > AVATAR_MAX_UPLOAD_SIZE {
                tracing::debug!(
                    field_count,
                    current_bytes = bytes.len(),
                    chunk_bytes = chunk.len(),
                    max_bytes = AVATAR_MAX_UPLOAD_SIZE,
                    "avatar upload rejected because payload is too large"
                );
                return Err(AsterError::request_payload_too_large(format!(
                    "avatar upload exceeds {AVATAR_MAX_UPLOAD_SIZE} bytes"
                )));
            }
            bytes.extend_from_slice(&chunk);
        }
        break;
    }

    if !saw_file || bytes.is_empty() {
        tracing::debug!(
            field_count,
            saw_file,
            bytes = bytes.len(),
            "avatar upload rejected because file is missing or empty"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AvatarFileRequired,
            "avatar file is required",
        ));
    }

    tracing::debug!(
        field_count,
        bytes = bytes.len(),
        "read avatar upload multipart payload"
    );
    Ok(AvatarUploadData { bytes })
}

pub(super) async fn process_avatar_upload(bytes: Vec<u8>) -> Result<ProcessedAvatar> {
    tracing::debug!(bytes = bytes.len(), "processing avatar upload");
    tokio::task::spawn_blocking(move || render_avatar_variants(bytes))
        .await
        .map_aster_err_ctx("process avatar image task", AsterError::internal_error)?
}

fn render_avatar_variants(bytes: Vec<u8>) -> Result<ProcessedAvatar> {
    tracing::debug!(bytes = bytes.len(), "rendering avatar variants");
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| {
            AsterError::validation_error_code(
                AsterErrorCode::AvatarEmptyImage,
                format!("avatar image format cannot be detected: {error}"),
            )
        })?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_AVATAR_DECODE_ALLOC);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| {
        AsterError::validation_error_code(
            AsterErrorCode::AvatarEmptyImage,
            format!("avatar image cannot be decoded: {error}"),
        )
    })?;

    let square = center_square(image)?;
    let large = square.resize_exact(AVATAR_SIZE_LG, AVATAR_SIZE_LG, FilterType::Triangle);
    let small = square.resize_exact(AVATAR_SIZE_SM, AVATAR_SIZE_SM, FilterType::Triangle);

    let processed = ProcessedAvatar {
        small_bytes: encode_webp(&small)?,
        large_bytes: encode_webp(&large)?,
    };
    tracing::debug!(
        small_bytes = processed.small_bytes.len(),
        large_bytes = processed.large_bytes.len(),
        "rendered avatar variants"
    );
    Ok(processed)
}

fn center_square(image: DynamicImage) -> Result<DynamicImage> {
    let (width, height) = image.dimensions();
    tracing::debug!(width, height, "cropping avatar image to square");
    if width == 0 || height == 0 {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AvatarEmptyImage,
            "avatar image is empty",
        ));
    }

    let side = width.min(height);
    let x = (width - side) / 2;
    let y = (height - side) / 2;
    Ok(image.crop_imm(x, y, side, side))
}

fn encode_webp(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)
        .map_err(|error| {
            AsterError::avatar_render_failed(format!("failed to encode avatar webp: {error}"))
        })?;
    validate_webp(&bytes)?;
    tracing::debug!(bytes = bytes.len(), "encoded avatar webp variant");
    Ok(bytes)
}

fn validate_webp(bytes: &[u8]) -> Result<()> {
    let reader = ImageReader::new(Cursor::new(bytes));
    reader
        .with_guessed_format()
        .map_err(|error| {
            AsterError::internal_error_code(
                AsterErrorCode::AvatarOutputInvalid,
                format!("failed to inspect avatar output: {error}"),
            )
        })?
        .decode()
        .map_err(|error| {
            AsterError::internal_error_code(
                AsterErrorCode::AvatarOutputInvalid,
                format!("avatar output is invalid: {error}"),
            )
        })?;
    Ok(())
}
