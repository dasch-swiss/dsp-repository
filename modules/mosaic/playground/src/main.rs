//! Mosaic playground: a plain Axum + Maud server that renders the component
//! showcase as a server-rendered MPA.

mod app;
#[cfg(feature = "dev")]
mod dev_reload;
mod showcase;

#[tokio::main]
async fn main() {
    // Listen address and static-asset root are env-overridable so the same
    // binary serves both local dev (`just watch-mosaic-playground`) and the
    // Cloud Run container (`MOSAIC_SITE_ADDR=0.0.0.0:8080`, `MOSAIC_PUBLIC_DIR=/app/public`).
    let addr = std::env::var("MOSAIC_SITE_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let public_dir = std::env::var("MOSAIC_PUBLIC_DIR").unwrap_or_else(|_| "public".to_string());

    let router = app::router(public_dir.clone().into());

    // Dev-only browser live-reload (`dev` feature).
    #[cfg(feature = "dev")]
    let router = dev_reload::apply(router, std::path::Path::new(&public_dir));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));
    println!("listening on http://{addr}");
    axum::serve(listener, router).await.expect("server exited with error");
}
