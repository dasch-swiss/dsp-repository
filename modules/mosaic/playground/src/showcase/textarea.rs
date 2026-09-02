//! Textarea showcase.

use maud::{html, Markup};
use mosaic_tiles::textarea::textarea;
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Textarea",
        "A label, a multi-line input, and an optional hint and error — the same `field-*` shell as the text \
         field, so a long-form field is the same control with a different box.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "textarea-basic",
                "Label and control",
                "The name is given once; the id, the label's `for` and the error region's id are derived from \
                 it.",
                basic(),
            )
        })
        ({
            example(
                "textarea-rows",
                "Height",
                "`rows` sets the visible height. The control is vertically resizable and does not grow as the \
                 text does — a field that grows while someone types moves everything below it.",
                rows(),
            )
        })
        ({
            example(
                "textarea-prefilled",
                "Holding a value",
                "A textarea's value is element content, not a `value` attribute — a `value=\"…\"` here is \
                 silently ignored and the field renders empty. Line breaks and leading blank lines survive the \
                 round trip.",
                prefilled(),
            )
        })
        ({
            example(
                "textarea-error",
                "Rejected, with the reason",
                "`error` sets `aria-invalid` and the description together, and keeps the hint rather than \
                 replacing it.",
                error_state(),
            )
        })
        ({
            example(
                "textarea-capped",
                "Capped length",
                "`maxlength` is an affordance: browsers enforce it on typing but not on every paste, and never \
                 on a request that did not come from the form, so the server still checks.",
                capped(),
            )
        })
    }
}

/// The showcase renders fields in a column, as a form would.
fn form_column(content: Markup) -> Markup {
    html! {
        div class="flex max-w-md flex-col gap-4" { (content) }
    }
}

fn basic() -> Markup {
    let fields = html! {
        (textarea("description", "Description").required())
        ({
            textarea("provenance", "Provenance")
                .hint("Where the data came from, or how it was produced.")
        })
    };
    form_column(fields)
}

fn rows() -> Markup {
    let fields = html! {
        (textarea("abstract", "Abstract").rows(2))
        (textarea("long_description", "Description").rows(8))
    };
    form_column(fields)
}

fn prefilled() -> Markup {
    let fields = html! {
        ({
            textarea("description", "Description")
                .rows(6)
                .value(
                    "\nA blank first line, kept.\n\nA second paragraph, and a <script> tag that is text.",
                )
        })
    };
    form_column(fields)
}

fn error_state() -> Markup {
    let fields = html! {
        ({
            textarea("description", "Description")
                .hint("At least one language.")
                .error("Enter a description before submitting.")
                .required()
        })
        ({
            textarea("valid", "Provenance")
                .value("Digitised from the institute's slide archive.")
        })
    };
    form_column(fields)
}

fn capped() -> Markup {
    let fields = html! {
        ({
            textarea("short_description", "Short description")
                .rows(2)
                .maxlength(200)
                .hint("One line for cards and listings. Up to 200 characters.")
                .with_test_id("short-description-input")
        })
    };
    form_column(fields)
}
