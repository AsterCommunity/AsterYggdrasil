//! Public frontend bootstrap routes.

use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::ops::config;
use actix_web::{HttpResponse, http::header, web};

/// Registers the unauthenticated `/public` scope with the frontend bootstrap config endpoint.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/public").route("/frontend-config", web::get().to(frontend_config)));
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/public/frontend-config",
    tag = "public",
    operation_id = "get_public_frontend_config",
    responses(
        (status = 200, description = "Public frontend bootstrap config", body = inline(ApiResponse<config::PublicFrontendConfig>)),
    ),
)]
/// Serves the public bootstrap config (branding, captcha, Yggdrasil, texture library)
/// that the web panel fetches before login; responses are cacheable for 60s and vary on credentials.
pub async fn frontend_config(state: web::Data<AppState>) -> Result<HttpResponse> {
    Ok(public_config_response(config::get_public_frontend_config(
        state.get_ref(),
    )))
}

fn public_config_response<T: serde::Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, config::PUBLIC_CONFIG_CACHE_CONTROL))
        .insert_header((header::VARY, "Authorization, Cookie"))
        .json(ApiResponse::ok(data))
}
