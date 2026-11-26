//! Administrator user management API routes.

use crate::api::dto::{
    AdminUserListQuery, CreateAdminUserReq, CreateUserInvitationReq, UpdateAdminUserReq,
    validate_request,
};
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth::local::AuthUserInfo;
use crate::services::ops::audit;
use crate::services::user;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use aster_forge_api::{CreatedAtCursorQuery, LimitQuery, parse_datetime_id_cursor};
#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor};
use aster_yggdrasil_model::types::user::{UserRole, UserStatus};

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

fn user_audit_details(user: &user::admin::AdminUserInfo) -> Option<serde_json::Value> {
    audit::details(audit::UserAuditDetails {
        username: &user.username,
        email: &user.email,
        role: user.role,
        status: user.status,
        must_change_password: user.must_change_password,
        temporary_password_generated: None,
        profile_count: user.profile_count,
        active_session_count: user.active_session_count,
    })
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/users",
    tag = "admin",
    operation_id = "admin_list_users",
    params(LimitQuery, AdminUserListQuery),
    responses(
        (status = 200, description = "Users", body = inline(ApiResponse<CursorPage<user::admin::AdminUserInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists platform users with keyword search and role/status filters, using
/// cursor pagination.
pub async fn list_users(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminUserListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let limit = page.limit_or(20, 100);
    let cursor = parse_datetime_id_cursor(query.after_created_at, query.after_id, "admin user")?;
    tracing::debug!(
        limit,
        has_keyword = query.keyword.is_some(),
        role = ?query.role,
        status = ?query.status,
        "admin listing users"
    );
    let users = user::admin::list_users(
        state.get_ref(),
        limit,
        user::admin::AdminUserListFilters {
            keyword: query.keyword.clone(),
            role: query.role,
            status: query.status,
        },
        cursor,
    )
    .await?;
    tracing::debug!(
        count = users.items.len(),
        total = users.total,
        "admin listed users"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(users)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/users",
    tag = "admin",
    operation_id = "admin_create_user",
    request_body = CreateAdminUserReq,
    responses(
        (status = 201, description = "User created", body = inline(ApiResponse<user::admin::CreateAdminUserOutput>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Creates a user account directly as an administrator, optionally generating
/// a temporary password, and records the creation in the audit log.
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
    let output = user::admin::create_user(
        state.get_ref(),
        user::admin::AdminCreateUserInput {
            username: body.username.clone(),
            email: body.email.clone(),
            password: body.password.clone(),
            role: body.role.unwrap_or(UserRole::User),
            operator_scopes: body.operator_scopes.clone(),
            status: body.status.unwrap_or(UserStatus::Active),
            must_change_password: body.must_change_password,
        },
    )
    .await?;
    let user = &output.user;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminCreateUser,
        audit::AuditEntityType::User,
        Some(user.id),
        Some(&user.username),
        || {
            let mut details = user_audit_details(user)?;
            if let Some(object) = details.as_object_mut() {
                object.insert(
                    "temporary_password_generated".to_string(),
                    serde_json::Value::Bool(output.generated_password.is_some()),
                );
            }
            Some(details)
        },
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        role = ?user.role,
        status = ?user.status,
        "admin created user"
    );
    Ok(HttpResponse::Created().json(ApiResponse::ok(output)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/users/invitations",
    tag = "admin",
    operation_id = "admin_create_user_invitation",
    request_body = CreateUserInvitationReq,
    responses(
        (status = 201, description = "User invitation created", body = inline(ApiResponse<crate::services::user::invitation::AdminUserInvitationInfo>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Creates a registration invitation for the given email, queues an
/// invitation mail, and records the action in the audit log.
pub async fn create_user_invitation(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateUserInvitationReq>,
) -> Result<HttpResponse> {
    let mut body = body.into_inner();
    body.email = body.email.trim().to_string();
    validate_request(&body)?;
    let admin_user_id = current_admin_user_id(&req)?;
    let invitation =
        user::invitation::create_invitation(state.get_ref(), &body.email, admin_user_id).await?;
    let ctx = audit::AuditContext::from_request(&req, admin_user_id);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminCreateInvitation,
        audit::AuditEntityType::Invitation,
        Some(invitation.id),
        Some(&invitation.email),
        || invitation_audit_details(&invitation),
    )
    .await;
    Ok(HttpResponse::Created().json(ApiResponse::ok(invitation)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/users/invitations",
    tag = "admin",
    operation_id = "admin_list_user_invitations",
    params(LimitQuery, CreatedAtCursorQuery),
    responses(
        (status = 200, description = "User invitations", body = inline(ApiResponse<CursorPage<crate::services::user::invitation::AdminUserInvitationInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists registration invitations with limit and datetime-id cursor
/// pagination.
pub async fn list_user_invitations(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    cursor_query: web::Query<CreatedAtCursorQuery>,
) -> Result<HttpResponse> {
    let cursor = parse_datetime_id_cursor(
        cursor_query.after_created_at,
        cursor_query.after_id,
        "user invitation",
    )?;
    let invitations =
        user::invitation::list_invitations(state.get_ref(), page.limit_or(20, 100), cursor).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(invitations)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/users/invitations/{id}/revoke",
    tag = "admin",
    operation_id = "admin_revoke_user_invitation",
    params(("id" = i64, Path, description = "Invitation ID")),
    responses(
        (status = 200, description = "User invitation revoked", body = inline(ApiResponse<crate::services::user::invitation::AdminUserInvitationInfo>)),
        (status = 400, description = "Invitation cannot be revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Invitation not found"),
    ),
    security(("bearer" = [])),
)]
/// Revokes a pending registration invitation so it can no longer be accepted,
/// and records the revocation in the audit log.
pub async fn revoke_user_invitation(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let invitation = user::invitation::revoke_invitation(state.get_ref(), *path).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminRevokeInvitation,
        audit::AuditEntityType::Invitation,
        Some(invitation.id),
        Some(&invitation.email),
        || invitation_audit_details(&invitation),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(invitation)))
}

fn invitation_audit_details(
    invitation: &user::invitation::AdminUserInvitationInfo,
) -> Option<serde_json::Value> {
    audit::details(serde_json::json!({
        "email": invitation.email,
        "status": invitation.status,
        "invited_by": invitation.invited_by,
        "accepted_user_id": invitation.accepted_user_id,
        "expires_at": invitation.expires_at,
        "mail_queued": invitation.mail_queued,
    }))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/users/{id}",
    tag = "admin",
    operation_id = "admin_get_user",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User", body = inline(ApiResponse<user::admin::AdminUserInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
/// Returns a single user's administrative profile, including role, status,
/// operator scopes, profile count, and active session count.
pub async fn get_user(state: web::Data<AppState>, path: web::Path<i64>) -> Result<HttpResponse> {
    let user_id = *path;
    tracing::debug!(user_id, "admin loading user");
    let user = user::admin::get_user(state.get_ref(), user_id).await?;
    tracing::debug!(
        user_id = user.id,
        role = ?user.role,
        status = ?user.status,
        "admin loaded user"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(user)))
}

#[aster_forge_api_docs_macros::path(
    patch,
    path = "/api/v1/admin/users/{id}",
    tag = "admin",
    operation_id = "admin_update_user",
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateAdminUserReq,
    responses(
        (status = 200, description = "User updated", body = inline(ApiResponse<user::admin::AdminUserInfo>)),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
/// Updates a user's account fields (username, email, password, role, operator
/// scopes, status); disabling the user is audited as a dedicated action.
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
        must_change_password_changed = body.must_change_password.is_some(),
        "admin updating user"
    );
    let user = user::admin::update_user(
        state.get_ref(),
        user_id,
        user::admin::AdminUpdateUserInput {
            username: body.username.clone(),
            email: body.email.clone(),
            password: body.password.clone(),
            role: body.role,
            operator_scopes: body.operator_scopes.clone(),
            status: body.status,
            must_change_password: body.must_change_password,
        },
    )
    .await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    let action = if body.status == Some(UserStatus::Disabled) {
        audit::AuditAction::AdminDisableUser
    } else {
        audit::AuditAction::AdminUpdateUser
    };
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        action,
        audit::AuditEntityType::User,
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

#[aster_forge_api_docs_macros::path(
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
/// Revokes all active sessions of a user, forcing re-login everywhere, and
/// records the revocation in the audit log.
pub async fn revoke_user_sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let user_id = *path;
    tracing::debug!(user_id, "admin revoking user sessions");
    let removed = user::admin::revoke_user_sessions(state.get_ref(), user_id).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminRevokeUserSessions,
        audit::AuditEntityType::User,
        Some(user_id),
        Some(&format!("user #{user_id}")),
        || audit::details(audit::UserSessionRevokeAuditDetails { removed }),
    )
    .await;
    tracing::debug!(user_id, removed, "admin revoked user sessions");
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(crate::api::dto::RemovedCountResponse {
            removed,
        })),
    )
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/users/{id}",
    tag = "admin",
    operation_id = "admin_delete_user",
    params(("id" = i64, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted", body = inline(ApiResponse<user::admin::DeleteAdminUserOutput>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
/// Deletes a user along with their Minecraft profiles, textures, sessions,
/// and Yggdrasil tokens, reporting per-category removal counts.
pub async fn delete_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let user_id = *path;
    tracing::debug!(user_id, "admin deleting user");
    let output = user::admin::delete_user(state.get_ref(), user_id).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminDeleteUser,
        audit::AuditEntityType::User,
        Some(output.user.id),
        Some(&output.user.username),
        || user_audit_details(&output.user),
    )
    .await;
    tracing::debug!(
        user_id = output.user.id,
        deleted_profile_count = output.deleted_profile_count,
        deleted_profile_texture_count = output.deleted_profile_texture_count,
        deleted_wardrobe_texture_count = output.deleted_wardrobe_texture_count,
        revoked_session_count = output.revoked_session_count,
        revoked_yggdrasil_token_count = output.revoked_yggdrasil_token_count,
        "admin deleted user"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(output)))
}
