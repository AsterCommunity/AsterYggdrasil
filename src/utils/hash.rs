//! Password hashing and digest helpers.

use crate::errors::{AsterError, Result};
use sha2::Sha256;

fn map_crypto_error(error: aster_forge_crypto::CryptoError) -> AsterError {
    AsterError::internal_error(error.to_string())
}

/// Hashes a password using the shared AsterForge Argon2 implementation.
pub fn hash_password(password: &str) -> Result<String> {
    aster_forge_crypto::hash_password(password).map_err(map_crypto_error)
}

/// Verifies a password against an Argon2 password hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    aster_forge_crypto::verify_password(password, hash).map_err(map_crypto_error)
}

/// Computes the SHA-256 digest of `data` and returns lowercase hex.
pub fn sha256_hex(data: &[u8]) -> String {
    aster_forge_crypto::sha256_hex(data)
}

/// Encodes arbitrary bytes as lowercase hex.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    aster_forge_crypto::bytes_to_hex(bytes)
}

/// Encodes a SHA-256 digest as lowercase hex.
pub fn sha256_digest_to_hex(digest: &[u8]) -> String {
    aster_forge_crypto::sha256_digest_to_hex(digest)
}

/// Creates a new incremental SHA-256 hasher.
pub fn new_sha256() -> Sha256 {
    aster_forge_crypto::new_sha256()
}
