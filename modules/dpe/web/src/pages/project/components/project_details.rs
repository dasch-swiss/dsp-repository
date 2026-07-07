use leptos::prelude::*;

use crate::domain::{lang_value, Project, ResolvedContributor};
use crate::pages::project::components::breadcrumb::Breadcrumb;
use crate::pages::project::components::project_details_tabs::ProjectDetailsTabs;
use crate::pages::project::components::project_header::ProjectHeader;
use crate::pages::project::components::project_sidebar::ProjectSidebar;

#[component]
pub fn ProjectDetails(proj: Project, contributors: Vec<ResolvedContributor>) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <Breadcrumb project_name=proj.name.clone() />

            <ProjectHeader
                name=proj.name.clone()
                shortcode=proj.shortcode.clone()
                description=lang_value(&proj.description).cloned().unwrap_or_default()
                alternative_names=proj
                    .alternative_names
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|m| lang_value(m).cloned())
                    .collect()
                url=proj.url.clone()
                secondary_url=proj.secondary_url.clone()
            />

            <div class="flex flex-col lg:flex-row gap-6 lg:items-start">
                <ProjectDetailsTabs proj=proj.clone() contributors=contributors />
                <ProjectSidebar proj=proj />
            </div>
        </div>
    }
}
