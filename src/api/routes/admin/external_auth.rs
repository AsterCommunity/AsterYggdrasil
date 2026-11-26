//! Administrator external auth API routes.

use crate::api::dto::{
    CreateExternalAuthProviderReq, ExternalAuthProviderTestParamsReq,
    UpdateExternalAuthProviderReq, validate_request,
};
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth::external::{
    self, AdminExternalAuthProviderInfo, ExternalAuthProviderAuditDetails,
};
use crate::services::auth::local::AuthUserInfo;
use crate::services::ops::audit;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, StringIdCursor};
use aster_forge_api::{LimitQuery, parse_string_id_cursor};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Cursor pagination query for external auth provider listing, addressing
/// entries after the given display name and ID.
pub struct AdminExternalAuthProviderCursorQuery {
    /// Display name to start after for the next page.
    pub after_display_name: Option<String>,
    /// Provider ID to start after for the next page.
    pub after_id: Option<i64>,
}

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

fn external_auth_provider_audit_details(
    provider: &AdminExternalAuthProviderInfo,
) -> Option<serde_json::Value> {
    audit::details(ExternalAuthProviderAuditDetails {
        key: &provider.key,
        kind: provider.provider_kind,
        icon_url: provider.icon_url.as_deref(),
        issuer_url: provider.issuer_url.as_deref(),
        enabled: provider.enabled,
        auto_provision_enabled: provider.auto_provision_enabled,
        auto_link_verified_email_enabled: provider.auto_link_verified_email_enabled,
        require_email_verified: provider.require_email_verified,
    })
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/external-auth/providers",
    tag = "admin",
    operation_id = "admin_list_external_auth_providers",
    params(LimitQuery, AdminExternalAuthProviderCursorQuery),
    responses(
        (status = 200, description = "External auth providers", body = inline(ApiResponse<CursorPage<external::AdminExternalAuthProviderInfo, StringIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists configured external auth providers with limit and display-name/ID
/// cursor pagination.
pub async fn list_external_auth_providers(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    cursor: web::Query<AdminExternalAuthProviderCursorQuery>,
) -> Result<HttpResponse> {
    let after = parse_string_id_cursor(
        cursor.after_display_name.clone(),
        cursor.after_id,
        "external auth provider",
    )?;
    let providers =
        external::list_admin_providers(state.get_ref(), page.limit_or(50, 100), after).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(providers)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/external-auth/provider-kinds",
    tag = "admin",
    operation_id = "admin_list_external_auth_provider_kinds",
    responses(
        (status = 200, description = "Supported external auth provider kinds", body = inline(ApiResponse<Vec<external::ExternalAuthProviderKindInfo>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Returns the external auth provider kinds supported by this deployment,
/// for building the provider creation form.
pub async fn list_external_auth_provider_kinds() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::ok(external::list_provider_kinds())))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/external-auth/providers",
    tag = "admin",
    operation_id = "admin_create_external_auth_provider",
    request_body = CreateExternalAuthProviderReq,
    responses(
        (status = 201, description = "External auth provider created", body = inline(ApiResponse<external::AdminExternalAuthProviderInfo>)),
        (status = 400, description = "Invalid provider configuration"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Creates an external auth provider configuration (issuer, enabled flags,
/// auto-provision/auto-link behavior) and records it in the audit log.
pub async fn create_external_auth_provider(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateExternalAuthProviderReq>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    validate_request(&body)?;
    let provider = external::create_provider(state.get_ref(), body.into()).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminCreateExternalAuthProvider,
        audit::AuditEntityType::ExternalAuthProvider,
        Some(provider.id),
        Some(&provider.key),
        || external_auth_provider_audit_details(&provider),
    )
    .await;
    Ok(HttpResponse::Created().json(ApiResponse::ok(provider)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/external-auth/providers/{id}",
    tag = "admin",
    operation_id = "admin_get_external_auth_provider",
    params(("id" = i64, Path, description = "External auth provider ID")),
    responses(
        (status = 200, description = "External auth provider", body = inline(ApiResponse<external::AdminExternalAuthProviderInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "External auth provider not found"),
    ),
    security(("bearer" = [])),
)]
/// Returns a single external auth provider configuration by ID.
pub async fn get_external_auth_provider(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let provider = external::get_admin_provider(state.get_ref(), *path).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(provider)))
}

#[aster_forge_api_docs_macros::path(
    patch,
    path = "/api/v1/admin/external-auth/providers/{id}",
    tag = "admin",
    operation_id = "admin_update_external_auth_provider",
    params(("id" = i64, Path, description = "External auth provider ID")),
    request_body = UpdateExternalAuthProviderReq,
    responses(
        (status = 200, description = "External auth provider updated", body = inline(ApiResponse<external::AdminExternalAuthProviderInfo>)),
        (status = 400, description = "Invalid provider configuration"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "External auth provider not found"),
    ),
    security(("bearer" = [])),
)]
/// Updates an external auth provider's configuration and records the change
/// in the audit log.
pub async fn update_external_auth_provider(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateExternalAuthProviderReq>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    validate_request(&body)?;
    let provider = external::update_provider(state.get_ref(), *path, body.into()).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminUpdateExternalAuthProvider,
        audit::AuditEntityType::ExternalAuthProvider,
        Some(provider.id),
        Some(&provider.key),
        || external_auth_provider_audit_details(&provider),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(provider)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/external-auth/providers/{id}",
    tag = "admin",
    operation_id = "admin_delete_external_auth_provider",
    params(("id" = i64, Path, description = "External auth provider ID")),
    responses(
        (status = 200, description = "External auth provider deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "External auth provider not found"),
    ),
    security(("bearer" = [])),
)]
/// Deletes an external auth provider configuration and records the removed
/// provider details in the audit log.
pub async fn delete_external_auth_provider(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let provider = external::get_admin_provider(state.get_ref(), *path).await?;
    external::delete_provider(state.get_ref(), *path).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminDeleteExternalAuthProvider,
        audit::AuditEntityType::ExternalAuthProvider,
        Some(provider.id),
        Some(&provider.key),
        || external_auth_provider_audit_details(&provider),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/external-auth/providers/test",
    tag = "admin",
    operation_id = "admin_test_external_auth_provider_params",
    request_body = ExternalAuthProviderTestParamsReq,
    responses(
        (status = 200, description = "External auth provider parameters tested", body = inline(ApiResponse<external::ExternalAuthProviderTestResult>)),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Tests a draft external auth provider configuration (before it is saved)
/// by probing its endpoints, auditing the attempt without persisting it.
pub async fn test_external_auth_provider_params(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ExternalAuthProviderTestParamsReq>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    validate_request(&body)?;
    let result = external::test_provider_params(state.get_ref(), body.into()).await;
    let success = result.is_ok();
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminTestExternalAuthProvider,
        audit::AuditEntityType::ExternalAuthProvider,
        None,
        Some("draft"),
        || {
            audit::details(audit::ExternalAuthProviderTestParamsAuditDetails {
                provider: "draft",
                key: "draft",
                success,
            })
        },
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(result?)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/external-auth/providers/{id}/test",
    tag = "admin",
    operation_id = "admin_test_external_auth_provider",
    params(("id" = i64, Path, description = "External auth provider ID")),
    responses(
        (status = 200, description = "External auth provider tested", body = inline(ApiResponse<external::ExternalAuthProviderTestResult>)),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "External auth provider not found"),
    ),
    security(("bearer" = [])),
)]
/// Tests a saved external auth provider by probing its endpoints and
/// auditing the result.
pub async fn test_external_auth_provider(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let provider = external::get_admin_provider(state.get_ref(), *path).await?;
    let result = external::test_provider(state.get_ref(), *path).await?;
    let ctx = audit::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit::log_with_details(
        state.get_ref(),
        &ctx,
        audit::AuditAction::AdminTestExternalAuthProvider,
        audit::AuditEntityType::ExternalAuthProvider,
        Some(provider.id),
        Some(&provider.key),
        || external_auth_provider_audit_details(&provider),
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(result)))
}
