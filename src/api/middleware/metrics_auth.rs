//! Basic authentication for the Prometheus scrape route.
//!
//! Credentials are verified against `RuntimeConfig`'s Argon2 worker pool, so
//! account login and scrape auth share one bounded work budget. A missing
//! `AppState`, unknown user or invalid password all produce the same 401 body:
//! the route must not reveal whether a username exists.

use actix_web::{
    Error, HttpResponse,
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::header::{self, HeaderValue},
    middleware::Next,
    web,
};
use base64::{Engine, prelude::BASE64_STANDARD};

use crate::{config::metrics::PrometheusBasicAuth, runtime::AppState};

/// Checks the request against the configured scrape credentials.
///
/// Returns `true` when Basic auth is disabled, so callers can keep the
/// middleware mounted unconditionally.
async fn authorized(
    state: &AppState,
    config: &PrometheusBasicAuth,
    request: &actix_web::HttpRequest,
) -> bool {
    let Some((username, password)) = credentials(request) else {
        return false;
    };
    // Verify before comparing the username so a wrong user still pays the
    // same Argon2 cost as a wrong password; otherwise the response time tells
    // a scanner which half of the pair was wrong.
    let Ok(runtimes) = state.runtime_config().auth_runtimes() else {
        return false;
    };
    let password_matches = runtimes
        .password_hash_runtime()
        .verify_password(&password, &config.password_hash)
        .await
        .map(|verification| verification.is_valid)
        .unwrap_or(false);
    password_matches && username == config.username
}

/// Parses exactly one `Authorization: Basic` header.
///
/// Rejects duplicate headers, oversized values, control characters and
/// non-UTF-8 payloads before any password work starts.
fn credentials(request: &actix_web::HttpRequest) -> Option<(String, String)> {
    let mut values = request.headers().get_all(header::AUTHORIZATION);
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() || value.len() > 8192 {
        return None;
    }
    let (scheme, encoded) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Basic") || encoded.is_empty() {
        return None;
    }
    let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).ok()?).ok()?;
    let (username, password) = decoded.split_once(':')?;
    if username.len() > 256
        || password.len() > 4096
        || username.chars().any(char::is_control)
        || password.chars().any(char::is_control)
    {
        return None;
    }
    Some((username.to_owned(), password.to_owned()))
}

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized()
        .insert_header((
            header::WWW_AUTHENTICATE,
            "Basic realm=\"AsterYggdrasil metrics\"",
        ))
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

/// Actix middleware applied to the Prometheus resource only.
pub async fn middleware(
    request: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<EitherBody<impl MessageBody>>, Error> {
    // The scope carries its own copy of the auth block, so a test app or a
    // future embedding that mounts only the metrics scope can still serve
    // /metrics without inventing a runtime pool.
    let config = request
        .app_data::<web::Data<PrometheusBasicAuth>>()
        .map(|config| config.as_ref().clone())
        .or_else(|| {
            request
                .app_data::<web::Data<AppState>>()
                .map(|state| state.config.observability.prometheus.basic_auth.clone())
        });
    let state = request.app_data::<web::Data<AppState>>().cloned();
    let allowed = match (config, state) {
        (Some(config), _) if !config.enabled => true,
        (Some(config), Some(state)) => authorized(state.as_ref(), &config, request.request()).await,
        _ => false,
    };
    if !allowed {
        return Ok(request.into_response(unauthorized()).map_into_right_body());
    }
    let mut response = next.call(request).await?.map_into_left_body();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    fn basic(user: &str, password: &str) -> String {
        format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{user}:{password}"))
        )
    }

    fn request_with(header_value: Option<&str>) -> actix_web::HttpRequest {
        let mut builder = test::TestRequest::get();
        if let Some(value) = header_value {
            builder = builder.insert_header((header::AUTHORIZATION, value));
        }
        builder.to_http_request()
    }

    #[actix_web::test]
    async fn rejects_malformed_credentials() {
        assert!(credentials(&request_with(None)).is_none());
        for value in [
            "Bearer token".to_owned(),
            "Basic".to_owned(),
            "Basic ".to_owned(),
            "Basic !!".to_owned(),
            format!("Basic {}", BASE64_STANDARD.encode("no-colon")),
            format!(
                "Basic {}",
                BASE64_STANDARD.encode("user:pass\nwith-control")
            ),
        ] {
            assert!(
                credentials(&request_with(Some(&value))).is_none(),
                "{value} must be rejected"
            );
        }
    }

    #[actix_web::test]
    async fn accepts_one_basic_header_and_keeps_password_colons() {
        let parsed = credentials(&request_with(Some(&basic("scrape", "p:a:ss"))));
        assert_eq!(parsed, Some(("scrape".to_owned(), "p:a:ss".to_owned())));
        // A colon in the configured username is rejected by config validation,
        // so any colon beyond the separator belongs to the password.
        assert_eq!(
            credentials(&request_with(Some(&basic("us:er", "pass")))),
            Some(("us".to_owned(), "er:pass".to_owned()))
        );
    }

    #[actix_web::test]
    async fn unauthorized_response_sets_basic_challenge_and_no_store() {
        let response = unauthorized();
        assert_eq!(response.status(), actix_web::http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(header::WWW_AUTHENTICATE).unwrap(),
            "Basic realm=\"AsterYggdrasil metrics\""
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
    }
}
