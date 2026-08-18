//! App router assembly, kept separate from `serve()` (which does I/O + global
//! setup) so the routing is unit-testable.
//!
//! ## URL scheme
//!
//! The editor runs on its own hostname, so paths are root-mounted — there is no
//! `/editor` prefix. DPE carries `/dpe/…` only because it shares
//! `repository.dasch.swiss`; adopting a prefix here would keep alive the
//! path-routing option the plan rejects, since a shared origin defeats the
//! `Sec-Fetch-Site` CSRF control. See `docs/src/editor/architecture.md`.
//!
//! Routes are added as their surfaces land. What this module owns from the
//! start is the layer order and the traced/untraced split.

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::Router;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

use crate::AppState;

/// The whole app: the traced router plus the routes that must stay untraced.
///
/// `serve()` uses this so the untraced routes cannot drift apart from the ones
/// the tests assemble — the traced/untraced split is positional (an Axum layer
/// wraps only routes declared before it), so it is invisible in the route table
/// and silently reversible by moving one line.
pub(crate) fn build_app(state: AppState, public_dir: &std::path::Path) -> Router {
    use axum::http::StatusCode;
    use axum::routing::get;

    build_router(state, public_dir)
        // --- Untraced routes (declared AFTER build_router's .layer() calls) ---
        // A liveness probe every 30s would otherwise mint one span per probe
        // and bury the real traffic.
        .route("/healthz", get(|| async { StatusCode::OK }))
        // The browser beacon: fire-and-forget, unauthenticated, and reachable
        // before login, so it is rate-limited per client IP. Untraced for the
        // same reason as /healthz — tracing it would attach a server span to
        // every page's telemetry upload. Shared with DPE via `platform-telemetry`;
        // "editor" names the OTel instrumentation scope (`editor.browser`).
        .route(
            "/telemetry/collect",
            platform_telemetry::collector::collect_route("editor").layer({
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::GovernorLayer;

                let config = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(RightmostXffKeyExtractor)
                    .finish()
                    .expect("beacon GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(config) }
            }),
        )
}

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
/// Deliberately a copy of `dpe-server`'s extractor rather than a shared
/// dependency: it is stateless, both services sit behind the same ingress, and a
/// shared home for fifteen lines is not worth a crate today. Both copies carry
/// the same tests, and `grep RightmostXffKeyExtractor` finds both — extract it
/// once a third call site appears (the login endpoints will be one).
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

/// Assemble the traced app router. Static assets are served from `public_dir`,
/// falling back to the app's 404.
///
/// Everything declared here is wrapped by the OTel layers. Routes that must
/// stay **untraced** — `/healthz` and the telemetry beacon — are added by
/// [`build_app`] after this returns.
fn build_router(state: AppState, public_dir: &std::path::Path) -> Router {
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
    use tower_http::services::ServeDir;

    // Static assets + 404: serve files from the public dir, falling back to the
    // "Page not found" shell.
    let serve_dir = ServeDir::new(public_dir).not_found_service(get(crate::not_found).with_state(state.clone()));

    Router::new()
        // --- Traced routes (declared BEFORE .layer()) ---
        // The service root. The shell's header links here from every page, so
        // without it the 404 page's own header led to another 404.
        .route("/", get(crate::root))
        // Static assets + 404 fallback.
        .fallback_service(serve_dir)
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    use super::{build_app, build_router};
    use crate::AppState;

    // Static assets come from a nonexistent dir: these tests target the
    // fallback and /healthz, never a real static file.
    const NO_PUBLIC_DIR: &str = "nonexistent-test-dir";

    fn test_state() -> AppState {
        AppState { css_href: "/assets/app.css".to_string() }
    }

    async fn status_of(app: axum::Router, uri: &str) -> StatusCode {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_not_found() {
        assert_eq!(
            status_of(build_app(test_state(), NO_PUBLIC_DIR.as_ref()), "/no-such-page").await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn root_is_served_so_the_shell_header_is_not_a_dead_end() {
        // Every page's header links to `/`. Without this route the 404 page's own
        // header led straight back to another 404.
        assert_eq!(
            status_of(build_app(test_state(), NO_PUBLIC_DIR.as_ref()), "/").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn healthz_is_ok() {
        assert_eq!(
            status_of(build_app(test_state(), NO_PUBLIC_DIR.as_ref()), "/healthz").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn healthz_is_not_part_of_the_traced_router() {
        // The traced/untraced split is positional — an Axum layer wraps only
        // routes declared before it — so it is invisible in the route table and
        // reversible by moving one line. Pinning it here: `/healthz` answers
        // through `build_app` but is absent from `build_router`, so moving its
        // declaration above the `.layer()` calls turns this 404 into a 200 and
        // fails the test, instead of silently minting a span per liveness probe.
        //
        // Asserted structurally rather than via the traceparent response header
        // `OtelInResponseLayer` injects: that header only appears once the OTel
        // subscriber is installed, which is `serve()`'s job, not a unit test's.
        assert_eq!(
            status_of(build_router(test_state(), NO_PUBLIC_DIR.as_ref()), "/healthz").await,
            StatusCode::NOT_FOUND
        );
    }

    /// A beacon request as Traefik delivers it. The `x-forwarded-for` header is
    /// not optional decoration: the rate limiter keys on it, and with neither an
    /// XFF header nor `ConnectInfo` the extractor cannot produce a key and
    /// `tower_governor` answers 500. In production Traefik always appends one.
    fn beacon_request(method: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri("/telemetry/collect")
            .header("x-forwarded-for", "203.0.113.7")
            .header("origin", "https://edit.dasch.swiss")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"signals":[],"connection":null}"#))
            .unwrap()
    }

    #[tokio::test]
    async fn telemetry_beacon_is_wired_and_untraced() {
        // Same positional invariant as /healthz: the beacon must sit after the
        // OTel layers, or every page's telemetry upload mints a server span.
        let app = build_app(test_state(), NO_PUBLIC_DIR.as_ref());
        let status = app.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::NO_CONTENT);

        // Absent from the traced router, so it cannot have drifted above the
        // `.layer()` calls. An unmatched POST lands on the `ServeDir` fallback's
        // not-found service, which is GET-only and so answers 405 rather than
        // 404 — the point is that it is not the beacon's 204.
        let traced = build_router(test_state(), NO_PUBLIC_DIR.as_ref());
        let status = traced.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_ne!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn telemetry_beacon_rejects_get() {
        // The beacon mutates nothing, but method discipline is the invariant the
        // no-state-change-on-GET rule rests on; a GET must not reach it.
        let app = build_app(test_state(), NO_PUBLIC_DIR.as_ref());
        let status = app.oneshot(beacon_request("GET")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn telemetry_beacon_is_rate_limited_per_ip() {
        // Burst is 10 at 1/s; the 11th back-to-back request from one IP must be
        // throttled. Without this the beacon is the one endpoint an anonymous
        // client can drive without limit.
        let app = build_app(test_state(), NO_PUBLIC_DIR.as_ref());
        let mut statuses = Vec::new();
        for _ in 0..12 {
            statuses.push(app.clone().oneshot(beacon_request("POST")).await.unwrap().status());
        }
        assert!(
            statuses.contains(&StatusCode::TOO_MANY_REQUESTS),
            "expected throttling within 12 requests, got {statuses:?}"
        );

        // A different IP has its own bucket, so one noisy client cannot deny the
        // endpoint to everyone else.
        let other = Request::builder()
            .method("POST")
            .uri("/telemetry/collect")
            .header("x-forwarded-for", "198.51.100.4")
            .header("origin", "https://edit.dasch.swiss")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"signals":[],"connection":null}"#))
            .unwrap();
        assert_eq!(app.oneshot(other).await.unwrap().status(), StatusCode::NO_CONTENT);
    }

    /// The rate-limit key extractor: keys on the rightmost (Traefik-appended)
    /// `X-Forwarded-For` entry, never the client-forgeable leftmost one.
    ///
    /// Copied alongside the extractor itself from `dpe-server`, so the editor's
    /// copy is pinned independently rather than trusting DPE's tests to cover it.
    mod key_extractor {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        use axum::extract::ConnectInfo;
        use axum::http::Request;
        use tower_governor::key_extractor::KeyExtractor;

        use crate::router::RightmostXffKeyExtractor;

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
