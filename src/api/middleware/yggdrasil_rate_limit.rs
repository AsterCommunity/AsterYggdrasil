//! Yggdrasil protocol rate limiting helpers.

use crate::config::{RateLimitConfig, RateLimitTier};
use aster_forge_middleware::actix::rate_limit::{NormalizedStringRateLimiter, RateLimitRejection};
use aster_yggdrasil_protocol::YggdrasilErrorBody;
use std::sync::Arc;

#[derive(Clone)]
pub struct YggdrasilRateLimiter {
    authenticate: Arc<NormalizedStringRateLimiter>,
    signout: Arc<NormalizedStringRateLimiter>,
}

impl YggdrasilRateLimiter {
    pub fn from_config(config: &RateLimitConfig) -> Self {
        Self {
            authenticate: Arc::new(build_keyed_limiter(config.enabled, &config.auth)),
            signout: Arc::new(build_keyed_limiter(config.enabled, &config.auth)),
        }
    }

    pub fn check_authenticate(&self, username: &str) -> Option<YggdrasilRateLimitRejection> {
        self.check(&self.authenticate, username)
    }

    pub fn check_signout(&self, username: &str) -> Option<YggdrasilRateLimitRejection> {
        self.check(&self.signout, username)
    }

    fn check(&self, limiter: &KeyedLimiter, username: &str) -> Option<YggdrasilRateLimitRejection> {
        limiter
            .check(username)
            .map(|inner| YggdrasilRateLimitRejection { inner })
    }
}

type KeyedLimiter = NormalizedStringRateLimiter;

pub struct YggdrasilRateLimitRejection {
    inner: RateLimitRejection,
}

impl YggdrasilRateLimitRejection {
    pub const fn retry_after_seconds(&self) -> u64 {
        self.inner.retry_after_seconds()
    }

    pub fn into_response(self) -> actix_web::HttpResponse {
        let message = format!(
            "Too many requests. Retry after {} seconds.",
            self.retry_after_seconds()
        );
        actix_web::HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", self.retry_after_seconds().to_string()))
            .json(YggdrasilErrorBody {
                error: "TooManyRequestsException",
                error_message: message,
                cause: None,
            })
    }
}

fn build_keyed_limiter(enabled: bool, tier: &RateLimitTier) -> KeyedLimiter {
    NormalizedStringRateLimiter::new(enabled, tier.seconds_per_request, tier.burst_size)
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
