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
use crate::services::{auth::local, ops::audit, texture};
use aster_forge_api::{LimitQuery, parse_datetime_id_cursor, parse_sort_order_name_id_cursor};
use aster_yggdrasil_model::types::{
    yggdrasil::MinecraftTextureLibraryStatus, yggdrasil::MinecraftTextureReportReason,
    yggdrasil::MinecraftTextureReportStatus, yggdrasil::MinecraftTextureType,
    yggdrasil::MinecraftTextureVisibility, yggdrasil::TextureTagSearchMethod,
};

#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor, SortOrderNameIdCursor};

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Filter and cursor query for the admin texture library listing.
pub struct AdminTextureLibraryTextureQuery {
    /// Free-text search keyword (max 96 characters).
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    /// Restrict results to one texture type (e.g. skin, cape).
    pub texture_type: Option<MinecraftTextureType>,
    /// Restrict results to one texture visibility.
    pub visibility: Option<MinecraftTextureVisibility>,
    /// Restrict results to one library review status.
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    /// Restrict results to published or unpublished textures.
    pub published: Option<bool>,
    /// Comma-separated texture tag IDs (max 16) to filter by.
    #[serde(default, deserialize_with = "deserialize_tag_id_sequence")]
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Vec<i64>,
    /// How the tag filters combine (any or all).
    #[serde(default)]
    pub tag_search_method: TextureTagSearchMethod,
    /// Datetime-id cursor: last updated timestamp of the previous page.
    pub after_updated_at: Option<DateTime<Utc>>,
    /// Datetime-id cursor: ID of the last item on the previous page.
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Filter and cursor query for the admin texture report listing.
pub struct AdminTextureReportQuery {
    /// Restrict results to one report handling status.
    pub status: Option<MinecraftTextureReportStatus>,
    /// Restrict results to one report reason.
    pub reason: Option<MinecraftTextureReportReason>,
    /// Restrict results to reports of a single texture.
    #[validate(range(min = 1, message = "texture_id must be positive"))]
    pub texture_id: Option<i64>,
    /// Datetime-id cursor: created timestamp of the previous page.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Datetime-id cursor: ID of the last item on the previous page.
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Cursor query for the admin texture tag listing, ordered by sort order,
/// name, then ID.
pub struct AdminTextureLibraryTagQuery {
    /// Sort-order value of the last item on the previous page.
    pub after_sort_order: Option<i32>,
    /// Name of the last item on the previous page.
    pub after_name: Option<String>,
    /// ID of the last item on the previous page.
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
        (status = 200, description = "Texture library reports", body = inline(ApiResponse<CursorPage<texture::TextureReportInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists texture library reports with status/reason/texture filters and
/// datetime-id cursor pagination, for moderation triage.
pub async fn list_texture_library_reports(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminTextureReportQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let cursor =
        parse_datetime_id_cursor(query.after_created_at, query.after_id, "texture report")?;
    let reports = texture::list_admin_texture_library_reports_cursor(
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
        (status = 200, description = "Texture library report detail", body = inline(ApiResponse<texture::TextureReportInfo>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
/// Returns a single texture library report, including the reported texture
/// metadata.
pub async fn get_texture_library_report(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let report =
        texture::get_admin_texture_library_report(state.get_ref(), path.into_inner()).await?;
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
        (status = 200, description = "Accepted texture report", body = inline(ApiResponse<texture::TextureReportInfo>)),
        (status = 400, description = "Invalid report state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
/// Accepts a texture report as valid (the reported content is upheld),
/// recording the moderator decision in the audit log.
pub async fn accept_texture_library_report(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<HandleTextureReportReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let handler = local::current_user(state.get_ref(), &req).await?;
    let report = texture::accept_texture_library_report(
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
        audit::AuditAction::MinecraftTextureReportAccept,
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
        (status = 200, description = "Rejected texture report", body = inline(ApiResponse<texture::TextureReportInfo>)),
        (status = 400, description = "Invalid report state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Report not found"),
    ),
    security(("bearer" = [])),
)]
/// Dismisses a texture report as invalid, recording the moderator decision
/// in the audit log.
pub async fn reject_texture_library_report(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<HandleTextureReportReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let handler = local::current_user(state.get_ref(), &req).await?;
    let report = texture::reject_texture_library_report(
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
        audit::AuditAction::MinecraftTextureReportReject,
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
        (status = 200, description = "Texture library moderation textures", body = inline(ApiResponse<CursorPage<texture::PublicTextureLibraryTextureMetadata, DateTimeIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists library textures for moderation with type/visibility/status/published
/// tag, and keyword filters, using datetime-id cursor pagination.
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
    let page =
        texture::list_admin_texture_library_textures_cursor(state.get_ref(), limit, cursor, filter)
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
        (status = 200, description = "Texture library moderation texture", body = inline(ApiResponse<texture::PublicTextureLibraryTextureMetadata>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Returns a single library texture's metadata for the moderation view.
pub async fn get_texture_library_texture(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let texture =
        texture::get_admin_texture_library_texture(state.get_ref(), path.into_inner()).await?;
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
        (status = 200, description = "Approved texture", body = inline(ApiResponse<texture::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Approves a library texture with an optional review note and tag
/// assignments, recording the reviewer decision in the audit log.
pub async fn approve_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = local::current_user(state.get_ref(), &req).await?;
    let texture = texture::approve_texture_library_texture(
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
        audit::AuditAction::MinecraftTextureLibraryApprove,
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
        (status = 200, description = "Rejected texture", body = inline(ApiResponse<texture::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state or review note"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Rejects a library texture with a required review note, recording the
/// reviewer decision in the audit log.
pub async fn reject_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = local::current_user(state.get_ref(), &req).await?;
    let texture = texture::reject_texture_library_texture(
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
        audit::AuditAction::MinecraftTextureLibraryReject,
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
        (status = 200, description = "Unpublished texture", body = inline(ApiResponse<texture::PublicTextureLibraryTextureMetadata>)),
        (status = 400, description = "Invalid texture state"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Withdraws an already published library texture from public availability
/// with a review note, recording the action in the audit log.
pub async fn unpublish_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ReviewTextureLibraryTextureReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let reviewer = local::current_user(state.get_ref(), &req).await?;
    let texture = texture::unpublish_texture_library_texture(
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
        audit::AuditAction::MinecraftTextureLibraryUnpublish,
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
/// Permanently removes a library texture and its profile bindings, recording
/// the removed binding count in the audit log.
pub async fn delete_texture_library_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let actor = local::current_user(state.get_ref(), &req).await?;
    let deleted = texture::delete_texture_library_texture(state.get_ref(), path.into_inner())
        .await
        .map_err(texture_library_delete_error_to_api_error)?;
    log_texture_library_delete_audit(state.get_ref(), &req, actor.id, &deleted).await;
    Ok(HttpResponse::NoContent().finish())
}

async fn log_texture_library_review_audit(
    state: &AppState,
    req: &HttpRequest,
    reviewer_user_id: i64,
    action: audit::AuditAction,
    texture: &texture::PublicTextureLibraryTextureMetadata,
) {
    let ctx = audit::AuditContext::from_request(req, reviewer_user_id);
    audit::log_with_details(
        state,
        &ctx,
        action,
        audit::AuditEntityType::MinecraftTexture,
        Some(texture.id),
        Some(&texture.name),
        || {
            audit::details(audit::MinecraftTextureAuditDetails {
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
    deleted: &texture::DeletedTextureLibraryTexture,
) {
    let texture = &deleted.texture;
    let ctx = audit::AuditContext::from_request(req, actor_user_id);
    audit::log_with_details(
        state,
        &ctx,
        audit::AuditAction::MinecraftTextureDelete,
        audit::AuditEntityType::MinecraftTexture,
        Some(texture.id),
        Some(&texture.name),
        || {
            audit::details(serde_json::json!({
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

fn texture_library_delete_error_to_api_error(error: texture::TextureError) -> AsterError {
    match error.kind() {
        texture::TextureErrorKind::NotFound => AsterError::record_not_found_code(
            AsterErrorCode::TextureLibraryTextureNotFound,
            error.protocol_message(),
        ),
        texture::TextureErrorKind::Storage => AsterError::internal_error_code(
            AsterErrorCode::MinecraftTextureStoreFailed,
            error.protocol_message(),
        ),
        _ => AsterError::internal_error(error.protocol_message()),
    }
}

async fn log_texture_report_audit(
    state: &AppState,
    req: &HttpRequest,
    actor_user_id: i64,
    action: audit::AuditAction,
    report: &texture::TextureReportInfo,
) {
    let ctx = audit::AuditContext::from_request(req, actor_user_id);
    audit::log_with_details(
        state,
        &ctx,
        action,
        audit::AuditEntityType::MinecraftTexture,
        Some(report.texture_id),
        report.texture.as_ref().map(|texture| texture.name.as_str()),
        || {
            audit::details(audit::MinecraftTextureReportAuditDetails {
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
        (status = 200, description = "Texture library tags", body = inline(ApiResponse<CursorPage<texture::MinecraftTextureTagInfo, SortOrderNameIdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Lists texture library tags ordered by sort order then name, using a
/// composite cursor pagination.
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
        texture::list_texture_library_tags_paginated(state.get_ref(), limit, after, None).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(tags)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/texture-library/tags",
    tag = "admin",
    operation_id = "admin_create_texture_library_tag",
    request_body = CreateMinecraftTextureTagReq,
    responses(
        (status = 200, description = "Created texture library tag", body = inline(ApiResponse<texture::MinecraftTextureTagInfo>)),
        (status = 400, description = "Invalid or duplicate tag"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Creates a texture library tag with name, color, and sort order; duplicate
/// names are rejected.
pub async fn create_texture_library_tag(
    state: web::Data<AppState>,
    body: web::Json<CreateMinecraftTextureTagReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let tag = texture::create_texture_library_tag(
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
        (status = 200, description = "Updated texture library tag", body = inline(ApiResponse<texture::MinecraftTextureTagInfo>)),
        (status = 400, description = "Invalid or duplicate tag"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Tag not found"),
    ),
    security(("bearer" = [])),
)]
/// Updates a texture library tag's name, color, or sort order; fields left
/// absent keep their current value.
pub async fn update_texture_library_tag(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<UpdateMinecraftTextureTagReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let tag = texture::update_texture_library_tag(
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
/// Deletes a texture library tag, detaching it from the textures that
/// reference it.
pub async fn delete_texture_library_tag(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    texture::delete_texture_library_tag(state.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
