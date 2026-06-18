use crate::db::repository::minecraft_texture_repo;
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, ObjectStorageRuntimeState};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanTextureCleanupResult {
    pub scanned: u64,
    pub deleted: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStorageConsistencyIssue {
    pub texture_id: i64,
    pub storage_key: String,
    pub hash: String,
    pub kind: ObjectStorageConsistencyIssueKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStorageConsistencyIssueKind {
    MissingObject,
    HashMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStorageConsistencyReport {
    pub checked: u64,
    pub missing: u64,
    pub hash_mismatched: u64,
    pub issues: Vec<ObjectStorageConsistencyIssue>,
}

pub async fn cleanup_orphan_texture_blobs<S>(state: &S) -> Result<OrphanTextureCleanupResult>
where
    S: DatabaseRuntimeState + ObjectStorageRuntimeState,
{
    let storage_keys = state.object_storage().list_keys("").await?;
    let texture_storage_keys = storage_keys
        .into_iter()
        .filter(|storage_key| !storage_key.starts_with("texture-previews/"))
        .collect::<Vec<_>>();
    let mut result = OrphanTextureCleanupResult {
        scanned: crate::utils::numbers::usize_to_u64(
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

        state.object_storage().delete(&storage_key).await?;
        result.deleted += 1;
    }

    Ok(result)
}

pub async fn check_object_storage_consistency<S>(
    state: &S,
) -> Result<ObjectStorageConsistencyReport>
where
    S: DatabaseRuntimeState + ObjectStorageRuntimeState,
{
    let textures = minecraft_texture_repo::list_all(state.reader_db()).await?;
    let storage_keys = state.object_storage().list_keys("").await?;
    let storage_key_set = storage_keys.into_iter().collect::<HashSet<_>>();
    let mut report = ObjectStorageConsistencyReport {
        checked: crate::utils::numbers::usize_to_u64(
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
            report.issues.push(ObjectStorageConsistencyIssue {
                texture_id: texture.id,
                storage_key: texture.storage_key,
                hash: texture.hash,
                kind: ObjectStorageConsistencyIssueKind::MissingObject,
            });
            continue;
        }

        if !object_storage_key_matches_hash(&texture.storage_key, &texture.hash) {
            report.hash_mismatched += 1;
            report.issues.push(ObjectStorageConsistencyIssue {
                texture_id: texture.id,
                storage_key: texture.storage_key,
                hash: texture.hash,
                kind: ObjectStorageConsistencyIssueKind::HashMismatch,
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

pub(super) async fn cleanup_texture_blob_if_unreferenced<S>(
    state: &S,
    storage_key: &str,
    reason: &str,
) where
    S: DatabaseRuntimeState + ObjectStorageRuntimeState,
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

    match state.object_storage().delete(storage_key).await {
        Ok(()) => {}
        Err(error) => match state.object_storage().exists(storage_key).await {
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
