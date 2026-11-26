//! Yggdrasil protocol error state machine.
//!
//! Every protocol failure maps to a stable wire error name (for example
//! `ForbiddenOperationException`), a protocol message, and an HTTP status, as
//! consumed by launchers and authlib-injector. Host applications convert their
//! own internal errors into these kinds; conversion of infrastructure errors
//! (including redaction policy) stays with the host.

use http::StatusCode;

/// The stable protocol failure kinds served by authserver and sessionserver.
///
/// Each kind maps to a fixed wire error name and HTTP status through the
/// accessors on [`YggdrasilError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YggdrasilErrorKind {
    /// Access token is unknown, expired, or revoked for the operation.
    InvalidToken,
    /// Credentials were rejected (wrong password, or client token mismatch).
    InvalidCredentials,
    /// The access token already has a profile assigned to it.
    AccessTokenAlreadyHasProfile,
    /// The targeted profile is not owned by the authenticated user.
    ForbiddenProfile,
    /// Request arguments are invalid; [`YggdrasilError::with_detail`] supplies
    /// the client-facing message.
    BadRequest,
    /// The `agent` product in an authenticate request is not supported.
    UnsupportedAgent,
    /// More profile names were requested in one batch than allowed.
    TooManyProfilesRequested,
    /// No profile matches the requested identifier; served as 204 No Content.
    ProfileNotFound,
    /// Unexpected server-side failure; the detail never carries secrets to clients.
    Internal,
}

/// A protocol error carrying a [`YggdrasilErrorKind`] plus an optional
/// host-provided detail used as the client-facing message.
#[derive(Debug, Clone)]
pub struct YggdrasilError {
    kind: YggdrasilErrorKind,
    detail: Option<String>,
}

impl YggdrasilError {
    /// Creates an error of `kind` without an extra protocol message detail.
    pub const fn new(kind: YggdrasilErrorKind) -> Self {
        Self { kind, detail: None }
    }

    /// Creates an error whose detail overrides the default protocol message
    /// (used by kinds like [`YggdrasilErrorKind::BadRequest`] and
    /// [`YggdrasilErrorKind::Internal`]).
    pub fn with_detail(kind: YggdrasilErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: Some(detail.into()),
        }
    }

    /// Returns the stable error kind.
    pub const fn kind(&self) -> YggdrasilErrorKind {
        self.kind
    }

    /// Returns the HTTP status the protocol assigns to this kind
    /// (403, 400, 204, or 500); a 204 body stays empty.
    pub const fn status_code(&self) -> StatusCode {
        match self.kind {
            YggdrasilErrorKind::InvalidToken
            | YggdrasilErrorKind::InvalidCredentials
            | YggdrasilErrorKind::ForbiddenProfile => StatusCode::FORBIDDEN,
            YggdrasilErrorKind::AccessTokenAlreadyHasProfile
            | YggdrasilErrorKind::BadRequest
            | YggdrasilErrorKind::UnsupportedAgent
            | YggdrasilErrorKind::TooManyProfilesRequested => StatusCode::BAD_REQUEST,
            YggdrasilErrorKind::ProfileNotFound => StatusCode::NO_CONTENT,
            YggdrasilErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns the wire `error` name (`ForbiddenOperationException`,
    /// `IllegalArgumentException`, `NotFound`, or `InternalServerError`).
    pub const fn protocol_error_name(&self) -> &'static str {
        match self.kind {
            YggdrasilErrorKind::AccessTokenAlreadyHasProfile
            | YggdrasilErrorKind::BadRequest
            | YggdrasilErrorKind::UnsupportedAgent
            | YggdrasilErrorKind::TooManyProfilesRequested => "IllegalArgumentException",
            YggdrasilErrorKind::InvalidToken
            | YggdrasilErrorKind::InvalidCredentials
            | YggdrasilErrorKind::ForbiddenProfile => "ForbiddenOperationException",
            YggdrasilErrorKind::ProfileNotFound => "NotFound",
            YggdrasilErrorKind::Internal => "InternalServerError",
        }
    }

    /// Returns the wire `errorMessage`: the kind's canonical protocol text,
    /// or the stored detail when the kind echoes host context.
    pub fn protocol_message(&self) -> String {
        match self.kind {
            YggdrasilErrorKind::InvalidToken => "Invalid token.".to_string(),
            YggdrasilErrorKind::InvalidCredentials => {
                "Invalid credentials. Invalid username or password.".to_string()
            }
            YggdrasilErrorKind::AccessTokenAlreadyHasProfile => {
                "Access token already has a profile assigned.".to_string()
            }
            YggdrasilErrorKind::ForbiddenProfile => {
                "Profile does not belong to this user.".to_string()
            }
            YggdrasilErrorKind::BadRequest => self
                .detail
                .clone()
                .unwrap_or_else(|| "Invalid request.".to_string()),
            YggdrasilErrorKind::UnsupportedAgent => "Unsupported Yggdrasil agent.".to_string(),
            YggdrasilErrorKind::TooManyProfilesRequested => {
                "Too many profile names requested.".to_string()
            }
            YggdrasilErrorKind::ProfileNotFound => "Profile not found.".to_string(),
            YggdrasilErrorKind::Internal => self
                .detail
                .clone()
                .unwrap_or_else(|| "Internal server error.".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{YggdrasilError, YggdrasilErrorKind};
    use http::StatusCode;

    #[test]
    fn forbidden_kinds_map_to_403_and_forbidden_operation_exception() {
        for kind in [
            YggdrasilErrorKind::InvalidToken,
            YggdrasilErrorKind::InvalidCredentials,
            YggdrasilErrorKind::ForbiddenProfile,
        ] {
            let error = YggdrasilError::new(kind);
            assert_eq!(error.status_code(), StatusCode::FORBIDDEN);
            assert_eq!(error.protocol_error_name(), "ForbiddenOperationException");
        }
    }

    #[test]
    fn bad_request_kinds_map_to_400_and_illegal_argument_exception() {
        for kind in [
            YggdrasilErrorKind::AccessTokenAlreadyHasProfile,
            YggdrasilErrorKind::BadRequest,
            YggdrasilErrorKind::UnsupportedAgent,
            YggdrasilErrorKind::TooManyProfilesRequested,
        ] {
            let error = YggdrasilError::new(kind);
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
            assert_eq!(error.protocol_error_name(), "IllegalArgumentException");
        }
    }

    #[test]
    fn profile_not_found_maps_to_204_and_not_found() {
        let error = YggdrasilError::new(YggdrasilErrorKind::ProfileNotFound);
        assert_eq!(error.status_code(), StatusCode::NO_CONTENT);
        assert_eq!(error.protocol_error_name(), "NotFound");
        assert_eq!(error.protocol_message(), "Profile not found.");
    }

    #[test]
    fn internal_prefers_detail_but_hides_it_when_absent() {
        let without_detail = YggdrasilError::new(YggdrasilErrorKind::Internal);
        assert_eq!(without_detail.protocol_message(), "Internal server error.");
        let with_detail =
            YggdrasilError::with_detail(YggdrasilErrorKind::Internal, "driver exploded");
        assert_eq!(with_detail.protocol_message(), "driver exploded");
    }

    #[test]
    fn bad_request_uses_detail_as_message() {
        let error = YggdrasilError::with_detail(YggdrasilErrorKind::BadRequest, "name taken");
        assert_eq!(error.protocol_message(), "name taken");
        let fallback = YggdrasilError::new(YggdrasilErrorKind::BadRequest);
        assert_eq!(fallback.protocol_message(), "Invalid request.");
    }

    #[test]
    fn stable_messages_match_the_protocol_examples() {
        let cases = [
            (YggdrasilErrorKind::InvalidToken, "Invalid token."),
            (
                YggdrasilErrorKind::InvalidCredentials,
                "Invalid credentials. Invalid username or password.",
            ),
            (
                YggdrasilErrorKind::AccessTokenAlreadyHasProfile,
                "Access token already has a profile assigned.",
            ),
            (
                YggdrasilErrorKind::ForbiddenProfile,
                "Profile does not belong to this user.",
            ),
            (
                YggdrasilErrorKind::UnsupportedAgent,
                "Unsupported Yggdrasil agent.",
            ),
            (
                YggdrasilErrorKind::TooManyProfilesRequested,
                "Too many profile names requested.",
            ),
        ];
        for (kind, message) in cases {
            assert_eq!(YggdrasilError::new(kind).protocol_message(), message);
        }
    }
}
