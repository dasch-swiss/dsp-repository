//! `serve()` and `shutdown_signal`.

use std::process::ExitCode;

use axum::Router;
use dpe_server::Dpe;

/// Assembles the app the composition root serves: DPE's router, then the
/// OTel layers, then the untraced routes.
///
/// Kept separate from `serve()` (which does I/O and process-global setup) so
/// the traced/untraced route order — routes declared after `.layer()` are not
/// wrapped by it — is unit-testable.
fn app(dpe: &Dpe) -> Router {
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};

    Router::new()
        .merge(dpe.router())
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

    let addr: std::net::SocketAddr = std::env::var("DPE_SITE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid site address (DPE_SITE_ADDR)");

    let app = app(&dpe);

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
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
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    use super::app;

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

    async fn traceparent_header_of(app: axum::Router, uri: &str) -> Option<String> {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();
        response.headers().get("traceparent").map(|v| v.to_str().unwrap().to_string())
    }

    /// `OtelInResponseLayer` only writes `traceparent` when a `tracing-opentelemetry` layer is
    /// on the global subscriber and a text-map propagator is set, both of which `serve()` does.
    /// This sets up a no-exporter equivalent once per test process, so pages get a real span id
    /// to inject; it is global for the whole test binary.
    fn init_test_otel() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            use opentelemetry::trace::TracerProvider as _;
            use tracing_subscriber::layer::SubscriberExt;

            opentelemetry::global::set_text_map_propagator(
                opentelemetry_sdk::propagation::TraceContextPropagator::new(),
            );
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
            let tracer = provider.tracer("access-server-test");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            let subscriber = tracing_subscriber::registry().with(otel_layer);
            // Set once for the whole test binary; ignore failure if another test beat us to it.
            let _ = tracing::subscriber::set_global_default(subscriber);
        });
    }

    /// The one thing this crate exists to get right: routes declared after
    /// the OTel layers carry no `traceparent` response header, and DPE's
    /// pages — declared before them — do.
    #[tokio::test]
    async fn untraced_routes_carry_no_traceparent_header_and_a_dpe_page_does() {
        init_test_otel();
        let dpe = dpe_server::Dpe::new(&test_dpe_config());
        let router = app(&dpe);

        assert_eq!(
            traceparent_header_of(router.clone(), "/healthz").await,
            None,
            "/healthz is declared after the OTel layers"
        );
        assert!(
            traceparent_header_of(router.clone(), "/dpe/projects").await.is_some(),
            "a DPE page is declared before the OTel layers"
        );
    }

    #[tokio::test]
    async fn untraced_post_to_telemetry_collect_carries_no_traceparent_header() {
        init_test_otel();
        let dpe = dpe_server::Dpe::new(&test_dpe_config());
        let router = app(&dpe);
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

    #[tokio::test]
    async fn the_assembled_router_serves_dpe_and_a_static_asset() {
        let dpe = dpe_server::Dpe::new(&test_dpe_config());
        let router = app(&dpe);

        for uri in [
            "/dpe/projects",
            "/dpe/projects/0803",
            "/dpe/oai?verb=Identify",
            "/favicon.ico",
        ] {
            assert_eq!(status_of(router.clone(), uri).await, StatusCode::OK, "{uri}");
        }
    }
}
