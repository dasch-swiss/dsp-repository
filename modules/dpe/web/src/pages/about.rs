use maud::{html, Markup};
use mosaic_tiles::icon::{icon, IconArrowLeft};
use mosaic_tiles::link::link;

pub fn about_page() -> Markup {
    html! {
        a class="inline-flex items-center gap-2 text-sm text-primary mb-6" href="/" {
            (icon(IconArrowLeft, "w-3 h-3"))
            "Back to Projects"
        }

        div class="bg-white border border-gray-200 rounded-lg p-8" {
            h1 class="font-display text-3xl font-bold text-gray-900 mb-6" { "Help & Documentation" }
            div class="space-y-6" {
                section {
                    h2 class="font-display text-xl font-semibold text-gray-900 mb-3" {
                        "About the Metadata Browser"
                    }
                    p class="text-gray-700 leading-relaxed" {
                        "The DaSCH Metadata Browser provides access to comprehensive metadata about humanities research projects archived by DaSCH (Data and Service Center for the Humanities). Browse projects, collections, and clusters to discover research data across various disciplines, time periods, and institutions."
                    }
                }
                section {
                    h2 class="font-display text-xl font-semibold text-gray-900 mb-3" {
                        "Searching & Filtering"
                    }
                    p class="text-gray-700 leading-relaxed mb-3" {
                        "Use the search bar to find projects by name, description, or keywords. Combine search with filters to narrow down results:"
                    }
                    ul class="list-disc list-inside space-y-2 text-gray-700 ml-4" {
                        li { "Filter by discipline, time period, or geographic region" }
                        li { "Filter by access rights to find open access projects" }
                        li { "Filter by project status (finished or ongoing)" }
                        li { "Multiple filters within a category use OR logic" }
                        li { "Filters across categories use AND logic" }
                    }
                }
                section {
                    h2 class="font-display text-xl font-semibold text-gray-900 mb-3" {
                        "Understanding Access Rights"
                    }
                    p class="text-gray-700 leading-relaxed mb-3" {
                        "Projects are marked with color-coded badges indicating their access level:"
                    }
                    ul class="list-disc list-inside space-y-2 text-gray-700 ml-4" {
                        li {
                            span class="font-medium text-green-700" { "Full Open Access" }
                            " - Data is freely available to everyone"
                        }
                        li {
                            span class="font-medium text-yellow-700" {
                                "Open Access with Restrictions"
                            }
                            " - Some access limitations apply"
                        }
                        li {
                            span class="font-medium text-gray-700" { "Embargoed Access" }
                            " - Data will become available after embargo period"
                        }
                        li {
                            span class="font-medium text-gray-700" { "Metadata only Access" }
                            " - Only metadata is publicly available"
                        }
                    }
                }
                section {
                    h2 class="font-display text-xl font-semibold text-gray-900 mb-3" {
                        "Need More Help?"
                    }
                    p class="text-gray-700 leading-relaxed" {
                        "For questions about specific projects, data access, or depositing your own data at DaSCH, please visit "
                        (link("dasch.swiss", "https://dasch.swiss").external())
                        " or contact the DaSCH team directly."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_help_heading_and_back_link() {
        let out = about_page().into_string();
        assert!(out.contains("Help &amp; Documentation"), "{out}");
        assert!(out.contains(r#"href="/""#), "{out}");
        assert!(out.contains("Back to Projects"), "{out}");
    }

    #[test]
    fn links_to_dasch_externally() {
        let out = about_page().into_string();
        assert!(out.contains(r#"href="https://dasch.swiss""#), "{out}");
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains("dasch.swiss"), "{out}");
    }
}
