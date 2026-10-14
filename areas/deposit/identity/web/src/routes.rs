//! Every route that changes state is `POST`: the root's `Sec-Fetch-Site` control exempts `GET`.
//! The one deliberate exception is the session's idle-timeout touch in [`identity_core::session`].

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::routing::MethodRouter;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

use crate::shell::IdentityState;

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
/// front of the editor.
///
/// A deliberate copy of `editor-server`'s and `dpe-server`'s extractor rather than a shared
/// dependency: it is stateless and the services sit behind the same ingress. Every copy carries
/// the same tests, and `grep RightmostXffKeyExtractor` finds them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RightmostXffKeyExtractor;

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

/// Per-IP budget for `POST /login`, the endpoint that sends mail.
///
/// Deliberately generous: DaSCH staff and a depositing project team can share
/// one NAT address, so a tight bucket denies sign-in to a whole office. This is
/// not the control that protects the relay quota (the global daily cap is) or
/// bounds guessing (the per-account counter is); what it stops is one host
/// driving the endpoint flat out.
pub const ISSUE_BURST: u32 = 20;
const ISSUE_REPLENISH_SECS: u64 = 30;

/// Per-IP budget for `POST /login/code`. Higher than the issue budget: entering
/// a code is a thing a legitimate user does repeatedly and a thing that sends no
/// mail.
const VERIFY_BURST: u32 = 30;
const VERIFY_REPLENISH_SECS: u64 = 5;

/// A per-IP rate limit keyed by [`RightmostXffKeyExtractor`].
///
/// A macro rather than a function because the layer's type parameters come from
/// the `governor` crate, which `tower_governor` re-exports only in part — naming
/// the return type would mean a direct dependency on `governor` for a
/// signature.
///
/// `replenish_secs` is the interval at which one unit of the burst comes back,
/// so the sustained rate is `1 / replenish_secs` per second per IP.
macro_rules! per_ip_limit {
    ($replenish_secs:expr, $burst:expr, $what:literal) => {{
        use tower_governor::governor::GovernorConfigBuilder;
        use tower_governor::GovernorLayer;

        let config = GovernorConfigBuilder::default()
            .per_second($replenish_secs)
            .burst_size($burst)
            .key_extractor(RightmostXffKeyExtractor)
            .finish()
            .expect(concat!($what, " GovernorConfig should build from its constants"));
        GovernorLayer { config: std::sync::Arc::new(config) }
    }};
}

pub(crate) type Route = (&'static str, MethodRouter<IdentityState>);

/// Declared once: [`crate::Identity::router`] folds them, and [`route_paths`] hands their paths
/// to the root's page-URL normalizer.
pub(crate) fn routes() -> Vec<Route> {
    use axum::routing::{get, post};

    use crate::handlers::{depositors, login};

    vec![
        // The two login endpoints. The rate limit is on the POST alone, merged
        // in rather than layered over the whole route: a limit that counted page
        // loads would spend an office's budget on people reading the form.
        (
            "/login",
            get(login::login_form).merge(post(login::login_submit).layer(per_ip_limit!(
                ISSUE_REPLENISH_SECS,
                ISSUE_BURST,
                "login"
            ))),
        ),
        (
            "/login/code",
            get(login::code_form).merge(post(login::code_submit).layer(per_ip_limit!(
                VERIFY_REPLENISH_SECS,
                VERIFY_BURST,
                "code"
            ))),
        ),
        // Not rate-limited: signing out costs nothing and refusing it would
        // strand a user in a session they asked to end.
        ("/logout", post(login::logout)),
        // --- RDU-only routes ---
        // Every handler below takes identity's `Rdu`, which composes `Authenticated`: no session
        // redirects to login, and a depositor's session gets the 403 page. Each write shares a
        // URL with the `GET` that renders its form, so a rejected submission re-renders somewhere
        // bookmarkable rather than on a bare 405.
        ("/depositors", get(depositors::list).post(depositors::create)),
        ("/depositors/new", get(depositors::create_form)),
        ("/depositors/{id}/edit", get(depositors::edit_form).post(depositors::update)),
        ("/depositors/{id}/remove", get(depositors::remove_form).post(depositors::remove)),
    ]
}

/// The paths of identity's routes, so the root can label a page view with its route's template
/// without seeing `IdentityState`. A path may repeat.
pub fn route_paths() -> impl Iterator<Item = &'static str> {
    routes().into_iter().map(|(path, _)| path)
}

#[cfg(test)]
mod tests {
    /// The rate-limit key extractor: keys on the rightmost (Traefik-appended)
    /// `X-Forwarded-For` entry, never the client-forgeable leftmost one.
    ///
    /// Copied alongside the extractor itself, so this copy is pinned independently rather than
    /// trusting another crate's tests to cover it.
    mod key_extractor {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        use axum::extract::ConnectInfo;
        use axum::http::Request;
        use tower_governor::key_extractor::KeyExtractor;

        use crate::routes::RightmostXffKeyExtractor;

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
}
