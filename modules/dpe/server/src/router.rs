//! App router assembly, kept separate from `serve()` (which does I/O + global
//! setup) so the routing — including the per-route rate limiter — is
//! unit-testable.

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::Request;
use axum::Router;
use tower_governor::key_extractor::KeyExtractor;
use tower_governor::GovernorError;

use crate::config::DpeConfig;
use crate::{about_page_handler, fragments, project_page_handler, projects_page_handler, AppState};

/// Rate-limit key extractor that keys on the **rightmost** `X-Forwarded-For`
/// entry — the address our reverse proxy (Traefik) itself appended — falling back
/// to the connection peer IP.
///
/// SECURITY: `tower_governor`'s stock `SmartIpKeyExtractor` reads the *leftmost*
/// XFF entry, which is client-forgeable. Traefik appends the real client IP after
/// any value the client supplied, so the leftmost entry stays attacker-controlled;
/// a single client can rotate it to mint unlimited rate-limit buckets and defeat
/// the per-IP limit. The rightmost entry is the one Traefik wrote and cannot be
/// spoofed, given Traefik is the only hop in front of DPE. This mirrors SIPI's
/// `client_ip` resolver, which is deployed behind the same ingress.
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

/// The rate-limited routes as a standalone sub-router with `limiter` applied to
/// them: `/dpe/oai` and the two machine-readable representations of a project.
///
/// One layer over all three, so they share a per-IP bucket and one set of
/// `DPE_OAI_RATE_LIMIT_*` settings. The representations belong here from their
/// first day rather than after measurement: the JSON-LD one serves an uncapped
/// `hasPart`, which for the largest committed project is a few megabytes built
/// in memory per request.
///
/// The limiter type is erased here — the result is a plain `Router<AppState>` the
/// caller merges in — so `build_router` never has to name the `GovernorLayer`
/// type. In production `limiter` is the real `GovernorLayer` (see
/// [`rate_limited_router`]); tests pass a fake to drive gating deterministically.
pub(crate) fn rate_limited_router_with<L>(limiter: L) -> Router<AppState>
where
    L: tower::Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
    L::Service: tower::Service<axum::extract::Request, Response = axum::response::Response, Error = std::convert::Infallible>
        + Clone
        + Send
        + Sync
        + 'static,
    <L::Service as tower::Service<axum::extract::Request>>::Future: Send + 'static,
{
    use axum::routing::get;
    Router::new()
        .route("/dpe/oai", get(dpe_api_oai::oai_handler))
        // The suffixes are `metadata::REPRESENTATIONS`' second column; that
        // table is what builds the URLs the page links and negotiates to.
        .route(
            "/dpe/projects/{id}/metadata.jsonld",
            get(crate::metadata::project_json_ld_handler),
        )
        .route(
            "/dpe/projects/{id}/metadata.datacite.json",
            get(crate::metadata::project_datacite_json_handler),
        )
        .route_layer(limiter)
}

/// The production rate-limited sub-router, limited per-IP from config.
/// `use_headers()` adds `X-RateLimit-*` and `Retry-After` to 429 responses.
pub(crate) fn rate_limited_router(config: &DpeConfig) -> Router<AppState> {
    use tower_governor::governor::GovernorConfigBuilder;
    use tower_governor::GovernorLayer;

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.oai_rate_limit_per_second)
        .burst_size(config.oai_rate_limit_burst)
        .key_extractor(RightmostXffKeyExtractor)
        .use_headers()
        .finish()
        .expect("OAI GovernorConfig should build from valid config");

    rate_limited_router_with(GovernorLayer { config: std::sync::Arc::new(governor_conf) })
}

/// Assemble the traced app router. `rate_limited` carries the (already
/// rate-limited) `/dpe/oai` route and the two representation routes; passing it
/// in — rather than a bare layer — keeps this signature free of the
/// `GovernorLayer` trait bounds, so it stays reconstructable by hand and lets
/// tests substitute a fake-limited sub-router. Static assets are served from
/// `public_dir`, falling back to the app's 404 shell.
pub(crate) fn build_router(state: AppState, public_dir: &std::path::Path, rate_limited: Router<AppState>) -> Router {
    use axum::response::Redirect;
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
    use tower_http::services::ServeDir;

    let serve_dir = ServeDir::new(public_dir).not_found_service(get(crate::not_found).with_state(state.clone()));

    Router::new()
        // --- Traced routes (declared BEFORE .layer()) ---
        .route("/", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe/projects", get(projects_page_handler))
        .route("/dpe/about", get(about_page_handler))
        .route("/dpe/projects/{id}", get(project_page_handler))
        // Not under /dpe/oai: that is `?verb=`-dispatched, and its limiter must
        // not apply here.
        .route(
            "/dpe/records/{shortcode}/{record_id}/file",
            get(crate::downloads::record_file_handler),
        )
        // OAI-PMH (note: /dpe/oai, not /oai) — XML, must stay unbroken — and
        // the two machine-readable representations. Rate-limited per-IP; the
        // limiter is scoped to those routes only (see `rate_limited_router`).
        .merge(rate_limited)
        .route("/dpe/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
        .route("/dpe/projects/search", get(fragments::search_fragment_handler))
        .route("/dpe/api/v2/projects", get(fragments::projects_json_handler))
        .route("/dpe/api/v2/projects/{id}", get(fragments::project_json_handler))
        .fallback_service(serve_dir)
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .with_state(state)
}

/// Tests for the rate-limit *seam*: that the limiter is wired onto `/dpe/oai`
/// and the two representation routes and nothing else. The actual throttling
/// algorithm is `tower_governor`'s and is not
/// re-tested here. Fake layers (`AllowAll`/`DenyAll`) stand in for the real
/// `GovernorLayer` so gating is deterministic and independent of timing.
///
/// Each fake is defined inside the submodule of the single test that uses it,
/// so the source itself shows the fake cannot leak to another test.
/// `test_state` and `NO_PUBLIC_DIR` come from `test_support`, shared with the
/// landing-page metadata tests so both build the same app.
#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    use crate::test_support::{test_state, NO_PUBLIC_DIR};

    async fn status_of(app: axum::Router, uri: &str) -> StatusCode {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        app.oneshot(req).await.unwrap().status()
    }

    /// `DenyAll` — a fake limiter that rejects every request with 429 without
    /// calling inner. Scoped to `deny_all::gates_oai_only`.
    mod deny_all {
        use std::convert::Infallible;
        use std::task::{Context, Poll};

        use axum::body::Body;
        use axum::extract::Request;
        use axum::http::StatusCode;
        use axum::response::Response;
        use tower::{Layer, Service};

        use super::{status_of, test_state, NO_PUBLIC_DIR};
        use crate::router::{build_router, rate_limited_router_with};

        #[derive(Clone)]
        struct DenyAll;

        #[derive(Clone)]
        struct DenyAllService<S> {
            // DenyAll short-circuits, so inner is never called — held only to satisfy Layer.
            _inner: S,
        }

        impl<S> Layer<S> for DenyAll {
            type Service = DenyAllService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                DenyAllService { _inner: inner }
            }
        }

        impl<S> Service<Request> for DenyAllService<S>
        where
            S: Service<Request, Response = Response, Error = Infallible>,
        {
            type Response = Response;
            type Error = Infallible;
            type Future = std::future::Ready<Result<Response, Infallible>>;

            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: Request) -> Self::Future {
                let resp = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Body::empty())
                    .unwrap();
                std::future::ready(Ok(resp))
            }
        }

        #[tokio::test]
        async fn gates_oai_and_the_representations() {
            // The `/dpe` redirect is a pure handler (no data/cache access), so it is a
            // stable "not rate-limited" control that avoids the set_data_dir global.
            let app = build_router(test_state(), NO_PUBLIC_DIR.as_ref(), rate_limited_router_with(DenyAll));
            for uri in [
                "/dpe/oai",
                "/dpe/projects/0862/metadata.jsonld",
                "/dpe/projects/0862/metadata.datacite.json",
            ] {
                assert_eq!(status_of(app.clone(), uri).await, StatusCode::TOO_MANY_REQUESTS, "{uri}");
            }
            assert_eq!(status_of(app, "/dpe").await, StatusCode::PERMANENT_REDIRECT);
        }

        /// Pins the scope: widening the limiter here would throttle downloads,
        /// and it must not reach the landing page a person reads either.
        #[tokio::test]
        async fn does_not_gate_the_record_file_or_landing_page_routes() {
            let app = build_router(test_state(), NO_PUBLIC_DIR.as_ref(), rate_limited_router_with(DenyAll));
            for uri in ["/dpe/records/0862/RMgW_EICR3OLcMi7LNE=Sgu/file", "/dpe/projects/0862"] {
                assert_ne!(status_of(app.clone(), uri).await, StatusCode::TOO_MANY_REQUESTS, "{uri}");
            }
        }
    }

    /// `AllowAll` — a fake limiter that lets every request through unchanged.
    /// Scoped to `allow_all::passes_the_limited_routes_through`.
    mod allow_all {
        use std::convert::Infallible;
        use std::task::{Context, Poll};

        use axum::extract::Request;
        use axum::http::StatusCode;
        use axum::response::Response;
        use tower::{Layer, Service};

        use super::{status_of, test_state, NO_PUBLIC_DIR};
        use crate::router::{build_router, rate_limited_router_with};

        #[derive(Clone)]
        struct AllowAll;

        #[derive(Clone)]
        struct AllowAllService<S> {
            inner: S,
        }

        impl<S> Layer<S> for AllowAll {
            type Service = AllowAllService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                AllowAllService { inner }
            }
        }

        impl<S> Service<Request> for AllowAllService<S>
        where
            S: Service<Request, Response = Response, Error = Infallible>,
        {
            type Response = Response;
            type Error = Infallible;
            type Future = S::Future;

            fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                self.inner.poll_ready(cx)
            }
            fn call(&mut self, req: Request) -> Self::Future {
                self.inner.call(req)
            }
        }

        #[tokio::test]
        async fn passes_the_limited_routes_through() {
            // With a passthrough limiter, the limited routes must NOT be 429 — proving
            // the layer gates rather than hard-blocks. (Each handler answers its own
            // request.)
            let app = build_router(test_state(), NO_PUBLIC_DIR.as_ref(), rate_limited_router_with(AllowAll));
            for uri in ["/dpe/oai", "/dpe/projects/0862/metadata.jsonld"] {
                assert_ne!(status_of(app.clone(), uri).await, StatusCode::TOO_MANY_REQUESTS, "{uri}");
            }
        }
    }

    #[tokio::test]
    async fn record_file_route_is_registered_and_does_not_shadow_the_project_route() {
        use axum::routing::get;
        use http_body_util::BodyExt;

        use crate::router::build_router;

        dpe_core::set_data_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));
        // Covers live under the public dir, which cargo does NOT put on the test cwd:
        // `cargo test` runs from the package dir, so the relative default misses and
        // every project would render as coverless. Distinct from NO_PUBLIC_DIR below,
        // which is the ServeDir static-file root, not dpe-core's asset lookup.
        dpe_core::set_public_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../public"));

        let app = build_router(
            test_state(),
            NO_PUBLIC_DIR.as_ref(),
            axum::Router::new().route("/dpe/oai", get(dpe_api_oai::oai_handler)),
        );

        let body_of = |uri: &str| {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let app = app.clone();
            async move {
                let response = app.oneshot(req).await.unwrap();
                let status = response.status();
                let bytes = response.into_body().collect().await.unwrap().to_bytes();
                (status, String::from_utf8_lossy(&bytes).into_owned())
            }
        };

        // Any record with a file, taken from the cache: the populated-metadata
        // record is a shared-metadata test fixture and is not part of the
        // served data.
        let record = dpe_core::record_cache::all_records()
            .iter()
            .find(|r| r.file.is_some())
            .expect("the committed data has records with files");
        let (status, _) =
            body_of(&format!("/dpe/records/{}/{}/file", record.pid.shortcode, record.pid.record_id)).await;
        assert_eq!(status, StatusCode::OK, "the record file route should resolve");

        let (status, body) = body_of("/dpe/projects/0862").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!body.contains("Page not found."), "the project page route should still render");
        // 0862 has a cover on disk, so the wired-up handler must resolve it. This is the
        // only test that renders a full project page through the router, so without the
        // assertion the whole set_public_dir → cover_image_url → <img> path is exercised
        // nowhere in `cargo test` and a broken lookup would look like a passing suite.
        assert!(
            body.contains(r#"src="/assets/images/0862.webp""#),
            "0862's cover should resolve: {body}"
        );
    }

    #[tokio::test]
    async fn the_real_rate_limited_router_builds_and_wires() {
        // The production sub-router must build from default config (guards the
        // .expect()) and attach without a passthrough request tripping the limit.
        use crate::router::{build_router, rate_limited_router};

        let config = crate::config::DpeConfig::default();
        let app = build_router(test_state(), NO_PUBLIC_DIR.as_ref(), rate_limited_router(&config));
        assert_ne!(status_of(app, "/dpe/oai").await, StatusCode::TOO_MANY_REQUESTS);
    }

    /// The rate-limit key extractor: keys on the rightmost (Traefik-appended)
    /// `X-Forwarded-For` entry, never the client-forgeable leftmost one.
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
