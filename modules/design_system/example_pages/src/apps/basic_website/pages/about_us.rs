use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::components::{container, content_section, ContainerWidth};
use crate::layout::page_layout;

/// About Us page
pub async fn about_us() -> Markup {
    let content = html! {
        // Hero section
        (hero::hero("About Us")
            .with_id("about-heading")
            .build())

        (content_section(html! {
            // Section 1: Who We Are
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Who We Are" }

                    p class="mt-6 text-lg text-gray-600 dark:text-gray-400" {
                        "The Swiss National Data and Service Center for the Humanities (DaSCH) began as a pilot project initiated by the Digital Humanities Lab at the University of Basel and the Swiss Academy of Humanities and Social Sciences."
                    }

                    div class="mt-6 space-y-4 text-gray-600 dark:text-gray-400" {
                        p {
                            strong { "2017: " }
                            "Became a national facility"
                        }
                        p {
                            strong { "2021: " }
                            "Operating as research data infrastructure"
                        }
                        p {
                            strong { "Funding: " }
                            "Primarily financed by the Swiss National Science Foundation"
                        }
                    }
                }
            }))

            // Section 2: Mission Statement
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Mission Statement" }

                    p class="mt-6 text-lg text-gray-600 dark:text-gray-400" {
                        "Developing a FAIR enabling trusted digital repository for open research data in the humanities."
                    }

                    div class="mt-8 rounded-lg bg-indigo-50 p-6 dark:bg-indigo-950" {
                        h3 class="text-xl font-semibold text-gray-900 dark:text-white" { "Our Priorities" }
                        ul class="mt-4 list-disc space-y-2 pl-6 text-gray-700 dark:text-gray-300" {
                            li { "Long-term direct access to research data" }
                            li { "Continuous data editing capabilities" }
                            li { "Precise citation features" }
                        }
                    }

                    p class="mt-6 text-gray-600 dark:text-gray-400" {
                        strong { "Core Values: " }
                        "Reliability, flexibility, appreciation, curiosity, and persistence"
                    }
                }
            }))

            // Section 3: Service Platform Evolution
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Service Platform Evolution (2025-2028)" }

                    p class="mt-6 text-gray-600 dark:text-gray-400" {
                        "Transition from monolithic system to modular architecture:"
                    }

                    div class="mt-6 grid gap-4 sm:grid-cols-3" {
                        div class="rounded-lg border border-gray-200 p-4 dark:border-gray-700" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Virtual Research Environment" }
                            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" { "Separate module" }
                        }
                        div class="rounded-lg border border-gray-200 p-4 dark:border-gray-700" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Archiving" }
                            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" { "Separate module" }
                        }
                        div class="rounded-lg border border-gray-200 p-4 dark:border-gray-700" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Discovery/Re-Use" }
                            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" { "Separate module" }
                        }
                    }

                    p class="mt-6 text-gray-600 dark:text-gray-400" {
                        strong { "Benefits: " }
                        "Enhanced scalability and interoperability"
                    }
                }
            }))

            // Section 4: Governance Structure
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Governance Structure" }

                    div class="mt-6 space-y-4" {
                        div class="rounded-lg border-l-4 border-indigo-600 bg-gray-50 p-4 dark:bg-gray-800" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Assembly of Delegates" }
                            p class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Highest decision-making body" }
                        }
                        div class="rounded-lg border-l-4 border-indigo-600 bg-gray-50 p-4 dark:bg-gray-800" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Board of the Association" }
                            p class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Strategic oversight" }
                        }
                        div class="rounded-lg border-l-4 border-indigo-600 bg-gray-50 p-4 dark:bg-gray-800" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Scientific Advisory Board" }
                            p class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "International expertise" }
                        }
                        div class="rounded-lg border-l-4 border-indigo-600 bg-gray-50 p-4 dark:bg-gray-800" {
                            h4 class="font-semibold text-gray-900 dark:text-white" { "Executive Board" }
                            p class="mt-1 text-sm text-gray-600 dark:text-gray-400" { "Operational management" }
                        }
                    }

                    p class="mt-6 text-gray-600 dark:text-gray-400" {
                        strong { "Host institution: " }
                        "University of Basel"
                    }
                }
            }))

            // Section 5: Team Composition
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    h2 class="text-3xl font-bold text-gray-900 dark:text-white" { "Team Composition" }

                    div class="mt-6 grid gap-6 sm:grid-cols-2" {
                        div class="rounded-lg bg-blue-50 p-6 dark:bg-blue-950" {
                            p class="text-3xl font-bold text-indigo-600 dark:text-indigo-400" { "~19" }
                            p class="mt-2 text-gray-700 dark:text-gray-300" { "Core staff members" }
                        }
                        div class="rounded-lg bg-blue-50 p-6 dark:bg-blue-950" {
                            p class="text-3xl font-bold text-indigo-600 dark:text-indigo-400" { "3" }
                            p class="mt-2 text-gray-700 dark:text-gray-300" { "Associated researchers" }
                        }
                    }

                    div class="mt-6" {
                        h3 class="text-xl font-semibold text-gray-900 dark:text-white" { "Divisions" }
                        ul class="mt-4 list-disc space-y-2 pl-6 text-gray-600 dark:text-gray-400" {
                            li { "Repository Services" }
                            li { "Engineering" }
                        }
                    }

                    p class="mt-6 text-gray-600 dark:text-gray-400" {
                        "Team members come from diverse backgrounds including computer science, digital humanities, archaeology, and other disciplines."
                    }
                }
            }))

            // Section 6: DARIAH-CH Coordination
            (container(ContainerWidth::Medium, html! {
                div class="mt-16 rounded-lg bg-gray-50 p-8 dark:bg-gray-800" {
                    h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "DARIAH-CH Coordination" }

                    p class="mt-4 text-gray-600 dark:text-gray-400" {
                        "DaSCH coordinates Swiss participation in the European Digital Research Infrastructure for Arts and Humanities (DARIAH)."
                    }

                    div class="mt-6 space-y-3" {
                        p class="text-gray-700 dark:text-gray-300" {
                            strong { "National Coordinator: " }
                            "Prof. Dr. Rita Gautschy"
                        }
                        p class="text-gray-700 dark:text-gray-300" {
                            strong { "National Coordination Officer: " }
                            "Dr. Cristina Grisot"
                        }
                    }
                }
            }))
        }))
    };

    page_layout("About Us - DaSCH Swiss", content)
}
