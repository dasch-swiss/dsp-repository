use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::components::{
    callout, container, content_section, grid, info_card, CalloutVariant, ContainerWidth, GridColumns,
};
use crate::layout::page_layout;

/// Contact page
pub async fn contact() -> Markup {
    let content = html! {
        // Hero section
        (hero::hero("Contact Us")
            .with_description("Get in touch with our team for support and consultation.")
            .with_id("contact-heading")
            .build())

        (content_section(html! {
            // Contact reasons
            (container(ContainerWidth::Medium, html! {
                h2 class="text-2xl font-bold text-gray-900 dark:text-white" { "How We Can Help" }

                div class="mt-8" {
                    (grid(GridColumns::Two, html! {
                        (info_card("Project Consultation", "Assessing research project compatibility with our infrastructure"))
                        (info_card("Data Management Support", "Guidance on FAIR principles and data management planning"))
                        (info_card("Training & Workshops", "Best practices for humanities data management"))
                        (info_card("Technical Support", "Questions regarding archiving infrastructure"))
                    }))
                }
            }))

            // Contact information
            (container(ContainerWidth::Medium, html! {
                div class="mt-16" {
                    (grid(GridColumns::Two, html! {
                        // Contact details
                        div class="rounded-lg bg-gray-50 p-8 dark:bg-gray-800" {
                            h2 class="text-xl font-bold text-gray-900 dark:text-white" { "Contact Information" }

                            div class="mt-6 space-y-4" {
                                div {
                                    h3 class="text-sm font-semibold text-gray-900 dark:text-white" { "Email" }
                                    p class="mt-1 text-gray-600 dark:text-gray-400" {
                                        a href="mailto:info@dasch.swiss" class="text-indigo-600 hover:text-indigo-500 dark:text-indigo-400" {
                                            "info@dasch.swiss"
                                        }
                                    }
                                }

                                div {
                                    h3 class="text-sm font-semibold text-gray-900 dark:text-white" { "Phone" }
                                    p class="mt-1 text-gray-600 dark:text-gray-400" {
                                        a href="tel:+41612076400" class="text-indigo-600 hover:text-indigo-500 dark:text-indigo-400" {
                                            "+41 61 207 64 00"
                                        }
                                    }
                                }

                                div {
                                    h3 class="text-sm font-semibold text-gray-900 dark:text-white" { "Availability" }
                                    ul class="mt-1 space-y-1 text-sm text-gray-600 dark:text-gray-400" {
                                        li { "Monday: Full day" }
                                        li { "Tuesday & Wednesday: Afternoons only" }
                                    }
                                }
                            }
                        }

                        // Physical location
                        div class="rounded-lg bg-gray-50 p-8 dark:bg-gray-800" {
                            h2 class="text-xl font-bold text-gray-900 dark:text-white" { "Physical Location" }

                            div class="mt-6" {
                                h3 class="text-sm font-semibold text-gray-900 dark:text-white" { "Address" }
                                address class="mt-1 not-italic text-gray-600 dark:text-gray-400" {
                                    "Kornhausgasse 7" br;
                                    "4051 Basel" br;
                                    "Switzerland"
                                }
                            }
                        }
                    }))
                }
            }))

            // Additional information
            (container(ContainerWidth::Medium, html! {
                div class="mt-12" {
                    (callout(
                        html! {
                            p class="text-gray-700 dark:text-gray-300" {
                                "For organizational details and team information, please visit our "
                                a href="/about-us" class="font-semibold text-indigo-600 hover:text-indigo-500 dark:text-indigo-400" {
                                    "About Us"
                                }
                                " section."
                            }
                        },
                        CalloutVariant::Blue
                    ))
                }
            }))
        }))
    };

    page_layout("Contact - DaSCH Swiss", content)
}
