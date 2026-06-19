//! Tabs showcase.
//!
//! The old `interactive` example embedded a Leptos `Counter` island; it is
//! dropped along with the islands runtime. Tab switching itself is CSS-only and
//! survives unchanged.

use maud::{html, Markup};
use mosaic_tiles::icon::{icon, IconGitHub, IconLinkedIn, IconSearch};
use mosaic_tiles::tabs::{tab, tabs, TabProps};

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
    tabs(html! {
        ({
            tab(
                TabProps {
                    name: "basic-tabs",
                    value: "home",
                    label: "Home",
                    checked: true,
                    ..Default::default()
                },
                html! {
                    div class = "space-y-2" { h3 class = "text-lg font-semibold" { "Home"
                    } p {
                    "Welcome to the home tab. This is a simple tab example with text content."
                    } }
                },
            )
        })
        ({
            tab(
                TabProps {
                    name: "basic-tabs",
                    value: "about",
                    label: "About",
                    ..Default::default()
                },
                html! {
                    div class = "space-y-2" { h3 class = "text-lg font-semibold" {
                    "About" } p {
                    "This is the about tab. Each tab can contain any HTML content." } }
                },
            )
        })
        ({
            tab(
                TabProps {
                    name: "basic-tabs",
                    value: "contact",
                    label: "Contact",
                    ..Default::default()
                },
                html! {
                    div class = "space-y-2" { h3 class = "text-lg font-semibold" {
                    "Contact" } p {
                    "Get in touch through the contact tab. Tab switching is done purely with CSS."
                    } }
                },
            )
        })
    })
}

fn with_icons() -> Markup {
    tabs(html! {
        ({
            tab(
                TabProps {
                    name: "icon-tabs",
                    value: "search",
                    label: "Search",
                    icon: Some(IconSearch),
                    checked: true,
                },
                html! {
                    div class = "space-y-2" { h3 class =
                    "text-lg font-semibold flex items-center gap-2" { (icon(IconSearch,
                    "w-5 h-5")) "Search" } p {
                    "Find what you're looking for with our powerful search feature." }
                    div class = "mt-4" { input type = "text" placeholder = "Search..."
                    class = "px-3 py-2 border border-neutral-300 rounded-md w-full"; } }
                },
            )
        })
        ({
            tab(
                TabProps {
                    name: "icon-tabs",
                    value: "github",
                    label: "GitHub",
                    icon: Some(IconGitHub),
                    ..Default::default()
                },
                html! {
                    div class = "space-y-2" { h3 class =
                    "text-lg font-semibold flex items-center gap-2" { (icon(IconGitHub,
                    "w-5 h-5")) "GitHub" } p {
                    "View our repositories and contribute to open source projects." } a
                    href = "https://github.com" class =
                    "inline-block mt-4 px-4 py-2 bg-neutral-900 text-white rounded-md hover:bg-neutral-800"
                    { "Visit GitHub" } }
                },
            )
        })
        ({
            tab(
                TabProps {
                    name: "icon-tabs",
                    value: "linkedin",
                    label: "LinkedIn",
                    icon: Some(IconLinkedIn),
                    ..Default::default()
                },
                html! {
                    div class = "space-y-2" { h3 class =
                    "text-lg font-semibold flex items-center gap-2" { (icon(IconLinkedIn,
                    "w-5 h-5")) "LinkedIn" } p {
                    "Connect with us on LinkedIn for professional networking." } a href =
                    "https://linkedin.com" class =
                    "inline-block mt-4 px-4 py-2 bg-primary-600 text-white rounded-md hover:bg-primary-700"
                    { "Visit LinkedIn" } }
                },
            )
        })
    })
}

fn multiple_groups() -> Markup {
    html! {
        div class="space-y-8" {
            div {
                h4 class="text-sm font-semibold text-neutral-700 mb-2" { "Navigation Tabs" }
                ({
                    tabs(
                        html! {
                            (tab(TabProps { name : "nav-tabs", value : "dashboard", label
                            : "Dashboard", checked : true, ..Default::default() }, html!
                            { p {
                            "Dashboard content - Overview of your account and activities."
                            } })) (tab(TabProps { name : "nav-tabs", value : "projects",
                            label : "Projects", ..Default::default() }, html! { p {
                            "Projects content - List of all your projects and their status."
                            } })) (tab(TabProps { name : "nav-tabs", value : "settings",
                            label : "Settings", ..Default::default() }, html! { p {
                            "Settings content - Configure your preferences and account settings."
                            } }))
                        },
                    )
                })
            }
            div {
                h4 class="text-sm font-semibold text-neutral-700 mb-2" { "Content Tabs" }
                ({
                    tabs(
                        html! {
                            (tab(TabProps { name : "content-tabs", value : "overview",
                            label : "Overview", checked : true, ..Default::default() },
                            html! { p {
                            "Overview section - High-level summary of the content." } }))
                            (tab(TabProps { name : "content-tabs", value : "details",
                            label : "Details", ..Default::default() }, html! { p {
                            "Details section - In-depth information and specifications."
                            } }))
                        },
                    )
                })
            }
        }
    }
}
