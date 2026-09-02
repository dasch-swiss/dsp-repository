//! Radio group showcase.

use maud::{html, Markup};
use mosaic_tiles::radio_group::radio_group;
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Radio Group",
        "A field with exactly one value, chosen from a short visible list. Use it for a discriminant — where \
         the choice changes which fields below apply, so the reader has to see the alternatives. A closed list \
         nobody needs to compare is a select.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "radio_group-inline",
                "A discriminant",
                "`inline` lays the choices out in a row, which reads as one question with alternatives where a \
                 column reads as a list of unrelated settings. Every radio shares the group's name — that is \
                 what makes them exclude each other; distinct names render as radios and behave as independent \
                 toggles.",
                inline(),
            )
        })
        ({
            example(
                "radio_group-column",
                "Choices that need explaining",
                "A column, where each choice's label runs longer than a word or two. The control stays beside \
                 the label's first line rather than centred against the whole block.",
                column(),
            )
        })
        ({
            example(
                "radio_group-error",
                "Rejected, with the reason",
                "As with the checkbox group, the message hangs off the fieldset and is announced once. Note a \
                 radio group cannot be returned to unset once a choice is made — that is the control's \
                 behaviour, so a field that may be empty needs an explicit \"none\" choice or a select.",
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

fn inline() -> Markup {
    let fields = html! {
        ({
            radio_group("funding.kind", "How is funding recorded?")
                .option("grants", "Grants")
                .option("text", "Free text")
                .selected("grants")
                .inline()
                .hint(
                    "Grants carry a funder, a number and a programme; free text is one line.",
                )
        })
    };
    form_column(fields)
}

fn column() -> Markup {
    let fields = html! {
        ({
            radio_group("temporalCoverage.k1.kind", "How is this period recorded?")
                .option(
                    "reference",
                    "An authority reference — a Chronontology or PeriodO entry, which resolves to a date range",
                )
                .option(
                    "text",
                    "Free text, one term per language — recorded as written, and not resolvable to a date",
                )
                .selected("reference")
        })
    };
    form_column(fields)
}

fn error_state() -> Markup {
    let fields = html! {
        ({
            radio_group("disciplines.k2.kind", "How is this discipline recorded?")
                .option("reference", "An authority reference")
                .option("text", "Free text, per language")
                .inline()
                .hint("Changes which fields below apply.")
                .error("Choose how the discipline is recorded.")
                .with_test_id("discipline-kind-group")
        })
    };
    form_column(fields)
}
