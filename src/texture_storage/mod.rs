//! Minecraft object storage abstraction.

mod local;
mod s3;

use crate::errors::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncRead;

pub use local::LocalTextureStore;
pub use s3::S3TextureStore;

/// Metadata for a stored texture object, as reported by the storage backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectBlobMetadata {
    /// Object size in bytes.
    pub size: u64,
    /// Content type served for the object; currently always `image/png`.
    pub content_type: &'static str,
}

/// Storage abstraction for texture files backed by the local filesystem or S3-compatible object storage.
///
/// Keys are normalized object keys (`textures/aa/bb.png` style); implementations map them onto
/// backend-specific paths and may add a configured base prefix.
#[async_trait]
pub trait TextureStore: Send + Sync {
    /// Name of the storage backend, e.g. `"local"` or `"s3"`.
    fn backend_name(&self) -> &'static str;
    /// Uploads the file at `local_path` as `storage_key`.
    ///
    /// Local backend skips the copy when the key already exists; S3 overwrites. Returns the
    /// normalized storage key actually used.
    async fn put_file(&self, storage_key: &str, local_path: &Path) -> Result<String>;
    /// Opens `storage_key` for streaming reads.
    ///
    /// Returns `Err(record_not_found)` when the object does not exist.
    async fn get_stream(&self, storage_key: &str) -> Result<Box<dyn AsyncRead + Unpin + Send>>;
    /// Deletes `storage_key`.
    ///
    /// Deleting a missing object succeeds idempotently.
    async fn delete(&self, storage_key: &str) -> Result<()>;
    /// Returns whether `storage_key` currently exists in the backend.
    async fn exists(&self, storage_key: &str) -> Result<bool>;
    /// Returns size and content type of `storage_key`.
    ///
    /// Returns `Err(record_not_found)` when the object does not exist.
    async fn metadata(&self, storage_key: &str) -> Result<ObjectBlobMetadata>;
    /// Lists all storage keys under `prefix`, sorted lexicographically.
    ///
    /// An empty prefix lists every object; a missing prefix yields an empty list.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
}

/// Builds the texture store configured by `config`.
///
/// Accepts `""`/`"local"` (filesystem backend) and `"s3"`/`"minio"`; any other backend name is
/// a configuration error.
pub fn create_texture_store(
    config: &crate::config::TextureStorageConfig,
) -> Result<Arc<dyn TextureStore>> {
    match config.backend.trim() {
        "" | "local" => Ok(Arc::new(LocalTextureStore::new(&config.local_root))),
        "s3" | "minio" => Ok(Arc::new(S3TextureStore::new(&config.s3)?)),
        backend => Err(crate::errors::AsterError::config_error(format!(
            "unsupported object_storage backend '{backend}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::create_texture_store;
    use crate::config::TextureStorageConfig;

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
