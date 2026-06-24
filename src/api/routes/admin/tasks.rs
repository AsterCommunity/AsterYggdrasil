//! Administrator background task API routes.

use crate::api::dto::{
    AdminTaskCleanupReq, AdminTaskListQuery, RemovedCountResponse, validate_request,
};
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth_service::AuthUserInfo;
use crate::services::{audit_service, task_service};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor};
use aster_forge_api::{LimitQuery, parse_datetime_id_cursor};

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/tasks",
    tag = "admin",
    operation_id = "admin_list_tasks",
    params(LimitQuery, AdminTaskListQuery),
    responses(
        (status = 200, description = "Background tasks", body = inline(ApiResponse<CursorPage<task_service::types::TaskInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_tasks(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminTaskListQuery>,
) -> Result<HttpResponse> {
    let limit = page.limit_or(20, 100);
    let cursor = parse_datetime_id_cursor(query.after_updated_at, query.after_id, "admin task")?;
    tracing::debug!(
        limit,
        kind = ?query.kind,
        status = ?query.status,
        "admin listing tasks"
    );
    let page = task_service::list_tasks_paginated_for_admin(
        state.get_ref(),
        limit,
        task_service::AdminTaskListFilters {
            kind: query.kind,
            status: query.status,
        },
        cursor,
    )
    .await?;
    tracing::debug!(
        count = page.items.len(),
        total = page.total,
        "admin listed tasks"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/tasks/{id}/retry",
    tag = "admin",
    operation_id = "admin_retry_task",
    params(("id" = i64, Path, description = "Background task ID")),
    responses(
        (status = 200, description = "Task reset for retry", body = inline(ApiResponse<task_service::types::TaskInfo>)),
        (status = 400, description = "Task cannot be retried"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn retry_task(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let task_id = *path;
    tracing::debug!(task_id, "admin retry task request received");
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    let task =
        task_service::retry_task_for_admin_with_audit(state.get_ref(), task_id, &ctx).await?;
    tracing::debug!(
        task_id = task.id,
        kind = ?task.kind,
        status = ?task.status,
        "admin retry task request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(task)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/tasks/cleanup",
    tag = "admin",
    operation_id = "admin_cleanup_tasks",
    request_body = AdminTaskCleanupReq,
    responses(
        (status = 200, description = "Completed tasks cleaned up", body = inline(ApiResponse<RemovedCountResponse>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn cleanup_tasks(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<AdminTaskCleanupReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    tracing::debug!(
        finished_before = %body.finished_before,
        kind = ?body.kind,
        status = ?body.status,
        "admin cleanup tasks request received"
    );
    let removed = task_service::cleanup_tasks_for_admin(
        state.get_ref(),
        task_service::AdminTaskCleanupFilters {
            finished_before: body.finished_before,
            kind: body.kind,
            status: body.status,
        },
    )
    .await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminCleanupTasks,
        audit_service::AuditEntityType::Task,
        None,
        None,
        || {
            audit_service::details(audit_service::AdminTaskCleanupAuditDetails {
                removed,
                finished_before: body.finished_before,
                kind: body.kind,
                status: body.status,
            })
        },
    )
    .await;
    tracing::debug!(removed, "admin cleanup tasks request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(RemovedCountResponse { removed })))
}
