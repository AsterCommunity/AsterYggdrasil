//! Repository for upstream Yggdrasil session server forwarding.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, sea_query::Expr,
};

use crate::errors::{AsterError, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::yggdrasil_session_forward_server::{
    self, Entity as YggdrasilSessionForwardServer,
};
use aster_yggdrasil_model::types::{
    yggdrasil::YggdrasilSessionForwardProviderKind, yggdrasil::YggdrasilSessionForwardServerSortBy,
};

/// Lists upstream servers eligible for session forwarding: enabled with positive weight.
///
/// Ordered by priority ascending then id ascending — the order in which upstreams are tried.
pub async fn list_enabled_ordered(
    db: &DatabaseConnection,
) -> Result<Vec<yggdrasil_session_forward_server::Model>> {
    YggdrasilSessionForwardServer::find()
        .filter(yggdrasil_session_forward_server::Column::Enabled.eq(true))
        .filter(yggdrasil_session_forward_server::Column::Weight.gt(0))
        .order_by_asc(yggdrasil_session_forward_server::Column::Priority)
        .order_by_asc(yggdrasil_session_forward_server::Column::Id)
        .all(db)
        .await
        .map_err(AsterError::from)
}

/// Keyset cursor for forward-server pagination, matching the chosen sort order.
#[derive(Debug, Clone)]
pub enum SessionForwardServerCursor {
    /// Cursor for call-order sorting: last item's enabled flag, priority, and id.
    CallOrder {
        /// Enabled flag of the page's last row.
        enabled: bool,
        /// Priority of the page's last row.
        priority: i32,
        /// Id of the page's last row.
        id: i64,
    },
    /// Cursor for id sorting: last item's id.
    Id(i64),
}

/// Paginates forward servers using a keyset cursor, returning the filtered total.
///
/// `CallOrder` sorting puts enabled servers first, then ascending priority and id — the order
/// upstreams are consulted in. `Id` sorting is a plain id-ascending listing. `limit` is clamped
/// to 1..=100; a mismatched `after`/`sort_by` pair starts from the first page.
pub async fn find_cursor(
    db: &DatabaseConnection,
    limit: u64,
    after: Option<SessionForwardServerCursor>,
    sort_by: YggdrasilSessionForwardServerSortBy,
) -> Result<CursorSlice<yggdrasil_session_forward_server::Model>> {
    let limit = limit.clamp(1, 100);
    let base = YggdrasilSessionForwardServer::find();
    let total = base.clone().count(db).await.map_err(AsterError::from)?;
    if total == 0 {
        return Ok(CursorSlice::empty(total));
    }

    let mut query = base;
    match (sort_by, after) {
        (
            YggdrasilSessionForwardServerSortBy::CallOrder,
            Some(SessionForwardServerCursor::CallOrder {
                enabled,
                priority,
                id,
            }),
        ) => {
            query = query.filter(
                Condition::any()
                    .add(yggdrasil_session_forward_server::Column::Enabled.lt(enabled))
                    .add(
                        Condition::all()
                            .add(yggdrasil_session_forward_server::Column::Enabled.eq(enabled))
                            .add(yggdrasil_session_forward_server::Column::Priority.gt(priority)),
                    )
                    .add(
                        Condition::all()
                            .add(yggdrasil_session_forward_server::Column::Enabled.eq(enabled))
                            .add(yggdrasil_session_forward_server::Column::Priority.eq(priority))
                            .add(yggdrasil_session_forward_server::Column::Id.gt(id)),
                    ),
            );
        }
        (YggdrasilSessionForwardServerSortBy::Id, Some(SessionForwardServerCursor::Id(id))) => {
            query = query.filter(yggdrasil_session_forward_server::Column::Id.gt(id));
        }
        _ => {}
    }

    query = match sort_by {
        YggdrasilSessionForwardServerSortBy::CallOrder => query
            .order_by_desc(yggdrasil_session_forward_server::Column::Enabled)
            .order_by_asc(yggdrasil_session_forward_server::Column::Priority)
            .order_by_asc(yggdrasil_session_forward_server::Column::Id),
        YggdrasilSessionForwardServerSortBy::Id => {
            query.order_by_asc(yggdrasil_session_forward_server::Column::Id)
        }
    };

    let items = query
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_err(AsterError::from)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Loads a forward server by id.
///
/// Returns `Err(record_not_found)` when no server with that id exists.
pub async fn find_by_id(
    db: &DatabaseConnection,
    id: i64,
) -> Result<yggdrasil_session_forward_server::Model> {
    YggdrasilSessionForwardServer::find_by_id(id)
        .one(db)
        .await
        .map_err(AsterError::from)?
        .ok_or_else(|| {
            AsterError::record_not_found(format!("Yggdrasil session forward server #{id}"))
        })
}

/// Loads a forward server by its base URL.
///
/// Returns `None` when no server is registered under that URL.
pub async fn find_by_base_url(
    db: &DatabaseConnection,
    base_url: &str,
) -> Result<Option<yggdrasil_session_forward_server::Model>> {
    YggdrasilSessionForwardServer::find()
        .filter(yggdrasil_session_forward_server::Column::BaseUrl.eq(base_url))
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Loads the row representing this deployment itself (`Local` provider kind).
///
/// Returns `None` when the local server record has not been created yet.
pub async fn find_local(
    db: &DatabaseConnection,
) -> Result<Option<yggdrasil_session_forward_server::Model>> {
    YggdrasilSessionForwardServer::find()
        .filter(
            yggdrasil_session_forward_server::Column::ProviderKind
                .eq(YggdrasilSessionForwardProviderKind::Local),
        )
        .order_by_asc(yggdrasil_session_forward_server::Column::Id)
        .one(db)
        .await
        .map_err(AsterError::from)
}

/// Inserts a new upstream forward server row.
pub async fn create(
    db: &DatabaseConnection,
    model: yggdrasil_session_forward_server::ActiveModel,
) -> Result<yggdrasil_session_forward_server::Model> {
    model.insert(db).await.map_err(AsterError::from)
}

/// Persists changes on an existing forward server row.
pub async fn update(
    db: &DatabaseConnection,
    model: yggdrasil_session_forward_server::ActiveModel,
) -> Result<yggdrasil_session_forward_server::Model> {
    model.update(db).await.map_err(AsterError::from)
}

/// Deletes a forward server by id.
///
/// Returns `Err(record_not_found)` when no server with that id exists.
pub async fn delete(db: &DatabaseConnection, id: i64) -> Result<()> {
    let result = YggdrasilSessionForwardServer::delete_by_id(id)
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    if result.rows_affected == 0 {
        return Err(AsterError::record_not_found(format!(
            "Yggdrasil session forward server #{id}"
        )));
    }
    Ok(())
}

/// Records a successful upstream health check, clearing any previous failure message.
///
/// Returns `false` when the server row no longer exists.
pub async fn mark_success(
    db: &DatabaseConnection,
    id: i64,
    checked_at: chrono::DateTime<Utc>,
) -> Result<bool> {
    let result = YggdrasilSessionForwardServer::update_many()
        .col_expr(
            yggdrasil_session_forward_server::Column::LastCheckedAt,
            Expr::value(checked_at),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::LastSuccessAt,
            Expr::value(checked_at),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::LastFailureMessage,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::UpdatedAt,
            Expr::value(checked_at),
        )
        .filter(yggdrasil_session_forward_server::Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

/// Records a failed upstream health check with the failure message truncated to 512 chars.
///
/// Returns `false` when the server row no longer exists.
pub async fn mark_failure(
    db: &DatabaseConnection,
    id: i64,
    checked_at: chrono::DateTime<Utc>,
    message: &str,
) -> Result<bool> {
    let result = YggdrasilSessionForwardServer::update_many()
        .col_expr(
            yggdrasil_session_forward_server::Column::LastCheckedAt,
            Expr::value(checked_at),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::LastFailureAt,
            Expr::value(checked_at),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::LastFailureMessage,
            Expr::value(truncate_failure_message(message)),
        )
        .col_expr(
            yggdrasil_session_forward_server::Column::UpdatedAt,
            Expr::value(checked_at),
        )
        .filter(yggdrasil_session_forward_server::Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

fn truncate_failure_message(message: &str) -> String {
    const MAX_LEN: usize = 512;
    let trimmed = message.trim();
    if trimmed.len() <= MAX_LEN {
        return trimmed.to_string();
    }
    trimmed.chars().take(MAX_LEN).collect()
}
