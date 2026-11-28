//! Offline verifier generation with hidden terminal input or bounded piped stdin.
//!
//! Terminal input is masked and may be retried on invalid entries; piped input is
//! read to EOF with one overflow byte so truncated input is rejected instead of
//! silently accepted. The command never touches configuration, logging, or server
//! resources, so it works on machines that cannot run the service.

use super::super::common::error::CliResult;
use crate::errors::AsterError;
#[cfg(feature = "operator-cli")]
use inquire::validator::Validation;
use secrecy::{ExposeSecret, SecretString};
use std::io::{IsTerminal, Read};

const INVALID_PASSWORD: &str = "password must contain 1..4096 bytes without control characters";

fn valid_password(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}

#[cfg(feature = "operator-cli")]
fn validate_terminal_password(value: &str) -> Validation {
    if valid_password(value) {
        Validation::Valid
    } else {
        Validation::Invalid(INVALID_PASSWORD.into())
    }
}

#[cfg(feature = "operator-cli")]
fn terminal_password_validator(value: &str) -> Result<Validation, inquire::error::CustomUserError> {
    Ok(validate_terminal_password(value))
}

pub(in crate::cli) async fn execute() -> CliResult<String> {
    let input: SecretString = if std::io::stdin().is_terminal() {
        read_terminal_password()?
    } else {
        read_piped_password()?
    };
    let password = input.expose_secret();
    if !valid_password(password) {
        return Err(AsterError::validation_error(INVALID_PASSWORD).into());
    }
    let hash = aster_forge_auth::PasswordHashRuntime::default()
        .hash_password(password)
        .await
        .map_err(|_| AsterError::internal_error("cannot generate authentication verifier"))?;
    Ok(hash)
}

#[cfg(feature = "operator-cli")]
fn read_terminal_password() -> CliResult<SecretString> {
    super::super::common::input::read_terminal_value(
        true,
        "Password (masked)",
        terminal_password_validator,
    )
}

#[cfg(not(feature = "operator-cli"))]
fn read_terminal_password() -> CliResult<SecretString> {
    Err(AsterError::validation_error(
        "interactive input requires the operator-cli build feature; pipe input on stdin instead",
    )
    .into())
}

fn read_piped_password() -> CliResult<SecretString> {
    read_piped_password_from(std::io::stdin())
}

fn read_piped_password_from(reader: impl Read) -> CliResult<SecretString> {
    let mut password = String::new();
    // Include CRLF and one overflow byte so a long input cannot be
    // silently accepted after truncation at a line ending.
    reader
        .take(4099)
        .read_to_string(&mut password)
        .map_err(|_| AsterError::validation_error("input must be valid UTF-8"))?;
    let password = password.strip_suffix('\n').unwrap_or(&password);
    let password = password.strip_suffix('\r').unwrap_or(password);
    Ok(password.to_owned().into())
}

#[cfg(test)]
mod tests {
    use super::valid_password;
    use secrecy::ExposeSecret;
    use std::io::Cursor;

    #[cfg(feature = "operator-cli")]
    #[test]
    fn terminal_validator_uses_the_same_secret_boundaries() {
        assert!(matches!(
            super::validate_terminal_password("valid"),
            inquire::validator::Validation::Valid
        ));
        assert!(matches!(
            super::validate_terminal_password("\n"),
            inquire::validator::Validation::Invalid(_)
        ));
    }

    #[cfg(feature = "operator-cli")]
    #[test]
    fn terminal_prompt_requires_a_real_tty_before_opening_the_prompt() {
        let error = super::read_terminal_password().unwrap_err().to_string();
        assert!(error.contains("interactive input requires terminal stdin and stderr"));
        assert!(matches!(
            super::terminal_password_validator("valid"),
            Ok(inquire::validator::Validation::Valid)
        ));
    }

    #[test]
    fn piped_reader_strips_line_endings_and_rejects_invalid_utf8() {
        let value = super::read_piped_password_from(Cursor::new(b"secret\r\n")).unwrap();
        assert_eq!(value.expose_secret(), "secret");
        let error = super::read_piped_password_from(Cursor::new(vec![0xff])).unwrap_err();
        assert!(error.to_string().contains("valid UTF-8"));
    }

    #[cfg(not(feature = "operator-cli"))]
    #[test]
    fn terminal_input_without_operator_cli_explains_how_to_pipe_input() {
        let error = super::read_terminal_password().unwrap_err().to_string();
        assert!(error.contains("requires the operator-cli build feature"));
        assert!(error.contains("pipe input on stdin"));
    }

    #[test]
    fn validation_counts_utf8_bytes_and_rejects_empty_and_control_input() {
        for value in ["", "\n", "\r\n", "a\nb", "a\tb", "a\0b", "a\x1bb", "a\x7fb"] {
            assert!(!valid_password(value));
        }
        assert!(valid_password("  abc  "));
        assert!(valid_password(&"x".repeat(4096)));
        assert!(!valid_password(&"x".repeat(4097)));
        assert!(valid_password(&"猫".repeat(1365)));
        assert!(!valid_password(&"猫".repeat(1366)));
    }
}
