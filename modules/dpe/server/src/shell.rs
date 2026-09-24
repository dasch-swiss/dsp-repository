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
    /// Origin and path of the OAI endpoint, for the `describedby` targets; on DEV another host.
    /// `serve()` also sets it as `dpe-api-oai`'s process-global `baseURL` from the same
    /// `DpeConfig` field: the two must never be set apart.
    pub(crate) oai_base_url: String,
    /// Origin emitted ARKs are rewritten to, and where the `/ark:/…` resolver route answers.
    /// `None` (production, DEV, STAGE) keeps the corpus's ARKs and mounts no resolver. Same
    /// second copy in `dpe-api-oai` as `oai_base_url`.
    pub(crate) ark_resolver_base_url: Option<String>,
}

/// Query params for the project detail page: `?tab=` pre-selects the tab.
#[derive(serde::Deserialize, Default)]
pub(crate) struct TabQuery {
    #[serde(default)]
    tab: Option<String>,
}

pub(crate) async fn projects_page_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(query): axum::extract::Query<dpe_web::domain::ProjectQuery>,
) -> axum::response::Html<String> {
    let tp = traceparent::extract_traceparent();
    let content = dpe_web::pages::projects_page(&query);
    axum::response::Html(
        view::page(
            "DaSCH Metadata Browser Projects Overview",
            tp.as_deref(),
            &state.css_href,
            state.fathom_site_id.as_deref(),
            view::HeadExtras(maud::html! {}),
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
    let content = dpe_web::pages::project_page(&id, active_tab);
    // The display name when the project resolves (a cache read), else the shortcode.
    let title = dpe_core::project_cache::project_by_shortcode(&id)
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
