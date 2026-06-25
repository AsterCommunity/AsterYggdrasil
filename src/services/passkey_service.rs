//! Passkey / WebAuthn service logic.

use base64::Engine as _;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, SqlErr};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::{
    CreationChallengeResponse, CredentialID, DiscoverableAuthentication, DiscoverableKey, Passkey,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Uuid, Webauthn, WebauthnBuilder, WebauthnError,
};
use webauthn_rs_proto::{ResidentKeyRequirement, UserVerificationPolicy};

use crate::api::error_code::AsterErrorCode;
use crate::config::{auth_runtime::RuntimeAuthPolicy, branding, site_url};
use crate::db::repository::{passkey_repo, user_repo};
use crate::entities::{passkey, user};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::SharedRuntimeState;
use crate::services::auth_service::{self, is_email_verified};
use crate::types::StoredPasskeyCredential;
use actix_web::HttpRequest;
use aster_forge_utils::net::is_loopback_host;
use aster_forge_utils::numbers::{u32_to_i64, u64_to_i64};
use aster_forge_utils::text::char_count;

const PASSKEY_CHALLENGE_TTL_SECS: u64 = 300;
const PASSKEY_NAME_MAX_LEN: usize = 128;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct PasskeyInfo {
    pub id: i64,
    pub name: String,
    pub transports: Option<Vec<String>>,
    pub backup_eligible: bool,
    pub backed_up: bool,
    pub sign_count: i64,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_used_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct PasskeyRegisterStartResp {
    pub flow_id: String,
    pub public_key: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct PasskeyLoginStartResp {
    pub flow_id: String,
    pub public_key: serde_json::Value,
}

#[derive(Debug)]
pub struct PasskeyLoginResult {
    pub session: auth_service::AuthTokenBundle,
    pub passkey_id: i64,
    pub passkey_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PasskeyRegistrationChallenge {
    user_id: i64,
    user_handle: Uuid,
    default_name: String,
    state: PasskeyRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PasskeyAuthenticationChallenge {
    identifier: Option<String>,
    state: DiscoverableAuthentication,
}

fn registration_cache_key(flow_id: &str) -> String {
    format!("external_auth:passkey:registration:{flow_id}")
}

fn login_cache_key(flow_id: &str) -> String {
    format!("external_auth:passkey:login:{flow_id}")
}

fn new_flow_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn credential_id_to_storage(credential_id: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(credential_id)
}

fn user_handle_to_storage(user_handle: Uuid) -> String {
    user_handle.to_string()
}

fn user_handle_from_storage(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_aster_err_ctx(
        "invalid stored passkey user handle",
        AsterError::database_operation,
    )
}

async fn user_handle_for_registration(
    db: &DatabaseConnection,
    existing: &[passkey::Model],
) -> Result<Uuid> {
    match existing.first() {
        Some(passkey) => user_handle_from_storage(&passkey.user_handle),
        None => {
            aster_forge_utils::id::new_best_effort_uuid("passkey user handle", |candidate| {
                let storage = user_handle_to_storage(candidate);
                async move { passkey_repo::user_handle_exists(db, &storage).await }
            })
            .await
        }
    }
}

fn normalize_passkey_name(name: Option<&str>) -> Result<String> {
    let trimmed = name.map(str::trim).filter(|value| !value.is_empty());
    let normalized = trimmed.unwrap_or("Passkey");
    if char_count(normalized) > PASSKEY_NAME_MAX_LEN {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::PasskeyNameTooLong,
            format!("passkey name exceeds {PASSKEY_NAME_MAX_LEN} characters"),
        ));
    }
    if normalized.chars().any(char::is_control) {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::PasskeyNameInvalid,
            "passkey name cannot contain control characters",
        ));
    }
    Ok(normalized.to_string())
}

fn passkey_to_json(passkey: &Passkey) -> Result<serde_json::Value> {
    serde_json::to_value(passkey)
        .map_aster_err_ctx("failed to serialize passkey", AsterError::internal_error)
}

fn stored_passkey_credential(value: &serde_json::Value) -> Result<StoredPasskeyCredential> {
    StoredPasskeyCredential::from_json(value)
        .map_aster_err_ctx("failed to serialize passkey", AsterError::internal_error)
}

fn passkey_from_json(value: &StoredPasskeyCredential) -> Result<Passkey> {
    let credential = value.parse().map_aster_err_ctx(
        "failed to parse stored passkey credential JSON",
        AsterError::database_operation,
    )?;
    serde_json::from_value(credential).map_aster_err_ctx(
        "failed to deserialize passkey",
        AsterError::database_operation,
    )
}

type PasskeyMetadata = (StoredPasskeyCredential, Option<String>, bool, bool, i64);

fn passkey_metadata(passkey: &Passkey) -> Result<PasskeyMetadata> {
    let credential = passkey_to_json(passkey)?;
    let transports = transports_from_passkey_json(&credential);
    let backup_eligible = credential
        .get("cred")
        .and_then(|cred| cred.get("backup_eligible"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let backed_up = credential
        .get("cred")
        .and_then(|cred| cred.get("backup_state"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let sign_count = credential
        .get("cred")
        .and_then(|cred| cred.get("counter"))
        .and_then(serde_json::Value::as_u64)
        .map(|value| u64_to_i64(value, "passkey sign count"))
        .transpose()?
        .unwrap_or(0);

    Ok((
        stored_passkey_credential(&credential)?,
        transports,
        backup_eligible,
        backed_up,
        sign_count,
    ))
}

fn transports_from_passkey_json(value: &serde_json::Value) -> Option<String> {
    value
        .get("cred")
        .and_then(|cred| cred.get("transports"))
        .and_then(|transports| {
            if transports.is_null() {
                None
            } else {
                serde_json::to_string(transports).ok()
            }
        })
}

fn transports_to_vec(value: Option<String>) -> Option<Vec<String>> {
    let raw = value?;
    serde_json::from_str::<Vec<String>>(&raw).ok()
}

fn model_to_info(model: passkey::Model) -> PasskeyInfo {
    PasskeyInfo {
        id: model.id,
        name: model.name,
        transports: transports_to_vec(model.transports),
        backup_eligible: model.backup_eligible,
        backed_up: model.backed_up,
        sign_count: model.sign_count,
        created_at: model.created_at,
        updated_at: model.updated_at,
        last_used_at: model.last_used_at,
    }
}

fn webauthn_error(error: WebauthnError) -> AsterError {
    AsterError::auth_invalid_credentials(format!("passkey verification failed: {error}"))
}

fn webauthn_config_error(error: impl std::fmt::Display) -> AsterError {
    AsterError::config_error(format!(
        "invalid passkey relying party configuration: {error}"
    ))
}

fn rp_id_from_origin(origin: &url::Url) -> Result<String> {
    let host = origin
        .host_str()
        .ok_or_else(|| AsterError::config_error("public_site_url origin must include a host"))?;
    Ok(host.to_string())
}

fn primary_public_origin(state: &impl SharedRuntimeState) -> Result<String> {
    let origin = site_url::public_site_url(state.runtime_config()).ok_or_else(|| {
        AsterError::validation_error_code(
            AsterErrorCode::ConfigPublicSiteUrlRequired,
            "public_site_url must be configured before enabling passkey authentication",
        )
    })?;

    let parsed = url::Url::parse(&origin).map_err(|_| {
        AsterError::validation_error_code(
            AsterErrorCode::ConfigPublicSiteUrlInvalid,
            "passkey authentication requires a valid public_site_url",
        )
    })?;
    let is_local_http =
        parsed.scheme() == "http" && parsed.host_str().is_some_and(is_loopback_host);

    if parsed.scheme() == "https" || is_local_http {
        Ok(origin)
    } else {
        Err(AsterError::validation_error_code(
            AsterErrorCode::ConfigPublicSiteUrlInvalid,
            "passkey authentication requires HTTPS public_site_url, except localhost",
        ))
    }
}

fn build_webauthn(state: &impl SharedRuntimeState) -> Result<Webauthn> {
    let origins = site_url::public_site_urls(state.runtime_config());
    let primary_origin = primary_public_origin(state)?;
    let primary = url::Url::parse(&primary_origin).map_aster_err(webauthn_config_error)?;
    let rp_id = rp_id_from_origin(&primary)?;
    let rp_name = branding::title_or_default(state.runtime_config());
    let mut builder = WebauthnBuilder::new(&rp_id, &primary)
        .map_err(webauthn_config_error)?
        .rp_name(&rp_name);

    for origin in origins {
        if origin == primary_origin {
            continue;
        }
        let parsed = url::Url::parse(&origin).map_aster_err(webauthn_config_error)?;
        builder = builder.append_allowed_origin(&parsed);
    }

    builder.build().map_err(webauthn_config_error)
}

fn ensure_passkey_login_enabled(state: &impl SharedRuntimeState) -> Result<()> {
    if RuntimeAuthPolicy::from_runtime_config(state.runtime_config()).passkey_login_enabled {
        tracing::debug!("passkey login policy allows login");
        return Ok(());
    }

    tracing::debug!("passkey login rejected because policy disabled passkey login");
    Err(AsterError::auth_forbidden_code(
        AsterErrorCode::AuthPasskeyLoginDisabled,
        "passkey login is disabled by administrator policy",
    ))
}

async fn store_registration_challenge(
    state: &impl SharedRuntimeState,
    flow_id: &str,
    challenge: &PasskeyRegistrationChallenge,
) {
    tracing::debug!(
        flow_id,
        user_id = challenge.user_id,
        ttl_secs = PASSKEY_CHALLENGE_TTL_SECS,
        "storing passkey registration challenge"
    );
    crate::services::cache_facade::set(
        state,
        &registration_cache_key(flow_id),
        challenge,
        Some(PASSKEY_CHALLENGE_TTL_SECS),
    )
    .await;
}

async fn take_registration_challenge(
    state: &impl SharedRuntimeState,
    flow_id: &str,
) -> Result<PasskeyRegistrationChallenge> {
    let key = registration_cache_key(flow_id);
    tracing::debug!(flow_id, "taking passkey registration challenge");
    let challenge: PasskeyRegistrationChallenge = crate::services::cache_facade::take(state, &key)
        .await
        .ok_or_else(|| {
            tracing::debug!(flow_id, "passkey registration challenge expired or missing");
            AsterError::auth_token_invalid("passkey registration challenge expired")
        })?;
    tracing::debug!(
        flow_id,
        user_id = challenge.user_id,
        "took passkey registration challenge"
    );
    Ok(challenge)
}

async fn store_login_challenge(
    state: &impl SharedRuntimeState,
    flow_id: &str,
    challenge: &PasskeyAuthenticationChallenge,
) {
    tracing::debug!(
        flow_id,
        has_identifier = challenge.identifier.is_some(),
        ttl_secs = PASSKEY_CHALLENGE_TTL_SECS,
        "storing passkey login challenge"
    );
    crate::services::cache_facade::set(
        state,
        &login_cache_key(flow_id),
        challenge,
        Some(PASSKEY_CHALLENGE_TTL_SECS),
    )
    .await;
}

async fn take_login_challenge(
    state: &impl SharedRuntimeState,
    flow_id: &str,
) -> Result<PasskeyAuthenticationChallenge> {
    let key = login_cache_key(flow_id);
    tracing::debug!(flow_id, "taking passkey login challenge");
    let challenge: PasskeyAuthenticationChallenge =
        crate::services::cache_facade::take(state, &key)
            .await
            .ok_or_else(|| {
                tracing::debug!(flow_id, "passkey login challenge expired or missing");
                AsterError::auth_token_invalid("passkey login challenge expired")
            })?;
    tracing::debug!(
        flow_id,
        has_identifier = challenge.identifier.is_some(),
        "took passkey login challenge"
    );
    Ok(challenge)
}

fn serialize_registration_options(
    options: &CreationChallengeResponse,
) -> Result<serde_json::Value> {
    serde_json::to_value(options).map_aster_err_ctx(
        "failed to serialize passkey registration options",
        AsterError::internal_error,
    )
}

fn require_discoverable_registration(options: &mut CreationChallengeResponse) {
    let selection = options
        .public_key
        .authenticator_selection
        .get_or_insert_with(Default::default);
    selection.resident_key = Some(ResidentKeyRequirement::Required);
    selection.require_resident_key = true;
    selection.user_verification = UserVerificationPolicy::Required;
}

fn serialize_login_options(options: &RequestChallengeResponse) -> Result<serde_json::Value> {
    serde_json::to_value(options).map_aster_err_ctx(
        "failed to serialize passkey login options",
        AsterError::internal_error,
    )
}

fn parse_registration_credential(value: serde_json::Value) -> Result<RegisterPublicKeyCredential> {
    serde_json::from_value(value).map_aster_err_ctx(
        "invalid passkey registration credential",
        AsterError::validation_error,
    )
}

fn ensure_discoverable_registration_result(credential: &RegisterPublicKeyCredential) -> Result<()> {
    if credential
        .extensions
        .cred_props
        .as_ref()
        .and_then(|props| props.rk)
        == Some(false)
    {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::PasskeyNotDiscoverable,
            "passkey registration did not create a discoverable credential",
        ));
    }
    Ok(())
}

fn parse_login_credential(value: serde_json::Value) -> Result<PublicKeyCredential> {
    serde_json::from_value(value).map_aster_err_ctx(
        "invalid passkey login credential",
        AsterError::validation_error,
    )
}

fn ensure_login_user(user: &user::Model) -> Result<()> {
    if !user.status.is_active() {
        return Err(AsterError::auth_forbidden("account is disabled"));
    }
    if !is_email_verified(user) {
        return Err(AsterError::auth_pending_activation(
            "account pending activation",
        ));
    }
    Ok(())
}

fn normalize_login_identifier(identifier: Option<&str>) -> Option<String> {
    identifier
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ensure_login_identifier_matches(user: &user::Model, identifier: &str) -> Result<()> {
    let matches = if identifier.contains('@') {
        user.email == identifier
    } else {
        user.username == identifier
    };
    if matches {
        Ok(())
    } else {
        Err(AsterError::auth_invalid_credentials(
            "passkey does not match requested identifier",
        ))
    }
}

fn map_unique_passkey_err(err: DbErr) -> AsterError {
    if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return AsterError::validation_error("passkey credential already exists");
    }
    AsterError::from(err)
}

pub async fn list_passkeys(
    state: &impl SharedRuntimeState,
    user_id: i64,
) -> Result<Vec<PasskeyInfo>> {
    tracing::debug!(user_id, "listing passkeys");
    passkey_repo::list_for_user(state.writer_db(), user_id)
        .await
        .map(|items| {
            tracing::debug!(user_id, count = items.len(), "listed passkeys");
            items.into_iter().map(model_to_info).collect()
        })
}

pub async fn list_passkeys_cursor(
    state: &impl SharedRuntimeState,
    user_id: i64,
    limit: u64,
    cursor: Option<(chrono::DateTime<chrono::Utc>, i64)>,
) -> Result<aster_forge_api::CursorPage<PasskeyInfo, aster_forge_api::DateTimeIdCursor>> {
    let limit = limit.clamp(1, 100);
    tracing::debug!(user_id, limit, "listing passkeys page");
    let page =
        passkey_repo::list_for_user_cursor(state.writer_db(), user_id, limit, cursor).await?;
    let next_cursor = if page.has_more {
        page.items
            .last()
            .map(|passkey| aster_forge_api::DateTimeIdCursor {
                value: passkey.created_at,
                id: passkey.id,
            })
    } else {
        None
    };
    let items = page
        .items
        .into_iter()
        .map(model_to_info)
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id,
        returned = items.len(),
        total = page.total,
        limit,
        "listed passkeys page"
    );
    Ok(aster_forge_api::CursorPage::new(
        items,
        page.total,
        limit,
        next_cursor,
    ))
}

pub async fn start_registration(
    state: &impl SharedRuntimeState,
    user_id: i64,
    name: Option<&str>,
) -> Result<PasskeyRegisterStartResp> {
    tracing::debug!(
        user_id,
        has_name = name.is_some_and(|value| !value.trim().is_empty()),
        "starting passkey registration"
    );
    let webauthn = build_webauthn(state)?;
    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    if !user.status.is_active() {
        tracing::debug!(
            user_id,
            status = ?user.status,
            "passkey registration rejected inactive user"
        );
        return Err(AsterError::auth_forbidden("account is disabled"));
    }
    let existing = passkey_repo::list_for_user(state.writer_db(), user.id).await?;
    let user_handle = user_handle_for_registration(state.writer_db(), &existing).await?;
    let exclude_credentials = existing
        .iter()
        .map(|item| passkey_from_json(&item.credential).map(|passkey| passkey.cred_id().clone()))
        .collect::<Result<Vec<CredentialID>>>()?;
    let exclude_credentials = (!exclude_credentials.is_empty()).then_some(exclude_credentials);
    let has_exclude_credentials = exclude_credentials.is_some();
    let display_name = user
        .email
        .split('@')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(&user.username);

    let (mut options, registration) = webauthn
        .start_passkey_registration(
            user_handle,
            &user.username,
            display_name,
            exclude_credentials,
        )
        .map_err(webauthn_error)?;
    require_discoverable_registration(&mut options);
    let flow_id = new_flow_id();
    let challenge = PasskeyRegistrationChallenge {
        user_id: user.id,
        user_handle,
        default_name: normalize_passkey_name(name)?,
        state: registration,
    };
    store_registration_challenge(state, &flow_id, &challenge).await;

    tracing::debug!(
        user_id,
        flow_id,
        existing_passkey_count = existing.len(),
        has_exclude_credentials,
        "started passkey registration"
    );
    Ok(PasskeyRegisterStartResp {
        flow_id,
        public_key: serialize_registration_options(&options)?,
    })
}

pub async fn finish_registration(
    state: &impl SharedRuntimeState,
    user_id: i64,
    flow_id: &str,
    credential: serde_json::Value,
    name: Option<&str>,
) -> Result<PasskeyInfo> {
    tracing::debug!(
        user_id,
        flow_id,
        has_name = name.is_some_and(|value| !value.trim().is_empty()),
        "finishing passkey registration"
    );
    let webauthn = build_webauthn(state)?;
    let challenge = take_registration_challenge(state, flow_id).await?;
    if challenge.user_id != user_id {
        tracing::debug!(
            user_id,
            challenge_user_id = challenge.user_id,
            flow_id,
            "passkey registration rejected because challenge user mismatched"
        );
        return Err(AsterError::auth_forbidden(
            "passkey registration challenge does not belong to user",
        ));
    }
    let user = user_repo::find_by_id(state.writer_db(), user_id).await?;
    if !user.status.is_active() {
        tracing::debug!(
            user_id,
            status = ?user.status,
            "passkey registration rejected inactive user"
        );
        return Err(AsterError::auth_forbidden("account is disabled"));
    }

    let credential = parse_registration_credential(credential)?;
    ensure_discoverable_registration_result(&credential)?;
    let passkey = webauthn
        .finish_passkey_registration(&credential, &challenge.state)
        .map_err(webauthn_error)?;
    let credential_id = credential_id_to_storage(passkey.cred_id());
    if passkey_repo::find_by_credential_id(state.writer_db(), &credential_id)
        .await?
        .is_some()
    {
        tracing::debug!(
            user_id,
            flow_id,
            "passkey registration rejected duplicate credential"
        );
        return Err(AsterError::validation_error(
            "passkey credential already exists",
        ));
    }

    let final_name = match name {
        Some(name) => normalize_passkey_name(Some(name))?,
        None => challenge.default_name,
    };
    let (credential, transports, backup_eligible, backed_up, sign_count) =
        passkey_metadata(&passkey)?;
    let now = Utc::now();
    let model = passkey::ActiveModel {
        user_id: Set(user.id),
        credential_id: Set(credential_id),
        user_handle: Set(user_handle_to_storage(challenge.user_handle)),
        credential: Set(credential),
        name: Set(final_name),
        transports: Set(transports),
        backup_eligible: Set(backup_eligible),
        backed_up: Set(backed_up),
        sign_count: Set(sign_count),
        created_at: Set(now),
        updated_at: Set(now),
        last_used_at: Set(None),
        ..Default::default()
    };
    let created = model
        .insert(state.writer_db())
        .await
        .map_err(map_unique_passkey_err)?;
    tracing::debug!(
        user_id,
        passkey_id = created.id,
        flow_id,
        backup_eligible = created.backup_eligible,
        backed_up = created.backed_up,
        "finished passkey registration"
    );
    Ok(model_to_info(created))
}

pub async fn rename_passkey(
    state: &impl SharedRuntimeState,
    user_id: i64,
    id: i64,
    name: &str,
) -> Result<PasskeyInfo> {
    tracing::debug!(user_id, passkey_id = id, "renaming passkey");
    let name = normalize_passkey_name(Some(name))?;
    if !passkey_repo::update_name_for_user(state.writer_db(), id, user_id, &name).await? {
        tracing::debug!(
            user_id,
            passkey_id = id,
            "passkey rename rejected missing passkey"
        );
        return Err(AsterError::record_not_found(format!("passkey #{id}")));
    }
    let passkey = passkey_repo::find_by_id_for_user(state.writer_db(), id, user_id)
        .await?
        .ok_or_else(|| AsterError::record_not_found(format!("passkey #{id}")))?;
    tracing::debug!(user_id, passkey_id = id, "renamed passkey");
    Ok(model_to_info(passkey))
}

pub async fn delete_passkey(
    state: &impl SharedRuntimeState,
    user_id: i64,
    id: i64,
) -> Result<bool> {
    tracing::debug!(user_id, passkey_id = id, "deleting passkey");
    let deleted = passkey_repo::delete_for_user(state.writer_db(), id, user_id).await?;
    tracing::debug!(user_id, passkey_id = id, deleted, "deleted passkey");
    Ok(deleted)
}

pub async fn start_login(
    state: &impl SharedRuntimeState,
    identifier: Option<&str>,
    conditional: bool,
) -> Result<PasskeyLoginStartResp> {
    tracing::debug!(
        has_identifier = identifier.is_some_and(|value| !value.trim().is_empty()),
        conditional,
        "starting passkey login"
    );
    ensure_passkey_login_enabled(state)?;
    let webauthn = build_webauthn(state)?;
    let (mut options, auth_state) = webauthn
        .start_discoverable_authentication()
        .map_err(webauthn_error)?;
    let challenge = PasskeyAuthenticationChallenge {
        identifier: normalize_login_identifier(identifier),
        state: auth_state,
    };
    if !conditional {
        options.mediation = None;
    }

    let flow_id = new_flow_id();
    store_login_challenge(state, &flow_id, &challenge).await;
    tracing::debug!(
        flow_id,
        conditional,
        has_identifier = challenge.identifier.is_some(),
        "started passkey login"
    );
    Ok(PasskeyLoginStartResp {
        flow_id,
        public_key: serialize_login_options(&options)?,
    })
}

pub async fn finish_login(
    state: &impl SharedRuntimeState,
    flow_id: &str,
    credential: serde_json::Value,
    req: &HttpRequest,
) -> Result<PasskeyLoginResult> {
    tracing::debug!(flow_id, "finishing passkey login");
    ensure_passkey_login_enabled(state)?;
    let webauthn = build_webauthn(state)?;
    let challenge = take_login_challenge(state, flow_id).await?;
    let credential = parse_login_credential(credential)?;
    let credential_id = credential_id_to_storage(credential.get_credential_id());

    let (user_handle, discovered_credential_id) = webauthn
        .identify_discoverable_authentication(&credential)
        .map_err(webauthn_error)?;
    let discovered_credential_id = credential_id_to_storage(discovered_credential_id);
    if discovered_credential_id != credential_id {
        tracing::debug!(flow_id, "passkey login rejected credential id mismatch");
        return Err(AsterError::auth_invalid_credentials(
            "passkey credential id mismatch",
        ));
    }
    let passkey = passkey_repo::find_by_user_handle_and_credential_id(
        state.writer_db(),
        &user_handle_to_storage(user_handle),
        &credential_id,
    )
    .await?
    .ok_or_else(|| AsterError::auth_invalid_credentials("passkey not found"))?;
    tracing::debug!(
        flow_id,
        passkey_id = passkey.id,
        user_id = passkey.user_id,
        "passkey login credential matched stored passkey"
    );

    let user = user_repo::find_by_id(state.writer_db(), passkey.user_id).await?;
    if let Some(identifier) = challenge.identifier.as_deref() {
        ensure_login_identifier_matches(&user, identifier)?;
    }
    ensure_login_user(&user)?;
    let mut stored = passkey_from_json(&passkey.credential)?;
    let discoverable = DiscoverableKey::from(&stored);
    let result = webauthn
        .finish_discoverable_authentication(&credential, challenge.state, &[discoverable])
        .map_err(webauthn_error)?;
    if !result.user_verified() {
        tracing::debug!(
            flow_id,
            passkey_id = passkey.id,
            "passkey login rejected because user verification was not present"
        );
        return Err(AsterError::auth_invalid_credentials(
            "passkey user verification required",
        ));
    }

    let now = Utc::now();
    let changed = stored.update_credential(&result).ok_or_else(|| {
        AsterError::auth_invalid_credentials("passkey credential update mismatch")
    })?;
    if changed {
        let credential = stored_passkey_credential(&passkey_to_json(&stored)?)?;
        if !passkey_repo::update_credential_after_auth(
            state.writer_db(),
            passkey.id,
            credential,
            result.backup_eligible(),
            result.backup_state(),
            u32_to_i64(result.counter()),
            now,
        )
        .await?
        {
            return Err(AsterError::auth_invalid_credentials("passkey not found"));
        }
        tracing::debug!(
            flow_id,
            passkey_id = passkey.id,
            "updated passkey credential after login"
        );
    } else if !passkey_repo::touch_last_used(state.writer_db(), passkey.id, now).await? {
        return Err(AsterError::auth_invalid_credentials("passkey not found"));
    } else {
        tracing::debug!(
            flow_id,
            passkey_id = passkey.id,
            "touched passkey last-used timestamp"
        );
    }

    let session = auth_service::issue_tokens_for_authenticated_user(state, user, req).await?;
    tracing::debug!(flow_id, passkey_id = passkey.id, "finished passkey login");
    Ok(PasskeyLoginResult {
        session,
        passkey_id: passkey.id,
        passkey_name: passkey.name,
    })
}
