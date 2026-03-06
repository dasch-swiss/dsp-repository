use leptos::prelude::*;
use mosaic_tiles::card::{Card, CardBody, CardVariant};

use super::legal_info::LegalInfo;
use crate::domain::{Project, ProjectStatus};
use crate::pages::project::components::access_rights_section::AccessRightsSection;
use crate::pages::project::components::citation::Citation;
use crate::pages::project::components::funding_section::FundingSection;
use crate::pages::project::components::permalink::Permalink;

#[component]
pub fn CiteProjectSection(proj: Project) -> impl IntoView {
    view! {
        <Card variant=CardVariant::Bordered class="lg:w-96">
            <CardBody>
                <div class="space-y-4 text-sm">
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
                                <div id="legal-information" class="rounded-lg scroll-mt-52">
                                    <LegalInfo legal_info=proj.legal_info.clone() />
                                </div>
                            }
                        })}

                    <div class="border-t border-gray-200"></div>

                    <h3 class="dpe-title">"Project Timeline"</h3>
                    <div>
                        <div class="dpe-subtitle">"Period"</div>
                        <div>
                            {if proj.end_date == "MISSING" {
                                proj.start_date.clone()
                            } else {
                                format!("{} – {}", proj.start_date, proj.end_date)
                            }}
                        </div>
                    </div>
                    <div>
                        <div class="dpe-subtitle">"Status"</div>
                        <div>
                            {match proj.status {
                                ProjectStatus::Ongoing => "Ongoing",
                                ProjectStatus::Finished => "Finished",
                            }}
                        </div>
                    </div>

                    <div class="border-t border-gray-200"></div>

                    <h3 class="dpe-title">"Funding"</h3>

                    <FundingSection funding=proj.funding.clone() />
                </div>
            </CardBody>
        </Card>
    }
}
