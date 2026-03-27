/// Fragment handlers for Datastar SSE-driven content updates.
///
/// Each handler renders a Leptos component to an HTML string and returns it
/// as an SSE stream with Datastar events (PatchElements + ExecuteScript).
use std::convert::Infallible;

use app::domain::{get_contributors, get_project, lang_value, Project, ResolvedContributor};
use app::pages::project::components::project_details_tabs::{
    AttributionsSectionComponent, DatasetOverviewSectionComponent, PublicationTabComponent,
};
use axum::extract::Path;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use datastar::prelude::{ExecuteScript, PatchElements};
use futures::stream::{self, Stream};
use axum::http::StatusCode;
use leptos::prelude::*;
use mosaic_tiles::icon::{Document, Info, People};
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
                <FragmentTabLink
                    value="overview"
                    active_tab=active_tab.clone()
                    icon=Info
                    label="Overview"
                    shortcode=shortcode.clone()
                />
                {has_publications_tab.then(|| {
                    view! {
                        <FragmentTabLink
                            value="publications"
                            active_tab=active_tab.clone()
                            icon=Document
                            label="Publications"
                            shortcode=shortcode.clone()
                        />
                    }
                })}
                <FragmentTabLink
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
                            let abstract_en = project.abstract_text.as_ref().and_then(|m| lang_value(m).cloned());
                            let publications = project.publications.clone();
                            view! {
                                <PublicationTabComponent
                                    abstract_en=abstract_en
                                    publications=publications
                                />
                            }.into_any()
                        }
                        "contributors" => {
                            view! {
                                <AttributionsSectionComponent
                                    contributors=contributors
                                />
                            }.into_any()
                        }
                        _ => {
                            view! {
                                <DatasetOverviewSectionComponent
                                    proj=project
                                />
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        }
        .to_html()
    })
}

/// Tab link component for fragment rendering (with Datastar attributes).
///
/// Unlike the original TabLink in the app crate, this version includes
/// Datastar `data-on:click__prevent` for SSE-driven tab switching and
/// the project shortcode for building the fragment URL.
#[component]
fn FragmentTabLink(
    value: &'static str,
    active_tab: String,
    icon: mosaic_tiles::icon::IconData,
    label: &'static str,
    shortcode: String,
) -> impl IntoView {
    let is_active = active_tab == value;
    let class = if is_active {
        "tab-label !text-primary-600 !border-primary-600"
    } else {
        "tab-label"
    };

    view! {
        <a
            href=format!("/projects/{}?tab={}", shortcode, value)
            role="tab"
            id=format!("tab-{value}")
            aria-selected=is_active.to_string()
            aria-controls="tab-panel"
            tabindex=if is_active { "0" } else { "-1" }
            data-on:click__prevent=format!("@get('/projects/{}/tab/{}', {{retry: 'never'}})", shortcode, value)
            class=class
        >
            <svg
                class="tab-icon"
                xmlns="http://www.w3.org/2000/svg"
                viewBox=icon.view_box
                fill="currentColor"
                inner_html=icon.data
            ></svg>
            <span>{label}</span>
        </a>
    }
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
            view! {
                <TestTabPanel active_tab="overview".to_string() project_name="Test Project".to_string() />
            }
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
