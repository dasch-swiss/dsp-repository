//! Every route that changes state is `POST`: the root's `Sec-Fetch-Site` control exempts `GET` and
//! `HEAD` by necessity, because a navigation from anywhere is a `GET`, so a `GET` that changes
//! state is a `GET` nothing protects. Rust cannot check the converse (a `MethodRouter` cannot be
//! asked which methods it serves), so the assertions live with the handlers, and the one deliberate
//! exception is the session's idle-timeout touch, which identity owns.
//!
//! The editor runs on its own hostname, so paths are root-mounted: there is no `/editor`
//! prefix. A shared origin would defeat the `Sec-Fetch-Site` CSRF control. See
//! `docs/src/editor/routing.md`.

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::routing::MethodRouter;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

use crate::shell::EditorState;

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
/// A deliberate copy of `dpe-server`'s and `identity-web`'s extractor rather than a shared
/// dependency: it is stateless and the services sit behind the same ingress. The root's beacon
/// limit uses this copy. Every copy carries the same tests, and `grep RightmostXffKeyExtractor`
/// finds them all.
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

/// The route the root exempts from its `Sec-Fetch-Site` check (its `csrf.rs` says why). The root
/// reads this constant, so renaming the route moves the exemption with it.
pub const COLLECTION_REPORT_PATH: &str = "/api/v1/collection-report";

/// Per-IP budget for `GET /api/v1/approved-records`.
///
/// Sized for a CI poller checking in every few minutes, not a browser beacon
/// firing on every page: a low sustained rate with a small burst to absorb a
/// poller's retry without opening the door to a scrape of the whole table on
/// a tight loop.
const APPROVED_RECORDS_BURST: u32 = 5;
const APPROVED_RECORDS_REPLENISH_SECS: u64 = 30;

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

/// What a route answers with, which decides what the page-URL normalizer owes it.
///
/// The kind is about the response, not the method: a row action is a `POST`, but
/// its plain path renders the whole page, so it is a [`RouteKind::Page`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteKind {
    /// Renders the page shell for some request, so the page's beacon reports this
    /// path and `page_url.rs` must label it; that module's tests fail otherwise.
    Page,
    /// Machine-facing, under `/api/v1/`: JSON or a bare status, never the shell.
    Api,
    /// A `GET` that redirects. Only its 404 and 403 render the shell, and a
    /// beacon from those reports `other`, which is what they should report.
    Redirect,
}

pub(crate) type Route = (&'static str, RouteKind, MethodRouter<EditorState>);

/// The editor's routes, declared once. [`crate::Editor::router`] folds them, and `page_url.rs`'s
/// tests read their kinds. A route added outside this table escapes that test; the only ones are
/// identity's, pinned by `identity-web`'s own table, and the root's untraced `/healthz` and
/// `/telemetry/collect`, which render no page.
pub(crate) fn routes() -> Vec<Route> {
    use axum::routing::{get, post};
    use RouteKind::{Api, Page, Redirect};

    vec![
        // The service root: a redirect to `/projects`. The shell's header links
        // here from every page, so without it the 404 page's own header is
        // another 404.
        ("/", Redirect, get(crate::shell::root)),
        // Public and unauthenticated on purpose — `crate::handlers::collection::list` takes no
        // `Authenticated`/`Rdu` extractor, which is what "public" looks like from a handler
        // signature. Traced, unlike `/telemetry/collect`: this is the one thing a CI poller
        // actually needs a span for.
        (
            "/api/v1/approved-records",
            Api,
            get(crate::handlers::collection::list).layer(per_ip_limit!(
                APPROVED_RECORDS_REPLENISH_SECS,
                APPROVED_RECORDS_BURST,
                "approved records"
            )),
        ),
        // Also public in the sense of taking no `Authenticated`/`Rdu` extractor, but not
        // unauthenticated: `crate::handlers::collection::report` checks a bearer token itself, because its
        // caller is a CI job with no session to hold. The root's `csrf.rs` holds the matching
        // same-origin exemption this path needs.
        (COLLECTION_REPORT_PATH, Api, post(crate::handlers::collection::report)),
        // --- Authenticated routes ---
        // Every handler below takes `Authenticated`, which is what performs the
        // check: a handler that omits it is public, visibly, in its signature.
        // They take [`crate::handlers::auth`]'s pair.
        ("/projects", Page, get(crate::handlers::projects::list)),
        ("/states", Page, get(crate::handlers::projects::states)),
        // A redirect to the first section. Sections are real URLs rather than
        // fragment swaps, so the form is bookmarkable and Back-friendly.
        ("/projects/{shortcode}", Redirect, get(crate::handlers::projects::detail)),
        // The form. One URL for the section and the save, so a rejected save
        // re-renders at a path that still answers `GET`.
        (
            "/projects/{shortcode}/sections/{section}",
            Page,
            get(crate::handlers::sections::show).post(crate::handlers::sections::act),
        ),
        // Adding and removing a row of a repeatable field. Under the section's
        // own URL, and handled by the same module, so a row action resolves
        // through one `context()` — the audience gate, the lock check and the
        // shortcode fold are the section's rather than a second copy of the rule
        // that decides who may write what.
        (
            "/projects/{shortcode}/sections/{section}/fields/{field}/add",
            Page,
            post(crate::handlers::sections::add_row),
        ),
        (
            "/projects/{shortcode}/sections/{section}/fields/{field}/{key}/remove",
            Page,
            post(crate::handlers::sections::remove_row),
        ),
        // The entity form: one person or organisation proposal. `{proposal}` is the
        // proposal's `entity_id`, the same value `pages::section` already links to — see
        // `entities.rs`'s module docs for why. One `GET`/`POST` pair, like the section
        // form's.
        (
            "/projects/{shortcode}/entities/{proposal}",
            Page,
            get(crate::handlers::entities::show).post(crate::handlers::entities::act),
        ),
        (
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/add",
            Page,
            post(crate::handlers::entities::add_row),
        ),
        (
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove",
            Page,
            post(crate::handlers::entities::remove_row),
        ),
        // --- RDU-only routes ---
        // `Rdu` composes `Authenticated`, so these are closed twice over: no
        // session redirects to login, and a depositor's session gets the 403
        // page. Each write shares a URL with the `GET` that renders its form, so
        // a rejected submission re-renders somewhere bookmarkable rather than on
        // a bare 405.
        //
        // The review surfaces carry claim, save and accept-all as an `intent`
        // pair on one `POST` URL per submission, for the same reason.
        ("/review", Page, get(crate::handlers::review::queue)),
        (
            "/review/{shortcode}",
            Page,
            get(crate::handlers::review::show).post(crate::handlers::review::act),
        ),
        ("/collection", Page, get(crate::handlers::collection::overview)),
        (
            "/collection/{id}/discard",
            Page,
            get(crate::handlers::collection::discard_form).post(crate::handlers::collection::discard),
        ),
    ]
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
