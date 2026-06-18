//! Icon showcase.

use maud::{html, Markup};
use mosaic_tiles::icon::{
    icon, Clock, CopyPaste, Data, Document, DownloadFile, Flag, Grid, Hamburger, Help, IconChevronDown,
    IconChevronLeft, IconChevronRight, IconChevronUp, IconData, IconGitHub, IconLinkedIn, IconSearch, IconX, Info,
    LinkExternal, LockClosed, LockOpen, Mail, OpenDocument, People, Sidebar, Tune,
};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Icon",
        "A collection of SVG icons sourced from the icondata crate, styled with the base icon class plus Tailwind utilities.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("icon-all_icons", "All Available Icons", "All available icons with their names", all_icons()))
        (example("icon-sizes_and_colors", "Sizes and Colors", "Control icon size with Tailwind width/height classes and color with text color classes", sizes_and_colors()))
        (example("icon-usage", "Real-World Usage", "Icons in buttons, links, alerts, and other UI elements", usage()))
    }
}

/// The icons shown in the catalogue grid, paired with their export name.
const CATALOGUE: &[(IconData, &str)] = &[
    (IconChevronUp, "ChevronUp"),
    (IconChevronDown, "ChevronDown"),
    (IconChevronLeft, "ChevronLeft"),
    (IconChevronRight, "ChevronRight"),
    (IconSearch, "Search"),
    (LinkExternal, "LinkExternal"),
    (Info, "Info"),
    (Mail, "Mail"),
    (CopyPaste, "CopyPaste"),
    (IconLinkedIn, "IconLinkedIn"),
    (IconX, "IconX"),
    (IconGitHub, "IconGitHub"),
    (People, "People"),
    (Hamburger, "Hamburger"),
    (Sidebar, "Sidebar"),
    (Document, "Document"),
    (Data, "Data"),
    (Help, "Help"),
    (LockClosed, "LockClosed"),
    (LockOpen, "LockOpen"),
    (Clock, "Clock"),
    (Flag, "Flag"),
    (OpenDocument, "OpenDocument"),
    (Tune, "Tune"),
    (Grid, "Grid"),
    (DownloadFile, "DownloadFile"),
];

fn all_icons() -> Markup {
    html! {
        div class="grid grid-cols-4 gap-6" {
            @for &(ic, name) in CATALOGUE {
                div class="flex flex-col items-center gap-2 p-4 border rounded-lg" {
                    (icon(ic, "w-6 h-6"))
                    span class="text-sm text-neutral-600" { (name) }
                }
            }
        }
    }
}

fn sizes_and_colors() -> Markup {
    html! {
        div class="space-y-8" {
            div {
                h4 class="text-base font-semibold mb-3" { "Sizes" }
                p class="text-sm text-neutral-600 mb-4" { "Control icon size using Tailwind width and height classes." }
                div class="space-y-3" {
                    @for (cls, label) in [("w-4 h-4", "w-4 h-4 (16px)"), ("w-5 h-5", "w-5 h-5 (20px)"), ("w-6 h-6", "w-6 h-6 (24px)"), ("w-8 h-8", "w-8 h-8 (32px)"), ("w-12 h-12", "w-12 h-12 (48px)")] {
                        div class="flex items-center gap-4" {
                            (icon(IconSearch, cls))
                            code class="text-sm" { (label) }
                        }
                    }
                }
            }
            div {
                h4 class="text-base font-semibold mb-3" { "Colors" }
                p class="text-sm text-neutral-600 mb-4" { "Icons use currentColor and inherit text color from parent or Tailwind classes." }
                div class="flex gap-6" {
                    (icon(IconGitHub, "w-8 h-8 text-neutral-500"))
                    (icon(IconGitHub, "w-8 h-8 text-primary-600"))
                    (icon(IconGitHub, "w-8 h-8 text-danger-500"))
                    (icon(IconGitHub, "w-8 h-8 text-success-600"))
                    (icon(IconGitHub, "w-8 h-8 text-accent-500"))
                }
            }
        }
    }
}

fn usage() -> Markup {
    html! {
        div class="space-y-6" {
            div {
                h4 class="text-base font-semibold mb-3" { "Buttons" }
                button class="inline-flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700" {
                    (icon(IconSearch, "w-4 h-4"))
                    "Search"
                }
            }
            div {
                h4 class="text-base font-semibold mb-3" { "Links" }
                a href="#" class="inline-flex items-center gap-2 text-primary-600 hover:underline" {
                    "Visit Documentation"
                    (icon(LinkExternal, "w-4 h-4"))
                }
            }
            div {
                h4 class="text-base font-semibold mb-3" { "Alerts" }
                div class="flex items-center gap-2 p-3 bg-info-50 border border-info-200 rounded-lg" {
                    (icon(Info, "w-5 h-5 text-info-600"))
                    span class="text-sm text-info-800" { "This is an informational message" }
                }
            }
            div {
                h4 class="text-base font-semibold mb-3" { "Navigation" }
                div class="flex items-center gap-2" {
                    button class="p-2 border rounded hover:bg-neutral-50" aria-label="Previous" {
                        (icon(IconChevronLeft, "w-4 h-4 text-neutral-600"))
                    }
                    span class="text-sm text-neutral-700" { "Page 1 of 10" }
                    button class="p-2 border rounded hover:bg-neutral-50" aria-label="Next" {
                        (icon(IconChevronRight, "w-4 h-4 text-neutral-600"))
                    }
                }
            }
            div {
                h4 class="text-base font-semibold mb-3" { "Social Media" }
                div class="flex gap-4" {
                    a href="#" class="p-2 text-neutral-600 hover:text-neutral-900" aria-label="GitHub" {
                        (icon(IconGitHub, "w-6 h-6"))
                    }
                    a href="#" class="p-2 text-neutral-600 hover:text-primary-600" aria-label="LinkedIn" {
                        (icon(IconLinkedIn, "w-6 h-6"))
                    }
                    a href="#" class="p-2 text-neutral-600 hover:text-neutral-900" aria-label="X" {
                        (icon(IconX, "w-6 h-6"))
                    }
                }
            }
        }
    }
}
