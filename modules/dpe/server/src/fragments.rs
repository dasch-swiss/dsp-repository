//! Fragment handlers for Datastar SSE-driven content updates.
//!
//! Each handler renders Maud `Markup` to an HTML string and returns it as an SSE
//! stream of Datastar events (PatchElements + ExecuteScript). The tab fragment
//! reuses the single `project_tabs` renderer from `dpe-web`, so the SSE morph
//! and the full-page render can never drift.

use std::convert::Infallible;

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use datastar::axum::ReadSignals;
use datastar::prelude::{ExecuteScript, PatchElements};
use dpe_core::project::{is_valid_shortcode, VALID_TABS};
use dpe_core::Page;
use dpe_web::domain::{get_contributors, get_project, list_projects};
use dpe_web::pages::project::components::project_details_tabs::{has_publications, project_tabs};
use futures::stream::{self, Stream};
use maud::html;
use mosaic_tiles::icon::{icon, IconSearch};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TabParams {
    pub id: String,
    pub tab: String,
}

/// SSE fragment handler for project tab switching.
///
/// Returns a Datastar SSE stream with:
/// 1. PatchElements — outer-morphs `#project-tabs` (tab bar + panel)
/// 2. ExecuteScript — updates the browser URL via `history.replaceState`
/// 3. ExecuteScript — restores focus to the active tab after the patch
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "SSE tab_fragment",
        shortcode = tracing::field::Empty,
        tab = tracing::field::Empty,
    )
)]
pub async fn tab_fragment_handler(
    Path(params): Path<TabParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, impl IntoResponse> {
    let TabParams { id, tab } = params;
    tracing::Span::current().record("shortcode", id.as_str());
    tracing::Span::current().record("tab", tab.as_str());

    // Validate shortcode format (alphanumeric only — prevents XSS in ExecuteScript)
    if !is_valid_shortcode(&id) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Validate tab name
    if !VALID_TABS.contains(&tab.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Load project data
    let project = get_project(&id).ok_or(StatusCode::NOT_FOUND)?;

    // If the publications tab is requested but the project has none, return 404
    let has_publications_tab = has_publications(&project);
    if tab == "publications" && !has_publications_tab {
        return Err(StatusCode::NOT_FOUND);
    }

    // Load contributors only when rendering the contributors tab
    let contributors = if tab == "contributors" {
        get_contributors(project.attributions.clone())
    } else {
        vec![]
    };

    // Render the `#project-tabs` morph root (the same renderer the full page uses)
    let html = project_tabs(&project, &contributors, &tab, has_publications_tab).into_string();

    let patch = PatchElements::new(html).selector("#project-tabs").use_view_transition(true);

    let url_update = ExecuteScript::new(format!("history.replaceState({{}}, '', '/dpe/projects/{}?tab={}')", id, tab));

    // Restore focus to the active tab after the DOM patch (prevents focus loss to <body>)
    let focus_tab = ExecuteScript::new(format!("document.getElementById('tab-{}')?.focus()", tab));

    let stream = stream::iter(vec![
        Ok::<_, Infallible>(patch.into()),
        Ok(url_update.into()),
        Ok(focus_tab.into()),
    ]);

    Ok(Sse::new(stream))
}

// --- Search Fragment Handler ---

#[derive(Deserialize)]
pub struct SearchSignals {
    pub search: String,
}

/// SSE fragment handler for project search autocomplete.
///
/// Returns search results as a Datastar PatchElements event targeting
/// `#search-results`. Called by the `data-on:input__debounce` on the search
/// input.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "SSE search_fragment",
    )
)]
pub async fn search_fragment_handler(
    ReadSignals(signals): ReadSignals<SearchSignals>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let search = signals.search.trim().to_string();
    if search.len() > 200 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let results = if search.is_empty() {
        Page { items: vec![], nr_pages: 0, total_items: 0 }
    } else {
        list_projects(
            None,                 // ongoing
            None,                 // finished
            Some(search.clone()), // search
            None,                 // page
            Some(5),              // page_size
            None,                 // type_of_data
            None,                 // data_language
            None,                 // access_rights
        )
    };

    let html = render_search_results(&search, &results);

    let patch = PatchElements::new(html)
        .selector("#search-results")
        .mode(datastar::prelude::ElementPatchMode::Inner);

    let stream = stream::iter(vec![Ok::<_, Infallible>(patch.into())]);
    Ok(Sse::new(stream))
}

/// Render the search-results dropdown content. The query echo is a plain,
/// auto-escaped Maud splice — never `PreEscaped` (it is user-controlled).
fn render_search_results(query: &str, results: &Page) -> String {
    if results.items.is_empty() {
        return html! { p class="text-sm text-neutral-500 px-2 py-1" { "No results" } }.into_string();
    }

    let encoded_query = urlencoding::encode(query);
    html! {
        ul role="listbox" {
            @for p in &results.items {
                li role="option" {
                    a href=(format!("/dpe/projects/{}", p.shortcode))
                      class="block px-4 py-3 hover:bg-neutral-100 transition-colors text-sm" {
                        div class="font-medium text-neutral-700" { (p.name) }
                        div class="text-sm text-neutral-500 truncate mt-0.5" { (p.short_description) }
                    }
                }
            }
        }
        div class="border-t border-neutral-200 mt-1 pt-1" {
            a href=(format!("/dpe/projects?search={encoded_query}"))
              class="flex items-center gap-2 px-2 py-1 hover:bg-neutral-100 rounded text-sm text-neutral-500" {
                (icon(IconSearch, "w-4 h-4"))
                (format!("Search for \"{query}\" ({} results)", results.total_items))
            }
        }
    }
    .into_string()
}

pub async fn projects_json_handler() -> impl IntoResponse {
    use dpe_core::project::ProjectRaw;
    use dpe_core::project_repository::{FsProjectRepository, ProjectRepository};

    let repo = FsProjectRepository::new();
    let projects: Vec<ProjectRaw> = repo.get_all().iter().map(ProjectRaw::from).collect();
    axum::Json(projects).into_response()
}

pub async fn project_json_handler(Path(id): Path<String>) -> impl IntoResponse {
    use dpe_core::project::is_valid_shortcode;
    use dpe_core::project_repository::{FsProjectRepository, ProjectRepository};

    if !is_valid_shortcode(&id) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let repo = FsProjectRepository::new();
    match repo.get_by_shortcode(&id) {
        Some(proj) => axum::Json(dpe_core::project::ProjectRaw::from(proj)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxumStatusCode};
    use axum::routing::get;
    use axum::Router;
    use dpe_core::Project;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    // --- Unit tests: is_valid_shortcode ---

    #[test]
    fn valid_shortcode_alphanumeric() {
        assert!(is_valid_shortcode("0803"));
        assert!(is_valid_shortcode("080C"));
        assert!(is_valid_shortcode("abc123"));
    }

    #[test]
    fn invalid_shortcode_empty() {
        assert!(!is_valid_shortcode(""));
    }

    #[test]
    fn invalid_shortcode_special_chars() {
        assert!(!is_valid_shortcode("08/03"));
        assert!(!is_valid_shortcode("../etc"));
        assert!(!is_valid_shortcode("<script>"));
        assert!(!is_valid_shortcode("ab cd"));
        assert!(!is_valid_shortcode("ab-cd"));
        assert!(!is_valid_shortcode("ab_cd"));
    }

    #[test]
    fn invalid_shortcode_xss_attempt() {
        assert!(!is_valid_shortcode("0803'OR'1'='1"));
        assert!(!is_valid_shortcode("0803&tab=overview"));
    }

    // --- Unit tests: render_search_results ---

    #[test]
    fn search_results_empty_renders_no_results() {
        let page = Page { items: vec![], nr_pages: 0, total_items: 0 };
        let html = render_search_results("anything", &page);
        assert!(html.contains("No results"), "{html}");
    }

    #[test]
    fn search_query_echo_is_html_escaped() {
        // The user-controlled query must be auto-escaped, never raw — no XSS.
        let page = Page { items: vec![], nr_pages: 0, total_items: 0 };
        // Empty items short-circuits, so use a non-empty page to reach the echo.
        let page = Page { items: vec![sample_project()], ..page };
        let html = render_search_results(r#"<img src=x onerror=alert(1)>"#, &page);
        assert!(!html.contains("<img src=x"), "raw script must not appear: {html}");
        assert!(html.contains("&lt;img src=x"), "query must be escaped: {html}");
    }

    /// Minimal project for the search-echo escaping test.
    fn sample_project() -> Project {
        Project {
            id: "0001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/0001".to_string(),
            name: "Test".to_string(),
            shortcode: "0001".to_string(),
            official_name: "Test".to_string(),
            status: dpe_core::ProjectStatus::Ongoing,
            short_description: "desc".to_string(),
            description: std::collections::HashMap::new(),
            start_date: "2020".to_string(),
            end_date: "2024".to_string(),
            url: None,
            secondary_url: None,
            how_to_cite: "cite".to_string(),
            access_rights: dpe_core::AccessRights {
                access_rights: dpe_core::AccessRightsType::FullOpenAccess,
                embargo_date: None,
            },
            legal_info: vec![],
            data_management_plan: None,
            data_publication_year: None,
            type_of_data: None,
            data_language: None,
            clusters: vec![],
            collections: vec![],
            collection_ids: vec![],
            records: None,
            keywords: vec![],
            disciplines: vec![],
            temporal_coverage: vec![],
            spatial_coverage: vec![],
            attributions: vec![],
            abstract_text: None,
            contact_point: None,
            publications: None,
            funding: dpe_core::project::Funding::Text("None".to_string()),
            alternative_names: None,
            documentation_material: None,
            provenance: None,
            additional_material: None,
        }
    }

    // --- Integration tests: tab_fragment_handler via tower ---

    fn init_test_data() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let data_dir = format!("{}/data", env!("CARGO_MANIFEST_DIR"));
            dpe_core::set_data_dir(&data_dir);
            // Show placeholders in tests to exercise the red-styled rendering path
            dpe_core::set_show_placeholder_values(true);
        });
    }

    fn test_app() -> Router {
        init_test_data();
        Router::new().route("/dpe/projects/{id}/tab/{tab}", get(tab_fragment_handler))
    }

    #[tokio::test]
    async fn handler_invalid_tab_returns_404() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_invalid_shortcode_returns_404() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/../etc/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_nonexistent_project_returns_404() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/9999/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_overview_tab_returns_sse_stream() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::OK);

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("text/event-stream"),
            "expected SSE content type, got: {content_type}"
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("datastar-patch-elements"),
            "SSE body should contain patch-elements event, got: {}",
            &body_str[..body_str.len().min(500)]
        );
    }

    #[tokio::test]
    async fn handler_sse_contains_replace_state_script() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/contributors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("history.replaceState"),
            "SSE body should contain replaceState call"
        );
        assert!(
            body_str.contains("/dpe/projects/0803?tab=contributors"),
            "replaceState URL should match request"
        );
    }

    #[tokio::test]
    async fn handler_lowercase_shortcode_for_uppercase_project_returns_ok() {
        // Project 080C exists in the test data with an uppercase 'C'.
        // Requesting it with a lowercase 'c' should still resolve.
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/080c/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::OK);
    }

    #[tokio::test]
    async fn handler_sse_html_contains_aria_attributes() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        assert!(
            body_str.contains(r#"role=\"tablist\""#) || body_str.contains(r#"role="tablist""#),
            "missing tablist role in SSE HTML"
        );
        assert!(body_str.contains("aria-selected"), "missing aria-selected on active tab");
    }

    /// Extract the `<div id="project-tabs" …>` opening tag, up to and including
    /// the first `>`. The morph-root open tag carries no `>` inside its attribute
    /// values, so the first `>` terminates it. Both the Maud page render and the
    /// SSE `elements` line escape attribute values identically (`&` → `&amp;`),
    /// so the two open tags are directly comparable without entity-decoding.
    fn project_tabs_open_tag(html: &str) -> &str {
        let start = html.find(r#"<div id="project-tabs""#).expect("no #project-tabs element");
        let end = html[start..].find('>').expect("unterminated #project-tabs open tag");
        &html[start..start + end + 1]
    }

    /// Morph contract: the `#project-tabs` open tag the full page emits (wrapped
    /// in a card by `project_details`) must be byte-identical to the one the
    /// `/tab/{tab}` SSE route emits. Both render through the single `project_tabs`
    /// fn, so this guards against a caller wrapping or mutating the morph root and
    /// silently breaking Datastar's outer-morph.
    #[tokio::test]
    async fn morph_contract_project_tabs_open_tag_identical() {
        init_test_data();

        // Page path: the full project-detail view wraps `project_tabs` in a card.
        let project = dpe_core::project_cache::project_by_shortcode("0803")
            .expect("project 0803 missing in test data")
            .clone();
        let page_html =
            dpe_web::pages::project::components::project_details::project_details(&project, &[], "overview")
                .into_string();
        let page_open_tag = project_tabs_open_tag(&page_html);

        // SSE path: the tab fragment handler emits `#project-tabs` via PatchElements.
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);
        let sse_open_tag = project_tabs_open_tag(&body_str);

        assert_eq!(
            page_open_tag, sse_open_tag,
            "the #project-tabs morph-root open tag drifted between the page and SSE paths"
        );
    }

    #[tokio::test]
    async fn snapshot_sse_overview_response() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/dpe/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        insta::with_settings!({
            filters => vec![
                (r"https://ark\.dasch\.swiss/ark:/\S+", "[ARK-URL]"),
            ],
        }, {
            insta::assert_snapshot!("sse_overview_response", body_str.as_ref());
        });
    }

    // --- Snapshot tests: project_sidebar (covers LegalInfo, ContactSection,
    //     FundingSection, EntityName, OrganizationName, Person, AffiliationName) ---

    fn render_project_sidebar(project: &Project) -> String {
        dpe_web::pages::project::components::project_sidebar::project_sidebar(project).into_string()
    }

    /// Snapshot the sidebar for a real project (0803) — exercises `person`
    /// (resolves `person-073`), `organization_name` (resolves `organization-002`
    /// from the funding section), and the affiliation chain for the contact.
    #[test]
    fn snapshot_project_sidebar_real_project() {
        init_test_data();
        let project = dpe_core::project_cache::project_by_shortcode("0803")
            .expect("project 0803 missing in test data")
            .clone();
        let html = render_project_sidebar(&project);
        insta::assert_snapshot!("project_sidebar_real_project", html);
    }

    /// Snapshot the sidebar for a project whose `copyright_holder` and
    /// `authorship` hold real person/organization IDs — the path that exercises
    /// `entity_name`'s person and organization branches.
    #[test]
    fn snapshot_project_sidebar_with_entity_ids() {
        use dpe_core::{AccessRights, AccessRightsType, Funding, ProjectStatus};

        init_test_data();
        let project = Project {
            id: "test-entity-ids".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/0001".to_string(),
            name: "Entity-id sidebar fixture".to_string(),
            shortcode: "0001".to_string(),
            official_name: "Entity-id sidebar fixture".to_string(),
            status: ProjectStatus::Ongoing,
            short_description: "Fixture exercising EntityName person/org branches".to_string(),
            description: std::collections::HashMap::new(),
            start_date: "2020-01-01".to_string(),
            end_date: "2025-12-31".to_string(),
            url: None,
            secondary_url: None,
            how_to_cite: "Fixture (2024)".to_string(),
            access_rights: AccessRights {
                access_rights: AccessRightsType::FullOpenAccess,
                embargo_date: None,
            },
            legal_info: vec![dpe_core::LegalInfo {
                license: dpe_core::License {
                    license_identifier: "CC BY 4.0".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                    license_date: "2024-01-01".to_string(),
                },
                copyright_holder: "person-073".to_string(),
                authorship: vec!["person-073".to_string(), "organization-002".to_string()],
            }],
            data_management_plan: None,
            data_publication_year: None,
            type_of_data: None,
            data_language: None,
            clusters: vec![],
            collections: vec![],
            collection_ids: vec![],
            records: None,
            keywords: vec![],
            disciplines: vec![],
            temporal_coverage: vec![],
            spatial_coverage: vec![],
            attributions: vec![],
            abstract_text: None,
            contact_point: Some(vec!["person-073".to_string(), "organization-002".to_string()]),
            publications: None,
            funding: Funding::Grants(vec![]),
            alternative_names: None,
            documentation_material: None,
            provenance: None,
            additional_material: None,
        };
        let html = render_project_sidebar(&project);
        insta::assert_snapshot!("project_sidebar_with_entity_ids", html);
    }
}
