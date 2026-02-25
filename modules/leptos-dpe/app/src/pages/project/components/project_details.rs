use leptos::prelude::*;

use crate::domain::Project;
use crate::pages::project::components::breadcrumb::Breadcrumb;
use crate::pages::project::components::cite_project_section::CiteProjectSection;
use crate::pages::project::components::project_details_tabs::ProjectDetailsTabs;
use crate::pages::project::components::project_header::ProjectHeader;

#[component]
pub fn ProjectDetails(proj: Project) -> impl IntoView {
    view! {
          <div class="space-y-6">
              <Breadcrumb project_name=proj.name.clone() />

              <ProjectHeader
                  name=proj.name.clone()
                  description=proj.description.get("en").cloned().unwrap_or_default()
                  alternative_names=proj.alternative_names.as_deref().unwrap_or_default().iter().filter_map(|m| m.get("en").cloned()).collect()
                  url=proj.url.clone()
                  secondary_url=proj.secondary_url.clone()
              />

    <div class="flex flex-col lg:flex-row gap-6 lg:items-start">
          <ProjectDetailsTabs proj=proj.clone() attributions=proj.attributions.clone() />
        <CiteProjectSection proj=proj />
    </div>
          </div>
      }
}
