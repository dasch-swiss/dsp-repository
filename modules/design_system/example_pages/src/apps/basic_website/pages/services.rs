use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::components::{
    callout_with_heading, container, content_section, grid, CalloutVariant, ContainerWidth, GridColumns,
};
use crate::layout::page_layout;

/// Services page
pub async fn services() -> Markup {
    let content = html! {
        // Hero section
        (hero::hero("Our Services")
            .with_description("Comprehensive data management services for humanities research.")
            .with_id("services-page-heading")
            .build())

        (content_section(html! {
            // Section 1: Training & Consulting
            (container(ContainerWidth::Medium, html! {
                    div class="space-y-12" {
                        // Workshops
                        div id="workshops" {
                            h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Workshops" }
                            p class="mt-4 text-lg text-gray-600 dark:text-gray-400" {
                                span class="font-semibold text-green-600 dark:text-green-400" { "Free of charge" }
                            }
                            p class="mt-4 text-gray-600 dark:text-gray-400" {
                                "Customized workshops including hands-on training in the use of the DaSCH infrastructure. Topics include:"
                            }
                            ul class="mt-4 list-disc space-y-2 pl-6 text-gray-600 dark:text-gray-400" {
                                li { "Data management plan guidance" }
                                li { "Best practices for qualitative humanities data" }
                                li { "FAIR principles implementation" }
                            }
                            p class="mt-4 text-gray-600 dark:text-gray-400" {
                                "Particularly available to doctoral researchers."
                            }
                        }

                        // Consulting
                        div id="consulting" {
                            h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Consulting Services" }
                            p class="mt-4 text-gray-600 dark:text-gray-400" {
                                "Guidance on data management for qualitative materials. Services include:"
                            }
                            ul class="mt-4 list-disc space-y-2 pl-6 text-gray-600 dark:text-gray-400" {
                                li { "Data modeling" }
                                li { "Project-specific workflows" }
                                li { "Custom script development for data cleaning and DSP integration" }
                            }
                            p class="mt-4 text-lg font-semibold text-indigo-600 dark:text-indigo-400" {
                                "Initial consultation up to eight hours is complimentary for scholars"
                            }
                        }
                    }
            }))

            // Section 2: Pricing Structure
            (container(ContainerWidth::Medium, html! {
                div class="mt-20" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Pricing Structure" }

                    div class="mt-8 space-y-8" {
                        // Archiving costs
                        div class="rounded-lg border border-gray-200 p-6 dark:border-gray-700" {
                            h3 class="text-xl font-semibold text-gray-900 dark:text-white" { "Archiving Costs" }
                            p class="mt-4 text-lg font-semibold text-green-600 dark:text-green-400" {
                                "Repository services are free for Swiss national research projects"
                            }
                            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" {
                                "Data exceeding 500 GB may require cost-sharing after 2025"
                            }
                        }

                        // Consulting rates
                        div class="rounded-lg border border-gray-200 p-6 dark:border-gray-700" {
                            h3 class="text-xl font-semibold text-gray-900 dark:text-white" { "Consulting Rates" }
                            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" {
                                "(After initial free consultation)"
                            }
                            ul class="mt-4 space-y-3 text-gray-600 dark:text-gray-400" {
                                li class="flex justify-between" {
                                    span { "PI at the University of Basel" }
                                    span class="font-semibold" { "CHF 640/day" }
                                }
                                li class="flex justify-between" {
                                    span { "Other Higher Education Institutions" }
                                    span class="font-semibold" { "CHF 640/day + 8.1% VAT" }
                                }
                                li class="flex justify-between" {
                                    span { "All other external institutions" }
                                    span class="font-semibold" { "CHF 1,280/day + 8.1% VAT" }
                                }
                            }
                        }
                    }
                }
            }))

            // Section 3: Data Deposit Options
            (container(ContainerWidth::Medium, html! {
                div class="mt-20" {
                    h2 id="data-deposit" class="text-3xl font-bold text-gray-900 dark:text-white" { "Data Deposit Options" }

                    div class="mt-8" {
                        (grid(GridColumns::Two, html! {
                            // Option 1: DSP
                        div class="rounded-lg border border-indigo-200 bg-indigo-50 p-6 dark:border-indigo-800 dark:bg-indigo-950" {
                            h3 class="text-2xl font-bold text-gray-900 dark:text-white" {
                                "DaSCH Service Platform (DSP)"
                            }
                            p class="mt-2 text-sm font-semibold text-indigo-600 dark:text-indigo-400" {
                                "Specialization: XML text-based and multimedia humanities data"
                            }

                            h4 class="mt-6 font-semibold text-gray-900 dark:text-white" { "Features:" }
                            ul class="mt-2 list-disc space-y-2 pl-6 text-gray-700 dark:text-gray-300" {
                                li { "Customizable metadata models" }
                                li { "Version control" }
                                li { "Persistent identifiers" }
                                li { "Ongoing editing capabilities" }
                            }

                            p class="mt-4 text-gray-700 dark:text-gray-300" {
                                strong { "Preservation: " }
                                "Long-term preservation with ongoing access"
                            }
                        }

                        // Option 2: SWISSUbase
                        div class="rounded-lg border border-gray-200 bg-gray-50 p-6 dark:border-gray-700 dark:bg-gray-800" {
                            h3 class="text-2xl font-bold text-gray-900 dark:text-white" {
                                "SWISSUbase for Humanities"
                            }
                            p class="mt-2 text-sm font-semibold text-gray-600 dark:text-gray-400" {
                                "Suited for: Smaller datasets"
                            }

                            h4 class="mt-6 font-semibold text-gray-900 dark:text-white" { "Specifications:" }
                            ul class="mt-2 list-disc space-y-2 pl-6 text-gray-700 dark:text-gray-300" {
                                li { "Limits: Up to 1 GB, 10 files" }
                                li { "Simpler metadata requirements" }
                            }

                            p class="mt-4 text-gray-700 dark:text-gray-300" {
                                strong { "Preservation: " }
                                "Approximately 10-year preservation without active management"
                            }
                        }
                        }))
                    }

                    (callout_with_heading(
                        "Both platforms",
                        html! {
                            p class="text-gray-700 dark:text-gray-300" {
                                "Maintain FAIR compliance and provide expert curatorial support"
                            }
                        },
                        CalloutVariant::Blue
                    ))
                }
            }))
        }))
    };

    page_layout("Services - DaSCH Swiss", content)
}
