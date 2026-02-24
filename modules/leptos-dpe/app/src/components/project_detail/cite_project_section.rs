use leptos::prelude::*;

use crate::components::project_detail::access_rights_section::AccessRightsSection;
use crate::components::project_detail::citation::Citation;
use crate::components::project_detail::funding_section::FundingSection;
use crate::components::project_detail::permalink::Permalink;
use crate::components::*;
use crate::domain::Project;

#[component]
pub fn CiteProjectSection(proj: Project) -> impl IntoView {
    view! {
        <div class="border border-gray-200 rounded-lg p-6 space-y-4 text-sm">
            <h2 class="text-lg font-semibold">"Cite this Project"</h2>

        <div>
            <Permalink permalink=proj.pid.clone() />
        </div>
        <div>
            <Citation citation=proj.how_to_cite.clone() />
        </div>
            <div class="border-t border-gray-200"></div>

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

            <div class="border-t border-gray-200"></div>

            <h3 class="text-base font-semibold">"Project Timeline"</h3>
        <div>
            <div class="font-semibold">"Period"</div>
            <div>"TODO"</div>
        </div>
        <div>
            <div class="font-semibold mt-2">"Status"</div>
            <div>"Ongoing"</div>
        </div>

            <div class="border-t border-gray-200"></div>

            <h3 class="text-base font-semibold">"Funding"</h3>

            <FundingSection funding=proj.funding.clone() />
        </div>
    }
}
