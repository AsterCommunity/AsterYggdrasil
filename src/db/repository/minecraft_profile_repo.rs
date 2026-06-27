//! Minecraft profile repository.

use crate::entities::minecraft_profile::{self, Entity as MinecraftProfile};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::yggdrasil::MinecraftTextureModel;
use aster_forge_api::CursorSlice;
use aster_forge_db::search_query;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

#[derive(Debug, Clone, Default)]
pub struct MinecraftProfileFilters {
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub uuid: Option<String>,
    pub query: Option<String>,
}

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

pub async fn count_all<C: ConnectionTrait>(db: &C) -> Result<u64> {
    MinecraftProfile::find()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

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

pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<minecraft_profile::Model> {
    MinecraftProfile::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("minecraft profile #{id}")))
}

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
