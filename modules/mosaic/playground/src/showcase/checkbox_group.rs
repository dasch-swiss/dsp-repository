//! Checkbox group showcase.

use maud::{html, Markup};
use mosaic_tiles::checkbox_group::checkbox_group;
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Checkbox Group",
        "A field whose value is a set: a legend, one checkbox per choice, and an optional hint and error. Every \
         checkbox posts under the group's name, so two checked choices send the name twice.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "checkbox_group-basic",
                "Legend and choices",
                "The group is named by a legend rather than a label with a `for` — a `for` needs one control to \
                 point at, and a group has one per choice. Each choice's id comes from its index, so a value \
                 carrying spaces or punctuation is still a valid id.",
                basic(),
            )
        })
        ({
            example(
                "checkbox_group-checked",
                "Holding a set",
                "`checked` marks the current values. A stored value the group does not offer adds no control: \
                 inventing one would let a value the contract no longer has ride back out through the form.",
                checked(),
            )
        })
        ({
            example(
                "checkbox_group-error",
                "Rejected, with the reason",
                "The message hangs off the fieldset and is announced once, not once per choice — `aria-invalid` \
                 is not valid on a fieldset and repeating it on every control would say the same thing three \
                 times. The left rule is always present and only changes colour, so nothing shifts.",
                error_state(),
            )
        })
    }
}

/// The showcase renders fields in a column, as a form would.
fn form_column(content: Markup) -> Markup {
    html! {
        div class="flex max-w-md flex-col gap-6" { (content) }
    }
}

/// The kinds of data the published set actually uses.
fn kinds() -> [(&'static str, &'static str); 5] {
    [
        ("Text", "Text"),
        ("Image", "Image"),
        ("Movie", "Movie"),
        ("XML", "XML"),
        ("Table", "Table"),
    ]
}

fn basic() -> Markup {
    let fields = html! {
        ({
            checkbox_group("typeOfData", "Type of data")
                .options(kinds())
                .hint("Every kind the dataset holds.")
        })
    };
    form_column(fields)
}

fn checked() -> Markup {
    let fields = html! {
        ({
            checkbox_group("known", "Type of data")
                .options(kinds())
                .checked(["Image", "Table"])
                .hint("Two of the five are current.")
        })
        ({
            checkbox_group("retired", "Type of data")
                .options(kinds())
                .checked(["Software"])
                .hint(
                    "A stored value the group does not offer — no control is invented for it.",
                )
        })
    };
    form_column(fields)
}

fn error_state() -> Markup {
    let fields = html! {
        ({
            checkbox_group("rejected", "Type of data")
                .options(kinds())
                .hint("Every kind the dataset holds.")
                .error("Pick at least one kind of data.")
                .with_test_id("type-of-data-group")
        })
        ({
            checkbox_group("dataLanguage", "Data languages")
                .options([
                    ("de", "German"),
                    ("en", "English"),
                    ("fr", "French"),
                    ("it", "Italian"),
                ])
                .checked(["de", "en"])
                .hint("Nothing wrong with this one.")
        })
    };
    form_column(fields)
}
