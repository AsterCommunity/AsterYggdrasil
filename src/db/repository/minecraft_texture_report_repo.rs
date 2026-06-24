//! Repository helpers for public texture library reports.

use crate::entities::minecraft_texture_report::{self, Entity as MinecraftTextureReport};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::{MinecraftTextureReportReason, MinecraftTextureReportStatus};
use aster_forge_api::CursorSlice;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

#[derive(Debug, Clone)]
pub struct CreateTextureReport {
    pub texture_id: i64,
    pub reporter_user_id: i64,
    pub reason: MinecraftTextureReportReason,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdminTextureReportListFilter {
    pub status: Option<MinecraftTextureReportStatus>,
    pub reason: Option<MinecraftTextureReportReason>,
    pub texture_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct HandleTextureReport {
    pub status: MinecraftTextureReportStatus,
    pub admin_note: Option<String>,
    pub handled_by_user_id: i64,
    pub handled_at: DateTime<Utc>,
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateTextureReport,
) -> Result<minecraft_texture_report::Model> {
    let now = Utc::now();
    minecraft_texture_report::ActiveModel {
        texture_id: Set(input.texture_id),
        reporter_user_id: Set(input.reporter_user_id),
        reason: Set(input.reason),
        message: Set(input.message),
        status: Set(MinecraftTextureReportStatus::Pending),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_id<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<minecraft_texture_report::Model>> {
    MinecraftTextureReport::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_pending_for_reporter_and_texture<C: ConnectionTrait>(
    db: &C,
    reporter_user_id: i64,
    texture_id: i64,
) -> Result<Option<minecraft_texture_report::Model>> {
    MinecraftTextureReport::find()
        .filter(minecraft_texture_report::Column::ReporterUserId.eq(reporter_user_id))
        .filter(minecraft_texture_report::Column::TextureId.eq(texture_id))
        .filter(minecraft_texture_report::Column::Status.eq(MinecraftTextureReportStatus::Pending))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    filter: AdminTextureReportListFilter,
    after: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorSlice<minecraft_texture_report::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = filtered_query(filter);
    let total = query
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    if let Some((created_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(minecraft_texture_report::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(minecraft_texture_report::Column::CreatedAt.eq(created_at))
                        .add(minecraft_texture_report::Column::Id.lt(id)),
                ),
        );
    }

    let items = query
        .order_by_desc(minecraft_texture_report::Column::CreatedAt)
        .order_by_desc(minecraft_texture_report::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

pub async fn handle<C: ConnectionTrait>(
    db: &C,
    report: minecraft_texture_report::Model,
    input: HandleTextureReport,
) -> Result<minecraft_texture_report::Model> {
    let mut active: minecraft_texture_report::ActiveModel = report.into();
    active.status = Set(input.status);
    active.admin_note = Set(input.admin_note);
    active.handled_by_user_id = Set(Some(input.handled_by_user_id));
    active.handled_at = Set(Some(input.handled_at));
    active.updated_at = Set(Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn handle_pending_for_texture<C: ConnectionTrait>(
    db: &C,
    texture_id: i64,
    input: HandleTextureReport,
) -> Result<Vec<minecraft_texture_report::Model>> {
    let reports = MinecraftTextureReport::find()
        .filter(minecraft_texture_report::Column::TextureId.eq(texture_id))
        .filter(minecraft_texture_report::Column::Status.eq(MinecraftTextureReportStatus::Pending))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    let mut updated = Vec::with_capacity(reports.len());
    let now = Utc::now();
    for report in reports {
        let mut active = report.into_active_model();
        active.status = Set(input.status);
        active.admin_note = Set(input.admin_note.clone());
        active.handled_by_user_id = Set(Some(input.handled_by_user_id));
        active.handled_at = Set(Some(input.handled_at));
        active.updated_at = Set(now);
        updated.push(
            active
                .update(db)
                .await
                .map_aster_err(AsterError::database_operation)?,
        );
    }
    Ok(updated)
}

fn filtered_query(
    filter: AdminTextureReportListFilter,
) -> sea_orm::Select<minecraft_texture_report::Entity> {
    let mut query = MinecraftTextureReport::find();

    if let Some(status) = filter.status {
        query = query.filter(minecraft_texture_report::Column::Status.eq(status));
    }
    if let Some(reason) = filter.reason {
        query = query.filter(minecraft_texture_report::Column::Reason.eq(reason));
    }
    if let Some(texture_id) = filter.texture_id {
        query = query.filter(minecraft_texture_report::Column::TextureId.eq(texture_id));
    }

    query
}
