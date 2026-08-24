//! CSRF defence: `Sec-Fetch-Site: same-origin` on every state-changing request.
//!
//! `SameSite=Lax` on the session cookie is **not** what closes CSRF here.
//! `SameSite` is scoped to the registrable domain, not the origin, so a request
//! originating from any other `*.dasch.swiss` host counts as same-site and
//! carries the cookie. For an organisation with a large subdomain estate that is
//! a live gap. The `__Host-` prefix does not close it either — it stops a
//! sibling host *setting* our cookie, not one *triggering a request that
//! carries* it.
//!
//! `Sec-Fetch-Site` distinguishes same-origin from same-site, which is exactly
//! the axis `SameSite` cannot see. It is a [forbidden request header], so page
//! script cannot set or spoof it, and OWASP ASVS 3.5.3 names it as an accepted
//! CSRF control. Baseline widely available since March 2023.
//!
//! `Datastar-Request: true` is deliberately not used instead: Datastar sets it
//! on its own actions, but the progressive-enhancement path is a plain
//! `<form method="post">`, which sends nothing of the kind. `Sec-Fetch-Site`
//! covers both paths.
//!
//! [forbidden request header]: https://developer.mozilla.org/en-US/docs/Glossary/Forbidden_request_header

use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The only value that may accompany a state-changing request.
const SAME_ORIGIN: &str = "same-origin";

/// Whether this method may change state, and therefore needs the header.
///
/// `GET` and `HEAD` are exempt because they are the navigations the header
/// exists to let through — which is only sound while no `GET` handler mutates
/// state. That invariant is the router's, not this middleware's; see the
/// method-discipline tests in [`crate::router`].
fn is_state_changing(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD)
}

/// Reject a state-changing request unless `Sec-Fetch-Site` is exactly
/// `same-origin`.
///
/// **Fails closed**, which is the whole point: `same-site`, `cross-site`,
/// `none`, an unparseable value and an *absent* header are all refused. Absent
/// is the case worth being explicit about — treating a missing header as
/// permissive would hand every attacker the bypass, since a cross-site form
/// post from a browser too old to send `Sec-Fetch-*` is indistinguishable from
/// a legitimate one. The cost is that such a browser cannot use the editor at
/// all, which is accepted.
///
/// The response is plain text rather than the page shell: this middleware is
/// the outermost layer, so it also covers the telemetry beacon, which expects
/// no HTML — and a request that reaches this branch is an attack or an
/// unsupported browser, not a user mid-task.
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

    /// A router with one GET and one POST behind the middleware.
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
        // Fails closed. Treating absent as permissive would be the bypass: a
        // cross-site post from a browser that does not send `Sec-Fetch-*` is
        // indistinguishable from a legitimate one.
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
