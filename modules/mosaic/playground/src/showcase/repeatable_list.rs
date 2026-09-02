//! Repeatable list showcase.

use maud::{html, Markup};
use mosaic_tiles::repeatable_list::repeatable_list;
use mosaic_tiles::text_field::text_field;
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

/// The base URL the examples add and remove rows under. A real caller passes
/// its own route; this is what the buttons' `formaction` values are built from.
const ROWS: &str = "/projects/0801/sections/dataset/rows/keywords";

pub fn page() -> Markup {
    let header = page_header(
        "Repeatable List",
        "An ordered list of rows a reader can add to and remove from, each row's fields supplied by the \
         caller. Rows are keyed by an opaque server id, never an index, and both controls are submit buttons \
         so the whole form body travels with them.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "repeatable_list-rows",
                "Rows with an opaque key",
                "Each row submits a hidden `keywords.row` field holding its key, repeated in DOM order — the \
                 order is read off that repetition rather than from a number that has to be kept in step with \
                 it. Naming a row's fields `keywords[0]`, `keywords[1]` instead breaks the moment a middle row \
                 goes: the next submit carries 0 and 2, and per-row errors keyed by index then point at the \
                 wrong rows.",
                rows(),
            )
        })
        ({
            example(
                "repeatable_list-empty",
                "Nothing added yet",
                "An empty list says so. Without a message it renders as a legend and an add button with \
                 nothing between them, which reads as a section that failed to load.",
                empty(),
            )
        })
        ({
            example(
                "repeatable_list-error",
                "Rejected, with the reason",
                "The message hangs off the fieldset and is announced once, as with the other grouped fields.",
                error_state(),
            )
        })
    }
}

fn form_column(content: Markup) -> Markup {
    html! {
        div class="flex max-w-lg flex-col gap-6" { (content) }
    }
}

/// One keyword row: the same term in two languages.
fn keyword_row(key: &str, en: &str, de: &str) -> Markup {
    html! {
        ({
            text_field(format!("keywords.{key}.en"), "English")
                .value(en)
                .with_id(format!("kw-{key}-en"))
        })
        ({
            text_field(format!("keywords.{key}.de"), "German")
                .value(de)
                .with_id(format!("kw-{key}-de"))
        })
    }
}

fn rows() -> Markup {
    let list = repeatable_list("keywords", "Keywords", ROWS)
        .item_noun("keyword")
        .add_label("Add a keyword")
        .hint("Each keyword is one multilingual term.")
        .row("k7f3", keyword_row("k7f3", "manuscripts", "Handschriften"))
        .row("k2b9", keyword_row("k2b9", "palaeography", "Paläographie"));
    form_column(html! {
        (list)
    })
}

fn empty() -> Markup {
    let list = repeatable_list("alternativeNames", "Alternative names", ROWS)
        .item_noun("name")
        .add_label("Add an alternative name")
        .empty_message("No alternative names yet.")
        .hint("Acronyms or alternate spellings — one value per language.");
    form_column(html! {
        (list)
    })
}

fn error_state() -> Markup {
    let list = repeatable_list("disciplines", "Disciplines", ROWS)
        .item_noun("discipline")
        .add_label("Add a discipline")
        .empty_message("No disciplines yet.")
        .hint("At least one is required before submitting.")
        .error("Add at least one discipline.")
        .with_test_id("disciplines-rows");
    form_column(html! {
        (list)
    })
}
