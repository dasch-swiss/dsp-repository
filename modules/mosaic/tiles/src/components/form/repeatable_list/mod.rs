//! Repeatable list tile: an ordered list of rows a reader can add to and remove
//! from, each row's fields supplied by the caller.
//!
//! `repeatable_list(name, legend, rows_action)` returns a
//! [`RepeatableListBuilder`]; add rows with [`RepeatableListBuilder::row`], then
//! splice it into `html!` (it implements [`Render`]) or call `.build()`.
//!
//! ## Rows are keyed, never indexed
//!
//! Each row carries an **opaque key** the server made when the row was created,
//! submitted as a repeated hidden `{name}.row` field. The alternative — naming a
//! row's fields `keywords[0]`, `keywords[1]` — breaks the moment a middle row
//! goes: the next submit carries indices 0 and 2, a sequence with a hole that
//! either errors or silently compacts, at which point per-row validation errors
//! keyed by index point at the **wrong rows**. It bites the no-JavaScript path
//! and the enhanced one equally.
//!
//! Order is not encoded at all. The hidden fields repeat in DOM order, which the
//! browser preserves for free, so the decoder reads the order off the body
//! rather than off a number that has to be kept in step with it.
//!
//! ## Add and remove are server round-trips
//!
//! Both controls are submit buttons carrying a `formaction`, so the **whole form
//! body is posted** and the server re-renders the list. Nothing is spliced
//! client-side. Three things follow, and all three are the point:
//!
//! - Nothing typed elsewhere in the form is lost when a row is added or removed, because the body
//!   went with the request.
//! - The server keeps owning form state, so a re-render is the only way rows change and there is no
//!   client-side array to disagree with it.
//! - It works with no JavaScript, because a submit button with a `formaction` is plain HTML.
//!
//! A `<button type="submit" name="…" value="…">` would be the other way to say
//! which row to remove. It is avoided deliberately: a form submitted
//! programmatically — `new FormData(form)` — does **not** include the submitting
//! button's name and value unless the submitter is passed explicitly, so the
//! action would arrive on the plain path and vanish on the enhanced one. In the
//! URL it is carried by both.

use maud::{html, Markup, Render};

use super::shell::group_shell;
use crate::builder::ComponentBuilder;
use crate::button::{button, ButtonVariant};

/// One row: its opaque key and the caller's fields for it.
struct Row {
    key: String,
    content: Markup,
}

/// Builder for a list of removable rows. Construct with [`repeatable_list`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct RepeatableListBuilder {
    name: String,
    legend: Markup,
    rows_action: String,
    rows: Vec<Row>,
    item_noun: String,
    add_label: Option<Markup>,
    empty_message: Option<Markup>,
    hint: Option<Markup>,
    error: Option<Markup>,
    id: Option<String>,
    test_id: Option<String>,
}

/// The noun a row is called when the caller does not say. Generic on purpose —
/// "Remove item 2" is worse than "Remove keyword 2" and better than "Remove",
/// which five rows repeat identically.
const DEFAULT_ITEM_NOUN: &str = "item";

/// Start a repeatable list for the field `name`, named by `legend`, whose rows
/// are added and removed under `rows_action`.
///
/// `rows_action` is a base URL: the add button posts to `{rows_action}/add` and
/// a row's remove button to `{rows_action}/{key}/remove`. It is a constructor
/// argument rather than an option because a list with no way to add or remove a
/// row is not this tile — that is a list.
pub fn repeatable_list(
    name: impl Into<String>,
    legend: impl Render,
    rows_action: impl Into<String>,
) -> RepeatableListBuilder {
    RepeatableListBuilder {
        name: name.into(),
        legend: legend.render(),
        rows_action: rows_action.into(),
        rows: Vec::new(),
        item_noun: DEFAULT_ITEM_NOUN.to_string(),
        add_label: None,
        empty_message: None,
        hint: None,
        error: None,
        id: None,
        test_id: None,
    }
}

impl RepeatableListBuilder {
    /// Add a row, in the order it should appear.
    ///
    /// `key` is the server's opaque identifier for the row — never an index. It
    /// is submitted as a hidden field so the server can tell which row each of
    /// the caller's fields belongs to.
    ///
    /// It also becomes a **path segment** of the row's remove URL, so it must be
    /// URL-safe: `[A-Za-z0-9_-]+`. A key holding `/`, `?` or `#` would silently
    /// retarget that button at another route. Every key this workspace mints
    /// satisfies that (`editor_core::form` enforces the same character set when
    /// reading one back), and the `debug_assert!` below catches a caller that
    /// does not in a test or a dev build.
    pub fn row(mut self, key: impl Into<String>, content: impl Render) -> Self {
        let key = key.into();
        debug_assert!(
            !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "a row key becomes a path segment of the remove URL, so it must be [A-Za-z0-9_-]+, got {key:?}"
        );
        self.rows.push(Row { key, content: content.render() });
        self
    }

    /// Name one row, for the remove buttons' accessible names: "Remove
    /// {noun} 2".
    pub fn item_noun(mut self, noun: impl Into<String>) -> Self {
        self.item_noun = noun.into();
        self
    }

    /// Set the add button's visible text (default "Add").
    pub fn add_label(mut self, label: impl Render) -> Self {
        self.add_label = Some(label.render());
        self
    }

    /// What to show in place of the list when there are no rows.
    ///
    /// Without one, an empty list renders as a legend and an add button with
    /// nothing between them, which reads as a section that failed to load rather
    /// than as one nobody has filled in.
    pub fn empty_message(mut self, message: impl Render) -> Self {
        self.empty_message = Some(message.render());
        self
    }

    /// Add help text below the list, announced with it via `aria-describedby`.
    pub fn hint(mut self, hint: impl Render) -> Self {
        self.hint = Some(hint.render());
        self
    }

    /// Mark the list invalid and say why.
    pub fn error(mut self, message: impl Render) -> Self {
        self.error = Some(message.render());
        self
    }

    /// The id the legend, hint and error hang off.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let row_field = format!("{}.row", self.name);
        let add_label = self.add_label.clone().unwrap_or_else(|| {
            html! {
                "Add"
            }
        });
        let body = html! {
            @if self.rows.is_empty() {
                // The empty marker. A list with no rows would otherwise post no
                // `{name}.row` at all, and a decoder cannot tell "the depositor
                // removed the last row" from "this section did not carry the
                // field" — so the last removal would not stick. Same reason a
                // checkbox group with nothing checked needs one: an absent name
                // and an empty one mean different things.
                input type="hidden" name=(row_field) value="";
                @if let Some(message) = &self.empty_message {
                    p class="repeatable-empty" { (message) }
                }
            } @else {
                ol class="repeatable-rows" data-testid=[self.test_id.as_deref()] {
                    @for (position, row) in self.rows.iter().enumerate() {
                        li class="repeatable-row" {
                            // The row's identity, in DOM order. Order is read off
                            // the repetition of this field, never from a number.
                            input type="hidden" name=(row_field) value=(row.key);
                            div class="repeatable-row-body" { (row.content) }
                            ({
                                button("Remove")
                                    .variant(ButtonVariant::Secondary)
                                    .class("repeatable-remove")
                                    .form_action(
                                        format!("{}/{}/remove", self.rows_action, row.key),
                                    )
                                    .aria_label(
                                        format!("Remove {} {}", self.item_noun, position + 1),
                                    )
                            })
                        }
                    }
                }
            }
            ({
                button(add_label)
                    .variant(ButtonVariant::Secondary)
                    .class("repeatable-add")
                    .form_action(format!("{}/add", self.rows_action))
            })
        };
        group_shell(id, &self.legend, self.hint.as_ref(), self.error.as_ref(), body)
    }
}

impl ComponentBuilder for RepeatableListBuilder {
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

impl Render for RepeatableListBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTION: &str = "/projects/0801/sections/dataset/rows/keywords";

    fn keywords() -> RepeatableListBuilder {
        repeatable_list("keywords", "Keywords", ACTION)
            .item_noun("keyword")
            .add_label("Add a keyword")
            .row(
                "k7f3",
                html! {
                    input name="keywords.k7f3.en" value="manuscripts";
                },
            )
            .row(
                "k2b9",
                html! {
                    input name="keywords.k2b9.en" value="palaeography";
                },
            )
    }

    #[test]
    fn each_row_submits_its_own_opaque_key_in_dom_order() {
        // Order is read off the repetition of this field, so it must appear once
        // per row and in the rows' own order.
        let out = keywords().build().into_string();
        let first = out.find(r#"name="keywords.row" value="k7f3""#).expect("first row key");
        let second = out.find(r#"name="keywords.row" value="k2b9""#).expect("second row key");
        assert!(first < second, "{out}");
        assert_eq!(out.matches(r#"name="keywords.row""#).count(), 2, "{out}");
    }

    #[test]
    fn no_row_field_carries_an_index() {
        // Removing a middle row would leave indices 0 and 2 — a hole that either
        // errors or compacts, after which per-row errors point at wrong rows.
        let out = keywords().build().into_string();
        assert!(out.contains("keywords.k7f3.en"), "{out}");
        assert!(!out.contains("keywords[0]"), "{out}");
        assert!(!out.contains("keywords.0."), "{out}");
    }

    #[test]
    fn removing_a_row_posts_to_that_row_s_key_not_its_position() {
        let out = keywords().build().into_string();
        assert!(out.contains(&format!(r#"formaction="{ACTION}/k7f3/remove""#)), "{out}");
        assert!(out.contains(&format!(r#"formaction="{ACTION}/k2b9/remove""#)), "{out}");
    }

    #[test]
    fn adding_a_row_posts_to_the_list_s_add_url() {
        let out = keywords().build().into_string();
        assert!(out.contains(&format!(r#"formaction="{ACTION}/add""#)), "{out}");
        assert!(out.contains("Add a keyword"), "{out}");
    }

    #[test]
    fn both_controls_are_submit_buttons_so_the_whole_body_travels_with_them() {
        // The reason nothing typed elsewhere is lost when a row is added: the
        // form body is posted, and only the destination differs.
        let out = keywords().build().into_string();
        assert_eq!(out.matches(r#"type="submit""#).count(), 3, "{out}");
        assert_eq!(out.matches(r#"formmethod="post""#).count(), 3, "{out}");
        assert!(!out.contains("<a "), "{out}");
    }

    #[test]
    fn every_remove_button_says_which_row_it_removes() {
        // "Remove, button" twice tells a screen-reader user nothing about which
        // is which. Positions are 1-based and human-facing — an index in a
        // *label* is fine; an index in a *name* is the trap above.
        let out = keywords().build().into_string();
        assert!(out.contains(r#"aria-label="Remove keyword 1""#), "{out}");
        assert!(out.contains(r#"aria-label="Remove keyword 2""#), "{out}");
    }

    #[test]
    fn a_list_with_no_rows_says_so_rather_than_rendering_an_empty_box() {
        let out = repeatable_list("keywords", "Keywords", ACTION)
            .empty_message("No keywords yet.")
            .add_label("Add a keyword")
            .build()
            .into_string();
        assert!(out.contains("No keywords yet."), "{out}");
        assert!(!out.contains("<ol"), "{out}");
        // The way out of the empty state is still there.
        assert!(out.contains(&format!(r#"formaction="{ACTION}/add""#)), "{out}");
    }

    #[test]
    fn an_empty_list_still_posts_its_field_name_so_the_last_removal_sticks() {
        // Without the marker a list with no rows posts no `{name}.row` at all,
        // and a decoder cannot tell "the depositor removed the last row" from
        // "this section did not carry the field" — so the removal would not
        // stick. `editor_core::form::apply_multilingual_rows` reads exactly
        // this shape to clear a field.
        let out = repeatable_list("keywords", "Keywords", ACTION)
            .empty_message("No keywords yet.")
            .build()
            .into_string();
        assert!(out.contains(r#"<input type="hidden" name="keywords.row" value="">"#), "{out}");
    }

    #[test]
    fn a_populated_list_posts_no_empty_marker_beside_its_rows() {
        // The marker is the empty case only; an extra empty value among real
        // row keys would be a phantom row for the decoder to filter.
        let out = keywords().build().into_string();
        assert!(!out.contains(r#"name="keywords.row" value="">"#), "{out}");
        assert_eq!(out.matches(r#"name="keywords.row""#).count(), 2, "{out}");
    }

    #[test]
    fn the_rows_are_an_ordered_list_because_their_order_is_the_data() {
        let out = keywords().build().into_string();
        assert!(out.contains(r#"<ol class="repeatable-rows""#), "{out}");
        assert_eq!(out.matches(r#"<li class="repeatable-row">"#).count(), 2, "{out}");
    }

    #[test]
    fn the_caller_s_fields_are_rendered_inside_their_row() {
        let out = keywords().build().into_string();
        let row_start = out.find(r#"value="k7f3""#).expect("row key");
        let field = out.find("keywords.k7f3.en").expect("row field");
        let next_row = out.find(r#"value="k2b9""#).expect("next row key");
        assert!(row_start < field && field < next_row, "{out}");
    }

    #[test]
    fn an_error_is_described_by_the_fieldset_and_announced_once() {
        let out = keywords()
            .hint("Each keyword is one multilingual term.")
            .error("Add at least one keyword.")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-describedby="keywords-hint keywords-error""#), "{out}");
        assert!(out.contains(r#"data-invalid="true""#), "{out}");
        assert_eq!(out.matches("Add at least one keyword.").count(), 1, "{out}");
    }

    #[test]
    fn the_error_region_is_rendered_empty_when_valid() {
        let out = keywords().build().into_string();
        assert!(
            out.contains(r#"<p class="field-error" id="keywords-error" aria-live="polite"></p>"#),
            "{out}"
        );
        assert!(!out.contains("data-invalid"), "{out}");
    }

    #[test]
    fn the_default_item_noun_still_distinguishes_the_rows() {
        let out = repeatable_list("things", "Things", ACTION)
            .row("a", html! {})
            .row("b", html! {})
            .build()
            .into_string();
        assert!(out.contains(r#"aria-label="Remove item 1""#), "{out}");
        assert!(out.contains(r#"aria-label="Remove item 2""#), "{out}");
    }

    #[test]
    fn an_explicit_id_moves_the_hint_and_error_but_not_the_submitted_names() {
        let out = keywords().with_id("dataset-keywords").build().into_string();
        assert!(out.contains(r#"id="dataset-keywords-error""#), "{out}");
        assert!(out.contains(r#"name="keywords.row""#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = keywords().build().into_string();
        let spliced = html! {
            (keywords())
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
