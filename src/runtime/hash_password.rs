//! Offline Argon2id verifier generation for static configuration.
//!
//! Passwords are read from stdin so they do not land in shell history or
//! process listings. The output is one PHC string, suitable for
//! `[observability.prometheus.basic_auth] password_hash`.

use std::io::{self, Read};

const MAX_PASSWORD_BYTES: usize = 4096;

/// Reads one line (or an EOF-terminated value) from stdin and prints its PHC
/// verifier.
///
/// # Errors
/// Returns an error for empty input, control characters, oversized input or
/// when the hashing primitive rejects the password.
pub fn run() -> io::Result<()> {
    let mut password = String::new();
    io::stdin()
        .take(MAX_PASSWORD_BYTES as u64 + 4)
        .read_to_string(&mut password)?;
    let password = password.trim_end_matches(['\r', '\n']);
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

fn valid_password(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PASSWORD_BYTES && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

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
