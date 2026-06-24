use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, sea_query::Expr,
};

use super::common::{AdminTaskFilters, active_processing_by_kinds_condition, apply_admin_filters};
use crate::entities::background_task::{self, Entity as BackgroundTask};
use crate::errors::{AsterError, Result};
use crate::types::{BackgroundTaskKind, BackgroundTaskStatus, StoredTaskPayload};
use aster_forge_api::CursorSlice;

pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<background_task::Model> {
    BackgroundTask::find_by_id(id)
        .one(db)
        .await
        .map_err(AsterError::from)?
        .ok_or_else(|| AsterError::record_not_found(format!("task #{id}")))
}

pub async fn find_cursor_filtered<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    filters: &AdminTaskFilters,
    after: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorSlice<background_task::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = apply_admin_filters(BackgroundTask::find(), filters);
    let total = query.clone().count(db).await.map_err(AsterError::from)?;
    if let Some((updated_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(background_task::Column::UpdatedAt.lt(updated_at))
                .add(
                    Condition::all()
                        .add(background_task::Column::UpdatedAt.eq(updated_at))
                        .add(background_task::Column::Id.lt(id)),
                ),
        );
    }
    let items = query
        .order_by_desc(background_task::Column::UpdatedAt)
        .order_by_desc(background_task::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_err(AsterError::from)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

pub async fn list_recent<C: ConnectionTrait>(
    db: &C,
    limit: u64,
) -> Result<Vec<background_task::Model>> {
    BackgroundTask::find()
        .order_by_desc(background_task::Column::UpdatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(AsterError::from)
}

pub async fn find_latest_system_runtime_by_payload<C: ConnectionTrait>(
    db: &C,
    payload_json: &StoredTaskPayload,
) -> Result<Option<background_task::Model>> {
    BackgroundTask::find()
        .filter(background_task::Column::Kind.eq(BackgroundTaskKind::SystemRuntime))
        .filter(background_task::Column::PayloadJson.eq(payload_json.clone()))
        .order_by_desc(background_task::Column::UpdatedAt)
        .one(db)
        .await
        .map_err(AsterError::from)
}

pub async fn count_processing<C: ConnectionTrait>(db: &C) -> Result<u64> {
    BackgroundTask::find()
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .count(db)
        .await
        .map_err(AsterError::from)
}

pub async fn count_pending_or_retry<C: ConnectionTrait>(db: &C) -> Result<u64> {
    BackgroundTask::find()
        .filter(
            background_task::Column::Status
                .is_in([BackgroundTaskStatus::Pending, BackgroundTaskStatus::Retry]),
        )
        .count(db)
        .await
        .map_err(AsterError::from)
}

pub async fn count_active_processing_by_kinds<C: ConnectionTrait>(
    db: &C,
    now: DateTime<Utc>,
    kinds: &[BackgroundTaskKind],
) -> Result<u64> {
    if kinds.is_empty() {
        return Ok(0);
    }

    let count = BackgroundTask::find()
        .select_only()
        .column_as(
            Expr::col(background_task::Column::Id).count(),
            "active_processing_count",
        )
        .filter(active_processing_by_kinds_condition(now, kinds))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(AsterError::from)?
        .unwrap_or(0);

    crate::utils::numbers::i64_to_u64(count, "active processing task count")
}

pub async fn find_latest_by_kind_and_display_name<C: ConnectionTrait>(
    db: &C,
    kind: BackgroundTaskKind,
    display_name: &str,
) -> Result<Option<background_task::Model>> {
    BackgroundTask::find()
        .filter(background_task::Column::Kind.eq(kind))
        .filter(background_task::Column::DisplayName.eq(display_name))
        .order_by_desc(background_task::Column::CreatedAt)
        .one(db)
        .await
        .map_err(AsterError::from)
}
