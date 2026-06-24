//! Administrator audit log API routes.

#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::pagination::{CursorPage, DateTimeIdCursor};
use crate::api::pagination::{LimitQuery, parse_datetime_id_cursor};
use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::audit_service;
use actix_web::{HttpResponse, web};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/audit-logs",
    tag = "admin",
    operation_id = "list_audit_logs",
    params(LimitQuery, audit_service::AuditLogFilterQuery),
    responses(
        (status = 200, description = "Audit log entries", body = inline(ApiResponse<CursorPage<audit_service::AuditLogEntry, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_audit_logs(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<audit_service::AuditLogFilterQuery>,
) -> Result<HttpResponse> {
    let filters = audit_service::AuditLogFilters::from_query(&query);
    let cursor = parse_datetime_id_cursor(query.after_created_at, query.after_id, "audit log")?;
    let page =
        audit_service::query(state.get_ref(), filters, page.limit_or(50, 200), cursor).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}
