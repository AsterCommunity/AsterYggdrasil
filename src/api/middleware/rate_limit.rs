//! Product boundary for API rate limiting.
//!
//! Forge owns the trusted-proxy-aware IP extraction mechanics. This module
//! keeps Yggdrasil's API response envelope for `429 Too Many Requests` and maps
//! deployment config into the Actix governor middleware.

use actix_governor::{
    GovernorConfig, GovernorConfigBuilder, KeyExtractor, SimpleKeyExtractionError,
};
use actix_web::dev::ServiceRequest;
use actix_web::{HttpResponse, HttpResponseBuilder};
use governor::NotUntil;
use governor::clock::QuantaInstant;
use governor::middleware::NoOpMiddleware;
use std::net::IpAddr;

use crate::api::error_code::AsterErrorCode;
use crate::api::response::ApiResponse;
use crate::config::RateLimitTier;

#[derive(Debug, Clone)]
pub struct AsterIpKeyExtractor {
    inner: aster_forge_middleware::actix::rate_limit::TrustedProxyIpKeyExtractor,
}

impl AsterIpKeyExtractor {
    pub fn new(trusted_proxies: &[String]) -> Self {
        Self {
            inner: aster_forge_middleware::actix::rate_limit::TrustedProxyIpKeyExtractor::new(
                trusted_proxies,
            ),
        }
    }

    #[cfg(test)]
    fn is_trusted(&self, ip: IpAddr) -> bool {
        self.inner.is_trusted(ip)
    }
}

impl KeyExtractor for AsterIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        self.inner.extract(req)
    }

    fn exceed_rate_limit_response(
        &self,
        negative: &NotUntil<QuantaInstant>,
        _response: HttpResponseBuilder,
    ) -> HttpResponse {
        let wait_time = aster_forge_middleware::actix::rate_limit::retry_after_seconds(negative);
        let msg = format!("Too Many Requests, retry after {wait_time}s");
        HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", wait_time.to_string()))
            .json(ApiResponse::<()>::error_body(
                AsterErrorCode::RateLimited,
                &msg,
                Some(true),
            ))
    }
}

/// Builds an Actix governor config from Yggdrasil's rate-limit tier.
#[expect(
    clippy::expect_used,
    reason = "actix-governor exposes fallible finish() for zero values; RateLimitTier stores both inputs as NonZero"
)]
pub fn build_governor(
    tier: &RateLimitTier,
    trusted_proxies: &[String],
) -> GovernorConfig<AsterIpKeyExtractor, NoOpMiddleware> {
    GovernorConfigBuilder::default()
        .key_extractor(AsterIpKeyExtractor::new(trusted_proxies))
        .seconds_per_request(tier.seconds_per_request.get())
        .burst_size(tier.burst_size.get())
        .finish()
        .expect("non-zero rate limit tier should always build")
}

#[cfg(test)]
mod tests {
    use super::{AsterIpKeyExtractor, build_governor};
    use crate::api::error_code::AsterErrorCode;
    use crate::config::RateLimitTier;
    use actix_governor::{Governor, KeyExtractor};
    use actix_web::{App, HttpResponse, test as actix_test, web};
    use std::net::IpAddr;
    use std::num::{NonZeroU32, NonZeroU64};

    #[test]
    fn empty_trusted_always_uses_peer() {
        let ext = AsterIpKeyExtractor::new(&[]);
        let peer: IpAddr = "1.2.3.4".parse().unwrap();
        assert!(!ext.is_trusted(peer));
    }

    #[test]
    fn cidr_match_trusts_proxy_and_reads_xff() {
        let ext = AsterIpKeyExtractor::new(&["10.0.0.0/8".to_string()]);
        assert!(ext.is_trusted("10.0.0.1".parse().unwrap()));
        assert!(!ext.is_trusted("11.0.0.1".parse().unwrap()));
    }

    #[test]
    fn single_ip_trusted_proxy() {
        let ext = AsterIpKeyExtractor::new(&["192.168.1.1".to_string()]);
        assert!(ext.is_trusted("192.168.1.1".parse().unwrap()));
        assert!(!ext.is_trusted("192.168.1.2".parse().unwrap()));
    }

    #[actix_web::test]
    async fn extract_uses_peer_when_request_has_no_trusted_proxy() {
        let ext = AsterIpKeyExtractor::new(&[]);
        let req = actix_test::TestRequest::default()
            .peer_addr("198.51.100.10:12345".parse().unwrap())
            .insert_header(("X-Forwarded-For", "203.0.113.10"))
            .to_srv_request();

        assert_eq!(
            ext.extract(&req).unwrap(),
            "198.51.100.10".parse::<IpAddr>().unwrap()
        );
    }

    #[actix_web::test]
    async fn extract_uses_last_untrusted_forwarded_ip_from_trusted_peer() {
        // Forge walks X-Forwarded-For right to left and skips trusted hops, so the
        // last untrusted hop is used as the client key instead of the spoofable
        // leftmost entry.
        let ext = AsterIpKeyExtractor::new(&["10.0.0.0/8".to_string()]);
        let req = actix_test::TestRequest::default()
            .peer_addr("10.0.0.5:12345".parse().unwrap())
            .insert_header(("X-Forwarded-For", "203.0.113.10, 198.51.100.2"))
            .to_srv_request();

        assert_eq!(
            ext.extract(&req).unwrap(),
            "198.51.100.2".parse::<IpAddr>().unwrap()
        );
    }

    #[actix_web::test]
    async fn extract_falls_back_to_peer_for_invalid_forwarded_ip_or_missing_peer() {
        let ext = AsterIpKeyExtractor::new(&["10.0.0.0/8".to_string()]);
        let invalid_forwarded = actix_test::TestRequest::default()
            .peer_addr("10.0.0.5:12345".parse().unwrap())
            .insert_header(("X-Forwarded-For", "not-an-ip"))
            .to_srv_request();
        assert_eq!(
            ext.extract(&invalid_forwarded).unwrap(),
            "10.0.0.5".parse::<IpAddr>().unwrap()
        );

        let missing_peer = actix_test::TestRequest::default().to_srv_request();
        assert_eq!(
            ext.extract(&missing_peer).unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[actix_web::test]
    async fn governor_uses_api_response_for_rate_limited_requests() {
        let tier = RateLimitTier {
            seconds_per_request: NonZeroU64::new(60).unwrap(),
            burst_size: NonZeroU32::new(1).unwrap(),
        };
        let config = build_governor(&tier, &[]);
        let app = actix_test::init_service(
            App::new()
                .wrap(Governor::new(&config))
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
}
