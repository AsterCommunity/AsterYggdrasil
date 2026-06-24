//! Public generated texture preview routes.

use actix_web::{HttpRequest, HttpResponse, web};

use crate::api::cache::conditional_bytes_response;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::texture_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/texture-previews").route(
        "/{hash}/{file_name}",
        web::get().to(texture_preview_by_hash),
    ));
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/texture-previews/{hash}/{file_name}",
    tag = "texture-preview",
    operation_id = "texture_preview_by_hash",
    params(
        ("hash" = String, Path, description = "Texture SHA-256 hash"),
        ("file_name" = String, Path, description = "Generated preview variant filename"),
    ),
    responses(
        (status = 200, description = "Generated texture preview PNG bytes", content_type = "image/png"),
        (status = 404, description = "Texture or preview variant not found"),
    ),
)]
pub async fn texture_preview_by_hash(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (hash, file_name) = path.into_inner();
    let preview =
        texture_service::texture_preview_by_hash(state.get_ref(), &hash, &file_name).await?;
    Ok(conditional_bytes_response(
        &req,
        preview.bytes,
        "image/png",
        texture_service::TEXTURE_PREVIEW_CACHE_CONTROL,
    ))
}
