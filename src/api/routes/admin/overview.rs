//! Administrator overview route.

use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::ops::admin;
use actix_web::{HttpResponse, web};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/overview",
    tag = "admin",
    operation_id = "get_admin_overview",
    responses(
        (status = 200, description = "Admin overview", body = inline(ApiResponse<admin::overview::AdminOverviewResp>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_overview(state: web::Data<AppState>) -> Result<HttpResponse> {
    tracing::debug!("admin overview request received");
    let overview = admin::overview::overview(state.get_ref()).await?;
    tracing::debug!(
        total_users = overview.summary.total_users,
        minecraft_profiles = overview.summary.minecraft_profile_count,
        textures = overview.summary.texture_count,
        "admin overview request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(overview)))
}
