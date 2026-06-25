use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveEnum, ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder,
    QuerySelect, Select, sea_query::Expr,
};

use super::common::claimable_condition;
use crate::entities::background_task::{self, Entity as BackgroundTask};
use crate::errors::{AsterError, Result};
use crate::types::{BackgroundTaskKind, BackgroundTaskStatus};

pub async fn list_claimable<C: ConnectionTrait>(
    db: &C,
    now: DateTime<Utc>,
    stale_before: DateTime<Utc>,
    limit: u64,
) -> Result<Vec<background_task::Model>> {
    list_claimable_query(now, stale_before, limit)
        .all(db)
        .await
        .map_err(AsterError::from)
}

pub async fn list_claimable_by_kinds<C: ConnectionTrait>(
    db: &C,
    now: DateTime<Utc>,
    stale_before: DateTime<Utc>,
    kinds: &[BackgroundTaskKind],
    limit: u64,
) -> Result<Vec<background_task::Model>> {
    if kinds.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    list_claimable_query(now, stale_before, limit)
        .filter(background_task::Column::Kind.is_in(kinds.iter().copied()))
        .all(db)
        .await
        .map_err(AsterError::from)
}

pub async fn try_claim<C: ConnectionTrait>(
    db: &C,
    id: i64,
    expected_processing_token: i64,
    now: DateTime<Utc>,
    stale_before: DateTime<Utc>,
    next_processing_token: i64,
    lease_expires_at: DateTime<Utc>,
) -> Result<bool> {
    // try_claim is a compare-and-swap update. It moves the task to Processing
    // only when the id matches, the old processing_token still matches, and the
    // task is still claimable at this moment. The token is advanced atomically to
    // next_processing_token.
    //
    // When multiple dispatchers see the same task concurrently, only one can
    // claim it successfully.
    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::Status,
            Expr::value(BackgroundTaskStatus::Processing.to_value()),
        )
        .col_expr(
            background_task::Column::ProcessingStartedAt,
            Expr::value(Some(now)),
        )
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Some(now)),
        )
        .col_expr(
            background_task::Column::ProcessingToken,
            Expr::value(next_processing_token),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Some(lease_expires_at)),
        )
        .col_expr(
            background_task::Column::StartedAt,
            Expr::col(background_task::Column::StartedAt).if_null(now),
        )
        .col_expr(background_task::Column::UpdatedAt, Expr::value(now))
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::ProcessingToken.eq(expected_processing_token))
        .filter(claimable_condition(now, stale_before))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

pub async fn touch_heartbeat<C: ConnectionTrait>(
    db: &C,
    id: i64,
    processing_token: i64,
    now: DateTime<Utc>,
    lease_expires_at: DateTime<Utc>,
) -> Result<bool> {
    // Heartbeat updates also carry the processing token condition. A false
    // result means the task row still exists, but this worker's lease is no
    // longer current.
    let result = BackgroundTask::update_many()
        .col_expr(
            background_task::Column::LastHeartbeatAt,
            Expr::value(Some(now)),
        )
        .col_expr(
            background_task::Column::LeaseExpiresAt,
            Expr::value(Some(lease_expires_at)),
        )
        .filter(background_task::Column::Id.eq(id))
        .filter(background_task::Column::Status.eq(BackgroundTaskStatus::Processing))
        .filter(background_task::Column::ProcessingToken.eq(processing_token))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected == 1)
}

fn list_claimable_query(
    now: DateTime<Utc>,
    stale_before: DateTime<Utc>,
    limit: u64,
) -> Select<BackgroundTask> {
    BackgroundTask::find()
        .filter(claimable_condition(now, stale_before))
        .order_by_asc(background_task::Column::CreatedAt)
        .order_by_asc(background_task::Column::Id)
        .limit(limit)
}
