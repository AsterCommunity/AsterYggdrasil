//! Permission registry introspection route; any authenticated account may
//! read the descriptor dictionary so player-facing labels render without
//! admin privileges.

use crate::api::response::ApiResponse;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::auth::local;
use actix_web::{HttpRequest, HttpResponse, web};
use aster_yggdrasil_model::types::permission::PermissionKind;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// One registered permission descriptor.
pub struct AdminPermissionInfo {
    /// Namespaced permission identifier.
    pub id: String,
    /// Semantic class (`admin_area` or `capability`).
    pub kind: PermissionKind,
    /// Frontend i18n message key rendering the localized label.
    pub label_i18n_key: String,
    /// English fallback label used when the i18n bundle lacks the key.
    pub label: String,
    /// UI grouping hint.
    pub group: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Every permission the server considers valid at authorization boundaries.
pub struct AdminPermissionListResp {
    /// Registered descriptors ordered by identifier.
    pub permissions: Vec<AdminPermissionInfo>,
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/permissions",
    tag = "admin",
    operation_id = "list_admin_permissions",
    responses(
        (status = 200, description = "Registered permissions", body = inline(ApiResponse<AdminPermissionListResp>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
/// Lists every registered permission descriptor, so permission editors and
/// player-facing capability badges can render labels without hardcoding an
/// identifier-to-display mapping.
pub async fn list_permissions(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    local::current_user(state.get_ref(), &req).await?;
    let registry = state.permission_registry();
    let permissions = registry
        .all()
        .map(|descriptor| AdminPermissionInfo {
            id: descriptor.id.as_str().to_string(),
            kind: descriptor.kind,
            label_i18n_key: descriptor.label_i18n_key.to_string(),
            label: descriptor.label.to_string(),
            group: descriptor.group.to_string(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(ApiResponse::ok(AdminPermissionListResp { permissions })))
}

/// Registers the `/permissions` descriptor dictionary route.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/permissions").route("", web::get().to(list_permissions)));
}
