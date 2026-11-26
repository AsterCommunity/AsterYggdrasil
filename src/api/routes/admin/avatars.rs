//! Administrator shared avatar media routes.

use crate::api::cache::conditional_bytes_response;
use crate::errors::Result;
use crate::runtime::AppState;
use crate::services::user;
use actix_web::{HttpRequest, HttpResponse, web};

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/avatars/users/{id}/{size}",
    tag = "admin",
    operation_id = "admin_get_user_avatar",
    params(
        ("id" = i64, Path, description = "User ID"),
        ("size" = u32, Path, description = "Avatar size (512 or 1024)")
    ),
    responses(
        (status = 200, description = "Avatar image (WebP)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Avatar not found"),
    ),
    security(("bearer" = [])),
)]
/// Serves a user's avatar image (WebP) at the requested size with
/// conditional-request caching, for administrative inspection.
pub async fn get_user_avatar(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(i64, u32)>,
) -> Result<HttpResponse> {
    let (user_id, size) = path.into_inner();
    tracing::debug!(user_id, size, "admin loading user avatar");
    let bytes = user::profile::get_avatar_bytes(state.get_ref(), user_id, size).await?;
    tracing::debug!(
        user_id,
        size,
        bytes = bytes.len(),
        "admin loaded user avatar"
    );
    Ok(conditional_bytes_response(
        &req,
        bytes,
        user::profile::AVATAR_CONTENT_TYPE,
        user::profile::AVATAR_CACHE_CONTROL,
    ))
}
