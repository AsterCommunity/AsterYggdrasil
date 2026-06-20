use crate::cache::CacheExt;
use crate::runtime::CacheRuntimeState;

use super::{TextureBlob, is_valid_texture_hash};

const TEXTURE_BLOB_LOOKUP_PREFIX: &str = "minecraft-texture:blob:";
const TEXTURE_BLOB_LOOKUP_TTL_SECS: u64 = 600;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct CachedTextureBlob {
    storage_key: String,
}

pub(super) async fn get_texture_blob_lookup<S>(state: &S, hash: &str) -> Option<TextureBlob>
where
    S: CacheRuntimeState,
{
    if !is_valid_texture_hash(hash) {
        return None;
    }
    state
        .cache()
        .get::<CachedTextureBlob>(&texture_blob_lookup_key(hash))
        .await
        .map(|cached| TextureBlob {
            storage_key: cached.storage_key,
        })
}

pub(super) async fn set_texture_blob_lookup<S>(state: &S, hash: &str, blob: &TextureBlob)
where
    S: CacheRuntimeState,
{
    if !is_valid_texture_hash(hash) {
        return;
    }
    state
        .cache()
        .set(
            &texture_blob_lookup_key(hash),
            &CachedTextureBlob {
                storage_key: blob.storage_key.clone(),
            },
            Some(TEXTURE_BLOB_LOOKUP_TTL_SECS),
        )
        .await;
}

pub(super) async fn invalidate_texture_blob_lookup<S>(state: &S, hash: &str)
where
    S: CacheRuntimeState,
{
    if is_valid_texture_hash(hash) {
        state.cache().delete(&texture_blob_lookup_key(hash)).await;
    }
}

fn texture_blob_lookup_key(hash: &str) -> String {
    format!("{TEXTURE_BLOB_LOOKUP_PREFIX}{hash}")
}
