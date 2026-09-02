//! Text field showcase.

use maud::{html, Markup};
use mosaic_tiles::text_field::{text_field, InputType};
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Text Field",
        "A label, a single-line input, and an optional hint, wired together from one field name so the label's \
         `for` and the hint's `aria-describedby` cannot drift apart.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "text_field-basic",
                "Label and input",
                "The name is given once; the id and the label's `for` are derived from it.",
                basic(),
            )
        })
        ({
            example(
                "text_field-with_hint",
                "With a hint",
                "Help text below the input, announced with it rather than left as loose prose beside it.",
                with_hint(),
            )
        })
        ({
            example(
                "text_field-types",
                "Input types",
                "Text and email. Arms are added when a screen needs one.",
                types(),
            )
        })
        ({
            example(
                "text_field-one_time_code",
                "One-time code",
                "One intent method sets autocomplete, inputmode, pattern and maxlength together — setting three \
                 of the four is a field that looks right and autofills nothing.",
                one_time_code(),
            )
        })
        ({
            example(
                "text_field-dates_and_years",
                "Dates and years",
                "`InputType::Date` is the native date control — it holds a full `YYYY-MM-DD` and renders empty \
                 for anything else, so a lesser precision or a placeholder value has to be handled before it \
                 gets here. `year` bundles the three attributes a four-digit year needs; it is deliberately \
                 not `type=\"number\"`, whose spinner changes the value on a stray scroll.",
                dates_and_years(),
            )
        })
        ({
            example(
                "text_field-prefilled",
                "Holding a rejected entry",
                "A rejected form comes back showing what was typed, including the part that was wrong.",
                prefilled(),
            )
        })
        ({
            example(
                "text_field-error",
                "Rejected, with the reason",
                "`error` sets `aria-invalid` and the description together, and the message keeps the hint \
                 rather than replacing it — the reader needs the rule as well as the complaint.",
                error_state(),
            )
        })
        ({
            example(
                "text_field-duplicate_names",
                "The same name twice on one page",
                "`with_id` moves the id, the label's `for` and the hint's `aria-describedby` together, so two \
                 forms collecting the same field do not collide.",
                duplicate_names(),
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
        (text_field("name", "Name").required())
        (text_field("role", "Role"))
    };
    form_column(fields)
}

fn with_hint() -> Markup {
    let hint = html! {
        "Separated by commas, for example "
        code class="font-mono" { "0801, 080C" }
        ". Leave empty to assign none."
    };
    form_column(html! {
        (text_field("shortcodes", "Project shortcodes").hint(hint))
    })
}

fn types() -> Markup {
    let fields = html! {
        (text_field("full_name", "Name").input_type(InputType::Text))
        ({
            text_field("email", "Email address")
                .input_type(InputType::Email)
                .autocomplete("email")
                .required()
        })
        ({
            text_field("other_email", "Somebody else's address")
                .input_type(InputType::Email)
                .autocomplete("off")
                .hint(
                    "Autofill is refused here: this is not the address of whoever is filling the form in.",
                )
        })
    };
    form_column(fields)
}

fn one_time_code() -> Markup {
    form_column(html! {
        (text_field("code", "Six-digit code").one_time_code(6).required())
    })
}

fn dates_and_years() -> Markup {
    let fields = html! {
        ({
            text_field("startDate", "Start date")
                .input_type(InputType::Date)
                .value("2016-08-01")
                .required()
        })
        ({
            text_field("endDate", "End date")
                .input_type(InputType::Date)
                .hint("Leave empty while the project is ongoing.")
        })
        ({
            text_field("dataPublicationYear", "Data publication year")
                .year()
                .value("2026")
                .hint("Four digits.")
        })
    };
    form_column(fields)
}

fn prefilled() -> Markup {
    let fields = html! {
        (text_field("depositor_name", "Name").value("A Depositor").required())
        (text_field("codes", "Project shortcodes").value("0801, nope!"))
    };
    form_column(fields)
}

fn error_state() -> Markup {
    let fields = html! {
        ({
            text_field("codes", "Project shortcodes")
                .value("0801, nope!")
                .hint("Separated by commas.")
                .error(
                    r#""nope!" is not a project shortcode. Shortcodes are letters and digits only."#,
                )
        })
        ({
            text_field("contact", "Email address")
                .input_type(InputType::Email)
                .error("Enter a valid email address.")
                .required()
        })
        ({
            text_field("valid", "Name")
                .value("A Depositor")
                .hint("Nothing wrong with this one.")
        })
    };
    form_column(fields)
}

fn duplicate_names() -> Markup {
    let fields = html! {
        ({
            text_field("email", "Email address")
                .with_id("invite-email")
                .hint("Where the invitation goes.")
        })
        ({
            text_field("email", "Email address")
                .with_id("contact-email")
                .hint("Where replies go.")
        })
    };
    form_column(fields)
}
