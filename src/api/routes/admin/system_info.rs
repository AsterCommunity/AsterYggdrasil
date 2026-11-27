//! Administrator system information route.

use crate::api::response::ApiResponse;
use crate::runtime::AppState;
use crate::services::ops::health;
use actix_web::{HttpResponse, web};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/system-info",
    tag = "admin",
    operation_id = "get_admin_system_info",
    responses(
        (status = 200, description = "Admin system information", body = inline(ApiResponse<crate::services::ops::health::SystemInfoResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
/// Returns runtime system information (build info, components) reusing the
/// public health probe payload, exposed to administrators.
pub async fn get_system_info(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse::ok(health::system_info_response(
        state.get_ref(),
    )))
}
