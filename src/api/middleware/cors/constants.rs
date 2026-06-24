//! Runtime CORS middleware constants.
//!
//! The CSRF request header is intentionally not listed here because Yggdrasil
//! allows deployments to configure that name at startup. The middleware adds
//! the current CSRF header dynamically when validating preflight requests and
//! generating `Access-Control-Allow-Headers`.

pub(super) const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];

pub(super) const ALLOWED_HEADERS: &[&str] = &[
    "authorization",
    "accept",
    "content-type",
    "range",
    "timeout",
    "x-request-id",
];

pub(super) const EXPOSE_HEADERS: &[&str] = &["content-length", "etag", "x-request-id"];
