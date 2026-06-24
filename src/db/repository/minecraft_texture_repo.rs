//! Minecraft texture asset repository.

use crate::entities::minecraft_texture::{self, Entity as MinecraftTexture};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureType,
    MinecraftTextureVisibility, TextureTagSearchMethod,
};
use aster_forge_api::CursorSlice;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};

#[derive(Debug, Clone)]
pub struct CreateMinecraftTexture<'a> {
    pub user_id: i64,
    pub texture_type: MinecraftTextureType,
    pub hash: &'a str,
    pub storage_key: &'a str,
    pub mime_type: &'a str,
    pub file_size: i64,
    pub width: i32,
    pub height: i32,
    pub texture_model: MinecraftTextureModel,
    pub visibility: MinecraftTextureVisibility,
    pub is_wardrobe_item: bool,
    pub display_name: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct WardrobeTextureListFilter {
    pub texture_type: Option<MinecraftTextureType>,
    pub tag_ids: Vec<i64>,
    pub tag_search_method: TextureTagSearchMethod,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdminTextureLibraryListFilter {
    pub texture_type: Option<MinecraftTextureType>,
    pub visibility: Option<MinecraftTextureVisibility>,
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    pub published: Option<bool>,
    pub tag_ids: Vec<i64>,
    pub tag_search_method: TextureTagSearchMethod,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateWardrobeTextureMetadata {
    pub display_name: Option<Option<String>>,
    pub texture_model: Option<MinecraftTextureModel>,
    pub visibility: Option<MinecraftTextureVisibility>,
}

#[derive(Debug, Clone)]
pub struct UpdateTextureLibraryReview {
    pub library_status: MinecraftTextureLibraryStatus,
    pub library_submitted_at: Option<Option<DateTime<Utc>>>,
    pub library_reviewed_at: Option<Option<DateTime<Utc>>>,
    pub library_reviewer_user_id: Option<Option<i64>>,
    pub library_review_note: Option<Option<String>>,
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateMinecraftTexture<'_>,
) -> Result<minecraft_texture::Model> {
    let now = chrono::Utc::now();
    minecraft_texture::ActiveModel {
        user_id: Set(input.user_id),
        texture_type: Set(input.texture_type),
        hash: Set(input.hash.to_string()),
        storage_key: Set(input.storage_key.to_string()),
        mime_type: Set(input.mime_type.to_string()),
        file_size: Set(input.file_size),
        width: Set(input.width),
        height: Set(input.height),
        texture_model: Set(input.texture_model),
        visibility: Set(input.visibility),
        is_wardrobe_item: Set(input.is_wardrobe_item),
        display_name: Set(input.display_name.map(str::to_string)),
        library_status: Set(MinecraftTextureLibraryStatus::Private),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

pub async fn count_all<C: ConnectionTrait>(db: &C) -> Result<u64> {
    MinecraftTexture::find()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn count_created_between<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u64> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::CreatedAt.gte(start))
        .filter(minecraft_texture::Column::CreatedAt.lt(end))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_id_for_user<C: ConnectionTrait>(
    db: &C,
    id: i64,
    user_id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::Id.eq(id))
        .filter(minecraft_texture::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_ids<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> Result<Vec<minecraft_texture::Model>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::Id.is_in(ids.iter().copied()))
        .order_by_asc(minecraft_texture::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_public_wardrobe_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::Id.eq(id))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .filter(minecraft_texture::Column::Visibility.eq(MinecraftTextureVisibility::Public))
        .filter(
            minecraft_texture::Column::LibraryStatus.eq(MinecraftTextureLibraryStatus::Published),
        )
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_by_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::UserId.eq(user_id))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .order_by_desc(minecraft_texture::Column::UpdatedAt)
        .order_by_desc(minecraft_texture::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_by_user_cursor<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
    filter: WardrobeTextureListFilter,
) -> Result<CursorSlice<minecraft_texture::Model>> {
    let base = current_user_wardrobe_query(user_id, filter);
    fetch_updated_at_cursor_slice(db, base, limit, after).await
}

pub async fn list_public_wardrobe_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
    filter: WardrobeTextureListFilter,
) -> Result<CursorSlice<minecraft_texture::Model>> {
    let base = public_wardrobe_query(filter);
    fetch_updated_at_cursor_slice(db, base, limit, after).await
}

pub async fn list_admin_library_textures_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
    filter: AdminTextureLibraryListFilter,
) -> Result<CursorSlice<minecraft_texture::Model>> {
    let base = admin_library_query(filter);
    fetch_updated_at_cursor_slice(db, base, limit, after).await
}

fn current_user_wardrobe_query(
    user_id: i64,
    filter: WardrobeTextureListFilter,
) -> Select<MinecraftTexture> {
    let query = MinecraftTexture::find()
        .filter(minecraft_texture::Column::UserId.eq(user_id))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true));
    apply_wardrobe_filter(query, filter)
}

fn public_wardrobe_query(filter: WardrobeTextureListFilter) -> Select<MinecraftTexture> {
    let query = MinecraftTexture::find()
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .filter(minecraft_texture::Column::Visibility.eq(MinecraftTextureVisibility::Public))
        .filter(
            minecraft_texture::Column::LibraryStatus.eq(MinecraftTextureLibraryStatus::Published),
        );
    apply_wardrobe_filter(query, filter)
}

fn admin_library_query(filter: AdminTextureLibraryListFilter) -> Select<MinecraftTexture> {
    let mut query =
        MinecraftTexture::find().filter(minecraft_texture::Column::IsWardrobeItem.eq(true));

    if let Some(texture_type) = filter.texture_type {
        query = query.filter(minecraft_texture::Column::TextureType.eq(texture_type));
    }
    if let Some(visibility) = filter.visibility {
        query = query.filter(minecraft_texture::Column::Visibility.eq(visibility));
    }
    if let Some(library_status) = filter.library_status {
        query = query.filter(minecraft_texture::Column::LibraryStatus.eq(library_status));
    }
    if let Some(published) = filter.published {
        if published {
            query = query
                .filter(
                    minecraft_texture::Column::Visibility.eq(MinecraftTextureVisibility::Public),
                )
                .filter(
                    minecraft_texture::Column::LibraryStatus
                        .eq(MinecraftTextureLibraryStatus::Published),
                );
        } else {
            query = query.filter(
                Condition::any()
                    .add(
                        minecraft_texture::Column::Visibility
                            .ne(MinecraftTextureVisibility::Public),
                    )
                    .add(
                        minecraft_texture::Column::LibraryStatus
                            .ne(MinecraftTextureLibraryStatus::Published),
                    ),
            );
        }
    }

    query = apply_tag_filter(
        query,
        &WardrobeTextureListFilter {
            texture_type: None,
            tag_ids: filter.tag_ids,
            tag_search_method: filter.tag_search_method,
            keyword: None,
        },
    );

    if let Some(keyword) = filter
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(texture_keyword_condition(keyword));
    }

    query
}

fn apply_wardrobe_filter(
    mut query: Select<MinecraftTexture>,
    filter: WardrobeTextureListFilter,
) -> Select<MinecraftTexture> {
    if let Some(texture_type) = filter.texture_type {
        query = query.filter(minecraft_texture::Column::TextureType.eq(texture_type));
    }

    query = apply_tag_filter(query, &filter);

    if let Some(keyword) = filter
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(texture_keyword_condition(keyword));
    }

    query
}

async fn fetch_updated_at_cursor_slice<C: ConnectionTrait>(
    db: &C,
    base: Select<MinecraftTexture>,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorSlice<minecraft_texture::Model>> {
    let total = base
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    if total == 0 || limit == 0 {
        return Ok(CursorSlice::empty(total));
    }

    let mut query = base;
    if let Some((after_updated_at, after_id)) = after {
        query = query.filter(
            Condition::any()
                .add(minecraft_texture::Column::UpdatedAt.lt(after_updated_at))
                .add(
                    Condition::all()
                        .add(minecraft_texture::Column::UpdatedAt.eq(after_updated_at))
                        .add(minecraft_texture::Column::Id.lt(after_id)),
                ),
        );
    }

    let items = query
        .order_by_desc(minecraft_texture::Column::UpdatedAt)
        .order_by_desc(minecraft_texture::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

fn apply_tag_filter(
    mut query: Select<MinecraftTexture>,
    filter: &WardrobeTextureListFilter,
) -> Select<MinecraftTexture> {
    if filter.tag_ids.is_empty() {
        return query;
    }

    match filter.tag_search_method {
        TextureTagSearchMethod::All => {
            for tag_id in &filter.tag_ids {
                query = query.filter(texture_has_tag_condition(*tag_id));
            }
            query
        }
        TextureTagSearchMethod::Any => query.filter(
            minecraft_texture::Column::Id.in_subquery(
                sea_orm::sea_query::Query::select()
                    .column(crate::entities::minecraft_texture_tag_binding::Column::TextureId)
                    .from(crate::entities::minecraft_texture_tag_binding::Entity)
                    .and_where(
                        crate::entities::minecraft_texture_tag_binding::Column::TagId
                            .is_in(filter.tag_ids.clone()),
                    )
                    .to_owned(),
            ),
        ),
    }
}

fn texture_has_tag_condition(tag_id: i64) -> Condition {
    minecraft_texture::Column::Id
        .in_subquery(
            sea_orm::sea_query::Query::select()
                .column(crate::entities::minecraft_texture_tag_binding::Column::TextureId)
                .from(crate::entities::minecraft_texture_tag_binding::Entity)
                .and_where(crate::entities::minecraft_texture_tag_binding::Column::TagId.eq(tag_id))
                .to_owned(),
        )
        .into()
}

pub async fn find_wardrobe_by_fingerprint<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    texture_type: MinecraftTextureType,
    hash: &str,
    texture_model: MinecraftTextureModel,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::UserId.eq(user_id))
        .filter(minecraft_texture::Column::TextureType.eq(texture_type))
        .filter(minecraft_texture::Column::Hash.eq(hash))
        .filter(minecraft_texture::Column::TextureModel.eq(texture_model))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .order_by_asc(minecraft_texture::Column::Id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_wardrobe_by_display_name<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    display_name: &str,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::UserId.eq(user_id))
        .filter(minecraft_texture::Column::DisplayName.eq(display_name))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .order_by_asc(minecraft_texture::Column::Id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_all<C: ConnectionTrait>(db: &C) -> Result<Vec<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .order_by_asc(minecraft_texture::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_by_hash<C: ConnectionTrait>(
    db: &C,
    hash: &str,
) -> Result<Vec<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::Hash.eq(hash))
        .order_by_asc(minecraft_texture::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_hash<C: ConnectionTrait>(
    db: &C,
    hash: &str,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::Hash.eq(hash))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn mark_as_wardrobe_item<C: ConnectionTrait>(
    db: &C,
    texture: minecraft_texture::Model,
) -> Result<minecraft_texture::Model> {
    let now = chrono::Utc::now();
    let mut active: minecraft_texture::ActiveModel = texture.into();
    active.is_wardrobe_item = Set(true);
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn create_wardrobe_copy<C: ConnectionTrait>(
    db: &C,
    texture: &minecraft_texture::Model,
    user_id: i64,
    visibility: MinecraftTextureVisibility,
    display_name: Option<&str>,
) -> Result<minecraft_texture::Model> {
    let now = chrono::Utc::now();
    minecraft_texture::ActiveModel {
        user_id: Set(user_id),
        texture_type: Set(texture.texture_type),
        hash: Set(texture.hash.clone()),
        storage_key: Set(texture.storage_key.clone()),
        mime_type: Set(texture.mime_type.clone()),
        file_size: Set(texture.file_size),
        width: Set(texture.width),
        height: Set(texture.height),
        texture_model: Set(texture.texture_model),
        visibility: Set(visibility),
        is_wardrobe_item: Set(true),
        display_name: Set(display_name.map(str::to_string)),
        library_status: Set(MinecraftTextureLibraryStatus::Private),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

pub async fn update_wardrobe_metadata_for_user<C: ConnectionTrait>(
    db: &C,
    texture: minecraft_texture::Model,
    user_id: i64,
    input: UpdateWardrobeTextureMetadata,
) -> Result<Option<minecraft_texture::Model>> {
    if texture.user_id != user_id || !texture.is_wardrobe_item {
        return Ok(None);
    }
    let now = chrono::Utc::now();
    let mut active: minecraft_texture::ActiveModel = texture.into();
    if let Some(display_name) = input.display_name {
        active.display_name = Set(display_name);
    }
    if let Some(texture_model) = input.texture_model {
        active.texture_model = Set(texture_model);
    }
    if let Some(visibility) = input.visibility {
        active.visibility = Set(visibility);
        if visibility == MinecraftTextureVisibility::Private {
            active.library_status = Set(MinecraftTextureLibraryStatus::Private);
            active.library_submitted_at = Set(None);
            active.library_reviewed_at = Set(None);
            active.library_reviewer_user_id = Set(None);
            active.library_review_note = Set(None);
        }
    }
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map(Some)
        .map_aster_err(AsterError::database_operation)
}

pub async fn update_library_review<C: ConnectionTrait>(
    db: &C,
    texture: minecraft_texture::Model,
    input: UpdateTextureLibraryReview,
) -> Result<minecraft_texture::Model> {
    let now = chrono::Utc::now();
    let mut active: minecraft_texture::ActiveModel = texture.into();
    active.library_status = Set(input.library_status);
    if let Some(submitted_at) = input.library_submitted_at {
        active.library_submitted_at = Set(submitted_at);
    }
    if let Some(reviewed_at) = input.library_reviewed_at {
        active.library_reviewed_at = Set(reviewed_at);
    }
    if let Some(reviewer_user_id) = input.library_reviewer_user_id {
        active.library_reviewer_user_id = Set(reviewer_user_id);
    }
    if let Some(review_note) = input.library_review_note {
        active.library_review_note = Set(review_note);
    }
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn count_by_storage_key<C: ConnectionTrait>(db: &C, storage_key: &str) -> Result<u64> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::StorageKey.eq(storage_key))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn delete_by_hash<C: ConnectionTrait>(
    db: &C,
    hash: &str,
) -> Result<Vec<minecraft_texture::Model>> {
    let textures = list_by_hash(db, hash).await?;
    for texture in &textures {
        texture
            .clone()
            .delete(db)
            .await
            .map_aster_err(AsterError::database_operation)?;
    }
    Ok(textures)
}

pub async fn delete_by_id_for_user<C: ConnectionTrait>(
    db: &C,
    id: i64,
    user_id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    let Some(texture) = find_by_id_for_user(db, id, user_id).await? else {
        return Ok(None);
    };
    texture
        .clone()
        .delete(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(texture))
}

pub async fn delete_wardrobe_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    let Some(texture) = find_by_id(db, id).await? else {
        return Ok(None);
    };
    if !texture.is_wardrobe_item {
        return Ok(None);
    }
    texture
        .clone()
        .delete(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(Some(texture))
}

fn texture_keyword_condition(keyword: &str) -> Condition {
    let normalized = keyword.to_ascii_lowercase();
    let mut condition = Condition::any()
        .add(minecraft_texture::Column::Hash.contains(keyword))
        .add(minecraft_texture::Column::MimeType.contains(keyword))
        .add(minecraft_texture::Column::DisplayName.contains(keyword));

    if normalized == MinecraftTextureType::Skin.as_str() {
        condition =
            condition.add(minecraft_texture::Column::TextureType.eq(MinecraftTextureType::Skin));
    }
    if normalized == MinecraftTextureType::Cape.as_str() {
        condition =
            condition.add(minecraft_texture::Column::TextureType.eq(MinecraftTextureType::Cape));
    }
    if normalized == MinecraftTextureModel::Default.as_str() {
        condition = condition
            .add(minecraft_texture::Column::TextureModel.eq(MinecraftTextureModel::Default));
    }
    if normalized == MinecraftTextureModel::Slim.as_str() {
        condition =
            condition.add(minecraft_texture::Column::TextureModel.eq(MinecraftTextureModel::Slim));
    }

    condition
}
