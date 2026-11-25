//! `AppState` and the page/404 handlers; `view.rs` renders the document.

use crate::{metadata, traceparent, view};

/// Shared state for the page handlers. State rather than process-globals so a test can vary
/// the per-deployment URLs.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) fathom_site_id: Option<String>,
    pub(crate) css_href: String,
    /// Origin of the site itself, for landing-page and catalogue URLs.
    pub(crate) public_base_url: String,
    /// Origin and path of the OAI endpoint, for the `describedby` targets; may be another host.
    /// `dpe-api-oai`'s `OaiState` takes the same normalised value, built alongside this one.
    pub(crate) oai_base_url: String,
    /// Origin emitted ARKs are rewritten to, and where the `/ark:/…` resolver route answers.
    /// `None` (production, STAGE) keeps the corpus's ARKs and mounts no resolver. This is
    /// the resolver route's own copy; the corpus's setting (same `DpeConfig` field) is what
    /// actually normalises the ARKs it serves.
    pub(crate) ark_resolver_base_url: Option<String>,
    /// Whether placeholder values ("MISSING", "CALCULATED") render, styled red for QA
    /// visibility, instead of being hidden. Carried through to `dpe_web::RenderContext`.
    pub(crate) show_placeholder_values: bool,
    /// The corpus every handler reads projects, records and contributors from.
    pub(crate) corpus: &'static dpe_core::Corpus,
}

impl AppState {
    /// The `dpe_web::RenderContext` every page and fragment handler renders against.
    pub(crate) fn render_context(&self) -> dpe_web::RenderContext {
        dpe_web::RenderContext {
            show_placeholder_values: self.show_placeholder_values,
            corpus: self.corpus,
        }
    }
}

/// Query params for the project detail page: `?tab=` pre-selects the tab.
#[derive(serde::Deserialize, Default)]
pub(crate) struct TabQuery {
    #[serde(default)]
    tab: Option<String>,
}

pub(crate) async fn projects_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> axum::response::Html<String> {
    let query = dpe_web::domain::ProjectQuery::from_query_str(raw.as_deref().unwrap_or_default());
    let tp = traceparent::extract_traceparent();
    let ctx = state.render_context();
    let content = dpe_web::pages::projects_page(&query, &ctx);
    // A filtered view points crawlers that ignore robots.txt, and link previews, at the list.
    let canonical = query
        .has_filter_params()
        .then(|| format!("{}/dpe/projects", state.public_base_url));
    axum::response::Html(
        view::page(
            "DaSCH Metadata Browser Projects Overview",
            tp.as_deref(),
            &state.css_href,
            state.fathom_site_id.as_deref(),
            view::HeadExtras(maud::html! {
                @if let Some(href) = canonical {
                    link rel="canonical" href=(href);
                }
            }),
            content,
        )
        .into_string(),
    )
}

pub(crate) async fn about_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    let content = dpe_web::pages::about_page();
    axum::response::Html(
        view::page(
            "DaSCH Metadata Browser — About",
            tp.as_deref(),
            &state.css_href,
            state.fathom_site_id.as_deref(),
            view::HeadExtras(maud::html! {}),
            content,
        )
        .into_string(),
    )
}

pub(crate) async fn project_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(tab): axum::extract::Query<TabQuery>,
    request_headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::http::{header, HeaderValue};

    // On every answer from this route (200 and 303, GET and HEAD), so no cache replays a 303
    // to a person or the HTML to a harvester.
    let vary = [(header::VARY, HeaderValue::from_static("Accept"))];

    // A header that is not ASCII is no `Accept` at all, which means HTML.
    let accept = request_headers.get(header::ACCEPT).and_then(|value| value.to_str().ok());
    // Built before any await: `ContributorLookup` is not `Sync`. An unresolved project gets no
    // metadata, headers or redirect.
    let (extras, headers) = match metadata::landing_page(&id, accept, &state) {
        // The one negotiation step ADR-0005 allows. Nothing else about this
        // route varies by header.
        metadata::LandingPage::Redirect(location) => {
            return axum::response::IntoResponse::into_response((
                axum::http::StatusCode::SEE_OTHER,
                vary,
                [(header::LOCATION, location)],
            ));
        }
        metadata::LandingPage::Render(markup, headers) => (markup, headers),
    };

    let tp = traceparent::extract_traceparent();
    // Fall back to "overview" for a missing or unrecognized tab, mirroring the
    // validation the SSE fragment handler applies against VALID_TABS.
    let active_tab = tab
        .tab
        .as_deref()
        .filter(|t| dpe_core::project::VALID_TABS.contains(t))
        .unwrap_or("overview");
    let ctx = state.render_context();
    let content = dpe_web::pages::project_page(&id, active_tab, &ctx);
    // The display name when the project resolves (a cache read), else the shortcode.
    let title = state
        .corpus
        .project_by_shortcode(&id)
        .map(|p| format!("{} — DaSCH Metadata Browser", p.name))
        .unwrap_or_else(|| format!("Project {id} — DaSCH Metadata Browser"));
    let body = view::page(
        &title,
        tp.as_deref(),
        &state.css_href,
        state.fathom_site_id.as_deref(),
        view::HeadExtras(extras),
        content,
    )
    .into_string();
    // A `Response` rather than `Html<String>`: this route sets headers and has
    // a `303` branch.
    axum::response::IntoResponse::into_response((vary, headers, axum::response::Html(body)))
}

/// 404 fallback (after `ServeDir` finds no matching static file): the app shell
/// with a "Page not found." body.
pub(crate) async fn not_found(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let tp = traceparent::extract_traceparent();
    let content = maud::html! {
        "Page not found."
    };
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::response::Html(
            view::page(
                "DaSCH Metadata Browser — Page Not Found",
                tp.as_deref(),
                &state.css_href,
                state.fathom_site_id.as_deref(),
                view::HeadExtras(maud::html! {}),
                content,
            )
            .into_string(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{header, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::test_support::test_state;

    /// The real public directory, so `/robots.txt` comes from the `ServeDir` fallback.
    const PUBLIC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../public");

    const CANONICAL: &str = r#"<link rel="canonical" href="https://example.test/dpe/projects">"#;

    async fn get(uri: &str) -> (StatusCode, Option<String>, String) {
        let app = crate::router::build_router(test_state(), PUBLIC_DIR.as_ref(), axum::Router::new());
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, content_type, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn robots_txt_disallows_the_filter_parameters_and_the_fragments() {
        let (status, content_type, body) = get("/robots.txt").await;
        assert_eq!(status, StatusCode::OK);
        assert!(content_type.unwrap_or_default().starts_with("text/plain"));
        for line in [
            "User-agent: *",
            "Disallow: /dpe/projects?*ongoing=",
            "Disallow: /dpe/projects?*finished=",
            "Disallow: /dpe/projects?*search=",
            "Disallow: /dpe/projects?*type_of_data=",
            "Disallow: /dpe/projects?*data_language=",
            "Disallow: /dpe/projects?*access_rights=",
            "Disallow: /dpe/projects?*dialog=",
            "Disallow: /dpe/projects/results",
            "Disallow: /dpe/projects/search",
        ] {
            assert!(body.lines().any(|l| l == line), "missing {line:?}: {body}");
        }
    }

    #[tokio::test]
    async fn robots_txt_keeps_the_paginated_list_crawlable_and_names_no_cpe_path() {
        let (_, _, body) = get("/robots.txt").await;
        for line in body.lines().filter(|l| l.starts_with("Disallow:")) {
            let path = line.trim_start_matches("Disallow:").trim();
            assert_ne!(path, "/dpe/projects?", "{line}");
            assert_ne!(path, "/dpe/projects", "{line}");
            assert!(!path.contains("page="), "{line}");
            assert!(!path.contains("/cpe"), "{line}");
        }
    }

    #[tokio::test]
    async fn filtered_projects_pages_carry_a_canonical_link_to_the_list() {
        for query in [
            "ongoing=true",
            "finished=true",
            "type_of_data=Text",
            "data_language=en",
            "access_rights=x",
            "search=a",
            "dialog=true",
            "ongoing=true&page=2",
        ] {
            let (status, _, body) = get(&format!("/dpe/projects?{query}")).await;
            assert_eq!(status, StatusCode::OK, "{query}");
            assert!(body.contains(CANONICAL), "{query}: {body}");
            assert!(!body.contains(r#"<meta name="robots""#), "{query}");
        }
    }

    #[tokio::test]
    async fn the_list_and_its_pages_carry_no_canonical_link() {
        for uri in ["/dpe/projects", "/dpe/projects?page=2"] {
            let (_, _, body) = get(uri).await;
            assert!(!body.contains(r#"rel="canonical""#), "{uri}: {body}");
            assert!(!body.contains(r#"<meta name="robots""#), "{uri}");
        }
    }
}
