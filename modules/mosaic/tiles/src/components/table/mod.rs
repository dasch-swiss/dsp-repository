//! Table tile: a horizontally scrollable data table with a required caption,
//! plus head-cell and cell partials.
//!
//! `table(caption)` returns a [`TableBuilder`]; set the head and body rows with
//! chained methods and either splice it into `html!` directly (it implements
//! [`Render`]) or call `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.
//!
//! ## Why the caption is required, and a `String` rather than `impl Render`
//!
//! The caption does two jobs, and both need it to exist. It is the table's
//! accessible name, rendered as a visually hidden `<caption>` — a table
//! announced as "table with 6 columns" and nothing else leaves a screen reader
//! user to infer what it lists from the first row. And it names the scroll
//! region described below, which as a `role="region"` must have a name or it is
//! worse than no landmark at all.
//!
//! It is `impl Into<String>` rather than the `impl Render` the other tiles take
//! because it is reused verbatim as an `aria-label`, and an icon or nested
//! markup in an accessible name means nothing.
//!
//! ## Why the scroll wrapper is focusable
//!
//! A table wider than its column scrolls horizontally, and a scroll container is
//! not reachable by keyboard on its own everywhere. Measured on an empty
//! scroller with no `tabindex`, Chromium focuses it; WebKit and Firefox do not.
//! So the wrapper carries `tabindex="0"` — with it, arrow keys scroll the region
//! in all three — and, having a tab stop, `role="region"` named by the caption,
//! so what the reader lands on announces itself rather than being an unlabelled
//! stop. The cost is one tab stop per table even when it does not overflow; the
//! alternative is columns a keyboard user cannot reach at all in two engines of
//! three.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

/// Builder for a data table. Construct with [`table`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct TableBuilder {
    caption: String,
    head: Markup,
    body: Markup,
    extra_classes: String,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a table named `caption` — its accessible name, and the name of its
/// scroll region. See the module docs for why it is required.
pub fn table(caption: impl Into<String>) -> TableBuilder {
    TableBuilder {
        caption: caption.into(),
        head: Markup::default(),
        body: Markup::default(),
        extra_classes: String::new(),
        id: None,
        test_id: None,
    }
}

impl TableBuilder {
    /// Set the `<thead>` row(s) — typically [`table_head_cell`]s inside a `tr`.
    pub fn head(mut self, head: impl Render) -> Self {
        self.head = head.render();
        self
    }

    /// Set the `<tbody>` rows — typically [`table_cell`]s inside `tr`s.
    pub fn body(mut self, body: impl Render) -> Self {
        self.body = body.render();
        self
    }

    /// Append extra utility classes to the `<table>` element.
    pub fn class(mut self, classes: impl Into<String>) -> Self {
        self.extra_classes = classes.into();
        self
    }

    fn markup(&self) -> Markup {
        let class = if self.extra_classes.is_empty() {
            "data-table".to_string()
        } else {
            format!("data-table {}", self.extra_classes)
        };
        html! {
            div class="data-table-scroll" role="region" aria-label=(self.caption) tabindex="0" {
                table class=(class) id=[self.id.as_deref()] data-testid=[self.test_id.as_deref()] {
                    caption class="sr-only" { (self.caption) }
                    thead { (self.head) }
                    tbody { (self.body) }
                }
            }
        }
    }
}

impl ComponentBuilder for TableBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        self.markup()
    }
}

impl Render for TableBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

/// Render one column header.
///
/// `scope="col"` is the tile's job rather than the caller's: without it a screen
/// reader has to guess which cells a header governs, and the guess is only
/// reliable for the simplest tables.
#[must_use]
pub fn table_head_cell(content: impl Render) -> Markup {
    html! {
        th class="data-table-head-cell" scope="col" { (content) }
    }
}

/// Render one body cell.
#[must_use]
pub fn table_cell(content: impl Render) -> Markup {
    html! {
        td class="data-table-cell" { (content) }
    }
}

/// Render one body cell with extra classes on the cell itself (e.g. `font-mono`
/// for an address column, or `whitespace-nowrap` for a column of controls).
#[must_use]
pub fn table_cell_with_class(class: &str, content: impl Render) -> Markup {
    html! {
        td class={ "data-table-cell " (class) } { (content) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caption_names_both_the_table_and_its_scroll_region() {
        // A `role="region"` without a name is worse than no landmark, and a
        // table with no caption announces its column count and nothing else.
        let out = table("Accounts").build().into_string();
        assert!(out.contains(r#"aria-label="Accounts""#), "{out}");
        assert!(out.contains(r#"<caption class="sr-only">Accounts</caption>"#), "{out}");
    }

    #[test]
    fn the_scroll_wrapper_is_reachable_by_keyboard() {
        // Measured with Playwright: an untabindexed scroller is focusable in
        // Chromium but not in WebKit or Firefox, and with `tabindex="0"` the
        // arrow keys scroll the region in all three.
        let out = table("Accounts").build().into_string();
        assert!(out.contains(r#"class="data-table-scroll""#), "{out}");
        assert!(out.contains(r#"role="region""#), "{out}");
        assert!(out.contains(r#"tabindex="0""#), "{out}");
    }

    #[test]
    fn head_and_body_land_in_their_own_sections() {
        let head = html! {
            tr { (table_head_cell("Name")) }
        };
        let body = html! {
            tr { (table_cell("A Depositor")) }
        };
        let out = table("Accounts").head(head).body(body).build().into_string();
        let thead_at = out.find("<thead>").expect("thead missing");
        let tbody_at = out.find("<tbody>").expect("tbody missing");
        assert!(thead_at < tbody_at, "{out}");
        assert!(out[thead_at..tbody_at].contains("Name"), "head content in thead: {out}");
        assert!(out[tbody_at..].contains("A Depositor"), "body content in tbody: {out}");
    }

    #[test]
    fn a_head_cell_declares_what_it_governs() {
        let out = table_head_cell("Email").into_string();
        assert!(out.contains(r#"<th class="data-table-head-cell" scope="col">"#), "{out}");
    }

    #[test]
    fn a_cell_can_carry_extra_classes_without_losing_the_base_one() {
        let out = table_cell_with_class("font-mono text-sm", "a@example.test").into_string();
        assert!(out.contains(r#"class="data-table-cell font-mono text-sm""#), "{out}");
    }

    #[test]
    fn cells_accept_markup() {
        let out = table_cell(html! {
            span class="text-neutral-500" { "—" }
        })
        .into_string();
        assert!(out.contains(r#"<span class="text-neutral-500">—</span>"#), "{out}");
    }

    #[test]
    fn an_empty_table_still_renders_its_sections() {
        // A table built with no rows is a caller bug, not a render failure; the
        // shell has to be stable so the caller can see what it produced.
        let out = table("Accounts").build().into_string();
        assert!(out.contains("<thead></thead>"), "{out}");
        assert!(out.contains("<tbody></tbody>"), "{out}");
    }

    #[test]
    fn extra_classes_follow_the_base_class() {
        let out = table("Accounts").class("text-sm").build().into_string();
        assert!(out.contains(r#"class="data-table text-sm""#), "{out}");
    }

    #[test]
    fn id_and_test_id_land_on_the_table_not_the_wrapper() {
        let out = table("Accounts")
            .with_id("accounts")
            .with_test_id("accounts-table")
            .build()
            .into_string();
        let table_at = out.find("<table").expect("table missing");
        assert!(out[table_at..].contains(r#"id="accounts""#), "{out}");
        assert!(out[table_at..].contains(r#"data-testid="accounts-table""#), "{out}");
    }

    #[test]
    fn the_caption_is_escaped() {
        let out = table("<script>alert(1)</script>").build().into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = table("Accounts").build().into_string();
        let spliced = html! {
            (table("Accounts"))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
