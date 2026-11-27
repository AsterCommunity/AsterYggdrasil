//! S3-compatible object storage.

use super::TextureStoreResult;
use crate::errors::{AsterError, Result};
use aster_forge_storage_core::{
    S3ConfigError, join_key_prefix, normalize_object_key, normalize_object_prefix,
    normalize_s3_endpoint_and_bucket, strip_key_prefix,
};
use aster_yggdrasil_texture_storage::{
    ObjectBlobMetadata, S3TextureStorageConfig, TextureStore, TextureStoreError,
};
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region, timeout::TimeoutConfig};
use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_s3::primitives::ByteStream;
use secrecy::ExposeSecret;
use std::error::Error as StdError;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncRead;

const DEFAULT_CONTENT_TYPE: &str = "image/png";
const DEFAULT_REGION: &str = "us-east-1";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

/// [`TextureStore`](super::TextureStore) implementation backed by an S3-compatible object storage service.
pub struct S3TextureStore {
    client: aws_sdk_s3::Client,
    bucket: String,
    base_path: String,
    endpoint: Option<String>,
    force_path_style: bool,
}

impl S3TextureStore {
    /// Creates an S3 client from `config`, validating bucket, credentials, and endpoint first.
    ///
    /// Performs no network I/O; an empty `region` falls back to `us-east-1` and a missing
    /// endpoint targets the AWS default.
    pub fn new(config: &S3TextureStorageConfig) -> Result<Self> {
        validate_config(config)?;

        let credentials = Credentials::new(
            config.access_key_id.trim(),
            config.secret_access_key.expose_secret().trim(),
            None,
            None,
            "asteryggdrasil-object-storage",
        );
        let region = if config.region.trim().is_empty() {
            DEFAULT_REGION
        } else {
            config.region.trim()
        };

        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(credentials)
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(CONNECT_TIMEOUT)
                    .read_timeout(READ_TIMEOUT)
                    .operation_timeout(OPERATION_TIMEOUT)
                    .build(),
            )
            .force_path_style(config.force_path_style);

        let normalized_s3 = normalize_storage_s3_config(config)?;
        if !normalized_s3.endpoint.is_empty() {
            let endpoint = normalized_s3.endpoint.as_str();
            builder = builder.endpoint_url(endpoint);
        }

        Ok(Self {
            client: aws_sdk_s3::Client::from_conf(builder.build()),
            bucket: normalized_s3.bucket,
            base_path: normalize_object_prefix(&config.base_path)
                .map_err(|error| AsterError::validation_error(error.to_string()))?,
            endpoint: endpoint_option(normalized_s3.endpoint),
            force_path_style: config.force_path_style,
        })
    }

    fn full_key(&self, storage_key: &str) -> TextureStoreResult<String> {
        let key = normalize_object_key(storage_key)?;
        Ok(join_key_prefix(&self.base_path, &key))
    }

    fn full_prefix(&self, prefix: &str) -> TextureStoreResult<String> {
        let prefix = normalize_object_prefix(prefix)?;
        if self.base_path.is_empty() {
            return Ok(prefix);
        }
        if prefix.is_empty() {
            return Ok(format!("{}/", self.base_path));
        }
        Ok(join_key_prefix(&self.base_path, &prefix))
    }

    fn strip_base_path<'a>(&self, key: &'a str) -> Option<&'a str> {
        strip_key_prefix(&self.base_path, key)
    }
}

#[async_trait]
impl TextureStore for S3TextureStore {
    fn backend_name(&self) -> &'static str {
        "s3"
    }

    async fn put_file(&self, storage_key: &str, local_path: &Path) -> TextureStoreResult<String> {
        let key = normalize_object_key(storage_key)?;
        let full_key = self.full_key(&key)?;
        let body = ByteStream::from_path(local_path).await.map_err(|error| {
            TextureStoreError::backend(format!("open object for S3 upload: {error}"))
        })?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .content_type(DEFAULT_CONTENT_TYPE)
            .body(body)
            .send()
            .await
            .map_err(|error| {
                self.map_sdk_error(
                    "put_object",
                    Some(&full_key),
                    "S3 object upload failed",
                    error,
                )
            })?;

        Ok(key)
    }

    async fn get_stream(
        &self,
        storage_key: &str,
    ) -> TextureStoreResult<Box<dyn AsyncRead + Unpin + Send>> {
        let key = self.full_key(storage_key)?;
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|error| {
                self.map_not_found_or_sdk_error(
                    "get_object",
                    Some(&key),
                    "S3 object download failed",
                    error,
                )
            })?;

        Ok(Box::new(response.body.into_async_read()))
    }

    async fn delete(&self, storage_key: &str) -> TextureStoreResult<()> {
        let key = self.full_key(storage_key)?;
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|error| {
                self.map_sdk_error(
                    "delete_object",
                    Some(&key),
                    "S3 object delete failed",
                    error,
                )
            })?;
        Ok(())
    }

    async fn exists(&self, storage_key: &str) -> TextureStoreResult<bool> {
        let key = self.full_key(storage_key)?;
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(error) if is_not_found(&error) => Ok(false),
            Err(error) => Err(self.map_sdk_error(
                "head_object",
                Some(&key),
                "S3 object exists check failed",
                error,
            )),
        }
    }

    async fn metadata(&self, storage_key: &str) -> TextureStoreResult<ObjectBlobMetadata> {
        let key = self.full_key(storage_key)?;
        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|error| {
                self.map_not_found_or_sdk_error(
                    "head_object",
                    Some(&key),
                    "S3 object metadata failed",
                    error,
                )
            })?;
        let size = response
            .content_length
            .map(|value| {
                aster_forge_utils::numbers::i64_to_u64(value, "S3 object content_length")
                    .map_err(|error| TextureStoreError::backend(error.to_string()))
            })
            .transpose()?
            .unwrap_or(0);

        Ok(ObjectBlobMetadata {
            size,
            content_type: DEFAULT_CONTENT_TYPE,
        })
    }

    async fn list_keys(&self, prefix: &str) -> TextureStoreResult<Vec<String>> {
        let prefix = self.full_prefix(prefix)?;
        let mut continuation_token: Option<String> = None;
        let mut keys = Vec::new();

        loop {
            let mut request = self.client.list_objects_v2().bucket(&self.bucket);
            if !prefix.is_empty() {
                request = request.prefix(prefix.clone());
            }
            if let Some(token) = continuation_token.as_deref() {
                request = request.continuation_token(token);
            }

            let response = request.send().await.map_err(|error| {
                self.map_sdk_error(
                    "list_objects_v2",
                    Some(&prefix),
                    "S3 object list failed",
                    error,
                )
            })?;

            for object in response.contents() {
                let Some(key) = object.key() else {
                    continue;
                };
                if let Some(storage_key) = self.strip_base_path(key) {
                    keys.push(storage_key.to_string());
                }
            }

            let truncated = response.is_truncated().unwrap_or(false);
            continuation_token = response.next_continuation_token().map(ToOwned::to_owned);
            if !truncated || continuation_token.is_none() {
                break;
            }
        }

        keys.sort();
        Ok(keys)
    }
}

impl S3TextureStore {
    fn map_not_found_or_sdk_error<E>(
        &self,
        operation: &'static str,
        key: Option<&str>,
        context: &'static str,
        error: SdkError<E>,
    ) -> TextureStoreError
    where
        E: StdError + ProvideErrorMetadata + Send + Sync + 'static,
    {
        if is_not_found(&error) {
            TextureStoreError::not_found(format!("{context}: object not found"))
        } else {
            self.map_sdk_error(operation, key, context, error)
        }
    }

    fn map_sdk_error<E>(
        &self,
        operation: &'static str,
        key: Option<&str>,
        context: &'static str,
        error: SdkError<E>,
    ) -> TextureStoreError
    where
        E: StdError + ProvideErrorMetadata + Send + Sync + 'static,
    {
        let formatted = format_sdk_error(&error);
        tracing::warn!(
            operation,
            bucket = %self.bucket,
            key = key.unwrap_or(""),
            endpoint = self.endpoint.as_deref().unwrap_or("aws-default"),
            force_path_style = self.force_path_style,
            error = %formatted,
            "S3 object storage request failed"
        );
        TextureStoreError::backend(format!("{context}: {formatted}"))
    }
}

fn validate_config(config: &S3TextureStorageConfig) -> Result<()> {
    normalize_storage_s3_config(config)?;
    if config.access_key_id.trim().is_empty() {
        return Err(AsterError::config_error(
            "object_storage.s3.access_key_id cannot be empty",
        ));
    }
    if config.secret_access_key.expose_secret().trim().is_empty() {
        return Err(AsterError::config_error(
            "object_storage.s3.secret_access_key cannot be empty",
        ));
    }
    Ok(())
}

fn normalize_storage_s3_config(
    config: &S3TextureStorageConfig,
) -> Result<aster_forge_storage_core::NormalizedS3Config> {
    normalize_s3_endpoint_and_bucket(&config.endpoint, &config.bucket).map_err(map_s3_config_error)
}

fn map_s3_config_error(error: S3ConfigError) -> AsterError {
    match error {
        S3ConfigError::MissingBucket => {
            AsterError::config_error("object_storage.s3.bucket cannot be empty")
        }
        S3ConfigError::InvalidEndpoint(message) => AsterError::config_error(message),
    }
}

fn endpoint_option(endpoint: String) -> Option<String> {
    if endpoint.is_empty() {
        None
    } else {
        Some(endpoint)
    }
}

fn is_not_found<E>(error: &SdkError<E>) -> bool
where
    E: StdError + ProvideErrorMetadata + Send + Sync + 'static,
{
    matches!(error.code(), Some("NoSuchKey" | "NotFound"))
        || error.raw_response().map(|raw| raw.status().as_u16()) == Some(404)
}

fn format_sdk_error<E>(error: &SdkError<E>) -> String
where
    E: StdError + ProvideErrorMetadata + Send + Sync + 'static,
{
    let mut details = Vec::new();
    if let Some(status) = error.raw_response().map(|raw| raw.status().as_u16()) {
        details.push(format!("http_status={status}"));
    }
    if let Some(code) = error.code() {
        details.push(format!("code={code}"));
    }
    if let Some(message) = error.message() {
        details.push(format!("message={message}"));
    }
    let source_chain = format_error_source_chain(error);
    if !source_chain.is_empty() {
        details.push(format!("source={source_chain}"));
    }

    if details.is_empty() {
        error.to_string()
    } else {
        details.join(", ")
    }
}

fn format_error_source_chain(error: &dyn StdError) -> String {
    let mut sources = Vec::new();
    let mut current = error.source();
    while let Some(source) = current {
        sources.push(source.to_string());
        current = source.source();
    }
    sources.join(": ")
}

#[cfg(test)]
mod tests {
    use super::{S3TextureStore, TextureStoreError, format_sdk_error, normalize_storage_s3_config};
    use crate::texture_storage::create_texture_store;
    use aster_forge_storage_core::normalize_object_key;
    use aster_yggdrasil_texture_storage::{
        S3TextureStorageConfig, TextureStorageConfig, TextureStore,
    };
    use aws_credential_types::Credentials;
    use aws_sdk_s3::error::ProvideErrorMetadata;
    use aws_sdk_s3::error::SdkError;
    use aws_sdk_s3::operation::put_object::PutObjectError;
    use testcontainers::{GenericImage, ImageExt, runners::AsyncRunner};
    use tokio::io::AsyncReadExt;

    const RUSTFS_ACCESS_KEY: &str = "rustfsadmin";
    const RUSTFS_SECRET_KEY: &str = "rustfsadmin123";
    const RUSTFS_PORT: u16 = 9000;

    fn valid_config() -> S3TextureStorageConfig {
        S3TextureStorageConfig {
            endpoint: "http://127.0.0.1:9000".to_string(),
            region: "us-east-1".to_string(),
            bucket: "textures".to_string(),
            base_path: String::new(),
            access_key_id: RUSTFS_ACCESS_KEY.to_string(),
            secret_access_key: secrecy::SecretString::from(RUSTFS_SECRET_KEY.to_string()),
            force_path_style: true,
        }
    }

    #[test]
    fn initializes_without_network_io() {
        let storage = S3TextureStore::new(&valid_config()).unwrap();

        assert_eq!(storage.bucket, "textures");
        assert_eq!(storage.base_path, "");
        assert_eq!(storage.endpoint.as_deref(), Some("http://127.0.0.1:9000"));
        assert!(storage.force_path_style);
    }

    #[test]
    fn endpoint_is_normalized_and_validated() {
        let mut config = valid_config();
        config.endpoint = String::new();
        config.bucket = "textures".to_string();
        let normalized = normalize_storage_s3_config(&config).unwrap();
        assert_eq!(normalized.endpoint, "");
        assert_eq!(normalized.bucket, "textures");

        config.endpoint = " http://127.0.0.1:9000/ ".to_string();
        assert_eq!(
            normalize_storage_s3_config(&config).unwrap().endpoint,
            "http://127.0.0.1:9000"
        );

        config.endpoint = "https://s3.example.test/root/".to_string();
        assert_eq!(
            normalize_storage_s3_config(&config).unwrap().endpoint,
            "https://s3.example.test/root"
        );

        for endpoint in [
            "ftp://s3.example.test",
            "http://",
            "https://s3.example.test/root?x=1",
            "https://s3.example.test/root#fragment",
        ] {
            config.endpoint = endpoint.to_string();
            assert!(normalize_storage_s3_config(&config).is_err());
        }
    }

    #[test]
    fn sdk_error_format_includes_source_chain_for_transport_failures() {
        let error: SdkError<PutObjectError> = SdkError::construction_failure(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "connection refused",
        ));

        let formatted = format_sdk_error(&error);

        assert!(formatted.contains("failed to construct request") || formatted.contains("source="));
        assert!(formatted.contains("connection refused"));
    }

    #[test]
    fn rejects_incomplete_config() {
        for (config, expected) in [
            {
                let mut config = valid_config();
                config.bucket.clear();
                (config, "object_storage.s3.bucket cannot be empty")
            },
            {
                let mut config = valid_config();
                config.access_key_id = "  ".to_string();
                (config, "object_storage.s3.access_key_id cannot be empty")
            },
            {
                let mut config = valid_config();
                config.secret_access_key = secrecy::SecretString::from("  ".to_string());
                (
                    config,
                    "object_storage.s3.secret_access_key cannot be empty",
                )
            },
        ] {
            let error = match S3TextureStore::new(&config) {
                Ok(_) => panic!("S3 object storage should reject incomplete config"),
                Err(error) => error,
            };

            assert!(error.message().contains(expected));
        }
    }

    #[test]
    fn storage_core_key_failures_map_to_invalid_key_errors() {
        // Happy-path normalization behavior is owned by aster_forge_storage_core;
        // this store only owns the mapping into the contract error type.
        for rejected in ["", "  ", "../a.png", "textures/../../a.png"] {
            let error = normalize_object_key(rejected).expect_err("key should be rejected");
            let mapped: TextureStoreError = error.into();
            assert!(
                matches!(mapped, TextureStoreError::InvalidKey { .. }),
                "key '{rejected}' should map to InvalidKey"
            );
        }
    }

    #[test]
    fn base_path_rejects_dangerous_paths_at_construction() {
        let result = S3TextureStore::new(&S3TextureStorageConfig {
            base_path: "textures/../other".to_string(),
            ..valid_config()
        });

        assert!(result.is_err(), "dangerous base path should be rejected");
    }

    #[test]
    fn base_path_is_applied_without_leaking_to_storage_keys() {
        let storage = S3TextureStore::new(&S3TextureStorageConfig {
            base_path: "env/textures/".to_string(),
            ..valid_config()
        })
        .unwrap();

        assert_eq!(storage.base_path, "env/textures");
        assert_eq!(
            storage.full_key("aa/bb.png").unwrap(),
            "env/textures/aa/bb.png"
        );
        assert_eq!(storage.full_prefix("").unwrap(), "env/textures/");
        assert_eq!(storage.full_prefix("aa/").unwrap(), "env/textures/aa");
        assert_eq!(
            storage.strip_base_path("env/textures/aa/bb.png"),
            Some("aa/bb.png")
        );
        assert_eq!(
            storage.strip_base_path("env/textures-other/aa/bb.png"),
            None
        );
    }

    #[test]
    fn s3_and_minio_backends_initialize_without_network_io() {
        for backend in ["s3", "minio"] {
            let config = TextureStorageConfig {
                backend: backend.to_string(),
                s3: valid_config(),
                ..TextureStorageConfig::default()
            };

            let storage = create_texture_store(&config).unwrap();

            assert_eq!(storage.backend_name(), "s3");
        }
    }

    #[tokio::test]
    async fn s3_backend_satisfies_streaming_object_storage_contract() {
        let (_container, endpoint) = start_rustfs().await;
        let bucket = format!("textures-{}", uuid::Uuid::new_v4());
        create_bucket(&endpoint, &bucket).await;
        let storage = S3TextureStore::new(&S3TextureStorageConfig {
            endpoint: endpoint.clone(),
            bucket: bucket.clone(),
            base_path: "env/production/textures".to_string(),
            ..valid_config()
        })
        .unwrap();

        let temp_dir = aster_forge_test::temp::TestTempDir::new("s3-storage-contract");
        let source = temp_dir.path().join("source.png");
        tokio::fs::write(&source, b"png-bytes").await.unwrap();

        let prefix = format!("contract/{}/", uuid::Uuid::new_v4());
        let key = format!("{prefix}skin.png");
        let sibling_key = format!("{prefix}cape.png");
        assert_eq!(storage.put_file(&key, &source).await.unwrap(), key);
        assert_eq!(
            storage.put_file(&sibling_key, &source).await.unwrap(),
            sibling_key
        );

        assert!(storage.exists(&key).await.unwrap());
        assert!(
            !storage
                .exists(&format!("{prefix}missing.png"))
                .await
                .unwrap()
        );

        let metadata = storage.metadata(&key).await.unwrap();
        assert_eq!(metadata.size, 9);
        assert_eq!(metadata.content_type, "image/png");

        let mut stream = storage.get_stream(&key).await.unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"png-bytes");

        let mut keys = storage.list_keys(&prefix).await.unwrap();
        keys.sort();
        assert_eq!(keys, vec![sibling_key.clone(), key.clone()]);

        let raw_client = s3_test_client(&endpoint);
        let uploaded = raw_client
            .get_object()
            .bucket(&bucket)
            .key(format!("env/production/textures/{key}"))
            .send()
            .await
            .unwrap()
            .body
            .collect()
            .await
            .unwrap()
            .into_bytes();
        assert_eq!(uploaded.as_ref(), b"png-bytes");

        storage.delete(&key).await.unwrap();
        storage.delete(&key).await.unwrap();
        assert!(!storage.exists(&key).await.unwrap());
        assert!(storage.exists(&sibling_key).await.unwrap());
        storage.delete(&sibling_key).await.unwrap();
    }

    #[tokio::test]
    async fn s3_backend_reports_missing_objects_without_failing_exists() {
        let (_container, endpoint) = start_rustfs().await;
        let bucket = format!("textures-{}", uuid::Uuid::new_v4());
        create_bucket(&endpoint, &bucket).await;
        let storage = S3TextureStore::new(&S3TextureStorageConfig {
            endpoint,
            bucket,
            ..valid_config()
        })
        .unwrap();

        assert!(!storage.exists("missing/skin.png").await.unwrap());
        assert!(storage.metadata("missing/skin.png").await.is_err());
        assert!(storage.get_stream("missing/skin.png").await.is_err());
    }

    async fn start_rustfs() -> (
        testcontainers::ContainerAsync<testcontainers::GenericImage>,
        String,
    ) {
        let container = GenericImage::new("rustfs/rustfs", "latest")
            .with_exposed_port(testcontainers::core::IntoContainerPort::tcp(RUSTFS_PORT))
            .with_env_var("RUSTFS_ACCESS_KEY", RUSTFS_ACCESS_KEY)
            .with_env_var("RUSTFS_SECRET_KEY", RUSTFS_SECRET_KEY)
            .start()
            .await
            .expect("failed to start rustfs container");

        let port = container
            .get_host_port_ipv4(testcontainers::core::IntoContainerPort::tcp(RUSTFS_PORT))
            .await
            .expect("rustfs test port should be exposed");

        (container, format!("http://127.0.0.1:{port}"))
    }

    async fn create_bucket(endpoint: &str, bucket: &str) {
        let client = s3_test_client(endpoint);
        let mut last_error = None;
        let ready = tokio::time::timeout(std::time::Duration::from_secs(45), async {
            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    client.create_bucket().bucket(bucket).send(),
                )
                .await
                {
                    Ok(Ok(_)) => break,
                    Ok(Err(error)) => {
                        let code = error
                            .as_service_error()
                            .and_then(|service_error| service_error.code());
                        if matches!(
                            code,
                            Some("BucketAlreadyOwnedByYou" | "BucketAlreadyExists")
                        ) {
                            break;
                        }
                        last_error = Some(error.to_string());
                    }
                    Err(_) => {
                        last_error = Some("create_bucket attempt timed out".to_string());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        })
        .await;

        if ready.is_err() {
            panic!(
                "timed out waiting for S3 bucket {bucket} at {endpoint}: {}",
                last_error.unwrap_or_else(|| "unknown error".to_string())
            );
        }
    }

    fn s3_test_client(endpoint: &str) -> aws_sdk_s3::Client {
        let credentials = Credentials::new(
            RUSTFS_ACCESS_KEY,
            RUSTFS_SECRET_KEY,
            None,
            None,
            "asteryggdrasil-s3-test",
        );
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .credentials_provider(credentials)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        aws_sdk_s3::Client::from_conf(config)
    }
}
