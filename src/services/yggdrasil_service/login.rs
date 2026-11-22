use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_repo, user_repo};
use crate::runtime::DatabaseRuntimeState;
use aster_yggdrasil_model::entities::{minecraft_profile, user};
use aster_yggdrasil_protocol::YggdrasilUser;

use super::error::{YggdrasilError, YggdrasilErrorKind};

pub(super) struct LoginTarget {
    pub(super) user: user::Model,
    pub(super) forced_profile: Option<minecraft_profile::Model>,
}

pub(super) async fn resolve_login_target<S: DatabaseRuntimeState>(
    state: &S,
    identifier: &str,
    policy: &RuntimeYggdrasilPolicy,
) -> std::result::Result<LoginTarget, YggdrasilError> {
    tracing::debug!(
        identifier_len = identifier.len(),
        identifier_has_at = identifier.contains('@'),
        allow_profile_name_login = policy.allow_profile_name_login,
        "resolving yggdrasil login target"
    );
    if let Some(user) = user_repo::find_by_identifier(state.reader_db(), identifier)
        .await
        .map_err(YggdrasilError::from)?
    {
        tracing::debug!(
            user_id = user.id,
            "yggdrasil login target resolved by local user identifier"
        );
        return Ok(LoginTarget {
            user,
            forced_profile: None,
        });
    }

    if !policy.allow_profile_name_login {
        tracing::debug!(
            identifier_len = identifier.len(),
            "yggdrasil login target rejected because profile-name login is disabled"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    }

    let Some(profile) = minecraft_profile_repo::find_by_name(state.reader_db(), identifier)
        .await
        .map_err(YggdrasilError::from)?
    else {
        tracing::debug!(
            identifier_len = identifier.len(),
            "yggdrasil login target rejected because profile name was not found"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidCredentials));
    };
    let user = user_repo::find_by_id(state.reader_db(), profile.user_id)
        .await
        .map_err(YggdrasilError::from)?;
    tracing::debug!(
        user_id = user.id,
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        "yggdrasil login target resolved by profile name"
    );
    Ok(LoginTarget {
        user,
        forced_profile: Some(profile),
    })
}

pub(super) fn user_info(user: &user::Model) -> YggdrasilUser {
    YggdrasilUser {
        // authlib-injector serializes User ID as an unhyphenated UUID. Keep
        // the internal auto-increment database id out of the protocol surface.
        id: user.public_uuid.clone(),
        properties: Vec::new(),
    }
}
