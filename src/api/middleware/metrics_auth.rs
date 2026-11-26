//! Basic authentication for the Prometheus scrape route.
//!
//! Credentials are verified by an instance-owned Argon2 runtime that is
//! independent of login, so scrape traffic and login traffic cannot exhaust
//! each other's password-work budget (each permit holds the full Argon2
//! working memory). A missing `MetricsAuth`, unknown user or invalid password
//! all produce the same empty 401: the route must not reveal which half of
//! the pair was wrong.

use actix_web::{
    Error, HttpResponse,
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::header::{self, HeaderValue},
    middleware::Next,
    web,
};
use aster_forge_auth::PasswordHashRuntime;
use aster_forge_metrics::MetricsBasicAuthConfig;
use base64::{Engine, prelude::BASE64_STANDARD};

/// Instance-owned checker for the scrape credential.
///
/// One per process, shared by HTTP workers and independent of login. The
/// default Forge budget (two concurrent Argon2 jobs) is sized for a periodic
/// scraper; saturation fails closed instead of queueing behind login work.
pub struct MetricsAuth {
    config: MetricsBasicAuthConfig,
    runtime: PasswordHashRuntime,
}

impl MetricsAuth {
    /// Creates the checker from the scrape credential snapshot; the
    /// password hash is bound at construction, so rotating the credential
    /// requires building a new `MetricsAuth`.
    pub fn new(config: MetricsBasicAuthConfig) -> Self {
        Self {
            config,
            runtime: PasswordHashRuntime::default(),
        }
    }

    /// Checks the request against the configured scrape credentials.
    ///
    /// Returns `true` when Basic auth is disabled, so the middleware can stay
    /// mounted unconditionally. `try_verify_password` is the Forge-prescribed
    /// shape for a static credential shared by every request: saturation
    /// rejects immediately and reveals nothing about the username, unlike
    /// per-account login paths that must queue to keep timings uniform.
    pub async fn authorize(&self, request: &actix_web::HttpRequest) -> bool {
        if !self.config.enabled {
            return true;
        }
        let Some((username, password)) = credentials(request) else {
            return false;
        };
        let verification = self
            .runtime
            .try_verify_password(&password, &self.config.password_hash)
            .await;
        verification
            .is_ok_and(|verification| verification.is_valid && username == self.config.username)
    }
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
    let authorized = match request.app_data::<web::Data<MetricsAuth>>() {
        Some(auth) => auth.authorize(request.request()).await,
        None => false,
    };
    if !authorized {
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
    async fn accepts_mixed_case_scheme_and_keeps_password_colons() {
        let mixed = format!("bAsIc {}", BASE64_STANDARD.encode("scrape:pass"));
        assert_eq!(
            credentials(&request_with(Some(&mixed))),
            Some(("scrape".to_owned(), "pass".to_owned()))
        );
        // A colon in the configured username is rejected by config validation,
        // so any colon beyond the separator belongs to the password.
        assert_eq!(
            credentials(&request_with(Some(&basic("us:er", "pass")))),
            Some(("us".to_owned(), "er:pass".to_owned()))
        );
    }

    /// An empty username/password pair parses; rejection happens in
    /// `authorize`, because config validation never accepts an empty username.
    #[actix_web::test]
    async fn parses_empty_pair_from_minimal_payload() {
        assert_eq!(
            credentials(&request_with(Some("Basic Og=="))),
            Some((String::new(), String::new()))
        );
    }

    #[actix_web::test]
    async fn rejects_non_utf8_and_duplicate_authorization_headers() {
        let non_utf8 = format!("Basic {}", BASE64_STANDARD.encode([0xff, b':', b'x']));
        assert!(credentials(&request_with(Some(&non_utf8))).is_none());

        let value = basic("scrape", "pass");
        let request = test::TestRequest::get()
            .append_header((header::AUTHORIZATION, value.clone()))
            .append_header((header::AUTHORIZATION, value))
            .to_http_request();
        assert!(credentials(&request).is_none());
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

    /// Disabled auth authorizes without touching credentials at all, so a
    /// mis-typed or malicious header cannot wedge the scrape path.
    #[actix_web::test]
    async fn disabled_config_authorizes_without_parsing_credentials() {
        let auth = MetricsAuth::new(MetricsBasicAuthConfig::default());
        assert!(auth.authorize(&request_with(Some("Bearer jwt"))).await);
        assert!(auth.authorize(&request_with(Some("Basic !!"))).await);
    }

    /// The checker binds its credential snapshot at construction (AsterDrive
    /// parity): a rotated verifier lives in a new `MetricsAuth`, which rejects
    /// the stale password and accepts the new one.
    #[actix_web::test]
    async fn credential_rotation_uses_the_construction_snapshot() {
        let policy = aster_forge_crypto::PasswordHashPolicy::new(
            aster_forge_crypto::PasswordHashWorkFactor::new(8, 1, 1, 32).unwrap(),
            aster_forge_crypto::PasswordHashVerificationLimits::new(8, 1, 1, 32).unwrap(),
        )
        .unwrap();
        let checker_for = |password_hash: &str| {
            MetricsAuth::new(MetricsBasicAuthConfig {
                enabled: true,
                username: "prometheus".into(),
                password_hash: password_hash.to_owned(),
            })
        };
        let first =
            checker_for(&aster_forge_crypto::hash_password_with_policy("first", &policy).unwrap());
        let valid = request_with(Some(&basic("prometheus", "first")));
        assert!(first.authorize(&valid).await);
        assert!(
            !first
                .authorize(&request_with(Some(&basic("prometheus", "second"))))
                .await
        );

        let rotated =
            checker_for(&aster_forge_crypto::hash_password_with_policy("second", &policy).unwrap());
        assert!(!rotated.authorize(&valid).await);
        assert!(
            rotated
                .authorize(&request_with(Some(&basic("prometheus", "second"))))
                .await
        );
    }
}
