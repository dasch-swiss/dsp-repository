mod attributions_section;
mod dataset_overview_section;
mod publication_tab;

use attributions_section::attributions_section;
use dataset_overview_section::dataset_overview_section;
use dpe_core::{Project, ResolvedContributor};
use maud::{html, Markup, PreEscaped};
use mosaic_tiles::icon::{Document, IconData, Info, People};
use publication_tab::publication_tab;

/// Whether the project has content for a publications tab (at least one
/// publication). The abstract lives in the Overview tab, so it does not count.
pub fn has_publications(project: &Project) -> bool {
    project.publications.as_ref().map(|p| !p.is_empty()).unwrap_or(false)
}

/// Render the `#project-tabs` morph-root element: the tablist plus the active
/// tab's panel. This single function is the source of truth for the tab UI,
/// used by both the full-page project view (wrapped in a Mosaic card by
/// `project_details`) and the `/dpe/projects/{id}/tab/{tab}` SSE fragment, which
/// outer-morphs `#project-tabs` with this output. Keeping one renderer prevents
/// the two paths from drifting.
///
/// NOTE (DEV-6642): this is a second, DPE-specific tab implementation, separate
/// from the generic `mosaic_tiles::tabs` tile (a CSS-only radio group). This one
/// is a Datastar/SSE, URL-addressable tablist with full ARIA + keyboard nav.
/// The two should probably converge onto one ARIA-complete Mosaic tab component;
/// see the note in `modules/mosaic/tiles/src/components/tabs/mod.rs`.
pub fn project_tabs(
    proj: &Project,
    contributors: &[ResolvedContributor],
    active_tab: &str,
    has_publications_tab: bool,
) -> Markup {
    let shortcode = &proj.shortcode;
    html! {
        div id="project-tabs"
            data-on:datastar-fetch="(evt.detail.type === 'error' || evt.detail.type === 'retries-failed') && evt.detail.el.closest('#project-tabs') && (window.location.href = evt.detail.el.getAttribute('href'))"
        {
            div class="tabs"
                role="tablist"
                aria-label="Project details"
                aria-orientation="horizontal"
                data-on:keydown="const tabs=[...evt.currentTarget.querySelectorAll('[role=tab]')];const idx=tabs.indexOf(evt.target);if(idx<0)return;let next;if(evt.key==='ArrowRight')next=tabs[(idx+1)%tabs.length];else if(evt.key==='ArrowLeft')next=tabs[(idx-1+tabs.length)%tabs.length];else if(evt.key==='Home')next=tabs[0];else if(evt.key==='End')next=tabs[tabs.length-1];else if(evt.key===' '){evt.preventDefault();evt.target.click();return}else return;evt.preventDefault();next.focus()"
            {
                (tab_link("overview", active_tab, Info, "Overview", shortcode))
                @if has_publications_tab {
                    ({
                        tab_link(
                            "publications",
                            active_tab,
                            Document,
                            "Publications",
                            shortcode,
                        )
                    })
                }
                ({
                    tab_link(
                        "contributors",
                        active_tab,
                        People,
                        "Contributors",
                        shortcode,
                    )
                })
            }

            div id="tab-panel"
                class="tab-panel"
                style="display: block"
                role="tabpanel"
                aria-labelledby=(format!("tab-{active_tab}"))
            {
                @if active_tab == "publications" && has_publications_tab {
                    (publication_tab(proj.publications.as_deref()))
                } @else if active_tab == "contributors" { (attributions_section(contributors)) } @else {
                    (dataset_overview_section(proj))
                }
            }
        }
    }
}

/// A single tab: an `<a role="tab">` that both links to the full-page tab URL
/// (no-JS fallback) and, via Datastar, fetches the tab fragment over SSE.
fn tab_link(value: &str, active_tab: &str, icon: IconData, label: &str, shortcode: &str) -> Markup {
    let is_active = active_tab == value;
    let class = if is_active {
        "tab-label !text-primary-600 !border-primary-600"
    } else {
        "tab-label"
    };
    html! {
        a   href=(format!("/dpe/projects/{shortcode}?tab={value}"))
            rel="external"
            role="tab"
            id=(format!("tab-{value}"))
            aria-selected=(is_active.to_string())
            aria-controls="tab-panel"
            tabindex=(if is_active { "0" } else { "-1" })
            data-on:click__prevent=({
                format!(
                    "@get('/dpe/projects/{shortcode}/tab/{value}', {{retry: 'never'}})",
                )
            })
            data-indicator:_tab_loading
            class=(class)
        {
            svg class="tab-icon"
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                viewBox=[icon.view_box]
                fill="currentColor"
            { (PreEscaped(icon.data)) }
            span { (label) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn renders_morph_root_with_aria_roles() {
        let out = project_tabs(&sample_project(), &[], "overview", true).into_string();
        assert!(out.contains(r#"id="project-tabs""#), "morph root: {out}");
        assert!(out.contains(r#"role="tablist""#), "{out}");
        assert!(out.contains(r#"role="tab""#), "{out}");
        assert!(out.contains(r#"role="tabpanel""#), "{out}");
    }

    #[test]
    fn marks_active_tab_selected() {
        let out = project_tabs(&sample_project(), &[], "overview", false).into_string();
        assert!(out.contains(r#"id="tab-overview" aria-selected="true""#), "{out}");
        assert!(out.contains(r#"id="tab-contributors" aria-selected="false""#), "{out}");
    }

    #[test]
    fn publications_tab_shown_only_when_available() {
        let with = project_tabs(&sample_project(), &[], "overview", true).into_string();
        assert!(with.contains("tab-publications"), "{with}");
        let without = project_tabs(&sample_project(), &[], "overview", false).into_string();
        assert!(!without.contains("tab-publications"), "{without}");
    }

    #[test]
    fn tab_link_carries_both_no_js_href_and_datastar_get() {
        let out = project_tabs(&sample_project(), &[], "overview", false).into_string();
        assert!(
            out.contains(r#"href="/dpe/projects/0ABC?tab=contributors""#),
            "no-JS href: {out}"
        );
        assert!(
            out.contains("@get('/dpe/projects/0ABC/tab/contributors', {retry: 'never'})"),
            "datastar get: {out}"
        );
        assert!(out.contains("data-indicator:_tab_loading"), "{out}");
    }

    #[test]
    fn has_publications_requires_publications() {
        assert!(has_publications(&sample_project()));
        let bare = Project { abstract_text: None, publications: None, ..sample_project() };
        assert!(!has_publications(&bare));
        // The abstract now lives in the Overview tab, so an abstract alone must
        // not surface the Publications tab.
        let abstract_only = Project { publications: None, ..sample_project() };
        assert!(
            !has_publications(&abstract_only),
            "abstract without publications no longer counts"
        );
    }
}
