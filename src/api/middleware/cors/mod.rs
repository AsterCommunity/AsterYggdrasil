//! Product boundary for runtime CORS middleware.
//!
//! `aster_forge_middleware::actix::cors` owns the Actix middleware mechanics: origin parsing,
//! preflight validation, response header application, and `Vary` handling. This module keeps
//! Yggdrasil-specific concerns at the edge by injecting `AppState`, runtime config policy,
//! product static-asset exemptions, dynamic CSRF header names, and `AsterError` mapping.

mod constants;

use actix_web::{
    Error,
    body::{EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web,
};
use aster_forge_middleware::actix::cors::{
    CorsMiddlewareError, RuntimeCors as ForgeRuntimeCors, RuntimeCorsConfig,
};
use futures::future::Ready;

use self::constants::{ALLOWED_HEADERS, ALLOWED_METHODS, EXPOSE_HEADERS};
use crate::api::middleware::csrf;
use crate::config::cors;
use crate::errors::AsterError;
use crate::runtime::AppState;

pub struct RuntimeCors;

impl<S, B> Transform<S, ServiceRequest> for RuntimeCors
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = <ForgeRuntimeCors as Transform<S, ServiceRequest>>::Transform;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ForgeRuntimeCors::new(runtime_cors_config()).new_transform(service)
    }
}

fn runtime_cors_config() -> RuntimeCorsConfig {
    RuntimeCorsConfig::new(
        |req| {
            let state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(|| AsterError::internal_error("AppState not found"))?;
            Ok(cors::runtime_cors_policy(
                state.get_ref().runtime_config.as_ref(),
            ))
        },
        is_cors_exempt_path,
        map_cors_error,
    )
    .allowed_methods(ALLOWED_METHODS.iter().copied())
    .allowed_headers(allowed_headers_for_response())
    .exposed_headers(EXPOSE_HEADERS.iter().copied())
}

/// Paths that serve static assets or public pages and do not need CORS enforcement.
fn is_cors_exempt_path(path: &str) -> bool {
    matches!(
        path,
        "/" | "/index.html"
            | "/favicon.svg"
            | "/manifest.webmanifest"
            | "/registerSW.js"
            | "/sw.js"
    ) || path.starts_with("/workbox-")
        || path.starts_with("/assets/")
        || path.starts_with("/static/")
}

fn allowed_headers_for_response() -> Vec<&'static str> {
    let mut headers = ALLOWED_HEADERS.to_vec();
    let csrf_header = csrf::token_names().header_name_str();
    if !headers.contains(&csrf_header) {
        let insert_index = headers
            .iter()
            .position(|header| *header == "x-request-id")
            .unwrap_or(headers.len());
        headers.insert(insert_index, csrf_header);
    }
    headers
}

fn map_cors_error(error: CorsMiddlewareError) -> Error {
    AsterError::validation_error(error.message()).into()
}
