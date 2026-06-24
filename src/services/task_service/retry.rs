//! Background task retry policy.

use crate::errors::AsterError;

pub(super) fn default_retry_class(error: &AsterError) -> aster_forge_tasks::TaskRetryClass {
    match error {
        AsterError::Public {
            status, retryable, ..
        } => match retryable {
            Some(true) => aster_forge_tasks::TaskRetryClass::Auto,
            Some(false) => aster_forge_tasks::TaskRetryClass::Never,
            None if status.is_server_error() => aster_forge_tasks::TaskRetryClass::Manual,
            None => aster_forge_tasks::TaskRetryClass::Never,
        },
        AsterError::DatabaseConnection(_) | AsterError::MailDeliveryFailed(_) => {
            aster_forge_tasks::TaskRetryClass::Auto
        }
        AsterError::DatabaseOperation(_)
        | AsterError::ConfigError(_)
        | AsterError::ExternalAuthError(_)
        | AsterError::InternalError(_) => aster_forge_tasks::TaskRetryClass::Manual,
        AsterError::ValidationError(_)
        | AsterError::AuthInvalidCredentials(_)
        | AsterError::AuthTokenInvalid(_)
        | AsterError::AuthTokenExpired(_)
        | AsterError::AuthForbidden(_)
        | AsterError::RecordNotFound(_)
        | AsterError::MailNotConfigured(_) => aster_forge_tasks::TaskRetryClass::Never,
    }
}

#[cfg(test)]
mod tests {
    use super::default_retry_class;
    use crate::errors::AsterError;
    use aster_forge_tasks::TaskRetryClass;

    #[test]
    fn default_retry_class_groups_transient_manual_and_permanent_errors() {
        assert_eq!(
            default_retry_class(&AsterError::database_connection("connect failed")),
            TaskRetryClass::Auto
        );
        assert_eq!(
            default_retry_class(&AsterError::mail_delivery_failed("smtp timeout")),
            TaskRetryClass::Auto
        );
        assert_eq!(
            default_retry_class(&AsterError::runtime_unavailable_retryable(
                "runtime unavailable"
            )),
            TaskRetryClass::Auto
        );

        for error in [
            AsterError::database_operation("query failed"),
            AsterError::config_error("config failed"),
            AsterError::external_auth_error("provider failed"),
            AsterError::internal_error("internal failed"),
            AsterError::internal_error_code(
                crate::api::error_code::AsterErrorCode::InternalServerError,
                "internal failed",
            ),
        ] {
            assert_eq!(default_retry_class(&error), TaskRetryClass::Manual);
        }

        for error in [
            AsterError::validation_error("bad input"),
            AsterError::auth_invalid_credentials("bad credentials"),
            AsterError::auth_token_invalid("bad token"),
            AsterError::auth_token_expired("expired token"),
            AsterError::auth_forbidden("forbidden"),
            AsterError::record_not_found("missing"),
            AsterError::mail_not_configured("smtp missing"),
            AsterError::validation_failed("bad input"),
        ] {
            assert_eq!(default_retry_class(&error), TaskRetryClass::Never);
        }
    }
}
