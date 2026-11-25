//! Product boundary for API rate limiting.
//!
//! Forge owns the trusted-proxy-aware IP extraction mechanics and the Actix
//! middleware that wraps the `governor` crate directly (GPL-free since
//! `aster_forge_middleware` 0.1.0-alpha.6). This module keeps Yggdrasil's API
//! response envelope for `429 Too Many Requests` and maps deployment config
//! into the shared limiter.

use actix_web::{HttpResponse, HttpResponseBuilder};

use aster_forge_middleware::actix::rate_limit::{
    RateLimiterConfig, TrustedProxyIpKeyExtractor,
    build_ip_rate_limit_config_with_rejection_response,
};

use crate::api::error_code::AsterErrorCode;
use crate::api::response::ApiResponse;
use crate::config::RateLimitTier;

/// Builds a trusted-proxy-aware limiter config with Yggdrasil's API envelope.
///
/// The returned config shares one quota state across Actix workers; wrap it in
/// [`aster_forge_middleware::actix::rate_limit::RateLimit`] per worker app.
pub fn build_rate_limit(
    tier: &RateLimitTier,
    trusted_proxies: &[String],
) -> RateLimiterConfig<TrustedProxyIpKeyExtractor> {
    build_ip_rate_limit_config_with_rejection_response(
        tier.seconds_per_request,
        tier.burst_size,
        trusted_proxies,
        rate_limit_response,
    )
}

fn rate_limit_response(wait_time: u64, mut response: HttpResponseBuilder) -> HttpResponse {
    let msg = format!("Too Many Requests, retry after {wait_time}s");
    // Forge already set Retry-After; keep the envelope fields product-owned.
    response.json(ApiResponse::<()>::error_body(
        AsterErrorCode::RateLimited,
        &msg,
        Some(true),
    ))
}

#[cfg(test)]
mod tests {
    use super::{build_rate_limit, rate_limit_response};
    use crate::api::error_code::AsterErrorCode;
    use crate::config::RateLimitTier;
    use actix_web::middleware::Condition;
    use actix_web::{App, HttpResponse, test as actix_test, web};
    use aster_forge_middleware::actix::rate_limit::RateLimit;
    use std::num::{NonZeroU32, NonZeroU64};

    fn tier(burst: u32) -> RateLimitTier {
        RateLimitTier {
            seconds_per_request: NonZeroU64::new(60).unwrap(),
            burst_size: NonZeroU32::new(burst).unwrap(),
        }
    }

    #[actix_web::test]
    async fn limited_requests_use_the_api_response_envelope() {
        let config = build_rate_limit(&tier(1), &[]);
        let app = actix_test::init_service(
            App::new()
                .wrap(RateLimit::new(&config))
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let first = actix_test::TestRequest::get()
            .uri("/")
            .peer_addr("198.51.100.20:12345".parse().unwrap())
            .to_request();
        assert_eq!(actix_test::call_service(&app, first).await.status(), 200);

        let second = actix_test::TestRequest::get()
            .uri("/")
            .peer_addr("198.51.100.20:12345".parse().unwrap())
            .to_request();
        let response = actix_test::call_service(&app, second).await;
        assert_eq!(response.status(), 429);
        assert!(response.headers().contains_key("Retry-After"));

        let body: serde_json::Value = actix_test::read_body_json(response).await;
        assert_eq!(body["code"], AsterErrorCode::RateLimited.as_str());
        assert_eq!(body["error"]["retryable"], true);
        assert!(body["data"].is_null());
    }

    #[actix_web::test]
    async fn trusted_proxy_clients_get_independent_buckets() {
        // Two clients behind one trusted proxy must not share quota state; the
        // same client from an untrusted peer keeps its own bucket.
        let config = build_rate_limit(&tier(1), &["10.0.0.0/8".to_string()]);
        let app = actix_test::init_service(
            App::new()
                .wrap(RateLimit::new(&config))
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let request = |peer: &str, forwarded: &str| {
            actix_test::TestRequest::get()
                .uri("/")
                .peer_addr(peer.parse().unwrap())
                .insert_header(("X-Forwarded-For", forwarded))
                .to_request()
        };

        let first = actix_test::call_service(&app, request("10.0.0.5:12345", "203.0.113.10")).await;
        assert_eq!(first.status(), 200);

        let exhausted =
            actix_test::call_service(&app, request("10.0.0.5:54321", "203.0.113.10")).await;
        assert_eq!(exhausted.status(), 429, "same client IP shares the bucket");

        let other = actix_test::call_service(&app, request("10.0.0.5:12345", "198.51.100.2")).await;
        assert_eq!(
            other.status(),
            200,
            "a different client gets a fresh bucket"
        );

        let spoofed =
            actix_test::call_service(&app, request("11.0.0.1:12345", "203.0.113.10")).await;
        assert_eq!(
            spoofed.status(),
            200,
            "untrusted peers ignore forwarded IPs"
        );
    }

    #[actix_web::test]
    async fn cloned_configs_share_quota_across_worker_apps() {
        let config = build_rate_limit(&tier(1), &[]);
        let worker_one = config.clone();
        let worker_two = config.clone();
        let app_one = actix_test::init_service(
            App::new()
                .wrap(RateLimit::new(&worker_one))
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;
        let app_two = actix_test::init_service(
            App::new()
                .wrap(RateLimit::new(&worker_two))
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let first = actix_test::TestRequest::get()
            .uri("/")
            .peer_addr("198.51.100.30:12345".parse().unwrap())
            .to_request();
        assert_eq!(
            actix_test::call_service(&app_one, first).await.status(),
            200
        );

        let second = actix_test::TestRequest::get()
            .uri("/")
            .peer_addr("198.51.100.30:12345".parse().unwrap())
            .to_request();
        assert_eq!(
            actix_test::call_service(&app_two, second).await.status(),
            429,
            "worker apps must consume the same allowance"
        );
    }

    #[actix_web::test]
    async fn disabled_condition_leaves_requests_unlimited() {
        // Admin routes install the limiter behind `Condition::new(rl.enabled, ...)`.
        let config = build_rate_limit(&tier(1), &[]);
        let app = actix_test::init_service(
            App::new()
                .wrap(Condition::new(false, RateLimit::new(&config)))
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;
        for _ in 0..3 {
            let request = actix_test::TestRequest::get()
                .uri("/")
                .peer_addr("198.51.100.40:12345".parse().unwrap())
                .to_request();
            assert_eq!(actix_test::call_service(&app, request).await.status(), 200);
        }
    }

    #[test]
    fn rejection_response_maps_the_api_envelope_without_touching_retry_after() {
        // Forge's middleware inserts Retry-After before invoking the factory.
        let mut builder =
            actix_web::HttpResponse::build(actix_web::http::StatusCode::TOO_MANY_REQUESTS);
        builder.insert_header(("Retry-After", "7"));
        let response = rate_limit_response(7, builder);
        assert_eq!(
            response.headers().get("Retry-After").unwrap(),
            "7",
            "the middleware-provided Retry-After survives the factory"
        );
        assert_eq!(
            response.status(),
            actix_web::http::StatusCode::TOO_MANY_REQUESTS
        );
    }
}
