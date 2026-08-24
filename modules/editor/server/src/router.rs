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
//!
//! ## Method discipline
//!
//! Every route that changes state is `POST`. That is not tidiness: the
//! `Sec-Fetch-Site` CSRF control exempts `GET` and `HEAD` by necessity, because
//! a navigation from anywhere is a `GET` — so a `GET` that changes state is a
//! `GET` nothing protects. The tests below assert each state-changing route
//! refuses `GET`.
//!
//! Rust cannot check the converse — that no handler reached through `get(...)`
//! writes — because the route table is not introspectable. The one deliberate
//! exception is the session's idle-timeout touch, and it is argued for where it
//! happens, in [`crate::auth::session`].

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::Router;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

use crate::AppState;

/// The whole app: the traced router plus the routes that must stay untraced,
/// with the CSRF middleware wrapped around all of them.
///
/// `serve()` uses this so the untraced routes cannot drift apart from the ones
/// the tests assemble — the traced/untraced split is positional (an Axum layer
/// wraps only routes declared before it), so it is invisible in the route table
/// and silently reversible by moving one line.
///
/// [`crate::csrf::require_same_origin`] is applied **last, and therefore
/// outermost**, so it covers every route this function returns — including the
/// untraced ones, and including any route added later. Putting it inside
/// `build_router` would have left `/telemetry/collect` unprotected, since that
/// route is declared after `build_router`'s layers.
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
            platform_telemetry::collector::collect_route("editor", crate::page_url::normalize_page_url).layer({
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
        // --- CSRF, outermost so nothing declared above can escape it ---
        .layer(axum::middleware::from_fn(crate::csrf::require_same_origin))
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
/// the same tests, and `grep RightmostXffKeyExtractor` finds both.
///
/// The login endpoints reuse this one rather than adding a third copy, so the
/// count of copies is still two and the case for extracting it into a shared
/// crate has not moved. It moves when a third *service* needs it.
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
/// Deliberately generous. DaSCH staff and a depositing project team can share
/// one NAT address, so a tight bucket denies sign-in to a whole office — a
/// self-inflicted outage in exchange for very little, because this is not the
/// control that protects the relay quota (the global daily cap is) or bounds
/// guessing (the per-account counter is). What it stops is one host driving the
/// endpoint flat out.
const ISSUE_BURST: u32 = 20;
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
/// the return type would mean taking a direct dependency on `governor` for a
/// signature. The beacon route builds its config inline for the same reason.
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

/// Assemble the traced app router. Static assets are served from `public_dir`,
/// falling back to the app's 404.
///
/// Everything declared here is wrapped by the OTel layers. Routes that must
/// stay **untraced** — `/healthz` and the telemetry beacon — are added by
/// [`build_app`] after this returns.
fn build_router(state: AppState, public_dir: &std::path::Path) -> Router {
    use axum::routing::{get, post};
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
        // The two login endpoints. The rate limit is on the POST alone, merged
        // in rather than layered over the whole route: a limit that counted page
        // loads would spend an office's budget on people reading the form.
        .route(
            "/login",
            get(crate::auth::login_form).merge(post(crate::auth::login_submit).layer(per_ip_limit!(
                ISSUE_REPLENISH_SECS,
                ISSUE_BURST,
                "login"
            ))),
        )
        .route(
            "/login/code",
            get(crate::auth::code_form).merge(post(crate::auth::code_submit).layer(per_ip_limit!(
                VERIFY_REPLENISH_SECS,
                VERIFY_BURST,
                "code"
            ))),
        )
        // Not rate-limited: signing out costs nothing and refusing it would
        // strand a user in a session they asked to end.
        .route("/logout", post(crate::auth::logout))
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

    use super::{build_app, build_router, ISSUE_BURST};
    use crate::test_support::test_state;

    // Static assets come from a nonexistent dir: these tests target the
    // fallback and /healthz, never a real static file.
    const NO_PUBLIC_DIR: &str = "nonexistent-test-dir";

    async fn status_of(app: axum::Router, uri: &str) -> StatusCode {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_not_found() {
        assert_eq!(
            status_of(build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref()), "/no-such-page").await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn root_is_served_so_the_shell_header_is_not_a_dead_end() {
        // Every page's header links to `/`. Without this route the 404 page's own
        // header led straight back to another 404.
        assert_eq!(
            status_of(build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref()), "/").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn healthz_is_ok() {
        assert_eq!(
            status_of(build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref()), "/healthz").await,
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
            status_of(build_router(test_state("router").await.0, NO_PUBLIC_DIR.as_ref()), "/healthz").await,
            StatusCode::NOT_FOUND
        );
    }

    /// A beacon request as Traefik delivers it. The `x-forwarded-for` header is
    /// not optional decoration: the rate limiter keys on it, and with neither an
    /// XFF header nor `ConnectInfo` the extractor cannot produce a key and
    /// `tower_governor` answers 500. In production Traefik always appends one.
    ///
    /// `sec-fetch-site` is not decoration either: the CSRF middleware wraps the
    /// beacon too, and `navigator.sendBeacon` to the page's own origin sends
    /// exactly this value.
    fn beacon_request(method: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri("/telemetry/collect")
            .header("x-forwarded-for", "203.0.113.7")
            .header("origin", "https://edit.dasch.swiss")
            .header("sec-fetch-site", "same-origin")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"signals":[],"connection":null}"#))
            .unwrap()
    }

    #[tokio::test]
    async fn telemetry_beacon_is_wired_and_untraced() {
        // Same positional invariant as /healthz: the beacon must sit after the
        // OTel layers, or every page's telemetry upload mints a server span.
        let app = build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref());
        let status = app.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::NO_CONTENT);

        // Absent from the traced router, so it cannot have drifted above the
        // `.layer()` calls. An unmatched POST lands on the `ServeDir` fallback's
        // not-found service, which is GET-only and so answers 405 rather than
        // 404 — the point is that it is not the beacon's 204.
        let traced = build_router(test_state("router").await.0, NO_PUBLIC_DIR.as_ref());
        let status = traced.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_ne!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn telemetry_beacon_rejects_get() {
        // The beacon mutates nothing, but method discipline is the invariant the
        // no-state-change-on-GET rule rests on; a GET must not reach it.
        let app = build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref());
        let status = app.oneshot(beacon_request("GET")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn telemetry_beacon_is_rate_limited_per_ip() {
        // Burst is 10 at 1/s; the 11th back-to-back request from one IP must be
        // throttled. Without this the beacon is the one endpoint an anonymous
        // client can drive without limit.
        let app = build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref());
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
            .header("sec-fetch-site", "same-origin")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"signals":[],"connection":null}"#))
            .unwrap();
        assert_eq!(app.oneshot(other).await.unwrap().status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn the_login_rate_limit_counts_submissions_and_not_page_loads() {
        // The limit is merged onto the POST alone rather than layered over the
        // whole route, and that is not decoration: DaSCH staff and a depositing
        // project team can share one NAT address, so a limiter that counted GETs
        // would spend an office's entire budget on people reading the form and
        // lock all of them out of signing in.
        //
        // Pins both halves, because the mechanism — `MethodRouter::merge` of a
        // layered `post(...)` into an unlayered `get(...)` — is invisible in the
        // route table and would silently start covering GET if someone replaced
        // the merge with a `.layer()` on the whole route.
        let (state, _) = test_state("rate-limit").await;
        let app = build_app(state, NO_PUBLIC_DIR.as_ref());

        let mut form_loads = Vec::new();
        for _ in 0..(ISSUE_BURST + 10) {
            let request = Request::builder()
                .method("GET")
                .uri("/login")
                .header("x-forwarded-for", "198.51.100.77")
                .body(Body::empty())
                .unwrap();
            form_loads.push(app.clone().oneshot(request).await.unwrap().status());
        }
        assert!(
            !form_loads.contains(&StatusCode::TOO_MANY_REQUESTS),
            "reading the form must never be throttled, got {form_loads:?}"
        );

        let mut submissions = Vec::new();
        for _ in 0..(ISSUE_BURST + 5) {
            let request = Request::builder()
                .method("POST")
                .uri("/login")
                .header("x-forwarded-for", "198.51.100.77")
                .header("sec-fetch-site", "same-origin")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("email=nobody@example.test"))
                .unwrap();
            submissions.push(app.clone().oneshot(request).await.unwrap().status());
        }
        assert!(
            submissions.contains(&StatusCode::TOO_MANY_REQUESTS),
            "submitting must be throttled past the burst, got {submissions:?}"
        );

        // A different address has its own bucket, so one noisy client cannot
        // deny sign-in to everyone behind a different egress.
        let other = Request::builder()
            .method("POST")
            .uri("/login")
            .header("x-forwarded-for", "203.0.113.201")
            .header("sec-fetch-site", "same-origin")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("email=nobody@example.test"))
            .unwrap();
        assert_eq!(app.oneshot(other).await.unwrap().status(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn csrf_middleware_covers_the_untraced_routes_too() {
        // The CSRF layer is applied last in `build_app`, which makes it
        // outermost and therefore the one layer the positional traced/untraced
        // split cannot route around. Pinning it on the beacon specifically: the
        // beacon is declared *after* the OTel layers, so a CSRF layer added
        // inside `build_router` would have left it — the only pre-auth POST in
        // the app — unprotected, with no test failing.
        let app = build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref());
        let unprotected = Request::builder()
            .method("POST")
            .uri("/telemetry/collect")
            .header("x-forwarded-for", "203.0.113.7")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"signals":[],"connection":null}"#))
            .unwrap();
        assert_eq!(
            app.oneshot(unprotected).await.unwrap().status(),
            StatusCode::FORBIDDEN,
            "a POST with no Sec-Fetch-Site must be refused before it reaches the beacon"
        );
    }

    #[tokio::test]
    async fn healthz_stays_reachable_without_sec_fetch_site() {
        // The liveness probe is a GET from Traefik, which sends no
        // `Sec-Fetch-*`. A CSRF check that applied to GET would fail every
        // probe and take the service out of rotation.
        assert_eq!(
            status_of(build_app(test_state("router").await.0, NO_PUBLIC_DIR.as_ref()), "/healthz").await,
            StatusCode::OK
        );
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
