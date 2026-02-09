mod api;
mod app_state;
mod db;
mod domain;
mod error;
mod services;
mod views;

use core::error::Error;
use std::sync::Arc;

use services::project_repository::ProjectRepository;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_tracing();
    db::setup_db();
    run_http().await;

    Ok(())
}

fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn run_http() {
    let app_state = Arc::new(app_state::AppState { project_repository: ProjectRepository {} });

    let cors = CorsLayer::new()
        .expose_headers(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = api::routes().with_state(app_state).layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3333").await.unwrap();

    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
