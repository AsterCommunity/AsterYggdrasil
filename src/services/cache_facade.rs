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
    let value = get(state, key).await;
    delete(state, key).await;
    value
}
