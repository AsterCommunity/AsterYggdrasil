use super::error_code::AsterErrorCode;
use super::response::ApiResponse;
use crate::errors::AsterError;
use actix_web::{HttpResponse, web};

pub(super) fn project_query_config() -> web::QueryConfig {
    web::QueryConfig::default().error_handler(|error, _req| {
        AsterError::validation_error_code(
            AsterErrorCode::RequestMalformed,
            format!("invalid query: {error}"),
        )
        .into()
    })
}

pub(super) async fn api_not_found() -> HttpResponse {
    HttpResponse::NotFound().json(ApiResponse::<()>::error(
        AsterErrorCode::EndpointNotFound,
        "endpoint not found",
    ))
}
