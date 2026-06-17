//! Authentication routes.

mod cookies;

use crate::api::cache::conditional_bytes_response;
use crate::api::dto::{
    CheckResp, LoginReq, LogoutReq, LogoutResp, PasskeyLoginFinishReq, PasskeyLoginStartReq,
    PasskeyRegisterFinishReq, PasskeyRegisterStartReq, PatchPasskeyReq, RefreshReq, RegisterReq,
    RemovedCountResponse, SetupReq, UpdateAvatarSourceReq, UpdateProfileReq, validate_request,
};
use crate::api::middleware::csrf::{self, RequestSourceMode};
use crate::api::pagination::LimitOffsetQuery;
#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::pagination::OffsetPage;
use crate::api::request_auth::access_cookie_token;
use crate::api::response::ApiResponse;
use crate::config::auth_runtime::RuntimeAuthPolicy;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, passkey_service, profile_service};
use crate::utils::numbers::u64_to_i64;
use actix_multipart::Multipart;
use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, web};
use serde::Serialize;

use self::cookies::{
    REFRESH_COOKIE, build_access_cookie, build_csrf_cookie, build_refresh_cookie,
    clear_access_cookie, clear_csrf_cookie, clear_refresh_cookie,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/check", web::get().to(check))
            .route("/setup", web::post().to(setup))
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh))
            .route("/logout", web::post().to(logout))
            .route("/me", web::get().to(me))
            .route("/profile", web::patch().to(patch_profile))
            .route("/profile/avatar/upload", web::post().to(upload_avatar))
            .route("/profile/avatar/source", web::put().to(put_avatar_source))
            .route("/profile/avatar/{size}", web::get().to(get_self_avatar))
            .service(
                web::scope("/sessions")
                    .route("", web::get().to(sessions))
                    .route("/others", web::delete().to(delete_other_sessions))
                    .route("/{id}", web::delete().to(delete_session)),
            )
            .service(
                web::scope("/passkeys")
                    .route("", web::get().to(list_passkeys))
                    .route(
                        "/register/start",
                        web::post().to(start_passkey_registration),
                    )
                    .route(
                        "/register/finish",
                        web::post().to(finish_passkey_registration),
                    )
                    .route("/login/start", web::post().to(start_passkey_login))
                    .route("/login/finish", web::post().to(finish_passkey_login))
                    .route("/{id}", web::patch().to(rename_passkey))
                    .route("/{id}", web::delete().to(delete_passkey)),
            ),
    );
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/auth/check",
    tag = "auth",
    operation_id = "check_auth_state",
    responses(
        (status = 200, description = "Authentication bootstrap state", body = inline(ApiResponse<CheckResp>)),
    ),
)]
pub async fn check(state: web::Data<AppState>) -> Result<HttpResponse> {
    tracing::debug!("auth check request received");
    let initialized =
        crate::db::repository::user_repo::count_all(state.get_ref().reader_db()).await? > 0;
    tracing::debug!(initialized, "auth check request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(CheckResp { initialized })))
}

pub(super) fn authenticated_response_with_body<T: Serialize>(
    state: &AppState,
    session: auth_service::AuthTokenBundle,
    body: T,
) -> Result<HttpResponse> {
    let auth_policy = RuntimeAuthPolicy::from_runtime_config(state.runtime_config());
    let secure = auth_policy.cookie_secure;
    let csrf_token = csrf::build_csrf_token();
    let access_ttl = u64_to_i64(auth_policy.access_token_ttl_secs, "access token ttl")?;
    let refresh_ttl = u64_to_i64(auth_policy.refresh_token_ttl_secs, "refresh token ttl")?;

    Ok(HttpResponse::Ok()
        .cookie(build_access_cookie(
            &session.access_token,
            access_ttl,
            secure,
        ))
        .cookie(build_refresh_cookie(
            &session.refresh_token,
            refresh_ttl,
            secure,
        ))
        .cookie(build_csrf_cookie(&csrf_token, refresh_ttl, secure))
        .json(ApiResponse::ok(body)))
}

fn authenticated_response(
    state: &AppState,
    session: auth_service::AuthTokenBundle,
) -> Result<HttpResponse> {
    let response = session.response();
    authenticated_response_with_body(state, session, response)
}

pub(super) fn authenticated_redirect_response(
    state: &AppState,
    session: auth_service::AuthTokenBundle,
    location: String,
) -> Result<HttpResponse> {
    let auth_policy = RuntimeAuthPolicy::from_runtime_config(state.runtime_config());
    let secure = auth_policy.cookie_secure;
    let csrf_token = csrf::build_csrf_token();
    let access_ttl = u64_to_i64(auth_policy.access_token_ttl_secs, "access token ttl")?;
    let refresh_ttl = u64_to_i64(auth_policy.refresh_token_ttl_secs, "refresh token ttl")?;

    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, location))
        .cookie(build_access_cookie(
            &session.access_token,
            access_ttl,
            secure,
        ))
        .cookie(build_refresh_cookie(
            &session.refresh_token,
            refresh_ttl,
            secure,
        ))
        .cookie(build_csrf_cookie(&csrf_token, refresh_ttl, secure))
        .finish())
}

fn refresh_token_from_request(
    req: &HttpRequest,
    body: Option<&web::Json<RefreshReq>>,
) -> Result<String> {
    req.cookie(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .or_else(|| body.map(|body| body.refresh_token.clone()))
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| AsterError::auth_token_invalid("missing refresh token"))
}

fn logout_token_from_request(
    req: &HttpRequest,
    body: Option<&web::Json<LogoutReq>>,
) -> Option<String> {
    req.cookie(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .or_else(|| body.map(|body| body.refresh_token.clone()))
        .filter(|token| !token.trim().is_empty())
}

fn ensure_cookie_write_allowed(state: &AppState, req: &HttpRequest) -> Result<()> {
    csrf::ensure_request_source_allowed(
        req,
        state.runtime_config(),
        RequestSourceMode::OptionalWhenPresent,
    )?;
    csrf::ensure_double_submit_token(req)
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/setup",
    tag = "auth",
    operation_id = "setup_first_admin",
    request_body = SetupReq,
    responses(
        (status = 200, description = "First admin account created and session cookies issued", body = inline(ApiResponse<auth_service::AuthTokenResponse>)),
        (status = 400, description = "System is already initialized or input is invalid"),
    ),
)]
pub async fn setup(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<SetupReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        username_len = body.username.len(),
        has_public_site_url = body.public_site_url.is_some(),
        "auth setup request received"
    );
    validate_request(&*body)?;
    let data = auth_service::setup_first_admin(
        state.get_ref(),
        &body.username,
        &body.email,
        &body.password,
        body.public_site_url.as_deref(),
        &req,
    )
    .await?;
    tracing::debug!(user_id = data.user.id, "auth setup request completed");
    authenticated_response(state.get_ref(), data)
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    operation_id = "register",
    request_body = RegisterReq,
    responses(
        (status = 200, description = "User account created and session cookies issued", body = inline(ApiResponse<auth_service::AuthTokenResponse>)),
        (status = 400, description = "Input is invalid"),
        (status = 403, description = "Registration is disabled"),
    ),
)]
pub async fn register(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RegisterReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        username_len = body.username.len(),
        "auth register request received"
    );
    validate_request(&*body)?;
    let data = auth_service::register(
        state.get_ref(),
        &body.username,
        &body.email,
        &body.password,
        &req,
    )
    .await?;
    tracing::debug!(user_id = data.user.id, "auth register request completed");
    authenticated_response(state.get_ref(), data)
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    operation_id = "login",
    request_body = LoginReq,
    responses(
        (status = 200, description = "Session cookies issued", body = inline(ApiResponse<auth_service::AuthTokenResponse>)),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "User is disabled"),
    ),
)]
pub async fn login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<LoginReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        identifier_len = body.identifier.len(),
        identifier_has_at = body.identifier.contains('@'),
        "auth login request received"
    );
    validate_request(&*body)?;
    let data = auth_service::login(state.get_ref(), &body.identifier, &body.password, &req).await?;
    tracing::debug!(user_id = data.user.id, "auth login request completed");
    authenticated_response(state.get_ref(), data)
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    operation_id = "refresh_token",
    responses(
        (status = 200, description = "Fresh session cookies issued", body = inline(ApiResponse<auth_service::AuthTokenResponse>)),
        (status = 401, description = "Refresh token is invalid, expired, or stale"),
    ),
)]
pub async fn refresh(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: Option<web::Json<RefreshReq>>,
) -> Result<HttpResponse> {
    tracing::debug!(
        has_json_body = body.is_some(),
        has_refresh_cookie = req.cookie(REFRESH_COOKIE).is_some(),
        "auth refresh request received"
    );
    if let Some(body) = body.as_ref() {
        validate_request(&**body)?;
    }
    if req.cookie(REFRESH_COOKIE).is_some() {
        ensure_cookie_write_allowed(state.get_ref(), &req)?;
    }
    let refresh_token = refresh_token_from_request(&req, body.as_ref())?;
    let data = auth_service::refresh(state.get_ref(), &refresh_token, &req).await?;
    tracing::debug!(user_id = data.user.id, "auth refresh request completed");
    authenticated_response(state.get_ref(), data)
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    operation_id = "logout",
    responses(
        (status = 200, description = "Refresh token revocation result and auth cookies cleared", body = inline(ApiResponse<LogoutResp>)),
    ),
)]
pub async fn logout(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: Option<web::Json<LogoutReq>>,
) -> Result<HttpResponse> {
    tracing::debug!(
        has_json_body = body.is_some(),
        has_access_cookie = access_cookie_token(&req).is_some(),
        has_refresh_cookie = req.cookie(REFRESH_COOKIE).is_some(),
        "auth logout request received"
    );
    if let Some(body) = body.as_ref() {
        validate_request(&**body)?;
    }
    if access_cookie_token(&req).is_some() || req.cookie(REFRESH_COOKIE).is_some() {
        ensure_cookie_write_allowed(state.get_ref(), &req)?;
    }

    let revoked = if let Some(refresh_token) = logout_token_from_request(&req, body.as_ref()) {
        auth_service::logout(state.get_ref(), &refresh_token, &req).await?
    } else {
        false
    };
    let secure =
        RuntimeAuthPolicy::from_runtime_config(state.get_ref().runtime_config()).cookie_secure;
    tracing::debug!(revoked, "auth logout request completed");
    Ok(HttpResponse::Ok()
        .cookie(clear_access_cookie(secure))
        .cookie(clear_refresh_cookie(secure))
        .cookie(clear_csrf_cookie(secure))
        .json(ApiResponse::ok(LogoutResp { revoked })))
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    operation_id = "get_current_user",
    responses(
        (status = 200, description = "Current authenticated user", body = inline(ApiResponse<auth_service::AuthUserInfo>)),
        (status = 401, description = "Missing or invalid access token"),
        (status = 403, description = "User is disabled"),
    ),
    security(("bearer" = [])),
)]
pub async fn me(state: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse> {
    tracing::debug!(
        has_access_cookie = access_cookie_token(&req).is_some(),
        has_authorization_header = req.headers().get(header::AUTHORIZATION).is_some(),
        "auth me request received"
    );
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let info = auth_service::auth_user_info(state.get_ref(), user).await?;
    tracing::debug!(user_id = info.id, "auth me request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(info)))
}

#[api_docs_macros::path(
    patch,
    path = "/api/v1/auth/profile",
    tag = "auth",
    operation_id = "update_profile",
    request_body = UpdateProfileReq,
    responses(
        (status = 200, description = "Profile updated", body = inline(ApiResponse<profile_service::UserProfileInfo>)),
        (status = 400, description = "Invalid profile input"),
        (status = 401, description = "Missing or invalid access token"),
    ),
    security(("bearer" = [])),
)]
pub async fn patch_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpdateProfileReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        display_name_changed = body.display_name.is_some(),
        "auth profile patch request received"
    );
    validate_request(&*body)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    if access_cookie_token(&req).is_some() {
        ensure_cookie_write_allowed(state.get_ref(), &req)?;
    }
    let profile =
        profile_service::update_profile(state.get_ref(), user.id, body.display_name.clone())
            .await?;
    tracing::debug!(user_id = user.id, "auth profile patch request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(profile)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/profile/avatar/upload",
    tag = "auth",
    operation_id = "upload_avatar",
    request_body(content = String, content_type = "multipart/form-data", description = "Avatar image to upload"),
    responses(
        (status = 200, description = "Avatar uploaded", body = inline(ApiResponse<profile_service::UserProfileInfo>)),
        (status = 400, description = "Invalid image upload"),
        (status = 401, description = "Missing or invalid access token"),
    ),
    security(("bearer" = [])),
)]
pub async fn upload_avatar(
    state: web::Data<AppState>,
    req: HttpRequest,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(user_id = user.id, "auth avatar upload request received");
    if access_cookie_token(&req).is_some() {
        ensure_cookie_write_allowed(state.get_ref(), &req)?;
    }
    let profile = profile_service::upload_avatar(state.get_ref(), user.id, &mut payload).await?;
    tracing::debug!(user_id = user.id, "auth avatar upload request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(profile)))
}

#[api_docs_macros::path(
    put,
    path = "/api/v1/auth/profile/avatar/source",
    tag = "auth",
    operation_id = "set_avatar_source",
    request_body = UpdateAvatarSourceReq,
    responses(
        (status = 200, description = "Avatar source updated", body = inline(ApiResponse<profile_service::UserProfileInfo>)),
        (status = 400, description = "Invalid avatar source"),
        (status = 401, description = "Missing or invalid access token"),
    ),
    security(("bearer" = [])),
)]
pub async fn put_avatar_source(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpdateAvatarSourceReq>,
) -> Result<HttpResponse> {
    tracing::debug!(source = ?body.source, "auth avatar source request received");
    validate_request(&*body)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    if access_cookie_token(&req).is_some() {
        ensure_cookie_write_allowed(state.get_ref(), &req)?;
    }
    let profile = profile_service::set_avatar_source(state.get_ref(), user.id, body.source).await?;
    tracing::debug!(user_id = user.id, source = ?body.source, "auth avatar source request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(profile)))
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/auth/profile/avatar/{size}",
    tag = "auth",
    operation_id = "get_self_avatar",
    params(("size" = u32, Path, description = "Avatar size (512 or 1024)")),
    responses(
        (status = 200, description = "Avatar image (WebP)"),
        (status = 401, description = "Missing or invalid access token"),
        (status = 404, description = "Avatar not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_self_avatar(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<u32>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(
        user_id = user.id,
        size = *path,
        "auth self avatar request received"
    );
    let bytes = profile_service::get_avatar_bytes(state.get_ref(), user.id, *path).await?;
    tracing::debug!(
        user_id = user.id,
        size = *path,
        bytes = bytes.len(),
        "auth self avatar request completed"
    );
    Ok(conditional_bytes_response(
        &req,
        bytes,
        profile_service::AVATAR_CONTENT_TYPE,
        profile_service::AVATAR_CACHE_CONTROL,
    ))
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/auth/sessions",
    tag = "auth",
    operation_id = "list_auth_sessions",
    params(LimitOffsetQuery),
    responses(
        (status = 200, description = "Current user's sessions", body = inline(ApiResponse<OffsetPage<auth_service::AuthSessionInfo>>)),
        (status = 401, description = "Missing or invalid access token"),
        (status = 403, description = "User is disabled"),
    ),
    security(("bearer" = [])),
)]
pub async fn sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitOffsetQuery>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let limit = page.limit_or(50, 100);
    let offset = page.offset();
    tracing::debug!(
        user_id = user.id,
        limit,
        offset,
        "auth sessions list request received"
    );
    let refresh_token = req
        .cookie(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string());
    let sessions = auth_service::list_sessions_paginated(
        state.get_ref(),
        user.id,
        refresh_token.as_deref(),
        limit,
        offset,
    )
    .await?;
    tracing::debug!(
        user_id = user.id,
        returned = sessions.items.len(),
        total = sessions.total,
        "auth sessions list request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(sessions)))
}

#[api_docs_macros::path(
    delete,
    path = "/api/v1/auth/sessions/others",
    tag = "auth",
    operation_id = "revoke_other_auth_sessions",
    responses(
        (status = 200, description = "Other login devices revoked", body = inline(ApiResponse<RemovedCountResponse>)),
        (status = 401, description = "Missing or invalid access token"),
        (status = 403, description = "User is disabled"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_other_sessions(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(
        user_id = user.id,
        "auth revoke other sessions request received"
    );
    ensure_cookie_write_allowed(state.get_ref(), &req)?;
    let refresh_token = req
        .cookie(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| AsterError::auth_token_invalid("missing current refresh session"))?;
    let removed =
        auth_service::revoke_other_sessions(state.get_ref(), user.id, &refresh_token, &req).await?;
    tracing::debug!(
        user_id = user.id,
        removed,
        "auth revoke other sessions request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(RemovedCountResponse { removed })))
}

#[api_docs_macros::path(
    delete,
    path = "/api/v1/auth/sessions/{id}",
    tag = "auth",
    operation_id = "revoke_auth_session",
    params(("id" = String, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Login device revoked"),
        (status = 401, description = "Missing or invalid access token"),
        (status = 403, description = "User is disabled"),
        (status = 404, description = "Session not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_session(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let session_id = path.into_inner();
    tracing::debug!(
        user_id = user.id,
        session_id,
        "auth revoke session request received"
    );
    ensure_cookie_write_allowed(state.get_ref(), &req)?;
    let refresh_token = req
        .cookie(REFRESH_COOKIE)
        .map(|cookie| cookie.value().to_string());
    let revoked_current = auth_service::revoke_session(
        state.get_ref(),
        user.id,
        session_id.as_str(),
        refresh_token.as_deref(),
        &req,
    )
    .await?;

    let secure =
        RuntimeAuthPolicy::from_runtime_config(state.get_ref().runtime_config()).cookie_secure;
    let mut response = HttpResponse::Ok();
    if revoked_current {
        response
            .cookie(clear_access_cookie(secure))
            .cookie(clear_refresh_cookie(secure))
            .cookie(clear_csrf_cookie(secure));
    }
    tracing::debug!(
        user_id = user.id,
        session_id,
        revoked_current,
        "auth revoke session request completed"
    );
    Ok(response.json(ApiResponse::<()>::ok_empty()))
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/auth/passkeys",
    tag = "auth",
    operation_id = "list_passkeys",
    params(LimitOffsetQuery),
    responses(
        (status = 200, description = "Registered passkeys for current user", body = inline(ApiResponse<OffsetPage<passkey_service::PasskeyInfo>>)),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_passkeys(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitOffsetQuery>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let limit = page.limit_or(20, 100);
    let offset = page.offset();
    tracing::debug!(
        user_id = user.id,
        limit,
        offset,
        "auth passkey list request received"
    );
    let items =
        passkey_service::list_passkeys_paginated(state.get_ref(), user.id, limit, offset).await?;
    tracing::debug!(
        user_id = user.id,
        returned = items.items.len(),
        total = items.total,
        "auth passkey list request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(items)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/passkeys/register/start",
    tag = "auth",
    operation_id = "start_passkey_registration",
    request_body = PasskeyRegisterStartReq,
    responses(
        (status = 200, description = "Passkey registration challenge", body = inline(ApiResponse<passkey_service::PasskeyRegisterStartResp>)),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer" = [])),
)]
pub async fn start_passkey_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PasskeyRegisterStartReq>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(
        user_id = user.id,
        has_name = body.name.is_some(),
        "auth passkey registration start request received"
    );
    let resp =
        passkey_service::start_registration(state.get_ref(), user.id, body.name.as_deref()).await?;
    tracing::debug!(user_id = user.id, flow_id = %resp.flow_id, "auth passkey registration start request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(resp)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/passkeys/register/finish",
    tag = "auth",
    operation_id = "finish_passkey_registration",
    request_body = PasskeyRegisterFinishReq,
    responses(
        (status = 200, description = "Passkey registered", body = inline(ApiResponse<passkey_service::PasskeyInfo>)),
        (status = 400, description = "Invalid passkey registration"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer" = [])),
)]
pub async fn finish_passkey_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PasskeyRegisterFinishReq>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(user_id = user.id, flow_id = %body.flow_id, has_name = body.name.is_some(), "auth passkey registration finish request received");
    ensure_cookie_write_allowed(state.get_ref(), &req)?;
    let passkey = passkey_service::finish_registration(
        state.get_ref(),
        user.id,
        &body.flow_id,
        body.credential.clone(),
        body.name.as_deref(),
    )
    .await?;
    let audit_ctx = audit_service::AuditContext::from_request(&req, user.id);
    let details = passkey_info_audit_details(&passkey);
    audit_service::log_with_details(
        state.get_ref(),
        &audit_ctx,
        audit_service::AuditAction::UserPasskeyRegister,
        audit_service::AuditEntityType::Passkey,
        Some(passkey.id),
        Some(&passkey.name),
        || details.clone(),
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        passkey_id = passkey.id,
        "auth passkey registration finish request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(passkey)))
}

#[api_docs_macros::path(
    patch,
    path = "/api/v1/auth/passkeys/{id}",
    tag = "auth",
    operation_id = "rename_passkey",
    params(("id" = i64, Path, description = "Passkey ID")),
    request_body = PatchPasskeyReq,
    responses(
        (status = 200, description = "Passkey renamed", body = inline(ApiResponse<passkey_service::PasskeyInfo>)),
        (status = 400, description = "Invalid passkey name"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Passkey not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn rename_passkey(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<PatchPasskeyReq>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    ensure_cookie_write_allowed(state.get_ref(), &req)?;
    let id = path.into_inner();
    tracing::debug!(
        user_id = user.id,
        passkey_id = id,
        "auth passkey rename request received"
    );
    let previous =
        crate::db::repository::passkey_repo::find_by_id_for_user(state.writer_db(), id, user.id)
            .await?
            .ok_or_else(|| AsterError::record_not_found(format!("passkey #{id}")))?;
    let passkey = passkey_service::rename_passkey(state.get_ref(), user.id, id, &body.name).await?;
    let audit_ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &audit_ctx,
        audit_service::AuditAction::UserPasskeyRename,
        audit_service::AuditEntityType::Passkey,
        Some(passkey.id),
        Some(&passkey.name),
        || {
            audit_service::details(audit_service::PasskeyAuditDetails {
                passkey_id: passkey.id,
                name: Some(&passkey.name),
                previous_name: Some(&previous.name),
                next_name: Some(&passkey.name),
                backup_eligible: Some(passkey.backup_eligible),
                backed_up: Some(passkey.backed_up),
                sign_count: None,
                last_used_at: None,
            })
        },
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        passkey_id = passkey.id,
        "auth passkey rename request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(passkey)))
}

#[api_docs_macros::path(
    delete,
    path = "/api/v1/auth/passkeys/{id}",
    tag = "auth",
    operation_id = "delete_passkey",
    params(("id" = i64, Path, description = "Passkey ID")),
    responses(
        (status = 200, description = "Passkey deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Passkey not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_passkey(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    ensure_cookie_write_allowed(state.get_ref(), &req)?;
    let id = path.into_inner();
    tracing::debug!(
        user_id = user.id,
        passkey_id = id,
        "auth passkey delete request received"
    );
    let passkey =
        crate::db::repository::passkey_repo::find_by_id_for_user(state.writer_db(), id, user.id)
            .await?
            .ok_or_else(|| AsterError::record_not_found(format!("passkey #{id}")))?;
    let passkey_name = passkey.name.clone();
    if !passkey_service::delete_passkey(state.get_ref(), user.id, id).await? {
        return Err(AsterError::record_not_found(format!("passkey #{id}")));
    }
    let audit_ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &audit_ctx,
        audit_service::AuditAction::UserPasskeyDelete,
        audit_service::AuditEntityType::Passkey,
        Some(id),
        Some(&passkey_name),
        || {
            audit_service::details(audit_service::PasskeyAuditDetails {
                passkey_id: passkey.id,
                name: Some(&passkey.name),
                previous_name: None,
                next_name: None,
                backup_eligible: Some(passkey.backup_eligible),
                backed_up: Some(passkey.backed_up),
                sign_count: None,
                last_used_at: passkey.last_used_at,
            })
        },
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        passkey_id = id,
        "auth passkey delete request completed"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/passkeys/login/start",
    tag = "auth",
    operation_id = "start_passkey_login",
    request_body = PasskeyLoginStartReq,
    responses(
        (status = 200, description = "Passkey login challenge", body = inline(ApiResponse<passkey_service::PasskeyLoginStartResp>)),
        (status = 401, description = "Invalid credentials"),
    ),
)]
pub async fn start_passkey_login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PasskeyLoginStartReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        has_identifier = body.identifier.is_some(),
        conditional = body.conditional.unwrap_or(false),
        "auth passkey login start request received"
    );
    csrf::ensure_request_source_allowed(
        &req,
        state.get_ref().runtime_config(),
        RequestSourceMode::Required,
    )?;
    let resp = passkey_service::start_login(
        state.get_ref(),
        body.identifier.as_deref(),
        body.conditional.unwrap_or(false),
    )
    .await?;
    tracing::debug!(flow_id = %resp.flow_id, "auth passkey login start request completed");
    Ok(HttpResponse::Ok().json(ApiResponse::ok(resp)))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/auth/passkeys/login/finish",
    tag = "auth",
    operation_id = "finish_passkey_login",
    request_body = PasskeyLoginFinishReq,
    responses(
        (status = 200, description = "Passkey login successful, tokens set in HttpOnly cookies", body = inline(ApiResponse<auth_service::AuthTokenResponse>)),
        (status = 401, description = "Invalid credentials"),
    ),
)]
pub async fn finish_passkey_login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PasskeyLoginFinishReq>,
) -> Result<HttpResponse> {
    tracing::debug!(flow_id = %body.flow_id, "auth passkey login finish request received");
    csrf::ensure_request_source_allowed(
        &req,
        state.get_ref().runtime_config(),
        RequestSourceMode::OptionalWhenPresent,
    )?;
    let result = passkey_service::finish_login(
        state.get_ref(),
        &body.flow_id,
        body.credential.clone(),
        &req,
    )
    .await?;
    let audit_ctx = audit_service::AuditContext::from_request(&req, result.session.user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &audit_ctx,
        audit_service::AuditAction::UserPasskeyLogin,
        audit_service::AuditEntityType::Passkey,
        Some(result.passkey_id),
        Some(&result.passkey_name),
        || {
            audit_service::details(audit_service::PasskeyAuditDetails {
                passkey_id: result.passkey_id,
                name: Some(&result.passkey_name),
                previous_name: None,
                next_name: None,
                backup_eligible: None,
                backed_up: None,
                sign_count: None,
                last_used_at: None,
            })
        },
    )
    .await;

    tracing::debug!(
        user_id = result.session.user.id,
        passkey_id = result.passkey_id,
        "auth passkey login finish request completed"
    );
    authenticated_response(state.get_ref(), result.session)
}

fn passkey_info_audit_details(passkey: &passkey_service::PasskeyInfo) -> Option<serde_json::Value> {
    audit_service::details(audit_service::PasskeyAuditDetails {
        passkey_id: passkey.id,
        name: Some(&passkey.name),
        previous_name: None,
        next_name: None,
        backup_eligible: Some(passkey.backup_eligible),
        backed_up: Some(passkey.backed_up),
        sign_count: Some(passkey.sign_count),
        last_used_at: passkey.last_used_at,
    })
}
