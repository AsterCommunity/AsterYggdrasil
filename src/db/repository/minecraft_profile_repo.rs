//! Minecraft profile repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_forge_db::search_query;
use aster_yggdrasil_model::entities::minecraft_profile::{self, Entity as MinecraftProfile};
use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureModel;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

/// Optional filters for admin profile listing; all set filters combine with AND.
#[derive(Debug, Clone, Default)]
pub struct MinecraftProfileFilters {
    /// Restricts results to profiles owned by this user.
    pub user_id: Option<i64>,
    /// Exact (case-sensitive) profile name match when non-empty.
    pub name: Option<String>,
    /// Exact profile UUID match when non-empty.
    pub uuid: Option<String>,
    /// Case-insensitive substring match against name or UUID when non-empty.
    pub query: Option<String>,
}

/// Inserts a new Minecraft profile row.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    uuid: &str,
    name: &str,
    texture_model: MinecraftTextureModel,
    uploadable_textures: &str,
) -> Result<minecraft_profile::Model> {
    let now = chrono::Utc::now();
    minecraft_profile::ActiveModel {
        user_id: Set(user_id),
        uuid: Set(uuid.to_string()),
        name: Set(name.to_string()),
        texture_model: Set(texture_model),
        uploadable_textures: Set(uploadable_textures.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

/// Counts all Minecraft profiles.
pub async fn count_all<C: ConnectionTrait>(db: &C) -> Result<u64> {
    MinecraftProfile::find()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Counts profiles created in the `[start, end)` interval.
pub async fn count_created_between<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u64> {
    MinecraftProfile::find()
        .filter(minecraft_profile::Column::CreatedAt.gte(start))
        .filter(minecraft_profile::Column::CreatedAt.lt(end))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads a profile by id.
///
/// Returns `Err(record_not_found)` when no profile with that id exists.
pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<minecraft_profile::Model> {
    MinecraftProfile::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("minecraft profile #{id}")))
}

/// Loads a profile by its Minecraft UUID.
///
/// Returns `None` when no profile carries that UUID.
pub async fn find_by_uuid<C: ConnectionTrait>(
    db: &C,
    uuid: &str,
) -> Result<Option<minecraft_profile::Model>> {
    MinecraftProfile::find()
        .filter(minecraft_profile::Column::Uuid.eq(uuid))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads a profile by UUID scoped to its owner.
///
/// Returns `None` when the UUID is unknown or the profile belongs to a different user.
pub async fn find_by_uuid_for_user<C: ConnectionTrait>(
    db: &C,
    uuid: &str,
    user_id: i64,
) -> Result<Option<minecraft_profile::Model>> {
    MinecraftProfile::find()
        .filter(minecraft_profile::Column::Uuid.eq(uuid))
        .filter(minecraft_profile::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads a profile by its exact current name.
///
/// Returns `None` when no live profile uses that name; uniqueness is only guaranteed among
/// current profiles.
pub async fn find_by_name<C: ConnectionTrait>(
    db: &C,
    name: &str,
) -> Result<Option<minecraft_profile::Model>> {
    MinecraftProfile::find()
        .filter(minecraft_profile::Column::Name.eq(name))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Deletes a profile by id, returning the removed row.
///
/// Returns `None` when no profile with that id exists. Bound textures and tokens are not
/// touched here; callers handle cleanup.
pub async fn delete_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_profile::Model>> {
    let Some(existing) = MinecraftProfile::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
    else {
        return Ok(None);
    };
    MinecraftProfile::delete_by_id(id)
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(existing))
}

/// Renames a profile in place, stamping `updated_at`; the UUID stays fixed across renames.
///
/// Returns `None` when no profile with that id exists.
pub async fn update_name_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
    name: &str,
) -> Result<Option<minecraft_profile::Model>> {
    let Some(existing) = MinecraftProfile::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
    else {
        return Ok(None);
    };
    let mut active: minecraft_profile::ActiveModel = existing.into();
    active.name = Set(name.to_string());
    active.updated_at = Set(chrono::Utc::now());
    let updated = active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(updated))
}

/// Refreshes only a profile's `updated_at`, returning the updated row.
///
/// Returns `None` when no profile with that id exists.
pub async fn touch_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_profile::Model>> {
    let Some(existing) = MinecraftProfile::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
    else {
        return Ok(None);
    };
    let mut active: minecraft_profile::ActiveModel = existing.into();
    active.updated_at = Set(chrono::Utc::now());
    let updated = active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(updated))
}

/// Lists all profiles owned by a user, ordered by id ascending.
pub async fn list_by_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<minecraft_profile::Model>> {
    MinecraftProfile::find()
        .filter(minecraft_profile::Column::UserId.eq(user_id))
        .order_by_asc(minecraft_profile::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Paginates profiles using a keyset cursor over ascending id with the given filters.
///
/// The returned [`CursorSlice`] carries the filtered total count; its next cursor is the last
/// item's id (absent on the last page). `limit` 0 yields an empty page.
pub async fn list_cursor<C: ConnectionTrait>(
    db: &C,
    filters: MinecraftProfileFilters,
    limit: u64,
    after_id: Option<i64>,
) -> Result<CursorSlice<minecraft_profile::Model>> {
    let base = apply_filters(MinecraftProfile::find(), &filters);
    let total = base
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    if total == 0 || limit == 0 {
        return Ok(CursorSlice::empty(total));
    }

    let mut query = base;
    if let Some(after_id) = after_id {
        query = query.filter(minecraft_profile::Column::Id.gt(after_id));
    }

    let items = query
        .order_by_asc(minecraft_profile::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

fn apply_filters(
    mut query: sea_orm::Select<MinecraftProfile>,
    filters: &MinecraftProfileFilters,
) -> sea_orm::Select<MinecraftProfile> {
    if let Some(user_id) = filters.user_id {
        query = query.filter(minecraft_profile::Column::UserId.eq(user_id));
    }
    if let Some(name) = filters
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(minecraft_profile::Column::Name.eq(name));
    }
    if let Some(uuid) = filters
        .uuid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(minecraft_profile::Column::Uuid.eq(uuid));
    }
    if let Some(search) = filters
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(
            search_query::lower_like_condition(minecraft_profile::Column::Name, search).or(
                search_query::lower_like_condition(minecraft_profile::Column::Uuid, search),
            ),
        );
    }
    query
}

/// Batch-loads profiles by exact current names, ordered by id ascending.
///
/// An empty `names` slice returns an empty list without querying; names without a live profile
/// are simply absent from the result.
pub async fn list_by_names<C: ConnectionTrait>(
    db: &C,
    names: &[String],
) -> Result<Vec<minecraft_profile::Model>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    MinecraftProfile::find()
        .filter(minecraft_profile::Column::Name.is_in(names.iter().cloned()))
        .order_by_asc(minecraft_profile::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}
