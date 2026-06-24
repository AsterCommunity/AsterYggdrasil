//! Administrator user capability ban routes.

use actix_web::{HttpRequest, HttpResponse, web};

use crate::api::dto::{
    AdminUserBanListQuery, CreateUserBanReq, RevokeUserBanReq, UpdateUserBanReq, validate_request,
};
use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, ban_service};
use aster_forge_api::{LimitQuery, parse_datetime_id_cursor};

#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/user-bans",
    tag = "admin",
    operation_id = "admin_list_user_bans",
    params(LimitQuery, AdminUserBanListQuery),
    responses(
        (status = 200, description = "User capability bans", body = inline(ApiResponse<CursorPage<ban_service::UserBanInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_user_bans(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminUserBanListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let cursor = parse_datetime_id_cursor(query.after_created_at, query.after_id, "user ban")?;
    let bans = ban_service::list_user_bans(
        state.get_ref(),
        ban_service::ListUserBansInput {
            limit: page.limit_or(50, 100),
            cursor,
            user_id: query.user_id,
            scope: query.scope,
            status: query.status,
            effective_only: query.effective_only,
        },
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(bans)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/user-bans/{ban_id}",
    tag = "admin",
    operation_id = "admin_get_user_ban",
    params(("ban_id" = i64, Path, description = "User ban ID")),
    responses(
        (status = 200, description = "User capability ban detail", body = inline(ApiResponse<ban_service::UserBanInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User ban not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_user_ban(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let ban = ban_service::get_user_ban(state.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(ban)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/users/{user_id}/bans",
    tag = "admin",
    operation_id = "admin_create_user_ban",
    request_body = CreateUserBanReq,
    params(("user_id" = i64, Path, description = "Target user ID")),
    responses(
        (status = 200, description = "Created user capability ban", body = inline(ApiResponse<ban_service::UserBanInfo>)),
        (status = 400, description = "Invalid user ban request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Target user not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn create_user_ban(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<CreateUserBanReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let actor = auth_service::current_user(state.get_ref(), &req).await?;
    let ban = ban_service::create_user_ban(
        state.get_ref(),
        actor.id,
        ban_service::CreateUserBanInput {
            target_user_id: path.into_inner(),
            scopes: body.scopes.clone(),
            reason: body.reason.clone(),
            public_reason: body.public_reason.clone(),
            admin_note: body.admin_note.clone(),
            starts_at: body.starts_at,
            expires_at: body.expires_at,
        },
    )
    .await?;
    log_user_ban_audit(
        state.get_ref(),
        &req,
        actor.id,
        audit_service::AuditAction::AdminCreateUserBan,
        &ban,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(ban)))
}

#[aster_forge_api_docs_macros::path(
    patch,
    path = "/api/v1/admin/user-bans/{ban_id}",
    tag = "admin",
    operation_id = "admin_update_user_ban",
    request_body = UpdateUserBanReq,
    params(("ban_id" = i64, Path, description = "User ban ID")),
    responses(
        (status = 200, description = "Updated user capability ban", body = inline(ApiResponse<ban_service::UserBanInfo>)),
        (status = 400, description = "Invalid user ban request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User ban not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn update_user_ban(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateUserBanReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let actor = auth_service::current_user(state.get_ref(), &req).await?;
    let ban = ban_service::update_user_ban(
        state.get_ref(),
        actor.id,
        path.into_inner(),
        ban_service::UpdateUserBanInput {
            scopes: body.scopes.clone(),
            reason: body.reason.clone(),
            public_reason: body.public_reason.clone(),
            admin_note: body.admin_note.clone(),
            starts_at: body.starts_at,
            expires_at: body.expires_at,
        },
    )
    .await?;
    log_user_ban_audit(
        state.get_ref(),
        &req,
        actor.id,
        audit_service::AuditAction::AdminUpdateUserBan,
        &ban,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(ban)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/user-bans/{ban_id}/revoke",
    tag = "admin",
    operation_id = "admin_revoke_user_ban",
    request_body = RevokeUserBanReq,
    params(("ban_id" = i64, Path, description = "User ban ID")),
    responses(
        (status = 200, description = "Revoked user capability ban", body = inline(ApiResponse<ban_service::UserBanInfo>)),
        (status = 400, description = "Invalid user ban state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User ban not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn revoke_user_ban(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<RevokeUserBanReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let actor = auth_service::current_user(state.get_ref(), &req).await?;
    let ban = ban_service::revoke_user_ban(
        state.get_ref(),
        actor.id,
        path.into_inner(),
        body.revoke_note.clone(),
    )
    .await?;
    log_user_ban_audit(
        state.get_ref(),
        &req,
        actor.id,
        audit_service::AuditAction::AdminRevokeUserBan,
        &ban,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(ban)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/user-bans/{ban_id}/events",
    tag = "admin",
    operation_id = "admin_list_user_ban_events",
    params(("ban_id" = i64, Path, description = "User ban ID")),
    responses(
        (status = 200, description = "User ban events", body = inline(ApiResponse<Vec<ban_service::UserBanEventInfo>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User ban not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_user_ban_events(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let events = ban_service::list_user_ban_events(state.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(events)))
}

async fn log_user_ban_audit(
    state: &AppState,
    req: &HttpRequest,
    actor_user_id: i64,
    action: audit_service::AuditAction,
    ban: &ban_service::UserBanInfo,
) {
    let ctx = audit_service::AuditContext::from_request(req, actor_user_id);
    audit_service::log_with_details(
        state,
        &ctx,
        action,
        audit_service::AuditEntityType::UserBan,
        Some(ban.id),
        Some(&ban_scope_entity_name(&ban.scopes)),
        || {
            audit_service::details(audit_service::UserBanAuditDetails {
                target_user_id: ban.user_id,
                scopes: &ban.scopes,
                status: ban.status,
                effective_status: ban.effective_status,
                reason: &ban.reason,
                public_reason: ban.public_reason.as_deref(),
                admin_note: ban.admin_note.as_deref(),
                revoke_note: ban.revoke_note.as_deref(),
                starts_at: ban.starts_at,
                expires_at: ban.expires_at,
            })
        },
    )
    .await;
}

fn ban_scope_entity_name(scopes: &[crate::types::UserBanScope]) -> String {
    scopes
        .iter()
        .map(|scope| scope.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
