use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::db::repository::{minecraft_profile_repo, yggdrasil_token_repo};
use crate::errors::{AsterError, Result};
use crate::runtime::{
    CacheRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState, TextureStorageRuntimeState,
};
use crate::services::{texture, user};
use aster_yggdrasil_model::entities::minecraft_profile;
use aster_yggdrasil_model::types::user::UserBanScope;
use aster_yggdrasil_protocol::YggdrasilProfile;
use serde::Serialize;

/// Outcome of deleting a Minecraft profile: the profile snapshot plus the
/// cleanup performed alongside it.
#[derive(Debug, Clone)]
pub struct DeleteMinecraftProfileResult {
    /// The profile row as it looked right before deletion.
    pub profile: minecraft_profile::Model,
    /// Number of texture bindings removed with the profile.
    pub deleted_texture_count: usize,
    /// Number of tokens revoked because they selected this profile.
    pub revoked_token_count: u64,
}

/// Outcome of a profile rename, keeping the old name and the token state
/// change triggered by it.
#[derive(Debug, Clone)]
pub struct RenameMinecraftProfileResult {
    /// The profile row after the rename.
    pub profile: minecraft_profile::Model,
    /// Name the profile had before the rename.
    pub old_name: String,
    /// Number of bound tokens temporarily invalidated so launchers must
    /// refresh to pick up the new name.
    pub temporarily_invalidated_token_count: u64,
}

/// Administrative/user-facing info about a Minecraft profile.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct MinecraftProfileInfo {
    /// Profile row id.
    pub id: i64,
    /// Owning user's row id.
    pub user_id: i64,
    /// Profile UUID (unhyphenated 32-char hex) used as the Yggdrasil player id.
    pub uuid: String,
    /// Current player name.
    pub name: String,
    /// Comma-separated texture types the profile may upload (skin/cape).
    pub uploadable_textures: String,
    /// Default skin arm model used when no skin texture overrides it.
    pub texture_model: aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel,
    /// When the profile was created (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the profile was last updated (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Projects a profile row into the minimal Yggdrasil profile shape
/// (`id`/`name`, no properties) used in profile listings.
pub fn profile_summary(profile: &minecraft_profile::Model) -> YggdrasilProfile {
    YggdrasilProfile {
        id: profile.uuid.clone(),
        name: profile.name.clone(),
        properties: None,
    }
}

/// Projects a profile row into [`MinecraftProfileInfo`]; pure mapping.
pub fn profile_info(profile: &minecraft_profile::Model) -> MinecraftProfileInfo {
    MinecraftProfileInfo {
        id: profile.id,
        user_id: profile.user_id,
        uuid: profile.uuid.clone(),
        name: profile.name.clone(),
        uploadable_textures: profile.uploadable_textures.clone(),
        texture_model: profile.texture_model,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    }
}

/// Creates a Minecraft profile for the user: name is validated (3-16 ASCII
/// letters/digits/underscores) and must be unique among live profiles; the
/// UUID is a fresh random unhyphenated UUID and the texture upload allowlist
/// comes from the runtime Yggdrasil policy.
pub async fn create_profile<S>(
    state: &S,
    user_id: i64,
    name: &str,
) -> Result<minecraft_profile::Model>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(user_id, name_len = name.len(), "creating minecraft profile");
    user::ban::ensure_user_not_banned(state, user_id, UserBanScope::MinecraftProfileManage).await?;
    validate_profile_name(name)?;
    if minecraft_profile_repo::find_by_name(state.reader_db(), name)
        .await?
        .is_some()
    {
        tracing::debug!(
            user_id,
            profile_name = name,
            "minecraft profile creation rejected because name is taken"
        );
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::MinecraftProfileNameTaken,
            "profile name already exists",
        ));
    }
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    let profile = minecraft_profile_repo::create(
        state.writer_db(),
        user_id,
        &aster_forge_utils::id::new_short_token(),
        name,
        aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel::Default,
        &policy.uploadable_textures_value(),
    )
    .await?;
    tracing::debug!(
        user_id,
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        uploadable_textures = %profile.uploadable_textures,
        "created minecraft profile"
    );
    Ok(profile)
}

/// Deletes one of the user's profiles by UUID: removes its texture bindings
/// and assets, revokes tokens bound to it, then deletes the row. `None` when
/// the UUID does not belong to this user.
pub async fn delete_profile_for_user<S>(
    state: &S,
    user_id: i64,
    uuid: &str,
) -> Result<Option<DeleteMinecraftProfileResult>>
where
    S: CacheRuntimeState + DatabaseRuntimeState + TextureStorageRuntimeState,
{
    tracing::debug!(
        user_id,
        profile_uuid = uuid,
        "deleting minecraft profile for user"
    );
    user::ban::ensure_user_not_banned(state, user_id, UserBanScope::MinecraftProfileManage).await?;
    let Some(profile) =
        minecraft_profile_repo::find_by_uuid_for_user(state.reader_db(), uuid, user_id).await?
    else {
        tracing::debug!(
            user_id,
            profile_uuid = uuid,
            "minecraft profile delete skipped because profile was not found"
        );
        return Ok(None);
    };

    super::token::invalidate_token_cache_for_selected_profile(state, profile.id).await?;
    let deleted_textures = texture::delete_all_textures_for_profile(state, &profile)
        .await
        .map_err(|error| AsterError::internal_error(error.protocol_message()))?;
    let revoked_token_count =
        yggdrasil_token_repo::revoke_all_for_selected_profile(state.writer_db(), profile.id)
            .await?;
    let deleted_profile = minecraft_profile_repo::delete_by_id(state.writer_db(), profile.id)
        .await?
        .unwrap_or_else(|| profile.clone());

    tracing::debug!(
        user_id,
        profile_id = deleted_profile.id,
        profile_uuid = %deleted_profile.uuid,
        deleted_texture_count = deleted_textures.len(),
        revoked_token_count,
        "deleted minecraft profile"
    );
    Ok(Some(DeleteMinecraftProfileResult {
        profile: deleted_profile,
        deleted_texture_count: deleted_textures.len(),
        revoked_token_count,
    }))
}

/// User-facing rename: checks ban status and name validity, verifies the
/// profile belongs to the user, then delegates to [`rename_profile`]. `None`
/// when the UUID does not belong to this user.
pub async fn rename_profile_for_user<S>(
    state: &S,
    user_id: i64,
    uuid: &str,
    new_name: &str,
) -> Result<Option<RenameMinecraftProfileResult>>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    tracing::debug!(
        user_id,
        profile_uuid = uuid,
        new_name_len = new_name.len(),
        "renaming minecraft profile for user"
    );
    user::ban::ensure_user_not_banned(state, user_id, UserBanScope::MinecraftProfileManage).await?;
    validate_profile_name(new_name)?;
    let Some(profile) =
        minecraft_profile_repo::find_by_uuid_for_user(state.reader_db(), uuid, user_id).await?
    else {
        tracing::debug!(
            user_id,
            profile_uuid = uuid,
            "minecraft profile rename skipped because profile was not found"
        );
        return Ok(None);
    };
    rename_profile(state, profile, new_name).await.map(Some)
}

/// Renames a profile inside a transaction: UUID, texture bindings, and audit
/// trail stay intact, tokens bound to the profile are temporarily invalidated
/// so launchers must call refresh to observe the new name. Same-name requests
/// are a counted no-op.
pub async fn rename_profile<S>(
    state: &S,
    profile: minecraft_profile::Model,
    new_name: &str,
) -> Result<RenameMinecraftProfileResult>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    validate_profile_name(new_name)?;
    if profile.name == new_name {
        tracing::debug!(
            profile_id = profile.id,
            profile_uuid = %profile.uuid,
            "minecraft profile rename no-op because name is unchanged"
        );
        return Ok(RenameMinecraftProfileResult {
            old_name: profile.name.clone(),
            profile,
            temporarily_invalidated_token_count: 0,
        });
    }

    let existing = minecraft_profile_repo::find_by_name(state.reader_db(), new_name).await?;
    if existing.is_some_and(|existing| existing.id != profile.id) {
        tracing::debug!(
            profile_id = profile.id,
            profile_uuid = %profile.uuid,
            new_profile_name = new_name,
            "minecraft profile rename rejected because name is taken"
        );
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::MinecraftProfileNameTaken,
            "profile name already exists",
        ));
    }

    let old_name = profile.name.clone();
    let token_hashes =
        super::token::invalidate_token_cache_for_selected_profile(state, profile.id).await?;
    let result = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let updated = minecraft_profile_repo::update_name_by_id(txn, profile.id, new_name)
            .await?
            .ok_or_else(|| {
                AsterError::record_not_found(format!("minecraft profile '{}'", profile.uuid))
            })?;
        let temporarily_invalidated_token_count =
            yggdrasil_token_repo::temporarily_invalidate_all_for_selected_profile(txn, profile.id)
                .await?;
        Ok(RenameMinecraftProfileResult {
            profile: updated,
            old_name,
            temporarily_invalidated_token_count,
        })
    })
    .await?;
    // Clear the same cache entries again after commit so tokens repopulated
    // during the rename transaction observe the temporary invalidation state.
    super::token::invalidate_token_cache_hashes(state, &token_hashes).await;
    crate::services::yggdrasil::protocol::invalidate_profile_name_summary_cache(
        state,
        &result.old_name,
    )
    .await;
    crate::services::yggdrasil::protocol::invalidate_profile_name_summary_cache(
        state,
        &result.profile.name,
    )
    .await;
    Ok(result)
}

/// Validates a player name against the shared Yggdrasil rule: 3-16 ASCII
/// letters, digits, or underscores.
pub fn validate_profile_name(name: &str) -> Result<()> {
    if aster_yggdrasil_protocol::minecraft_profile_name_is_valid(name) {
        Ok(())
    } else {
        Err(AsterError::validation_error(
            "profile name must be 3-16 ASCII letters, numbers, or underscores",
        ))
    }
}
