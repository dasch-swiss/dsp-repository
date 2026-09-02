//! Select showcase.

use maud::{html, Markup};
use mosaic_tiles::select::select;
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Select",
        "A label, a single-choice dropdown, and an optional hint and error — the same `field-*` shell as the \
         text field. There is no multiple-choice mode: several values is a checkbox group.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "select-basic",
                "Label and choices",
                "Choices render in the order they were added.",
                basic(),
            )
        })
        ({
            example(
                "select-placeholder",
                "Nothing chosen yet",
                "Without a placeholder a select shows its first choice as though it had been picked — which is \
                 how an untouched field acquires a value nobody entered. The placeholder posts an empty value, \
                 and is never `disabled`: a disabled empty choice cannot be selected back, so an optional \
                 field becomes a one-way door.",
                placeholder(),
            )
        })
        ({
            example(
                "select-selected",
                "Holding a value",
                "A stored value that is no longer on offer leaves the placeholder showing rather than silently \
                 reading as the first choice.",
                selected(),
            )
        })
        ({
            example(
                "select-error",
                "Rejected, with the reason",
                "`error` sets `aria-invalid` and the description together, and keeps the hint.",
                error_state(),
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
        ({
            select("status", "Status")
                .option("ongoing", "Ongoing")
                .option("finished", "Finished")
                .hint("Whether the project is ongoing or finished.")
        })
    };
    form_column(fields)
}

/// The four access-rights values as the contract spells them.
fn access_rights() -> [(&'static str, &'static str); 4] {
    [
        ("Full Open Access", "Full Open Access"),
        ("Open Access with Restrictions", "Open Access with Restrictions"),
        ("Embargoed Access", "Embargoed Access"),
        ("Metadata only Access", "Metadata only Access"),
    ]
}

fn placeholder() -> Markup {
    let fields = html! {
        ({
            select("access_rights", "Access rights")
                .placeholder("Select access rights…")
                .options(access_rights())
                .required()
                .hint("How openly the data can be accessed.")
        })
    };
    form_column(fields)
}

fn selected() -> Markup {
    let fields = html! {
        ({
            select("known", "Access rights")
                .placeholder("Select access rights…")
                .options(access_rights())
                .selected("Embargoed Access")
                .hint("A value the contract still offers.")
        })
        ({
            select("retired", "Access rights")
                .placeholder("Select access rights…")
                .options(access_rights())
                .selected("Closed Access")
                .hint(
                    "A stored value the contract no longer offers — the placeholder stays showing.",
                )
        })
    };
    form_column(fields)
}

fn error_state() -> Markup {
    let fields = html! {
        ({
            select("rejected", "Access rights")
                .placeholder("Select access rights…")
                .options(access_rights())
                .hint("How openly the data can be accessed.")
                .error("Choose the access rights before submitting.")
                .required()
                .with_test_id("access-rights-select")
        })
    };
    form_column(fields)
}
