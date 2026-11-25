//! Shared bounded credential input. Debug and error paths never expose input bytes.

use std::io::IsTerminal;

use inquire::validator::{StringValidator, Validation};
use inquire::{InquireError, Password, PasswordDisplayMode, Text};
use secrecy::SecretString;

use super::error::{CliError, CliResult};
use crate::errors::AsterError;

const MAX_INPUT_BYTES: u64 = 1024 * 1024;

/// Terminal prompts use stderr so structured stdout remains machine-readable.
pub(in crate::cli) fn read_terminal_value<V>(
    sensitive: bool,
    label: &str,
    validator: V,
) -> CliResult<SecretString>
where
    V: StringValidator + 'static,
{
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Err(AsterError::validation_error(
            "interactive input requires terminal stdin and stderr; use a pipe or an explicit input source",
        )
        .into());
    }
    let limit = |value: &str| {
        Ok(if value.len() as u64 > MAX_INPUT_BYTES {
            Validation::Invalid("input exceeds 1 MiB limit".into())
        } else {
            Validation::Valid
        })
    };
    let label = label.trim().trim_end_matches(':');
    if sensitive {
        Password::new(label)
            .without_confirmation()
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_validator(limit)
            .with_validator(validator)
            .prompt()
    } else {
        Text::new(label)
            .with_validator(limit)
            .with_validator(validator)
            .prompt()
    }
    .map(SecretString::from)
    .map_err(terminal_error)
}

pub(in crate::cli) fn terminal_error(error: InquireError) -> CliError {
    match error {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => CliError::Canceled,
        _ => AsterError::config_error("interactive terminal input failed (values redacted)").into(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn input_limit_stays_above_the_password_maximum_so_validators_own_rejection() {
        // The 1 MiB interactive cap only guards pathological terminal paste; the
        // 4096-byte password boundary must be reported by the password validator
        // with its actionable message, not by this generic limit.
        const _: () = assert!(super::MAX_INPUT_BYTES > 4096);
    }
}
