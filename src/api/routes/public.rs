//! Public frontend bootstrap routes.

use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::config_service;
use actix_web::{HttpResponse, http::header, web};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/public").route("/frontend-config", web::get().to(frontend_config)));
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/public/frontend-config",
    tag = "public",
    operation_id = "get_public_frontend_config",
    responses(
        (status = 200, description = "Public frontend bootstrap config", body = inline(ApiResponse<config_service::PublicFrontendConfig>)),
    ),
)]
pub async fn frontend_config(state: web::Data<AppState>) -> Result<HttpResponse> {
    Ok(public_config_response(
        config_service::get_public_frontend_config(state.get_ref()),
    ))
}

fn public_config_response<T: serde::Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((
            header::CACHE_CONTROL,
            config_service::PUBLIC_CONFIG_CACHE_CONTROL,
        ))
        .insert_header((header::VARY, "Authorization, Cookie"))
        .json(ApiResponse::ok(data))
}
