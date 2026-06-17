//! Administrator user management API routes.

use crate::api::cache::conditional_bytes_response;
use crate::api::dto::{
    AdminUserListQuery, CreateAdminUserReq, UpdateAdminUserReq, validate_request,
};
use crate::api::pagination::LimitOffsetQuery;
#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::pagination::OffsetPage;
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::audit_service;
use crate::services::auth_service::{self, AuthUserInfo};
use crate::services::profile_service;
use crate::types::{UserRole, UserStatus};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

fn user_audit_details(user: &auth_service::AdminUserInfo) -> Option<serde_json::Value> {
    audit_service::details(audit_service::UserAuditDetails {
        username: &user.username,
        email: &user.email,
        role: user.role,
        status: user.status,
        profile_count: user.profile_count,
        active_session_count: user.active_session_count,
    })
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/admin/users",
    tag = "admin",
    operation_id = "admin_list_users",
    params(LimitOffsetQuery, AdminUserListQuery),
    responses(
        (status = 200, description = "Users", body = inline(ApiResponse<OffsetPage<auth_service::AdminUserInfo>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_users(
    state: web::Data<AppState>,
    page: web::Query<LimitOffsetQuery>,
    query: web::Query<AdminUserListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let limit = page.limit_or(20, 100);
    let offset = page.offset();
    tracing::debug!(
        limit,
        offset,
        has_keyword = query.keyword.is_some(),
        role = ?query.role,
        status = ?query.status,
        "admin listing users"
    );
    let users = auth_service::list_admin_users(
        state.get_ref(),
        limit,
        offset,
        auth_service::AdminUserListFilters {
            keyword: query.keyword.clone(),
            role: query.role,
            status: query.status,
        },
        query.sort_by(),
        query.sort_order(),
    )
    .await?;
    tracing::debug!(
        count = users.items.len(),
        total = users.total,
        "admin listed users"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(users)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/admin/users",
    tag = "admin",
    operation_id = "admin_create_user",
    request_body = CreateAdminUserReq,
    responses(
        (status = 201, description = "User created", body = inline(ApiResponse<auth_service::AdminUserInfo>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn create_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateAdminUserReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    tracing::debug!(
        username_len = body.username.len(),
        role = ?body.role.unwrap_or(UserRole::User),
        status = ?body.status.unwrap_or(UserStatus::Active),
        "admin creating user"
    );
    let user = auth_service::create_admin_user(
        state.get_ref(),
        &body.username,
        &body.email,
        &body.password,
        body.role.unwrap_or(UserRole::User),
        body.status.unwrap_or(UserStatus::Active),
    )
    .await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminCreateUser,
        audit_service::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        || user_audit_details(&user),
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        role = ?user.role,
        status = ?user.status,
        "admin created user"
    );
    Ok(HttpResponse::Created().json(ApiResponse::ok(user)))
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/admin/users/{id}",
    tag = "admin",
    operation_id = "admin_get_user",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User", body = inline(ApiResponse<auth_service::AdminUserInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_user(state: web::Data<AppState>, path: web::Path<i64>) -> Result<HttpResponse> {
    let user_id = *path;
    tracing::debug!(user_id, "admin loading user");
    let user = auth_service::get_admin_user(state.get_ref(), user_id).await?;
    tracing::debug!(
        user_id = user.id,
        role = ?user.role,
        status = ?user.status,
        "admin loaded user"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(user)))
}

#[api_docs_macros::path(
    patch,
    path = "/api/v1/admin/users/{id}",
    tag = "admin",
    operation_id = "admin_update_user",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateAdminUserReq,
    responses(
        (status = 200, description = "User updated", body = inline(ApiResponse<auth_service::AdminUserInfo>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn update_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateAdminUserReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let user_id = *path;
    tracing::debug!(
        user_id,
        username_changed = body.username.is_some(),
        email_changed = body.email.is_some(),
        password_changed = body.password.is_some(),
        role_changed = body.role.is_some(),
        status_changed = body.status.is_some(),
        "admin updating user"
    );
    let user = auth_service::update_admin_user(
        state.get_ref(),
        user_id,
        body.username.clone(),
        body.email.clone(),
        body.password.clone(),
        body.role,
        body.status,
    )
    .await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    let action = if body.status == Some(UserStatus::Disabled) {
        audit_service::AuditAction::AdminDisableUser
    } else {
        audit_service::AuditAction::AdminUpdateUser
    };
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        action,
        audit_service::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        || user_audit_details(&user),
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        role = ?user.role,
        status = ?user.status,
        "admin updated user"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(user)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/admin/users/{id}/sessions/revoke",
    tag = "admin",
    operation_id = "admin_revoke_user_sessions",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User sessions revoked", body = inline(ApiResponse<crate::api::dto::RemovedCountResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn revoke_user_sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let user_id = *path;
    tracing::debug!(user_id, "admin revoking user sessions");
    let removed = auth_service::revoke_admin_user_sessions(state.get_ref(), user_id).await?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::AdminRevokeUserSessions,
        audit_service::AuditEntityType::User,
        Some(user_id),
        Some(&format!("user #{user_id}")),
        || audit_service::details(audit_service::UserSessionRevokeAuditDetails { removed }),
    )
    .await;
    tracing::debug!(user_id, removed, "admin revoked user sessions");
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(crate::api::dto::RemovedCountResponse {
            removed,
        })),
    )
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/admin/users/{id}/avatar/{size}",
    tag = "admin",
    operation_id = "admin_get_user_avatar",
    params(
        ("id" = i64, Path, description = "User ID"),
        ("size" = u32, Path, description = "Avatar size (512 or 1024)")
    ),
    responses(
        (status = 200, description = "Avatar image (WebP)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Avatar not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_user_avatar(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(i64, u32)>,
) -> Result<HttpResponse> {
    let (user_id, size) = path.into_inner();
    tracing::debug!(user_id, size, "admin loading user avatar");
    let bytes = profile_service::get_avatar_bytes(state.get_ref(), user_id, size).await?;
    tracing::debug!(
        user_id,
        size,
        bytes = bytes.len(),
        "admin loaded user avatar"
    );
    Ok(conditional_bytes_response(
        &req,
        bytes,
        profile_service::AVATAR_CONTENT_TYPE,
        profile_service::AVATAR_CACHE_CONTROL,
    ))
}
