//! Administrator Yggdrasil configuration API routes.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};

use crate::api::dto::{
    AdminYggdrasilSessionForwardServerListQuery, CreateYggdrasilSessionForwardServerReq,
    UpdateYggdrasilSessionForwardServerReq, validate_request,
};
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{
    audit_service, auth_service::AuthUserInfo, yggdrasil_session_forward_service,
};
use crate::types::YggdrasilSessionForwardServerSortBy;
#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::CursorPage;
use aster_forge_api::{parse_enabled_priority_id_cursor, parse_id_cursor};

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/yggdrasil/session-forward-servers",
    tag = "admin",
    operation_id = "admin_list_yggdrasil_session_forward_servers",
    params(
        ("limit" = Option<u64>, Query, description = "Maximum number of forwarding servers to return"),
        ("after_id" = Option<i64>, Query, description = "Cursor server ID"),
        ("after_enabled" = Option<bool>, Query, description = "Call-order cursor enabled value"),
        ("after_priority" = Option<i32>, Query, description = "Call-order cursor priority value"),
        ("sort_by" = Option<crate::types::YggdrasilSessionForwardServerSortBy>, Query, description = "Forwarding server list sort mode"),
    ),
    responses(
        (status = 200, description = "Yggdrasil session forwarding servers", body = inline(ApiResponse<CursorPage<yggdrasil_session_forward_service::AdminYggdrasilSessionForwardServerInfo, yggdrasil_session_forward_service::SessionForwardServerCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_session_forward_servers(
    state: web::Data<AppState>,
    page: web::Query<AdminYggdrasilSessionForwardServerListQuery>,
) -> Result<HttpResponse> {
    let sort_by = page.sort_by();
    let after_id = match sort_by {
        YggdrasilSessionForwardServerSortBy::Id => {
            parse_id_cursor(page.after_id, "Yggdrasil session forwarding server")?
        }
        YggdrasilSessionForwardServerSortBy::CallOrder => None,
    };
    let after_call_order = match sort_by {
        YggdrasilSessionForwardServerSortBy::CallOrder => parse_enabled_priority_id_cursor(
            page.after_enabled,
            page.after_priority,
            page.after_id,
            "Yggdrasil session forwarding server",
        )?,
        YggdrasilSessionForwardServerSortBy::Id => None,
    };
    let servers = yggdrasil_session_forward_service::list_servers(
        state.get_ref(),
        page.limit_or(50, 100),
        after_id,
        after_call_order,
        sort_by,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(servers)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/yggdrasil/session-forward-servers/{id}",
    tag = "admin",
    operation_id = "admin_get_yggdrasil_session_forward_server",
    params(("id" = i64, Path, description = "Yggdrasil session forwarding server ID")),
    responses(
        (status = 200, description = "Yggdrasil session forwarding server", body = inline(ApiResponse<yggdrasil_session_forward_service::AdminYggdrasilSessionForwardServerInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Yggdrasil session forwarding server not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_session_forward_server(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let server = yggdrasil_session_forward_service::get_server(state.get_ref(), *path).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(server)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/yggdrasil/session-forward-servers",
    tag = "admin",
    operation_id = "admin_create_yggdrasil_session_forward_server",
    request_body = CreateYggdrasilSessionForwardServerReq,
    responses(
        (status = 201, description = "Yggdrasil session forwarding server created", body = inline(ApiResponse<yggdrasil_session_forward_service::AdminYggdrasilSessionForwardServerInfo>)),
        (status = 400, description = "Invalid Yggdrasil session forwarding server"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn create_session_forward_server(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateYggdrasilSessionForwardServerReq>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    validate_request(&body)?;
    let server =
        yggdrasil_session_forward_service::create_server(state.get_ref(), body.into()).await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminCreateYggdrasilSessionForwardServer,
        audit_service::AuditEntityType::YggdrasilSession,
        Some(server.id),
        Some(&server.display_name),
        || yggdrasil_session_forward_service::server_audit_details(&server),
    )
    .await;
    Ok(HttpResponse::Created().json(ApiResponse::ok(server)))
}

#[aster_forge_api_docs_macros::path(
    patch,
    path = "/api/v1/admin/yggdrasil/session-forward-servers/{id}",
    tag = "admin",
    operation_id = "admin_update_yggdrasil_session_forward_server",
    params(("id" = i64, Path, description = "Yggdrasil session forwarding server ID")),
    request_body = UpdateYggdrasilSessionForwardServerReq,
    responses(
        (status = 200, description = "Yggdrasil session forwarding server updated", body = inline(ApiResponse<yggdrasil_session_forward_service::AdminYggdrasilSessionForwardServerInfo>)),
        (status = 400, description = "Invalid Yggdrasil session forwarding server"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Yggdrasil session forwarding server not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn update_session_forward_server(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateYggdrasilSessionForwardServerReq>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    validate_request(&body)?;
    let server =
        yggdrasil_session_forward_service::update_server(state.get_ref(), *path, body.into())
            .await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminUpdateYggdrasilSessionForwardServer,
        audit_service::AuditEntityType::YggdrasilSession,
        Some(server.id),
        Some(&server.display_name),
        || yggdrasil_session_forward_service::server_audit_details(&server),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(server)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/yggdrasil/session-forward-servers/{id}",
    tag = "admin",
    operation_id = "admin_delete_yggdrasil_session_forward_server",
    params(("id" = i64, Path, description = "Yggdrasil session forwarding server ID")),
    responses(
        (status = 200, description = "Yggdrasil session forwarding server deleted"),
        (status = 400, description = "Local Yggdrasil session forwarding server cannot be deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Yggdrasil session forwarding server not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_session_forward_server(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let server = yggdrasil_session_forward_service::delete_server(state.get_ref(), *path).await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminDeleteYggdrasilSessionForwardServer,
        audit_service::AuditEntityType::YggdrasilSession,
        Some(server.id),
        Some(&server.display_name),
        || yggdrasil_session_forward_service::server_audit_details(&server),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}
