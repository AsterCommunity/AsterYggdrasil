//! Minecraft texture library tag repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_forge_db::search_query;
use aster_yggdrasil_model::entities::{
    minecraft_texture_tag::{self, Entity as MinecraftTextureTag},
    minecraft_texture_tag_binding::{self, Entity as MinecraftTextureTagBinding},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select, Set,
};
use std::collections::BTreeMap;

/// Input for creating a texture library tag.
#[derive(Debug, Clone)]
pub struct CreateMinecraftTextureTag<'a> {
    /// Display name shown in the admin panel.
    pub name: &'a str,
    /// Canonicalized name used for uniqueness checks and lookups.
    pub normalized_name: &'a str,
    /// Tag color, e.g. a hex string; format is validated upstream.
    pub color: &'a str,
    /// Manual ordering weight; lower values sort first.
    pub sort_order: i32,
}

/// Partial update for a texture tag; `None` fields are left unchanged.
#[derive(Debug, Clone, Default)]
pub struct UpdateMinecraftTextureTag {
    /// Replaces the display name when set.
    pub name: Option<String>,
    /// Replaces the canonicalized name when set.
    pub normalized_name: Option<String>,
    /// Replaces the tag color when set.
    pub color: Option<String>,
    /// Replaces the ordering weight when set.
    pub sort_order: Option<i32>,
}

/// Optional filters for the tag list; all set filters combine with AND.
#[derive(Debug, Clone, Default)]
pub struct MinecraftTextureTagListFilter {
    /// Case-insensitive substring match against display and normalized names when non-empty.
    pub keyword: Option<String>,
}

/// Inserts a new texture tag row.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateMinecraftTextureTag<'_>,
) -> Result<minecraft_texture_tag::Model> {
    let now = chrono::Utc::now();
    minecraft_texture_tag::ActiveModel {
        name: Set(input.name.to_string()),
        normalized_name: Set(input.normalized_name.to_string()),
        color: Set(input.color.to_string()),
        sort_order: Set(input.sort_order),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

/// Lists every tag ordered by sort order then display name ascending.
pub async fn list<C: ConnectionTrait>(db: &C) -> Result<Vec<minecraft_texture_tag::Model>> {
    MinecraftTextureTag::find()
        .order_by_asc(minecraft_texture_tag::Column::SortOrder)
        .order_by_asc(minecraft_texture_tag::Column::Name)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Paginates tags using a keyset cursor over `(sort_order, name, id)` ascending.
///
/// `limit` is clamped to 1..=100; `after` is the cursor of the last item of the previous page.
/// The returned [`CursorSlice`] carries the filtered total and a next cursor, which is absent
/// on the last page.
pub async fn list_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after: Option<(i32, String, i64)>,
    filter: MinecraftTextureTagListFilter,
) -> Result<CursorSlice<minecraft_texture_tag::Model>> {
    let limit = limit.clamp(1, 100);
    let base = texture_tag_list_query(filter);
    let total = base
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    if total == 0 {
        return Ok(CursorSlice::empty(total));
    }

    let mut query = base;
    if let Some((sort_order, name, id)) = after {
        query = query.filter(
            Condition::any()
                .add(minecraft_texture_tag::Column::SortOrder.gt(sort_order))
                .add(
                    Condition::all()
                        .add(minecraft_texture_tag::Column::SortOrder.eq(sort_order))
                        .add(minecraft_texture_tag::Column::Name.gt(name.clone())),
                )
                .add(
                    Condition::all()
                        .add(minecraft_texture_tag::Column::SortOrder.eq(sort_order))
                        .add(minecraft_texture_tag::Column::Name.eq(name))
                        .add(minecraft_texture_tag::Column::Id.gt(id)),
                ),
        );
    }

    let items = query
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

fn texture_tag_list_query(filter: MinecraftTextureTagListFilter) -> Select<MinecraftTextureTag> {
    let mut query = MinecraftTextureTag::find()
        .order_by_asc(minecraft_texture_tag::Column::SortOrder)
        .order_by_asc(minecraft_texture_tag::Column::Name)
        .order_by_asc(minecraft_texture_tag::Column::Id);

    if let Some(keyword) = filter
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(
            Condition::any()
                .add(search_query::lower_like_condition(
                    minecraft_texture_tag::Column::Name,
                    keyword,
                ))
                .add(search_query::lower_like_condition(
                    minecraft_texture_tag::Column::NormalizedName,
                    keyword,
                )),
        );
    }

    query
}

/// Loads a tag by id.
///
/// Returns `None` when no tag with that id exists.
pub async fn find_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture_tag::Model>> {
    MinecraftTextureTag::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads a tag by its canonicalized name.
///
/// Returns `None` when no tag normalizes to that value; normalized names are unique, so at
/// most one row matches.
pub async fn find_by_normalized_name<C: ConnectionTrait>(
    db: &C,
    normalized_name: &str,
) -> Result<Option<minecraft_texture_tag::Model>> {
    MinecraftTextureTag::find()
        .filter(minecraft_texture_tag::Column::NormalizedName.eq(normalized_name))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Batch-loads tags by id with list ordering (sort order, then name).
///
/// An empty `ids` slice returns an empty list without querying; unknown ids are simply absent.
pub async fn find_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> Result<Vec<minecraft_texture_tag::Model>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    MinecraftTextureTag::find()
        .filter(minecraft_texture_tag::Column::Id.is_in(ids.iter().copied()))
        .order_by_asc(minecraft_texture_tag::Column::SortOrder)
        .order_by_asc(minecraft_texture_tag::Column::Name)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Applies the set fields of the update to a tag row, stamping `updated_at`.
pub async fn update<C: ConnectionTrait>(
    db: &C,
    tag: minecraft_texture_tag::Model,
    input: UpdateMinecraftTextureTag,
) -> Result<minecraft_texture_tag::Model> {
    let mut active: minecraft_texture_tag::ActiveModel = tag.into();
    if let Some(name) = input.name {
        active.name = Set(name);
    }
    if let Some(normalized_name) = input.normalized_name {
        active.normalized_name = Set(normalized_name);
    }
    if let Some(color) = input.color {
        active.color = Set(color);
    }
    if let Some(sort_order) = input.sort_order {
        active.sort_order = Set(sort_order);
    }
    active.updated_at = Set(chrono::Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Deletes a tag by id.
///
/// Returns `false` when no tag with that id exists; bindings referencing the tag are not
/// cleaned up here.
pub async fn delete<C: ConnectionTrait>(db: &C, id: i64) -> Result<bool> {
    let result = MinecraftTextureTag::delete_by_id(id)
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected > 0)
}

/// Replaces a texture's full tag set: deletes all existing bindings, then inserts `tag_ids`
/// in order.
///
/// Not transactional — callers must wrap in a transaction when atomicity matters, since each
/// insert commits independently otherwise.
pub async fn replace_texture_tags<C: ConnectionTrait>(
    db: &C,
    texture_id: i64,
    tag_ids: &[i64],
) -> Result<()> {
    MinecraftTextureTagBinding::delete_many()
        .filter(minecraft_texture_tag_binding::Column::TextureId.eq(texture_id))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    let now = chrono::Utc::now();
    for tag_id in tag_ids {
        minecraft_texture_tag_binding::ActiveModel {
            texture_id: Set(texture_id),
            tag_id: Set(*tag_id),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    }
    Ok(())
}

/// Lists the tags bound to one texture, in list ordering (sort order, then name).
pub async fn list_for_texture<C: ConnectionTrait>(
    db: &C,
    texture_id: i64,
) -> Result<Vec<minecraft_texture_tag::Model>> {
    let rows = MinecraftTextureTagBinding::find()
        .filter(minecraft_texture_tag_binding::Column::TextureId.eq(texture_id))
        .join(
            JoinType::InnerJoin,
            minecraft_texture_tag_binding::Relation::MinecraftTextureTag.def(),
        )
        .select_also(MinecraftTextureTag)
        .order_by_asc(minecraft_texture_tag::Column::SortOrder)
        .order_by_asc(minecraft_texture_tag::Column::Name)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(binding, tag)| {
            tag.ok_or_else(|| AsterError::record_not_found(format!("tag '{}'", binding.tag_id)))
        })
        .collect()
}

/// Batch-loads tags for several textures into a map keyed by texture id.
///
/// An empty `texture_ids` slice returns an empty map without querying; each value list keeps
/// list ordering, and textures without tags are absent from the map.
pub async fn list_for_texture_ids<C: ConnectionTrait>(
    db: &C,
    texture_ids: &[i64],
) -> Result<BTreeMap<i64, Vec<minecraft_texture_tag::Model>>> {
    if texture_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = MinecraftTextureTagBinding::find()
        .filter(minecraft_texture_tag_binding::Column::TextureId.is_in(texture_ids.iter().copied()))
        .join(
            JoinType::InnerJoin,
            minecraft_texture_tag_binding::Relation::MinecraftTextureTag.def(),
        )
        .select_also(MinecraftTextureTag)
        .order_by_asc(minecraft_texture_tag::Column::SortOrder)
        .order_by_asc(minecraft_texture_tag::Column::Name)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    let mut map = BTreeMap::new();
    for (binding, tag) in rows {
        let tag =
            tag.ok_or_else(|| AsterError::record_not_found(format!("tag '{}'", binding.tag_id)))?;
        map.entry(binding.texture_id)
            .or_insert_with(Vec::new)
            .push(tag);
    }
    Ok(map)
}
