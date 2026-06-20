use chrono::{SecondsFormat, Utc};
use rsa::RsaPrivateKey;
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};

use crate::api::dto::yggdrasil::{
    MinecraftServicesBanStatus, MinecraftServicesCertificateResp, MinecraftServicesChatPreferences,
    MinecraftServicesFriendsPreferences, MinecraftServicesKeyPair,
    MinecraftServicesPlayerAttributesResp, MinecraftServicesPreferenceState,
    MinecraftServicesPrivacyBlocklistResp, MinecraftServicesPrivilege, MinecraftServicesPrivileges,
    MinecraftServicesPrivilegesResp, MinecraftServicesProfanityFilterPreferences,
};
use crate::errors::AsterError;
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState};
use crate::services::ban_service;
use crate::types::UserBanScope;

use super::{YggdrasilError, YggdrasilErrorKind, active_token_for_protocol};

const PROFILE_KEY_BITS: usize = 2048;
const DUMMY_PUBLIC_KEY_SIGNATURE: &str = "AA==";

pub async fn profile_key_certificate<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<MinecraftServicesCertificateResp, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let token = active_token_for_protocol(state, access_token).await?;
    if token.selected_profile_id.is_none() {
        tracing::debug!(
            token_id = token.id,
            user_id = token.user_id,
            "minecraft services certificate rejected token without selected profile"
        );
        return Err(YggdrasilError::new(YggdrasilErrorKind::InvalidToken));
    }

    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        selected_profile_id = token.selected_profile_id,
        "issuing minecraft services profile key certificate"
    );

    let key_pair = generate_profile_key_pair().map_err(YggdrasilError::from)?;
    let now = Utc::now();
    Ok(MinecraftServicesCertificateResp {
        key_pair,
        public_key_signature: DUMMY_PUBLIC_KEY_SIGNATURE.to_string(),
        public_key_signature_v2: DUMMY_PUBLIC_KEY_SIGNATURE.to_string(),
        expires_at: (now + chrono::Duration::hours(48)).to_rfc3339_opts(SecondsFormat::Secs, true),
        refreshed_after: (now + chrono::Duration::hours(36))
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    })
}

pub async fn minecraft_services_privileges<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<MinecraftServicesPrivilegesResp, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let token = validate_minecraft_services_token(state, access_token).await?;
    Ok(MinecraftServicesPrivilegesResp {
        privileges: privileges_for_user(state, token.user_id).await?,
    })
}

pub async fn minecraft_services_player_attributes<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<MinecraftServicesPlayerAttributesResp, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let token = validate_minecraft_services_token(state, access_token).await?;
    let join_banned =
        ban_service::is_user_banned(state, token.user_id, UserBanScope::YggdrasilJoin)
            .await
            .map_err(YggdrasilError::from)?;
    Ok(MinecraftServicesPlayerAttributesResp {
        privileges: privileges_for_user(state, token.user_id).await?,
        profanity_filter_preferences: MinecraftServicesProfanityFilterPreferences {
            profanity_filter_on: false,
        },
        friends_preferences: MinecraftServicesFriendsPreferences {
            friends: MinecraftServicesPreferenceState::Disabled,
            accept_invites: MinecraftServicesPreferenceState::Disabled,
        },
        chat_preferences: MinecraftServicesChatPreferences {
            text_communication: MinecraftServicesPreferenceState::Enabled,
        },
        ban_status: MinecraftServicesBanStatus {
            banned_scopes: crate::api::dto::yggdrasil::MinecraftServicesBannedScopes {
                multiplayer: join_banned,
            },
        },
    })
}

pub async fn minecraft_services_privacy_blocklist<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<MinecraftServicesPrivacyBlocklistResp, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    validate_minecraft_services_token(state, access_token).await?;
    Ok(MinecraftServicesPrivacyBlocklistResp {
        blocked_profiles: Vec::new(),
    })
}

async fn validate_minecraft_services_token<S>(
    state: &S,
    access_token: &str,
) -> std::result::Result<crate::entities::yggdrasil_token::Model, YggdrasilError>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let token = active_token_for_protocol(state, access_token).await?;
    tracing::debug!(
        token_id = token.id,
        user_id = token.user_id,
        selected_profile_id = token.selected_profile_id,
        "validated minecraft services policy bearer token"
    );
    Ok(token)
}

async fn privileges_for_user<S: DatabaseRuntimeState>(
    state: &S,
    user_id: i64,
) -> std::result::Result<MinecraftServicesPrivileges, YggdrasilError> {
    let multiplayer_enabled =
        !ban_service::is_user_banned(state, user_id, UserBanScope::YggdrasilJoin)
            .await
            .map_err(YggdrasilError::from)?;
    Ok(MinecraftServicesPrivileges {
        online_chat: MinecraftServicesPrivilege { enabled: true },
        multiplayer_server: MinecraftServicesPrivilege {
            enabled: multiplayer_enabled,
        },
        multiplayer_realms: MinecraftServicesPrivilege {
            enabled: multiplayer_enabled,
        },
        telemetry: MinecraftServicesPrivilege { enabled: true },
        optional_telemetry: MinecraftServicesPrivilege { enabled: true },
    })
}

fn generate_profile_key_pair() -> crate::errors::Result<MinecraftServicesKeyPair> {
    let mut rng = rand::rng();
    let private_key = RsaPrivateKey::new(&mut rng, PROFILE_KEY_BITS).map_err(|error| {
        AsterError::internal_error(format!(
            "failed to generate minecraft services profile key: {error}"
        ))
    })?;
    let public_key = private_key.to_public_key();

    let private_key = private_key
        .to_pkcs1_pem(LineEnding::LF)
        .map(|pem| pem.to_string())
        .map_err(|error| {
            AsterError::internal_error(format!(
                "failed to encode minecraft services private key: {error}"
            ))
        })?;
    let public_key = public_key.to_pkcs1_pem(LineEnding::LF).map_err(|error| {
        AsterError::internal_error(format!(
            "failed to encode minecraft services public key: {error}"
        ))
    })?;

    Ok(MinecraftServicesKeyPair {
        private_key,
        public_key,
    })
}
