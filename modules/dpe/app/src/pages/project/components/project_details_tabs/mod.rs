mod attributions_section;
mod dataset_overview_section;
mod publication_tab;

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use mosaic_tiles::icon::IconData;
use mosaic_tiles::icon::{Document, Info, People};

use crate::domain::{lang_value, Project, ResolvedContributor};
use attributions_section::AttributionsSection;
use dataset_overview_section::DatasetOverviewSection;
use publication_tab::PublicationTab;

// Re-export sub-components for use in fragment handlers
pub use attributions_section::AttributionsSection as AttributionsSectionComponent;
pub use dataset_overview_section::DatasetOverviewSection as DatasetOverviewSectionComponent;
pub use publication_tab::PublicationTab as PublicationTabComponent;


#[component]
pub fn ProjectDetailsTabs(
    proj: Project,
    contributors: Vec<ResolvedContributor>,
) -> impl IntoView {
    let shortcode = proj.shortcode.clone();
    let abstract_en = proj.abstract_text.as_ref().and_then(|m| lang_value(m).cloned());
    let publications = proj.publications.clone();
    let has_publications_tab =
        abstract_en.is_some() || publications.as_ref().map(|p| !p.is_empty()).unwrap_or(false);

    let query = use_query_map();
    let active_tab = query
        .read()
        .get("tab")
        .unwrap_or_else(|| "overview".to_string());

    view! {
        <div class="dpe-card flex-1 pt-4">
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
                    aria-labelledby=format!("tab-{}", active_tab)
                >
                    {match active_tab.as_str() {
                        "publications" if has_publications_tab => {
                            view! {
                                <PublicationTab abstract_en=abstract_en publications=publications />
                            }
                                .into_any()
                        }
                        "contributors" => {
                            view! { <AttributionsSection contributors=contributors /> }.into_any()
                        }
                        _ => view! { <DatasetOverviewSection proj=proj /> }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn TabLink(
    value: &'static str,
    active_tab: String,
    icon: IconData,
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
            rel="external"
            role="tab"
            id=format!("tab-{value}")
            aria-selected=is_active.to_string()
            aria-controls="tab-panel"
            tabindex=if is_active { "0" } else { "-1" }
            data-on:click__prevent=format!(
                "@get('/projects/{}/tab/{}', {{retry: 'never'}})",
                shortcode,
                value,
            )
            data-indicator:_tab_loading
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
