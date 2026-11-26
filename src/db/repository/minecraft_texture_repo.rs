//! Minecraft texture asset repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::minecraft_texture::{self, Entity as MinecraftTexture};
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureLibraryStatus, yggdrasil::MinecraftTextureModel,
    yggdrasil::MinecraftTextureType, yggdrasil::MinecraftTextureVisibility,
    yggdrasil::TextureTagSearchMethod,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};

/// Input for registering an uploaded texture file as a texture record.
#[derive(Debug, Clone)]
pub struct CreateMinecraftTexture<'a> {
    /// Owning user.
    pub user_id: i64,
    /// Texture slot type (skin, cape, ...).
    pub texture_type: MinecraftTextureType,
    /// Content hash of the file; identical files share a hash.
    pub hash: &'a str,
    /// Key the file was stored under in object storage.
    pub storage_key: &'a str,
    /// Validated MIME type of the file.
    pub mime_type: &'a str,
    /// File size in bytes.
    pub file_size: i64,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Player model the texture targets (default or slim arms).
    pub texture_model: MinecraftTextureModel,
    /// Who may read the texture.
    pub visibility: MinecraftTextureVisibility,
    /// Whether the texture participates in the wardrobe/library flows.
    pub is_wardrobe_item: bool,
    /// Optional user-facing name for wardrobe display.
    pub display_name: Option<&'a str>,
}

/// Optional filters for wardrobe texture listings; all set filters combine with AND.
#[derive(Debug, Clone, Default)]
pub struct WardrobeTextureListFilter {
    /// Restricts results to one texture slot type when set.
    pub texture_type: Option<MinecraftTextureType>,
    /// Tag ids the results must be bound to (meaning depends on `tag_search_method`).
    pub tag_ids: Vec<i64>,
    /// Whether the texture must carry all of `tag_ids` or any of them.
    pub tag_search_method: TextureTagSearchMethod,
    /// Case-insensitive substring match against hash, MIME type, display name, and enum names
    /// when non-empty.
    pub keyword: Option<String>,
}

/// Optional filters for the admin texture library listing; all set filters combine with AND.
#[derive(Debug, Clone, Default)]
pub struct AdminTextureLibraryListFilter {
    /// Restricts results to one texture slot type when set.
    pub texture_type: Option<MinecraftTextureType>,
    /// Restricts results to one visibility when set.
    pub visibility: Option<MinecraftTextureVisibility>,
    /// Restricts results to one library review status when set.
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    /// When true, only library-published public textures; when false, everything else.
    pub published: Option<bool>,
    /// Tag ids the results must be bound to (meaning depends on `tag_search_method`).
    pub tag_ids: Vec<i64>,
    /// Whether the texture must carry all of `tag_ids` or any of them.
    pub tag_search_method: TextureTagSearchMethod,
    /// Case-insensitive substring match against hash, MIME type, display name, and enum names
    /// when non-empty.
    pub keyword: Option<String>,
}

/// Partial wardrobe metadata update; `None` fields are left unchanged.
///
/// Double-`Option` fields distinguish clearing a value (`Some(None)`) from leaving it alone
/// (`None`).
#[derive(Debug, Clone)]
pub struct UpdateWardrobeTextureMetadata {
    /// Replaces or clears the wardrobe display name when set.
    pub display_name: Option<Option<String>>,
    /// Replaces the player model when set.
    pub texture_model: Option<MinecraftTextureModel>,
    /// Replaces the visibility when set; switching to private also resets library review state.
    pub visibility: Option<MinecraftTextureVisibility>,
}

/// Library review decision applied by an admin.
#[derive(Debug, Clone)]
pub struct UpdateTextureLibraryReview {
    /// New library review status.
    pub library_status: MinecraftTextureLibraryStatus,
    /// Replaces or clears the submission timestamp when set.
    pub library_submitted_at: Option<Option<DateTime<Utc>>>,
    /// Replaces or clears the review timestamp when set.
    pub library_reviewed_at: Option<Option<DateTime<Utc>>>,
    /// Replaces or clears the reviewing admin when set.
    pub library_reviewer_user_id: Option<Option<i64>>,
    /// Replaces or clears the review note when set.
    pub library_review_note: Option<Option<String>>,
}

/// Inserts a new texture record with `Private` library status.
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

/// Counts all texture records.
pub async fn count_all<C: ConnectionTrait>(db: &C) -> Result<u64> {
    MinecraftTexture::find()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Counts textures uploaded in the `[start, end)` interval.
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

/// Loads a texture by id scoped to its owner.
///
/// Returns `None` when the texture does not exist or belongs to a different user.
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

/// Loads a texture by id, regardless of owner or visibility.
///
/// Returns `None` when no texture with that id exists.
pub async fn find_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture::Model>> {
    MinecraftTexture::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Batch-loads textures by id, ordered by id ascending.
///
/// An empty `ids` slice returns an empty list without querying; unknown ids are simply absent.
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

/// Loads a texture readable through the public wardrobe: wardrobe item, public visibility,
/// and published library status must all hold.
///
/// Returns `None` when any of those conditions fails.
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

/// Lists all wardrobe textures owned by a user, most recently updated first.
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

/// Paginates a user's wardrobe textures with filters, using the shared `(updated_at, id)`
/// keyset cursor, newest first.
///
/// See [`WardrobeTextureListFilter`] for filter semantics and [`fetch_updated_at_cursor_slice`]
/// for pagination behavior.
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

/// Paginates publicly readable wardrobe textures (public visibility plus published library
/// status) with the same cursor semantics as [`list_by_user_cursor`].
pub async fn list_public_wardrobe_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
    filter: WardrobeTextureListFilter,
) -> Result<CursorSlice<minecraft_texture::Model>> {
    let base = public_wardrobe_query(filter);
    fetch_updated_at_cursor_slice(db, base, limit, after).await
}

/// Paginates wardrobe textures across all owners for the admin library view, with the same
/// cursor semantics as [`list_by_user_cursor`].
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
                    .column(aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Column::TextureId)
                    .from(aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Entity)
                    .and_where(
                        aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Column::TagId
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
                .column(aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Column::TextureId)
                .from(aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Entity)
                .and_where(aster_yggdrasil_model::entities::minecraft_texture_tag_binding::Column::TagId.eq(tag_id))
                .to_owned(),
        )
        .into()
}

/// Finds the user's wardrobe texture matching an upload fingerprint: owner, slot type, content
/// hash, and player model must all match.
///
/// Used to deduplicate re-uploads of the same file; returns `None` on first upload or any
/// mismatch.
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

/// Finds a user's wardrobe texture by exact display name.
///
/// Display names are unique per user; returns `None` when the user has no wardrobe texture
/// under that name.
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

/// Lists every texture record across all owners, ordered by id ascending.
pub async fn list_all<C: ConnectionTrait>(db: &C) -> Result<Vec<minecraft_texture::Model>> {
    MinecraftTexture::find()
        .order_by_asc(minecraft_texture::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Lists every texture record sharing a content hash, ordered by id ascending.
///
/// Multiple records can share one stored file when users upload identical content.
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

/// Loads an arbitrary texture record with the given content hash.
///
/// Returns `None` when the hash is unknown; used mainly to check whether a file was already
/// uploaded by anyone.
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

/// Flags a texture as a wardrobe item, stamping `updated_at`.
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

/// Creates a wardrobe record for another user pointing at the same stored file.
///
/// Copies file metadata (hash, storage key, size, dimensions) from the source texture while
/// giving the copy its own owner, visibility, name, and `Private` library status. Used when a
/// user picks a public library texture into their own wardrobe.
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

/// Updates wardrobe metadata on a texture owned by `user_id`.
///
/// Returns `None` without modifying anything when the texture belongs to another user or is
/// not a wardrobe item. Switching visibility to `Private` also resets the library review
/// state (status back to `Private`, timestamps, reviewer, and note cleared).
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

/// Applies a library review decision to a texture, stamping `updated_at`.
///
/// Double-`Option` fields in [`UpdateTextureLibraryReview`] control whether review timestamps,
/// reviewer, and note are kept, replaced, or cleared.
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

/// Counts texture records referencing a storage key; zero means the stored file is unreferenced
/// and safe to delete from object storage.
pub async fn count_by_storage_key<C: ConnectionTrait>(db: &C, storage_key: &str) -> Result<u64> {
    MinecraftTexture::find()
        .filter(minecraft_texture::Column::StorageKey.eq(storage_key))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Deletes every texture record sharing a content hash, returning the removed records.
///
/// Callers must check [`count_by_storage_key`] afterwards before removing the stored file.
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

/// Deletes a texture owned by `user_id`, returning the removed record.
///
/// Returns `None` when the texture does not exist or belongs to a different user.
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

/// Deletes a wardrobe texture by id, returning the removed record.
///
/// Returns `None` when the id is unknown or the texture is not a wardrobe item (profile-bound
/// textures are protected from this path).
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
