//! Repository for upstream Yggdrasil session server forwarding.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    sea_query::Expr,
};

use crate::db::repository::pagination_repo::fetch_offset_page;
use crate::entities::yggdrasil_session_forward_server::{
    self, Entity as YggdrasilSessionForwardServer,
};
use crate::errors::{AsterError, Result};
use crate::types::{YggdrasilSessionForwardProviderKind, YggdrasilSessionForwardServerSortBy};

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

pub async fn find_paginated(
    db: &DatabaseConnection,
    limit: u64,
    offset: u64,
    sort_by: YggdrasilSessionForwardServerSortBy,
) -> Result<(Vec<yggdrasil_session_forward_server::Model>, u64)> {
    let select = match sort_by {
        YggdrasilSessionForwardServerSortBy::CallOrder => YggdrasilSessionForwardServer::find()
            .order_by_desc(yggdrasil_session_forward_server::Column::Enabled)
            .order_by_asc(yggdrasil_session_forward_server::Column::Priority)
            .order_by_asc(yggdrasil_session_forward_server::Column::Id),
        YggdrasilSessionForwardServerSortBy::Id => YggdrasilSessionForwardServer::find()
            .order_by_asc(yggdrasil_session_forward_server::Column::Id),
    };
    fetch_offset_page(db, select, limit, offset).await
}

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

pub async fn create(
    db: &DatabaseConnection,
    model: yggdrasil_session_forward_server::ActiveModel,
) -> Result<yggdrasil_session_forward_server::Model> {
    model.insert(db).await.map_err(AsterError::from)
}

pub async fn update(
    db: &DatabaseConnection,
    model: yggdrasil_session_forward_server::ActiveModel,
) -> Result<yggdrasil_session_forward_server::Model> {
    model.update(db).await.map_err(AsterError::from)
}

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
