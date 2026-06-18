use actix_web::HttpRequest;

use crate::api::dto::yggdrasil::{
    YggdrasilAuthenticateReq, YggdrasilAuthenticateResp, YggdrasilRefreshReq, YggdrasilRefreshResp,
    YggdrasilTokenReq,
};
use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_repo, user_repo};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::audit_service;
use crate::utils::hash::verify_password;

use super::error::{YggdrasilError, YggdrasilErrorKind};
use super::login::{resolve_login_target, user_info};
use super::profile_summary;
use super::token::{
    active_token, issue_token, refresh_token, refreshable_token, revoke_all_for_user,
    revoke_by_access_token, selected_profile_for_token,
};

pub async fn authenticate<S>(
    state: &S,
    body: YggdrasilAuthenticateReq,
    req: &HttpRequest,
) -> std::result::Result<YggdrasilAuthenticateResp, YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        identifier_len = body.username.len(),
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        request_user = body.request_user,
        has_agent = body.agent.is_some(),
        "starting yggdrasil authenticate"
    );
    if body.username.trim().is_empty() || body.password.is_empty() {
        tracing::debug!("yggdrasil authenticate rejected because credentials were empty");
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }
    if let Some(agent) = body.agent.as_ref()
        && (agent.name != "Minecraft" || agent.version != 1)
    {
        tracing::debug!(
            agent_name = %agent.name,
            agent_version = agent.version,
            "yggdrasil authenticate rejected unsupported agent"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::UnsupportedAgent));
    }

    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let login_target = resolve_login_target(state, &body.username, &policy).await?;
    if !login_target.user.status.is_active() {
        tracing::debug!(
            user_id = login_target.user.id,
            status = ?login_target.user.status,
            "yggdrasil authenticate rejected inactive user"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }
    if login_target.user.email_verified_at.is_none() {
        tracing::debug!(
            user_id = login_target.user.id,
            "yggdrasil authenticate rejected pending email activation"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }
    if !verify_password(&body.password, &login_target.user.password_hash)
        .map_err(YggdrasilError::from)?
    {
        tracing::debug!(
            user_id = login_target.user.id,
            "yggdrasil authenticate rejected password mismatch"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }

    let profiles = minecraft_profile_repo::list_by_user(state.reader_db(), login_target.user.id)
        .await
        .map_err(YggdrasilError::from)?;
    let selected_profile_id = login_target
        .forced_profile
        .as_ref()
        .map(|profile| profile.id)
        .or_else(|| {
            if profiles.len() == 1 {
                profiles.first().map(|profile| profile.id)
            } else {
                None
            }
        });
    tracing::debug!(
        user_id = login_target.user.id,
        available_profile_count = profiles.len(),
        selected_profile_id,
        forced_profile = login_target.forced_profile.is_some(),
        "yggdrasil authenticate profile selection resolved"
    );

    let client_token = body
        .client_token
        .filter(|token| !token.trim().is_empty())
        .unwrap_or_else(crate::utils::id::new_unsigned_uuid);
    let issued = issue_token(
        state,
        login_target.user.id,
        &client_token,
        selected_profile_id,
        &policy,
        req,
    )
    .await?;
    tracing::debug!(
        user_id = login_target.user.id,
        token_id = issued.token_id,
        selected_profile_id,
        "yggdrasil authenticate issued token"
    );

    let selected_profile = selected_profile_id
        .and_then(|id| profiles.iter().find(|profile| profile.id == id))
        .cloned();
    let ctx = audit_service::AuditContext::from_request(req, login_target.user.id);
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::YggdrasilAuthenticate,
        audit_service::AuditEntityType::YggdrasilToken,
        Some(issued.token_id),
        selected_profile
            .as_ref()
            .map(|profile| profile.name.as_str()),
        || {
            audit_service::details(audit_service::YggdrasilAuthenticateAuditDetails {
                identifier: &body.username,
                selected_profile_uuid: selected_profile
                    .as_ref()
                    .map(|profile| profile.uuid.as_str()),
                selected_profile_name: selected_profile
                    .as_ref()
                    .map(|profile| profile.name.as_str()),
                available_profile_count: profiles.len(),
            })
        },
    )
    .await;

    Ok(YggdrasilAuthenticateResp {
        access_token: issued.access_token,
        client_token,
        available_profiles: profiles.iter().map(profile_summary).collect(),
        selected_profile: selected_profile.as_ref().map(profile_summary),
        user: body.request_user.then(|| user_info(&login_target.user)),
    })
}

pub async fn refresh<S>(
    state: &S,
    body: YggdrasilRefreshReq,
    req: &HttpRequest,
) -> std::result::Result<YggdrasilRefreshResp, YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        has_selected_profile = body.selected_profile.is_some(),
        request_user = body.request_user,
        "starting yggdrasil token refresh"
    );
    let existing =
        refreshable_token(state, &body.access_token, body.client_token.as_deref()).await?;
    let user = user_repo::find_by_id(state.reader_db(), existing.user_id)
        .await
        .map_err(YggdrasilError::from)?;
    if !user.status.is_active() {
        tracing::debug!(
            user_id = user.id,
            token_id = existing.id,
            status = ?user.status,
            "yggdrasil refresh rejected inactive user"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }

    let selected_profile_id = if let Some(selected) = body.selected_profile.as_ref() {
        if existing.selected_profile_id.is_some() {
            tracing::debug!(
                user_id = user.id,
                token_id = existing.id,
                existing_selected_profile_id = existing.selected_profile_id,
                "yggdrasil refresh rejected profile reselection"
            );
            return Err(YggdrasilError::new(
                YggdrasilErrorKind::AccessTokenAlreadyHasProfile,
            ));
        }
        let Some(profile) = minecraft_profile_repo::find_by_uuid(state.reader_db(), &selected.id)
            .await
            .map_err(YggdrasilError::from)?
        else {
            tracing::debug!(
                user_id = user.id,
                token_id = existing.id,
                requested_profile_uuid = %selected.id,
                "yggdrasil refresh rejected missing selected profile"
            );
            return Err(YggdrasilError::new(YggdrasilErrorKind::ForbiddenProfile));
        };
        if profile.user_id != user.id {
            tracing::debug!(
                user_id = user.id,
                token_id = existing.id,
                requested_profile_id = profile.id,
                requested_profile_owner_id = profile.user_id,
                "yggdrasil refresh rejected profile owned by another user"
            );
            return Err(YggdrasilError::new(YggdrasilErrorKind::ForbiddenProfile));
        }
        Some(profile.id)
    } else {
        existing.selected_profile_id
    };

    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let issued = refresh_token(
        state,
        &body.access_token,
        user.id,
        &existing.client_token,
        selected_profile_id,
        &policy,
        req,
    )
    .await?;
    tracing::debug!(
        user_id = user.id,
        old_token_id = existing.id,
        new_token_id = issued.token_id,
        selected_profile_id,
        "yggdrasil token refresh issued replacement"
    );

    let selected_profile = match selected_profile_id {
        Some(id) => Some(
            minecraft_profile_repo::find_by_id(state.reader_db(), id)
                .await
                .map_err(YggdrasilError::from)?,
        ),
        None => None,
    };
    let ctx = audit_service::AuditContext::from_request(req, user.id);
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::YggdrasilRefreshToken,
        audit_service::AuditEntityType::YggdrasilToken,
        Some(issued.token_id),
        selected_profile
            .as_ref()
            .map(|profile| profile.name.as_str()),
        || {
            audit_service::details(audit_service::YggdrasilTokenAuditDetails {
                profile_uuid: selected_profile
                    .as_ref()
                    .map(|profile| profile.uuid.as_str()),
                profile_name: selected_profile
                    .as_ref()
                    .map(|profile| profile.name.as_str()),
            })
        },
    )
    .await;

    Ok(YggdrasilRefreshResp {
        access_token: issued.access_token,
        client_token: existing.client_token,
        selected_profile: selected_profile.as_ref().map(profile_summary),
        user: body.request_user.then(|| user_info(&user)),
    })
}

pub async fn validate<S: DatabaseRuntimeState>(
    state: &S,
    body: YggdrasilTokenReq,
) -> std::result::Result<(), YggdrasilError> {
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        "validating yggdrasil token"
    );
    let token = active_token(state, &body.access_token, body.client_token.as_deref()).await?;
    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        "yggdrasil token validated"
    );
    Ok(())
}

pub async fn invalidate<S>(
    state: &S,
    body: YggdrasilTokenReq,
) -> std::result::Result<(), YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        "invalidating yggdrasil token"
    );
    let token = active_token(state, &body.access_token, None).await.ok();
    let profile = selected_profile_for_token(state, token.as_ref()).await?;
    revoke_by_access_token(state, &body.access_token).await?;
    tracing::debug!(
        token_id = token.as_ref().map(|token| token.id),
        user_id = token.as_ref().map(|token| token.user_id),
        selected_profile_id = token.as_ref().and_then(|token| token.selected_profile_id),
        "yggdrasil token invalidation completed"
    );
    if let Some(token) = token {
        let ctx = audit_service::AuditContext {
            user_id: token.user_id,
            ip_address: None,
            user_agent: None,
        };
        audit_service::log_with_details(
            state,
            &ctx,
            audit_service::AuditAction::YggdrasilInvalidateToken,
            audit_service::AuditEntityType::YggdrasilToken,
            Some(token.id),
            profile.as_ref().map(|profile| profile.name.as_str()),
            || {
                audit_service::details(audit_service::YggdrasilTokenAuditDetails {
                    profile_uuid: profile.as_ref().map(|profile| profile.uuid.as_str()),
                    profile_name: profile.as_ref().map(|profile| profile.name.as_str()),
                })
            },
        )
        .await;
    }
    Ok(())
}

pub async fn signout<S>(
    state: &S,
    username: &str,
    password: &str,
) -> std::result::Result<(), YggdrasilError>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        identifier_len = username.len(),
        "starting yggdrasil signout"
    );
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let login_target = resolve_login_target(state, username, &policy).await?;
    if !login_target.user.status.is_active() {
        tracing::debug!(
            user_id = login_target.user.id,
            status = ?login_target.user.status,
            "yggdrasil signout rejected inactive user"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }
    if !verify_password(password, &login_target.user.password_hash).map_err(YggdrasilError::from)? {
        tracing::debug!(
            user_id = login_target.user.id,
            "yggdrasil signout rejected password mismatch"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }

    revoke_all_for_user(state, login_target.user.id).await?;
    tracing::debug!(
        user_id = login_target.user.id,
        "yggdrasil signout revoked user tokens"
    );
    let ctx = audit_service::AuditContext {
        user_id: login_target.user.id,
        ip_address: None,
        user_agent: None,
    };
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::YggdrasilSignout,
        audit_service::AuditEntityType::User,
        Some(login_target.user.id),
        Some(&login_target.user.username),
        || {
            audit_service::details(audit_service::LoginAuditDetails {
                identifier: username,
            })
        },
    )
    .await;
    Ok(())
}
