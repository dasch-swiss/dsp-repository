//! Textarea tile: a label, a multi-line `<textarea>`, and an optional hint and
//! error, wired together as one group.
//!
//! `textarea(name, label)` returns a [`TextareaBuilder`]; set options with
//! chained methods and either splice it into `html!` directly (it implements
//! [`Render`]) or call `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.
//!
//! Shares [`FieldShell`](super::shell::FieldShell) and the `field-*` classes
//! with [`text_field`](super::text_field), so a multi-line field is the same
//! control with a different box rather than a second look.
//!
//! ## The leading newline
//!
//! A `<textarea>` holds its value as element content, not in an attribute, and
//! the HTML parser **drops a newline immediately after the start tag**
//! (§13.2.5, "A single U+000A LINE FEED character may be placed immediately
//! after the start tag"). A value that itself begins with a newline therefore
//! comes back one newline shorter than it went out. That is invisible until a
//! depositor opens a description that starts with a blank line, saves, and finds
//! the blank line gone — and it compounds, because every save loses another one.
//! [`TextareaBuilder::markup`] emits an extra newline in that case, which the
//! parser then eats, leaving the value intact.

use maud::{html, Markup, PreEscaped, Render};

use super::shell::FieldShell;
use crate::builder::ComponentBuilder;

/// The default visible height, in lines. Enough to read as multi-line rather
/// than as a tall single-line input, without a short field claiming the screen.
const DEFAULT_ROWS: u32 = 3;

/// Builder for a labelled multi-line input. Construct with [`textarea`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct TextareaBuilder {
    name: String,
    label: Markup,
    value: Option<String>,
    hint: Option<Markup>,
    error: Option<Markup>,
    rows: u32,
    maxlength: Option<u32>,
    required: bool,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a textarea posting under `name`, labelled `label`.
///
/// `name` is both the submitted field name and — unless [`ComponentBuilder::with_id`]
/// overrides it — the `id` the label, hint and error point at.
pub fn textarea(name: impl Into<String>, label: impl Render) -> TextareaBuilder {
    TextareaBuilder {
        name: name.into(),
        label: label.render(),
        value: None,
        hint: None,
        error: None,
        rows: DEFAULT_ROWS,
        maxlength: None,
        required: false,
        id: None,
        test_id: None,
    }
}

impl TextareaBuilder {
    /// Pre-fill the textarea, so a rejected form comes back holding what was
    /// typed.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Add help text below the control, announced with it via `aria-describedby`.
    pub fn hint(mut self, hint: impl Render) -> Self {
        self.hint = Some(hint.render());
        self
    }

    /// Mark the field invalid and say why. See [`super::text_field::TextFieldBuilder::error`].
    pub fn error(mut self, message: impl Render) -> Self {
        self.error = Some(message.render());
        self
    }

    /// Set the visible height in lines (default 3).
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }

    /// Cap the entry length.
    ///
    /// A `maxlength` on a textarea is enforced by the browser on typing but not
    /// on a paste in every engine, and never on a request that did not come from
    /// the form — so it is an affordance, and the server still has to check.
    pub fn maxlength(mut self, maxlength: u32) -> Self {
        self.maxlength = Some(maxlength);
        self
    }

    /// Mark the control required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// The id the label, hint and error point at: the explicit one if set, else
    /// the name.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let shell = FieldShell {
            id,
            label: &self.label,
            hint: self.hint.as_ref(),
            error: self.error.as_ref(),
        };
        let described_by = shell.described_by();
        // See the module docs: the parser eats one newline after the start tag,
        // so a value beginning with one needs a spare in front of it. The spare
        // is `PreEscaped` because a newline has no escaped form; the value
        // itself still goes through `Render` and is escaped.
        let leading_newline = self.value.as_deref().is_some_and(|v| v.starts_with('\n'));
        let control = html! {
            textarea
                class="field-input field-textarea"
                id=(id)
                name=(self.name)
                rows=(self.rows)
                maxlength=[self.maxlength]
                aria-describedby=[described_by.as_deref()]
                aria-invalid=[shell.aria_invalid()]
                data-testid=[self.test_id.as_deref()]
                required[self.required]
            {
                @if leading_newline { (PreEscaped("\n")) }
                @if let Some(value) = &self.value { (value) }
            }
        };
        shell.render(control)
    }
}

impl ComponentBuilder for TextareaBuilder {
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

impl Render for TextareaBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `<textarea>`'s element content, which is where its value lives.
    fn content_of_textarea(out: &str) -> &str {
        let after_tag = out
            .split_once("<textarea")
            .and_then(|(_, rest)| rest.split_once('>'))
            .expect("a textarea with a closed start tag")
            .1;
        after_tag.split_once("</textarea>").expect("a closed textarea").0
    }

    #[test]
    fn the_label_points_at_the_control_it_labels() {
        let out = textarea("description", "Description").build().into_string();
        assert!(
            out.contains(r#"<label class="field-label" for="description">Description</label>"#),
            "{out}"
        );
        assert!(out.contains(r#"id="description""#), "{out}");
        assert!(out.contains(r#"name="description""#), "{out}");
    }

    #[test]
    fn the_value_is_element_content_not_an_attribute() {
        // The trap the tile exists to get right: `value="…"` on a textarea is
        // silently ignored by the browser, and the field renders empty.
        let out = textarea("description", "Description").value("A project.").build().into_string();
        assert!(out.contains(">A project.</textarea>"), "{out}");
        assert!(!out.contains("value="), "{out}");
    }

    #[test]
    fn a_value_beginning_with_a_newline_survives_the_parser() {
        // The parser drops one newline after the start tag, so the value needs a
        // spare in front of it or a description opening with a blank line loses
        // it on every save.
        let out = textarea("description", "Description")
            .value("\nIndented.")
            .build()
            .into_string();
        let content = content_of_textarea(&out);
        assert!(content.starts_with("\n\n"), "{content:?}");
        assert_eq!(content, "\n\nIndented.");
    }

    #[test]
    fn a_value_not_beginning_with_a_newline_gains_nothing() {
        let out = textarea("description", "Description").value("Plain.").build().into_string();
        assert_eq!(content_of_textarea(&out), "Plain.");
    }

    #[test]
    fn the_value_is_escaped() {
        // It is element content, so an unescaped `</textarea>` in a stored value
        // would close the control and inject markup into the page.
        let out = textarea("description", "Description")
            .value("</textarea><script>alert(1)</script>")
            .build()
            .into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;/textarea&gt;"), "{out}");
    }

    #[test]
    fn rows_defaults_to_three_and_is_settable() {
        assert!(textarea("d", "D").build().into_string().contains(r#"rows="3""#));
        assert!(textarea("d", "D").rows(8).build().into_string().contains(r#"rows="8""#));
    }

    #[test]
    fn an_error_marks_the_control_invalid_and_describes_it_by_the_message() {
        let out = textarea("description", "Description")
            .hint("At least one language.")
            .error("Enter a description.")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-invalid="true""#), "{out}");
        assert!(
            out.contains(r#"aria-describedby="description-hint description-error""#),
            "{out}"
        );
        assert!(out.contains("Enter a description."), "{out}");
    }

    #[test]
    fn the_error_region_is_rendered_empty_when_valid() {
        let out = textarea("description", "Description").build().into_string();
        assert!(
            out.contains(r#"<p class="field-error" id="description-error" aria-live="polite"></p>"#),
            "{out}"
        );
        assert!(!out.contains("aria-invalid"), "{out}");
    }

    #[test]
    fn omits_optional_attributes_when_unset() {
        let out = textarea("d", "D").build().into_string();
        assert!(!out.contains("maxlength="), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("data-testid="), "{out}");
        assert!(!out.contains("aria-describedby"), "{out}");
    }

    #[test]
    fn maxlength_and_required_render_when_set() {
        let out = textarea("short", "Short description")
            .maxlength(200)
            .required()
            .build()
            .into_string();
        assert!(out.contains(r#"maxlength="200""#), "{out}");
        assert!(out.contains("required"), "{out}");
    }

    #[test]
    fn an_explicit_id_moves_the_label_hint_and_error_with_it() {
        let out = textarea("note", "Note")
            .with_id("reviewer-note")
            .hint("Seen by the depositor.")
            .error("Say something.")
            .build()
            .into_string();
        assert!(out.contains(r#"for="reviewer-note""#), "{out}");
        assert!(out.contains(r#"id="reviewer-note-hint""#), "{out}");
        assert!(out.contains(r#"id="reviewer-note-error""#), "{out}");
        // The submitted name is unchanged — only the id moved.
        assert!(out.contains(r#"name="note""#), "{out}");
    }

    #[test]
    fn test_id_lands_on_the_control_not_the_wrapper() {
        let out = textarea("d", "D").with_test_id("description-input").build().into_string();
        let testid_at = out.find("data-testid").expect("test id missing");
        let control_at = out.find("<textarea").expect("textarea missing");
        assert!(control_at < testid_at, "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = textarea("d", "D").required().build().into_string();
        let spliced = html! {
            (textarea("d", "D").required())
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
