//! Administrator system information route.

use crate::api::response::ApiResponse;
use crate::api::routes::health;
use crate::runtime::AppState;
use actix_web::{HttpResponse, web};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/system-info",
    tag = "admin",
    operation_id = "get_admin_system_info",
    responses(
        (status = 200, description = "Admin system information", body = inline(ApiResponse<crate::api::response::SystemInfoResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_system_info(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse::ok(health::system_info_response(
        state.get_ref(),
    )))
}
