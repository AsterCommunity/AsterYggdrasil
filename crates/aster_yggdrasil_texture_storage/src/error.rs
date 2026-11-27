//! Structured errors reported by [`TextureStore`](crate::TextureStore)
//! implementations.

use std::fmt;

/// The failure modes a texture store can report to its host.
///
/// Hosts map these onto their own error types; the variants are exhaustive so
/// implementations cannot leak backend-specific error shapes into the
/// contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureStoreError {
    /// The requested object does not exist. `context` names the failed
    /// operation (e.g. `open object`).
    NotFound {
        /// Human-readable description of the failed operation.
        context: String,
    },
    /// The backend failed to complete the operation (filesystem I/O, S3 SDK
    /// error). `context` carries the operation name and backend detail.
    Backend {
        /// Human-readable description of the failure.
        context: String,
    },
    /// A storage key or prefix failed validation.
    InvalidKey {
        /// Why the key or prefix was rejected.
        reason: String,
    },
}

impl TextureStoreError {
    /// Builds a [`TextureStoreError::NotFound`] from an operation context.
    pub fn not_found(context: impl Into<String>) -> Self {
        Self::NotFound {
            context: context.into(),
        }
    }

    /// Builds a [`TextureStoreError::Backend`] from an operation context.
    pub fn backend(context: impl Into<String>) -> Self {
        Self::Backend {
            context: context.into(),
        }
    }

    /// Builds a [`TextureStoreError::InvalidKey`] from a rejection reason.
    pub fn invalid_key(reason: impl Into<String>) -> Self {
        Self::InvalidKey {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for TextureStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { context } => write!(f, "{context}"),
            Self::Backend { context } => write!(f, "{context}"),
            Self::InvalidKey { reason } => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for TextureStoreError {}

/// Forge storage-core key/prefix validation failures map to
/// [`TextureStoreError::InvalidKey`], so store implementations can use
/// `normalize_object_key(...)?` directly.
impl From<aster_forge_storage_core::StorageCoreError> for TextureStoreError {
    fn from(error: aster_forge_storage_core::StorageCoreError) -> Self {
        Self::InvalidKey {
            reason: error.to_string(),
        }
    }
}
