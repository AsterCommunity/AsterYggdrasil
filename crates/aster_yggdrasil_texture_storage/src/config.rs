//! `[object_storage]` configuration section types.

use aster_forge_config::ValidatableConfig;
use serde::{Deserialize, Serialize};

/// `[object_storage]` section: where texture files are stored.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextureStorageConfig {
    /// Backend selector: `local` (default) for filesystem storage, `s3` or
    /// `minio` for object storage; cluster deployments require an object
    /// backend.
    #[serde(default = "TextureStorageConfig::default_backend")]
    pub backend: String,
    /// Filesystem root for the `local` backend; relative values resolve
    /// against the instance root. Defaults to `<data>/storage`.
    #[serde(default = "TextureStorageConfig::default_local_root")]
    pub local_root: String,
    /// Connection settings for the `s3`/`minio` backends.
    #[serde(default)]
    pub s3: S3TextureStorageConfig,
}

impl Default for TextureStorageConfig {
    fn default() -> Self {
        Self {
            backend: Self::default_backend(),
            local_root: Self::default_local_root(),
            s3: S3TextureStorageConfig::default(),
        }
    }
}

impl TextureStorageConfig {
    fn default_backend() -> String {
        "local".to_string()
    }

    fn default_local_root() -> String {
        "storage".to_string()
    }

    /// Returns the backend selector trimmed and lowercased for comparison.
    pub fn normalized_backend(&self) -> String {
        self.backend.trim().to_ascii_lowercase()
    }
}

impl ValidatableConfig for TextureStorageConfig {
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
}

/// `[object_storage.s3]` section: object-storage connection settings.
#[derive(Debug, Default, Deserialize, Clone)]
pub struct S3TextureStorageConfig {
    /// S3-compatible endpoint base URL (e.g. MinIO); required for non-AWS
    /// providers.
    #[serde(default)]
    pub endpoint: String,
    /// AWS region name for signature v4; required by AWS and most
    /// S3-compatible providers.
    #[serde(default)]
    pub region: String,
    /// Bucket texture objects are written to.
    #[serde(default)]
    pub bucket: String,
    /// Optional bucket prefix. Stored texture keys stay prefix-free in the
    /// database; the S3 backend prepends this only for object-storage calls.
    #[serde(default)]
    pub base_path: String,
    /// Static access key ID for authentication.
    #[serde(default)]
    pub access_key_id: String,
    /// Secret access key paired with `access_key_id`; kept redacted in debug
    /// output.
    #[serde(default)]
    pub secret_access_key: secrecy::SecretString,
    /// Address the bucket as `endpoint/bucket` (path style) instead of
    /// `bucket.endpoint` (virtual-host style); required by MinIO and most
    /// self-hosted S3-compatible stores.
    #[serde(default)]
    pub force_path_style: bool,
}

// Manual impl: same rationale as `AuthConfig` — the default `config.toml`
// writer needs the real secret value, while `Debug` stays redacted.
impl serde::Serialize for S3TextureStorageConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use secrecy::ExposeSecret as _;
        use serde::ser::SerializeStruct as _;
        let mut state = serializer.serialize_struct("S3TextureStorageConfig", 7)?;
        state.serialize_field("endpoint", &self.endpoint)?;
        state.serialize_field("region", &self.region)?;
        state.serialize_field("bucket", &self.bucket)?;
        state.serialize_field("base_path", &self.base_path)?;
        state.serialize_field("access_key_id", &self.access_key_id)?;
        state.serialize_field("secret_access_key", self.secret_access_key.expose_secret())?;
        state.serialize_field("force_path_style", &self.force_path_style)?;
        state.end()
    }
}
