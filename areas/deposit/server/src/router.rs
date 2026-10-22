//! App router assembly, kept separate from `serve()` (which does I/O + global
//! setup) so the routing is unit-testable. What this module owns is the layer
//! order and the traced/untraced split: the editor's and identity's routers
//! arrive built, and their route tables live with them.
//!
//! The editor runs on its own hostname, so paths are root-mounted — there is no
//! `/editor` prefix. A shared origin would defeat the `Sec-Fetch-Site` CSRF
//! control. See `docs/src/editor/routing.md`.
//!
//! **Method discipline.** Every route that changes state is `POST`: the
//! `Sec-Fetch-Site` control exempts `GET` and `HEAD` by necessity, because a
//! navigation from anywhere is a `GET`, so a `GET` that changes state is a `GET`
//! nothing protects. Rust cannot check the converse — a `MethodRouter` cannot
//! be asked which methods it serves — so the assertions live with the
//! handlers, and the one deliberate exception is the session's idle-timeout
//! touch in `identity-core`'s session rules.

use axum::Router;

/// The whole app: the traced router plus the routes that must stay untraced,
/// with the CSRF middleware wrapped around all of them.
///
/// The traced/untraced split is positional — an Axum layer wraps only routes
/// declared before it — so it is invisible in the route table and silently
/// reversible by moving one line. `serve()` uses this function so the untraced
/// routes cannot drift from the ones the tests assemble.
///
/// [`crate::csrf::require_same_origin`] is applied **last, and therefore
/// outermost**, so it covers every route here including any added later. Putting
/// it inside `build_router` would leave `/telemetry/collect` unprotected.
pub(crate) fn build_app(identity: Router, editor: Router, css_href: String, public_dir: &std::path::Path) -> Router {
    use axum::http::StatusCode;
    use axum::routing::get;

    build_router(identity, editor, css_href, public_dir)
        // --- Untraced routes (declared AFTER build_router's .layer() calls) ---
        // A liveness probe every 30s would otherwise mint one span per probe
        // and bury the real traffic.
        .route("/healthz", get(|| async { StatusCode::OK }))
        // The browser beacon: fire-and-forget, unauthenticated, and reachable
        // before login, so it is rate-limited per client IP. Untraced for the
        // same reason as /healthz — tracing it would attach a server span to
        // every page's telemetry upload. Shared with DPE via `shared-telemetry`;
        // "editor" names the OTel instrumentation scope (`editor.browser`).
        .route(
            "/telemetry/collect",
            shared_telemetry::collector::collect_route("editor", crate::page_url::normalize_page_url).layer({
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::GovernorLayer;

                let config = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(shared_rate_limit::RightmostXffKeyExtractor)
                    .finish()
                    .expect("beacon GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(config) }
            }),
        )
        // --- CSRF, outermost so nothing declared above can escape it ---
        .layer(axum::middleware::from_fn(crate::csrf::require_same_origin))
}

/// Assemble the traced app router: the editor's routes merged with identity's. Static assets are
/// served from `public_dir`, falling back to the app's 404.
///
/// Every route in `editor` and `identity` is wrapped by the OTel layers. Routes that must stay
/// **untraced** — `/healthz` and the telemetry beacon — are added by [`build_app`] after this
/// returns.
fn build_router(identity: Router, editor: Router, css_href: String, public_dir: &std::path::Path) -> Router {
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
    use tower_http::services::ServeDir;

    let serve_dir = ServeDir::new(public_dir).not_found_service(get(crate::shell::not_found).with_state(css_href));

    // --- Traced routes (declared BEFORE .layer()) ---
    editor
        .merge(identity)
        // Static assets + 404 fallback.
        .fallback_service(serve_dir)
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use identity_web::ISSUE_BURST;
    use tower::ServiceExt;

    use super::{build_app, build_router};
    use crate::test_support::{editor_router, identity_router, test_state, TestState, CSS_HREF};

    // Static assets come from a nonexistent dir: these tests target the
    // fallback and /healthz, never a real static file.
    const NO_PUBLIC_DIR: &str = "nonexistent-test-dir";

    fn app(state: TestState) -> axum::Router {
        build_app(
            identity_router(&state),
            editor_router(&state),
            CSS_HREF.to_string(),
            NO_PUBLIC_DIR.as_ref(),
        )
    }

    fn traced(state: TestState) -> axum::Router {
        build_router(
            identity_router(&state),
            editor_router(&state),
            CSS_HREF.to_string(),
            NO_PUBLIC_DIR.as_ref(),
        )
    }

    async fn status_of(app: axum::Router, uri: &str) -> StatusCode {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_not_found() {
        assert_eq!(
            status_of(app(test_state("router").await.0), "/no-such-page").await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn root_is_served_so_the_shell_header_is_not_a_dead_end() {
        // Every page's header links to `/`, so without this route the 404 page's
        // own header is another 404. A redirect rather than a page, so the
        // project list is the single place that decides what a signed-out
        // visitor gets.
        let app = app(test_state("router").await.0);
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/projects")
        );
    }

    #[tokio::test]
    async fn the_project_routes_are_closed_to_a_request_with_no_session() {
        // The guard is an extractor, so this is what "closed by default" looks
        // like from outside: no session, no page, and the destination is kept.
        let app = app(test_state("router").await.0);
        for (uri, expected) in [
            ("/projects", "/login?next=/projects"),
            ("/projects/0801", "/login?next=/projects/0801"),
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{uri}");
            assert_eq!(
                response
                    .headers()
                    .get(axum::http::header::LOCATION)
                    .and_then(|v| v.to_str().ok()),
                Some(expected),
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn healthz_is_ok() {
        assert_eq!(status_of(app(test_state("router").await.0), "/healthz").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn healthz_is_not_part_of_the_traced_router() {
        // Pinning the positional split: `/healthz` answers through `build_app`
        // but is absent from `build_router`, so moving its declaration above the
        // `.layer()` calls turns this 404 into a 200 and fails the test, instead
        // of silently minting a span per liveness probe.
        //
        // Asserted structurally rather than via the traceparent response header,
        // which only appears once the OTel subscriber is installed — `serve()`'s
        // job, not a unit test's.
        assert_eq!(
            status_of(traced(test_state("router").await.0), "/healthz").await,
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
        let app = app(test_state("router").await.0);
        let status = app.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::NO_CONTENT);

        // Absent from the traced router, so it cannot have drifted above the
        // `.layer()` calls. An unmatched POST lands on the `ServeDir` fallback's
        // not-found service, which is GET-only and so answers 405 rather than
        // 404 — the point is that it is not the beacon's 204.
        let traced = traced(test_state("router").await.0);
        let status = traced.oneshot(beacon_request("POST")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_ne!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn telemetry_beacon_rejects_get() {
        // The beacon mutates nothing, but method discipline is the invariant the
        // no-state-change-on-GET rule rests on; a GET must not reach it.
        let app = app(test_state("router").await.0);
        let status = app.oneshot(beacon_request("GET")).await.unwrap().status();
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn telemetry_beacon_is_rate_limited_per_ip() {
        // Burst is 10 at 1/s; the 11th back-to-back request from one IP must be
        // throttled. Without this the beacon is the one endpoint an anonymous
        // client can drive without limit.
        let app = app(test_state("router").await.0);
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
        // Pins both halves: the mechanism — `MethodRouter::merge` of a layered
        // `post(...)` into an unlayered `get(...)` — is invisible in the route
        // table and would silently start covering `GET` if someone replaced the
        // merge with a `.layer()` on the whole route, spending an office's shared
        // NAT budget on people reading the form.
        let (state, _) = test_state("rate-limit").await;
        let app = app(state);

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
        // Pinning the outermost CSRF layer on the beacon specifically: the beacon
        // is declared *after* the OTel layers, so a CSRF layer added inside
        // `build_router` would have left it — the only pre-auth POST in the app
        // — unprotected, with no test failing.
        let app = app(test_state("router").await.0);
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
        assert_eq!(status_of(app(test_state("router").await.0), "/healthz").await, StatusCode::OK);
    }
}
