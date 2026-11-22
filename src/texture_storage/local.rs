//! Local filesystem object storage.

use super::{ObjectBlobMetadata, TextureStore};
use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_storage_core::{normalize_object_key, normalize_object_prefix};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::io::AsyncRead;

const DEFAULT_CONTENT_TYPE: &str = "image/png";

pub struct LocalTextureStore {
    base_path: PathBuf,
}

impl LocalTextureStore {
    pub fn new(local_root: &str) -> Self {
        let base_path = Path::new(local_root).to_path_buf();
        Self { base_path }
    }

    fn full_path(&self, storage_key: &str) -> Result<PathBuf> {
        let relative = normalize_storage_object_key(storage_key)?;
        Ok(self.base_path.join(relative))
    }
}

#[async_trait]
impl TextureStore for LocalTextureStore {
    fn backend_name(&self) -> &'static str {
        "local"
    }

    async fn put_file(&self, storage_key: &str, local_path: &Path) -> Result<String> {
        let key = normalize_storage_object_key(storage_key)?;
        let target = self.base_path.join(&key);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_aster_err_ctx("create object storage dir", AsterError::internal_error)?;
        }
        if tokio::fs::try_exists(&target)
            .await
            .map_aster_err_ctx("check existing object", AsterError::internal_error)?
        {
            return Ok(key);
        }
        tokio::fs::copy(local_path, &target)
            .await
            .map_aster_err_ctx("store object", AsterError::internal_error)?;
        Ok(key)
    }

    async fn get_stream(&self, storage_key: &str) -> Result<Box<dyn AsyncRead + Unpin + Send>> {
        let file = tokio::fs::File::open(self.full_path(storage_key)?)
            .await
            .map_aster_err_ctx("open object", AsterError::record_not_found)?;
        Ok(Box::new(file))
    }

    async fn delete(&self, storage_key: &str) -> Result<()> {
        let target = self.full_path(storage_key)?;
        match tokio::fs::remove_file(target).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AsterError::internal_error(format!(
                "delete object: {error}"
            ))),
        }
    }

    async fn exists(&self, storage_key: &str) -> Result<bool> {
        tokio::fs::try_exists(self.full_path(storage_key)?)
            .await
            .map_aster_err_ctx("check object exists", AsterError::internal_error)
    }

    async fn metadata(&self, storage_key: &str) -> Result<ObjectBlobMetadata> {
        let metadata = tokio::fs::metadata(self.full_path(storage_key)?)
            .await
            .map_aster_err_ctx("read object metadata", AsterError::record_not_found)?;
        Ok(ObjectBlobMetadata {
            size: metadata.len(),
            content_type: DEFAULT_CONTENT_TYPE,
        })
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let relative_prefix = match normalize_storage_prefix(prefix)?.as_str() {
            "" => PathBuf::new(),
            prefix => PathBuf::from(prefix),
        };
        let root = self.base_path.join(&relative_prefix);
        if !tokio::fs::try_exists(&root)
            .await
            .map_aster_err_ctx("check object storage prefix", AsterError::internal_error)?
        {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let mut entries = tokio::fs::read_dir(&dir)
                .await
                .map_aster_err_ctx("read object storage dir", AsterError::internal_error)?;
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_aster_err_ctx("iterate object storage dir", AsterError::internal_error)?
            {
                let path = entry.path();
                let file_type = entry.file_type().await.map_aster_err_ctx(
                    "read object storage entry type",
                    AsterError::internal_error,
                )?;
                if file_type.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !file_type.is_file() {
                    continue;
                }
                let relative = path.strip_prefix(&self.base_path).map_err(|error| {
                    AsterError::internal_error(format!(
                        "object storage key is outside local root: {error}"
                    ))
                })?;
                keys.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        keys.sort();
        Ok(keys)
    }
}

fn normalize_storage_object_key(storage_key: &str) -> Result<String> {
    normalize_object_key(storage_key).map_err(map_storage_core_error)
}

fn normalize_storage_prefix(prefix: &str) -> Result<String> {
    normalize_object_prefix(prefix).map_err(map_storage_core_error)
}

fn map_storage_core_error(error: aster_forge_storage_core::StorageCoreError) -> AsterError {
    AsterError::validation_error(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{LocalTextureStore, normalize_storage_object_key};
    use crate::texture_storage::TextureStore;

    #[test]
    fn storage_key_normalizes_like_aster_drive() {
        assert_eq!(
            normalize_storage_object_key("/textures/a.png").unwrap(),
            "textures/a.png"
        );
        assert_eq!(
            normalize_storage_object_key("textures\\a.png").unwrap(),
            "textures/a.png"
        );
        assert!(normalize_storage_object_key("").is_err());
        assert!(normalize_storage_object_key("/").is_err());
        assert!(normalize_storage_object_key("../a.png").is_err());
        assert!(normalize_storage_object_key("textures/../../a.png").is_err());
    }

    #[tokio::test]
    async fn list_keys_returns_relative_sorted_keys_under_prefix() {
        let root = aster_forge_test::temp::TestTempDir::new("object-storage");
        let storage = LocalTextureStore::new(root.path().to_str().unwrap());
        let first = root.join("textures/ab/abc.png");
        let second = root.join("textures/cd/cde.png");
        tokio::fs::create_dir_all(first.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(second.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&first, b"one").await.unwrap();
        tokio::fs::write(&second, b"two").await.unwrap();

        let keys = storage.list_keys("textures").await.unwrap();

        assert_eq!(
            keys,
            vec![
                "textures/ab/abc.png".to_string(),
                "textures/cd/cde.png".to_string()
            ]
        );
    }
}
