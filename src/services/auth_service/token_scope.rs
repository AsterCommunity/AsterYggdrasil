//! Token scope helpers for restricted authentication sessions.

use actix_web::HttpRequest;

fn password_change_path_allowed(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/api/v1/auth/me")
            | ("PUT", "/api/v1/auth/password")
            | ("POST", "/api/v1/auth/logout")
    )
}

pub(crate) fn password_change_request_allowed(req: &HttpRequest) -> bool {
    password_change_path_allowed(req.method().as_str(), req.path())
}
