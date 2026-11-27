//! The backend-neutral texture store contract.

use crate::error::TextureStoreError;
use async_trait::async_trait;
use std::path::Path;
use tokio::io::AsyncRead;

/// Alias for store operation results.
pub type TextureStoreResult<T> = std::result::Result<T, TextureStoreError>;

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
    async fn put_file(&self, storage_key: &str, local_path: &Path) -> TextureStoreResult<String>;
    /// Opens `storage_key` for streaming reads.
    ///
    /// Returns `Err(NotFound)` when the object does not exist.
    async fn get_stream(
        &self,
        storage_key: &str,
    ) -> TextureStoreResult<Box<dyn AsyncRead + Unpin + Send>>;
    /// Deletes `storage_key`.
    ///
    /// Deleting a missing object succeeds idempotently.
    async fn delete(&self, storage_key: &str) -> TextureStoreResult<()>;
    /// Returns whether `storage_key` currently exists in the backend.
    async fn exists(&self, storage_key: &str) -> TextureStoreResult<bool>;
    /// Returns size and content type of `storage_key`.
    ///
    /// Returns `Err(NotFound)` when the object does not exist.
    async fn metadata(&self, storage_key: &str) -> TextureStoreResult<ObjectBlobMetadata>;
    /// Lists all storage keys under `prefix`, sorted lexicographically.
    ///
    /// An empty prefix lists every object; a missing prefix yields an empty list.
    async fn list_keys(&self, prefix: &str) -> TextureStoreResult<Vec<String>>;
}
