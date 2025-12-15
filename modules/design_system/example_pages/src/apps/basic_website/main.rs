use std::net::SocketAddr;

use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod api;
mod components;
mod layout;
mod pages;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "basic_website=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Build application router
    let pages_router = Router::new()
        .route("/", get(pages::home))
        .route("/projects", get(pages::projects))
        .route("/services", get(pages::services))
        .route("/knowledge-hub", get(pages::knowledge_hub))
        .route("/about-us", get(pages::about_us))
        .route("/faq", get(pages::faq))
        .route("/contact", get(pages::contact))
        .route("/news", get(pages::news))
        .route("/status", get(pages::status));

    let api_router = Router::new()
        .route("/stats/stream", get(api::stats_stream_handler))
        .route("/status/stream", get(api::status_stream_handler))
        .route("/project/{id}", get(api::project_detail_handler));

    let app = Router::new()
        .merge(pages_router)
        .nest("/api", api_router)
        .nest_service("/assets", ServeDir::new("assets"));

    // Determine port (default to 3500)
    let port = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3500);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Basic Website example listening on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
