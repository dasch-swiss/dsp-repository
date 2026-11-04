//! Combobox showcase.
//!
//! Static renders only: the playground loads no Datastar script, so typing here
//! searches nothing. Arrow keys and Escape still work, because that script is
//! inline.

use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonVariant};
use mosaic_tiles::combobox::{combobox, combobox_groups, combobox_listbox, combobox_option};
use mosaic_tiles::ComponentBuilder;

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Combobox",
        "A search input with a listbox of results the server renders. Arrow keys move focus between the options, \
         and each option is a submit button, so choosing one is an ordinary form post.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "combobox-empty",
                "Empty",
                "No results are showing, so the results container is empty and takes no space. Its id is derived \
                 from the field name.",
                empty(),
            )
        })
        ({
            example(
                "combobox-results",
                "With results",
                "The results are a listbox of options. ArrowDown from the input focuses the first option, \
                 ArrowUp from the first returns to the input, and Escape closes the list.",
                with_results(),
            )
        })
        ({
            example(
                "combobox-grouped",
                "Grouped results",
                "`combobox_groups` puts the options under group headings. The arrow keys still move through every \
                 option in order, across groups.",
                grouped(),
            )
        })
        ({
            example(
                "combobox-no_match",
                "No match",
                "Anything can fill the results container, so a miss can offer a way forward instead of an empty list.",
                no_match(),
            )
        })
        ({
            example(
                "combobox-results_id",
                "A separate results id",
                "`.results_id()` names the results container on its own, so the input's id can change on a response \
                 (which makes the morph replace it and drop what was typed) while the container keeps the id the \
                 search endpoint answers for.",
                separate_results_id(),
            )
        })
        ({
            example(
                "combobox-label_hidden",
                "A hidden label",
                "`.label_hidden()` keeps the label for a screen reader and takes it out of the layout, for a field \
                 a surrounding heading already names.",
                label_hidden(),
            )
        })
    }
}

fn form_column(content: Markup) -> Markup {
    html! {
        form class="max-w-md" method="post" action="#" { (content) }
    }
}

fn search_button() -> Markup {
    html! {
        (button("Search").variant(ButtonVariant::Secondary))
    }
}

fn empty() -> Markup {
    let field = combobox("person_empty", "Person")
        .hint("Type a name to search.")
        .search_button(search_button());
    form_column(html! {
        (field)
    })
}

fn person(name: &str, detail: &str, id: &str) -> Markup {
    let content = html! {
        span class="block font-bold" { (name) }
        span class="block text-sm text-neutral-600" { (detail) }
    };
    combobox_option(content, "person", id)
        .aria_label(format!("{name}, {detail}"))
        .build()
}

fn with_results() -> Markup {
    let listbox = combobox_listbox(
        "People",
        [
            person("Ada Lovelace", "University of London", "p1"),
            person("Alan Turing", "University of Manchester", "p2"),
            person("Grace Hopper", "Yale University", "p3"),
        ],
    );
    let field = combobox("person_results", "Person")
        .value("a")
        .search_button(search_button())
        .results(listbox)
        .expanded(true);
    form_column(html! {
        (field)
    })
}

fn grouped() -> Markup {
    let groups = [
        (
            "Humanities".to_string(),
            vec![
                person("Ada Lovelace", "University of London", "p1"),
                person("Alan Turing", "University of Manchester", "p2"),
            ],
        ),
        (
            "Sciences".to_string(),
            vec![
                person("Grace Hopper", "Yale University", "p3"),
                person("Hedy Lamarr", "University of Vienna", "p4"),
            ],
        ),
    ];
    let listbox = combobox_groups("People by field", groups);
    let field = combobox("person_grouped", "Person")
        .value("a")
        .search_button(search_button())
        .results(listbox)
        .expanded(true);
    form_column(html! {
        (field)
    })
}

fn no_match() -> Markup {
    let message = html! {
        div class="flex flex-col items-start gap-2 p-3" {
            p { "Nobody matches \u{201c}zzz\u{201d}." }
            (button("Add as a new person").variant(ButtonVariant::Secondary))
        }
    };
    let field = combobox("person_none", "Person")
        .value("zzz")
        .search_button(search_button())
        .results(message)
        .expanded(true);
    form_column(html! {
        (field)
    })
}

fn separate_results_id() -> Markup {
    let field = combobox("person_stable", "Person")
        .with_id("person_stable-3f9a")
        .results_id("person_stable-results")
        .search_button(search_button());
    form_column(html! {
        (field)
    })
}

fn label_hidden() -> Markup {
    let field = combobox("person_hidden", "Person")
        .label_hidden()
        .search_button(search_button());
    form_column(html! {
        (field)
    })
}
