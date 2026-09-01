//! Table showcase.

use maud::{html, Markup};
use mosaic_tiles::link::link;
use mosaic_tiles::table::{table, table_cell, table_cell_with_class, table_head_cell};

use super::{example, page_header, page_layout};

pub fn page() -> Markup {
    let header = page_header(
        "Table",
        "A data table with a required caption — its accessible name and the name of its scroll region — inside a \
         horizontally scrollable wrapper that a keyboard can reach.",
    );
    page_layout(header, examples())
}

fn examples() -> Markup {
    html! {
        ({
            example(
                "table-basic",
                "Columns and rows",
                "Head cells declare scope=\"col\"; the caption is visually hidden but announced.",
                basic(),
            )
        })
        ({
            example(
                "table-cell_classes",
                "Per-column cell classes",
                "A monospace address column and a controls column that must not wrap.",
                cell_classes(),
            )
        })
        ({
            example(
                "table-overflow",
                "Wider than its column",
                "The wrapper scrolls and carries tabindex=\"0\", so the clipped columns are reachable by keyboard \
                 in Safari too. Tab to the table, then use the arrow keys.",
                overflow(),
            )
        })
    }
}

struct Row {
    name: &'static str,
    email: &'static str,
    role: &'static str,
}

const ROWS: [Row; 3] = [
    Row {
        name: "A Depositor",
        email: "a.depositor@example.test",
        role: "depositor",
    },
    Row {
        name: "Another Depositor",
        email: "another@example.test",
        role: "depositor",
    },
    Row { name: "An Admin", email: "rdu@example.test", role: "rdu" },
];

fn basic() -> Markup {
    let head = html! {
        tr { (table_head_cell("Name")) (table_head_cell("Email")) (table_head_cell("Role")) }
    };
    let body = html! {
        @for row in &ROWS {
            tr { (table_cell(row.name)) (table_cell(row.email)) (table_cell(row.role)) }
        }
    };
    html! {
        (table("Accounts").head(head).body(body))
    }
}

fn cell_classes() -> Markup {
    let head = html! {
        tr {
            (table_head_cell("Name"))
            (table_head_cell("Email"))
            th class="data-table-head-cell" scope="col" {
                span class="sr-only" { "Actions" }
            }
        }
    };
    let body = html! {
        @for row in &ROWS {
            tr {
                (table_cell(row.name))
                (table_cell_with_class("font-mono text-sm", row.email))
                (table_cell_with_class("whitespace-nowrap", controls()))
            }
        }
    };
    html! {
        (table("Accounts with controls").head(head).body(body))
    }
}

/// The controls column of [`cell_classes`].
fn controls() -> Markup {
    html! {
        div class="flex gap-3" { (link("Edit", "#")) (link("Remove", "#")) }
    }
}

fn overflow() -> Markup {
    let columns = [
        "Name",
        "Email",
        "Role",
        "Projects",
        "Last code sent",
        "Created",
        "Last signed in",
        "Drafts",
        "Submissions",
    ];
    let head = html! {
        tr {
            @for column in columns { (table_head_cell(column)) }
        }
    };
    let body = html! {
        @for row in &ROWS {
            tr {
                (table_cell(row.name))
                (table_cell_with_class("font-mono text-sm", row.email))
                (table_cell(row.role))
                (table_cell("0801, 080C, 080E"))
                (table_cell("2026-08-25 09:14 UTC"))
                (table_cell("2026-01-12"))
                (table_cell("2026-08-31 16:02 UTC"))
                (table_cell("2"))
                (table_cell("1"))
            }
        }
    };
    html! {
        div class="max-w-md" { (table("Accounts, in full").head(head).body(body)) }
    }
}
