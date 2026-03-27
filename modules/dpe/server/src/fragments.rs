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

const VALID_TABS: &[&str] = &["overview", "publications", "contributors"];

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
pub async fn tab_fragment_handler(
    Path(params): Path<TabParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, impl IntoResponse> {
    let TabParams { id, tab } = params;

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
        get_contributors(project.attributions.clone())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Render the full tab component (tab bar + panel) to HTML
    let html = render_project_tabs(&project, &contributors, &tab, has_publications_tab);

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

    let stream = stream::iter(vec![
        Ok::<_, Infallible>(patch.into()),
        Ok(url_update.into()),
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
    project: &Project,
    contributors: &[ResolvedContributor],
    active_tab: &str,
    has_publications_tab: bool,
) -> String {
    let project = project.clone();
    let contributors = contributors.to_vec();
    let active_tab = active_tab.to_string();
    let shortcode = project.shortcode.clone();

    let owner = Owner::new();
    owner.with(|| {
        view! {
            <div
                id="project-tabs"
                class="tabs"
                style="border-width: 0"
                role="tablist"
                aria-label="Project details"
                data-on:datastar-fetch="(evt.detail.type === 'error' || evt.detail.type === 'retries-failed') && evt.detail.el.closest('#project-tabs') && (window.location.href = evt.detail.el.getAttribute('href'))"
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
pub async fn search_fragment_handler(
    ReadSignals(signals): ReadSignals<SearchSignals>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let search = signals.search.trim().to_string();

    let results = if search.is_empty() {
        Page { items: vec![], nr_pages: 0, total_items: 0 }
    } else {
        list_projects(
            None,                     // status
            None,                     // ongoing
            Some(search.clone()),     // search
            None,                     // type_of_data
            Some(5),                  // page_size
            None,                     // page
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
                <ul>
                    {items
                        .iter()
                        .map(|p| {
                            let shortcode = p.shortcode.clone();
                            let name = p.name.clone();
                            let desc = p.short_description.clone();
                            view! {
                                <li>
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
    if !html.contains("<!--hot-reload") {
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
    use leptos::prelude::*;

    /// Phase 0 Spike: Verify that Leptos `view!` macro output can be rendered
    /// to an HTML string from outside the Leptos routing context.
    #[test]
    fn spike_leptos_view_renders_to_html_string() {
        let html = view! { <div class="test">"Hello from fragment"</div> }.to_html();
        assert!(html.contains("Hello from fragment"));
        assert!(html.contains(r#"class="test""#));
    }

    #[test]
    fn spike_leptos_component_renders_with_owner() {
        #[component]
        fn TestTabPanel(active_tab: String, project_name: String) -> impl IntoView {
            let is_overview = active_tab == "overview";
            view! {
                <div id="project-tabs">
                    <div class="tabs">
                        <a
                            href="?tab=overview"
                            class=if is_overview { "tab-active" } else { "tab" }
                        >
                            "Overview"
                        </a>
                        <a
                            href="?tab=publications"
                            class=if !is_overview { "tab-active" } else { "tab" }
                        >
                            "Publications"
                        </a>
                    </div>
                    <div id="tab-panel">
                        {if is_overview {
                            view! { <p>{format!("Overview of {}", project_name)}</p> }.into_any()
                        } else {
                            view! { <p>"Publications list"</p> }.into_any()
                        }}
                    </div>
                </div>
            }
        }

        let owner = Owner::new();
        let html = owner.with(|| {
            view! { <TestTabPanel active_tab="overview".to_string() project_name="Test Project".to_string() /> }
            .to_html()
        });

        assert!(html.contains("project-tabs"));
        assert!(html.contains("tab-panel"));
        assert!(html.contains("Overview of Test Project"));
        assert!(html.contains(r#"class="tab-active""#));
    }

    #[test]
    fn spike_leptos_component_renders_different_tab() {
        #[component]
        fn TabContent(tab: String) -> impl IntoView {
            match tab.as_str() {
                "publications" => view! { <div>"Publications content"</div> }.into_any(),
                "contributors" => view! { <div>"Contributors content"</div> }.into_any(),
                _ => view! { <div>"Overview content"</div> }.into_any(),
            }
        }

        let owner = Owner::new();
        let html = owner.with(|| {
            view! { <TabContent tab="overview".to_string() /> }.to_html()
        });
        assert!(html.contains("Overview content"));

        let owner2 = Owner::new();
        let html = owner2.with(|| {
            view! { <TabContent tab="publications".to_string() /> }.to_html()
        });
        assert!(html.contains("Publications content"));
    }
}
