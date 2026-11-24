//! Offline Argon2id verifier generation for static configuration.
//!
//! Passwords are read from stdin so they do not land in shell history or
//! process listings. The output is one PHC string, suitable for
//! `[observability.prometheus.basic_auth] password_hash`.

use std::io::{self, BufRead, Read};

const MAX_PASSWORD_BYTES: usize = 4096;

/// Reads one line (or an EOF-terminated value) from stdin and prints its PHC
/// verifier.
///
/// # Errors
/// Returns an error for empty input, control characters, oversized input or
/// when the hashing primitive rejects the password.
pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let password = read_password(&mut stdin.lock())?;
    let password = password.as_str();
    if !valid_password(password) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password must be 1..4096 bytes without control characters",
        ));
    }
    let hash = aster_forge_crypto::hash_password(password)
        .map_err(|error| io::Error::other(format!("failed to hash password: {error}")))?;
    println!("{hash}");
    Ok(())
}

/// Reads one line, capped at the supported password length.
///
/// `read_line` stops at the newline the user types; `read_to_string` would
/// wait for EOF, which makes the command look hung in a terminal.
fn read_password<R: BufRead>(reader: &mut R) -> io::Result<String> {
    let mut password = String::new();
    let mut limited = reader.by_ref().take(MAX_PASSWORD_BYTES as u64 + 2);
    if limited.read_line(&mut password)? == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty input"));
    }
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}

fn valid_password(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PASSWORD_BYTES && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typed newline is the terminator: the rest of the stream must not be
    /// consumed, otherwise an interactive caller waits for EOF.
    #[test]
    fn reads_one_line_and_leaves_the_rest_unread() {
        let mut input = std::io::Cursor::new(b"secret\nstill-open".to_vec());
        let password = read_password(&mut input).unwrap();
        assert_eq!(password, "secret");
        let mut rest = String::new();
        input.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "still-open");
    }

    #[test]
    fn password_bounds_are_enforced() {
        assert!(valid_password("pass:with:colons"));
        assert!(!valid_password(""));
        assert!(!valid_password("bad\npassword"));
        assert!(!valid_password(&"a".repeat(MAX_PASSWORD_BYTES + 1)));
    }

    /// The produced verifier must validate under the same limits the config
    /// loader enforces, and must round-trip through verification.
    #[test]
    fn produced_hash_is_a_valid_verifier() {
        let hash = aster_forge_crypto::hash_password("scrape-secret").unwrap();
        aster_forge_crypto::validate_password_hash(
            &hash,
            aster_forge_crypto::PasswordHashVerificationLimits::default(),
        )
        .unwrap();
        assert!(aster_forge_crypto::verify_password("scrape-secret", &hash).unwrap());
        assert!(!aster_forge_crypto::verify_password("wrong", &hash).unwrap());
    }
}
