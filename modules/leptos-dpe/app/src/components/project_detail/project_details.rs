use leptos::prelude::*;

use crate::components::project_detail::breadcrumb::Breadcrumb;
use crate::components::project_detail::cite_project_section::CiteProjectSection;
use crate::components::project_detail::project_header::ProjectHeader;
use crate::components::project_details_tabs::ProjectDetailsTabs;
use crate::domain::Project;

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

              <ProjectDetailsTabs proj=proj.clone() attributions=proj.attributions.clone() />

            <CiteProjectSection proj=proj />
              </div>
          }
}
