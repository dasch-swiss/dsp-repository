/// Fragment handlers for Datastar SSE-driven content updates.
///
/// Each handler renders a Leptos component to an HTML string and returns it
/// as an SSE stream with Datastar events (PatchElements + ExecuteScript).
use std::borrow::Cow;
use std::convert::Infallible;

use dpe_web::domain::models::Page;
use dpe_web::domain::{get_contributors, get_project, lang_value, list_projects, Project, ResolvedContributor};
use dpe_web::pages::project::components::project_details_tabs::{
    AttributionsSectionComponent, DatasetOverviewSectionComponent, PublicationTabComponent,
    TabLink,
};
use axum::extract::Path;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use datastar::axum::ReadSignals;
use datastar::prelude::{ExecuteScript, PatchElements};
use futures::stream::{self, Stream};
use axum::http::StatusCode;
use leptos::prelude::*;
use mosaic_tiles::icon::{Document, IconSearch, Info, People};
use serde::Deserialize;

use dpe_core::project::{VALID_TABS, is_valid_shortcode};

#[derive(Deserialize)]
pub struct TabParams {
    pub id: String,
    pub tab: String,
}

/// SSE fragment handler for project tab switching.
///
/// Returns a Datastar SSE stream with:
/// 1. PatchElements — replaces #project-tabs (full tab bar + panel)
/// 2. ExecuteScript — updates browser URL via history.replaceState
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
    let project = get_project(id.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // If publications tab requested but project has no publications, return 404
    let has_publications_tab = has_publications(&project);
    if tab == "publications" && !has_publications_tab {
        return Err(StatusCode::NOT_FOUND);
    }

    // Load contributors if needed
    let contributors = if tab == "contributors" {
        match get_contributors(project.attributions.clone()).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(shortcode = %id, error = %e, "failed to load contributors");
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Render the full tab component (tab bar + panel) to HTML
    let html = render_project_tabs(project, contributors, &tab, has_publications_tab);

    // Strip Leptos hot-reload comments that interfere with Datastar morphing
    let html = strip_hot_reload_comments(&html);

    // Build SSE events
    let patch = PatchElements::new(html)
        .selector("#project-tabs")
        .use_view_transition(true);

    let url_update = ExecuteScript::new(format!(
        "history.replaceState({{}}, '', '/projects/{}?tab={}')",
        id, tab
    ));

    // Restore focus to the active tab after DOM patch (prevents focus loss to <body>)
    let focus_tab = ExecuteScript::new(format!(
        "document.getElementById('tab-{}')?.focus()",
        tab
    ));

    let stream = stream::iter(vec![
        Ok::<_, Infallible>(patch.into()),
        Ok(url_update.into()),
        Ok(focus_tab.into()),
    ]);

    Ok(Sse::new(stream))
}

/// Check whether the project has content for a publications tab.
fn has_publications(project: &Project) -> bool {
    let has_abstract = project
        .abstract_text
        .as_ref()
        .and_then(|m| lang_value(m).cloned())
        .is_some();
    let has_pubs = project
        .publications
        .as_ref()
        .map(|p| !p.is_empty())
        .unwrap_or(false);
    has_abstract || has_pubs
}

/// Render the full project tabs component (tab bar + panel) to an HTML string.
fn render_project_tabs(
    project: Project,
    contributors: Vec<ResolvedContributor>,
    active_tab: &str,
    has_publications_tab: bool,
) -> String {
    let active_tab = active_tab.to_string();
    let shortcode = project.shortcode.clone();

    let owner = Owner::new();
    owner.with(|| {
        view! {
            <div
                id="project-tabs"
                data-on:datastar-fetch="(evt.detail.type === 'error' || evt.detail.type === 'retries-failed') && evt.detail.el.closest('#project-tabs') && (window.location.href = evt.detail.el.getAttribute('href'))"
            >
                <div
                    class="tabs"
                    style="border-width: 0"
                    role="tablist"
                    aria-label="Project details"
                    aria-orientation="horizontal"
                    data-on:keydown="const tabs=[...evt.currentTarget.querySelectorAll('[role=tab]')];const idx=tabs.indexOf(evt.target);if(idx<0)return;let next;if(evt.key==='ArrowRight')next=tabs[(idx+1)%tabs.length];else if(evt.key==='ArrowLeft')next=tabs[(idx-1+tabs.length)%tabs.length];else if(evt.key==='Home')next=tabs[0];else if(evt.key==='End')next=tabs[tabs.length-1];else if(evt.key===' '){evt.preventDefault();evt.target.click();return}else return;evt.preventDefault();next.focus()"
                >
                    <TabLink
                        value="overview"
                        active_tab=active_tab.clone()
                        icon=Info
                        label="Overview"
                        shortcode=shortcode.clone()
                    />
                    {has_publications_tab
                        .then(|| {
                            view! {
                                <TabLink
                                    value="publications"
                                    active_tab=active_tab.clone()
                                    icon=Document
                                    label="Publications"
                                    shortcode=shortcode.clone()
                                />
                            }
                        })}
                    <TabLink
                        value="contributors"
                        active_tab=active_tab.clone()
                        icon=People
                        label="Contributors"
                        shortcode=shortcode.clone()
                    />
                </div>

                <div
                    id="tab-panel"
                    class="tab-panel"
                    style="display: block"
                    role="tabpanel"
                    aria-labelledby=format!("tab-{active_tab}")
                >
                    {match active_tab.as_str() {
                        "publications" if has_publications_tab => {
                            let abstract_en = project
                                .abstract_text
                                .as_ref()
                                .and_then(|m| lang_value(m).cloned());
                            let publications = project.publications.clone();
                            view! {
                                <PublicationTabComponent
                                    abstract_en=abstract_en
                                    publications=publications
                                />
                            }
                                .into_any()
                        }
                        "contributors" => {
                            view! { <AttributionsSectionComponent contributors=contributors /> }
                                .into_any()
                        }
                        _ => view! { <DatasetOverviewSectionComponent proj=project /> }.into_any(),
                    }}
                </div>
            </div>
        }
        .to_html()
    })
}


// --- Search Fragment Handler ---

#[derive(Deserialize)]
pub struct SearchSignals {
    pub search: String,
}

/// SSE fragment handler for project search autocomplete.
///
/// Returns search results as a Datastar PatchElements event targeting
/// `#search-results`. Called by the `data-on:input__debounce` on the
/// search input.
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
            None,                     // ongoing
            None,                     // finished
            Some(search.clone()),     // search
            None,                     // page
            Some(5),                  // page_size
            None,                     // type_of_data
            None,                     // data_language
            None,                     // access_rights
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    let html = render_search_results(&search, &results);
    let html = strip_hot_reload_comments(&html);

    let patch = PatchElements::new(html)
        .selector("#search-results")
        .mode(datastar::prelude::ElementPatchMode::Inner);

    let stream = stream::iter(vec![Ok::<_, Infallible>(patch.into())]);
    Ok(Sse::new(stream))
}

/// Render search results dropdown content.
fn render_search_results(query: &str, results: &Page) -> String {
    let query = query.to_string();
    let items = results.items.clone();
    let total_items = results.total_items;
    let encoded_query = urlencoding::encode(&query).to_string();

    let owner = Owner::new();
    owner.with(|| {
        if items.is_empty() {
            view! { <p class="text-sm text-base-content/60 px-2 py-1">"No results"</p> }
            .to_html()
        } else {
            view! {
                <ul role="listbox">
                    {items
                        .iter()
                        .map(|p| {
                            let shortcode = p.shortcode.clone();
                            let name = p.name.clone();
                            let desc = p.short_description.clone();
                            view! {
                                <li role="option">
                                    <a
                                        href=format!("/projects/{}", shortcode)
                                        class="block px-4 py-3 hover:bg-base-200 transition-colors text-sm"
                                    >
                                        <div class="font-medium text-base-content">{name}</div>
                                        <div class="text-sm text-base-content/60 truncate mt-0.5">
                                            {desc}
                                        </div>
                                    </a>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
                <div class="border-t border-base-300 mt-1 pt-1">
                    <a
                        href=format!("/projects?search={}", encoded_query)
                        class="flex items-center gap-2 px-2 py-1 hover:bg-base-200 rounded text-sm text-base-content/70"
                    >
                        <mosaic_tiles::icon::Icon icon=IconSearch class="w-4 h-4" />
                        {format!("Search for \"{query}\" ({total_items} results)")}
                    </a>
                </div>
            }
            .to_html()
        }
    })
}

/// Strip Leptos hot-reload HTML comments that interfere with Datastar morphing.
///
/// In dev mode, Leptos wraps component output in `<!--hot-reload|...|open-->` and
/// `<!--hot-reload|...|close-->` comments. These confuse Datastar's morph algorithm
/// because the fragment's root node becomes a comment instead of the target element.
fn strip_hot_reload_comments(html: &str) -> Cow<'_, str> {
    // Hot-reload comments only exist in dev builds; no-op in release.
    if cfg!(not(debug_assertions)) || !html.contains("<!--hot-reload") {
        return Cow::Borrowed(html);
    }
    let mut result = html.to_string();
    while let Some(start) = result.find("<!--hot-reload") {
        if let Some(end) = result[start..].find("-->") {
            result.replace_range(start..start + end + 3, "");
        } else {
            break;
        }
    }
    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxumStatusCode};
    use axum::routing::get;
    use axum::Router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

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

    // --- Unit tests: strip_hot_reload_comments ---

    #[test]
    fn strip_no_comments_returns_borrowed() {
        let input = "<div>hello</div>";
        let result = strip_hot_reload_comments(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn strip_single_comment() {
        let input = "<!--hot-reload|abc|open--><div>content</div><!--hot-reload|abc|close-->";
        let result = strip_hot_reload_comments(input);
        assert_eq!(result.as_ref(), "<div>content</div>");
    }

    #[test]
    fn strip_nested_comments() {
        let input = "<!--hot-reload|outer|open--><!--hot-reload|inner|open--><p>text</p><!--hot-reload|inner|close--><!--hot-reload|outer|close-->";
        let result = strip_hot_reload_comments(input);
        assert_eq!(result.as_ref(), "<p>text</p>");
    }

    #[test]
    fn strip_preserves_regular_comments() {
        let input = "<!-- normal comment --><div>ok</div>";
        let result = strip_hot_reload_comments(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    // --- Unit tests: has_publications ---

    #[test]
    fn has_publications_with_abstract_only() {
        let mut project = minimal_test_project();
        let mut abstract_map = std::collections::HashMap::new();
        abstract_map.insert("en".to_string(), "Some abstract text".to_string());
        project.abstract_text = Some(abstract_map);
        assert!(has_publications(&project));
    }

    #[test]
    fn has_publications_with_pubs_only() {
        let mut project = minimal_test_project();
        project.publications = Some(vec![dpe_core::Publication {
            text: "A paper".to_string(),
            pid: None,
        }]);
        assert!(has_publications(&project));
    }

    #[test]
    fn has_publications_empty_project() {
        let project = minimal_test_project();
        assert!(!has_publications(&project));
    }

    #[test]
    fn has_publications_empty_pubs_vec() {
        let mut project = minimal_test_project();
        project.publications = Some(vec![]);
        assert!(!has_publications(&project));
    }

    // --- Unit tests: render_project_tabs ---

    #[test]
    fn render_overview_tab_contains_aria_roles() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", false);
        assert!(html.contains(r#"role="tablist""#), "missing tablist role");
        assert!(html.contains(r#"role="tab""#), "missing tab role");
        assert!(html.contains(r#"role="tabpanel""#), "missing tabpanel role");
    }

    #[test]
    fn render_overview_tab_marks_overview_as_selected() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", false);
        assert!(
            html.contains(r#"id="tab-overview" aria-selected="true""#),
            "overview tab should be selected, got: {}",
            html
        );
    }

    #[test]
    fn render_contributors_tab_marks_contributors_as_selected() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"contributors", false);
        assert!(
            html.contains(r#"id="tab-contributors" aria-selected="true""#),
            "contributors tab should be selected, got: {}",
            html
        );
        assert!(
            html.contains(r#"id="tab-overview" aria-selected="false""#),
            "overview tab should not be selected"
        );
    }

    #[test]
    fn render_tabs_includes_tabpanel_labelledby() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", false);
        assert!(
            html.contains(r#"aria-labelledby="tab-overview""#),
            "tabpanel should reference active tab"
        );
    }

    #[test]
    fn render_tabs_hides_publications_when_none() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", false);
        // Should only have overview and contributors tabs, not publications
        assert!(
            !html.contains("tab-publications"),
            "publications tab should not appear when has_publications_tab=false"
        );
    }

    #[test]
    fn render_tabs_shows_publications_when_available() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", true);
        assert!(
            html.contains("tab-publications"),
            "publications tab should appear when has_publications_tab=true"
        );
    }

    // --- Snapshot tests: render_project_tabs ---

    #[test]
    fn snapshot_overview_tab_html() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"overview", false);
        let html = strip_hot_reload_comments(&html);
        insta::with_settings!({
            filters => vec![
                // Scrub any Leptos-generated IDs or dynamic content
                (r"data-hk=\S+", "[DATA-HK]"),
            ],
        }, {
            insta::assert_snapshot!("overview_tab", html.as_ref());
        });
    }

    #[test]
    fn snapshot_contributors_tab_html() {
        let project = minimal_test_project();
        let html = render_project_tabs(project, vec![],"contributors", false);
        let html = strip_hot_reload_comments(&html);
        insta::with_settings!({
            filters => vec![
                (r"data-hk=\S+", "[DATA-HK]"),
            ],
        }, {
            insta::assert_snapshot!("contributors_tab", html.as_ref());
        });
    }

    #[test]
    fn snapshot_publications_tab_html() {
        let mut project = minimal_test_project();
        let mut abstract_map = std::collections::HashMap::new();
        abstract_map.insert("en".to_string(), "An example abstract".to_string());
        project.abstract_text = Some(abstract_map);
        project.publications = Some(vec![dpe_core::Publication {
            text: "Test Publication (2024)".to_string(),
            pid: Some(dpe_core::project::Pid {
                url: "https://example.org/pub".to_string(),
                text: None,
            }),
        }]);
        let html = render_project_tabs(project, vec![],"publications", true);
        let html = strip_hot_reload_comments(&html);
        insta::with_settings!({
            filters => vec![
                (r"data-hk=\S+", "[DATA-HK]"),
            ],
        }, {
            insta::assert_snapshot!("publications_tab", html.as_ref());
        });
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
        Router::new().route(
            "/projects/{id}/tab/{tab}",
            get(tab_fragment_handler),
        )
    }

    #[tokio::test]
    async fn handler_invalid_tab_returns_404() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/projects/0803/tab/nonexistent")
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
                    .uri("/projects/../etc/tab/overview")
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
                    .uri("/projects/9999/tab/overview")
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
                    .uri("/projects/0803/tab/overview")
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
            "expected SSE content type, got: {}",
            content_type
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        // SSE stream should contain datastar-patch-elements event
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
                    .uri("/projects/0803/tab/contributors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxumStatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        // Should contain replaceState script (delivered via Datastar ExecuteScript)
        assert!(
            body_str.contains("history.replaceState"),
            "SSE body should contain replaceState call, got: {}",
            &body_str[..body_str.len().min(2000)]
        );
        assert!(
            body_str.contains("/projects/0803?tab=contributors"),
            "replaceState URL should match request"
        );
    }

    #[tokio::test]
    async fn handler_sse_html_contains_aria_attributes() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        assert!(body_str.contains(r#"role="tablist""#), "missing tablist role in SSE HTML");
        assert!(body_str.contains(r#"role="tab""#), "missing tab role in SSE HTML");
        assert!(body_str.contains(r#"role="tabpanel""#), "missing tabpanel role in SSE HTML");
        assert!(
            body_str.contains(r#"aria-selected="true""#),
            "missing aria-selected on active tab"
        );
    }

    #[tokio::test]
    async fn snapshot_sse_overview_response() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/projects/0803/tab/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);

        insta::with_settings!({
            filters => vec![
                // Scrub Leptos-generated hydration keys
                (r"data-hk=\S+", "[DATA-HK]"),
                // Scrub dynamic data content that may change
                (r"https://ark\.dasch\.swiss/ark:/\S+", "[ARK-URL]"),
            ],
        }, {
            insta::assert_snapshot!("sse_overview_response", body_str.as_ref());
        });
    }

    // --- Test helpers ---

    /// Create a minimal Project for unit tests that don't need real data.
    fn minimal_test_project() -> Project {
        use dpe_web::domain::{AccessRights, AccessRightsType, Funding, ProjectStatus};

        Project {
            id: "test-001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/0001".to_string(),
            name: "Test Project".to_string(),
            shortcode: "0001".to_string(),
            official_name: "Test Project Official".to_string(),
            status: ProjectStatus::Ongoing,
            short_description: "A test project".to_string(),
            description: std::collections::HashMap::from([(
                "en".to_string(),
                "A test project for unit tests".to_string(),
            )]),
            start_date: "2020-01-01".to_string(),
            end_date: "2025-12-31".to_string(),
            url: None,
            secondary_url: None,
            how_to_cite: "Test Project (2024)".to_string(),
            access_rights: AccessRights {
                access_rights: AccessRightsType::FullOpenAccess,
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
            funding: Funding::Grants(vec![]),
            alternative_names: None,
            documentation_material: None,
            provenance: None,
            additional_material: None,
        }
    }
}
