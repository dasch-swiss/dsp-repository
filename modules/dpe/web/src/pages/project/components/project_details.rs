use dpe_core::{Project, ResolvedContributor};
use maud::{html, Markup};

use crate::pages::project::components::breadcrumb::breadcrumb;
use crate::pages::project::components::project_details_tabs::{has_publications, project_tabs};
use crate::pages::project::components::project_header::project_header;
use crate::pages::project::components::project_sidebar::project_sidebar;

/// The full project-detail view: breadcrumb, hero header, then the tabs (in a
/// card) alongside the sidebar. `active_tab` selects which tab panel renders.
pub fn project_details(proj: &Project, contributors: &[ResolvedContributor], active_tab: &str) -> Markup {
    html! {
        div class="space-y-6" {
            (breadcrumb(&proj.name))

            (project_header(proj))

            div class="flex flex-col lg:flex-row gap-6 lg:items-start" {
                div class="card card-bordered overflow-visible p-4 space-y-4 text-gray-700 flex-1 pt-4"
                { (project_tabs(proj, contributors, active_tab, has_publications(proj))) }
                (project_sidebar(proj))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn assembles_breadcrumb_header_tabs_and_sidebar() {
        let out = project_details(&sample_project(), &[], "overview").into_string();
        assert!(out.contains("breadcrumb"), "breadcrumb: {out}");
        assert!(out.contains(r#"id="project-tabs""#), "tabs morph root: {out}");
        assert!(out.contains("Cite this Project"), "sidebar: {out}");
        // The tabs card wrapper that the SSE fragment morphs within.
        assert!(
            out.contains(r#"class="card card-bordered overflow-visible p-4 space-y-4 text-gray-700 flex-1 pt-4""#),
            "{out}"
        );
    }
}
