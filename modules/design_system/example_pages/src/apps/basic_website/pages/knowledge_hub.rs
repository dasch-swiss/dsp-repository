use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::components::{
    article_category, callout_with_heading, container, content_section, Article, CalloutVariant, ContainerWidth,
};
use crate::layout::page_layout;

/// Knowledge Hub page
pub async fn knowledge_hub() -> Markup {
    // Define article categories
    let getting_started = [Article { title: "Welcome to DaSCH".to_string(), href: "#".to_string() }];

    let fundamentals = [
        Article {
            title: "FAIR and CARE Principles".to_string(),
            href: "#".to_string(),
        },
        Article { title: "Research Ethics".to_string(), href: "#".to_string() },
        Article { title: "Anonymization".to_string(), href: "#".to_string() },
        Article {
            title: "Copyright & Licenses".to_string(),
            href: "#".to_string(),
        },
    ];

    let guides = [
        Article {
            title: "Archiving Workflow".to_string(),
            href: "#".to_string(),
        },
        Article {
            title: "Supported File Formats".to_string(),
            href: "#".to_string(),
        },
        Article {
            title: "Naming Conventions".to_string(),
            href: "#".to_string(),
        },
        Article {
            title: "Permissions System".to_string(),
            href: "#".to_string(),
        },
    ];

    let best_practices = [Article {
        title: "Documentation Guidelines".to_string(),
        href: "#".to_string(),
    }];

    let advanced_topics = [Article { title: "Custom Frontends".to_string(), href: "#".to_string() }];

    let reference = [Article { title: "Glossary".to_string(), href: "#".to_string() }];

    let content = html! {
        // Hero section
        (hero::hero("Knowledge Hub")
            .with_description("Self-service resource center for humanities research data archiving. Step-by-step guides for common archiving tasks and best practices for data preparation and organization.")
            .with_id("knowledge-heading")
            .build())

        (content_section(html! {
            // Article Categories
            (container(ContainerWidth::Medium, html! {
                    div class="space-y-12" {
                        (article_category("Getting Started", 1, &getting_started, false))
                        (article_category("Fundamentals", 4, &fundamentals, false))
                        (article_category("Guides", 4, &guides, false))
                        (article_category("Best Practices", 1, &best_practices, false))
                        (article_category("Advanced Topics", 1, &advanced_topics, false))
                        (article_category("Reference", 1, &reference, true))
                    }
            }))

            // Support Distinction
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    (callout_with_heading(
                        "When to Use Knowledge Hub",
                        html! {
                            p class="mt-4 text-gray-700 dark:text-gray-300" {
                                "Use the Knowledge Hub for technical requirements and best practices for data preparation."
                            }
                            p class="mt-4 text-gray-700 dark:text-gray-300" {
                                strong { "Contact DaSCH for: " }
                                "Data modeling, custom integration, and team training."
                            }
                        },
                        CalloutVariant::Blue
                    ))
                }
            }))

            // Additional Features
            (container(ContainerWidth::Medium, html! {
                div class="mt-12" {
                    h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "Downloadable Resources" }
                    p class="mt-4 text-gray-600 dark:text-gray-400" {
                        "Legal documents available for download:"
                    }
                    ul class="mt-4 list-disc space-y-2 pl-6 text-gray-600 dark:text-gray-400" {
                        li { "Terms and Conditions" }
                        li { "Deposit Agreement" }
                        li { "Statutes" }
                        li { "Terms of Service" }
                    }
                    p class="mt-4 text-gray-600 dark:text-gray-400" {
                        "PDF export functionality available: Single or multi-article collections with cover pages and table of contents."
                    }
                }
            }))
        }))
    };

    page_layout("Knowledge Hub - DaSCH Swiss", content)
}
