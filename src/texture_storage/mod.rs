//! Minecraft texture object storage: concrete backends and backend selection.
//!
//! The [`TextureStore`] contract, structured store errors, and the
//! `[object_storage]` configuration types live in the
//! `aster_yggdrasil_texture_storage` crate; this module owns the concrete
//! local/S3 implementations, SDK wiring, and application error mapping.

mod local;
mod s3;

use aster_yggdrasil_texture_storage::{TextureStorageConfig, TextureStoreError};
use std::sync::Arc;

pub use aster_yggdrasil_texture_storage::{TextureStore, TextureStoreResult};
pub use local::LocalTextureStore;
pub use s3::S3TextureStore;

/// Builds the texture store configured by `config`.
///
/// Accepts `""`/`"local"` (filesystem backend) and `"s3"`/`"minio"`; any other backend name is
/// a configuration error.
pub fn create_texture_store(
    config: &TextureStorageConfig,
) -> crate::errors::Result<Arc<dyn TextureStore>> {
    match config.backend.trim() {
        "" | "local" => Ok(Arc::new(LocalTextureStore::new(&config.local_root))),
        "s3" | "minio" => Ok(Arc::new(S3TextureStore::new(&config.s3)?)),
        backend => Err(crate::errors::AsterError::config_error(format!(
            "unsupported object_storage backend '{backend}'"
        ))),
    }
}

/// Maps contract errors onto the application error type; store call sites
/// keep using `?` transparently.
impl From<TextureStoreError> for crate::errors::AsterError {
    fn from(error: TextureStoreError) -> Self {
        match error {
            TextureStoreError::NotFound { context } => Self::record_not_found(context),
            TextureStoreError::Backend { context } => Self::internal_error(context),
            TextureStoreError::InvalidKey { reason } => Self::validation_error(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::create_texture_store;
    use aster_yggdrasil_texture_storage::TextureStorageConfig;

    #[test]
    fn creates_local_backend_from_explicit_config() {
        let config = TextureStorageConfig {
            backend: "local".to_string(),
            local_root: "target/test-objects".to_string(),
            ..TextureStorageConfig::default()
        };

        let storage = create_texture_store(&config).expect("local object storage should init");

        assert_eq!(storage.backend_name(), "local");
    }

    #[test]
    fn rejects_incomplete_s3_config() {
        let config = TextureStorageConfig {
            backend: "s3".to_string(),
            ..TextureStorageConfig::default()
        };

        let error = match create_texture_store(&config) {
            Ok(_) => panic!("s3 object storage should reject incomplete config"),
            Err(error) => error,
        };

        assert!(error.message().contains("object_storage.s3.bucket"));
    }

    #[test]
    fn rejects_unknown_backend() {
        let config = TextureStorageConfig {
            backend: "ftp".to_string(),
            ..TextureStorageConfig::default()
        };

        let error = match create_texture_store(&config) {
            Ok(_) => panic!("unknown object storage backend should be rejected"),
            Err(error) => error,
        };

        assert!(
            error
                .message()
                .contains("unsupported object_storage backend")
        );
    }
}
