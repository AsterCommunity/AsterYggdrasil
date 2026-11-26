//! Public texture library routes.

use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use validator::Validate;

use crate::api::dto::{CopyPublicTextureReq, CreateTextureReportReq, validate_request};
use crate::api::response::ApiResponse;
use crate::db::repository::minecraft_texture_repo::WardrobeTextureListFilter;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{auth::local, ops::audit, texture};
use aster_forge_api::{LimitQuery, parse_datetime_id_cursor, parse_sort_order_name_id_cursor};
use aster_yggdrasil_model::types::yggdrasil::{MinecraftTextureType, TextureTagSearchMethod};

#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor, SortOrderNameIdCursor};

const TEXTURE_TAG_FILTER_LIMIT: usize = 16;
const DEFAULT_TEXTURE_TAG_PAGE_SIZE: u64 = 30;

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Query parameters for browsing published textures in the public texture library.
pub struct PublicTextureLibraryQuery {
    /// Substring matched against the texture's display name, hash, or MIME type.
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    /// Restricts results to one texture type (skin or cape).
    pub texture_type: Option<MinecraftTextureType>,
    /// Comma-separated texture tag IDs to filter by; at most 16 positive IDs.
    #[serde(default, deserialize_with = "deserialize_tag_id_sequence")]
    #[validate(length(max = 16, message = "tag_ids must not contain more than 16 items"))]
    pub tag_ids: Vec<i64>,
    /// Whether a texture must carry all of the given tags or any of them; defaults to all.
    #[serde(default)]
    pub tag_search_method: TextureTagSearchMethod,
    /// Updated-at timestamp of the last item on the previous page.
    pub after_updated_at: Option<DateTime<Utc>>,
    /// ID of the last item on the previous page, paired with `after_updated_at` as the cursor.
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Query parameters for listing the texture library tags shown to public visitors.
pub struct PublicTextureLibraryTagQuery {
    /// Case-insensitive substring matched against tag names.
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    /// Sort order value of the last tag on the previous page, for cursor pagination.
    pub after_sort_order: Option<i32>,
    /// Name of the last tag on the previous page, paired with `after_sort_order`.
    pub after_name: Option<String>,
    /// ID of the last tag on the previous page, completing the sort-order cursor.
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

/// Registers the `/texture-library` scope: anonymous browsing of published textures
/// and tags, plus authenticated copy-to-wardrobe and report endpoints.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/texture-library")
            .route("/tags", web::get().to(list_public_texture_library_tags))
            .route("/textures", web::get().to(list_public_textures))
            .route("/textures/{texture_id}", web::get().to(get_public_texture))
            .route(
                "/textures/{texture_id}/copy",
                web::post().to(copy_public_texture),
            )
            .route(
                "/textures/{texture_id}/reports",
                web::post().to(create_texture_report),
            ),
    );
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/texture-library/tags",
    tag = "texture-library",
    operation_id = "list_public_texture_library_tags",
    params(LimitQuery, PublicTextureLibraryTagQuery),
    responses(
        (status = 200, description = "Public texture library tags", body = inline(ApiResponse<CursorPage<texture::MinecraftTextureTagInfo, SortOrderNameIdCursor>>)),
    ),
)]
/// Lists the administrator-managed texture library tags for public visitors,
/// with keyword search and sort-order cursor pagination.
pub async fn list_public_texture_library_tags(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<PublicTextureLibraryTagQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let limit = page.limit_or(DEFAULT_TEXTURE_TAG_PAGE_SIZE, 100);
    let after = parse_sort_order_name_id_cursor(
        query.after_sort_order,
        query.after_name.clone(),
        query.after_id,
        "texture tag",
    )?;
    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let page = texture::list_texture_library_tags_paginated(state.get_ref(), limit, after, keyword)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/texture-library/textures",
    tag = "texture-library",
    operation_id = "list_public_texture_library_textures",
    params(LimitQuery, PublicTextureLibraryQuery),
    responses(
        (status = 200, description = "Public texture library textures", body = inline(ApiResponse<CursorPage<texture::PublicTextureLibraryTextureMetadata, DateTimeIdCursor>>)),
        (status = 400, description = "Invalid query"),
    ),
)]
/// Lists textures published to the public texture library with cursor pagination
/// and optional texture type, tag, and keyword filters; no authentication required.
pub async fn list_public_textures(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<PublicTextureLibraryQuery>,
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
    let filter = WardrobeTextureListFilter {
        texture_type: query.texture_type,
        tag_ids: normalize_tag_filter_ids(&query.tag_ids)?,
        tag_search_method: query.tag_search_method,
        keyword,
    };
    let page =
        texture::list_public_texture_library_cursor(state.get_ref(), limit, cursor, filter).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(page)))
}

fn normalize_tag_filter_ids(tag_ids: &[i64]) -> Result<Vec<i64>> {
    if tag_ids.len() > TEXTURE_TAG_FILTER_LIMIT {
        return Err(AsterError::validation_error_code(
            crate::api::error_code::AsterErrorCode::ValidationFailed,
            "tag_ids must not contain more than 16 items",
        ));
    }

    let mut normalized = Vec::with_capacity(tag_ids.len());
    for tag_id in tag_ids {
        if *tag_id <= 0 {
            return Err(AsterError::validation_error_code(
                crate::api::error_code::AsterErrorCode::ValidationFailed,
                "tag_ids must contain positive integers",
            ));
        }
        if !normalized.contains(tag_id) {
            normalized.push(*tag_id);
        }
    }
    Ok(normalized)
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/texture-library/textures/{texture_id}",
    tag = "texture-library",
    operation_id = "get_public_texture_library_texture",
    params(("texture_id" = i64, Path, description = "Public texture ID")),
    responses(
        (status = 200, description = "Public texture detail", body = inline(ApiResponse<texture::PublicTextureLibraryTextureMetadata>)),
        (status = 404, description = "Public texture not found"),
    ),
)]
/// Returns the detail of one texture published to the public library, or 404
/// for IDs that are not published.
pub async fn get_public_texture(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let texture_id = path.into_inner();
    let texture = texture::get_public_texture_library_texture(state.get_ref(), texture_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/texture-library/textures/{texture_id}/copy",
    tag = "texture-library",
    operation_id = "copy_public_texture_library_texture_to_wardrobe",
    request_body = CopyPublicTextureReq,
    params(("texture_id" = i64, Path, description = "Public texture ID")),
    responses(
        (status = 200, description = "Copied texture in current user's wardrobe", body = inline(ApiResponse<texture::MinecraftWardrobeTextureMetadata>)),
        (status = 400, description = "Invalid copy request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Public texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Copies a published library texture into the current user's wardrobe, optionally
/// renaming it; the request body may be empty to accept the defaults.
pub async fn copy_public_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Bytes,
) -> Result<HttpResponse> {
    let texture_id = path.into_inner();
    let user = local::current_user(state.get_ref(), &req).await?;
    let body = if body.is_empty() {
        CopyPublicTextureReq::default()
    } else {
        serde_json::from_slice::<CopyPublicTextureReq>(&body).map_err(|_| {
            AsterError::validation_error_code(
                crate::api::error_code::AsterErrorCode::RequestMalformed,
                "malformed copy texture request body",
            )
        })?
    };
    validate_request(&body)?;
    let texture = texture::copy_public_texture_to_wardrobe(
        state.get_ref(),
        user.id,
        texture_id,
        body.display_name,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(texture)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/texture-library/textures/{texture_id}/reports",
    tag = "texture-library",
    operation_id = "create_public_texture_library_texture_report",
    request_body = CreateTextureReportReq,
    params(("texture_id" = i64, Path, description = "Public texture ID")),
    responses(
        (status = 200, description = "Created texture report", body = inline(ApiResponse<texture::TextureReportInfo>)),
        (status = 400, description = "Invalid report request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Public texture not found"),
    ),
    security(("bearer" = [])),
)]
/// Files a report against a published library texture on behalf of the current user
/// (reason plus optional message) and records a report audit event.
pub async fn create_texture_report(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<CreateTextureReportReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let texture_id = path.into_inner();
    let user = local::current_user(state.get_ref(), &req).await?;
    let report = texture::create_texture_library_report(
        state.get_ref(),
        user.id,
        texture_id,
        body.reason,
        body.message.clone(),
    )
    .await?;
    log_texture_report_audit(
        state.get_ref(),
        &req,
        user.id,
        audit::AuditAction::MinecraftTextureReportCreate,
        &report,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(report)))
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
