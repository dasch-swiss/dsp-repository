//! The root's own response glue: the 404 that `ServeDir` misses and unrouted paths fall back to.

/// 404 fallback for `ServeDir` misses and for paths no route matches. Always renders signed
/// out: `ServeDir`'s not-found service has no session in hand.
pub(crate) async fn not_found(
    axum::extract::State(css_href): axum::extract::State<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let traceparent = crate::traceparent::extract_traceparent();
    let body = deposit_shell::view::page(
        "Page not found — DaSCH Metadata Editor",
        traceparent.as_deref(),
        &css_href,
        None,
        deposit_shell::pages::not_found::not_found(),
    );
    (axum::http::StatusCode::NOT_FOUND, axum::response::Html(body.into_string())).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    #[tokio::test]
    async fn not_found_renders_the_page_shell_with_a_404() {
        let response = not_found(axum::extract::State(test_support::CSS_HREF.to_string())).await;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        // It has to arrive inside the shell, with the header's route back.
        let body = test_support::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("DaSCH Metadata Editor"), "{body}");
        assert!(body.contains("Page not found"), "{body}");
    }
}
