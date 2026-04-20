mod access_rights_section;
mod citation;
mod funding_section;
mod legal_info;
mod permalink;

use access_rights_section::AccessRightsSection;
use citation::Citation;
use funding_section::FundingSection;
use legal_info::{ContactSection, LegalInfo};
use leptos::prelude::*;
use permalink::Permalink;

use crate::components::{should_render_value, PlaceholderValue};
use crate::domain::{Project, ProjectStatus};

#[component]
pub fn ProjectSidebar(proj: Project) -> impl IntoView {
    view! {
        <div class="dpe-card dpe-small lg:w-96">
            <div class="space-y-4">
                <h2 class="dpe-title">"Cite this Project"</h2>

                <div>
                    <Permalink permalink=proj.pid.clone() />
                </div>
                <div>
                    <Citation citation=proj.how_to_cite.clone() />
                </div>
                <div class="dpe-divider"></div>

                <div>
                    <h3 class="dpe-title">"Data Access"</h3>
                    <AccessRightsSection access_rights=proj.access_rights.clone() />
                </div>

                {(!proj.legal_info.is_empty())
                    .then(|| {
                        view! { <LegalInfo legal_info=proj.legal_info.clone() /> }
                    })}

                {proj
                    .contact_point
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .map(|ids| {
                        view! {
                            <div>
                                <ContactSection ids=ids.clone() />
                            </div>
                        }
                    })}

                <div class="dpe-divider"></div>

                <h3 class="dpe-title">"Project Timeline"</h3>
                <div>
                    <div class="dpe-subtitle">"Period"</div>
                    <div>
                        {if dpe_core::is_placeholder(&proj.end_date) {
                            if should_render_value(&proj.end_date) {
                                // DEV/STAGE: show start date with placeholder in red
                                view! {
                                    <span>
                                        {proj.start_date.clone()} " – "
                                        <PlaceholderValue value=proj.end_date.clone() />
                                    </span>
                                }
                                    .into_any()
                            } else {
                                // Production: show only start date
                                view! { <span>{proj.start_date.clone()}</span> }
                                    .into_any()
                            }
                        } else {
                            view! {
                                <span>{format!("{} – {}", proj.start_date, proj.end_date)}</span>
                            }
                                .into_any()
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

                <div class="dpe-divider"></div>

                <h3 class="dpe-title">"Funding"</h3>

                <FundingSection funding=proj.funding.clone() />

                {proj
                    .data_management_plan
                    .map(|dmp| {
                        view! {
                            <div>
                                <div class="dpe-subtitle">"Data Management Plan"</div>
                                {if dmp == "not accessible" {
                                    view! { <div>"Not accessible"</div> }.into_any()
                                } else {
                                    view! {
                                        <a
                                            href=dmp
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="text-primary"
                                        >
                                            "Available"
                                        </a>
                                    }
                                        .into_any()
                                }}
                            </div>
                        }
                    })}
            </div>
        </div>
    }
}
