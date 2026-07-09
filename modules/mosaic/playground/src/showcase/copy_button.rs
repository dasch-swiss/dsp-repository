//! Copy Button showcase.

use maud::{html, Markup};
use mosaic_tiles::copy_button::copy_button;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Copy Button",
        "A ghost icon button that copies a value to the clipboard and confirms via tooltip and an aria-live status.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "copy_button-basic",
                "Basic",
                "Click to copy the value to the clipboard.",
                basic(),
            )
        })
        ({
            example(
                "copy_button-inline",
                "Inline With Text",
                "Sits next to the value it copies, e.g. a permalink.",
                inline(),
            )
        })
    }
}

fn basic() -> Markup {
    html! {
        div class="flex gap-4 items-center" {
            (copy_button("https://example.org/permalink"))
        }
    }
}

fn inline() -> Markup {
    html! {
        div class="flex gap-2 items-center" {
            code class="bg-neutral-100 px-2 py-1 rounded text-sm" {
                "https://example.org/permalink"
            }
            (copy_button("https://example.org/permalink"))
        }
    }
}
