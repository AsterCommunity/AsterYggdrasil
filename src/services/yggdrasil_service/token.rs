use actix_web::HttpRequest;
use chrono::{Duration, Utc};

use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_repo, yggdrasil_token_repo};
use crate::entities::{minecraft_profile, yggdrasil_token};
use crate::errors::Result;
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState};
use crate::services::ban_service;
use crate::types::UserBanScope;
use crate::utils::hash::sha256_hex;

use super::error::{YggdrasilError, YggdrasilErrorKind};

pub(super) struct IssuedToken {
    pub(super) token_id: i64,
    pub(super) access_token: String,
}

async fn issue_token_in_connection<C: sea_orm::ConnectionTrait>(
    db: &C,
    user_id: i64,
    client_token: &str,
    selected_profile_id: Option<i64>,
    policy: &RuntimeYggdrasilPolicy,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<(IssuedToken, Vec<String>)> {
    let now = Utc::now();
    let token_ttl_days =
        crate::utils::numbers::u64_to_i64(policy.token_ttl_days, "yggdrasil token ttl days")?;
    let access_token = crate::utils::id::new_unsigned_uuid();
    tracing::debug!(
        user_id,
        selected_profile_id,
        token_ttl_days,
        max_active_tokens = policy.max_active_tokens,
        has_user_agent = user_agent.is_some(),
        has_ip_address = ip_address.is_some(),
        "creating yggdrasil token"
    );
    let token = yggdrasil_token_repo::create(
        db,
        yggdrasil_token_repo::CreateYggdrasilToken {
            user_id,
            access_token_hash: &sha256_hex(access_token.as_bytes()),
            client_token,
            selected_profile_id,
            issued_at: now,
            expires_at: now + Duration::days(token_ttl_days),
            user_agent,
            ip_address,
        },
    )
    .await?;
    let pruned_hashes =
        yggdrasil_token_repo::prune_oldest_for_user(db, user_id, policy.max_active_tokens).await?;
    tracing::debug!(
        user_id,
        token_id = token.id,
        selected_profile_id,
        "created yggdrasil token"
    );

    Ok((
        IssuedToken {
            token_id: token.id,
            access_token,
        },
        pruned_hashes,
    ))
}

pub(super) async fn issue_token<S>(
    state: &S,
    user_id: i64,
    client_token: &str,
    selected_profile_id: Option<i64>,
    policy: &RuntimeYggdrasilPolicy,
    req: &HttpRequest,
) -> std::result::Result<IssuedToken, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let user_agent = crate::services::auth_service::user_agent(req);
    let ip_address = crate::services::auth_service::peer_ip(req);
    tracing::debug!(
        user_id,
        selected_profile_id,
        has_user_agent = user_agent.is_some(),
        has_ip_address = ip_address.is_some(),
        "issuing yggdrasil token"
    );
    let (issued, pruned_hashes) =
        crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
            issue_token_in_connection(
                txn,
                user_id,
                client_token,
                selected_profile_id,
                policy,
                user_agent.clone(),
                ip_address.clone(),
            )
            .await
        })
        .await
        .map_err(YggdrasilError::from)?;
    invalidate_token_cache_hashes(state, &pruned_hashes).await;
    Ok(issued)
}

pub(super) async fn refresh_token<S>(
    state: &S,
    old_access_token: &str,
    user_id: i64,
    client_token: &str,
    selected_profile_id: Option<i64>,
    policy: &RuntimeYggdrasilPolicy,
    req: &HttpRequest,
) -> std::result::Result<IssuedToken, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let user_agent = crate::services::auth_service::user_agent(req);
    let ip_address = crate::services::auth_service::peer_ip(req);
    let old_access_token_hash = sha256_hex(old_access_token.as_bytes());
    tracing::debug!(
        user_id,
        selected_profile_id,
        has_user_agent = user_agent.is_some(),
        has_ip_address = ip_address.is_some(),
        "refreshing yggdrasil token"
    );
    // authlib-injector/Yggdrasil refresh requires failure to leave the
    // original token valid, so revocation and replacement issuance must commit
    // or roll back together.
    let txn = crate::db::transaction::begin(state.writer_db())
        .await
        .map_err(YggdrasilError::from)?;

    let result = async {
        let revoked = yggdrasil_token_repo::revoke_by_access_hash(&txn, &old_access_token_hash)
            .await
            .map_err(YggdrasilError::from)?;
        if !revoked {
            tracing::debug!(user_id, "yggdrasil refresh could not revoke old token");
            return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
        }
        issue_token_in_connection(
            &txn,
            user_id,
            client_token,
            selected_profile_id,
            policy,
            user_agent,
            ip_address,
        )
        .await
        .map_err(YggdrasilError::from)
    }
    .await;

    match result {
        Ok((issued, pruned_hashes)) => {
            crate::db::transaction::commit(txn)
                .await
                .map_err(YggdrasilError::from)?;
            let mut hashes = pruned_hashes;
            hashes.push(old_access_token_hash);
            invalidate_token_cache_hashes(state, &hashes).await;
            tracing::debug!(
                user_id,
                token_id = issued.token_id,
                selected_profile_id,
                "yggdrasil token refresh transaction committed"
            );
            Ok(issued)
        }
        Err(error) => {
            if let Err(rollback_error) = crate::db::transaction::rollback(txn).await {
                tracing::error!(
                    error = %error.protocol_message(),
                    rollback_error = %rollback_error,
                    "failed to roll back yggdrasil refresh transaction"
                );
            }
            Err(error)
        }
    }
}

pub async fn active_token_for_protocol<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<yggdrasil_token::Model, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    active_token(state, access_token, None).await
}

pub async fn cleanup_expired_or_revoked_tokens<S>(state: &S) -> Result<u64>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let removed =
        yggdrasil_token_repo::delete_expired_or_revoked(state.writer_db(), Utc::now()).await?;
    super::cache::invalidate_all_tokens(state).await;
    tracing::debug!(removed, "cleaned up expired or revoked yggdrasil tokens");
    Ok(removed)
}

pub(super) async fn active_token<S>(
    state: &S,
    access_token: &str,
    client_token: Option<&str>,
) -> std::result::Result<yggdrasil_token::Model, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    token_for_operation(state, access_token, client_token, false).await
}

pub(super) async fn refreshable_token<S>(
    state: &S,
    access_token: &str,
    client_token: Option<&str>,
) -> std::result::Result<yggdrasil_token::Model, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    token_for_operation(state, access_token, client_token, true).await
}

async fn token_for_operation<S>(
    state: &S,
    access_token: &str,
    client_token: Option<&str>,
    allow_temporarily_invalid: bool,
) -> std::result::Result<yggdrasil_token::Model, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let access_token_hash = sha256_hex(access_token.as_bytes());
    let token = match super::cache::get_token(state, &access_token_hash).await {
        Some(token) => {
            tracing::debug!(token_id = token.id, "yggdrasil token cache hit");
            token
        }
        None => {
            let Some(token) =
                yggdrasil_token_repo::find_by_access_hash(state.reader_db(), &access_token_hash)
                    .await
                    .map_err(YggdrasilError::from)?
            else {
                tracing::debug!("yggdrasil token lookup missed");
                return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
            };
            super::cache::set_token(state, &access_token_hash, &token).await;
            token
        }
    };
    if token.revoked_at.is_some() || token.expires_at <= Utc::now() {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            revoked = token.revoked_at.is_some(),
            expired = token.expires_at <= Utc::now(),
            "yggdrasil token rejected because it is inactive"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }
    if token.temporarily_invalidated_at.is_some() && !allow_temporarily_invalid {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "yggdrasil token rejected because it is temporarily invalid"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }
    if let Some(client_token) = client_token
        && token.client_token != client_token
    {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "yggdrasil token rejected because client token did not match"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }
    if ban_service::is_user_banned(state, token.user_id, UserBanScope::YggdrasilAccess)
        .await
        .map_err(YggdrasilError::from)?
    {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "yggdrasil token rejected because user is banned from yggdrasil access"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }
    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        selected_profile_id = token.selected_profile_id,
        "yggdrasil token resolved"
    );
    Ok(token)
}

pub(super) async fn selected_profile_for_token<S: DatabaseRuntimeState>(
    state: &S,
    token: Option<&yggdrasil_token::Model>,
) -> std::result::Result<Option<minecraft_profile::Model>, YggdrasilError> {
    let Some(profile_id) = token.and_then(|token| token.selected_profile_id) else {
        tracing::debug!("yggdrasil token has no selected profile");
        return Ok(None);
    };
    let profile = minecraft_profile_repo::find_by_id(state.reader_db(), profile_id)
        .await
        .map_err(YggdrasilError::from)?;
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        "loaded yggdrasil selected profile for token"
    );
    Ok(Some(profile))
}

pub(super) async fn revoke_by_access_token<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<(), YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let access_token_hash = sha256_hex(access_token.as_bytes());
    let revoked =
        yggdrasil_token_repo::revoke_by_access_hash(state.writer_db(), &access_token_hash)
            .await
            .map_err(YggdrasilError::from)?;
    invalidate_token_cache_by_hash(state, &access_token_hash).await;
    tracing::debug!(revoked, "revoked yggdrasil token by access hash");
    Ok(())
}

pub(super) async fn revoke_all_for_user<S>(
    state: &S,
    user_id: i64,
) -> std::result::Result<(), YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let hashes = yggdrasil_token_repo::list_active_hashes_for_user(state.reader_db(), user_id)
        .await
        .map_err(YggdrasilError::from)?;
    let revoked = yggdrasil_token_repo::revoke_all_for_user(state.writer_db(), user_id)
        .await
        .map_err(YggdrasilError::from)?;
    invalidate_token_cache_hashes(state, &hashes).await;
    tracing::debug!(user_id, revoked, "revoked all yggdrasil tokens for user");
    Ok(())
}

pub(crate) async fn invalidate_token_cache_by_hash<S>(state: &S, access_token_hash: &str)
where
    S: CacheRuntimeState,
{
    super::cache::invalidate_token(state, access_token_hash).await;
}

pub(crate) async fn invalidate_token_cache_hashes<S>(state: &S, access_token_hashes: &[String])
where
    S: CacheRuntimeState,
{
    super::cache::invalidate_tokens(state, access_token_hashes).await;
}

pub(crate) async fn invalidate_token_cache_for_user<S>(
    state: &S,
    user_id: i64,
) -> Result<Vec<String>>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let hashes =
        yggdrasil_token_repo::list_active_hashes_for_user(state.reader_db(), user_id).await?;
    invalidate_token_cache_hashes(state, &hashes).await;
    Ok(hashes)
}

pub(crate) async fn invalidate_token_cache_for_selected_profile<S>(
    state: &S,
    selected_profile_id: i64,
) -> Result<Vec<String>>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let hashes = yggdrasil_token_repo::list_active_hashes_for_selected_profile(
        state.reader_db(),
        selected_profile_id,
    )
    .await?;
    invalidate_token_cache_hashes(state, &hashes).await;
    Ok(hashes)
}
