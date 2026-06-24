use crate::errors::{AsterError, Result};

fn map_validation_error(error: aster_forge_validation::ValidationError) -> AsterError {
    AsterError::validation_error(error.to_string())
}

pub fn validate_email(email: &str) -> Result<()> {
    aster_forge_validation::email::validate_email(email).map_err(map_validation_error)
}

pub fn normalize_email(email: &str) -> Result<String> {
    aster_forge_validation::email::normalize_email(email).map_err(map_validation_error)
}

pub fn email_domain(email: &str) -> Result<String> {
    aster_forge_validation::email::email_domain(email).map_err(map_validation_error)
}

#[cfg(test)]
mod tests {
    use super::{email_domain, normalize_email, validate_email};

    #[test]
    fn validate_email_requires_exactly_one_at_separator() {
        assert!(validate_email("alice@example.com").is_ok());
        assert!(validate_email("alice@@example.com").is_err());
        assert!(validate_email("alice@example@com").is_err());
        assert!(validate_email("alice.example.com").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("alice@").is_err());
    }

    #[test]
    fn email_helpers_keep_existing_normalization_contract() {
        assert_eq!(
            normalize_email(" Alice@Example.COM ").unwrap(),
            "alice@example.com"
        );
        assert_eq!(email_domain("alice@Example.COM").unwrap(), "example.com");
    }
}
