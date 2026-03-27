use dpe_web::*;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower_http::trace::TraceLayer;

mod config;
mod fragments;

#[tokio::main]
async fn main() {
    // Initialize structured logging via RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    // Load DPE-specific configuration (defaults → dpe.toml → DPE_* env vars)
    let dpe_config = config::DpeConfig::load().expect("failed to load DPE configuration");
    tracing::info!(data_dir = %dpe_config.data_dir.display(), "DPE configuration loaded");

    if let Some(ref site_id) = dpe_config.fathom_site_id {
        tracing::info!(fathom_site_id = %site_id, "Fathom Analytics enabled");
    }

    // Set data directory for dpe-core (thread-safe OnceLock, no env mutation)
    dpe_core::set_data_dir(&dpe_config.data_dir.to_string_lossy());

    // Load Leptos configuration from Cargo.toml metadata
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        // Health check — lightweight probe for Traefik/load balancers
        .route("/healthz", get(|| async { StatusCode::OK }))
        // OAI-PMH 2.0 endpoint (from dpe-api-oai crate)
        .route("/oai", get(dpe_api_oai::oai_handler))
        // Datastar SSE fragment endpoints
        .route("/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
        .route("/projects/search", get(fragments::search_fragment_handler))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(TraceLayer::new_for_http())
        .with_state(leptos_options);

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
