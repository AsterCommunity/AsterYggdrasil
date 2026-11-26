//! Public generated texture preview routes.

use actix_web::{HttpRequest, HttpResponse, web};

use crate::api::cache::conditional_bytes_response;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::texture;

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
        (status = 200, description = "Cached texture preview PNG bytes", content_type = "image/png"),
        (status = 202, description = "Preview generation is queued or running; retry after the indicated delay"),
        (status = 404, description = "Texture or preview variant not found, or generation previously failed"),
    ),
)]
pub async fn texture_preview_by_hash(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (hash, file_name) = path.into_inner();
    match texture::texture_preview_by_hash(state.get_ref(), &hash, &file_name).await? {
        texture::TexturePreviewLookup::Ready(preview) => Ok(conditional_bytes_response(
            &req,
            preview.bytes,
            "image/png",
            texture::TEXTURE_PREVIEW_CACHE_CONTROL,
        )),
        // AsterDrive thumbnail parity: the render runs in the worker process's
        // background task lane, so a cache miss asks the client to come back.
        texture::TexturePreviewLookup::Accepted => Ok(HttpResponse::Accepted()
            .insert_header(("Retry-After", "2"))
            .finish()),
    }
}
