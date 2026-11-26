//! Unified error type.

use crate::api::error_code::AsterErrorCode;
use crate::api::response::ApiResponse;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use std::any::Any;

/// Convenience alias for results that fail with the unified [`AsterError`].
pub type Result<T> = std::result::Result<T, AsterError>;

/// Unified application error type.
///
/// `Public` errors carry a caller-chosen status, API code, message and
/// retryability hint that are exposed verbatim; every other variant maps to a
/// fixed internal code, HTTP status, [`AsterErrorCode`] and retryability
/// policy through the accessor methods on this type.
#[derive(Debug, Clone)]
pub enum AsterError {
    /// Publicly constructed error exposing an explicit HTTP status, API
    /// error code, client-facing message and optional retryability hint.
    Public {
        /// Stable internal error code recorded in logs; never exposed in responses.
        internal_code: &'static str,
        /// HTTP status returned to the client.
        status: StatusCode,
        /// Public API error code rendered in the response body.
        code: AsterErrorCode,
        /// Client-facing message rendered in the response body.
        message: String,
        /// Optional hint that the client may retry the same request.
        retryable: Option<bool>,
    },
    /// Database connection failure; internal code E001, mapped to a 500
    /// response with `DatabaseError` and a retryable hint.
    DatabaseConnection(String),
    /// Database operation failure after retries; internal code E002, mapped
    /// to a 500 response with `DatabaseError` and a retryable hint.
    DatabaseOperation(String),
    /// Database operation failure classified by a Forge `DatabaseErrorKind`;
    /// internal code E002, mapped to a 500 response with `DatabaseError` and
    /// a retryable hint.
    DatabaseOperationClassified {
        /// Human-readable failure description.
        message: String,
        /// Classified failure kind used by callers for retry decisions.
        kind: aster_forge_db::DatabaseErrorKind,
    },
    /// Transaction commit outcome could not be observed (connection lost
    /// mid-commit); internal code E002, mapped to a 500 response with
    /// `DatabaseError` and marked non-retryable because replaying is unsafe.
    DatabaseCommitOutcomeUnknown {
        /// Human-readable failure description.
        message: String,
        /// Classified failure kind, when the driver could infer one.
        kind: Option<aster_forge_db::DatabaseErrorKind>,
    },
    /// Configuration loading or lookup failure; internal code E003, mapped
    /// to a 500 response with `ConfigError`.
    ConfigError(String),
    /// Request or domain validation failure; internal code E005, mapped to a
    /// 400 response with `BadRequest`.
    ValidationError(String),
    /// Authentication rejected because credentials did not match; internal
    /// code E010, mapped to a 401 response with `AuthCredentialsFailed`.
    AuthInvalidCredentials(String),
    /// Token failed verification or parsing; internal code E012, mapped to a
    /// 401 response with `AuthTokenInvalid`.
    AuthTokenInvalid(String),
    /// Token is structurally valid but past its validity window; internal
    /// code E011, mapped to a 401 response with `AuthTokenExpired`.
    AuthTokenExpired(String),
    /// Authenticated principal lacks permission for the action; internal
    /// code E013, mapped to a 403 response with `Forbidden`.
    AuthForbidden(String),
    /// Requested record does not exist; internal code E006, mapped to a 404
    /// response with `NotFound`.
    RecordNotFound(String),
    /// External authentication provider flow failed; internal code E020,
    /// mapped to a 400 response with `ExternalAuthError`.
    ExternalAuthError(String),
    /// Mail subsystem has no usable SMTP configuration; internal code E030,
    /// mapped to a 503 response with `MailNotConfigured`, non-retryable.
    MailNotConfigured(String),
    /// Outbound SMTP delivery failed; internal code E031, mapped to a 503
    /// response with `MailDeliveryFailed`, retryable.
    MailDeliveryFailed(String),
    /// Unclassified internal failure; internal code E004, mapped to a 500
    /// response with `InternalServerError`.
    InternalError(String),
}

impl AsterError {
    /// Builds a database connection failure from a raw message.
    pub fn database_connection(message: impl Into<String>) -> Self {
        Self::DatabaseConnection(message.into())
    }

    fn public_error(status: StatusCode, code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::Public {
            internal_code: "E100",
            status,
            code,
            message: message.into(),
            retryable: None,
        }
    }

    fn public_error_with_retryable(
        status: StatusCode,
        code: AsterErrorCode,
        message: impl Into<String>,
        retryable: Option<bool>,
    ) -> Self {
        Self::Public {
            internal_code: "E100",
            status,
            code,
            message: message.into(),
            retryable,
        }
    }

    /// Builds a public 400 validation error carrying a specific AsterErrorCode.
    pub fn validation_error_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error(StatusCode::BAD_REQUEST, code, message)
    }

    /// Builds a public 400 error with the generic `ValidationFailed` code.
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::validation_error_code(AsterErrorCode::ValidationFailed, message)
    }

    /// Builds a public 404 error carrying a specific AsterErrorCode for
    /// missing resources.
    pub fn record_not_found_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error(StatusCode::NOT_FOUND, code, message)
    }

    /// Builds a public 403 error carrying a specific AsterErrorCode for
    /// denied actions.
    pub fn auth_forbidden_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error(StatusCode::FORBIDDEN, code, message)
    }

    /// Builds a public 401 error carrying a specific AsterErrorCode for
    /// unauthenticated requests.
    pub fn auth_unauthorized_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error(StatusCode::UNAUTHORIZED, code, message)
    }

    /// Builds a public 403 error signaling that administrator privileges
    /// are required.
    pub fn auth_admin_required(message: impl Into<String>) -> Self {
        Self::auth_forbidden_code(AsterErrorCode::AuthAdminRequired, message)
    }

    /// Builds a public 403 error for requests that carry no CSRF token.
    pub fn auth_csrf_missing(message: impl Into<String>) -> Self {
        Self::auth_forbidden_code(AsterErrorCode::AuthCsrfMissing, message)
    }

    /// Builds a public 403 error for requests with a mismatched or stale
    /// CSRF token.
    pub fn auth_csrf_invalid(message: impl Into<String>) -> Self {
        Self::auth_forbidden_code(AsterErrorCode::AuthCsrfInvalid, message)
    }

    /// Builds a public 500 error carrying a specific AsterErrorCode for
    /// internal failures.
    pub fn internal_error_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    /// Builds a public 503 error carrying a specific AsterErrorCode for
    /// temporarily unavailable services.
    pub fn service_unavailable_code(code: AsterErrorCode, message: impl Into<String>) -> Self {
        Self::public_error_with_retryable(StatusCode::SERVICE_UNAVAILABLE, code, message, None)
    }

    /// Builds a public 503 `RuntimeUnavailable` error flagged retryable so
    /// clients may poll again later.
    pub fn runtime_unavailable_retryable(message: impl Into<String>) -> Self {
        Self::public_error_with_retryable(
            StatusCode::SERVICE_UNAVAILABLE,
            AsterErrorCode::RuntimeUnavailable,
            message,
            Some(true),
        )
    }

    /// Builds a public 413 error for request bodies exceeding the configured
    /// size limit.
    pub fn request_payload_too_large(message: impl Into<String>) -> Self {
        Self::public_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            AsterErrorCode::RequestPayloadTooLarge,
            message,
        )
    }

    /// Builds a public 429 error emitted when a rate limit budget is
    /// exhausted.
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::public_error(
            StatusCode::TOO_MANY_REQUESTS,
            AsterErrorCode::RateLimited,
            message,
        )
    }

    /// Builds a database operation failure from a raw message.
    pub fn database_operation(message: impl Into<String>) -> Self {
        Self::DatabaseOperation(message.into())
    }

    /// Builds a database operation failure that preserves the Forge
    /// `DatabaseErrorKind` for retry decisions.
    pub fn database_operation_classified(
        message: impl Into<String>,
        kind: aster_forge_db::DatabaseErrorKind,
    ) -> Self {
        Self::DatabaseOperationClassified {
            message: message.into(),
            kind,
        }
    }

    /// Builds an unknown-commit-outcome error, optionally carrying a
    /// classified database failure kind.
    pub fn database_commit_outcome_unknown(
        message: impl Into<String>,
        kind: Option<aster_forge_db::DatabaseErrorKind>,
    ) -> Self {
        Self::DatabaseCommitOutcomeUnknown {
            message: message.into(),
            kind,
        }
    }

    /// Returns the classified database failure kind preserved by this error,
    /// if any.
    pub fn database_error_kind(&self) -> Option<aster_forge_db::DatabaseErrorKind> {
        match self {
            Self::DatabaseOperationClassified { kind, .. } => Some(*kind),
            Self::DatabaseCommitOutcomeUnknown { kind, .. } => *kind,
            _ => None,
        }
    }

    /// Returns true when the transaction commit outcome could not be
    /// observed, so replaying the operation is unsafe.
    pub fn database_commit_outcome_is_unknown(&self) -> bool {
        matches!(self, Self::DatabaseCommitOutcomeUnknown { .. })
    }

    /// Builds a configuration loading or lookup failure from a message.
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError(message.into())
    }

    /// Builds a request or domain validation failure from a message.
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }

    /// Builds a credentials-check failure for login and password flows.
    pub fn auth_invalid_credentials(message: impl Into<String>) -> Self {
        Self::AuthInvalidCredentials(message.into())
    }

    /// Builds a token verification failure for malformed or unverifiable
    /// tokens.
    pub fn auth_token_invalid(message: impl Into<String>) -> Self {
        Self::AuthTokenInvalid(message.into())
    }

    /// Builds a token expiry failure for tokens past their validity window.
    pub fn auth_token_expired(message: impl Into<String>) -> Self {
        Self::AuthTokenExpired(message.into())
    }

    /// Builds a permission failure for authenticated principals lacking
    /// required privileges.
    pub fn auth_forbidden(message: impl Into<String>) -> Self {
        Self::AuthForbidden(message.into())
    }

    /// Builds a public 403 error for accounts that have not completed
    /// activation.
    pub fn auth_pending_activation(message: impl Into<String>) -> Self {
        Self::auth_forbidden_code(AsterErrorCode::AuthPendingActivation, message)
    }

    /// Builds a public 400 error for an invalid contact verification code.
    pub fn contact_verification_invalid(message: impl Into<String>) -> Self {
        Self::validation_error_code(AsterErrorCode::ContactVerificationInvalid, message)
    }

    /// Builds a public 410 error for an expired contact verification code.
    pub fn contact_verification_expired(message: impl Into<String>) -> Self {
        Self::public_error(
            StatusCode::GONE,
            AsterErrorCode::ContactVerificationExpired,
            message,
        )
    }

    /// Builds a public 500 error emitted when avatar rendering fails.
    pub fn avatar_render_failed(message: impl Into<String>) -> Self {
        Self::internal_error_code(AsterErrorCode::AvatarRenderFailed, message)
    }

    /// Builds a missing-record failure from a message.
    pub fn record_not_found(message: impl Into<String>) -> Self {
        Self::RecordNotFound(message.into())
    }

    /// Builds an external authentication provider failure from a message.
    pub fn external_auth_error(message: impl Into<String>) -> Self {
        Self::ExternalAuthError(message.into())
    }

    /// Builds a mail-subsystem failure for missing SMTP configuration.
    pub fn mail_not_configured(message: impl Into<String>) -> Self {
        Self::MailNotConfigured(message.into())
    }

    /// Builds a mail-subsystem failure for failed outbound delivery.
    pub fn mail_delivery_failed(message: impl Into<String>) -> Self {
        Self::MailDeliveryFailed(message.into())
    }

    /// Builds an unclassified internal failure from a message.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }

    /// Returns the stable internal error code (E0xx) used for log
    /// correlation; never exposed in API responses.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Public { internal_code, .. } => internal_code,
            Self::DatabaseConnection(_) => "E001",
            Self::DatabaseOperation(_)
            | Self::DatabaseOperationClassified { .. }
            | Self::DatabaseCommitOutcomeUnknown { .. } => "E002",
            Self::ConfigError(_) => "E003",
            Self::InternalError(_) => "E004",
            Self::ValidationError(_) => "E005",
            Self::RecordNotFound(_) => "E006",
            Self::AuthInvalidCredentials(_) => "E010",
            Self::AuthTokenExpired(_) => "E011",
            Self::AuthTokenInvalid(_) => "E012",
            Self::AuthForbidden(_) => "E013",
            Self::ExternalAuthError(_) => "E020",
            Self::MailNotConfigured(_) => "E030",
            Self::MailDeliveryFailed(_) => "E031",
        }
    }

    /// Maps the error to the public AsterErrorCode rendered in the Forge
    /// response envelope.
    pub fn api_error_code(&self) -> AsterErrorCode {
        match self {
            Self::Public { code, .. } => *code,
            Self::DatabaseConnection(_)
            | Self::DatabaseOperation(_)
            | Self::DatabaseOperationClassified { .. }
            | Self::DatabaseCommitOutcomeUnknown { .. } => AsterErrorCode::DatabaseError,
            Self::ConfigError(_) => AsterErrorCode::ConfigError,
            Self::ValidationError(_) => AsterErrorCode::BadRequest,
            Self::AuthInvalidCredentials(_) => AsterErrorCode::AuthCredentialsFailed,
            Self::AuthTokenInvalid(_) => AsterErrorCode::AuthTokenInvalid,
            Self::AuthTokenExpired(_) => AsterErrorCode::AuthTokenExpired,
            Self::AuthForbidden(_) => AsterErrorCode::Forbidden,
            Self::RecordNotFound(_) => AsterErrorCode::NotFound,
            Self::ExternalAuthError(_) => AsterErrorCode::ExternalAuthError,
            Self::MailNotConfigured(_) => AsterErrorCode::MailNotConfigured,
            Self::MailDeliveryFailed(_) => AsterErrorCode::MailDeliveryFailed,
            Self::InternalError(_) => AsterErrorCode::InternalServerError,
        }
    }

    /// Returns the explicit API error code for `Public` errors, or `None`
    /// when the variant's code is derived from its fixed classification.
    pub fn api_error_code_override(&self) -> Option<AsterErrorCode> {
        match self {
            Self::Public { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Returns the retryability hint exposed to clients: database and mail
    /// delivery failures are retryable, unknown commit outcomes and missing
    /// mail configuration are not, and the remaining variants carry no hint.
    pub fn retryable(&self) -> Option<bool> {
        match self {
            Self::Public { retryable, .. } => *retryable,
            Self::DatabaseConnection(_)
            | Self::DatabaseOperation(_)
            | Self::DatabaseOperationClassified { .. } => Some(true),
            Self::DatabaseCommitOutcomeUnknown { .. } => Some(false),
            Self::MailDeliveryFailed(_) => Some(true),
            Self::MailNotConfigured(_) => Some(false),
            _ => None,
        }
    }

    /// Returns the client-facing message string carried by the error.
    pub fn message(&self) -> &str {
        match self {
            Self::Public { message, .. } => message,
            Self::DatabaseConnection(message)
            | Self::DatabaseOperation(message)
            | Self::ConfigError(message)
            | Self::ValidationError(message)
            | Self::AuthInvalidCredentials(message)
            | Self::AuthTokenInvalid(message)
            | Self::AuthTokenExpired(message)
            | Self::AuthForbidden(message)
            | Self::RecordNotFound(message)
            | Self::ExternalAuthError(message)
            | Self::MailNotConfigured(message)
            | Self::MailDeliveryFailed(message)
            | Self::InternalError(message) => message,
            Self::DatabaseOperationClassified { message, .. }
            | Self::DatabaseCommitOutcomeUnknown { message, .. } => message,
        }
    }

    /// Returns the HTTP status the error maps to when converted into a
    /// response.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Public { status, .. } => *status,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::AuthInvalidCredentials(_)
            | Self::AuthTokenInvalid(_)
            | Self::AuthTokenExpired(_) => StatusCode::UNAUTHORIZED,
            Self::AuthForbidden(_) => StatusCode::FORBIDDEN,
            Self::RecordNotFound(_) => StatusCode::NOT_FOUND,
            Self::ExternalAuthError(_) => StatusCode::BAD_REQUEST,
            Self::MailNotConfigured(_) | Self::MailDeliveryFailed(_) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::DatabaseConnection(_)
            | Self::DatabaseOperation(_)
            | Self::DatabaseOperationClassified { .. }
            | Self::DatabaseCommitOutcomeUnknown { .. }
            | Self::ConfigError(_)
            | Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Display for AsterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl std::error::Error for AsterError {}

impl ResponseError for AsterError {
    fn status_code(&self) -> StatusCode {
        self.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        if status.is_server_error() {
            tracing::error!(
                error = %self,
                internal_code = self.code(),
                api_code = %self.api_error_code(),
                "request failed"
            );
        } else {
            tracing::warn!(
                error = %self,
                internal_code = self.code(),
                api_code = %self.api_error_code(),
                "request failed"
            );
        }
        HttpResponse::build(status).json(ApiResponse::<()>::error_body(
            self.api_error_code(),
            self.message(),
            self.retryable(),
        ))
    }
}

impl From<sea_orm::DbErr> for AsterError {
    fn from(value: sea_orm::DbErr) -> Self {
        let kind = aster_forge_db::database_error_kind(&value);
        let message = value.to_string();
        match kind {
            Some(kind) => Self::database_operation_classified(message, kind),
            None => Self::database_operation(message),
        }
    }
}

impl From<aster_forge_db::DbError> for AsterError {
    fn from(value: aster_forge_db::DbError) -> Self {
        match value {
            aster_forge_db::DbError::DatabaseConnection(message) => {
                Self::database_connection(message)
            }
            aster_forge_db::DbError::DatabaseOperation(message)
            | aster_forge_db::DbError::NonRetryable(message) => Self::database_operation(message),
            aster_forge_db::DbError::DatabaseOperationClassified { message, kind } => {
                Self::database_operation_classified(message, kind)
            }
            aster_forge_db::DbError::CommitOutcomeUnknown { message, kind } => {
                Self::database_commit_outcome_unknown(message, kind)
            }
            aster_forge_db::DbError::RetryExhausted => Self::database_operation("retry exhausted"),
        }
    }
}

impl From<aster_forge_api::ApiError> for AsterError {
    fn from(value: aster_forge_api::ApiError) -> Self {
        Self::validation_error(value.to_string())
    }
}

impl From<aster_forge_utils::UtilsError> for AsterError {
    fn from(value: aster_forge_utils::UtilsError) -> Self {
        Self::internal_error(value.to_string())
    }
}

impl From<aster_forge_config::ConfigCoreError> for AsterError {
    fn from(value: aster_forge_config::ConfigCoreError) -> Self {
        match value {
            aster_forge_config::ConfigCoreError::InvalidValue(message) => {
                Self::validation_error(message)
            }
            aster_forge_config::ConfigCoreError::UnknownKey(key) => {
                Self::record_not_found(format!("config key '{key}'"))
            }
            aster_forge_config::ConfigCoreError::Json(error) => {
                Self::validation_error(error.to_string())
            }
            aster_forge_config::ConfigCoreError::Store(message)
            | aster_forge_config::ConfigCoreError::Notification(message) => {
                Self::internal_error(message)
            }
        }
    }
}

impl From<aster_forge_tasks::TaskCoreError> for AsterError {
    fn from(value: aster_forge_tasks::TaskCoreError) -> Self {
        Self::internal_error(value.to_string())
    }
}

impl From<aster_forge_external_auth::ExternalAuthError> for AsterError {
    fn from(value: aster_forge_external_auth::ExternalAuthError) -> Self {
        match value {
            aster_forge_external_auth::ExternalAuthError::Validation(message) => {
                Self::validation_error(message)
            }
            aster_forge_external_auth::ExternalAuthError::Config(message) => {
                Self::config_error(message)
            }
            aster_forge_external_auth::ExternalAuthError::InvalidCredentials(message) => {
                Self::auth_invalid_credentials(message)
            }
            aster_forge_external_auth::ExternalAuthError::State(message) => {
                Self::database_operation(message)
            }
            aster_forge_external_auth::ExternalAuthError::Internal(message) => {
                Self::internal_error(message)
            }
        }
    }
}

impl From<aster_forge_crypto::CryptoError> for AsterError {
    fn from(value: aster_forge_crypto::CryptoError) -> Self {
        Self::internal_error(value.to_string())
    }
}

impl From<aster_forge_validation::ValidationError> for AsterError {
    fn from(value: aster_forge_validation::ValidationError) -> Self {
        Self::validation_error(value.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AsterError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        match value.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                Self::auth_token_expired("token expired")
            }
            _ => Self::auth_token_invalid(value.to_string()),
        }
    }
}

fn map_display_error<E: std::fmt::Display + 'static>(
    error: E,
    context: Option<&str>,
    mapper: impl FnOnce(String) -> AsterError,
) -> AsterError {
    if let Some(db_error) = (&error as &dyn Any).downcast_ref::<sea_orm::DbErr>()
        && let sea_orm::DbErr::RecordNotFound(message) = db_error
    {
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.clone(),
        };
        return AsterError::record_not_found(message);
    }

    let message = match context {
        Some(context) => format!("{context}: {error}"),
        None => error.to_string(),
    };
    mapper(message)
}

/// Extension trait mapping foreign `Display` errors into [`AsterError`].
///
/// The message-based helpers special-case `sea_orm::DbErr::RecordNotFound`,
/// converting it into a `RecordNotFound` error regardless of the mapper
/// passed, so missing rows surface as 404 responses.
pub trait MapAsterErr<T> {
    /// Maps the error's `Display` representation through the given
    /// constructor.
    fn map_aster_err(self, mapper: impl FnOnce(String) -> AsterError) -> Result<T>;
    /// Like `map_aster_err`, but prefixes the message with `context`.
    fn map_aster_err_ctx(
        self,
        context: &str,
        mapper: impl FnOnce(String) -> AsterError,
    ) -> Result<T>;
    /// Replaces any error with a fixed [`AsterError`] built on demand,
    /// discarding the original message.
    fn map_aster_err_with(self, mapper: impl FnOnce() -> AsterError) -> Result<T>;
}

impl<T, E: std::fmt::Display + 'static> MapAsterErr<T> for std::result::Result<T, E> {
    fn map_aster_err(self, mapper: impl FnOnce(String) -> AsterError) -> Result<T> {
        self.map_err(|error| map_display_error(error, None, mapper))
    }

    fn map_aster_err_ctx(
        self,
        context: &str,
        mapper: impl FnOnce(String) -> AsterError,
    ) -> Result<T> {
        self.map_err(|error| map_display_error(error, Some(context), mapper))
    }

    fn map_aster_err_with(self, mapper: impl FnOnce() -> AsterError) -> Result<T> {
        self.map_err(|_| mapper())
    }
}

/// Formats an arbitrary `Display` value into its string representation.
pub fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Builds a public 400 validation error carrying a specific AsterErrorCode.
pub fn validation_error_with_code(code: AsterErrorCode, message: impl Into<String>) -> AsterError {
    AsterError::validation_error_code(code, message)
}

/// Builds a public 403 forbidden error carrying a specific AsterErrorCode.
pub fn auth_forbidden_with_code(code: AsterErrorCode, message: impl Into<String>) -> AsterError {
    AsterError::auth_forbidden_code(code, message)
}

#[cfg(test)]
mod tests {
    use super::{AsterError, MapAsterErr, display_error};
    use crate::api::error_code::AsterErrorCode;
    use actix_web::ResponseError;
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;

    #[test]
    fn error_variants_map_to_public_codes_statuses_and_retryability() {
        let cases = [
            (
                AsterError::database_connection("db connect"),
                "E001",
                AsterErrorCode::DatabaseError,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(true),
            ),
            (
                AsterError::database_operation("db op"),
                "E002",
                AsterErrorCode::DatabaseError,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(true),
            ),
            (
                AsterError::config_error("config"),
                "E003",
                AsterErrorCode::ConfigError,
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
            (
                AsterError::internal_error("internal"),
                "E004",
                AsterErrorCode::InternalServerError,
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
            (
                AsterError::validation_error("bad"),
                "E005",
                AsterErrorCode::BadRequest,
                StatusCode::BAD_REQUEST,
                None,
            ),
            (
                AsterError::record_not_found("missing"),
                "E006",
                AsterErrorCode::NotFound,
                StatusCode::NOT_FOUND,
                None,
            ),
            (
                AsterError::auth_invalid_credentials("invalid credentials"),
                "E010",
                AsterErrorCode::AuthCredentialsFailed,
                StatusCode::UNAUTHORIZED,
                None,
            ),
            (
                AsterError::auth_token_expired("expired"),
                "E011",
                AsterErrorCode::AuthTokenExpired,
                StatusCode::UNAUTHORIZED,
                None,
            ),
            (
                AsterError::auth_token_invalid("invalid"),
                "E012",
                AsterErrorCode::AuthTokenInvalid,
                StatusCode::UNAUTHORIZED,
                None,
            ),
            (
                AsterError::auth_forbidden("forbidden"),
                "E013",
                AsterErrorCode::Forbidden,
                StatusCode::FORBIDDEN,
                None,
            ),
            (
                AsterError::external_auth_error("external"),
                "E020",
                AsterErrorCode::ExternalAuthError,
                StatusCode::BAD_REQUEST,
                None,
            ),
        ];

        for (error, internal_code, public_code, status, retryable) in cases {
            assert_eq!(error.code(), internal_code);
            assert_eq!(error.api_error_code(), public_code);
            assert_eq!(error.status_code(), status);
            assert_eq!(ResponseError::status_code(&error), status);
            assert_eq!(error.retryable(), retryable);
            assert_eq!(error.to_string(), error.message());
        }
    }

    #[test]
    fn classified_database_errors_preserve_kind_and_commit_retry_boundary() {
        let classified = AsterError::from(aster_forge_db::DbError::database_operation_classified(
            "deadlock",
            aster_forge_db::DatabaseErrorKind::Deadlock,
        ));
        assert_eq!(
            classified.database_error_kind(),
            Some(aster_forge_db::DatabaseErrorKind::Deadlock)
        );
        assert!(!classified.database_commit_outcome_is_unknown());
        assert_eq!(classified.retryable(), Some(true));

        let commit_unknown = AsterError::from(aster_forge_db::DbError::commit_outcome_unknown(
            "connection lost during commit",
            Some(aster_forge_db::DatabaseErrorKind::LockTimeout),
        ));
        assert_eq!(
            commit_unknown.database_error_kind(),
            Some(aster_forge_db::DatabaseErrorKind::LockTimeout)
        );
        assert!(commit_unknown.database_commit_outcome_is_unknown());
        assert_eq!(commit_unknown.retryable(), Some(false));
        assert_eq!(
            commit_unknown.api_error_code(),
            AsterErrorCode::DatabaseError
        );
    }

    #[actix_web::test]
    async fn error_response_exposes_public_code_without_internal_code() {
        let response = AsterError::auth_invalid_credentials("invalid credentials").error_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body()).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value["code"],
            AsterErrorCode::AuthCredentialsFailed.as_str()
        );
        assert!(value.get("internal_code").is_none());
        // auth_invalid_credentials carries no retryability metadata, so the
        // Forge envelope omits the error object entirely.
        assert!(value["error"].is_null());
    }

    #[test]
    fn jsonwebtoken_errors_map_expired_signature_to_expired_token() {
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(b"secret").unwrap();
        let token_error = jsonwebtoken::decode::<serde_json::Value>(
            "not-a-jwt",
            &decoding_key,
            &jsonwebtoken::Validation::default(),
        )
        .unwrap_err();
        let error = AsterError::from(token_error);

        assert_eq!(error.api_error_code(), AsterErrorCode::AuthTokenInvalid);
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn map_aster_err_helpers_preserve_record_not_found_special_case() {
        let db_error: std::result::Result<(), sea_orm::DbErr> =
            Err(sea_orm::DbErr::RecordNotFound("row missing".to_string()));
        let mapped = db_error
            .map_aster_err_ctx("load user", AsterError::database_operation)
            .unwrap_err();

        assert_eq!(mapped.api_error_code(), AsterErrorCode::NotFound);
        assert_eq!(mapped.message(), "load user: row missing");

        let display_error_result: std::result::Result<(), &str> = Err("plain error");
        let mapped = display_error_result
            .map_aster_err(AsterError::config_error)
            .unwrap_err();
        assert_eq!(mapped.api_error_code(), AsterErrorCode::ConfigError);
        assert_eq!(mapped.message(), "plain error");

        let fixed_error_result: std::result::Result<(), &str> = Err("ignored");
        let mapped = fixed_error_result
            .map_aster_err_with(|| AsterError::internal_error("fixed"))
            .unwrap_err();
        assert_eq!(mapped.api_error_code(), AsterErrorCode::InternalServerError);
        assert_eq!(mapped.message(), "fixed");
    }

    #[test]
    fn display_error_uses_display_representation() {
        assert_eq!(display_error("plain"), "plain");
        assert_eq!(display_error(42), "42");
    }
}
