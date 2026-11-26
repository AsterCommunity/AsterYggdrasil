//! Repository helpers for external auth providers.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, sea_query::Expr,
};

use crate::errors::{AsterError, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::external_auth_provider::{
    self, Entity as ExternalAuthProvider,
};
use aster_yggdrasil_model::types::external_auth::ExternalAuthProviderKind;

/// Lists every configured external auth provider, ordered by display name then id ascending.
pub async fn find_all(db: &DatabaseConnection) -> Result<Vec<external_auth_provider::Model>> {
    ExternalAuthProvider::find()
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Batch-loads providers by id with the same ordering as [`find_all`].
///
/// An empty `ids` slice returns an empty list without querying; missing ids are simply absent.
pub async fn find_by_ids(
    db: &DatabaseConnection,
    ids: &[i64],
) -> Result<Vec<external_auth_provider::Model>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::Id.is_in(ids.iter().copied()))
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Paginates providers restricted to the registry's supported kinds, ordered by display name
/// ascending.
///
/// `limit` is clamped to 1..=100; `after` is the `(display_name, id)` cursor of the last item
/// of the previous page. The returned [`CursorSlice`] carries the filtered total and a next
/// cursor, which is absent on the last page.
pub async fn find_cursor(
    db: &DatabaseConnection,
    limit: u64,
    after: Option<(String, i64)>,
    supported_kinds: impl IntoIterator<Item = ExternalAuthProviderKind>,
) -> Result<CursorSlice<external_auth_provider::Model>> {
    let base = ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::ProviderKind.is_in(supported_kinds));
    fetch_display_name_cursor(db, base, limit, after).await
}

async fn fetch_display_name_cursor(
    db: &DatabaseConnection,
    base: sea_orm::Select<ExternalAuthProvider>,
    limit: u64,
    after: Option<(String, i64)>,
) -> Result<CursorSlice<external_auth_provider::Model>> {
    let limit = limit.clamp(1, 100);
    let total = base.clone().count(db).await.map_err(AsterError::from)?;
    if total == 0 {
        return Ok(CursorSlice::empty(total));
    }

    let mut query = base;
    if let Some((display_name, id)) = after {
        query = query.filter(
            Condition::any()
                .add(external_auth_provider::Column::DisplayName.gt(display_name.clone()))
                .add(
                    Condition::all()
                        .add(external_auth_provider::Column::DisplayName.eq(display_name))
                        .add(external_auth_provider::Column::Id.gt(id)),
                ),
        );
    }

    let items = query
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_err(AsterError::from)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Lists all providers of one kind, ordered by display name then id ascending.
pub async fn find_all_by_kind(
    db: &DatabaseConnection,
    kind: ExternalAuthProviderKind,
) -> Result<Vec<external_auth_provider::Model>> {
    ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::ProviderKind.eq(kind))
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Lists providers currently enabled for login, ordered by display name then id ascending.
pub async fn find_enabled(db: &DatabaseConnection) -> Result<Vec<external_auth_provider::Model>> {
    ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::Enabled.eq(true))
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Paginates enabled providers restricted to supported kinds, with the same cursor semantics
/// as [`find_cursor`].
pub async fn find_enabled_cursor(
    db: &DatabaseConnection,
    limit: u64,
    after: Option<(String, i64)>,
    supported_kinds: impl IntoIterator<Item = ExternalAuthProviderKind>,
) -> Result<CursorSlice<external_auth_provider::Model>> {
    let base = ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::Enabled.eq(true))
        .filter(external_auth_provider::Column::ProviderKind.is_in(supported_kinds));
    fetch_display_name_cursor(db, base, limit, after).await
}

/// Lists enabled providers of one kind, ordered by display name then id ascending.
pub async fn find_enabled_by_kind(
    db: &DatabaseConnection,
    kind: ExternalAuthProviderKind,
) -> Result<Vec<external_auth_provider::Model>> {
    ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::Enabled.eq(true))
        .filter(external_auth_provider::Column::ProviderKind.eq(kind))
        .order_by_asc(external_auth_provider::Column::DisplayName)
        .order_by_asc(external_auth_provider::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Paginates enabled providers of one kind, with the same cursor semantics as [`find_cursor`].
pub async fn find_enabled_by_kind_cursor(
    db: &DatabaseConnection,
    kind: ExternalAuthProviderKind,
    limit: u64,
    after: Option<(String, i64)>,
) -> Result<CursorSlice<external_auth_provider::Model>> {
    let base = ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::Enabled.eq(true))
        .filter(external_auth_provider::Column::ProviderKind.eq(kind));
    fetch_display_name_cursor(db, base, limit, after).await
}

/// Loads a single provider by id.
///
/// Returns `Err(record_not_found)` when no provider with that id exists.
pub async fn find_by_id(db: &DatabaseConnection, id: i64) -> Result<external_auth_provider::Model> {
    ExternalAuthProvider::find_by_id(id)
        .one(db)
        .await
        .map_err(AsterError::from)?
        .ok_or_else(|| AsterError::record_not_found(format!("external auth provider #{id}")))
}

/// Loads a provider by its kind and stable key.
///
/// Returns `None` when no provider of that kind carries the key; the pair is unique, so at
/// most one row matches.
pub async fn find_by_kind_key(
    db: &DatabaseConnection,
    kind: ExternalAuthProviderKind,
    key: &str,
) -> Result<Option<external_auth_provider::Model>> {
    ExternalAuthProvider::find()
        .filter(external_auth_provider::Column::ProviderKind.eq(kind))
        .filter(external_auth_provider::Column::Key.eq(key))
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Inserts a new external auth provider row.
pub async fn create(
    db: &DatabaseConnection,
    model: external_auth_provider::ActiveModel,
) -> Result<external_auth_provider::Model> {
    model.insert(db).await.map_err(AsterError::from)
}

/// Persists changes on an existing external auth provider row.
pub async fn update(
    db: &DatabaseConnection,
    model: external_auth_provider::ActiveModel,
) -> Result<external_auth_provider::Model> {
    model.update(db).await.map_err(AsterError::from)
}

/// Deletes a provider by id.
///
/// Returns `Err(record_not_found)` when no provider with that id exists.
pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<()> {
    let result = ExternalAuthProvider::delete_by_id(id)
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    if result.rows_affected == 0 {
        return Err(AsterError::record_not_found(format!(
            "external auth provider #{id}"
        )));
    }
    Ok(())
}

/// Bumps `updated_at` on a provider without changing its configuration.
///
/// Returns `false` when the provider no longer exists.
pub async fn touch_updated_at(
    db: &DatabaseConnection,
    id: i64,
    updated_at: chrono::DateTime<Utc>,
) -> Result<bool> {
    let result = ExternalAuthProvider::update_many()
        .col_expr(
            external_auth_provider::Column::UpdatedAt,
            Expr::value(updated_at),
        )
        .filter(external_auth_provider::Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}
