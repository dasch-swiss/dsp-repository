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
                shortcode=proj.shortcode.clone()
                name=proj.name.clone()
                description=proj.description.get("en").cloned().unwrap_or_default()
            />

        <ProjectDetailsTabs proj=proj.clone() attributions=proj.attributions.clone() />


        <div class="border border-gray-200">

                 <HowToCite citation=proj.how_to_cite.clone() />

            <AccessRightsSection access_rights=proj.access_rights.clone() />

                    {(!proj.legal_info.is_empty())
                .then(|| {
                    view! {
                        <div
                            id="legal-information"
                            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
                        >
                            <h3 class="text-xl font-bold mb-3">
                                "Legal Information"
                            </h3>
                            <LegalInfo legal_info=proj.legal_info.clone() />
                        </div>
                    }
                })}





            <FundingSection funding=proj.funding.clone() />

        </div>




        </div>
    }
}
