//! App router assembly, kept separate from `serve()` (which does I/O + global
//! setup) so the routing — including the per-route OAI rate limiter — is
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

/// The `/dpe/oai` route as a standalone sub-router with `limiter` applied to it.
/// The limiter type is erased here — the result is a plain `Router<AppState>` the
/// caller merges in — so `build_router` never has to name the `GovernorLayer`
/// type. In production `limiter` is the real `GovernorLayer` (see [`oai_router`]);
/// tests pass a fake to drive gating deterministically.
pub(crate) fn oai_router_with<L>(limiter: L) -> Router<AppState>
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
        // No literal-colon paths here, but the flag must match the routers this is
        // merged with: on merge Axum keeps the checks *unless both* sides disabled
        // them, and the Dataverse sub-router needs them off (see
        // `dataverse_router_with`). Leaving them on here would re-enable them for
        // the merged router and panic on the Dataverse paths.
        .without_v07_checks()
        .route("/dpe/oai", get(dpe_api_oai::oai_handler))
        .route_layer(limiter)
}

/// The production `/dpe/oai` sub-router, rate-limited per-IP from config.
/// `use_headers()` adds `X-RateLimit-*` and `Retry-After` to 429 responses.
pub(crate) fn oai_router(config: &DpeConfig) -> Router<AppState> {
    use tower_governor::governor::GovernorConfigBuilder;
    use tower_governor::GovernorLayer;

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.oai_rate_limit_per_second)
        .burst_size(config.oai_rate_limit_burst)
        .key_extractor(RightmostXffKeyExtractor)
        .use_headers()
        .finish()
        .expect("OAI GovernorConfig should build from valid config");

    oai_router_with(GovernorLayer { config: std::sync::Arc::new(governor_conf) })
}

/// The Dataverse-compat routes as a standalone sub-router with `limiter` applied.
/// Same type-erasure trick as [`oai_router_with`], for the same reason.
///
/// Both routes live at the host root rather than under `/dpe`, unlike the rest of
/// the app. Their shapes are fixed by the Dataverse contract: the versions
/// endpoint is the URL EOSC Data Commons seeds on their side, and the harvester
/// builds `{scheme}://{host}/api/access/datafile/{id}` itself from the
/// `dataFile.id` we emit, so neither can be nested under a prefix.
pub(crate) fn dataverse_router_with<L>(limiter: L) -> Router<AppState>
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
        // `:persistentId` and `:latest-published` are LITERAL segments of the
        // Dataverse URL, not path parameters: the dataset is selected by the
        // `persistentId` *query* parameter. Do not "fix" the colons.
        //
        // Axum 0.8 spells parameters `{name}` and panics on any segment starting
        // with `:` to catch un-migrated 0.7-style routes — so registering these
        // literals requires opting out of that check, which is what
        // `without_v07_checks` is for. It is scoped to this sub-router, so the rest
        // of the app keeps the guard.
        .without_v07_checks()
        .route(
            "/api/datasets/:persistentId/versions/:latest-published",
            get(dpe_api_dataverse::versions_handler),
        )
        .route("/api/access/datafile/{id}", get(dpe_api_dataverse::datafile_handler))
        .route_layer(limiter)
}

/// The production Dataverse-compat sub-router, rate-limited per-IP from config.
pub(crate) fn dataverse_router(config: &DpeConfig) -> Router<AppState> {
    use tower_governor::governor::GovernorConfigBuilder;
    use tower_governor::GovernorLayer;

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.dataverse_rate_limit_per_second)
        .burst_size(config.dataverse_rate_limit_burst)
        .key_extractor(RightmostXffKeyExtractor)
        .use_headers()
        .finish()
        .expect("Dataverse GovernorConfig should build from valid config");

    dataverse_router_with(GovernorLayer { config: std::sync::Arc::new(governor_conf) })
}

/// Assemble the traced app router. `oai_router` and `dataverse_router` carry the
/// (already rate-limited) `/dpe/oai` and Dataverse-compat routes; passing them in
/// — rather than bare layers — keeps this signature free of the `GovernorLayer`
/// trait bounds, so it stays reconstructable by hand and lets tests substitute
/// fake-limited sub-routers. Static assets are served from `public_dir`, falling
/// back to the app's 404 shell.
pub(crate) fn build_router(
    state: AppState,
    public_dir: &std::path::Path,
    oai_router: Router<AppState>,
    dataverse_router: Router<AppState>,
) -> Router {
    use axum::response::Redirect;
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
    use tower_http::services::ServeDir;

    // Static assets + 404: serve files from the public dir, falling back to the
    // "Page not found." shell.
    let serve_dir = ServeDir::new(public_dir).not_found_service(get(crate::not_found).with_state(state.clone()));

    Router::new()
        // Required to merge `dataverse_router`, whose paths contain literal `:`
        // segments mandated by the Dataverse contract (see `dataverse_router_with`).
        // On merge Axum re-validates the incoming paths and keeps the 0.7-param
        // check unless *both* routers disabled it, so every router in this merge
        // chain must opt out — here, plus `oai_router_with` and
        // `dataverse_router_with`. Every route declared in this function uses the
        // 0.8 `{param}` form, so nothing below relies on the guard.
        .without_v07_checks()
        // --- Traced routes (declared BEFORE .layer()) ---
        // Page routes.
        .route("/", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe", get(|| async { Redirect::permanent("/dpe/projects") }))
        .route("/dpe/projects", get(projects_page_handler))
        .route("/dpe/about", get(about_page_handler))
        .route("/dpe/projects/{id}", get(project_page_handler))
        // OAI-PMH (note: /dpe/oai, not /oai) — XML, must stay unbroken.
        // Rate-limited per-IP; the limiter is scoped to this route only (see `oai_router`).
        .merge(oai_router)
        // Dataverse-compat API for EOSC Data Commons harvesting — JSON, at the host
        // root because the contract fixes the URL shapes (see `dataverse_router_with`).
        .merge(dataverse_router)
        // Datastar SSE + JSON endpoints.
        .route("/dpe/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
        .route("/dpe/projects/search", get(fragments::search_fragment_handler))
        .route("/dpe/api/v2/projects", get(fragments::projects_json_handler))
        .route("/dpe/api/v2/projects/{id}", get(fragments::project_json_handler))
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

/// Tests for the rate-limit *seam*: that the limiter is wired onto `/dpe/oai`
/// only. The actual throttling algorithm is `tower_governor`'s and is not
/// re-tested here. Fake layers (`AllowAll`/`DenyAll`) stand in for the real
/// `GovernorLayer` so gating is deterministic and independent of timing.
///
/// Each fake is defined inside the submodule of the single test that uses it,
/// so the source itself shows the fake cannot leak to another test. The shared
/// harness (`test_state`, `status_of`, `NO_PUBLIC_DIR`) lives here at the top.
#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    use crate::AppState;

    fn test_state() -> AppState {
        AppState {
            fathom_site_id: None,
            css_href: "/assets/app.css".to_string(),
        }
    }

    // Static assets come from a nonexistent dir: these tests target redirect and
    // OAI routes, never a real static file, so the fallback is never exercised.
    const NO_PUBLIC_DIR: &str = "nonexistent-test-dir";

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
        use crate::router::{build_router, dataverse_router_with, oai_router_with};

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
        async fn gates_oai_only() {
            // The `/dpe` redirect is a pure handler (no data/cache access), so it is a
            // stable "not rate-limited" control that avoids the set_data_dir global.
            // The Dataverse sub-router gets its own passthrough limiter so this test
            // observes the OAI limiter in isolation.
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(DenyAll),
                dataverse_router_with(tower::layer::util::Identity::new()),
            );
            assert_eq!(status_of(app.clone(), "/dpe/oai").await, StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(status_of(app, "/dpe").await, StatusCode::PERMANENT_REDIRECT);
        }
    }

    /// `AllowAll` — a fake limiter that lets every request through unchanged.
    /// Scoped to `allow_all::passes_oai_through`.
    mod allow_all {
        use std::convert::Infallible;
        use std::task::{Context, Poll};

        use axum::extract::Request;
        use axum::http::StatusCode;
        use axum::response::Response;
        use tower::{Layer, Service};

        use super::{status_of, test_state, NO_PUBLIC_DIR};
        use crate::router::{build_router, dataverse_router_with, oai_router_with};

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
        async fn passes_oai_through() {
            // With a passthrough limiter, /dpe/oai must NOT be 429 — proving the layer
            // gates rather than hard-blocks. (The handler answers the OAI request itself.)
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(AllowAll),
                dataverse_router_with(AllowAll),
            );
            assert_ne!(status_of(app, "/dpe/oai").await, StatusCode::TOO_MANY_REQUESTS);
        }
    }

    #[tokio::test]
    async fn real_oai_router_builds_and_wires() {
        // The production oai_router must build from default config (guards the
        // .expect()) and attach without a passthrough request tripping the limit.
        use crate::router::{build_router, dataverse_router, oai_router};

        let config = crate::config::DpeConfig::default();
        let app = build_router(
            test_state(),
            NO_PUBLIC_DIR.as_ref(),
            oai_router(&config),
            dataverse_router(&config),
        );
        assert_ne!(status_of(app, "/dpe/oai").await, StatusCode::TOO_MANY_REQUESTS);
    }

    /// The Dataverse-compat routes: that they are reachable at the exact URLs the
    /// contract fixes, and that the rate limiter is scoped to them.
    ///
    /// These go through the real handlers, which read the process-wide file table.
    /// That table is loaded from the data dir set by `set_data_dir` — a global these
    /// tests deliberately do not touch — so the assertions are about routing and
    /// gating (which status is produced by which layer), never about fixture
    /// contents. The contract itself is tested in `dpe-api-dataverse`.
    mod dataverse {
        use std::convert::Infallible;
        use std::task::{Context, Poll};

        use axum::body::Body;
        use axum::extract::Request;
        use axum::http::StatusCode;
        use axum::response::Response;
        use tower::{Layer, Service};

        use super::{status_of, test_state, NO_PUBLIC_DIR};
        use crate::router::{build_router, dataverse_router, dataverse_router_with, oai_router_with};

        /// Rejects every request with 429 without calling inner.
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

        const VERSIONS_URI: &str =
            "/api/datasets/:persistentId/versions/:latest-published?persistentId=oai:dasch.swiss:ark:/72163/1/0803";
        const DATAFILE_URI: &str = "/api/access/datafile/1001";

        #[tokio::test]
        async fn routes_are_registered_at_the_contract_urls() {
            // The literal-colon path and the root-level download path must both
            // resolve to a handler rather than falling through to the static-file
            // fallback. Which answer each handler gives depends on the ambient data
            // dir — a process-global these tests deliberately leave alone, and which
            // another test in the same binary may already have set — so the
            // assertion is only that a *handler* answered: the Dataverse handlers
            // reply with JSON or a redirect, while the fallback replies with the
            // HTML 404 shell.
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(tower::layer::util::Identity::new()),
                dataverse_router_with(tower::layer::util::Identity::new()),
            );

            for uri in [VERSIONS_URI, DATAFILE_URI] {
                let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
                let resp = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
                let status = resp.status();
                let content_type = resp
                    .headers()
                    .get(axum::http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();

                assert!(
                    !content_type.starts_with("text/html"),
                    "{uri} fell through to the HTML 404 shell — the route did not match"
                );
                assert!(
                    content_type.starts_with("application/json") || status.is_redirection(),
                    "{uri} should be answered by a Dataverse handler, got {status} with content-type {content_type:?}"
                );
            }
        }

        #[tokio::test]
        async fn non_numeric_file_id_does_not_match_the_route() {
            // `Path<u64>` rejects a non-numeric segment, so the request falls through
            // to the app's 404 shell rather than reaching the handler.
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(tower::layer::util::Identity::new()),
                dataverse_router_with(tower::layer::util::Identity::new()),
            );

            assert_eq!(
                status_of(app, "/api/access/datafile/not-a-number").await,
                StatusCode::BAD_REQUEST,
                "a non-numeric id should be rejected by the extractor"
            );
        }

        #[tokio::test]
        async fn limiter_gates_dataverse_routes_only() {
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(tower::layer::util::Identity::new()),
                dataverse_router_with(DenyAll),
            );

            assert_eq!(status_of(app.clone(), DATAFILE_URI).await, StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(status_of(app.clone(), VERSIONS_URI).await, StatusCode::TOO_MANY_REQUESTS);
            // Control: a route outside the Dataverse sub-router is untouched.
            assert_eq!(status_of(app, "/dpe").await, StatusCode::PERMANENT_REDIRECT);
        }

        #[tokio::test]
        async fn real_dataverse_router_builds_and_wires() {
            // Guards the .expect() in `dataverse_router`: the production limiter must
            // build from default config, and a single request must not trip it.
            let config = crate::config::DpeConfig::default();
            let app = build_router(
                test_state(),
                NO_PUBLIC_DIR.as_ref(),
                oai_router_with(tower::layer::util::Identity::new()),
                dataverse_router(&config),
            );

            assert_ne!(status_of(app, DATAFILE_URI).await, StatusCode::TOO_MANY_REQUESTS);
        }
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
