//! Mosaic playground: a plain Axum + Maud server that renders the component
//! showcase as a server-rendered MPA.

mod app;
mod showcase;

#[tokio::main]
async fn main() {
    // Listen address and static-asset root are env-overridable so the same
    // binary serves both local dev (`just watch-mosaic-playground`) and the
    // Cloud Run container (`MOSAIC_SITE_ADDR=0.0.0.0:8080`, `MOSAIC_PUBLIC_DIR=/app/public`).
    let addr = std::env::var("MOSAIC_SITE_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let public_dir = std::env::var("MOSAIC_PUBLIC_DIR").unwrap_or_else(|_| "public".to_string());

    let router = app::router(public_dir.into());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    println!("listening on http://{addr}");
    axum::serve(listener, router).await.expect("server exited with error");
}
