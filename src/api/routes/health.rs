//! Health and readiness routes.

use crate::api::error_code::AsterErrorCode;
use crate::api::response::{ApiResponse, HealthResponse};
use crate::runtime::AppState;
use crate::services::ops::health;
use actix_web::{HttpResponse, web};

const READY_DB_UNAVAILABLE_MESSAGE: &str = "Database unavailable";
const READY_RUNTIME_UNAVAILABLE_MESSAGE: &str = "Runtime unavailable";
const READY_ROLE_NOT_READY_MESSAGE: &str = "Process role is initializing or draining";

/// Builds the `/health` scope with liveness and readiness probes (GET and HEAD),
/// plus the Prometheus scrape route when the metrics feature and exporter are enabled.
pub fn routes(observability: &aster_forge_metrics::ObservabilityConfig) -> actix_web::Scope {
    let scope = web::scope("/health")
        .route("", web::get().to(health))
        .route("", web::head().to(health))
        .route("/ready", web::get().to(ready))
        .route("/ready", web::head().to(ready));

    crate::metrics::configure_route(scope, observability)
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/health",
    tag = "health",
    operation_id = "health",
    responses(
        (status = 200, description = "Service is healthy", body = inline(HealthResponse)),
        (status = 503, description = "Role is initializing or a local task worker stopped", body = inline(HealthResponse)),
    ),
)]
/// Liveness probe: 200 while the process role is live, 503 during initialization,
/// draining, or when a local task worker has stopped.
pub async fn health(role: Option<web::Data<crate::runtime::role::RoleHealth>>) -> HttpResponse {
    tracing::debug!("serving health probe");
    if role.as_ref().is_some_and(|role| !role.live()) {
        return HttpResponse::ServiceUnavailable()
            .json(status_response("initializing_or_worker_stopped"));
    }
    HttpResponse::Ok().json(status_response("ok"))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/health/ready",
    tag = "health",
    operation_id = "ready",
    responses(
        (status = 200, description = "Service is ready", body = inline(ApiResponse<HealthResponse>)),
        (status = 503, description = "Database is unavailable or the process role is not ready"),
    ),
)]
/// Readiness probe: 200 only when the process role is ready and the database and runtime
/// checks pass; otherwise 503 with a stable error code so orchestrators can gate traffic.
pub async fn ready(
    state: web::Data<AppState>,
    role: Option<web::Data<crate::runtime::role::RoleHealth>>,
) -> HttpResponse {
    tracing::debug!("serving readiness probe");
    if role.as_ref().is_some_and(|role| !role.ready()) {
        tracing::debug!("readiness probe rejected: process role is not ready");
        return HttpResponse::ServiceUnavailable().json(ApiResponse::<()>::error_body(
            AsterErrorCode::RuntimeUnavailable,
            READY_ROLE_NOT_READY_MESSAGE,
            Some(true),
        ));
    }
    match health::check_ready(state.get_ref()).await {
        Ok(_) => {
            tracing::debug!("readiness probe succeeded");
            HttpResponse::Ok().json(ApiResponse::ok(status_response("ready")))
        }
        Err(error) => ready_error(error),
    }
}

fn ready_error(error: crate::errors::AsterError) -> HttpResponse {
    let code = error.api_error_code();
    let message = if code == AsterErrorCode::DatabaseError {
        READY_DB_UNAVAILABLE_MESSAGE
    } else {
        READY_RUNTIME_UNAVAILABLE_MESSAGE
    };
    tracing::error!(error = %error, api_code = %code, "health readiness check failed");
    HttpResponse::ServiceUnavailable().json(ApiResponse::<()>::error_body(
        code,
        message,
        error.retryable(),
    ))
}

fn status_response(status: &str) -> HealthResponse {
    HealthResponse {
        status: status.to_string(),
    }
}
