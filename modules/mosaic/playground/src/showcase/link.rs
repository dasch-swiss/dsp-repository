//! Link showcase.

use maud::{html, Markup, Render};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, IconGitHub, LinkExternal};
use mosaic_tiles::link::link;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("Link", "A navigation link component with optional button styling.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        (example("link-basic", "Basic Links", "Standard link styling", basic()))
        ({
            example(
                "link-as_button",
                "Links as Buttons",
                "Links styled as button components",
                as_button(),
            )
        })
        ({
            example(
                "link-target_attribute",
                "Target Attribute",
                "Different target attribute values: _self, _blank, _parent, _top",
                target_attribute(),
            )
        })
        ({
            example(
                "link-disabled",
                "Disabled State",
                "Links in disabled state",
                disabled(),
            )
        })
        ({
            example(
                "link-external",
                "External Links",
                "Links to external websites with target and rel attributes",
                external(),
            )
        })
    }
}

/// Shorthand: a plain link to the given href.
fn lnk(href: &str, content: impl Render) -> impl Render {
    link(content, href)
}

fn basic() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (lnk("/link", "About Us"))
            (lnk("/link", "Contact"))
            (lnk("/link", "Blog"))
        }
    }
}

fn as_button() -> Markup {
    let variants = [
        (ButtonVariant::Primary, "Primary"),
        (ButtonVariant::Secondary, "Secondary"),
        (ButtonVariant::Outline, "Outline"),
        (ButtonVariant::Soft, "Soft"),
        (ButtonVariant::Ghost, "Ghost"),
    ];
    html! {
        div class="flex gap-4 items-center" {
            @for (variant, label) in variants { (link(label, "/link").as_button(variant)) }
        }
    }
}

fn target_attribute() -> Markup {
    html! {
        div class="flex flex-col gap-4" {
            div class="flex gap-4 items-center" {
                (lnk("/link", "Default (same frame)"))
                span class="text-sm text-neutral-500" { "No target attribute" }
            }
            div class="flex gap-4 items-center" {
                (link("Target: _self", "/link").target("_self"))
                span class="text-sm text-neutral-500" { "Opens in same frame" }
            }
            div class="flex gap-4 items-center" {
                ({
                    link("Target: _blank", "/link")
                        .target("_blank")
                        .rel("noopener noreferrer")
                })
                span class="text-sm text-neutral-500" { "Opens in new tab/window" }
            }
            div class="flex gap-4 items-center" {
                (link("Target: _parent", "/link").target("_parent"))
                span class="text-sm text-neutral-500" { "Opens in parent frame" }
            }
            div class="flex gap-4 items-center" {
                (link("Target: _top", "/link").target("_top"))
                span class="text-sm text-neutral-500" { "Opens in top-level frame" }
            }
        }
    }
}

fn disabled() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (lnk("/link", "Available Link"))
            (link("Disabled Link", "/link").disabled())
            ({
                link("Disabled Button Link", "/link")
                    .as_button(ButtonVariant::Primary)
                    .disabled()
            })
        }
    }
}

fn external() -> Markup {
    let github = html! {
        (icon(IconGitHub, "w-4 h-4"))
        "GitHub Repository"
    };
    let ext_button = html! {
        "External Button Link"
        (icon(LinkExternal, "w-4 h-4"))
    };
    html! {
        div class="flex flex-col gap-4" {
            p class="text-sm text-neutral-600" {
                "External links should use "
                code class="bg-neutral-100 px-1 rounded" { "target=\"_blank\"" }
                " and "
                code class="bg-neutral-100 px-1 rounded" { "rel=\"noopener noreferrer\"" }
                " for security."
            }
            div class="flex gap-4 items-center" {
                (link("Documentation", "/link").external())
                (link(github, "/link").external())
                ({
                    link(ext_button, "/link")
                        .as_button(ButtonVariant::Primary)
                        .external()
                })
            }
        }
    }
}
