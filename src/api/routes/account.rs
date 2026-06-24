//! Current-user account routes.

use crate::api::dto::{
    AccountAuditLogFilterQuery, AccountOverviewResp, AccountUserBanInfo, AccountUserBanListQuery,
};
#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::pagination::{CursorPage, DateTimeIdCursor};
use crate::api::pagination::{LimitQuery, parse_datetime_id_cursor};
use crate::api::response::ApiResponse;
use crate::db::repository::user_repo;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, ban_service};
use actix_web::{HttpRequest, HttpResponse, web};

const ACCOUNT_OVERVIEW_ACTIVITY_LIMIT: u64 = 5;
const ACCOUNT_AUDIT_LOG_DEFAULT_LIMIT: u64 = 30;
const ACCOUNT_AUDIT_LOG_MAX_LIMIT: u64 = 100;
const ACCOUNT_USER_BAN_DEFAULT_LIMIT: u64 = 20;
const ACCOUNT_USER_BAN_MAX_LIMIT: u64 = 100;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/account")
            .route("/overview", web::get().to(overview))
            .route("/audit-logs", web::get().to(list_audit_logs))
            .route("/bans", web::get().to(list_user_bans)),
    );
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/account/overview",
    tag = "account",
    operation_id = "get_account_overview",
    responses(
        (status = 200, description = "Current account overview", body = inline(ApiResponse<AccountOverviewResp>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn overview(state: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(user_id = user.id, "account overview request received");
    let profile_count = user_repo::count_profiles_by_user_ids(state.reader_db(), &[user.id])
        .await?
        .get(&user.id)
        .copied()
        .unwrap_or(0);
    let recent_activity = query_current_user_audit_logs(
        state.get_ref(),
        user.id,
        audit_service::AuditLogFilters {
            user_id: Some(user.id),
            action: None,
            entity_type: None,
            entity_id: None,
            after: None,
            before: None,
        },
        ACCOUNT_OVERVIEW_ACTIVITY_LIMIT,
        None,
    )
    .await?
    .items;
    tracing::debug!(
        user_id = user.id,
        activity_count = recent_activity.len(),
        "account overview request completed"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(AccountOverviewResp {
            profile_count,
            recent_activity,
        })),
    )
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/account/audit-logs",
    tag = "account",
    operation_id = "list_account_audit_logs",
    params(LimitQuery, AccountAuditLogFilterQuery),
    responses(
        (status = 200, description = "Current user's audit log entries", body = inline(ApiResponse<CursorPage<audit_service::AuditLogEntry, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_audit_logs(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitQuery>,
    query: web::Query<AccountAuditLogFilterQuery>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(user_id = user.id, "account audit log list request received");
    let query = query.into_inner();
    let cursor =
        parse_datetime_id_cursor(query.after_created_at, query.after_id, "account audit log")?;
    let filters = query.into_filters_for_user(user.id);
    let page = query_current_user_audit_logs(
        state.get_ref(),
        user.id,
        filters,
        page.limit_or(ACCOUNT_AUDIT_LOG_DEFAULT_LIMIT, ACCOUNT_AUDIT_LOG_MAX_LIMIT),
        cursor,
    )
    .await?;
    tracing::debug!(
        user_id = user.id,
        count = page.items.len(),
        total = page.total,
        "account audit log list request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/account/bans",
    tag = "account",
    operation_id = "list_account_user_bans",
    params(LimitQuery, AccountUserBanListQuery),
    responses(
        (status = 200, description = "Current user's capability bans", body = inline(ApiResponse<CursorPage<AccountUserBanInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_user_bans(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitQuery>,
    query: web::Query<AccountUserBanListQuery>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(user_id = user.id, "account user ban list request received");
    let query = query.into_inner();
    let cursor = parse_datetime_id_cursor(query.after_created_at, query.after_id, "account ban")?;
    let page = ban_service::list_user_bans(
        state.get_ref(),
        ban_service::ListUserBansInput {
            limit: page.limit_or(ACCOUNT_USER_BAN_DEFAULT_LIMIT, ACCOUNT_USER_BAN_MAX_LIMIT),
            cursor,
            user_id: Some(user.id),
            scope: query.scope,
            status: query.status,
            effective_only: query.effective_only.unwrap_or(false),
        },
    )
    .await?;
    let items = page
        .items
        .into_iter()
        .map(AccountUserBanInfo::from)
        .collect::<Vec<_>>();
    let page =
        crate::api::pagination::CursorPage::new(items, page.total, page.limit, page.next_cursor);
    tracing::debug!(
        user_id = user.id,
        count = page.items.len(),
        total = page.total,
        "account user ban list request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

async fn query_current_user_audit_logs(
    state: &AppState,
    user_id: i64,
    mut filters: audit_service::AuditLogFilters,
    limit: u64,
    cursor: Option<(chrono::DateTime<chrono::Utc>, i64)>,
) -> Result<
    crate::api::pagination::CursorPage<
        audit_service::AuditLogEntry,
        crate::api::pagination::DateTimeIdCursor,
    >,
> {
    // Keep the current-user boundary server-side even if a future query type grows fields.
    filters.user_id = Some(user_id);
    audit_service::query(state, filters, limit, cursor).await
}
