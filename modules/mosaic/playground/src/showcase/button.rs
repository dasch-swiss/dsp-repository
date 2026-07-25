//! Button showcase. All examples are static renders.

use maud::{html, Markup, Render};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::icon::{icon, CopyPaste, IconChevronRight, IconSearch, Info, LinkExternal, Mail};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header("Button", "A clickable button component with multiple variants and states.");
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "button-variants",
                "Button Variants",
                "Available button styles: Primary, Secondary, Outline, Ghost, and Soft",
                variants(),
            )
        })
        ({
            example(
                "button-types",
                "Button Types",
                "HTML button types: Button, Submit, and Reset",
                types(),
            )
        })
        ({
            example(
                "button-disabled",
                "Disabled State",
                "Buttons in disabled state",
                disabled(),
            )
        })
        ({
            example(
                "button-with_icons",
                "Buttons with Icons",
                "Buttons combined with icon components",
                with_icons(),
            )
        })
    }
}

/// Shorthand: a button with a variant and label.
fn btn(variant: ButtonVariant, label: impl Render) -> impl Render {
    button(label).variant(variant)
}

fn variants() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (btn(ButtonVariant::Primary, "Primary Button"))
            (btn(ButtonVariant::Secondary, "Secondary Button"))
            (btn(ButtonVariant::Outline, "Outline Button"))
            (btn(ButtonVariant::Ghost, "Ghost Button"))
            (btn(ButtonVariant::Soft, "Soft Button"))
        }
    }
}

fn types() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (button("Button").button_type(ButtonType::Button))
            (button("Submit").button_type(ButtonType::Submit))
            (button("Reset").button_type(ButtonType::Reset))
        }
    }
}

fn disabled() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (button("Disabled Primary").variant(ButtonVariant::Primary).disabled())
            (button("Disabled Secondary").variant(ButtonVariant::Secondary).disabled())
        }
    }
}

fn with_icons() -> Markup {
    let search = html! {
        (icon(IconSearch, "w-4 h-4 inline mr-2"))
        "Search"
    };
    let email = html! {
        (icon(Mail, "w-4 h-4 inline mr-2"))
        "Send Email"
    };
    let docs = html! {
        "Visit Docs"
        (icon(LinkExternal, "w-4 h-4 inline ml-2"))
    };
    html! {
        div class="space-y-6" {
            div {
                h4 class="text-base font-semibold mb-2" { "Icon + Text" }
                div class="flex gap-4 items-center" {
                    (btn(ButtonVariant::Primary, search))
                    (btn(ButtonVariant::Secondary, email))
                    (btn(ButtonVariant::Primary, docs))
                }
            }
            div {
                h4 class="text-base font-semibold mb-2" { "Icon Only" }
                div class="flex gap-4 items-center" {
                    (btn(ButtonVariant::Secondary, icon(CopyPaste, "w-4 h-4")))
                    (btn(ButtonVariant::Secondary, icon(Info, "w-4 h-4")))
                    (btn(ButtonVariant::Primary, icon(IconChevronRight, "w-4 h-4")))
                }
            }
        }
    }
}
