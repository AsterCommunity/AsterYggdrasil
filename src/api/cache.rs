use actix_web::{HttpRequest, HttpResponse, http::header};

use crate::utils::hash;

pub(crate) fn weak_etag_for_bytes(bytes: &[u8]) -> String {
    format!("W/\"sha256-{}\"", hash::sha256_hex(bytes))
}

pub(crate) fn weak_etag_for_sha256_hash(hash: &str) -> String {
    format!("W/\"sha256-{hash}\"")
}

pub(crate) fn not_modified_response(etag: String, cache_control: &'static str) -> HttpResponse {
    HttpResponse::NotModified()
        .insert_header((header::ETAG, etag))
        .insert_header((header::CACHE_CONTROL, cache_control))
        .finish()
}

pub(crate) fn conditional_bytes_response(
    req: &HttpRequest,
    bytes: Vec<u8>,
    content_type: &'static str,
    cache_control: &'static str,
) -> HttpResponse {
    let etag = weak_etag_for_bytes(&bytes);

    if request_etag_matches(req, &etag) {
        return not_modified_response(etag, cache_control);
    }

    HttpResponse::Ok()
        .insert_header((header::ETAG, etag))
        .insert_header((header::CACHE_CONTROL, cache_control))
        .content_type(content_type)
        .body(bytes)
}

pub(crate) fn request_etag_matches(req: &HttpRequest, etag: &str) -> bool {
    req.headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| if_none_match_contains(value, etag))
}

fn if_none_match_contains(value: &str, etag: &str) -> bool {
    value.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*" || normalized_etag(candidate) == normalized_etag(etag)
    })
}

fn normalized_etag(value: &str) -> &str {
    let value = value.trim();
    if value
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("W/"))
    {
        value[2..].trim()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{if_none_match_contains, weak_etag_for_bytes};

    #[test]
    fn etag_uses_sha256_digest() {
        assert_eq!(
            weak_etag_for_bytes(b"abc"),
            "W/\"sha256-ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\""
        );
    }

    #[test]
    fn if_none_match_accepts_weak_or_strong_tag() {
        let etag = "W/\"sha256-test\"";

        assert!(if_none_match_contains(etag, etag));
        assert!(if_none_match_contains("\"sha256-test\"", etag));
        assert!(if_none_match_contains("\"other\", \"sha256-test\"", etag));
        assert!(if_none_match_contains("*", etag));
        assert!(!if_none_match_contains("\"other\"", etag));
    }
}
