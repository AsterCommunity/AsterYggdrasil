use crate::errors::AsterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YggdrasilErrorKind {
    InvalidToken,
    InvalidCredentials,
    AccessTokenAlreadyHasProfile,
    ForbiddenProfile,
    BadRequest,
    UnsupportedAgent,
    TooManyProfilesRequested,
    ProfileNotFound,
    Internal,
}

#[derive(Debug, Clone)]
pub struct YggdrasilError {
    kind: YggdrasilErrorKind,
    detail: Option<String>,
}

impl YggdrasilError {
    pub const fn new(kind: YggdrasilErrorKind) -> Self {
        Self { kind, detail: None }
    }

    pub fn with_detail(kind: YggdrasilErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: Some(detail.into()),
        }
    }

    pub const fn kind(&self) -> YggdrasilErrorKind {
        self.kind
    }

    pub const fn status_code(&self) -> actix_web::http::StatusCode {
        match self.kind {
            YggdrasilErrorKind::InvalidToken
            | YggdrasilErrorKind::InvalidCredentials
            | YggdrasilErrorKind::ForbiddenProfile => actix_web::http::StatusCode::FORBIDDEN,
            YggdrasilErrorKind::AccessTokenAlreadyHasProfile
            | YggdrasilErrorKind::BadRequest
            | YggdrasilErrorKind::UnsupportedAgent
            | YggdrasilErrorKind::TooManyProfilesRequested => {
                actix_web::http::StatusCode::BAD_REQUEST
            }
            YggdrasilErrorKind::ProfileNotFound => actix_web::http::StatusCode::NO_CONTENT,
            YggdrasilErrorKind::Internal => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

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

impl From<AsterError> for YggdrasilError {
    fn from(value: AsterError) -> Self {
        tracing::warn!(error = %value, "yggdrasil service failed");
        if matches!(
            &value,
            AsterError::DatabaseConnection(_)
                | AsterError::DatabaseOperation(_)
                | AsterError::DatabaseOperationClassified { .. }
                | AsterError::DatabaseCommitOutcomeUnknown { .. }
        ) {
            Self::new(YggdrasilErrorKind::Internal)
        } else {
            Self::with_detail(YggdrasilErrorKind::Internal, value.message())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::YggdrasilError;
    use crate::errors::AsterError;

    #[test]
    fn database_errors_are_redacted_from_protocol_messages() {
        for error in [
            AsterError::database_operation("driver query details"),
            AsterError::database_operation_classified(
                "deadlock details",
                aster_forge_db::DatabaseErrorKind::Deadlock,
            ),
            AsterError::database_commit_outcome_unknown(
                "commit transport details",
                Some(aster_forge_db::DatabaseErrorKind::LockTimeout),
            ),
        ] {
            let protocol_error = YggdrasilError::from(error);
            assert_eq!(protocol_error.protocol_message(), "Internal server error.");
        }
    }
}
