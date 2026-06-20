use actix_web::HttpRequest;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use validator::Validate;

use crate::api::dto::yggdrasil::{YggdrasilJoinReq, YggdrasilProfile, YggdrasilProfileProperty};
use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_repo, yggdrasil_session_forward_server_repo};
use crate::entities::yggdrasil_session_forward_server;
use crate::runtime::{
    AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState,
    YggdrasilSessionForwardRuntimeState,
};
use crate::services::{audit_service, ban_service, yggdrasil_signature};
use crate::types::UserBanScope;
use crate::types::{YggdrasilSessionForwardEndpointKind, YggdrasilSessionForwardProviderKind};
use crate::utils::hash::sha256_hex;

use super::error::{YggdrasilError, YggdrasilErrorKind};
use super::properties;
use super::token::active_token;

const DEFAULT_FORWARD_TIMEOUT_MS: u64 = 1500;
const MIN_FORWARD_TIMEOUT_MS: u64 = 100;
const MAX_FORWARD_TIMEOUT_MS: u64 = 10_000;

pub async fn join<S>(
    state: &S,
    body: YggdrasilJoinReq,
    req: &HttpRequest,
) -> std::result::Result<(), YggdrasilError>
where
    S: AppConfigRuntimeState + DatabaseRuntimeState + RuntimeConfigRuntimeState + CacheRuntimeState,
{
    let server_id_hash = sha256_hex(body.server_id.as_bytes());
    tracing::debug!(
        selected_profile_uuid = %body.selected_profile,
        server_id_hash = %server_id_hash,
        "starting yggdrasil join"
    );
    let token = active_token(state, &body.access_token, None).await?;
    let Some(selected_profile_id) = token.selected_profile_id else {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "yggdrasil join rejected because token has no selected profile"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    };
    if ban_service::is_user_banned(state, token.user_id, UserBanScope::YggdrasilJoin)
        .await
        .map_err(YggdrasilError::from)?
    {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "yggdrasil join rejected because user is banned from joining servers"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }
    let profile = minecraft_profile_repo::find_by_id(state.reader_db(), selected_profile_id)
        .await
        .map_err(YggdrasilError::from)?;
    if profile.uuid != body.selected_profile {
        tracing::debug!(
            token_id = token.id,
            profile_id = profile.id,
            expected_profile_uuid = %profile.uuid,
            requested_profile_uuid = %body.selected_profile,
            "yggdrasil join rejected because selected profile did not match token"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }

    let session = super::cache::YggdrasilJoinSession {
        profile_id: profile.id,
        profile_uuid: profile.uuid.clone(),
        profile_name: profile.name.clone(),
        server_id: body.server_id.clone(),
        ip_address: real_join_client_ip(state, req),
    };
    super::cache::set_join_session(state, &body.server_id, &session).await;
    let ctx = audit_service::AuditContext::from_request(req, token.user_id);
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::YggdrasilJoinServer,
        audit_service::AuditEntityType::YggdrasilSession,
        Some(profile.id),
        Some(&profile.name),
        || {
            audit_service::details(audit_service::YggdrasilJoinAuditDetails {
                profile_uuid: &profile.uuid,
                profile_name: &profile.name,
                server_id_hash: &server_id_hash,
            })
        },
    )
    .await;
    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        server_id_hash = %server_id_hash,
        has_ip_address = session.ip_address.is_some(),
        "yggdrasil join recorded"
    );
    Ok(())
}

fn real_join_client_ip<S: AppConfigRuntimeState>(state: &S, req: &HttpRequest) -> Option<String> {
    let peer = req.peer_addr()?.ip();
    // The authlib-injector hasJoined `ip` parameter is the client IP observed
    // by the Minecraft server. When the Yggdrasil server is behind a trusted
    // reverse proxy, record the same forwarded client IP instead of the proxy
    // socket address so prevent-proxy-connections checks can match.
    Some(
        crate::utils::net::real_ip_from_headers(
            req.headers(),
            peer,
            &state.config().network_trust.trusted_proxies,
        )
        .to_string(),
    )
}

pub async fn has_joined<S>(
    state: &S,
    username: &str,
    server_id: &str,
    ip: Option<&str>,
    request_info: audit_service::AuditRequestInfo,
) -> std::result::Result<Option<YggdrasilProfile>, YggdrasilError>
where
    S: DatabaseRuntimeState
        + AppConfigRuntimeState
        + RuntimeConfigRuntimeState
        + CacheRuntimeState
        + YggdrasilSessionForwardRuntimeState,
{
    let servers = enabled_session_forward_servers(state).await?;
    if servers.is_empty() {
        return local_has_joined(state, username, server_id, ip).await;
    }
    orchestrated_has_joined(state, servers, username, server_id, ip, &request_info).await
}

pub(crate) async fn invalidate_session_forward_server_cache<S>(state: &S)
where
    S: CacheRuntimeState,
{
    super::cache::invalidate_session_forward_servers(state).await;
}

async fn enabled_session_forward_servers<S>(
    state: &S,
) -> std::result::Result<Vec<yggdrasil_session_forward_server::Model>, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    if let Some(servers) = super::cache::get_enabled_session_forward_servers(state).await {
        tracing::debug!(
            count = servers.len(),
            "yggdrasil session forward server cache hit"
        );
        return Ok(servers);
    }

    let servers = yggdrasil_session_forward_server_repo::list_enabled_ordered(state.reader_db())
        .await
        .map_err(YggdrasilError::from)?;
    super::cache::set_enabled_session_forward_servers(state, &servers).await;
    tracing::debug!(
        count = servers.len(),
        "cached yggdrasil session forward server list"
    );
    Ok(servers)
}

async fn local_has_joined<S>(
    state: &S,
    username: &str,
    server_id: &str,
    ip: Option<&str>,
) -> std::result::Result<Option<YggdrasilProfile>, YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + CacheRuntimeState,
{
    let server_id_hash = sha256_hex(server_id.as_bytes());
    tracing::debug!(
        username,
        server_id_hash = %server_id_hash,
        has_ip = ip.is_some(),
        "checking yggdrasil hasJoined"
    );
    let Some(session) = super::cache::get_join_session(state, server_id).await else {
        tracing::debug!(
            username,
            server_id_hash = %server_id_hash,
            "yggdrasil hasJoined cache miss"
        );
        return Ok(None);
    };
    if session.server_id != server_id || session.profile_name != username {
        tracing::debug!(
            username,
            cached_profile_name = %session.profile_name,
            server_id_hash = %server_id_hash,
            "yggdrasil hasJoined cache record did not match request"
        );
        return Ok(None);
    }
    if let Some(ip) = ip
        && session.ip_address.as_deref() != Some(ip)
    {
        tracing::debug!(
            username,
            server_id_hash = %server_id_hash,
            has_cached_ip = session.ip_address.is_some(),
            "yggdrasil hasJoined rejected because client ip did not match"
        );
        return Ok(None);
    }

    let profile = minecraft_profile_repo::find_by_id(state.reader_db(), session.profile_id)
        .await
        .map_err(YggdrasilError::from)?;
    if ban_service::is_user_banned(state, profile.user_id, UserBanScope::YggdrasilJoin)
        .await
        .map_err(YggdrasilError::from)?
    {
        tracing::debug!(
            profile_id = profile.id,
            user_id = profile.user_id,
            server_id_hash = %server_id_hash,
            "yggdrasil hasJoined ignored cached session because user is banned from joining servers"
        );
        return Ok(None);
    }
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        server_id_hash = %server_id_hash,
        "yggdrasil hasJoined matched profile"
    );
    Ok(Some(
        properties::profile_with_properties(state, &profile, true).await?,
    ))
}

async fn orchestrated_has_joined<S>(
    state: &S,
    servers: Vec<yggdrasil_session_forward_server::Model>,
    username: &str,
    server_id: &str,
    ip: Option<&str>,
    request_info: &audit_service::AuditRequestInfo,
) -> std::result::Result<Option<YggdrasilProfile>, YggdrasilError>
where
    S: DatabaseRuntimeState
        + AppConfigRuntimeState
        + RuntimeConfigRuntimeState
        + CacheRuntimeState
        + YggdrasilSessionForwardRuntimeState,
{
    let server_id_hash = sha256_hex(server_id.as_bytes());
    for upstream in weighted_forward_order(servers) {
        let checked_at = Utc::now();
        tracing::debug!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            provider_kind = %upstream.provider_kind.as_str(),
            endpoint_kind = %upstream.endpoint_kind.as_str(),
            server_id_hash = %server_id_hash,
            username,
            has_ip = ip.is_some(),
            "checking yggdrasil hasJoined upstream"
        );
        let result = match upstream.provider_kind {
            YggdrasilSessionForwardProviderKind::Local => {
                local_has_joined(state, username, server_id, ip)
                    .await
                    .map_err(|error| error.protocol_message())
            }
            YggdrasilSessionForwardProviderKind::Remote => {
                query_forward_server(
                    state.yggdrasil_session_forward_http_client(),
                    &upstream,
                    username,
                    server_id,
                    ip,
                )
                .await
            }
        };
        match result {
            Ok(Some(mut profile)) => {
                if upstream.texture_forward_enabled {
                    match rewrite_forwarded_texture_urls(state, &upstream, &mut profile).await {
                        Ok(rewritten_count) => {
                            tracing::debug!(
                                upstream_id = upstream.id,
                                upstream_name = %upstream.display_name,
                                profile_uuid = %profile.id,
                                rewritten_count,
                                "finished forwarded yggdrasil texture URL rewrite"
                            );
                        }
                        Err(error) => {
                            tracing::warn!(
                                error = %error,
                                upstream_id = upstream.id,
                                upstream_name = %upstream.display_name,
                                "failed to rewrite forwarded yggdrasil texture URLs"
                            );
                        }
                    }
                }
                if let Err(error) = yggdrasil_session_forward_server_repo::mark_success(
                    state.writer_db(),
                    upstream.id,
                    checked_at,
                )
                .await
                {
                    tracing::warn!(
                        error = %error,
                        upstream_id = upstream.id,
                        "failed to record yggdrasil session forward success"
                    );
                }
                tracing::debug!(
                    upstream_id = upstream.id,
                    upstream_name = %upstream.display_name,
                    profile_uuid = %profile.id,
                    server_id_hash = %server_id_hash,
                    "yggdrasil hasJoined forwarded profile matched"
                );
                log_forward_check(
                    state,
                    request_info,
                    ForwardCheckAuditEvent {
                        username,
                        server_id_hash: &server_id_hash,
                        upstream: &upstream,
                        result: "matched",
                        profile_uuid: Some(&profile.id),
                        error: None,
                    },
                )
                .await;
                return Ok(Some(profile));
            }
            Ok(None) => {
                if let Err(error) = yggdrasil_session_forward_server_repo::mark_success(
                    state.writer_db(),
                    upstream.id,
                    checked_at,
                )
                .await
                {
                    tracing::warn!(
                        error = %error,
                        upstream_id = upstream.id,
                        "failed to record yggdrasil session forward no-match check"
                    );
                }
                tracing::debug!(
                    upstream_id = upstream.id,
                    upstream_name = %upstream.display_name,
                    server_id_hash = %server_id_hash,
                    "yggdrasil hasJoined upstream returned no matching session"
                );
                log_forward_check(
                    state,
                    request_info,
                    ForwardCheckAuditEvent {
                        username,
                        server_id_hash: &server_id_hash,
                        upstream: &upstream,
                        result: "no_match",
                        profile_uuid: None,
                        error: None,
                    },
                )
                .await;
            }
            Err(error) => {
                if let Err(record_error) = yggdrasil_session_forward_server_repo::mark_failure(
                    state.writer_db(),
                    upstream.id,
                    checked_at,
                    &error,
                )
                .await
                {
                    tracing::warn!(
                        error = %record_error,
                        upstream_id = upstream.id,
                        "failed to record yggdrasil session forward failure"
                    );
                }
                tracing::warn!(
                    upstream_id = upstream.id,
                    upstream_name = %upstream.display_name,
                    error = %error,
                    server_id_hash = %server_id_hash,
                    "yggdrasil hasJoined upstream failed"
                );
                log_forward_check(
                    state,
                    request_info,
                    ForwardCheckAuditEvent {
                        username,
                        server_id_hash: &server_id_hash,
                        upstream: &upstream,
                        result: "failed",
                        profile_uuid: None,
                        error: Some(&error),
                    },
                )
                .await;
            }
        }
    }

    Ok(None)
}

struct ForwardCheckAuditEvent<'a> {
    username: &'a str,
    server_id_hash: &'a str,
    upstream: &'a yggdrasil_session_forward_server::Model,
    result: &'a str,
    profile_uuid: Option<&'a str>,
    error: Option<&'a str>,
}

async fn log_forward_check<S>(
    state: &S,
    request_info: &audit_service::AuditRequestInfo,
    event: ForwardCheckAuditEvent<'_>,
) where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let ctx = request_info.to_context(0);
    let upstream = event.upstream;
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::YggdrasilSessionForwardCheck,
        audit_service::AuditEntityType::YggdrasilSession,
        Some(upstream.id),
        Some(&upstream.display_name),
        || {
            audit_service::details(audit_service::YggdrasilSessionForwardCheckAuditDetails {
                username: event.username,
                server_id_hash: event.server_id_hash,
                upstream_id: upstream.id,
                upstream_name: &upstream.display_name,
                provider_kind: upstream.provider_kind.as_str(),
                endpoint_kind: upstream.endpoint_kind.as_str(),
                result: event.result,
                texture_forward_enabled: upstream.texture_forward_enabled,
                profile_uuid: event.profile_uuid,
                error: event.error,
            })
        },
    )
    .await;
}

async fn rewrite_forwarded_texture_urls<S>(
    state: &S,
    upstream: &yggdrasil_session_forward_server::Model,
    profile: &mut YggdrasilProfile,
) -> std::result::Result<usize, String>
where
    S: AppConfigRuntimeState + RuntimeConfigRuntimeState + CacheRuntimeState,
{
    let Some(properties) = profile.properties.as_mut() else {
        tracing::debug!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            profile_uuid = %profile.id,
            "forwarded yggdrasil texture rewrite skipped because profile has no properties"
        );
        return Ok(0);
    };
    let Some(textures_property) = properties
        .iter_mut()
        .find(|property| property.name == "textures")
    else {
        tracing::debug!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            profile_uuid = %profile.id,
            property_count = properties.len(),
            "forwarded yggdrasil texture rewrite skipped because profile has no textures property"
        );
        return Ok(0);
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&textures_property.value)
        .map_err(|error| format!("textures property is not valid base64: {error}"))?;
    let mut payload: Value = serde_json::from_slice(&decoded)
        .map_err(|error| format!("textures property is not valid JSON: {error}"))?;
    let Some(textures) = payload.get_mut("textures").and_then(Value::as_object_mut) else {
        tracing::debug!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            profile_uuid = %profile.id,
            "forwarded yggdrasil texture rewrite skipped because textures payload has no textures object"
        );
        return Ok(0);
    };

    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let Some(public_base_url) = policy.public_base_urls.first() else {
        return Err("public base URL is required for forwarded texture proxy URLs".to_string());
    };

    let mut rewritten_count = 0;
    for (texture_type, texture) in textures.iter_mut() {
        let Some(url_value) = texture.get_mut("url") else {
            tracing::debug!(
                upstream_id = upstream.id,
                upstream_name = %upstream.display_name,
                profile_uuid = %profile.id,
                texture_type,
                "forwarded yggdrasil texture rewrite skipped texture without url"
            );
            continue;
        };
        let Some(original_url) = url_value.as_str() else {
            tracing::debug!(
                upstream_id = upstream.id,
                upstream_name = %upstream.display_name,
                profile_uuid = %profile.id,
                texture_type,
                "forwarded yggdrasil texture rewrite skipped texture with non-string url"
            );
            continue;
        };
        let original_url = original_url.to_string();
        let texture_hash = forwarded_texture_hash(&original_url);
        let ticket = encode_forwarded_texture_ticket(
            state,
            ForwardedTextureTicket {
                s: upstream.id,
                h: texture_hash.clone(),
                u: original_url.clone(),
            },
        )
        .map_err(|error| format!("failed to sign forwarded texture ticket: {error}"))?;
        let forwarded_url = format!(
            "{}/sessionserver/session/minecraft/forwardedTextures/{}/{}/{}",
            public_base_url.trim_end_matches('/'),
            upstream.id,
            texture_hash,
            ticket
        );
        let forwarded_host = reqwest::Url::parse(&forwarded_url)
            .ok()
            .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()));
        let forwarded_host_allowed = forwarded_host.as_ref().is_some_and(|host| {
            policy
                .skin_domains
                .iter()
                .any(|domain| skin_domain_matches_host(domain, host))
        });
        tracing::warn!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            profile_uuid = %profile.id,
            texture_type,
            texture_hash = %texture_hash,
            forwarded_host = forwarded_host.as_deref().unwrap_or("invalid"),
            forwarded_host_allowed,
            forwarded_texture_path = %reqwest::Url::parse(&forwarded_url)
                .ok()
                .map(|url| url.path().to_string())
                .unwrap_or_else(|| "/sessionserver/session/minecraft/forwardedTextures/<invalid>".to_string()),
            "rewrote forwarded yggdrasil texture URL"
        );
        *url_value = Value::String(forwarded_url);
        rewritten_count += 1;
    }

    if rewritten_count == 0 {
        tracing::debug!(
            upstream_id = upstream.id,
            upstream_name = %upstream.display_name,
            profile_uuid = %profile.id,
            texture_count = textures.len(),
            "forwarded yggdrasil texture rewrite found no texture URLs"
        );
        return Ok(0);
    }

    let encoded = serde_json::to_vec(&payload)
        .map(|payload| base64::engine::general_purpose::STANDARD.encode(payload))
        .map_err(|error| format!("failed to serialize rewritten textures property: {error}"))?;
    let signature = yggdrasil_signature::sign_texture_property(&policy, &encoded)
        .map_err(|error| error.to_string())?;
    let Some(signature) = signature else {
        return Err(
            "forwarded texture URL rewrite requires a configured Yggdrasil signature private key"
                .to_string(),
        );
    };
    tracing::warn!(
        upstream_id = upstream.id,
        upstream_name = %upstream.display_name,
        profile_uuid = %profile.id,
        value_hash = %sha256_hex(encoded.as_bytes()),
        signature_len = signature.len(),
        "re-signed forwarded yggdrasil textures property"
    );
    *textures_property = YggdrasilProfileProperty {
        name: textures_property.name.clone(),
        value: encoded,
        signature: Some(signature),
    };
    Ok(rewritten_count)
}

fn skin_domain_matches_host(domain: &str, host: &str) -> bool {
    domain
        .strip_prefix('.')
        .is_some_and(|suffix| host.ends_with(suffix))
        || domain == host
}

async fn query_forward_server(
    client: &reqwest::Client,
    upstream: &yggdrasil_session_forward_server::Model,
    username: &str,
    server_id: &str,
    ip: Option<&str>,
) -> std::result::Result<Option<YggdrasilProfile>, String> {
    let base_url = upstream
        .base_url
        .as_deref()
        .ok_or_else(|| "remote upstream base URL is missing".to_string())?;
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|error| format!("invalid upstream base URL: {error}"))?;
    {
        let mut path = url.path().trim_end_matches('/').to_string();
        path.push_str(session_forward_has_joined_path(upstream.endpoint_kind));
        url.set_path(&path);
        let mut query = url.query_pairs_mut();
        query.append_pair("username", username);
        query.append_pair("serverId", server_id);
        if let Some(ip) = ip {
            query.append_pair("ip", ip);
        }
    }
    tracing::debug!(
        upstream_id = upstream.id,
        upstream_name = %upstream.display_name,
        endpoint_kind = %upstream.endpoint_kind.as_str(),
        upstream_origin = %url.origin().ascii_serialization(),
        upstream_has_joined_path = %url.path(),
        username,
        server_id_hash = %sha256_hex(server_id.as_bytes()),
        has_ip = ip.is_some(),
        "querying yggdrasil hasJoined upstream"
    );

    let timeout = Duration::from_millis(normalize_forward_timeout_ms(upstream.timeout_ms));
    let response = client
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    if status == reqwest::StatusCode::NO_CONTENT || status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("upstream returned HTTP {status}"));
    }

    let profile = response
        .json::<YggdrasilProfile>()
        .await
        .map_err(|error| format!("invalid profile JSON: {error}"))?;
    profile
        .validate()
        .map_err(|error| format!("invalid profile body: {error}"))?;
    Ok(Some(profile))
}

fn session_forward_has_joined_path(kind: YggdrasilSessionForwardEndpointKind) -> &'static str {
    match kind {
        YggdrasilSessionForwardEndpointKind::AuthlibInjector => {
            "/sessionserver/session/minecraft/hasJoined"
        }
        YggdrasilSessionForwardEndpointKind::MojangSession => "/session/minecraft/hasJoined",
    }
}

pub async fn forwarded_texture_url<S>(
    state: &S,
    upstream_id: i64,
    texture_hash: &str,
    ticket: &str,
) -> std::result::Result<Option<String>, YggdrasilError>
where
    S: AppConfigRuntimeState,
{
    if !is_forwarded_texture_hash(texture_hash) {
        tracing::debug!("forwarded texture lookup rejected invalid texture hash");
        return Ok(None);
    }
    let ticket = match decode_forwarded_texture_ticket(state, ticket) {
        Ok(ticket) => ticket,
        Err(error) => {
            tracing::warn!(
                upstream_id,
                texture_hash,
                error = %error,
                "forwarded yggdrasil texture ticket was invalid"
            );
            return Ok(None);
        }
    };
    if ticket.s != upstream_id || ticket.h != texture_hash {
        tracing::warn!(
            upstream_id,
            texture_hash,
            ticket_upstream_id = ticket.s,
            ticket_texture_hash = %ticket.h,
            "forwarded yggdrasil texture ticket did not match request path"
        );
        return Ok(None);
    }
    let parsed_url = match reqwest::Url::parse(&ticket.u) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url,
        Ok(url) => {
            tracing::warn!(
                upstream_id,
                texture_hash,
                scheme = %url.scheme(),
                "forwarded yggdrasil texture ticket rejected unsupported URL scheme"
            );
            return Ok(None);
        }
        Err(error) => {
            tracing::warn!(
                upstream_id,
                texture_hash,
                error = %error,
                "forwarded yggdrasil texture ticket contained invalid URL"
            );
            return Ok(None);
        }
    };
    Ok(Some(parsed_url.to_string()))
}

fn encode_forwarded_texture_ticket<S>(
    state: &S,
    ticket: ForwardedTextureTicket,
) -> jsonwebtoken::errors::Result<String>
where
    S: AppConfigRuntimeState,
{
    encode(
        &Header::new(Algorithm::HS256),
        &ticket,
        &EncodingKey::from_secret(state.config().auth.jwt_secret.as_bytes()),
    )
}

fn decode_forwarded_texture_ticket<S>(
    state: &S,
    ticket: &str,
) -> jsonwebtoken::errors::Result<ForwardedTextureTicket>
where
    S: AppConfigRuntimeState,
{
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    validation.required_spec_claims.clear();
    decode::<ForwardedTextureTicket>(
        ticket,
        &DecodingKey::from_secret(state.config().auth.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
}

fn forwarded_texture_hash(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|segments| {
                    segments
                        .rev()
                        .find(|segment| is_forwarded_texture_hash(segment))
                })
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| sha256_hex(url.as_bytes()))
}

fn is_forwarded_texture_hash(value: &str) -> bool {
    matches!(value.len(), 32 | 40 | 64) && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn normalize_forward_timeout_ms(timeout_ms: i32) -> u64 {
    let Ok(timeout_ms) = u64::try_from(timeout_ms) else {
        return DEFAULT_FORWARD_TIMEOUT_MS;
    };
    timeout_ms.clamp(MIN_FORWARD_TIMEOUT_MS, MAX_FORWARD_TIMEOUT_MS)
}

fn weighted_forward_order(
    servers: Vec<yggdrasil_session_forward_server::Model>,
) -> Vec<yggdrasil_session_forward_server::Model> {
    let mut ordered = Vec::with_capacity(servers.len());
    let mut index = 0;
    let mut servers = servers;
    servers.sort_by_key(|server| (server.priority, server.id));

    while index < servers.len() {
        let priority = servers[index].priority;
        let mut end = index + 1;
        while end < servers.len() && servers[end].priority == priority {
            end += 1;
        }
        ordered.extend(weighted_group_order(servers[index..end].to_vec()));
        index = end;
    }

    ordered
}

fn weighted_group_order(
    mut servers: Vec<yggdrasil_session_forward_server::Model>,
) -> Vec<yggdrasil_session_forward_server::Model> {
    let mut ordered = Vec::with_capacity(servers.len());
    let mut rng = rand::rng();

    while !servers.is_empty() {
        let total_weight: i64 = servers
            .iter()
            .map(|server| i64::from(server.weight.max(1)))
            .sum();
        let mut ticket = rng.random_range(0..total_weight);
        let mut selected_index = 0;
        for (index, server) in servers.iter().enumerate() {
            ticket -= i64::from(server.weight.max(1));
            if ticket < 0 {
                selected_index = index;
                break;
            }
        }
        ordered.push(servers.remove(selected_index));
    }

    ordered
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ForwardedTextureTicket {
    s: i64,
    h: String,
    u: String,
}
