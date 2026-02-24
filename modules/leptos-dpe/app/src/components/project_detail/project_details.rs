use leptos::prelude::*;

use crate::components::project_detail::access_rights_section::AccessRightsSection;
use crate::components::project_detail::breadcrumb::Breadcrumb;
use crate::components::project_detail::funding_section::FundingSection;
use crate::components::project_detail::project_header::ProjectHeader;
use crate::components::project_details_tabs::ProjectDetailsTabs;
use crate::components::*;
use crate::domain::Project;

#[component]
pub fn ProjectDetails(proj: Project) -> impl IntoView {
    view! {
              <div class="space-y-6">
                  <Breadcrumb project_name=proj.name.clone() />

                  <ProjectHeader
                      name=proj.name.clone()
                      description=proj.description.get("en").cloned().unwrap_or_default()
                      url=proj.url.clone()
                      secondary_url=proj.secondary_url.clone()
                  />

              <ProjectDetailsTabs proj=proj.clone() attributions=proj.attributions.clone() />

              <div class="border border-gray-200 rounded-lg p-6 space-y-6 text-sm">
                  <h2 class="text-lg font-semibold">"Cite this Project"</h2>

                  <HowToCite
                      permalink=proj.pid.clone()
                      citation=proj.how_to_cite.clone()
                  />
        <div class="border-t border-gray-200 mt-4"></div>

                  <h3 class="text-base font-semibold">"Data Access"</h3>
                  <AccessRightsSection access_rights=proj.access_rights.clone() />



                  {(!proj.legal_info.is_empty())
                      .then(|| {
                          view! {
                              <div
                                  id="legal-information"
                                  class="bg-base-100 rounded-lg scroll-mt-52"
                              >
                                  <LegalInfo legal_info=proj.legal_info.clone() />
                              </div>
                          }
                      })}

                <div class="border-t border-gray-200 mt-4 pt-4"></div>

            <h3 class="text-base font-semibold">"Project Timeline"</h3>
    <div class="font-semibold">"Period"</div>
    <div>"2006-10-01 - 2024-12-31"</div>
    <div class="font-semibold mt-2">"Status"</div>
    <div>"Ongoing"</div>
        <div class="border-t border-gray-200"></div>

                  <FundingSection funding=proj.funding.clone() />
              </div>
              </div>
          }
}
