use crate::errors::AsterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureErrorKind {
    InvalidTextureType,
    UploadDisabled,
    UserBanForbidden,
    InvalidToken,
    ForbiddenProfile,
    NotFound,
    InvalidContentType,
    MissingFile,
    InvalidPng,
    InvalidDimensions,
    Storage,
}

#[derive(Debug, Clone)]
pub struct TextureError {
    kind: TextureErrorKind,
    detail: Option<String>,
}

impl TextureError {
    pub const fn new(kind: TextureErrorKind) -> Self {
        Self { kind, detail: None }
    }

    pub fn with_detail(kind: TextureErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: Some(detail.into()),
        }
    }

    pub const fn kind(&self) -> TextureErrorKind {
        self.kind
    }

    pub const fn status_code(&self) -> actix_web::http::StatusCode {
        match self.kind {
            // authlib-injector texture upload/delete is an exception to the
            // common Yggdrasil invalid-token rule: this endpoint requires 401
            // for a missing Authorization header or invalid access token.
            TextureErrorKind::InvalidToken => actix_web::http::StatusCode::UNAUTHORIZED,
            TextureErrorKind::ForbiddenProfile
            | TextureErrorKind::UploadDisabled
            | TextureErrorKind::UserBanForbidden => actix_web::http::StatusCode::FORBIDDEN,
            TextureErrorKind::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            TextureErrorKind::InvalidTextureType
            | TextureErrorKind::InvalidContentType
            | TextureErrorKind::MissingFile
            | TextureErrorKind::InvalidPng
            | TextureErrorKind::InvalidDimensions => actix_web::http::StatusCode::BAD_REQUEST,
            TextureErrorKind::Storage => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub const fn protocol_error_name(&self) -> &'static str {
        match self.kind {
            TextureErrorKind::InvalidToken
            | TextureErrorKind::ForbiddenProfile
            | TextureErrorKind::UploadDisabled
            | TextureErrorKind::UserBanForbidden => "ForbiddenOperationException",
            TextureErrorKind::NotFound => "IllegalArgumentException",
            TextureErrorKind::InvalidTextureType
            | TextureErrorKind::InvalidContentType
            | TextureErrorKind::MissingFile
            | TextureErrorKind::InvalidPng
            | TextureErrorKind::InvalidDimensions => "IllegalArgumentException",
            TextureErrorKind::Storage => "InternalServerError",
        }
    }

    pub fn protocol_message(&self) -> String {
        match self.kind {
            TextureErrorKind::InvalidTextureType => "Invalid texture type.".to_string(),
            TextureErrorKind::UploadDisabled => "Texture upload is disabled.".to_string(),
            TextureErrorKind::UserBanForbidden => {
                "Texture upload is restricted for this account.".to_string()
            }
            TextureErrorKind::InvalidToken => "Invalid token.".to_string(),
            TextureErrorKind::ForbiddenProfile => {
                "Profile does not belong to this user.".to_string()
            }
            TextureErrorKind::NotFound => self
                .detail
                .clone()
                .unwrap_or_else(|| "Texture not found.".to_string()),
            TextureErrorKind::InvalidContentType => "Texture file must be image/png.".to_string(),
            TextureErrorKind::MissingFile => "Texture upload file is missing.".to_string(),
            TextureErrorKind::InvalidPng => self
                .detail
                .clone()
                .unwrap_or_else(|| "Invalid PNG texture.".to_string()),
            TextureErrorKind::InvalidDimensions => self
                .detail
                .clone()
                .unwrap_or_else(|| "Invalid texture dimensions.".to_string()),
            TextureErrorKind::Storage => "Object storage failed.".to_string(),
        }
    }
}

impl From<AsterError> for TextureError {
    fn from(value: AsterError) -> Self {
        if value.api_error_code_override()
            == Some(crate::api::error_code::AsterErrorCode::UserBanForbidden)
        {
            tracing::debug!(error = %value, "texture service rejected by user capability ban");
            return Self::new(TextureErrorKind::UserBanForbidden);
        }
        tracing::warn!(error = %value, "texture service failed");
        Self::new(TextureErrorKind::Storage)
    }
}

#[cfg(test)]
mod tests {
    use super::{TextureError, TextureErrorKind};
    use crate::errors::AsterError;

    #[test]
    fn storage_errors_do_not_expose_internal_details_to_clients() {
        let error = TextureError::with_detail(
            TextureErrorKind::Storage,
            "S3 object upload failed: endpoint=https://s3.internal, bucket=private",
        );

        assert_eq!(error.protocol_message(), "Object storage failed.");
    }

    #[test]
    fn aster_errors_are_logged_but_mapped_to_generic_storage_errors() {
        let error = TextureError::from(AsterError::internal_error(
            "S3 object upload failed: source=connection refused",
        ));

        assert_eq!(error.kind(), TextureErrorKind::Storage);
        assert_eq!(error.protocol_message(), "Object storage failed.");
    }

    #[test]
    fn user_ban_errors_keep_their_own_texture_error_kind() {
        let error = TextureError::from(AsterError::auth_forbidden_code(
            crate::api::error_code::AsterErrorCode::UserBanForbidden,
            "user is banned from texture_upload",
        ));

        assert_eq!(error.kind(), TextureErrorKind::UserBanForbidden);
        assert_eq!(
            error.protocol_message(),
            "Texture upload is restricted for this account."
        );
    }
}
