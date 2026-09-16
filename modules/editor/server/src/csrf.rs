//! CSRF defence: `Sec-Fetch-Site: same-origin` on every state-changing request.
//!
//! `SameSite=Lax` on the session cookie is **not** what closes CSRF here: it is
//! scoped to the registrable domain, not the origin, so a request from any other
//! `*.dasch.swiss` host counts as same-site and carries the cookie. Nor is
//! `Datastar-Request: true` a substitute — the progressive-enhancement path is a
//! plain `<form method="post">`, which sends nothing of the kind. The argument is
//! in `docs/src/editor/authentication.md`.

use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The only value that may accompany a state-changing request.
const SAME_ORIGIN: &str = "same-origin";

/// `GET` and `HEAD` are exempt, which is only sound while no `GET` handler
/// mutates state — an invariant carried by the method-discipline tests in
/// [`crate::router`], not by this middleware.
fn is_state_changing(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD)
}

/// Reject a state-changing request unless `Sec-Fetch-Site` is exactly
/// `same-origin`.
///
/// **Fails closed**: `same-site`, `cross-site`, `none`, an unparseable value and
/// an *absent* header are all refused. Treating absent as permissive would be the
/// bypass, at the accepted cost that a browser too old to send `Sec-Fetch-*`
/// cannot use the editor.
///
/// The response is plain text rather than the page shell: this middleware is the
/// outermost layer, so it also covers the telemetry beacon, which expects no
/// HTML.
pub(crate) async fn require_same_origin(req: Request, next: Next) -> Response {
    if !is_state_changing(req.method()) {
        return next.run(req).await;
    }

    let site = req.headers().get("sec-fetch-site").and_then(|value| value.to_str().ok());
    if site == Some(SAME_ORIGIN) {
        return next.run(req).await;
    }

    // The value is logged because it is a fixed, four-valued enum plus absent —
    // no user data, and it is what distinguishes "cross-site attack" from
    // "browser does not send the header".
    tracing::warn!(
        http.request.method = %req.method(),
        sec_fetch_site = site.unwrap_or("<absent>"),
        "refused a state-changing request without Sec-Fetch-Site: same-origin"
    );
    (
        StatusCode::FORBIDDEN,
        "This request was refused because it did not originate from the editor itself.\n",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    use super::*;

    fn app() -> Router {
        Router::new()
            .route("/read", get(|| async { "read" }))
            .route("/write", post(|| async { "written" }))
            .layer(axum::middleware::from_fn(require_same_origin))
    }

    async fn status(method: &str, path: &str, site: Option<&str>) -> StatusCode {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(site) = site {
            builder = builder.header("sec-fetch-site", site);
        }
        app().oneshot(builder.body(Body::empty()).unwrap()).await.unwrap().status()
    }

    #[tokio::test]
    async fn test_same_origin_post_is_allowed() {
        assert_eq!(status("POST", "/write", Some("same-origin")).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_same_site_post_is_refused() {
        // The case `SameSite=Lax` cannot see: another *.dasch.swiss host is
        // same-site, and its request would carry our cookie.
        assert_eq!(status("POST", "/write", Some("same-site")).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_cross_site_and_none_posts_are_refused() {
        assert_eq!(status("POST", "/write", Some("cross-site")).await, StatusCode::FORBIDDEN);
        assert_eq!(status("POST", "/write", Some("none")).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_an_absent_header_is_refused() {
        // Fails closed: treating absent as permissive would be the bypass.
        assert_eq!(status("POST", "/write", None).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_an_unparseable_header_is_refused() {
        let req = Request::builder()
            .method("POST")
            .uri("/write")
            .header("sec-fetch-site", [0xff, 0xfe].as_slice())
            .body(Body::empty())
            .unwrap();
        let status = app().oneshot(req).await.unwrap().status();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_a_value_that_merely_contains_same_origin_is_refused() {
        // Equality, not a substring or prefix test.
        assert_eq!(
            status("POST", "/write", Some("same-origin, cross-site")).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(status("POST", "/write", Some("not-same-origin")).await, StatusCode::FORBIDDEN);
        assert_eq!(status("POST", "/write", Some("Same-Origin")).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_get_and_head_pass_without_the_header() {
        // Navigations must work: a link followed from anywhere is a GET, and no
        // GET handler mutates state.
        assert_eq!(status("GET", "/read", None).await, StatusCode::OK);
        assert_eq!(status("HEAD", "/read", None).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_every_other_method_needs_the_header() {
        // Not just POST: PUT, PATCH and DELETE change state too, and OPTIONS is
        // refused because the editor serves no CORS preflight.
        for method in ["PUT", "PATCH", "DELETE", "OPTIONS"] {
            assert_eq!(
                status(method, "/write", None).await,
                StatusCode::FORBIDDEN,
                "{method} must require the header"
            );
        }
    }
}
