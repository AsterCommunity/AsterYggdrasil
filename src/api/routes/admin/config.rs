//! Administrator config API routes.

use crate::api::dto::{ExecuteConfigActionReq, ExecuteConfigActionResp, SetConfigReq};
#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::pagination::{CursorPage, IdCursor};
use crate::api::pagination::{LimitQuery, parse_id_cursor};
use crate::api::response::ApiResponse;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth_service::AuthUserInfo;
use crate::services::{audit_service, config_service};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
pub struct ConfigCursorQuery {
    pub after_id: Option<i64>,
}

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/config",
    tag = "admin",
    operation_id = "list_config",
    params(LimitQuery, ConfigCursorQuery),
    responses(
        (status = 200, description = "List config entries", body = inline(ApiResponse<CursorPage<config_service::SystemConfig, IdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_config(
    state: web::Data<AppState>,
    query: web::Query<LimitQuery>,
    cursor: web::Query<ConfigCursorQuery>,
) -> Result<HttpResponse> {
    let limit = query.limit_or(50, 100);
    let after_id = parse_id_cursor(cursor.after_id, "config")?;
    tracing::debug!(
        limit,
        has_cursor = after_id.is_some(),
        "admin listing config entries"
    );
    let configs = config_service::list_cursor(state.get_ref(), limit, after_id).await?;
    tracing::debug!(
        count = configs.items.len(),
        total = configs.total,
        "admin listed config entries"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(configs)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/config/schema",
    tag = "admin",
    operation_id = "config_schema",
    responses(
        (status = 200, description = "Config schema", body = inline(ApiResponse<Vec<config_service::ConfigSchemaItem>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn config_schema() -> Result<HttpResponse> {
    let schema = config_service::get_schema();
    tracing::debug!(count = schema.len(), "admin loaded config schema");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(schema)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/config/template-variables",
    tag = "admin",
    operation_id = "config_template_variables",
    responses(
        (status = 200, description = "Template variables", body = inline(ApiResponse<Vec<config_service::TemplateVariableGroup>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn config_template_variables() -> Result<HttpResponse> {
    let groups = config_service::list_template_variable_groups();
    tracing::debug!(
        count = groups.len(),
        "admin loaded config template variable groups"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(groups)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/config/{key}",
    tag = "admin",
    operation_id = "get_config",
    params(("key" = String, Path, description = "Config key")),
    responses(
        (status = 200, description = "Config entry", body = inline(ApiResponse<config_service::SystemConfig>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Config key not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let key = path.into_inner();
    tracing::debug!(key, "admin loading config entry");
    let config = config_service::get_by_key(state.get_ref(), &key).await?;
    tracing::debug!(key, config_id = config.id, "admin loaded config entry");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(config)))
}

#[aster_forge_api_docs_macros::path(
    put,
    path = "/api/v1/admin/config/{key}",
    tag = "admin",
    operation_id = "set_config",
    params(("key" = String, Path, description = "Config key")),
    request_body = SetConfigReq,
    responses(
        (status = 200, description = "Config value set", body = inline(ApiResponse<config_service::SystemConfigUpdateResult>)),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn set_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<SetConfigReq>,
) -> Result<HttpResponse> {
    let user_id = current_admin_user_id(&req)?;
    let key = path.into_inner();
    tracing::debug!(
        admin_user_id = user_id,
        key,
        has_visibility = body.visibility.is_some(),
        "admin setting config entry"
    );
    let ctx = audit_service::AuditContext::from_request(&req, user_id);
    let result = config_service::set_with_audit_and_visibility_result(
        state.get_ref(),
        &key,
        &body.value,
        body.visibility,
        user_id,
        &ctx,
    )
    .await?;
    tracing::debug!(
        admin_user_id = user_id,
        key,
        config_id = result.config.id,
        warnings = result.warnings.len(),
        "admin set config entry"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(result)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/config/{key}",
    tag = "admin",
    operation_id = "delete_config",
    params(("key" = String, Path, description = "Config key")),
    responses(
        (status = 200, description = "Config entry deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Config key not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = current_admin_user_id(&req)?;
    let key = path.into_inner();
    tracing::debug!(admin_user_id = user_id, key, "admin deleting config entry");
    let ctx = audit_service::AuditContext::from_request(&req, user_id);
    config_service::delete_with_audit(state.get_ref(), &key, &ctx).await?;
    tracing::debug!(admin_user_id = user_id, key, "admin deleted config entry");
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/admin/config/{key}/action",
    tag = "admin",
    operation_id = "execute_config_action",
    params(("key" = String, Path, description = "Config action target key")),
    request_body = ExecuteConfigActionReq,
    responses(
        (status = 200, description = "Config action executed", body = inline(ApiResponse<ExecuteConfigActionResp>)),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Config action target not found"),
        (status = 503, description = "Mail service unavailable"),
    ),
    security(("bearer" = [])),
)]
pub async fn execute_config_action(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ExecuteConfigActionReq>,
) -> Result<HttpResponse> {
    crate::api::dto::validate_request(&*body)?;
    let user_id = current_admin_user_id(&req)?;
    let key = path.into_inner();
    tracing::debug!(
        admin_user_id = user_id,
        key,
        action = %body.action.as_str(),
        has_values = body.values.as_ref().is_some_and(|values| !values.is_empty()),
        "admin executing config action"
    );
    let ctx = audit_service::AuditContext::from_request(&req, user_id);
    let action_result = config_service::execute_action_with_audit(
        state.get_ref(),
        config_service::ExecuteConfigActionInput {
            key: &key,
            action: body.action,
            actor_user_id: user_id,
            values: body.values.as_ref(),
        },
        &ctx,
    )
    .await?;

    tracing::debug!(
        admin_user_id = user_id,
        key,
        action = %body.action.as_str(),
        has_value = action_result.value.is_some(),
        "admin executed config action"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(ExecuteConfigActionResp {
            message: action_result.message,
            value: action_result.value,
        })),
    )
}
