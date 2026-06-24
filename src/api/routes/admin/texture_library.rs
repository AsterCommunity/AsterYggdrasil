//! Administrator texture library routes.

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use validator::Validate;

use crate::api::dto::textures::{
    CreateMinecraftTextureTagReq, HandleTextureReportReq, ReviewTextureLibraryTextureReq,
    UpdateMinecraftTextureTagReq,
};
use crate::api::dto::validation::validate_request;
use crate::api::error_code::AsterErrorCode;
use crate::api::response::ApiResponse;
use crate::db::repository::{
    minecraft_texture_repo::AdminTextureLibraryListFilter,
    minecraft_texture_report_repo::AdminTextureReportListFilter,
};
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, texture_service};
use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureReportReason, MinecraftTextureReportStatus,
    MinecraftTextureType, MinecraftTextureVisibility, TextureTagSearchMethod,
};
use aster_forge_api::{LimitQuery, parse_datetime_id_cursor, parse_sort_order_name_id_cursor};

#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor, SortOrderNameIdCursor};

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
pub struct AdminTextureLibraryTextureQuery {
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    pub texture_type: Option<MinecraftTextureType>,
    pub visibility: Option<MinecraftTextureVisibility>,
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    pub published: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_tag_id_sequence")]
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Vec<i64>,
    #[serde(default)]
    pub tag_search_method: TextureTagSearchMethod,
    pub after_updated_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
pub struct AdminTextureReportQuery {
    pub status: Option<MinecraftTextureReportStatus>,
    pub reason: Option<MinecraftTextureReportReason>,
    #[validate(range(min = 1, message = "texture_id must be positive"))]
    pub texture_id: Option<i64>,
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
pub struct AdminTextureLibraryTagQuery {
    pub after_sort_order: Option<i32>,
    pub after_name: Option<String>,
    pub after_id: Option<i64>,
}

fn deserialize_tag_id_sequence<'de, D>(deserializer: D) -> std::result::Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                serde::de::Error::custom("tag_ids must be a comma-separated integer sequence")
            })
        })
        .collect()
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/reports",
    tag = "admin",
    operation_id = "admin_list_texture_library_reports",
    params(LimitQuery, AdminTextureReportQuery),
    responses(
        (status = 200, description = "Texture library reports", body = inline(ApiResponse<CursorPage<texture_service::TextureReportInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_texture_library_reports(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminTextureReportQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let cursor =
        parse_datetime_id_cursor(query.after_created_at, query.after_id, "texture report")?;
    let reports = texture_service::list_admin_texture_library_reports_cursor(
        state.get_ref(),
        page.limit_or(50, 100),
        AdminTextureReportListFilter {
            status: query.status,
            reason: query.reason,
            texture_id: query.texture_id,
        },
        cursor,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(reports)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/reports/{report_id}",
    tag = "admin",
    operation_id = "admin_get_texture_library_report",
    params(("report_id" = i64, Path, description = "Texture report ID")),
    responses(
        (status = 200, description = "Texture library report detail", body = inline(ApiResponse<texture_service::TextureReportInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_texture_library_report(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let report =
        texture_service::get_admin_texture_library_report(state.get_ref(), path.into_inner())
            .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(report)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/reports/{report_id}/accept",
    tag = "admin",
    operation_id = "admin_accept_texture_library_report",
    request_body = HandleTextureReportReq,
    params(("report_id" = i64, Path, description = "Texture report ID")),
    responses(
        (status = 200, description = "Accepted texture report", body = inline(ApiResponse<texture_service::TextureReportInfo>)),
        (status = 400, description = "Invalid report state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn accept_texture_library_report(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<HandleTextureReportReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let handler = auth_service::current_user(state.get_ref(), &req).await?;
    let report = texture_service::accept_texture_library_report(
        state.get_ref(),
        handler.id,
        path.into_inner(),
        body.admin_note.clone(),
    )
    .await?;
    log_texture_report_audit(
        state.get_ref(),
        &req,
        handler.id,
        audit_service::AuditAction::MinecraftTextureReportAccept,
        &report,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(report)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/reports/{report_id}/reject",
    tag = "admin",
    operation_id = "admin_reject_texture_library_report",
    request_body = HandleTextureReportReq,
    params(("report_id" = i64, Path, description = "Texture report ID")),
    responses(
        (status = 200, description = "Rejected texture report", body = inline(ApiResponse<texture_service::TextureReportInfo>)),
        (status = 400, description = "Invalid report state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn reject_texture_library_report(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<HandleTextureReportReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let handler = auth_service::current_user(state.get_ref(), &req).await?;
    let report = texture_service::reject_texture_library_report(
        state.get_ref(),
        handler.id,
        path.into_inner(),
        body.admin_note.clone(),
    )
    .await?;
    log_texture_report_audit(
        state.get_ref(),
        &req,
        handler.id,
        audit_service::AuditAction::MinecraftTextureReportReject,
        &report,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(report)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/textures",
    tag = "admin",
    operation_id = "admin_list_texture_library_textures",
    params(LimitQuery, AdminTextureLibraryTextureQuery),
    responses(
        (status = 200, description = "Texture library moderation textures", body = inline(ApiResponse<CursorPage<texture_service::PublicTextureLibraryTextureMetadata, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_texture_library_textures(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminTextureLibraryTextureQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let limit = page.limit_or(50, 100);
    let cursor =
        parse_datetime_id_cursor(query.after_updated_at, query.after_id, "texture library")?;
    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let filter = AdminTextureLibraryListFilter {
        texture_type: query.texture_type,
        visibility: query.visibility,
        library_status: query.library_status,
        published: query.published,
        tag_ids: query.tag_ids.clone(),
        tag_search_method: query.tag_search_method,
        keyword,
    };
    let page = texture_service::list_admin_texture_library_textures_cursor(
        state.get_ref(),
        limit,
        cursor,
        filter,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/textures/{texture_id}",
    tag = "admin",
    operation_id = "admin_get_texture_library_texture",
    params(("texture_id" = i64, Path, description = "Texture ID")),
    responses(
        (status = 200, description = "Texture library moderation texture", body = inline(ApiResponse<texture_service::PublicTextureLibraryTextureMetadata>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_texture_library_texture(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let texture =
        texture_service::get_admin_texture_library_texture(state.get_ref(), path.into_inner())
            .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/textures/{texture_id}/approve",
    tag = "admin",
    operation_id = "admin_approve_texture_library_texture",
    request_body = ReviewTextureLibraryTextureReq,
    params(("texture_id" = i64, Path, description = "Texture ID")),
    responses(
        (status = 200, description = "Approved texture", body = inline(ApiResponse<texture_service::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn approve_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = auth_service::current_user(state.get_ref(), &req).await?;
    let texture = texture_service::approve_texture_library_texture(
        state.get_ref(),
        reviewer.id,
        path.into_inner(),
        body.review_note.clone(),
        body.tag_ids.as_deref(),
    )
    .await?;
    log_texture_library_review_audit(
        state.get_ref(),
        &req,
        reviewer.id,
        audit_service::AuditAction::MinecraftTextureLibraryApprove,
        &texture,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/textures/{texture_id}/reject",
    tag = "admin",
    operation_id = "admin_reject_texture_library_texture",
    request_body = ReviewTextureLibraryTextureReq,
    params(("texture_id" = i64, Path, description = "Texture ID")),
    responses(
        (status = 200, description = "Rejected texture", body = inline(ApiResponse<texture_service::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state or review note"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn reject_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = auth_service::current_user(state.get_ref(), &req).await?;
    let texture = texture_service::reject_texture_library_texture(
        state.get_ref(),
        reviewer.id,
        path.into_inner(),
        body.review_note.clone(),
    )
    .await?;
    log_texture_library_review_audit(
        state.get_ref(),
        &req,
        reviewer.id,
        audit_service::AuditAction::MinecraftTextureLibraryReject,
        &texture,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/textures/{texture_id}/unpublish",
    tag = "admin",
    operation_id = "admin_unpublish_texture_library_texture",
    request_body = ReviewTextureLibraryTextureReq,
    params(("texture_id" = i64, Path, description = "Texture ID")),
    responses(
        (status = 200, description = "Unpublished texture", body = inline(ApiResponse<texture_service::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn unpublish_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = auth_service::current_user(state.get_ref(), &req).await?;
    let texture = texture_service::unpublish_texture_library_texture(
        state.get_ref(),
        reviewer.id,
        path.into_inner(),
        body.review_note.clone(),
    )
    .await?;
    log_texture_library_review_audit(
        state.get_ref(),
        &req,
        reviewer.id,
        audit_service::AuditAction::MinecraftTextureLibraryUnpublish,
        &texture,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/texture-library/textures/{texture_id}",
    tag = "admin",
    operation_id = "admin_delete_texture_library_texture",
    params(("texture_id" = i64, Path, description = "Texture ID")),
    responses(
        (status = 204, description = "Deleted texture library texture"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let actor = auth_service::current_user(state.get_ref(), &req).await?;
    let deleted =
        texture_service::delete_texture_library_texture(state.get_ref(), path.into_inner())
            .await
            .map_err(texture_library_delete_error_to_api_error)?;
    log_texture_library_delete_audit(state.get_ref(), &req, actor.id, &deleted).await;
    Ok(HttpResponse::NoContent().finish())
}

async fn log_texture_library_review_audit(
    state: &AppState,
    req: &HttpRequest,
    reviewer_user_id: i64,
    action: audit_service::AuditAction,
    texture: &texture_service::PublicTextureLibraryTextureMetadata,
) {
    let ctx = audit_service::AuditContext::from_request(req, reviewer_user_id);
    audit_service::log_with_details(
        state,
        &ctx,
        action,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(texture.id),
        Some(&texture.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: "",
                profile_name: "",
                texture_type: texture.texture_type,
                texture_hash: Some(&texture.hash),
                texture_model: Some(texture.texture_model),
                width: Some(texture.width),
                height: Some(texture.height),
                file_size: Some(texture.file_size),
                library_status: Some(texture.library_status),
                review_note: texture.library_review_note.as_deref(),
            })
        },
    )
    .await;
}

async fn log_texture_library_delete_audit(
    state: &AppState,
    req: &HttpRequest,
    actor_user_id: i64,
    deleted: &texture_service::DeletedTextureLibraryTexture,
) {
    let texture = &deleted.texture;
    let ctx = audit_service::AuditContext::from_request(req, actor_user_id);
    audit_service::log_with_details(
        state,
        &ctx,
        audit_service::AuditAction::MinecraftTextureDelete,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(texture.id),
        Some(&texture.name),
        || {
            audit_service::details(serde_json::json!({
                "profile_uuid": "",
                "profile_name": "",
                "texture_type": texture.texture_type,
                "texture_hash": texture.hash,
                "texture_model": texture.texture_model,
                "width": texture.width,
                "height": texture.height,
                "file_size": texture.file_size,
                "library_status": texture.library_status,
                "profile_binding_count": deleted.deleted_profile_binding_count,
            }))
        },
    )
    .await;
}

fn texture_library_delete_error_to_api_error(error: texture_service::TextureError) -> AsterError {
    match error.kind() {
        texture_service::TextureErrorKind::NotFound => AsterError::record_not_found_code(
            AsterErrorCode::TextureLibraryTextureNotFound,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::Storage => AsterError::internal_error_code(
            AsterErrorCode::MinecraftObjectStorageFailed,
            error.protocol_message(),
        ),
        _ => AsterError::internal_error(error.protocol_message()),
    }
}

async fn log_texture_report_audit(
    state: &AppState,
    req: &HttpRequest,
    actor_user_id: i64,
    action: audit_service::AuditAction,
    report: &texture_service::TextureReportInfo,
) {
    let ctx = audit_service::AuditContext::from_request(req, actor_user_id);
    audit_service::log_with_details(
        state,
        &ctx,
        action,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(report.texture_id),
        report.texture.as_ref().map(|texture| texture.name.as_str()),
        || {
            audit_service::details(audit_service::MinecraftTextureReportAuditDetails {
                texture_id: report.texture_id,
                report_id: report.id,
                reason: report.reason,
                report_status: report.status,
                library_status: report
                    .texture
                    .as_ref()
                    .map(|texture| texture.library_status),
            })
        },
    )
    .await;
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/tags",
    tag = "admin",
    operation_id = "admin_list_texture_library_tags",
    params(LimitQuery, AdminTextureLibraryTagQuery),
    responses(
        (status = 200, description = "Texture library tags", body = inline(ApiResponse<CursorPage<texture_service::MinecraftTextureTagInfo, SortOrderNameIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_texture_library_tags(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminTextureLibraryTagQuery>,
) -> Result<HttpResponse> {
    let limit = page.limit_or(50, 100);
    let after = parse_sort_order_name_id_cursor(
        query.after_sort_order,
        query.after_name.clone(),
        query.after_id,
        "texture tag",
    )?;
    let tags =
        texture_service::list_texture_library_tags_paginated(state.get_ref(), limit, after, None)
            .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(tags)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/tags",
    tag = "admin",
    operation_id = "admin_create_texture_library_tag",
    request_body = CreateMinecraftTextureTagReq,
    responses(
        (status = 200, description = "Created texture library tag", body = inline(ApiResponse<texture_service::MinecraftTextureTagInfo>)),
        (status = 400, description = "Invalid or duplicate tag"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn create_texture_library_tag(
    state: web::Data<AppState>,
    body: web::Json<CreateMinecraftTextureTagReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let tag = texture_service::create_texture_library_tag(
        state.get_ref(),
        &body.name,
        &body.color,
        body.sort_order,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(tag)))
}

#[aster_forge_api_docs_macros::path(
    patch,
    path = "/api/v1/admin/texture-library/tags/{tag_id}",
    tag = "admin",
    operation_id = "admin_update_texture_library_tag",
    request_body = UpdateMinecraftTextureTagReq,
    params(("tag_id" = i64, Path, description = "Texture library tag ID")),
    responses(
        (status = 200, description = "Updated texture library tag", body = inline(ApiResponse<texture_service::MinecraftTextureTagInfo>)),
        (status = 400, description = "Invalid or duplicate tag"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Tag not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn update_texture_library_tag(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<UpdateMinecraftTextureTagReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let tag = texture_service::update_texture_library_tag(
        state.get_ref(),
        path.into_inner(),
        body.name.as_deref(),
        body.color.as_deref(),
        body.sort_order,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(tag)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/texture-library/tags/{tag_id}",
    tag = "admin",
    operation_id = "admin_delete_texture_library_tag",
    params(("tag_id" = i64, Path, description = "Texture library tag ID")),
    responses(
        (status = 204, description = "Deleted texture library tag"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Tag not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_texture_library_tag(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    texture_service::delete_texture_library_tag(state.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
