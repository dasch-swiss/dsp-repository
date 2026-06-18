//! Link showcase.

use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, IconGitHub, LinkExternal};
use mosaic_tiles::link::{link, LinkProps};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("Link", "A navigation link component with optional button styling.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("link-basic", "Basic Links", "Standard link styling", basic()))
        (example("link-as_button", "Links as Buttons", "Links styled as button components", as_button()))
        (example("link-target_attribute", "Target Attribute", "Different target attribute values: _self, _blank, _parent, _top", target_attribute()))
        (example("link-disabled", "Disabled State", "Links in disabled state", disabled()))
        (example("link-external", "External Links", "Links to external websites with target and rel attributes", external()))
    }
}

/// Shorthand: a plain link to the given href.
fn lnk(href: &str, content: Markup) -> Markup {
    link(LinkProps { href, ..Default::default() }, content)
}

fn basic() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (lnk("/link", html! { "About Us" }))
            (lnk("/link", html! { "Contact" }))
            (lnk("/link", html! { "Blog" }))
        }
    }
}

fn as_button() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            @for (variant, label) in [
                (ButtonVariant::Primary, "Primary"),
                (ButtonVariant::Secondary, "Secondary"),
                (ButtonVariant::Outline, "Outline"),
                (ButtonVariant::Soft, "Soft"),
                (ButtonVariant::Ghost, "Ghost"),
            ] {
                (link(LinkProps { href: "/link", as_button: Some(variant), ..Default::default() }, html! { (label) }))
            }
        }
    }
}

fn target_attribute() -> Markup {
    html! {
        div class="flex flex-col gap-4" {
            div class="flex gap-4 items-center" {
                (lnk("/link", html! { "Default (same frame)" }))
                span class="text-sm text-neutral-500" { "No target attribute" }
            }
            div class="flex gap-4 items-center" {
                (link(LinkProps { href: "/link", target: Some("_self"), ..Default::default() }, html! { "Target: _self" }))
                span class="text-sm text-neutral-500" { "Opens in same frame" }
            }
            div class="flex gap-4 items-center" {
                (link(LinkProps { href: "/link", target: Some("_blank"), rel: Some("noopener noreferrer"), ..Default::default() }, html! { "Target: _blank" }))
                span class="text-sm text-neutral-500" { "Opens in new tab/window" }
            }
            div class="flex gap-4 items-center" {
                (link(LinkProps { href: "/link", target: Some("_parent"), ..Default::default() }, html! { "Target: _parent" }))
                span class="text-sm text-neutral-500" { "Opens in parent frame" }
            }
            div class="flex gap-4 items-center" {
                (link(LinkProps { href: "/link", target: Some("_top"), ..Default::default() }, html! { "Target: _top" }))
                span class="text-sm text-neutral-500" { "Opens in top-level frame" }
            }
        }
    }
}

fn disabled() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (lnk("/link", html! { "Available Link" }))
            (link(LinkProps { href: "/link", disabled: true, ..Default::default() }, html! { "Disabled Link" }))
            (link(LinkProps { href: "/link", as_button: Some(ButtonVariant::Primary), disabled: true, ..Default::default() }, html! { "Disabled Button Link" }))
        }
    }
}

fn external() -> Markup {
    html! {
        div class="flex flex-col gap-4" {
            p class="text-sm text-neutral-600" {
                "External links should use "
                code class="bg-neutral-100 px-1 rounded" { "target=\"_blank\"" } " and "
                code class="bg-neutral-100 px-1 rounded" { "rel=\"noopener noreferrer\"" }
                " for security."
            }
            div class="flex gap-4 items-center" {
                (link(LinkProps { href: "/link", target: Some("_blank"), rel: Some("noopener noreferrer"), ..Default::default() }, html! { "Documentation" }))
                (link(LinkProps { href: "/link", target: Some("_blank"), rel: Some("noopener noreferrer"), ..Default::default() }, html! { (icon(IconGitHub, "w-4 h-4")) "GitHub Repository" }))
                (link(LinkProps { href: "/link", target: Some("_blank"), rel: Some("noopener noreferrer"), as_button: Some(ButtonVariant::Primary), ..Default::default() }, html! { "External Button Link" (icon(LinkExternal, "w-4 h-4")) }))
            }
        }
    }
}
