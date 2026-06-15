//! Yggdrasil protocol rate limiting helpers.

use crate::api::dto::yggdrasil::YggdrasilErrorBody;
use crate::config::{RateLimitConfig, RateLimitTier};
use governor::clock::{Clock, DefaultClock, QuantaInstant};
use governor::middleware::NoOpMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{NotUntil, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

type KeyedLimiter =
    RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>;

#[derive(Clone)]
pub struct YggdrasilRateLimiter {
    enabled: bool,
    authenticate: Arc<KeyedLimiter>,
    signout: Arc<KeyedLimiter>,
}

impl YggdrasilRateLimiter {
    pub fn from_config(config: &RateLimitConfig) -> Self {
        Self {
            enabled: config.enabled,
            authenticate: Arc::new(build_keyed_limiter(&config.auth)),
            signout: Arc::new(build_keyed_limiter(&config.auth)),
        }
    }

    pub fn check_authenticate(&self, username: &str) -> Option<YggdrasilRateLimitRejection> {
        self.check(&self.authenticate, username)
    }

    pub fn check_signout(&self, username: &str) -> Option<YggdrasilRateLimitRejection> {
        self.check(&self.signout, username)
    }

    fn check(&self, limiter: &KeyedLimiter, username: &str) -> Option<YggdrasilRateLimitRejection> {
        if !self.enabled {
            return None;
        }
        let key = username.trim().to_ascii_lowercase();
        limiter
            .check_key(&key)
            .err()
            .map(YggdrasilRateLimitRejection::from_not_until)
    }
}

pub struct YggdrasilRateLimitRejection {
    retry_after_seconds: u64,
}

impl YggdrasilRateLimitRejection {
    fn from_not_until(not_until: NotUntil<QuantaInstant>) -> Self {
        let retry_after_seconds = not_until
            .wait_time_from(DefaultClock::default().now())
            .as_secs()
            .max(1);
        Self {
            retry_after_seconds,
        }
    }

    pub const fn retry_after_seconds(&self) -> u64 {
        self.retry_after_seconds
    }

    pub fn into_response(self) -> actix_web::HttpResponse {
        let message = format!(
            "Too many requests. Retry after {} seconds.",
            self.retry_after_seconds
        );
        actix_web::HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", self.retry_after_seconds.to_string()))
            .json(YggdrasilErrorBody {
                error: "TooManyRequestsException",
                error_message: message,
                cause: None,
            })
    }
}

fn build_keyed_limiter(tier: &RateLimitTier) -> KeyedLimiter {
    let quota = Quota::with_period(Duration::from_secs(tier.seconds_per_request.get()))
        .expect("non-zero rate limit tier should always build")
        .allow_burst(
            NonZeroU32::new(tier.burst_size.get()).expect("burst size should be non-zero"),
        );
    RateLimiter::keyed(quota)
}

#[cfg(test)]
mod tests {
    use super::YggdrasilRateLimiter;
    use crate::config::{RateLimitConfig, RateLimitTier};
    use std::num::{NonZeroU32, NonZeroU64};

    fn strict_config(enabled: bool) -> RateLimitConfig {
        RateLimitConfig {
            enabled,
            auth: RateLimitTier {
                seconds_per_request: NonZeroU64::new(60).unwrap(),
                burst_size: NonZeroU32::new(1).unwrap(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn disabled_limiter_does_not_reject() {
        let limiter = YggdrasilRateLimiter::from_config(&strict_config(false));

        assert!(limiter.check_authenticate("admin@example.com").is_none());
        assert!(limiter.check_authenticate("admin@example.com").is_none());
        assert!(limiter.check_signout("admin@example.com").is_none());
        assert!(limiter.check_signout("admin@example.com").is_none());
    }

    #[test]
    fn authenticate_and_signout_have_separate_buckets() {
        let limiter = YggdrasilRateLimiter::from_config(&strict_config(true));

        assert!(limiter.check_authenticate("admin@example.com").is_none());
        assert!(limiter.check_authenticate("admin@example.com").is_some());
        assert!(limiter.check_signout("admin@example.com").is_none());
        assert!(limiter.check_signout("admin@example.com").is_some());
    }

    #[test]
    fn authenticate_bucket_is_keyed_by_normalized_username() {
        let limiter = YggdrasilRateLimiter::from_config(&strict_config(true));

        assert!(limiter.check_authenticate("Admin@Example.com").is_none());
        assert!(limiter.check_authenticate("other@example.com").is_none());
        assert!(limiter.check_authenticate(" admin@example.com ").is_some());
    }

    #[test]
    fn signout_bucket_is_keyed_by_normalized_username() {
        let limiter = YggdrasilRateLimiter::from_config(&strict_config(true));

        assert!(limiter.check_signout("Admin@Example.com").is_none());
        assert!(limiter.check_signout("other@example.com").is_none());
        assert!(limiter.check_signout(" admin@example.com ").is_some());
    }
}
