mod app_state;
mod error;
mod handlers;
mod routes;

use core::error::Error;
use std::path::Path;
use std::sync::Arc;

use services::metadata::MetadataServiceImpl;
use services::metadata_v2::project_repository::ProjectRepository;
use storage::metadata::InMemoryMetadataRepository;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(app_state::AppState {
        metadata_service: MetadataServiceImpl {
            repo: InMemoryMetadataRepository::new_from_path(Path::new("./data")),
        },
        project_repository: ProjectRepository {},
    });

    let cors = CorsLayer::new()
        .expose_headers(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let app = routes::routes().with_state(app_state).layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3333").await.unwrap();

    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
