mod livereload;
mod playground;

use core::error::Error;

use axum::routing::get;
use axum::{Extension, Router};
use livereload::{reload_ws, trigger_reload, ReloadNotifier};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// TODO: rustdoc everything

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new()
        .expose_headers(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    let notifier = ReloadNotifier::new();

    let routes = Router::new()
        .route("/", get(playground::shell::shell))
        .route("/iframe", get(playground::iframe::iframe_component))
        .route("/reload-ws", get(reload_ws))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/playground-assets", ServeDir::new("modules/design_system/playground/assets"))
        .route_service(
            "/design-system/tailwind.css",
            ServeFile::new("modules/design_system/tailwind.css"),
        )
        .layer(Extension(notifier.clone()));

    let app = routes.layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3400").await.unwrap();

    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        tracing::debug!("Starting livereload task");
        trigger_reload(Extension(notifier)).await;
    });

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
