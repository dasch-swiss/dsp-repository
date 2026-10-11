//! The answer for what CPE knows and cannot serve: status 503.
//!
//! "Not ready" is not "not there". A project that did not load, and all of
//! CPE until its projects are loaded, answer 503 with `Retry-After` and
//! `Cache-Control: no-store`, so that no client and no cache takes the answer
//! for a missing page. 404 is kept for a path that names no project.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use maud::{html, Markup, DOCTYPE};

/// Seconds a client is asked to wait before it asks again.
const RETRY_AFTER: &str = "30";

const TITLE: &str = "CPE — temporarily unavailable";

/// The 503 response. It takes nothing from the request, so it can echo
/// nothing.
pub(crate) fn response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::RETRY_AFTER, RETRY_AFTER), (header::CACHE_CONTROL, "no-store")],
        page(),
    )
        .into_response()
}

/// The page's whole style. Light only, in the system's own typeface.
const STYLE: &str = "\
body{margin:0;font:1rem/1.5 system-ui,sans-serif;color:#1c1b19;background:#f7f4ee}\
main{max-width:34rem;margin:0 auto;padding:4rem 1.5rem}\
h1{margin:0 0 1rem;font-size:1.5rem;line-height:1.25}\
p{margin:0}";

/// A plain page in English that references no URL: no stylesheet, script,
/// font, image or link. Until the projects are loaded every path of CPE is
/// answered with this page, the static files included, so anything it asked
/// for would be answered 503 too.
fn page() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (TITLE) }
                style { (STYLE) }
            }
            body {
                main {
                    h1 { "Temporarily unavailable" }
                    p {
                        "This part of the site is starting or could not be loaded. Please try again shortly."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_answer_is_503_never_cached_and_a_page() {
        let response = response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()[header::RETRY_AFTER], RETRY_AFTER);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body is small and finite in tests");
        let body = String::from_utf8(body.into()).expect("the page is UTF-8");
        assert!(body.contains("<h1>Temporarily unavailable</h1>"), "{body}");
    }

    /// What a browser would fetch, or a reader follow, from the page: nothing.
    #[test]
    fn the_page_references_no_url() {
        let page = page().into_string();
        for reference in [
            "href", "src", "url(", "@import", "<link", "<script", "<img", "<a ", "http", "//",
        ] {
            assert!(!page.contains(reference), "the page holds {reference:?}: {page}");
        }
        assert!(
            page.contains("<style>") && page.contains("<title>CPE — temporarily unavailable</title>"),
            "{page}"
        );
        assert_eq!(page.matches("<h1>").count(), 1, "{page}");
        assert!(page.contains(r#"<html lang="en">"#) && page.contains("<main>"), "{page}");
        assert!(!page.contains("http-equiv"), "no refresh: {page}");
    }
}
