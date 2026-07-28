//! Tabs showcase. Tab switching is CSS-only (hidden radio inputs), so the
//! examples need no client-side runtime.

use maud::{html, Markup};
use mosaic_tiles::icon::{icon, IconGitHub, IconLinkedIn, IconSearch};
use mosaic_tiles::tabs::{tab, tabs};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Tabs",
        "A tabbed interface component with underlined style and CSS-only tab switching.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "tabs-basic",
                "Basic Tabs",
                "Simple tabs with text content demonstrating the underlined style",
                basic(),
            )
        })
        ({
            example(
                "tabs-with_icons",
                "Tabs with Icons",
                "Tabs with icons displayed before labels",
                with_icons(),
            )
        })
        ({
            example(
                "tabs-multiple_groups",
                "Multiple Tab Groups",
                "Multiple independent tab groups on the same page",
                multiple_groups(),
            )
        })
    }
}

fn basic() -> Markup {
    let home = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold" { "Home" }
            p { "Welcome to the home tab. This is a simple tab example with text content." }
        }
    };
    let about = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold" { "About" }
            p { "This is the about tab. Each tab can contain any HTML content." }
        }
    };
    let contact = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold" { "Contact" }
            p { "Get in touch through the contact tab. Tab switching is done purely with CSS." }
        }
    };
    let tab_group = html! {
        (tab("basic-tabs", "home", "Home", home).checked())
        (tab("basic-tabs", "about", "About", about))
        (tab("basic-tabs", "contact", "Contact", contact))
    };
    tabs(tab_group)
}

fn with_icons() -> Markup {
    let search = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold flex items-center gap-2" {
                (icon(IconSearch, "w-5 h-5"))
                "Search"
            }
            p { "Find what you're looking for with our powerful search feature." }
            div class="mt-4" {
                input
                    type="text"
                    placeholder="Search..."
                    class="px-3 py-2 border border-neutral-300 rounded-md w-full";
            }
        }
    };
    let github = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold flex items-center gap-2" {
                (icon(IconGitHub, "w-5 h-5"))
                "GitHub"
            }
            p { "View our repositories and contribute to open source projects." }
            a   href="https://github.com"
                class="inline-block mt-4 px-4 py-2 bg-neutral-900 text-white rounded-md hover:bg-neutral-800"
            { "Visit GitHub" }
        }
    };
    let linkedin = html! {
        div class="space-y-2" {
            h3 class="text-lg font-semibold flex items-center gap-2" {
                (icon(IconLinkedIn, "w-5 h-5"))
                "LinkedIn"
            }
            p { "Connect with us on LinkedIn for professional networking." }
            a   href="https://linkedin.com"
                class="inline-block mt-4 px-4 py-2 bg-primary-600 text-white rounded-md hover:bg-primary-700"
            { "Visit LinkedIn" }
        }
    };
    let tab_group = html! {
        (tab("icon-tabs", "search", "Search", search).icon(IconSearch).checked())
        (tab("icon-tabs", "github", "GitHub", github).icon(IconGitHub))
        (tab("icon-tabs", "linkedin", "LinkedIn", linkedin).icon(IconLinkedIn))
    };
    tabs(tab_group)
}

fn multiple_groups() -> Markup {
    let dashboard = html! {
        p { "Dashboard content - Overview of your account and activities." }
    };
    let projects = html! {
        p { "Projects content - List of all your projects and their status." }
    };
    let settings = html! {
        p { "Settings content - Configure your preferences and account settings." }
    };
    let nav_group = html! {
        (tab("nav-tabs", "dashboard", "Dashboard", dashboard).checked())
        (tab("nav-tabs", "projects", "Projects", projects))
        (tab("nav-tabs", "settings", "Settings", settings))
    };

    let overview = html! {
        p { "Overview section - High-level summary of the content." }
    };
    let details = html! {
        p { "Details section - In-depth information and specifications." }
    };
    let content_group = html! {
        (tab("content-tabs", "overview", "Overview", overview).checked())
        (tab("content-tabs", "details", "Details", details))
    };

    html! {
        div class="space-y-8" {
            div {
                h4 class="text-sm font-semibold text-neutral-700 mb-2" { "Navigation Tabs" }
                (tabs(nav_group))
            }
            div {
                h4 class="text-sm font-semibold text-neutral-700 mb-2" { "Content Tabs" }
                (tabs(content_group))
            }
        }
    }
}
