//! Dev-only browser live-reload (`dev` feature).
//!
//! Wraps the router in `tower_livereload::LiveReloadLayer` and watches the
//! asset dir, so the browser reloads both when the server restarts (bacon's
//! `kill_then_restart` — the injected client reconnects and reloads) and when
//! a CSS-only rebuild lands (Tailwind `--watch` rewrites `app.css` without a
//! server restart, which the file watcher turns into a reload push).
//!
//! Datastar-safe: the layer's default response predicate injects only into
//! `text/html` responses, never `text/event-stream`, so SSE morphing is
//! untouched. Do not replace the default response predicate.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::Router;
use notify::Watcher;
use tower_livereload::{LiveReloadLayer, Reloader};

/// Suppress reload bursts: one asset write can emit several filesystem events
/// (temp file + rename), and one browser reload per write is plenty.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Wrap `router` with the live-reload layer and spawn a watcher on
/// `asset_dir` that pushes a browser reload on any file change under it.
pub(crate) fn apply<S: Clone + Send + Sync + 'static>(router: Router<S>, asset_dir: &Path) -> Router<S> {
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
            tracing::warn!("live-reload: failed to create asset watcher: {e}");
            return;
        }
    };
    if let Err(e) = watcher.watch(asset_dir, notify::RecursiveMode::Recursive) {
        tracing::warn!("live-reload: failed to watch {}: {e}", asset_dir.display());
        return;
    }
    tracing::info!("live-reload: watching {} for asset changes", asset_dir.display());
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

    const PAGE: &str = "<html><head></head><body>hello</body></html>";
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

    /// HTML pages get the live-reload client injected (script under the
    /// layer's `/_tower-livereload` prefix).
    #[tokio::test]
    async fn injects_livereload_script_into_html_responses() {
        let (status, body) = body_string(test_router(), "/page").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("hello"), "page content must survive: {body}");
        assert!(
            body.contains("/_tower-livereload"),
            "expected injected live-reload script: {body}"
        );
    }

    /// SSE responses pass through byte-identical — the injection predicate
    /// must never touch `text/event-stream` (Datastar morphing).
    #[tokio::test]
    async fn leaves_sse_responses_untouched() {
        let (status, body) = body_string(test_router(), "/sse").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, SSE_BODY, "SSE body must not be modified by live-reload");
    }

    /// Writing a file into the watched asset dir pushes a `reload` event on
    /// the layer's event stream — the CSS-only reload path (Tailwind rewrites
    /// app.css without a server restart, so only the watcher can trigger it).
    #[tokio::test(flavor = "multi_thread")]
    async fn asset_change_emits_reload_event() {
        let dir = std::env::temp_dir().join(format!("dpe-livereload-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create watch dir");

        let router = apply(Router::new().route("/page", get(|| async { Html(PAGE) })), &dir);
        let response = router
            .oneshot(
                Request::get("/_tower-livereload/event-stream")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("event-stream response");
        assert_eq!(response.status(), StatusCode::OK);

        // The stream body ends after the first reload event, so collecting it
        // completes exactly when the watcher fires.
        let collect = tokio::spawn(axum::body::to_bytes(response.into_body(), usize::MAX));

        // Give the watcher a moment, then simulate a Tailwind write.
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::fs::write(dir.join("app.css"), "body{}").expect("write asset");

        let bytes = tokio::time::timeout(Duration::from_secs(5), collect)
            .await
            .expect("event stream must complete after the asset write")
            .expect("task")
            .expect("body");
        let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        assert!(body.contains("event: reload"), "expected a reload event, got: {body}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
