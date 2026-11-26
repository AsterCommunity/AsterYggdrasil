use crate::db::repository::minecraft_texture_repo;
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, TextureStorageRuntimeState};
use std::collections::HashSet;

/// Counters from an orphan texture-blob cleanup sweep over object storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanTextureCleanupResult {
    /// Texture-looking blobs found under the texture prefix.
    pub scanned: u64,
    /// Blobs deleted because no `minecraft_texture` row referenced them.
    pub deleted: u64,
    /// Blobs kept because at least one texture row still references them.
    pub skipped: u64,
}

/// One texture row whose stored object failed a consistency check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureStoreConsistencyIssue {
    /// The `minecraft_texture` row id whose object is inconsistent.
    pub texture_id: i64,
    /// Object-storage key recorded on the texture row.
    pub storage_key: String,
    /// Content hash recorded on the texture row.
    pub hash: String,
    /// Which check the texture failed.
    pub kind: TextureStoreConsistencyIssueKind,
}

/// The two ways a texture row's stored object can be inconsistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureStoreConsistencyIssueKind {
    /// No object exists at the row's storage key.
    MissingObject,
    /// The object exists but its key does not match the row's content hash
    /// (`<hash[..2]>/<hash>.png` layout).
    HashMismatch,
}

/// Aggregated result of a full texture-store consistency check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureStoreConsistencyReport {
    /// Number of `minecraft_texture` rows examined.
    pub checked: u64,
    /// Rows whose object is missing from storage.
    pub missing: u64,
    /// Rows whose storage key does not match their content hash.
    pub hash_mismatched: u64,
    /// Per-row details for every inconsistency found.
    pub issues: Vec<TextureStoreConsistencyIssue>,
}

/// Deletes texture blobs in object storage that no `minecraft_texture` row
/// references anymore, and returns per-outcome counts.
pub async fn cleanup_orphan_texture_blobs<S>(state: &S) -> Result<OrphanTextureCleanupResult>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let storage_keys = state.texture_storage().list_keys("").await?;
    let texture_storage_keys = storage_keys
        .into_iter()
        .filter(|storage_key| is_texture_blob_storage_key(storage_key))
        .collect::<Vec<_>>();
    let mut result = OrphanTextureCleanupResult {
        scanned: aster_forge_utils::numbers::usize_to_u64(
            texture_storage_keys.len(),
            "orphan texture cleanup scanned count",
        )?,
        deleted: 0,
        skipped: 0,
    };

    for storage_key in texture_storage_keys {
        let references = count_object_storage_references(state, &storage_key).await?;
        if references > 0 {
            result.skipped += 1;
            continue;
        }

        state.texture_storage().delete(&storage_key).await?;
        result.deleted += 1;
    }

    Ok(result)
}

/// Verifies every stored texture row against object storage: each object must
/// exist and live at the content-addressed key derived from the row's hash.
pub async fn check_texture_storage_consistency<S>(
    state: &S,
) -> Result<TextureStoreConsistencyReport>
where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let textures = minecraft_texture_repo::list_all(state.reader_db()).await?;
    let storage_keys = state.texture_storage().list_keys("").await?;
    let storage_key_set = storage_keys.into_iter().collect::<HashSet<_>>();
    let mut report = TextureStoreConsistencyReport {
        checked: aster_forge_utils::numbers::usize_to_u64(
            textures.len(),
            "object storage consistency checked count",
        )?,
        missing: 0,
        hash_mismatched: 0,
        issues: Vec::new(),
    };

    for texture in textures {
        if !storage_key_set.contains(&texture.storage_key) {
            report.missing += 1;
            report.issues.push(TextureStoreConsistencyIssue {
                texture_id: texture.id,
                storage_key: texture.storage_key,
                hash: texture.hash,
                kind: TextureStoreConsistencyIssueKind::MissingObject,
            });
            continue;
        }

        if !object_storage_key_matches_hash(&texture.storage_key, &texture.hash) {
            report.hash_mismatched += 1;
            report.issues.push(TextureStoreConsistencyIssue {
                texture_id: texture.id,
                storage_key: texture.storage_key,
                hash: texture.hash,
                kind: TextureStoreConsistencyIssueKind::HashMismatch,
            });
        }
    }

    Ok(report)
}

fn object_storage_key_matches_hash(storage_key: &str, hash: &str) -> bool {
    let Some(prefix) = hash.get(..2) else {
        return false;
    };
    storage_key == format!("{prefix}/{hash}.png")
}

fn is_texture_blob_storage_key(storage_key: &str) -> bool {
    let Some((prefix, file_name)) = storage_key.split_once('/') else {
        return false;
    };
    if prefix.len() != 2 || !prefix.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return false;
    }

    let Some(hash) = file_name.strip_suffix(".png") else {
        return false;
    };
    hash.len() == 64
        && hash.starts_with(prefix)
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) async fn cleanup_texture_blob_if_unreferenced<S>(
    state: &S,
    storage_key: &str,
    reason: &str,
) where
    S: DatabaseRuntimeState + TextureStorageRuntimeState,
{
    let ref_count = match count_object_storage_references(state, storage_key).await {
        Ok(ref_count) => ref_count,
        Err(error) => {
            tracing::warn!(
                error = %error,
                storage_key,
                reason,
                "failed to count texture blob references before cleanup"
            );
            return;
        }
    };
    if ref_count != 0 {
        tracing::debug!(
            storage_key,
            ref_count,
            reason,
            "skipping texture blob cleanup because it is still referenced"
        );
        return;
    }

    match state.texture_storage().delete(storage_key).await {
        Ok(()) => {}
        Err(error) => match state.texture_storage().exists(storage_key).await {
            Ok(false) => {
                tracing::warn!(
                    error = %error,
                    storage_key,
                    reason,
                    "texture blob delete failed but object is already absent"
                );
            }
            Ok(true) => {
                tracing::warn!(
                    error = %error,
                    storage_key,
                    reason,
                    "failed to delete unreferenced texture blob"
                );
            }
            Err(exists_error) => {
                tracing::warn!(
                    error = %error,
                    exists_error = %exists_error,
                    storage_key,
                    reason,
                    "failed to delete unreferenced texture blob and verify existence"
                );
            }
        },
    }
}

async fn count_object_storage_references<S>(state: &S, storage_key: &str) -> Result<u64>
where
    S: DatabaseRuntimeState,
{
    minecraft_texture_repo::count_by_storage_key(state.writer_db(), storage_key).await
}
