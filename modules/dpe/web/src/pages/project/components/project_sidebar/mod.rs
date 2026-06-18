mod access_rights_section;
mod citation;
mod funding_section;
mod legal_info;
mod permalink;

use access_rights_section::access_rights_section;
use citation::citation;
use dpe_core::project::ProjectStatus;
use dpe_core::Project;
use funding_section::funding_section;
use legal_info::{contact_section, legal_info};
use maud::{html, Markup};
use permalink::permalink;

use crate::components::{placeholder_value, should_render_value};

/// The project-detail sidebar: citation block, data access, legal info, contact,
/// timeline, funding, and the data-management plan link.
pub fn project_sidebar(proj: &Project) -> Markup {
    html! {
        div class="dpe-card dpe-small lg:w-96" {
            div class="space-y-4" {
                h2 class="dpe-title" { "Cite this Project" }

                div { (permalink(&proj.pid)) }
                div { (citation(&proj.how_to_cite)) }
                div class="dpe-divider" {}

                div {
                    h3 class="dpe-title" { "Data Access" }
                    (access_rights_section(&proj.access_rights))
                }

                @if !proj.legal_info.is_empty() {
                    (legal_info(&proj.legal_info))
                }

                @if let Some(ids) = proj.contact_point.as_ref().filter(|v| !v.is_empty()) {
                    div { (contact_section(ids)) }
                }

                div class="dpe-divider" {}

                h3 class="dpe-title" { "Project Timeline" }
                div {
                    div class="dpe-subtitle" { "Period" }
                    div {
                        @if dpe_core::is_placeholder(&proj.end_date) {
                            @if should_render_value(&proj.end_date) {
                                span { (proj.start_date) " – " (placeholder_value(&proj.end_date)) }
                            } @else {
                                span { (proj.start_date) }
                            }
                        } @else {
                            span { (format!("{} – {}", proj.start_date, proj.end_date)) }
                        }
                    }
                }
                div {
                    div class="dpe-subtitle" { "Status" }
                    div {
                        (match proj.status {
                            ProjectStatus::Ongoing => "Ongoing",
                            ProjectStatus::Finished => "Finished",
                        })
                    }
                }

                div class="dpe-divider" {}

                h3 class="dpe-title" { "Funding" }
                (funding_section(&proj.funding))

                @if let Some(dmp) = &proj.data_management_plan {
                    div {
                        div class="dpe-subtitle" { "Data Management Plan" }
                        @if dmp == "not accessible" {
                            div { "Not accessible" }
                        } @else {
                            a href=(dmp) target="_blank" rel="noopener noreferrer" class="text-primary" { "Available" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn renders_cite_access_timeline_and_funding() {
        let out = project_sidebar(&sample_project()).into_string();
        assert!(out.contains("Cite this Project"), "{out}");
        assert!(out.contains("Permalink"), "{out}");
        assert!(out.contains("Citation"), "{out}");
        assert!(out.contains("Data Access"), "{out}");
        assert!(out.contains("Full Open Access"), "{out}");
        assert!(out.contains("Project Timeline"), "{out}");
        assert!(out.contains("Ongoing"), "status: {out}");
        assert!(out.contains("Funding"), "{out}");
    }

    #[test]
    fn renders_period_range_for_real_dates() {
        let out = project_sidebar(&sample_project()).into_string();
        assert!(out.contains("2020-01-01 – 2024-12-31"), "period range: {out}");
    }
}
