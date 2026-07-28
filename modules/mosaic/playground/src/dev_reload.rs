//! Dev-only browser live-reload (`dev` feature) — same shape as
//! `dpe-server/src/dev_reload.rs`: the layer reloads the browser after a
//! server restart (cargo-watch), and the asset watcher covers CSS-only
//! rebuilds (Tailwind `--watch` rewrites `app.css` without a restart).
//!
//! Datastar-safe by the layer's default response predicate: injects only into
//! `text/html`, never `text/event-stream`. Do not replace the predicate.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::Router;
use notify::Watcher;
use tower_livereload::{LiveReloadLayer, Reloader};

/// Suppress reload bursts: one asset write can emit several filesystem events.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Wrap `router` with the live-reload layer and spawn a watcher on
/// `asset_dir` that pushes a browser reload on any file change under it.
pub(crate) fn apply(router: Router, asset_dir: &Path) -> Router {
    let layer = LiveReloadLayer::new();
    spawn_asset_watcher(asset_dir, layer.reloader());
    router.layer(layer)
}

fn spawn_asset_watcher(asset_dir: &Path, reloader: Reloader) {
    let last_reload = Mutex::new(Instant::now() - DEBOUNCE);
    let mut watcher = match notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
        if event.is_err() {
            return;
        }
        let mut last = last_reload.lock().expect("live-reload debounce lock poisoned");
        if last.elapsed() >= DEBOUNCE {
            *last = Instant::now();
            reloader.reload();
        }
    }) {
        Ok(watcher) => watcher,
        Err(e) => {
            eprintln!("live-reload: failed to create asset watcher: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(asset_dir, notify::RecursiveMode::Recursive) {
        eprintln!("live-reload: failed to watch {}: {e}", asset_dir.display());
        return;
    }
    println!("live-reload: watching {} for asset changes", asset_dir.display());
    // The watcher stops on drop; keep it alive for the (dev-only) process lifetime.
    std::mem::forget(watcher);
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::response::{Html, IntoResponse};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use super::*;

    // The watcher→reload wiring itself is covered by
    // `dpe-server::dev_reload::tests::asset_change_emits_reload_event`
    // (identical code); here we pin the injection predicate on this router.

    const PAGE: &str = "<html><head></head><body>showcase</body></html>";
    const SSE_BODY: &str = "event: datastar-patch-elements\ndata: elements <div id=\"x\"></div>\n\n";

    fn test_router() -> Router {
        let router = Router::new().route("/page", get(|| async { Html(PAGE) })).route(
            "/sse",
            get(|| async { ([(header::CONTENT_TYPE, "text/event-stream")], SSE_BODY).into_response() }),
        );
        apply(router, Path::new("."))
    }

    async fn body_string(router: Router, uri: &str) -> (StatusCode, String) {
        let response = router
            .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
        (status, String::from_utf8(bytes.to_vec()).expect("utf8 body"))
    }

    /// HTML pages get the live-reload client injected.
    #[tokio::test]
    async fn injects_livereload_script_into_html_responses() {
        let (status, body) = body_string(test_router(), "/page").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("showcase"), "page content must survive: {body}");
        assert!(
            body.contains("/_tower-livereload"),
            "expected injected live-reload script: {body}"
        );
    }

    /// SSE responses pass through byte-identical.
    #[tokio::test]
    async fn leaves_sse_responses_untouched() {
        let (status, body) = body_string(test_router(), "/sse").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, SSE_BODY, "SSE body must not be modified by live-reload");
    }
}
