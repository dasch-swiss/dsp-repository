//! `serve()` and `shutdown_signal`.

use std::process::ExitCode;

use axum::Router;
use cpe_server::Cpe;
use dpe_server::Dpe;

/// The directory of `sync`'s data when `SYNC_DATA_DIR` is not set: the
/// committed one, relative to the repository root, where a local run starts.
const DEFAULT_SYNC_DATA_DIR: &str = "areas/access/sync/data";

/// The directory of `media`'s data when `MEDIA_DATA_DIR` is not set: the
/// committed one, relative to the repository root, where a local run starts.
const DEFAULT_MEDIA_DATA_DIR: &str = "areas/access/media/data";

/// The directory a data variable names, `default` when it is unset or empty: a deployment that
/// passes an empty value (a compose file's `${VAR}` of an unset variable) means "not set".
fn data_dir(value: Option<String>, default: &str) -> String {
    value.filter(|value| !value.is_empty()).unwrap_or_else(|| default.to_owned())
}

/// The app the composition root serves over DPE's router, and whether CPE is
/// in it.
///
/// CPE is left out, and DPE served alone, without a base (`CPE_BASE` unset, or
/// refused by CPE): `nest` panics on an empty path, and CPE at the root would
/// answer DPE's paths. It is left out too when the app cannot be built with
/// CPE at its base: axum panics on two handlers for one path and method, and
/// no value of `CPE_BASE` may stop the binary. The whole build with CPE is
/// inside `catch_unwind`, as a route declared after the mount can be the one
/// that collides; a panic of the build without CPE is the binary's own.
fn app(dpe: Router, cpe: &Cpe) -> (Router, bool) {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    if cpe.base().is_empty() {
        return (assemble(dpe, None), false);
    }
    match catch_unwind(AssertUnwindSafe(|| assemble(dpe.clone(), Some(cpe)))) {
        Ok(app) => (app, true),
        Err(panic) => {
            let reason = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("the router's build panicked");
            tracing::error!(
                cpe_base = cpe.base(),
                reason,
                "the router's build with CPE at CPE_BASE panicked: CPE is not mounted, and DPE is served alone"
            );
            (assemble(dpe, None), false)
        }
    }
}

/// Assembles the app: DPE's router and, when given, CPE's at its base, then
/// the OTel layers, then the untraced routes. Panics where [`mount_cpe`] does.
///
/// Kept separate from `serve()` (which does I/O and process-global setup) so
/// the traced/untraced route order — routes declared after `.layer()` are not
/// wrapped by it — is unit-testable.
fn assemble(dpe: Router, cpe: Option<&Cpe>) -> Router {
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

    let capabilities = Router::new().merge(dpe);
    let capabilities = match cpe {
        Some(cpe) => mount_cpe(capabilities, cpe),
        None => capabilities,
    };
    capabilities
        // --- OTel layers ---
        // Axum layers wrap in reverse declaration order:
        // - OtelInResponseLayer (declared first) runs INNER — injects traceparent into response headers
        // - OtelAxumLayer (declared second) runs OUTER — creates the server span from the request
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        // --- Untraced routes ---
        // Routes declared AFTER .layer() calls are NOT wrapped by those layers.
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route(
            "/telemetry/collect",
            // "dpe" names the OTel instrumentation scope (`dpe.browser`) the
            // dashboards filter on: DPE's telemetry identity, not this crate's
            // name. Do not change it.
            shared_telemetry::collector::collect_route("dpe", dpe_server::normalize_page_url).layer({
                use dpe_server::RightmostXffKeyExtractor;
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::GovernorLayer;

                let governor_conf = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(RightmostXffKeyExtractor)
                    .finish()
                    .expect("GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(governor_conf) }
            }),
        )
}

/// `router` with CPE's routes under [`Cpe::base`], the one source of that
/// path: CPE writes it in front of every link it renders.
///
/// Nested, never merged: CPE's router ends in a fallback of its own, as DPE's
/// does, and two routers with a fallback each cannot be merged. A fallback
/// is no route, so nesting it collides with none, and a route `router` has
/// under the base still answers.
///
/// Panics on an empty base, and when `router` has a route for `<base>/` that
/// answers every method, as the one declared here does.
fn mount_cpe(router: Router, cpe: &Cpe) -> Router {
    use axum::extract::{MatchedPath, Request};
    use axum::http::Uri;

    let base = cpe.base();
    let cpe_root = cpe.router();
    let traced = cpe_root.clone().layer(axum::middleware::from_fn(name_span_by_cpe_route));
    router.nest(base, traced).route(
        // `nest` matches the base and what follows `<base>/`, and not
        // `<base>/` itself, which would fall through to DPE.
        &format!("{base}/"),
        axum::routing::any(move |mut request: Request| {
            let cpe_root = cpe_root.clone();
            async move {
                // CPE's router takes the path below the base, as from `nest`:
                // here its root, with the request's query. A query that a
                // URI was parsed with parses again, so `None` is no query.
                let below = request.uri().query().and_then(|query| format!("/?{query}").parse().ok());
                *request.uri_mut() = below.unwrap_or(Uri::from_static("/"));
                // This route's own match, which CPE would report as a route of its own.
                request.extensions_mut().remove::<MatchedPath>();
                // A router's error type is `Infallible`.
                let Ok(response) = tower::ServiceExt::oneshot(cpe_root, request).await;
                response
            }
        }),
    )
}

/// Names the request span by the route of CPE that answered. To the OTel
/// layer CPE is one service, so it names every CPE request by its method alone.
///
/// The name is set on the OpenTelemetry span: `tracing-opentelemetry` drops a
/// recorded `otel.name` once the span has been entered. `http.route` stays as
/// the layer set it, empty: a second record adds a second attribute of that
/// key.
async fn name_span_by_cpe_route(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let method = request.method().clone();
    let response = next.run(request).await;
    if let Some(route) = response.extensions().get::<cpe_server::MatchedRoute>() {
        tracing::Span::current()
            .context()
            .span()
            .update_name(format!("{method} {}", route.as_str()));
    }
    response
}

/// Starts CPE's activation on the blocking pool and returns at once: reading
/// the ports takes seconds, and CPE answers 503 until it is done.
fn activate_in_background(
    cpe: &Cpe,
    projection: impl cpe_ports::ArchiveProjection + 'static,
    files: impl cpe_ports::DepositedFiles + 'static,
) -> tokio::task::JoinHandle<()> {
    let cpe = cpe.clone();
    tokio::task::spawn_blocking(move || cpe.activate(&projection, &files))
}

#[tokio::main]
pub(crate) async fn serve() -> ExitCode {
    // Before OTel init, so a panic during init is captured.
    crate::observability::install_tracing_panic_hook();

    let (logger_provider, _otel_guard) = crate::observability::init_otel();

    let _pyroscope_agent = crate::observability::init_pyroscope();

    let dpe_config = dpe_server::DpeConfig::load().expect("failed to load DPE configuration");
    tracing::info!(data_dir = %dpe_config.data_dir.display(), "DPE configuration loaded");

    if let Some(ref site_id) = dpe_config.fathom_site_id {
        tracing::info!(fathom_site_id = %site_id, "Fathom Analytics enabled");
    }

    if let Some(ref url) = dpe_config.ark_resolver_base_url {
        tracing::info!(
            ark_resolver_base_url = %url,
            "this deployment publishes ARKs that resolve to itself, and serves the /ark:/ resolver"
        );
    }

    if dpe_config.show_placeholder_values {
        tracing::info!("Placeholder values (MISSING/CALCULATED) will be shown in the UI");
    }

    let dpe = Dpe::new(&dpe_config);
    tokio::task::spawn_blocking({
        let dpe = dpe.clone();
        move || dpe.warm()
    });

    // Neither call can fail: a CPE that cannot be served must not stop DPE.
    let cpe_config = cpe_server::CpeConfig::load();
    let cpe = Cpe::new(&cpe_config);

    let addr: std::net::SocketAddr = std::env::var("DPE_SITE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid site address (DPE_SITE_ADDR)");
    let sync_data_dir = data_dir(std::env::var("SYNC_DATA_DIR").ok(), DEFAULT_SYNC_DATA_DIR);
    let media_data_dir = data_dir(std::env::var("MEDIA_DATA_DIR").ok(), DEFAULT_MEDIA_DATA_DIR);

    let (app, cpe_is_mounted) = app(dpe.router(), &cpe);

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));

    // After the bind: the port takes connections before CPE reads anything.
    if !cpe_is_mounted {
        // A base that CPE refused is logged by CPE, and one that cannot be
        // mounted by `app`, each with the reason.
        if cpe_config.base.is_empty() {
            tracing::warn!("CPE_BASE is not set: CPE is not mounted, and DPE is served alone");
        }
    } else {
        if !std::path::Path::new(&sync_data_dir).is_dir() {
            tracing::warn!(
                sync_data_dir = sync_data_dir.as_str(),
                "SYNC_DATA_DIR names no directory: every project of CPE's will fail to load and answer 503"
            );
        }
        if !std::path::Path::new(&media_data_dir).is_dir() {
            tracing::warn!(
                media_data_dir = media_data_dir.as_str(),
                "MEDIA_DATA_DIR names no directory: every project of CPE's will fail to load and answer 503"
            );
        }
        tracing::info!(
            cpe_base = cpe.base(),
            projects_dir = %cpe_config.projects_dir.display(),
            sync_data_dir = sync_data_dir.as_str(),
            media_data_dir = media_data_dir.as_str(),
            "CPE is mounted and activates in the background"
        );
        activate_in_background(
            &cpe,
            sync_store::LiveArchiveProjection::new(sync_data_dir),
            media_store::LiveDepositedFiles::new(media_data_dir),
        );
    }

    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server exited with error");

    // Blocking I/O (thread joins, uploads, OTLP flush): run via spawn_blocking so the
    // Tokio runtime does not deadlock.
    tokio::task::spawn_blocking(move || {
        if let Some(agent) = _pyroscope_agent {
            if let Ok(ready) = agent.stop() {
                ready.shutdown();
            }
        }
        // Flush OTel logs before dropping the trace/metrics guard — log records
        // may reference trace context that becomes invalid after guard drop.
        if let Some(provider) = logger_provider {
            let _ = provider.force_flush();
            let _ = provider.shutdown();
        }
        drop(_otel_guard);
    })
    .await
    .ok();

    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, flushing telemetry");
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{mpsc, Mutex, OnceLock, PoisonError};
    use std::time::Duration;

    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{header, HeaderMap, StatusCode};
    use cpe_ports::{
        ArchiveProjection, ClassIri, DataArk, FakeArchiveProjection, FakeDepositedFiles, ProjectSnapshot,
        ProjectionError, PropertyIri, Resource, ResourceIri, Value, ValueKind, DATA_ARK_PREFIX,
    };
    use cpe_server::{Cpe, CpeConfig};
    use media_store::LiveDepositedFiles;
    use sync_store::LiveArchiveProjection;
    use tower::ServiceExt;

    use super::{activate_in_background, data_dir, DEFAULT_MEDIA_DATA_DIR, DEFAULT_SYNC_DATA_DIR};

    const BASE: &str = "/cpe";

    /// What CPE's own not-found page says, and no page of DPE.
    const CPE_NOT_FOUND: &str = "This address does not name a project.";

    /// A `DpeConfig` pointing at the committed data and public dirs, resolved
    /// relative to this crate's manifest dir so the test does not depend on
    /// the process's working directory.
    fn test_dpe_config() -> dpe_server::DpeConfig {
        dpe_server::DpeConfig {
            data_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../dpe/server/data").into(),
            public_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../dpe/public").into(),
            ..dpe_server::DpeConfig::default()
        }
    }

    fn dpe() -> dpe_server::Dpe {
        dpe_server::Dpe::new(&test_dpe_config())
    }

    fn app(dpe: &dpe_server::Dpe, cpe: &Cpe) -> axum::Router {
        super::app(dpe.router(), cpe).0
    }

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
    }

    fn cpe_config(projects_dir: &Path, base: &str) -> CpeConfig {
        let mut config = CpeConfig::default();
        config.projects_dir = projects_dir.to_owned();
        config.base = base.to_owned();
        config
    }

    /// A project built from archive project `0001`.
    const THINGS: &str = r#"
site {
    title "Things Fixture"
    languages "en" "de"
}
page id="home" route="/"
page id="thing" route="/things/:id" {
    resource-detail id="$id" title-property="name" {
        show-properties "name"
    }
}
remodel {
    shortcode "0001"
    class "Thing" iri="http://www.knora.org/ontology/0001/things#Thing" {
        id from="iri"
        property "name" iri="http://www.knora.org/ontology/0001/things#hasName" type="text"
    }
}
"#;

    fn things_snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            shortcode: "0001".to_owned(),
            resources: vec![Resource {
                iri: ResourceIri("http://rdfh.ch/0001/lamp".to_owned()),
                ark: DataArk(format!("{DATA_ARK_PREFIX}0001/lamp")),
                class: ClassIri("http://www.knora.org/ontology/0001/things#Thing".to_owned()),
                label: "Brass lamp".to_owned(),
                values: vec![Value {
                    property: PropertyIri("http://www.knora.org/ontology/0001/things#hasName".to_owned()),
                    uuid: Some("v1".to_owned()),
                    kind: ValueKind::Text { text: "Brass lamp".to_owned(), lang: None },
                }],
                file: None,
                part_of: Vec::new(),
                seqnum: None,
                annotation: None,
            }],
            list_nodes: Vec::new(),
        }
    }

    /// A projects folder with one Things project per `(id, shortcode)`, each a folder holding
    /// its `SHORTCODE`.
    fn projects_folder(projects: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("a temporary directory");
        for (id, shortcode) in projects {
            let dir = root.path().join(id);
            std::fs::create_dir(&dir).expect("the project folder is created");
            std::fs::write(dir.join("SHORTCODE"), shortcode).expect("SHORTCODE is written");
        }
        root
    }

    /// The deposited files of the Things projects `0001` and `0002`: each one's config.
    fn things_files() -> FakeDepositedFiles {
        ["0001", "0002"]
            .into_iter()
            .fold(FakeDepositedFiles::default(), |files, shortcode| {
                let config = THINGS.replace("shortcode \"0001\"", &format!("shortcode \"{shortcode}\""));
                files.with_file(shortcode, "config.kdl", config)
            })
    }

    /// CPE at `base` with `things` served and `offline` failed. The folder
    /// is returned to keep it for the test's length.
    fn things_and_offline(base: &str) -> (tempfile::TempDir, Cpe) {
        let root = projects_folder(&[("things", "0001"), ("offline", "0002")]);
        let cpe = Cpe::new(&cpe_config(root.path(), base));
        cpe.activate(
            &FakeArchiveProjection::default()
                .with_project(things_snapshot())
                .with_unavailable("0002"),
            &things_files(),
        );
        (root, cpe)
    }

    /// Incunabula from the committed folder and data, activated once.
    fn incunabula() -> &'static Cpe {
        static CPE: OnceLock<Cpe> = OnceLock::new();
        CPE.get_or_init(|| {
            let root = repository_root();
            let cpe = Cpe::new(&cpe_config(&root.join("areas/access/cpe/projects"), BASE));
            cpe.activate(
                &LiveArchiveProjection::new(root.join(DEFAULT_SYNC_DATA_DIR)),
                &LiveDepositedFiles::new(root.join(DEFAULT_MEDIA_DATA_DIR)),
            );
            cpe
        })
    }

    /// A request carrying `ConnectInfo`, as every real request does under
    /// `into_make_service_with_connect_info`: the OAI rate limiter's key
    /// extractor falls back to it when there is no `X-Forwarded-For` header,
    /// and errors the request (not merely declining to limit it) without one.
    fn request_with_peer(uri: &str) -> Request<Body> {
        use axum::extract::ConnectInfo;

        let mut req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 12345))));
        req
    }

    async fn status_of(app: axum::Router, uri: &str) -> StatusCode {
        app.oneshot(request_with_peer(uri)).await.unwrap().status()
    }

    async fn get(app: &axum::Router, uri: &str) -> (StatusCode, HeaderMap, String) {
        let response = app.clone().oneshot(request_with_peer(uri)).await.unwrap();
        let (parts, body) = response.into_parts();
        let body = axum::body::to_bytes(body, usize::MAX).await.expect("the body is finite");
        (parts.status, parts.headers, String::from_utf8_lossy(&body).into_owned())
    }

    fn assert_unavailable(status: StatusCode, headers: &HeaderMap, what: &str) {
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{what}");
        assert!(headers.contains_key(header::RETRY_AFTER), "{what}: Retry-After");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{what}");
    }

    async fn traceparent_header_of(app: axum::Router, uri: &str) -> Option<String> {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();
        response.headers().get("traceparent").map(|v| v.to_str().unwrap().to_string())
    }

    /// The name and the request path of every span the test process has ended.
    static ENDED_SPANS: Mutex<Vec<EndedSpan>> = Mutex::new(Vec::new());

    #[derive(Clone, Debug)]
    struct EndedSpan {
        name: String,
        path: Option<String>,
    }

    #[derive(Debug)]
    struct KeepEndedSpans;

    impl opentelemetry_sdk::trace::SpanExporter for KeepEndedSpans {
        async fn export(
            &self,
            batch: Vec<opentelemetry_sdk::trace::SpanData>,
        ) -> opentelemetry_sdk::error::OTelSdkResult {
            let attribute = |span: &opentelemetry_sdk::trace::SpanData, key: &str| {
                span.attributes
                    .iter()
                    .find(|attribute| attribute.key.as_str() == key)
                    .map(|attribute| attribute.value.as_str().into_owned())
            };
            let mut ended = ENDED_SPANS.lock().unwrap_or_else(PoisonError::into_inner);
            for span in &batch {
                ended.push(EndedSpan {
                    name: span.name.to_string(),
                    path: attribute(span, "url.path"),
                });
            }
            Ok(())
        }
    }

    /// The span of the one request for `path`, which no other test may ask
    /// for: the spans of all tests are kept together.
    fn span_of(path: &str) -> EndedSpan {
        let found: Vec<EndedSpan> = ENDED_SPANS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|span| span.path.as_deref() == Some(path))
            .cloned()
            .collect();
        assert_eq!(found.len(), 1, "one request span for {path}: {found:?}");
        found[0].clone()
    }

    /// `OtelInResponseLayer` only writes `traceparent` when a `tracing-opentelemetry` layer is
    /// on the global subscriber and a text-map propagator is set, both of which `serve()` does.
    /// This sets up an equivalent once per test process, so pages get a real span id to inject;
    /// it is global for the whole test binary. Its exporter keeps what ends in [`ENDED_SPANS`].
    fn init_test_otel() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            use opentelemetry::trace::TracerProvider as _;
            use tracing_subscriber::layer::SubscriberExt;

            opentelemetry::global::set_text_map_propagator(
                opentelemetry_sdk::propagation::TraceContextPropagator::new(),
            );
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_simple_exporter(KeepEndedSpans)
                .build();
            let tracer = provider.tracer("access-server-test");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            let subscriber = tracing_subscriber::registry().with(otel_layer);
            // Set once for the whole test binary; ignore failure if another test beat us to it.
            let _ = tracing::subscriber::set_global_default(subscriber);
        });
    }

    /// The one thing this crate exists to get right: routes declared after
    /// the OTel layers carry no `traceparent` response header, and the
    /// capabilities' pages — declared before them — do.
    #[tokio::test]
    async fn untraced_routes_carry_no_traceparent_header_and_the_capabilities_pages_do() {
        init_test_otel();
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        assert_eq!(
            traceparent_header_of(router.clone(), "/healthz").await,
            None,
            "/healthz is declared after the OTel layers"
        );
        assert!(
            traceparent_header_of(router.clone(), "/dpe/projects").await.is_some(),
            "a DPE page is declared before the OTel layers"
        );
        for uri in ["/cpe/things/", "/cpe/", "/cpe/nope", "/cpe/offline/"] {
            assert!(
                traceparent_header_of(router.clone(), uri).await.is_some(),
                "{uri}: CPE is mounted before the OTel layers"
            );
        }
    }

    #[tokio::test]
    async fn untraced_post_to_telemetry_collect_carries_no_traceparent_header() {
        init_test_otel();
        let router = app(&dpe(), &Cpe::new(&CpeConfig::default()));
        let req = Request::builder()
            .method("POST")
            .uri("/telemetry/collect")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let response = router.oneshot(req).await.unwrap();
        assert_eq!(
            response.headers().get("traceparent"),
            None,
            "/telemetry/collect is declared after the OTel layers"
        );
    }

    /// A path that matches no route must not become a span name: that set
    /// has no bound.
    #[tokio::test]
    async fn a_cpe_requests_span_is_named_by_its_route_and_never_by_an_unmatched_path() {
        init_test_otel();
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        // A record that is not there: the route matches, whatever it answers.
        let record = "/cpe/things/things/no-such-thing-e5f6";
        assert_eq!(status_of(router.clone(), record).await, StatusCode::NOT_FOUND);
        assert_eq!(span_of(record).name, "GET /cpe/things/things/{id}");

        for path in ["/cpe/no-such-project/a1b2", "/cpe/things/no-such-page-c3d4"] {
            assert_eq!(status_of(router.clone(), path).await, StatusCode::NOT_FOUND);
            assert_eq!(span_of(path).name, "GET", "{path}");
        }

        // The overview, with and without the slash, under a base no other test asks for.
        let folder = projects_folder(&[("things", "0001")]);
        let cpe = Cpe::new(&cpe_config(folder.path(), "/overview-g7h8"));
        cpe.activate(
            &FakeArchiveProjection::default().with_project(things_snapshot()),
            &things_files(),
        );
        let router = app(&dpe(), &cpe);
        for path in ["/overview-g7h8", "/overview-g7h8/"] {
            assert_eq!(status_of(router.clone(), path).await, StatusCode::OK, "{path}");
            assert_eq!(span_of(path).name, "GET /overview-g7h8/", "{path}");
        }
    }

    #[tokio::test]
    async fn the_assembled_router_serves_dpe_and_a_static_asset() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        for uri in [
            "/dpe/projects",
            "/dpe/projects/0803",
            "/dpe/oai?verb=Identify",
            "/favicon.ico",
        ] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
    }

    #[tokio::test]
    async fn a_project_is_served_under_the_base_with_and_without_the_trailing_slash() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        let (status, headers, with_slash) = get(&router, "/cpe/things/").await;
        assert_eq!(status, StatusCode::OK, "{with_slash}");
        assert!(with_slash.contains("Things Fixture"), "{with_slash}");
        assert!(
            with_slash.contains("href=\"/cpe/things/_static/cpe/tokens.css\""),
            "the page loads its stylesheet from under the base: {with_slash}"
        );
        assert!(!headers.contains_key(header::LOCATION));

        let (status, headers, without_slash) = get(&router, "/cpe/things").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(header::LOCATION), "no redirect to the slash");
        assert!(without_slash.contains("<title>Things Fixture</title>"), "{without_slash}");

        let (status, _, body) = get(&router, "/cpe/things/things/lamp").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("Brass lamp"), "{body}");
    }

    #[tokio::test]
    async fn the_shared_stylesheet_is_served_under_the_base_and_under_a_project() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);
        for uri in ["/cpe/_static/cpe/tokens.css", "/cpe/things/_static/cpe/tokens.css"] {
            let (status, headers, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert!(
                headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/css"),
                "{uri}: {headers:?}"
            );
            assert!(!body.is_empty(), "{uri}");
        }
    }

    /// The base is CPE's overview, with and without the slash: `nest` answers the first and the
    /// mount's own route the second.
    #[tokio::test]
    async fn the_base_is_cpes_overview_of_the_projects_it_serves() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        let (status, headers, overview) = get(&router, "/cpe/").await;
        assert_eq!(status, StatusCode::OK, "{overview}");
        assert!(!headers.contains_key(header::LOCATION), "no redirect to or from the slash");
        assert!(
            overview.contains("href=\"/cpe/things/\"") && overview.contains("Things Fixture"),
            "the served project is listed with its link under the base: {overview}"
        );
        assert!(
            !overview.contains("/cpe/offline"),
            "a project that answers 503 is not linked: {overview}"
        );
        assert!(
            overview.contains("href=\"/cpe/_static/cpe/tokens.css\""),
            "the page loads its stylesheet from under the base: {overview}"
        );
        for path in overview
            .split('"')
            .filter(|text| text.starts_with('/') && !text.starts_with("//"))
        {
            assert!(path.starts_with("/cpe/"), "{path} leaves the base");
        }

        for uri in ["/cpe", "/cpe?page=2", "/cpe/?page=2"] {
            let (status, headers, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
            assert_eq!(body, overview, "{uri} is the same page");
        }

        // The appearance toggle of the overview, from either form of the base.
        for uri in ["/cpe?appearance=dark", "/cpe/?appearance=dark"] {
            let (status, headers, _) = get(&router, uri).await;
            assert_eq!(status, StatusCode::SEE_OTHER, "{uri}");
            assert_eq!(headers[header::LOCATION], "/cpe/", "{uri}");
            assert_eq!(headers[header::SET_COOKIE], "appearance=dark; Path=/cpe; SameSite=Lax", "{uri}");
        }
    }

    /// The one exit of CPE's not-found page.
    #[tokio::test]
    async fn the_all_projects_link_of_cpes_not_found_leads_to_the_overview() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        let (status, _, not_found) = get(&router, "/cpe/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(not_found.contains("<a href=\"/cpe/\"><span>All projects</span>"), "{not_found}");
        let (status, _, overview) = get(&router, "/cpe/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(overview.contains("href=\"/cpe/things/\""), "{overview}");
    }

    #[tokio::test]
    async fn a_path_that_names_no_project_is_cpes_not_found() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);
        for uri in ["/cpe/nope", "/cpe/nope/deeper", "/cpe/THINGS/"] {
            let (status, headers, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(body.contains(CPE_NOT_FOUND), "{uri}: {body}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
        }
    }

    /// The base followed by `/` is CPE's root, whatever the projects are
    /// called: the base is removed from the path, not read as a project.
    #[tokio::test]
    async fn the_base_with_a_slash_is_cpes_root_even_beside_a_project_named_like_the_base() {
        let folder = projects_folder(&[("cpe", "0001")]);
        let cpe = Cpe::new(&cpe_config(folder.path(), BASE));
        cpe.activate(
            &FakeArchiveProjection::default().with_project(things_snapshot()),
            &things_files(),
        );
        let router = app(&dpe(), &cpe);

        for uri in ["/cpe", "/cpe/", "/cpe/?page=2"] {
            let (status, _, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}");
            assert!(
                body.contains("href=\"/cpe/cpe/\"") && !body.contains("<title>Things Fixture"),
                "{uri} is the overview, which links to the project: {body}"
            );
        }
        let (status, _, body) = get(&router, "/cpe/cpe/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("<title>Things Fixture"), "{body}");

        for uri in ["/cpe", "/cpe/"] {
            let root = router.clone().oneshot(request_with_peer(uri)).await.unwrap();
            assert_eq!(
                root.extensions()
                    .get::<cpe_server::MatchedRoute>()
                    .map(cpe_server::MatchedRoute::as_str),
                Some("/cpe/"),
                "{uri}: the overview's route, reported once and with the base"
            );
        }
    }

    #[tokio::test]
    async fn a_project_that_failed_to_activate_answers_503_on_its_prefix() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);
        for uri in ["/cpe/offline", "/cpe/offline/", "/cpe/offline/things/lamp"] {
            let (status, headers, _) = get(&router, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        assert_eq!(status_of(router.clone(), "/cpe/things/").await, StatusCode::OK);
        assert_eq!(status_of(router, "/dpe/projects").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn an_unknown_path_under_a_project_is_not_found_and_not_cpes_own_page() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        let (status, _, body) = get(&router, "/cpe/things/no-such-page").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(
            body.contains("Things Fixture") && !body.contains(CPE_NOT_FOUND),
            "the project's not-found page: {body}"
        );

        for uri in [
            "/cpe/things/_static/no-such-file.css",
            "/cpe/things/_assets/no-such-file.png",
            "/cpe/_static/no-such-file.css",
        ] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::NOT_FOUND, "{uri}");
        }
    }

    /// A project's `SHORTCODE` is beside its assets on disk.
    #[tokio::test]
    async fn a_path_that_climbs_out_of_its_folder_is_not_found() {
        let (folder, cpe) = things_and_offline(BASE);
        let marker = "0001";
        let shortcode = std::fs::read_to_string(folder.path().join("things/SHORTCODE")).expect("SHORTCODE is there");
        assert_eq!(shortcode, marker, "the file the paths below climb to is there");
        let router = app(&dpe(), &cpe);
        for uri in [
            "/cpe/%2e%2e/x",
            "/cpe/../x",
            "/cpe/things/_assets/..%2fSHORTCODE",
            "/cpe/things/_assets/../SHORTCODE",
            "/cpe/things/_assets/%2e%2e/SHORTCODE",
            "/cpe/things/_assets/SHORTCODE",
            "/cpe/things/_static/..%2f..%2fSHORTCODE",
        ] {
            let (status, headers, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
            assert!(!body.contains(marker), "{uri} must not serve SHORTCODE: {body}");
        }
    }

    /// With the query keys that make CPE redirect on its pages.
    #[tokio::test]
    async fn a_scheme_relative_looking_path_is_never_redirected_under_the_base_or_outside_it() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        for uri in [
            "/cpe//evil.example/x",
            "/cpe//evil.example/x?appearance=dark&accent=red",
            "/cpe///evil.example/x",
        ] {
            let (status, headers, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(body.contains(CPE_NOT_FOUND), "{uri}: {body}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
            assert!(!headers.contains_key(header::SET_COOKIE), "{uri} must set no cookie");
            assert!(!body.contains("evil.example"), "{uri} must not be echoed: {body}");
        }

        let (status, headers, body) = get(&router, "/cpe/things//evil.example/x?appearance=dark").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!headers.contains_key(header::LOCATION));
        assert!(!body.contains("evil.example"), "{body}");

        for uri in [
            "//evil.example/x",
            "//evil.example/x?appearance=dark&accent=red",
            "//evil.example/cpe/things/",
        ] {
            let (status, headers, body) = get(&router, uri).await;
            assert!(!status.is_redirection(), "{uri}: {status}");
            assert!(!headers.contains_key(header::LOCATION), "{uri} must not redirect");
            assert!(!body.contains(CPE_NOT_FOUND), "{uri} is DPE's to answer");
        }
    }

    /// A retired query key is removed with a redirect to the same page.
    #[tokio::test]
    async fn a_redirect_and_a_cookie_of_cpe_stay_under_the_base() {
        let (_folder, cpe) = things_and_offline(BASE);
        let router = app(&dpe(), &cpe);

        let (status, headers, _) = get(&router, "/cpe/things/?accent=red").await;
        assert!(status.is_redirection(), "{status}");
        assert_eq!(headers[header::LOCATION], "/cpe/things/");

        let (status, headers, _) = get(&router, "/cpe/things/?lang=de").await;
        assert_eq!(status, StatusCode::OK);
        let cookie = headers[header::SET_COOKIE].to_str().unwrap();
        assert!(cookie.starts_with("lang=de"), "{cookie}");
        assert!(
            cookie.split("; ").any(|attribute| attribute == "Path=/cpe"),
            "the cookie is CPE's and is not sent to DPE: {cookie}"
        );
    }

    #[tokio::test]
    async fn a_path_outside_the_base_is_dpes_to_answer_as_without_cpe() {
        let (_folder, cpe) = things_and_offline(BASE);
        let with_cpe = app(&dpe(), &cpe);
        let without_cpe = app(&dpe(), &Cpe::new(&CpeConfig::default()));

        for uri in [
            "/",
            "/nope",
            "/cpex",
            "/cpex/things/",
            "/things/",
            "/_static/cpe/tokens.css",
            "/%2e%2e/x",
            "/things/_assets/..%2fconfig.kdl",
            "/assets/no-such-file.css",
            "/favicon.ico",
            "/robots.txt",
        ] {
            let (status, headers, body) = get(&with_cpe, uri).await;
            let (expected_status, expected_headers, _) = get(&without_cpe, uri).await;
            assert_eq!(status, expected_status, "{uri}");
            assert_eq!(headers.get(header::LOCATION), expected_headers.get(header::LOCATION), "{uri}");
            assert_eq!(
                headers.get(header::CONTENT_TYPE),
                expected_headers.get(header::CONTENT_TYPE),
                "{uri}"
            );
            assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}");
            assert!(
                !body.contains(CPE_NOT_FOUND) && !body.contains("Things Fixture"),
                "{uri} is not CPE's: {body}"
            );
        }
    }

    #[tokio::test]
    async fn until_cpe_is_activated_its_base_answers_503_and_everything_else_is_served() {
        let folder = projects_folder(&[("things", "0001")]);
        let router = app(&dpe(), &Cpe::new(&cpe_config(folder.path(), BASE)));
        for uri in [
            "/cpe",
            "/cpe/",
            "/cpe/things/",
            "/cpe/things/things/lamp",
            "/cpe/_static/cpe/tokens.css",
            "/cpe/nope",
            "/cpe//evil.example/x",
        ] {
            let (status, headers, body) = get(&router, uri).await;
            assert_unavailable(status, &headers, uri);
            assert!(!body.contains("evil.example"), "{uri} must not be echoed");
        }
        for uri in ["/healthz", "/dpe/projects", "/favicon.ico"] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
    }

    /// A projection that says when it is asked and answers once the test
    /// lets it.
    struct HeldProjection {
        asked: Mutex<mpsc::Sender<()>>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl ArchiveProjection for HeldProjection {
        fn snapshot(&self, shortcode: &str) -> Result<ProjectSnapshot, ProjectionError> {
            let _ = self.asked.lock().unwrap().send(());
            // Returns when the test sends, or drops its end.
            let _ = self.release.lock().unwrap().recv_timeout(Duration::from_secs(30));
            FakeArchiveProjection::default()
                .with_project(things_snapshot())
                .snapshot(shortcode)
        }
    }

    #[tokio::test]
    async fn dpe_and_the_health_check_answer_while_cpe_activates() {
        let folder = projects_folder(&[("things", "0001")]);
        let cpe = Cpe::new(&cpe_config(folder.path(), BASE));
        let router = app(&dpe(), &cpe);
        let (asked, was_asked) = mpsc::channel();
        let (release, released) = mpsc::channel();

        let activation = activate_in_background(
            &cpe,
            HeldProjection { asked: Mutex::new(asked), release: Mutex::new(released) },
            things_files(),
        );
        tokio::task::spawn_blocking(move || was_asked.recv_timeout(Duration::from_secs(30)))
            .await
            .unwrap()
            .expect("activation asks the projection");
        assert!(!activation.is_finished(), "activation is held");

        for uri in ["/healthz", "/dpe/projects", "/dpe/projects/0803"] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
        for uri in ["/cpe/things/", "/cpe/", "/cpe/_static/cpe/tokens.css"] {
            let (status, headers, _) = get(&router, uri).await;
            assert_unavailable(status, &headers, uri);
        }

        release.send(()).expect("activation is waiting");
        activation.await.expect("activation does not panic");
        assert_eq!(status_of(router.clone(), "/cpe/things/").await, StatusCode::OK);
        assert_eq!(status_of(router, "/healthz").await, StatusCode::OK);
    }

    /// As on an image older than the one that ships the data.
    #[tokio::test]
    async fn without_syncs_data_directory_dpe_serves_and_incunabula_answers_503() {
        let root = repository_root();
        let cpe = Cpe::new(&cpe_config(&root.join("areas/access/cpe/projects"), BASE));
        let router = app(&dpe(), &cpe);

        activate_in_background(
            &cpe,
            LiveArchiveProjection::new(root.join("areas/access/sync/no-such-directory")),
            LiveDepositedFiles::new(root.join(DEFAULT_MEDIA_DATA_DIR)),
        )
        .await
        .expect("activation does not panic");

        for uri in ["/healthz", "/dpe/projects"] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
        for uri in ["/cpe/incunabula/", "/cpe/incunabula", "/cpe/incunabula/books"] {
            let (status, headers, _) = get(&router, uri).await;
            assert_unavailable(status, &headers, uri);
        }
        let (status, _, body) = get(&router, "/cpe/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains(CPE_NOT_FOUND), "{body}");
    }

    #[tokio::test]
    async fn incunabula_is_served_under_the_base_from_the_committed_data() {
        let router = app(&dpe(), incunabula());

        let (status, headers, with_slash) = get(&router, "/cpe/incunabula/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            with_slash.contains("<h1>Die Bilderfolgen der Basler Frühdrucke"),
            "Incunabula's start page"
        );
        assert!(!headers.contains_key(header::LOCATION));
        let (status, headers, without_slash) = get(&router, "/cpe/incunabula").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key(header::LOCATION), "no redirect to the slash");
        assert!(
            without_slash.contains("<h1>Die Bilderfolgen der Basler Frühdrucke"),
            "the same page without the slash"
        );

        for uri in [
            "/cpe/_static/cpe/tokens.css",
            "/cpe/incunabula/_static/cpe/tokens.css",
            "/cpe/incunabula/books/bereitung",
        ] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
        let (status, _, body) = get(&router, "/cpe/incunabula/no-such-page").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!body.contains(CPE_NOT_FOUND), "the project's not-found page");
        let folder = repository_root().join("areas/access/cpe/projects/incunabula");
        assert!(folder.join("PROVENANCE.md").is_file(), "the file the path climbs to is there");
        let (status, _, body) = get(&router, "/cpe/incunabula/_assets/..%2fPROVENANCE.md").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(
            !body.contains("where the store comes from"),
            "PROVENANCE.md is not served: {body}"
        );
    }

    /// A path written here would be a second source of the base, and the
    /// links CPE renders would leave a mount made from it.
    #[test]
    fn no_source_outside_the_tests_writes_cpes_base() {
        fn sources_under(dir: &Path, found: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    sources_under(&path, found);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    found.push(path);
                }
            }
        }

        let mut sources = Vec::new();
        sources_under(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")), &mut sources);
        let mut found_in_tests = false;
        for path in &sources {
            let source = std::fs::read_to_string(path).unwrap();
            let (production, tests) = source.split_once("#[cfg(test)]").unwrap_or((&source, ""));
            assert!(!production.contains("\"/cpe"), "{} writes a path of CPE's", path.display());
            found_in_tests |= tests.contains("\"/cpe");
        }
        // This file is one of them, and its tests name the base.
        assert!(found_in_tests, "the search reads the sources and finds the tests' constant");
    }

    /// A route of the binary's own or of DPE's is declared for one path, and
    /// CPE's nested fallback takes what no route answers.
    #[tokio::test]
    async fn a_base_that_is_a_route_of_the_binarys_or_of_dpes_leaves_that_route_answering() {
        let folder = projects_folder(&[("things", "0001")]);
        for base in [
            "/healthz",
            "/telemetry/collect",
            "/telemetry",
            "/dpe",
            "/dpe/projects",
            "/dpe/projects/0803",
            "/dpe/oai",
        ] {
            let cpe = Cpe::new(&cpe_config(folder.path(), base));
            cpe.activate(
                &FakeArchiveProjection::default().with_project(things_snapshot()),
                &things_files(),
            );
            let (router, cpe_is_mounted) = super::app(dpe().router(), &cpe);
            assert!(cpe_is_mounted, "{base}");

            for uri in [
                "/healthz",
                "/dpe/projects",
                "/dpe/projects/0803",
                "/dpe/oai?verb=Identify",
            ] {
                assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{base}: {uri}");
            }
            let beacon = Request::builder()
                .method("POST")
                .uri("/telemetry/collect")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "192.0.2.1")
                .body(Body::from("{}"))
                .unwrap();
            let status = router.clone().oneshot(beacon).await.unwrap().status();
            assert_ne!(status, StatusCode::NOT_FOUND, "{base}: the beacon's route answers");
            assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE, "{base}: the beacon's route answers");

            let (status, _, body) = get(&router, &format!("{base}/things/")).await;
            assert_eq!(status, StatusCode::OK, "{base}: CPE is under it: {body}");
        }
    }

    /// The one route the mount declares is `<base>/`, for every method. DPE
    /// has no route that ends in a slash; here it is given such a one.
    #[tokio::test]
    async fn a_base_whose_mount_panics_is_not_mounted_and_dpe_is_served_alone() {
        use std::panic::{catch_unwind, AssertUnwindSafe};

        let (_folder, cpe) = things_and_offline(BASE);
        let taken = || dpe().router().route("/cpe/", axum::routing::any(|| async { "taken" }));

        assert!(
            catch_unwind(AssertUnwindSafe(|| super::assemble(taken(), Some(&cpe)))).is_err(),
            "the mount panics on a second handler for every method of one path"
        );

        let (router, cpe_is_mounted) = super::app(taken(), &cpe);
        assert!(!cpe_is_mounted);
        for uri in ["/healthz", "/dpe/projects", "/favicon.ico"] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
        let (status, _, body) = get(&router, "/cpe/").await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "taken"));
        for uri in ["/cpe", "/cpe/things/", "/cpe/_static/cpe/tokens.css"] {
            let (status, _, body) = get(&router, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert!(
                !body.contains(CPE_NOT_FOUND) && !body.contains("Things Fixture"),
                "{uri} is DPE's: {body}"
            );
        }
    }

    #[test]
    fn a_data_directory_that_is_unset_or_empty_is_the_default() {
        assert_eq!(data_dir(None, "default"), "default");
        assert_eq!(data_dir(Some(String::new()), "default"), "default");
        assert_eq!(data_dir(Some("/data".to_owned()), "default"), "/data");
    }

    /// Or a run from the checkout serves no project.
    #[test]
    fn the_default_sync_data_directory_is_the_committed_one() {
        assert!(repository_root().join(DEFAULT_SYNC_DATA_DIR).join("0803.nq").is_file());
    }

    /// A run from the checkout finds the committed media data, or it serves no project.
    #[test]
    fn the_default_media_data_directory_is_the_committed_one() {
        assert!(repository_root().join(DEFAULT_MEDIA_DATA_DIR).join("0803/config.kdl").is_file());
    }

    /// Nothing of CPE's answers at the root either.
    #[tokio::test]
    async fn without_a_base_cpe_is_not_mounted_and_dpe_is_served_alone() {
        let (_folder, unset) = things_and_offline("");
        let (_folder, refused) = things_and_offline("cpe/");
        assert_eq!(unset.projects().len(), 1, "the unset base is the root, and CPE activates");
        assert!(refused.projects().is_empty(), "a refused base is not activated");

        let dpe_alone = [
            "/things/",
            "/things/things/lamp",
            "/offline/",
            "/_static/cpe/tokens.css",
            "/cpe",
            "/cpe/",
            "/cpe/things/",
            "/cpe/_static/cpe/tokens.css",
        ];
        for (what, cpe) in [("unset", &unset), ("refused", &refused)] {
            assert_eq!(cpe.base(), "", "{what}");
            let router = app(&dpe(), cpe);
            for uri in dpe_alone {
                let (status, _, body) = get(&router, uri).await;
                assert_eq!(status, StatusCode::NOT_FOUND, "{what}: {uri}");
                assert!(
                    !body.contains(CPE_NOT_FOUND) && !body.contains("Things Fixture"),
                    "{what}: {uri} is DPE's: {body}"
                );
            }
            for uri in ["/healthz", "/dpe/projects", "/favicon.ico"] {
                assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{what}: {uri}");
            }
        }
    }
}
