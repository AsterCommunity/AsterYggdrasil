//! Network address helpers.

pub use aster_forge_utils::net::is_loopback_host;

use actix_web::http::header::HeaderMap;
use ipnet::IpNet;
use std::net::IpAddr;

pub fn parse_trusted_proxies(trusted_proxies: &[String]) -> Vec<IpNet> {
    trusted_proxies
        .iter()
        .filter_map(|s| {
            s.parse::<IpNet>()
                .or_else(|_| s.parse::<IpAddr>().map(IpNet::from))
                .map_err(|e| tracing::warn!("invalid trusted_proxy entry '{s}': {e}"))
                .ok()
        })
        .collect()
}

pub fn is_trusted_proxy(ip: IpAddr, trusted: &[IpNet]) -> bool {
    trusted.iter().any(|net| net.contains(&ip))
}

pub fn real_ip_from_headers(
    headers: &HeaderMap,
    peer: IpAddr,
    trusted_proxies: &[String],
) -> IpAddr {
    let trusted = parse_trusted_proxies(trusted_proxies);
    real_ip_from_trusted(headers, peer, &trusted)
}

pub fn real_ip_from_trusted(headers: &HeaderMap, peer: IpAddr, trusted: &[IpNet]) -> IpAddr {
    if !trusted.is_empty() && is_trusted_proxy(peer, trusted) {
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .and_then(|p| p.trim().parse::<IpAddr>().ok());
        if let Some(ip) = ip {
            return ip;
        }
    }
    peer
}

#[cfg(test)]
mod tests {
    use super::{is_trusted_proxy, parse_trusted_proxies, real_ip_from_trusted};
    use actix_web::test as actix_test;
    use std::net::IpAddr;

    #[test]
    fn parse_trusted_proxies_accepts_cidr_and_single_ip() {
        let trusted = parse_trusted_proxies(&["10.0.0.0/8".to_string(), "192.168.1.1".to_string()]);

        assert!(is_trusted_proxy("10.0.0.5".parse().unwrap(), &trusted));
        assert!(is_trusted_proxy("192.168.1.1".parse().unwrap(), &trusted));
        assert!(!is_trusted_proxy("203.0.113.1".parse().unwrap(), &trusted));
    }

    #[test]
    fn real_ip_uses_leftmost_xff_only_for_trusted_peer() {
        let trusted = parse_trusted_proxies(&["10.0.0.0/8".to_string()]);
        let req = actix_test::TestRequest::default()
            .insert_header(("X-Forwarded-For", "203.0.113.10, 198.51.100.2"))
            .to_srv_request();

        assert_eq!(
            real_ip_from_trusted(
                req.headers(),
                "10.0.0.5".parse::<IpAddr>().unwrap(),
                &trusted
            ),
            "203.0.113.10".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            real_ip_from_trusted(
                req.headers(),
                "198.51.100.2".parse::<IpAddr>().unwrap(),
                &trusted
            ),
            "198.51.100.2".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn real_ip_falls_back_to_peer_for_invalid_xff() {
        let trusted = parse_trusted_proxies(&["10.0.0.0/8".to_string()]);
        let req = actix_test::TestRequest::default()
            .insert_header(("X-Forwarded-For", "not-an-ip"))
            .to_srv_request();

        assert_eq!(
            real_ip_from_trusted(
                req.headers(),
                "10.0.0.5".parse::<IpAddr>().unwrap(),
                &trusted
            ),
            "10.0.0.5".parse::<IpAddr>().unwrap()
        );
    }
}
