//! `serve()` and `shutdown_signal`.

use std::process::ExitCode;

#[tokio::main]
pub(crate) async fn serve() -> ExitCode {
    use axum::http::StatusCode;
    use axum::routing::get;

    // Before OTel init, so a panic during init is captured.
    crate::observability::install_tracing_panic_hook();

    let (logger_provider, _otel_guard) = crate::observability::init_otel();

    let _pyroscope_agent = crate::observability::init_pyroscope();

    let dpe_config = crate::config::DpeConfig::load().expect("failed to load DPE configuration");
    tracing::info!(data_dir = %dpe_config.data_dir.display(), "DPE configuration loaded");

    if let Some(ref site_id) = dpe_config.fathom_site_id {
        tracing::info!(fathom_site_id = %site_id, "Fathom Analytics enabled");
    }

    // A thread-safe OnceLock rather than env mutation.
    dpe_core::set_data_dir(dpe_config.data_dir.to_str().expect("data_dir path must be valid UTF-8"));

    // The same directory ServeDir serves, so a cover present under it is reachable at
    // its URL. dpe-core scans it to resolve cover presence at render time.
    dpe_core::set_public_dir(dpe_config.public_dir.to_str().expect("public_dir path must be valid UTF-8"));

    // Set the public OAI-PMH base URL (thread-safe OnceLock), emitted as baseURL / <request>.
    dpe_api_oai::set_base_url(&dpe_config.oai_base_url);
    tracing::info!(oai_base_url = %dpe_config.oai_base_url, "OAI-PMH base URL set");

    // Before any cache is populated (and before `record_cache::warm`), so the ARK host is
    // normalised as data enters every cache. Moves to `sync` when that lands.
    dpe_core::set_ark_resolver_base_url(dpe_config.ark_resolver_base_url.as_deref());
    if let Some(ref url) = dpe_config.ark_resolver_base_url {
        tracing::info!(
            ark_resolver_base_url = %url,
            "this deployment publishes ARKs that resolve to itself, and serves the /ark:/ resolver"
        );
    }

    dpe_core::set_show_placeholder_values(dpe_config.show_placeholder_values);
    if dpe_config.show_placeholder_values {
        tracing::info!("Placeholder values (MISSING/CALCULATED) will be shown in the UI");
    }

    tokio::task::spawn_blocking(dpe_core::record_cache::warm);

    let addr: std::net::SocketAddr = std::env::var("DPE_SITE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()
        .expect("invalid site address (DPE_SITE_ADDR)");

    let state = crate::shell::AppState {
        fathom_site_id: dpe_config.fathom_site_id.clone(),
        css_href: crate::assets::resolve_css_href(&dpe_config.public_dir),
        public_base_url: dpe_config.public_base_url.clone(),
        oai_base_url: dpe_config.oai_base_url.clone(),
        ark_resolver_base_url: dpe_config.ark_resolver_base_url.clone(),
    };

    // Traced routes, incl. the rate-limited /dpe/oai (limiter scoped to that route).
    let app =
        crate::router::build_router(state, &dpe_config.public_dir, crate::router::rate_limited_router(&dpe_config));

    // Dev-only browser live-reload (`dev` feature): wraps the page/static
    // routes declared above; the untraced routes below stay outside it.
    #[cfg(feature = "dev")]
    let app = crate::dev_reload::apply(app, &dpe_config.public_dir);

    let app = app
        // --- Untraced routes ---
        // Routes declared AFTER .layer() calls are NOT wrapped by those layers.
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route(
            "/telemetry/collect",
            // "dpe" names the OTel instrumentation scope (`dpe.browser`), which
            // the dashboards filter on — do not change it.
            shared_telemetry::collector::collect_route("dpe", crate::page_url::normalize_page_url).layer({
                use tower_governor::governor::GovernorConfigBuilder;
                use tower_governor::GovernorLayer;

                use crate::router::RightmostXffKeyExtractor;

                let governor_conf = GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(10)
                    .key_extractor(RightmostXffKeyExtractor)
                    .finish()
                    .expect("GovernorConfig should build with valid defaults");
                GovernorLayer { config: std::sync::Arc::new(governor_conf) }
            }),
        );

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
