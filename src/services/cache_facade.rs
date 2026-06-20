use crate::cache::CacheExt;
use crate::runtime::CacheRuntimeState;
use serde::{Serialize, de::DeserializeOwned};

pub(crate) async fn get<S, T>(state: &S, key: &str) -> Option<T>
where
    S: CacheRuntimeState,
    T: DeserializeOwned + Send,
{
    state.cache().get(key).await
}

pub(crate) async fn set<S, T>(state: &S, key: &str, value: &T, ttl_secs: Option<u64>)
where
    S: CacheRuntimeState,
    T: Serialize + Send + Sync,
{
    state.cache().set(key, value, ttl_secs).await;
}

pub(crate) async fn delete<S>(state: &S, key: &str)
where
    S: CacheRuntimeState,
{
    state.cache().delete(key).await;
}

pub(crate) async fn take<S, T>(state: &S, key: &str) -> Option<T>
where
    S: CacheRuntimeState,
    T: DeserializeOwned + Send,
{
    state.cache().take(key).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheBackend, create_cache};
    use crate::config::CacheConfig;
    use std::sync::Arc;

    struct CacheState {
        cache: Arc<dyn CacheBackend>,
    }

    impl CacheRuntimeState for CacheState {
        fn cache(&self) -> &Arc<dyn CacheBackend> {
            &self.cache
        }
    }

    async fn cache_state() -> CacheState {
        CacheState {
            cache: create_cache(&CacheConfig::default()).await,
        }
    }

    #[tokio::test]
    async fn take_deserializes_and_consumes_value_once() {
        let state = cache_state().await;
        set(&state, "challenge", &"value".to_string(), Some(60)).await;

        assert_eq!(
            take::<_, String>(&state, "challenge").await,
            Some("value".to_string())
        );
        assert_eq!(take::<_, String>(&state, "challenge").await, None);
        assert_eq!(get::<_, String>(&state, "challenge").await, None);
    }

    #[tokio::test]
    async fn take_returns_none_for_missing_or_invalid_json() {
        let state = cache_state().await;

        assert_eq!(take::<_, String>(&state, "missing").await, None);
        state
            .cache()
            .set_bytes("invalid", b"not-json".to_vec(), Some(60))
            .await;

        assert_eq!(take::<_, String>(&state, "invalid").await, None);
        assert_eq!(state.cache().get_bytes("invalid").await, None);
    }
}
