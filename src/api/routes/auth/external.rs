//! External authentication routes under `/auth/external-auth`.

use crate::api::dto::{ExternalAuthCallbackQuery, StartExternalAuthReq, validate_request};
use crate::api::error_code::AsterErrorCode;
use crate::api::middleware::auth::JwtAuth;
use crate::api::middleware::csrf;
use crate::api::response::ApiResponse;
use crate::config::site_url;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth::local::AuthUserInfo;
use crate::services::{auth::external, auth::local};
use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, web};
use aster_forge_api::{
    CreatedAtCursorQuery, LimitQuery, parse_datetime_id_cursor, parse_string_id_cursor,
};
#[cfg(all(debug_assertions, feature = "openapi"))]
use aster_forge_api::{CursorPage, DateTimeIdCursor, StringIdCursor};
use aster_forge_middleware::actix::csrf::RequestSourceMode;
use aster_yggdrasil_model::types::external_auth::ExternalAuthProviderKind;
use serde::{Deserialize, Serialize};

const AUTH_REDIRECT_PARAM: &str = "auth_redirect";
const AUTH_REDIRECT_LOGIN_SUCCESS: &str = "login_success";

/// Configures the external identity endpoints under `/auth/external-auth`:
/// provider discovery, browser login flows, email verification, and
/// link management for the signed-in user.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth/external-auth")
            .route("/providers", web::get().to(list_providers))
            .route("/{kind}/providers", web::get().to(list_providers_by_kind))
            .route(
                "/email-verification/start",
                web::post().to(start_email_verification),
            )
            .route(
                "/email-verification/confirm",
                web::get().to(confirm_email_verification),
            )
            .route("/password-link", web::post().to(link_with_password))
            .service(
                web::scope("/links")
                    .wrap(JwtAuth)
                    .route("", web::get().to(list_links))
                    .route("/{id}", web::delete().to(delete_link)),
            )
            .route("/{kind}/{provider}/start", web::post().to(start_login))
            .route("/{kind}/{provider}/callback", web::get().to(finish_login)),
    );
}

fn parse_kind(value: &str) -> Result<ExternalAuthProviderKind> {
    ExternalAuthProviderKind::parse(value).ok_or_else(|| {
        AsterError::record_not_found(format!("external auth provider kind '{value}'"))
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
/// Cursor query for paginating public external auth providers.
pub struct ExternalAuthProviderCursorQuery {
    /// Resume listing after providers ordered at or after this display name.
    pub after_display_name: Option<String>,
    /// Tiebreaker provider ID for entries sharing the same display name.
    pub after_id: Option<i64>,
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/auth/external-auth/providers",
    tag = "external-auth",
    operation_id = "auth_external_auth_list_providers",
    params(LimitQuery, ExternalAuthProviderCursorQuery),
    responses(
        (status = 200, description = "Enabled external auth providers", body = inline(ApiResponse<CursorPage<external::ExternalAuthPublicProvider, StringIdCursor>>)),
    ),
)]
/// Lists enabled external auth providers so unauthenticated clients can
/// discover the available login options.
pub async fn list_providers(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    cursor: web::Query<ExternalAuthProviderCursorQuery>,
) -> Result<HttpResponse> {
    let after = parse_string_id_cursor(
        cursor.after_display_name.clone(),
        cursor.after_id,
        "external auth provider",
    )?;
    let providers =
        external::list_public_providers_paginated(state.get_ref(), page.limit_or(20, 100), after)
            .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(providers)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/auth/external-auth/{kind}/providers",
    tag = "external-auth",
    operation_id = "auth_external_auth_list_providers_by_kind",
    params(("kind" = ExternalAuthProviderKind, Path, description = "External auth provider kind"), LimitQuery, ExternalAuthProviderCursorQuery),
    responses(
        (status = 200, description = "Enabled external auth providers for kind", body = inline(ApiResponse<CursorPage<external::ExternalAuthPublicProvider, StringIdCursor>>)),
        (status = 404, description = "Provider kind not found"),
    ),
)]
/// Lists enabled external auth providers filtered to one provider kind.
pub async fn list_providers_by_kind(
    state: web::Data<AppState>,
    path: web::Path<String>,
    page: web::Query<LimitQuery>,
    cursor: web::Query<ExternalAuthProviderCursorQuery>,
) -> Result<HttpResponse> {
    let kind = parse_kind(&path)?;
    let after = parse_string_id_cursor(
        cursor.after_display_name.clone(),
        cursor.after_id,
        "external auth provider",
    )?;
    let providers = external::list_public_providers_by_kind_paginated(
        state.get_ref(),
        kind,
        page.limit_or(20, 100),
        after,
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(providers)))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/auth/external-auth/{kind}/{provider}/start",
    tag = "external-auth",
    operation_id = "auth_external_auth_start_login",
    params(
        ("kind" = ExternalAuthProviderKind, Path, description = "External auth provider kind"),
        ("provider" = String, Path, description = "External auth provider slug"),
    ),
    request_body = StartExternalAuthReq,
    responses(
        (status = 200, description = "External auth authorization start response", body = inline(ApiResponse<external::ExternalAuthStartLoginResponse>)),
        (status = 400, description = "Provider is misconfigured or request is invalid"),
        (status = 404, description = "Provider not found"),
    ),
)]
/// Starts an external login for the given provider, returning the provider's
/// authorization target with the flow state carried through the callback.
pub async fn start_login(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<StartExternalAuthReq>,
) -> Result<HttpResponse> {
    validate_request(&*body)?;
    let (kind, provider) = path.into_inner();
    let kind = parse_kind(&kind)?;
    ensure_provider_kind(state.get_ref(), kind, &provider).await?;
    let data = external::start_login(
        state.get_ref(),
        &req,
        kind,
        &provider,
        body.return_path.as_deref(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(data)))
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
/// Response body describing the session state after an external-auth flow
/// completes without a browser redirect.
pub struct ExternalAuthFinishLoginResponse {
    /// Login outcome, e.g. `authenticated` or `password_change_required`.
    pub status: &'static str,
    /// Access token lifetime in seconds, when a session was established.
    pub expires_in: Option<u64>,
    /// Pending email-verification flow token, when verification is required.
    pub flow_token: Option<String>,
    /// Frontend path to return the user to once login completes.
    pub return_path: Option<String>,
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/auth/external-auth/{kind}/{provider}/callback",
    tag = "external-auth",
    operation_id = "auth_external_auth_finish_login",
    params(
        ("kind" = ExternalAuthProviderKind, Path, description = "External auth provider kind"),
        ("provider" = String, Path, description = "External auth provider slug"),
        ExternalAuthCallbackQuery,
    ),
    responses(
        (status = 302, description = "External auth callback completed and redirected"),
    ),
)]
/// Completes an external login from the provider callback: issues session
/// cookies on success or redirects into email verification / a status-bearing
/// login URL when the flow requires it.
pub async fn finish_login(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    query: web::Query<ExternalAuthCallbackQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let (kind, provider) = path.into_inner();
    let kind = match parse_kind(&kind) {
        Ok(kind) => kind,
        Err(error) => {
            return Ok(external_auth_error_redirect_response(
                state.get_ref(),
                &error,
            ));
        }
    };
    if let Err(error) = ensure_provider_kind(state.get_ref(), kind, &provider).await {
        return Ok(external_auth_error_redirect_response(
            state.get_ref(),
            &error,
        ));
    }
    let query = external::ExternalAuthCallbackQuery {
        code: query.code.clone(),
        state: query.state.clone(),
        error: query.error.clone(),
        error_description: query.error_description.clone(),
    };
    let outcome =
        match external::finish_callback(state.get_ref(), kind, &provider, &query, None, None).await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                return Ok(external_auth_error_redirect_response(
                    state.get_ref(),
                    &error,
                ));
            }
        };
    match outcome {
        external::ExternalAuthCallbackOutcome::Login(result) => {
            let session =
                local::issue_tokens_for_user(state.get_ref(), result.primary_login.user, &req)
                    .await?;
            let redirect_path = if matches!(
                session.status,
                local::AuthTokenStatus::PasswordChangeRequired
            ) {
                "/force-password-change"
            } else {
                &result.primary_login.return_path
            };
            let redirect_url = add_auth_redirect_status(
                site_url::public_app_url_or_path(state.get_ref().runtime_config(), redirect_path),
                AUTH_REDIRECT_LOGIN_SUCCESS,
            );
            Ok(super::authenticated_redirect_response(
                state.get_ref(),
                session,
                redirect_url,
            )?)
        }
        external::ExternalAuthCallbackOutcome::EmailVerificationRequired(pending) => {
            Ok(external_auth_email_required_redirect_response(
                state.get_ref(),
                &pending.flow_token,
                &pending.return_path,
            ))
        }
    }
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/auth/external-auth/email-verification/start",
    tag = "external-auth",
    operation_id = "auth_external_auth_start_email_verification",
    request_body = external::ExternalAuthEmailVerificationStartRequest,
    responses(
        (status = 200, description = "External auth email verification email queued", body = inline(ApiResponse<external::ExternalAuthEmailVerificationStartResponse>)),
        (status = 400, description = "Invalid flow or email"),
        (status = 403, description = "External auth linking or registration is not allowed"),
    ),
)]
/// Queues the verification email for a pending external login so the address
/// can be confirmed before the session is issued.
pub async fn start_email_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<external::ExternalAuthEmailVerificationStartRequest>,
) -> Result<HttpResponse> {
    csrf::ensure_request_source_allowed(
        &req,
        state.get_ref().runtime_config(),
        RequestSourceMode::Required,
    )?;
    let response = external::start_email_verification(state.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(response)))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/auth/external-auth/email-verification/confirm",
    tag = "external-auth",
    operation_id = "auth_external_auth_confirm_email_verification",
    params(external::ExternalAuthEmailVerificationConfirmQuery),
    responses((status = 302, description = "External auth email verification completed and redirected")),
)]
/// Consumes an external-login email verification token, completing the login
/// with session cookies or redirecting to a status-bearing login URL.
pub async fn confirm_email_verification(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<external::ExternalAuthEmailVerificationConfirmQuery>,
) -> Result<HttpResponse> {
    let Some(token) = query
        .token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
    else {
        return Ok(external_auth_status_redirect_response(
            state.get_ref(),
            "email_verification_missing",
        ));
    };

    let result =
        match external::confirm_email_verification(state.get_ref(), token, None, None).await {
            Ok(result) => result,
            Err(error) if error.api_error_code() == AsterErrorCode::ContactVerificationExpired => {
                return Ok(external_auth_status_redirect_response(
                    state.get_ref(),
                    "email_verification_expired",
                ));
            }
            Err(error) if error.api_error_code() == AsterErrorCode::ContactVerificationInvalid => {
                return Ok(external_auth_status_redirect_response(
                    state.get_ref(),
                    "email_verification_invalid",
                ));
            }
            Err(error) => {
                return Ok(external_auth_error_redirect_response(
                    state.get_ref(),
                    &error,
                ));
            }
        };
    let session =
        local::issue_tokens_for_user(state.get_ref(), result.primary_login.user, &req).await?;
    let redirect_path = if matches!(
        session.status,
        local::AuthTokenStatus::PasswordChangeRequired
    ) {
        "/force-password-change"
    } else {
        &result.primary_login.return_path
    };
    let redirect_url = add_auth_redirect_status(
        site_url::public_app_url_or_path(state.get_ref().runtime_config(), redirect_path),
        AUTH_REDIRECT_LOGIN_SUCCESS,
    );
    super::authenticated_redirect_response(state.get_ref(), session, redirect_url)
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/auth/external-auth/password-link",
    tag = "external-auth",
    operation_id = "auth_external_auth_link_with_password",
    request_body = external::ExternalAuthPasswordLinkRequest,
    responses(
        (status = 200, description = "External auth identity linked", body = inline(ApiResponse<ExternalAuthFinishLoginResponse>)),
        (status = 400, description = "Invalid flow or request"),
        (status = 401, description = "Invalid credentials"),
    ),
)]
/// Links an external identity from a pending login flow to an existing account
/// by password confirmation, issuing session cookies on success.
pub async fn link_with_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<external::ExternalAuthPasswordLinkRequest>,
) -> Result<HttpResponse> {
    csrf::ensure_request_source_allowed(
        &req,
        state.get_ref().runtime_config(),
        RequestSourceMode::Required,
    )?;
    let result =
        external::link_with_password(state.get_ref(), body.into_inner(), None, None).await?;
    let session =
        local::issue_tokens_for_user(state.get_ref(), result.primary_login.user, &req).await?;
    let expires_in = session.expires_in;
    let status = if matches!(
        session.status,
        local::AuthTokenStatus::PasswordChangeRequired
    ) {
        "password_change_required"
    } else {
        "authenticated"
    };
    super::authenticated_response_with_body(
        state.get_ref(),
        session,
        ExternalAuthFinishLoginResponse {
            status,
            expires_in: Some(expires_in),
            flow_token: None,
            return_path: Some(result.primary_login.return_path),
        },
    )
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/auth/external-auth/links",
    tag = "external-auth",
    operation_id = "auth_external_auth_list_links",
    params(LimitQuery, CreatedAtCursorQuery),
    responses(
        (status = 200, description = "Linked external auth identities", body = inline(ApiResponse<CursorPage<external::ExternalAuthLinkInfo, DateTimeIdCursor>>)),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer" = [])),
)]
/// Lists the external identities linked to the current user's account with
/// cursor pagination.
pub async fn list_links(
    state: web::Data<AppState>,
    user: web::ReqData<AuthUserInfo>,
    page: web::Query<LimitQuery>,
    cursor_query: web::Query<CreatedAtCursorQuery>,
) -> Result<HttpResponse> {
    let cursor = parse_datetime_id_cursor(
        cursor_query.after_created_at,
        cursor_query.after_id,
        "external auth link",
    )?;
    let links =
        external::list_links_paginated(state.get_ref(), user.id, page.limit_or(20, 100), cursor)
            .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::ok(links)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/auth/external-auth/links/{id}",
    tag = "external-auth",
    operation_id = "auth_external_auth_delete_link",
    params(("id" = i64, Path, description = "External auth identity link ID")),
    responses(
        (status = 200, description = "External auth identity unlinked"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "External auth identity link not found"),
    ),
    security(("bearer" = [])),
)]
/// Unlinks one external identity from the current user's account.
pub async fn delete_link(
    state: web::Data<AppState>,
    user: web::ReqData<AuthUserInfo>,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let id = path.into_inner();
    if !external::delete_link(state.get_ref(), user.id, id).await? {
        return Err(AsterError::record_not_found(format!(
            "external auth identity link #{id}"
        )));
    }
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

fn external_auth_error_redirect_response(state: &AppState, error: &AsterError) -> HttpResponse {
    tracing::warn!(error = %error, "external auth callback failed");
    let path = if error.status_code().is_server_error() {
        "/login?external_auth=error".to_string()
    } else {
        format!(
            "/login?external_auth=error&code={}",
            error.api_error_code().as_str()
        )
    };
    let redirect_url = site_url::public_app_url_or_path(state.runtime_config(), &path);
    HttpResponse::Found()
        .append_header((header::LOCATION, redirect_url))
        .finish()
}

fn external_auth_email_required_redirect_response(
    state: &AppState,
    flow_token: &str,
    return_path: &str,
) -> HttpResponse {
    let path = format!(
        "/login?external_auth=email_required&flow={}&return_path={}",
        urlencoding::encode(flow_token),
        urlencoding::encode(return_path)
    );
    let redirect_url = site_url::public_app_url_or_path(state.runtime_config(), &path);
    HttpResponse::Found()
        .append_header((header::LOCATION, redirect_url))
        .finish()
}

fn external_auth_status_redirect_response(state: &AppState, status: &str) -> HttpResponse {
    let path = format!("/login?external_auth={}", urlencoding::encode(status));
    let redirect_url = site_url::public_app_url_or_path(state.runtime_config(), &path);
    HttpResponse::Found()
        .append_header((header::LOCATION, redirect_url))
        .finish()
}

fn add_auth_redirect_status(location: String, status: &str) -> String {
    let (base, hash) = location
        .split_once('#')
        .map_or((location.as_str(), ""), |(base, hash)| (base, hash));
    let separator = if base.contains('?') { '&' } else { '?' };
    let mut next = format!(
        "{base}{separator}{AUTH_REDIRECT_PARAM}={}",
        urlencoding::encode(status)
    );
    if !hash.is_empty() {
        next.push('#');
        next.push_str(hash);
    }
    next
}

async fn ensure_provider_kind(
    state: &AppState,
    kind: ExternalAuthProviderKind,
    provider: &str,
) -> Result<()> {
    let providers = external::list_public_providers_by_kind(state, kind).await?;
    if providers.iter().any(|item| item.key == provider) {
        return Ok(());
    }
    Err(AsterError::record_not_found(format!(
        "external auth provider {provider}"
    )))
}

#[cfg(test)]
mod tests {
    use super::add_auth_redirect_status;

    #[test]
    fn add_auth_redirect_status_preserves_existing_query_and_hash() {
        assert_eq!(
            add_auth_redirect_status("/account".to_string(), "login_success"),
            "/account?auth_redirect=login_success"
        );
        assert_eq!(
            add_auth_redirect_status("/account?tab=skins".to_string(), "login_success"),
            "/account?tab=skins&auth_redirect=login_success"
        );
        assert_eq!(
            add_auth_redirect_status(
                "http://localhost:8080/account?tab=skins#profile".to_string(),
                "login_success",
            ),
            "http://localhost:8080/account?tab=skins&auth_redirect=login_success#profile"
        );
    }
}
