//! Product-side Minecraft profile lookups and listings for account and
//! admin surfaces.
//!
//! Protocol-side profile behavior (Yggdrasil wire responses, rename flows)
//! lives in `services::yggdrasil`; this module owns the ownership checks and
//! error mapping shared by the account and admin product routes.

use crate::api::error_code::AsterErrorCode;
use crate::db::repository::minecraft_profile_repo;
use crate::errors::{AsterError, Result};
use crate::runtime::DatabaseRuntimeState;
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::minecraft_profile;

/// Filter shape accepted by [`list_profiles`]; re-exported so callers do not
/// import the repository directly.
pub use crate::db::repository::minecraft_profile_repo::MinecraftProfileFilters;

/// Validates an unsigned Minecraft profile UUID supplied on a route path.
pub fn validate_profile_uuid(uuid: &str) -> Result<()> {
    crate::api::dto::validation::validate_unsigned_uuid(uuid).map_err(|error| {
        AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        )
    })
}

/// Finds one of the user's own Minecraft profiles by profile UUID, mapping
/// UUID and ownership failures to their stable API error codes.
pub async fn find_owned_profile<S>(
    state: &S,
    uuid: &str,
    user_id: i64,
) -> Result<minecraft_profile::Model>
where
    S: DatabaseRuntimeState,
{
    validate_profile_uuid(uuid)?;
    minecraft_profile_repo::find_by_uuid_for_user(state.reader_db(), uuid, user_id)
        .await?
        .ok_or_else(|| {
            tracing::debug!(
                user_id,
                profile_uuid = %uuid,
                "minecraft profile lookup missed for owner"
            );
            AsterError::record_not_found_code(
                AsterErrorCode::MinecraftProfileNotFound,
                format!("minecraft profile '{uuid}'"),
            )
        })
}

/// Finds any Minecraft profile by profile UUID without an ownership filter;
/// admin-facing lookups use this variant.
pub async fn find_any_profile<S>(state: &S, uuid: &str) -> Result<minecraft_profile::Model>
where
    S: DatabaseRuntimeState,
{
    validate_profile_uuid(uuid)?;
    minecraft_profile_repo::find_by_uuid(state.reader_db(), uuid)
        .await?
        .ok_or_else(|| {
            tracing::debug!(profile_uuid = %uuid, "minecraft profile lookup missed");
            AsterError::record_not_found_code(
                AsterErrorCode::MinecraftProfileNotFound,
                format!("minecraft profile '{uuid}'"),
            )
        })
}

/// Cursor-paginated profile listing restricted to one user's own profiles,
/// optionally narrowed by a name/UUID keyword.
pub async fn list_profiles_for_user<S>(
    state: &S,
    user_id: i64,
    query: Option<String>,
    limit: u64,
    after_id: Option<i64>,
) -> Result<CursorSlice<minecraft_profile::Model>>
where
    S: DatabaseRuntimeState,
{
    list_profiles(
        state,
        MinecraftProfileFilters {
            user_id: Some(user_id),
            query,
            ..Default::default()
        },
        limit,
        after_id,
    )
    .await
}

/// Cursor-paginated profile listing with explicit filters (admin surfaces).
pub async fn list_profiles<S>(
    state: &S,
    filters: MinecraftProfileFilters,
    limit: u64,
    after_id: Option<i64>,
) -> Result<CursorSlice<minecraft_profile::Model>>
where
    S: DatabaseRuntimeState,
{
    minecraft_profile_repo::list_cursor(state.reader_db(), filters, limit, after_id).await
}

/// Counts live Minecraft profiles per user id; users without profiles are
/// absent from the returned map.
pub async fn count_profiles_for_users<S>(
    state: &S,
    user_ids: &[i64],
) -> Result<std::collections::HashMap<i64, u64>>
where
    S: DatabaseRuntimeState,
{
    crate::db::repository::user_repo::count_profiles_by_user_ids(state.reader_db(), user_ids).await
}
