use crate::errors::AsterError;

pub use aster_yggdrasil_protocol::{YggdrasilError, YggdrasilErrorKind};

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
