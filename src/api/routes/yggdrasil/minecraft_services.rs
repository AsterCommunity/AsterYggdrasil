use actix_web::{HttpRequest, HttpResponse, web};

use crate::config::yggdrasil::RuntimeYggdrasilPolicy;
use crate::runtime::AppState;
use crate::services::yggdrasil_service::{self, YggdrasilError, YggdrasilErrorKind};

use super::yggdrasil_error_response;

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/minecraftservices/player/certificates",
    tag = "yggdrasil",
    operation_id = "minecraft_services_player_certificates",
    responses(
        (status = 200, description = "Minecraft services profile key certificate", body = crate::api::dto::yggdrasil::MinecraftServicesCertificateResp),
        (status = 401, description = "Missing or invalid bearer token", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
        (status = 404, description = "Profile key support is disabled", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
    ),
    security(("bearer" = [])),
)]
pub async fn player_certificates(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    const PATH: &str = "/player/certificates";
    tracing::debug!("received minecraft services player certificates request");
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config());
    if !policy.enable_profile_key {
        tracing::debug!(
            "minecraft services player certificates rejected because profile key is disabled"
        );
        return minecraft_services_not_found(PATH);
    }

    let Some(access_token) = crate::api::request_auth::bearer_token(&req) else {
        tracing::debug!("minecraft services player certificates rejected missing bearer token");
        return minecraft_services_unauthorized(PATH);
    };

    match yggdrasil_service::profile_key_certificate(state.get_ref(), &access_token).await {
        Ok(response) => {
            tracing::debug!("minecraft services player certificates request completed");
            HttpResponse::Ok().json(response)
        }
        Err(error) => minecraft_services_error_response(error, PATH),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/minecraftservices/privileges",
    tag = "yggdrasil",
    operation_id = "minecraft_services_privileges",
    responses(
        (status = 200, description = "Minecraft services privileges policy", body = crate::api::dto::yggdrasil::MinecraftServicesPrivilegesResp),
        (status = 401, description = "Missing or invalid bearer token", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
        (status = 404, description = "Minecraft services anti-feature policy support is disabled", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
    ),
    security(("bearer" = [])),
)]
pub async fn privileges(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    const PATH: &str = "/privileges";
    if !anti_features_enabled(state.get_ref()) {
        return minecraft_services_not_found(PATH);
    }
    let Some(access_token) = bearer_token_or_unauthorized(&req, "minecraft services privileges")
    else {
        return minecraft_services_unauthorized(PATH);
    };

    match yggdrasil_service::minecraft_services_privileges(state.get_ref(), &access_token).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => minecraft_services_error_response(error, PATH),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/minecraftservices/player/attributes",
    tag = "yggdrasil",
    operation_id = "minecraft_services_player_attributes",
    responses(
        (status = 200, description = "Minecraft services player attributes policy", body = crate::api::dto::yggdrasil::MinecraftServicesPlayerAttributesResp),
        (status = 401, description = "Missing or invalid bearer token", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
        (status = 404, description = "Minecraft services anti-feature policy support is disabled", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
    ),
    security(("bearer" = [])),
)]
pub async fn player_attributes(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    const PATH: &str = "/player/attributes";
    if !anti_features_enabled(state.get_ref()) {
        return minecraft_services_not_found(PATH);
    }
    let Some(access_token) =
        bearer_token_or_unauthorized(&req, "minecraft services player attributes")
    else {
        return minecraft_services_unauthorized(PATH);
    };

    match yggdrasil_service::minecraft_services_player_attributes(state.get_ref(), &access_token)
        .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => minecraft_services_error_response(error, PATH),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/minecraftservices/privacy/blocklist",
    tag = "yggdrasil",
    operation_id = "minecraft_services_privacy_blocklist",
    responses(
        (status = 200, description = "Minecraft services privacy blocklist", body = crate::api::dto::yggdrasil::MinecraftServicesPrivacyBlocklistResp),
        (status = 401, description = "Missing or invalid bearer token", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
        (status = 404, description = "Minecraft services anti-feature policy support is disabled", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
    ),
    security(("bearer" = [])),
)]
pub async fn privacy_blocklist(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    const PATH: &str = "/privacy/blocklist";
    if !anti_features_enabled(state.get_ref()) {
        return minecraft_services_not_found(PATH);
    }
    let Some(access_token) =
        bearer_token_or_unauthorized(&req, "minecraft services privacy blocklist")
    else {
        return minecraft_services_unauthorized(PATH);
    };

    match yggdrasil_service::minecraft_services_privacy_blocklist(state.get_ref(), &access_token)
        .await
    {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => minecraft_services_error_response(error, PATH),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/sessionserver/blockedservers",
    tag = "yggdrasil",
    operation_id = "minecraft_services_blocked_servers",
    responses(
        (status = 404, description = "Empty blocked server list or disabled anti-feature policy support", body = crate::api::dto::yggdrasil::MinecraftServicesPathError),
    ),
)]
pub async fn blocked_servers() -> HttpResponse {
    minecraft_services_not_found("/blockedservers")
}

fn anti_features_enabled(state: &AppState) -> bool {
    RuntimeYggdrasilPolicy::from_runtime_config(state.runtime_config()).enable_mojang_anti_features
}

fn bearer_token_or_unauthorized(req: &HttpRequest, context: &str) -> Option<String> {
    let Some(access_token) = crate::api::request_auth::bearer_token(req) else {
        tracing::debug!("{context} rejected missing bearer token");
        return None;
    };
    Some(access_token)
}

fn minecraft_services_error_response(error: YggdrasilError, path: &'static str) -> HttpResponse {
    if error.kind() == YggdrasilErrorKind::Internal {
        return yggdrasil_error_response(error);
    }
    tracing::debug!(
        error = %error.protocol_message(),
        path,
        "minecraft services request rejected invalid bearer token"
    );
    minecraft_services_unauthorized(path)
}

fn minecraft_services_unauthorized(path: &'static str) -> HttpResponse {
    HttpResponse::Unauthorized().json(crate::api::dto::yggdrasil::MinecraftServicesPathError {
        path: path.to_string(),
    })
}

pub fn minecraft_services_not_found(path: &str) -> HttpResponse {
    HttpResponse::NotFound()
        .insert_header(("Cache-Control", "no-store"))
        .json(crate::api::dto::yggdrasil::MinecraftServicesPathError {
            path: path.to_string(),
        })
}

pub async fn minecraft_services_not_found_req(req: HttpRequest) -> HttpResponse {
    let path = minecraft_services_relative_path(req.path());
    minecraft_services_not_found(&path)
}

fn minecraft_services_relative_path(path: &str) -> String {
    let relative = path
        .strip_prefix(crate::config::yggdrasil::DEFAULT_YGGDRASIL_API_ROOT)
        .unwrap_or(path);
    let relative = relative
        .strip_prefix("/minecraftservices")
        .unwrap_or(relative);
    if relative.is_empty() {
        "/".to_string()
    } else {
        relative.to_string()
    }
}
