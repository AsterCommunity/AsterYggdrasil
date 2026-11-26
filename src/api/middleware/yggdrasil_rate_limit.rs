//! Yggdrasil protocol rate limiting helpers.

use crate::config::{RateLimitConfig, RateLimitTier};
use aster_forge_middleware::actix::rate_limit::{NormalizedStringRateLimiter, RateLimitRejection};
use aster_yggdrasil_protocol::YggdrasilErrorBody;
use std::sync::Arc;

/// Per-username rate limiter for Yggdrasil protocol endpoints that accept a
/// plain-text account identifier.
///
/// Keeps separate buckets for authenticate and signout so one exhausted flow
/// cannot lock the other out; usernames are normalized before bucketing so
/// case and whitespace variants share a quota.
#[derive(Clone)]
pub struct YggdrasilRateLimiter {
    authenticate: Arc<NormalizedStringRateLimiter>,
    signout: Arc<NormalizedStringRateLimiter>,
}

impl YggdrasilRateLimiter {
    /// Builds both bucket families from the runtime rate limit config; when
    /// the feature is disabled every check passes.
    pub fn from_config(config: &RateLimitConfig) -> Self {
        Self {
            authenticate: Arc::new(build_keyed_limiter(config.enabled, &config.auth)),
            signout: Arc::new(build_keyed_limiter(config.enabled, &config.auth)),
        }
    }

    /// Consumes one authenticate slot for the given username, returning a
    /// rejection when the bucket is exhausted.
    pub fn check_authenticate(&self, username: &str) -> Option<YggdrasilRateLimitRejection> {
        self.check(&self.authenticate, username)
    }

    /// Consumes one signout slot for the given username, returning a
    /// rejection when the bucket is exhausted.
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

/// A rejected Yggdrasil protocol request; carries the wait time and renders
/// the protocol error body.
pub struct YggdrasilRateLimitRejection {
    inner: RateLimitRejection,
}

impl YggdrasilRateLimitRejection {
    /// Whole seconds the client should wait before retrying.
    pub const fn retry_after_seconds(&self) -> u64 {
        self.inner.retry_after_seconds()
    }

    /// Builds the `429` protocol response with a `Retry-After` header and the
    /// Yggdrasil error body.
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
