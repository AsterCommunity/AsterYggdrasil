//! Yggdrasil protocol routes.

pub(crate) mod minecraft_services;
pub(crate) mod texture;

use crate::api::dto::validate_request;
use crate::api::dto::yggdrasil::{
    YggdrasilAuthenticateReq, YggdrasilErrorBody, YggdrasilHasJoinedQuery, YggdrasilJoinReq,
    YggdrasilProfileQuery, YggdrasilRefreshReq, YggdrasilSignoutReq, YggdrasilTokenReq,
};
use crate::runtime::AppState;
use crate::services::{
    audit_service::AuditRequestInfo,
    yggdrasil_service::{self, YggdrasilError, YggdrasilErrorKind},
};
use actix_web::{HttpRequest, HttpResponse, web};

pub use texture::{delete_texture, forwarded_texture_by_hash, texture_by_hash, upload_texture};

const YGGDRASIL_METADATA_CACHE_CONTROL: &str = "no-cache, no-store, must-revalidate";

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(metadata))
        .route("/", web::get().to(metadata))
        .service(
            web::scope("/authserver")
                .route("/authenticate", web::post().to(authenticate))
                .route("/refresh", web::post().to(refresh))
                .route("/validate", web::post().to(validate))
                .route("/invalidate", web::post().to(invalidate))
                .route("/signout", web::post().to(signout)),
        )
        .service(
            web::scope("/sessionserver/session/minecraft")
                .route("/join", web::post().to(join))
                .route("/hasJoined", web::get().to(has_joined))
                .route(
                    "/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}",
                    web::get().to(texture::forwarded_texture_by_hash),
                )
                .route("/profile/{uuid}", web::get().to(profile_by_uuid)),
        )
        .route("/api/profiles/minecraft", web::post().to(profiles_by_names))
        .service(
            web::scope("/minecraftservices")
                .route(
                    "/player/certificates",
                    web::post().to(minecraft_services::player_certificates),
                )
                .route("/privileges", web::get().to(minecraft_services::privileges))
                .route(
                    "/player/attributes",
                    web::get().to(minecraft_services::player_attributes),
                )
                .route(
                    "/privacy/blocklist",
                    web::get().to(minecraft_services::privacy_blocklist),
                )
                .default_service(web::to(
                    minecraft_services::minecraft_services_not_found_req,
                )),
        )
        .route(
            "/sessionserver/blockedservers",
            web::get().to(minecraft_services::blocked_servers),
        )
        .route(
            "/api/user/profile/{uuid}/{texture_type}",
            web::put().to(upload_texture),
        )
        .route(
            "/api/user/profile/{uuid}/{texture_type}",
            web::delete().to(delete_texture),
        )
        .route("/textures/{hash}", web::get().to(texture_by_hash))
        .default_service(web::to(yggdrasil_not_found));
}

async fn yggdrasil_not_found(req: HttpRequest) -> HttpResponse {
    minecraft_services::minecraft_services_not_found_req(req).await
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil",
    tag = "yggdrasil",
    operation_id = "yggdrasil_metadata",
    responses(
        (status = 200, description = "authlib-injector Yggdrasil service metadata", body = crate::api::dto::yggdrasil::YggdrasilMetaResp),
    ),
)]
pub async fn metadata(state: web::Data<AppState>) -> HttpResponse {
    tracing::debug!("serving yggdrasil metadata");
    HttpResponse::Ok()
        .insert_header(("Cache-Control", YGGDRASIL_METADATA_CACHE_CONTROL))
        .json(yggdrasil_service::metadata(state.get_ref()))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/authserver/authenticate",
    tag = "yggdrasil",
    operation_id = "yggdrasil_authenticate",
    request_body = YggdrasilAuthenticateReq,
    responses(
        (status = 200, description = "Yggdrasil access token and available profiles", body = crate::api::dto::yggdrasil::YggdrasilAuthenticateResp),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid credentials or forbidden profile", body = YggdrasilErrorBody),
    ),
)]
pub async fn authenticate(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<YggdrasilAuthenticateReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        username_len = body.username.len(),
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        request_user = body.request_user,
        "received yggdrasil authenticate request"
    );
    if let Some(rejection) = state
        .yggdrasil_rate_limiter()
        .check_authenticate(&body.username)
    {
        tracing::warn!(
            username_hash = %crate::utils::hash::sha256_hex(body.username.trim().to_ascii_lowercase().as_bytes()),
            retry_after_seconds = rejection.retry_after_seconds(),
            "yggdrasil authenticate request rate limited"
        );
        return rejection.into_response();
    }
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil authenticate request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::authenticate(state.get_ref(), body, &req).await {
        Ok(response) => {
            tracing::debug!(
                available_profile_count = response.available_profiles.len(),
                selected_profile = response.selected_profile.is_some(),
                includes_user = response.user.is_some(),
                "yggdrasil authenticate request completed"
            );
            HttpResponse::Ok().json(response)
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/authserver/refresh",
    tag = "yggdrasil",
    operation_id = "yggdrasil_refresh",
    request_body = YggdrasilRefreshReq,
    responses(
        (status = 200, description = "Refreshed Yggdrasil access token", body = crate::api::dto::yggdrasil::YggdrasilRefreshResp),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid or forbidden token", body = YggdrasilErrorBody),
    ),
)]
pub async fn refresh(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<YggdrasilRefreshReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        has_selected_profile = body.selected_profile.is_some(),
        request_user = body.request_user,
        "received yggdrasil refresh request"
    );
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil refresh request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::refresh(state.get_ref(), body, &req).await {
        Ok(response) => {
            tracing::debug!(
                selected_profile = response.selected_profile.is_some(),
                includes_user = response.user.is_some(),
                "yggdrasil refresh request completed"
            );
            HttpResponse::Ok().json(response)
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/authserver/validate",
    tag = "yggdrasil",
    operation_id = "yggdrasil_validate",
    request_body = YggdrasilTokenReq,
    responses(
        (status = 204, description = "Token is valid"),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid token", body = YggdrasilErrorBody),
    ),
)]
pub async fn validate(
    state: web::Data<AppState>,
    body: web::Json<YggdrasilTokenReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        "received yggdrasil validate request"
    );
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil validate request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::validate(state.get_ref(), body).await {
        Ok(()) => {
            tracing::debug!("yggdrasil validate request completed");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/authserver/invalidate",
    tag = "yggdrasil",
    operation_id = "yggdrasil_invalidate",
    request_body = YggdrasilTokenReq,
    responses(
        (status = 204, description = "Token invalidated or already unusable"),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid token", body = YggdrasilErrorBody),
    ),
)]
pub async fn invalidate(
    state: web::Data<AppState>,
    body: web::Json<YggdrasilTokenReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        has_client_token = body
            .client_token
            .as_ref()
            .is_some_and(|token| !token.trim().is_empty()),
        "received yggdrasil invalidate request"
    );
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil invalidate request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::invalidate(state.get_ref(), body).await {
        Ok(()) => {
            tracing::debug!("yggdrasil invalidate request completed");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/authserver/signout",
    tag = "yggdrasil",
    operation_id = "yggdrasil_signout",
    request_body = YggdrasilSignoutReq,
    responses(
        (status = 204, description = "All tokens for the account were revoked"),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid credentials", body = YggdrasilErrorBody),
    ),
)]
pub async fn signout(
    state: web::Data<AppState>,
    body: web::Json<YggdrasilSignoutReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        username_len = body.username.len(),
        "received yggdrasil signout request"
    );
    if let Some(rejection) = state.yggdrasil_rate_limiter().check_signout(&body.username) {
        tracing::warn!(
            username_hash = %crate::utils::hash::sha256_hex(body.username.trim().to_ascii_lowercase().as_bytes()),
            retry_after_seconds = rejection.retry_after_seconds(),
            "yggdrasil signout request rate limited"
        );
        return rejection.into_response();
    }
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil signout request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::signout(state.get_ref(), &body.username, &body.password).await {
        Ok(()) => {
            tracing::debug!("yggdrasil signout request completed");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/api/profiles/minecraft",
    tag = "yggdrasil",
    operation_id = "yggdrasil_profiles_by_names",
    request_body = Vec<String>,
    responses(
        (status = 200, description = "Profiles matching the requested Minecraft names", body = Vec<crate::api::dto::yggdrasil::YggdrasilProfile>),
        (status = 400, description = "Invalid profile name or too many names", body = YggdrasilErrorBody),
    ),
)]
pub async fn profiles_by_names(
    state: web::Data<AppState>,
    body: web::Json<Vec<String>>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        requested_count = body.len(),
        "received yggdrasil profiles by names request"
    );
    if body
        .iter()
        .any(|name| crate::api::dto::validation::validate_minecraft_profile_name(name).is_err())
    {
        tracing::debug!("yggdrasil profiles by names request validation failed");
        return yggdrasil_error_response(YggdrasilError::new(YggdrasilErrorKind::BadRequest));
    }
    match yggdrasil_service::profiles_by_names(state.get_ref(), &body).await {
        Ok(response) => {
            tracing::debug!(
                returned_count = response.len(),
                "yggdrasil profiles by names request completed"
            );
            HttpResponse::Ok().json(response)
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/yggdrasil/sessionserver/session/minecraft/join",
    tag = "yggdrasil",
    operation_id = "yggdrasil_join",
    request_body = YggdrasilJoinReq,
    responses(
        (status = 204, description = "Join server record accepted"),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
        (status = 403, description = "Invalid token or forbidden profile", body = YggdrasilErrorBody),
    ),
)]
pub async fn join(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<YggdrasilJoinReq>,
) -> HttpResponse {
    let body = body.into_inner();
    tracing::debug!(
        selected_profile_uuid = %body.selected_profile,
        server_id_hash = %crate::utils::hash::sha256_hex(body.server_id.as_bytes()),
        "received yggdrasil join request"
    );
    if let Err(error) = validate_request(&body) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil join request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::join(state.get_ref(), body, &req).await {
        Ok(()) => {
            tracing::debug!("yggdrasil join request completed");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/sessionserver/session/minecraft/hasJoined",
    tag = "yggdrasil",
    operation_id = "yggdrasil_has_joined",
    params(
        ("username" = String, Query, description = "Minecraft profile name"),
        ("serverId" = String, Query, description = "Server ID hash from the join request"),
        ("ip" = Option<String>, Query, description = "Optional client IP address"),
    ),
    responses(
        (status = 200, description = "Joined profile", body = crate::api::dto::yggdrasil::YggdrasilProfile),
        (status = 204, description = "No matching join record"),
        (status = 400, description = "Invalid request", body = YggdrasilErrorBody),
    ),
)]
pub async fn has_joined(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    query: web::Query<YggdrasilHasJoinedQuery>,
) -> HttpResponse {
    let query = query.into_inner();
    tracing::debug!(
        username = %query.username,
        server_id_hash = %crate::utils::hash::sha256_hex(query.server_id.as_bytes()),
        has_ip = query.ip.is_some(),
        "received yggdrasil hasJoined request"
    );
    if let Err(error) = validate_request(&query) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil hasJoined request validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::has_joined(
        state.get_ref(),
        &query.username,
        &query.server_id,
        query.ip.as_deref(),
        AuditRequestInfo::from_request(&req),
    )
    .await
    {
        Ok(Some(profile)) => {
            tracing::debug!(
                profile_uuid = %profile.id,
                "yggdrasil hasJoined request matched profile"
            );
            HttpResponse::Ok().json(profile)
        }
        Ok(None) => {
            tracing::debug!("yggdrasil hasJoined request did not match a join record");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}",
    tag = "yggdrasil",
    operation_id = "yggdrasil_profile_by_uuid",
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("unsigned" = Option<bool>, Query, description = "Whether to omit texture signatures"),
    ),
    responses(
        (status = 200, description = "Minecraft profile", body = crate::api::dto::yggdrasil::YggdrasilProfile),
        (status = 204, description = "Profile not found"),
        (status = 400, description = "Invalid UUID or query parameter", body = YggdrasilErrorBody),
    ),
)]
pub async fn profile_by_uuid(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<YggdrasilProfileQuery>,
) -> HttpResponse {
    let uuid = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        unsigned = query.unsigned,
        "received yggdrasil profile by uuid request"
    );
    if let Err(error) = crate::api::dto::validation::validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "yggdrasil profile by uuid path validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message.unwrap_or_default(),
        ));
    }
    let query = query.into_inner();
    if let Err(error) = validate_request(&query) {
        tracing::debug!(
            message = %error.message(),
            "yggdrasil profile by uuid query validation failed"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message(),
        ));
    }
    match yggdrasil_service::profile_by_uuid(state.get_ref(), &uuid, query.unsigned.unwrap_or(true))
        .await
    {
        Ok(Some(profile)) => {
            tracing::debug!(
                profile_uuid = %profile.id,
                property_count = profile.properties.as_ref().map(Vec::len).unwrap_or(0),
                "yggdrasil profile by uuid request completed"
            );
            HttpResponse::Ok().json(profile)
        }
        Ok(None) => {
            tracing::debug!(profile_uuid = %uuid, "yggdrasil profile by uuid request not found");
            HttpResponse::NoContent().finish()
        }
        Err(error) => yggdrasil_error_response(error),
    }
}

fn yggdrasil_error_response(error: YggdrasilError) -> HttpResponse {
    let status = error.status_code();
    if status == actix_web::http::StatusCode::NO_CONTENT {
        return HttpResponse::NoContent().finish();
    }
    if status.is_server_error() {
        tracing::error!(kind = ?error.kind(), "yggdrasil request failed");
    } else {
        tracing::warn!(kind = ?error.kind(), "yggdrasil request failed");
    }
    HttpResponse::build(status).json(YggdrasilErrorBody {
        error: error.protocol_error_name(),
        error_message: error.protocol_message(),
        cause: None,
    })
}
