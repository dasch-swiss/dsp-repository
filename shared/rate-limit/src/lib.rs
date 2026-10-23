//! The per-client-IP key the services' `tower_governor` limiters bucket on.
//!
//! [`RightmostXffKeyExtractor`] is the key extractor every per-IP limiter uses. Each service keeps
//! its own limiter configuration and quotas; only the extractor lives here.

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

/// Rate-limit key extractor that keys on the **rightmost** `X-Forwarded-For`
/// entry — the address our reverse proxy (Traefik) itself appended — falling
/// back to the connection peer IP.
///
/// SECURITY: `tower_governor`'s stock `SmartIpKeyExtractor` reads the *leftmost*
/// XFF entry, which is client-forgeable. Traefik appends the real client IP
/// after any value the client supplied, so the leftmost entry stays
/// attacker-controlled; a single client can rotate it to mint unlimited
/// rate-limit buckets and defeat the per-IP limit. The rightmost entry is the
/// one Traefik wrote and cannot be spoofed, given Traefik is the only hop in
/// front of the services. This mirrors SIPI's `client_ip` resolver, which is
/// deployed behind the same ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightmostXffKeyExtractor;

impl KeyExtractor for RightmostXffKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .or_else(|| req.extensions().get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip()))
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use tower_governor::key_extractor::KeyExtractor;

    use crate::RightmostXffKeyExtractor;

    fn extract(xff: Option<&str>, peer: Option<SocketAddr>) -> Result<IpAddr, ()> {
        let mut b = Request::builder();
        if let Some(v) = xff {
            b = b.header("x-forwarded-for", v);
        }
        let mut req = b.body(()).unwrap();
        if let Some(addr) = peer {
            req.extensions_mut().insert(ConnectInfo(addr));
        }
        RightmostXffKeyExtractor.extract(&req).map_err(|_| ())
    }

    fn ip(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn takes_rightmost_xff() {
        assert_eq!(extract(Some("1.1.1.1, 2.2.2.2, 3.3.3.3"), None), Ok(ip(3, 3, 3, 3)));
    }

    #[test]
    fn single_xff() {
        assert_eq!(extract(Some("4.4.4.4"), None), Ok(ip(4, 4, 4, 4)));
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(extract(Some("1.1.1.1,  5.5.5.5 "), None), Ok(ip(5, 5, 5, 5)));
    }

    #[test]
    fn ignores_client_forged_leftmost() {
        // A client that prepends a fake IP cannot change the key: Traefik
        // appends the real client last, and we read the last entry.
        assert_eq!(extract(Some("9.9.9.9, 6.6.6.6"), None), Ok(ip(6, 6, 6, 6)));
    }

    #[test]
    fn falls_back_to_peer_when_no_xff() {
        let peer = SocketAddr::new(ip(7, 7, 7, 7), 40000);
        assert_eq!(extract(None, Some(peer)), Ok(ip(7, 7, 7, 7)));
    }

    #[test]
    fn falls_back_to_peer_when_rightmost_unparseable() {
        let peer = SocketAddr::new(ip(8, 8, 8, 8), 40000);
        assert_eq!(extract(Some("garbage"), Some(peer)), Ok(ip(8, 8, 8, 8)));
    }

    #[test]
    fn errors_when_no_xff_and_no_peer() {
        assert_eq!(extract(None, None), Err(()));
    }
}
