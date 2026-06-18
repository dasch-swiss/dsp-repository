//! Hand-written Maud showcase pages, one module per surviving component.
//!
//! Each module exposes `pub fn page() -> Markup`. Examples are wrapped with
//! `data-example-key="{component}-{example}"` so the Phase-6 component-isolation
//! visual A/B can address each render in isolation (plan Appendix C.10).

pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod icon;
pub mod link;
pub mod tabs;
pub mod theme;

use maud::{html, Markup};

/// The shared page header: component name and description.
fn page_header(name: &str, description: &str) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-3" { (name) }
        p class="text-xl text-neutral-600 mb-6" { (description) }
    }
}

/// One example block: a heading, an optional description, and the rendered
/// component inside a bordered preview surface. The `key` becomes
/// `data-example-key`, preserved verbatim for the visual A/B (Appendix C.10).
fn example(key: &str, title: &str, description: &str, body: Markup) -> Markup {
    html! {
        div class="mb-8" data-example-key=(key) {
            h3 class="text-xl font-semibold mb-2" { (title) }
            @if !description.is_empty() {
                p class="text-neutral-600 mb-3" { (description) }
            }
            div class="mb-4 p-6 border rounded bg-white" { (body) }
        }
    }
}

/// Wrap a page's examples in the standard content column.
fn page_layout(header: Markup, examples: Markup) -> Markup {
    html! {
        div class="max-w-5xl mx-auto" {
            (header)
            div class="mb-8" {
                h2 class="text-2xl font-bold mb-4" { "Examples" }
                (examples)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every showcase page renders, carries its `<h1>` heading, and preserves at
    /// least one `data-example-key` (the Phase-6 component-isolation A/B hook).
    #[test]
    fn pages_render_with_example_keys() {
        let pages: &[(&str, String)] = &[
            ("Badge", badge::page().into_string()),
            ("Breadcrumb", breadcrumb::page().into_string()),
            ("Button", button::page().into_string()),
            ("Card", card::page().into_string()),
            ("Icon", icon::page().into_string()),
            ("Link", link::page().into_string()),
            ("Tabs", tabs::page().into_string()),
            ("Design Tokens", theme::page().into_string()),
        ];
        for (name, html) in pages {
            assert!(
                html.contains(&format!("<h1 class=\"text-4xl font-bold mb-3\">{name}</h1>")),
                "{name}: missing heading"
            );
            assert!(html.contains("data-example-key="), "{name}: no data-example-key");
        }
    }

    /// The `example` wrapper emits the key verbatim on the outer block.
    #[test]
    fn example_emits_data_example_key() {
        let out = example("card-variants", "Title", "", html! { "x" }).into_string();
        assert!(out.contains(r#"<div class="mb-8" data-example-key="card-variants">"#), "{out}");
    }
}
