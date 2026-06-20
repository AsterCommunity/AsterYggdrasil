use crate::api::dto::yggdrasil::{YggdrasilProfile, YggdrasilProfileProperty};
use crate::cache::CacheExt;
use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::entities::{minecraft_profile, yggdrasil_session_forward_server, yggdrasil_token};
use crate::runtime::{CacheRuntimeState, RuntimeConfigRuntimeState};
use serde::{Deserialize, Serialize};

const JOIN_SESSION_TTL_SECS: u64 = 30;
const JOIN_SESSION_PREFIX: &str = "yggdrasil:join:";
const PROFILE_NAME_SUMMARY_TTL_SECS: u64 = 300;
const PROFILE_NAME_SUMMARY_PREFIX: &str = "yggdrasil:profile-name-summary:";
const PROFILE_PROPERTIES_TTL_SECS: u64 = 300;
const PROFILE_PROPERTIES_PREFIX: &str = "yggdrasil:profile-properties:";
const SESSION_FORWARD_SERVER_TTL_SECS: u64 = 60;
const SESSION_FORWARD_SERVER_KEY: &str = "yggdrasil:session-forward:enabled:v1";
const TOKEN_TTL_SECS: u64 = 60;
const TOKEN_PREFIX: &str = "yggdrasil:token:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct YggdrasilJoinSession {
    pub(super) profile_id: i64,
    pub(super) profile_uuid: String,
    pub(super) profile_name: String,
    pub(super) server_id: String,
    pub(super) ip_address: Option<String>,
}

pub(super) async fn set_join_session<S>(state: &S, server_id: &str, session: &YggdrasilJoinSession)
where
    S: CacheRuntimeState,
{
    state
        .cache()
        .set(
            &join_session_key(server_id),
            session,
            Some(JOIN_SESSION_TTL_SECS),
        )
        .await;
}

pub(super) async fn get_join_session<S>(state: &S, server_id: &str) -> Option<YggdrasilJoinSession>
where
    S: CacheRuntimeState,
{
    state.cache().get(&join_session_key(server_id)).await
}

pub(super) async fn get_profile_name_summary<S>(state: &S, name: &str) -> Option<YggdrasilProfile>
where
    S: CacheRuntimeState,
{
    state.cache().get(&profile_name_summary_key(name)).await
}

pub(super) async fn set_profile_name_summary<S>(state: &S, profile: &YggdrasilProfile)
where
    S: CacheRuntimeState,
{
    state
        .cache()
        .set(
            &profile_name_summary_key(&profile.name),
            profile,
            Some(PROFILE_NAME_SUMMARY_TTL_SECS),
        )
        .await;
}

pub(super) async fn invalidate_profile_name_summary<S>(state: &S, name: &str)
where
    S: CacheRuntimeState,
{
    state.cache().delete(&profile_name_summary_key(name)).await;
}

pub(super) async fn get_profile_properties<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    signed: bool,
) -> Option<Vec<YggdrasilProfileProperty>>
where
    S: CacheRuntimeState + RuntimeConfigRuntimeState,
{
    state
        .cache()
        .get(&profile_properties_key(state, profile, signed))
        .await
}

pub(super) async fn set_profile_properties<S>(
    state: &S,
    profile: &minecraft_profile::Model,
    signed: bool,
    properties: &[YggdrasilProfileProperty],
) where
    S: CacheRuntimeState + RuntimeConfigRuntimeState,
{
    state
        .cache()
        .set(
            &profile_properties_key(state, profile, signed),
            &properties,
            Some(PROFILE_PROPERTIES_TTL_SECS),
        )
        .await;
}

pub(super) async fn invalidate_profile_properties<S>(state: &S, profile_id: i64)
where
    S: CacheRuntimeState,
{
    state
        .cache()
        .invalidate_prefix(&profile_properties_prefix(profile_id))
        .await;
}

pub(super) async fn get_enabled_session_forward_servers<S>(
    state: &S,
) -> Option<Vec<yggdrasil_session_forward_server::Model>>
where
    S: CacheRuntimeState,
{
    state.cache().get(SESSION_FORWARD_SERVER_KEY).await
}

pub(super) async fn set_enabled_session_forward_servers<S>(
    state: &S,
    servers: &[yggdrasil_session_forward_server::Model],
) where
    S: CacheRuntimeState,
{
    state
        .cache()
        .set(
            SESSION_FORWARD_SERVER_KEY,
            &servers,
            Some(SESSION_FORWARD_SERVER_TTL_SECS),
        )
        .await;
}

pub(super) async fn invalidate_session_forward_servers<S>(state: &S)
where
    S: CacheRuntimeState,
{
    state.cache().delete(SESSION_FORWARD_SERVER_KEY).await;
}

pub(super) async fn get_token<S>(
    state: &S,
    access_token_hash: &str,
) -> Option<yggdrasil_token::Model>
where
    S: CacheRuntimeState,
{
    state.cache().get(&token_key(access_token_hash)).await
}

pub(super) async fn set_token<S>(state: &S, access_token_hash: &str, token: &yggdrasil_token::Model)
where
    S: CacheRuntimeState,
{
    state
        .cache()
        .set(&token_key(access_token_hash), token, Some(TOKEN_TTL_SECS))
        .await;
}

pub(super) async fn invalidate_token<S>(state: &S, access_token_hash: &str)
where
    S: CacheRuntimeState,
{
    state.cache().delete(&token_key(access_token_hash)).await;
}

pub(super) async fn invalidate_tokens<S>(state: &S, access_token_hashes: &[String])
where
    S: CacheRuntimeState,
{
    let keys = access_token_hashes
        .iter()
        .map(|access_token_hash| token_key(access_token_hash))
        .collect::<Vec<_>>();
    state.cache().delete_many(&keys).await;
}

pub(super) async fn invalidate_all_tokens<S>(state: &S)
where
    S: CacheRuntimeState,
{
    state.cache().invalidate_prefix(TOKEN_PREFIX).await;
}

fn profile_name_summary_key(name: &str) -> String {
    format!("{PROFILE_NAME_SUMMARY_PREFIX}{}", name.to_ascii_lowercase())
}

fn profile_properties_prefix(profile_id: i64) -> String {
    format!("{PROFILE_PROPERTIES_PREFIX}{profile_id}:")
}

fn profile_properties_key<S>(state: &S, profile: &minecraft_profile::Model, signed: bool) -> String
where
    S: RuntimeConfigRuntimeState,
{
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    format!(
        "{}{}:{}:{}:{}",
        profile_properties_prefix(profile.id),
        profile.updated_at.timestamp_millis(),
        signed,
        crate::utils::hash::sha256_hex(profile.uploadable_textures.as_bytes()),
        yggdrasil_policy_fingerprint(&policy)
    )
}

fn yggdrasil_policy_fingerprint(policy: &RuntimeYggdrasilPolicy) -> String {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        public_base_urls: &'a [String],
        texture_public_base_url: &'a Option<String>,
        signature_public_key_hash: String,
        signature_private_key_hash: String,
    }

    let payload = serde_json::to_vec(&Fingerprint {
        public_base_urls: &policy.public_base_urls,
        texture_public_base_url: &policy.texture_public_base_url,
        signature_public_key_hash: crate::utils::hash::sha256_hex(
            policy.signature_public_key.as_bytes(),
        ),
        signature_private_key_hash: crate::utils::hash::sha256_hex(
            policy.signature_private_key.as_bytes(),
        ),
    })
    .unwrap_or_else(|error| {
        tracing::warn!(
            error = %error,
            "failed to serialize yggdrasil cache policy fingerprint"
        );
        Vec::new()
    });
    crate::utils::hash::sha256_hex(&payload)
}

fn token_key(access_token_hash: &str) -> String {
    format!("{TOKEN_PREFIX}{access_token_hash}")
}

fn join_session_key(server_id: &str) -> String {
    format!(
        "{JOIN_SESSION_PREFIX}{}",
        crate::utils::hash::sha256_hex(server_id.as_bytes())
    )
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

    impl crate::runtime::CacheRuntimeState for CacheState {
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
    async fn profile_name_summary_cache_is_case_insensitive_for_ascii_names() {
        let state = cache_state().await;
        let profile = YggdrasilProfile {
            id: "0123456789abcdef0123456789abcdef".to_string(),
            name: "Steve".to_string(),
            properties: None,
        };

        set_profile_name_summary(&state, &profile).await;

        for name in ["Steve", "steve", "STEVE"] {
            let cached = get_profile_name_summary(&state, name)
                .await
                .expect("profile summary should be cached case-insensitively");
            assert_eq!(cached.id, profile.id);
            assert_eq!(cached.name, profile.name);
        }
        invalidate_profile_name_summary(&state, "sTeVe").await;
        assert!(get_profile_name_summary(&state, "Steve").await.is_none());
    }

    #[tokio::test]
    async fn profile_name_summary_key_preserves_ascii_safe_distinctions() {
        assert_eq!(
            profile_name_summary_key("Steve_1"),
            profile_name_summary_key("steve_1")
        );
        assert_ne!(
            profile_name_summary_key("Steve_1"),
            profile_name_summary_key("Steve_2")
        );
    }

    #[tokio::test]
    async fn invalidate_tokens_deletes_all_token_entries_and_allows_empty_input() {
        let state = cache_state().await;
        let token = yggdrasil_token::Model {
            id: 1,
            user_id: 1,
            access_token_hash: "hash-1".to_string(),
            client_token: "client".to_string(),
            selected_profile_id: None,
            issued_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            revoked_at: None,
            temporarily_invalidated_at: None,
            user_agent: None,
            ip_address: None,
        };
        set_token(&state, "hash-1", &token).await;
        set_token(
            &state,
            "hash-2",
            &yggdrasil_token::Model {
                id: 2,
                access_token_hash: "hash-2".to_string(),
                ..token.clone()
            },
        )
        .await;
        set_token(
            &state,
            "keep",
            &yggdrasil_token::Model {
                id: 3,
                access_token_hash: "keep".to_string(),
                ..token.clone()
            },
        )
        .await;

        invalidate_tokens(&state, &[]).await;
        assert!(get_token(&state, "hash-1").await.is_some());
        invalidate_tokens(&state, &["hash-1".to_string(), "hash-2".to_string()]).await;

        assert_eq!(get_token(&state, "hash-1").await, None);
        assert_eq!(get_token(&state, "hash-2").await, None);
        assert!(get_token(&state, "keep").await.is_some());
    }
}
