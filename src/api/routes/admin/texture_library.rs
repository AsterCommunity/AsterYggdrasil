//! Administrator texture library routes.

use actix_web::{HttpResponse, web};

use crate::api::dto::textures::{CreateMinecraftTextureTagReq, UpdateMinecraftTextureTagReq};
use crate::api::dto::validation::validate_request;
use crate::api::pagination::{LimitOffsetQuery, OffsetPage};
use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::texture_service;

#[api_docs_macros::path(
    get,
    path = "/api/v1/admin/texture-library/tags",
    tag = "admin",
    operation_id = "admin_list_texture_library_tags",
    params(LimitOffsetQuery),
    responses(
        (status = 200, description = "Texture library tags", body = inline(ApiResponse<OffsetPage<texture_service::MinecraftTextureTagInfo>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_texture_library_tags(
    state: web::Data<AppState>,
    page: web::Query<LimitOffsetQuery>,
) -> Result<HttpResponse> {
    let limit = page.limit_or(50, 100);
    let offset = page.offset();
    let tags = texture_service::list_texture_library_tags(state.get_ref()).await?;
    let total = crate::utils::numbers::usize_to_u64(tags.len(), "texture library tag count")?;
    let start = usize::try_from(offset).unwrap_or(usize::MAX);
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let items = tags
        .into_iter()
        .skip(start)
        .take(limit_usize)
        .collect::<Vec<_>>();
    Ok(HttpResponse::Ok().json(ApiResponse::ok(OffsetPage::new(
        items, total, limit, offset,
    ))))
}

#[api_docs_macros::path(
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

#[api_docs_macros::path(
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

#[api_docs_macros::path(
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
