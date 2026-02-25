use leptos::prelude::*;

use super::legal_info::LegalInfo;
use crate::domain::Project;
use crate::pages::project::components::access_rights_section::AccessRightsSection;
use crate::pages::project::components::citation::Citation;
use crate::pages::project::components::funding_section::FundingSection;
use crate::pages::project::components::permalink::Permalink;

#[component]
pub fn CiteProjectSection(proj: Project) -> impl IntoView {
    view! {
        <div class="border border-gray-200 rounded-lg p-4 space-y-4 text-sm lg:w-96">
            <h2 class="dpe-title">"Cite this Project"</h2>

        <div>
            <Permalink permalink=proj.pid.clone() />
        </div>
        <div>
            <Citation citation=proj.how_to_cite.clone() />
        </div>
            <div class="border-t border-gray-200"></div>

            <h3 class="dpe-title">"Data Access"</h3>
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

            <div class="border-t border-gray-200"></div>

            <h3 class="dpe-title">"Project Timeline"</h3>
        <div>
            <div class="dpe-subtitle">"Period"</div>
            <div>"TODO"</div>
        </div>
        <div>
            <div class="dpe-subtitle">"Status"</div>
            <div>"Ongoing"</div>
        </div>

            <div class="border-t border-gray-200"></div>

            <h3 class="dpe-title">"Funding"</h3>

            <FundingSection funding=proj.funding.clone() />
        </div>
    }
}
