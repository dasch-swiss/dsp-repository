//! Select tile: a label, a `<select>` of one choice, and an optional hint and
//! error.
//!
//! `select(name, label)` returns a [`SelectBuilder`]; add choices with
//! [`SelectBuilder::option`], then splice it into `html!` directly (it
//! implements [`Render`]) or call `.build()`. See
//! `docs/src/mosaic/component-api-conventions.md`.
//!
//! Shares [`FieldShell`](super::shell::FieldShell) and the `field-*` classes
//! with the other form tiles.
//!
//! ## Single choice only
//!
//! There is no `multiple`. A `<select multiple>` is a well-known usability
//! failure — it gives no affordance that more than one item is selectable, and
//! selecting a second item without holding a modifier silently deselects the
//! first. A field that takes several values is a checkbox group, which says what
//! it does and needs no modifier key.
//!
//! ## The placeholder is never `disabled`
//!
//! `<option value="" selected disabled>` is the common recipe and it is wrong
//! here: a disabled option cannot be selected *back*, so a depositor who picks a
//! value for an optional field can never return it to unset. The placeholder
//! carries `value=""` instead, which is what makes `required` refuse a
//! submission that left the field alone — the browser treats the empty string as
//! no value.

use maud::{html, Markup, Render};

use super::shell::FieldShell;
use crate::builder::ComponentBuilder;

/// One choice: the value posted, and the text shown.
struct Choice {
    value: String,
    label: Markup,
}

/// Builder for a labelled single-choice select. Construct with [`select`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct SelectBuilder {
    name: String,
    label: Markup,
    choices: Vec<Choice>,
    placeholder: Option<Markup>,
    selected: Option<String>,
    hint: Option<Markup>,
    error: Option<Markup>,
    required: bool,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a select posting under `name`, labelled `label`.
///
/// `name` is both the submitted field name and — unless [`ComponentBuilder::with_id`]
/// overrides it — the `id` the label, hint and error point at.
pub fn select(name: impl Into<String>, label: impl Render) -> SelectBuilder {
    SelectBuilder {
        name: name.into(),
        label: label.render(),
        choices: Vec::new(),
        placeholder: None,
        selected: None,
        hint: None,
        error: None,
        required: false,
        id: None,
        test_id: None,
    }
}

impl SelectBuilder {
    /// Add a choice, in the order it should appear.
    pub fn option(mut self, value: impl Into<String>, label: impl Render) -> Self {
        self.choices.push(Choice { value: value.into(), label: label.render() });
        self
    }

    /// Add every choice from an iterator of `(value, label)` pairs.
    ///
    /// For the common case where the choices are a `const` list on the contract
    /// — `ACCESS_RIGHTS_VALUES` and friends — so a caller does not fold over
    /// them by hand.
    pub fn options<V, L>(mut self, choices: impl IntoIterator<Item = (V, L)>) -> Self
    where
        V: Into<String>,
        L: Render,
    {
        for (value, label) in choices {
            self = self.option(value, label);
        }
        self
    }

    /// Add the leading "nothing chosen yet" choice, posting an empty value.
    ///
    /// Without one, a `<select>` shows its first choice as though the depositor
    /// had picked it, which is how an untouched field acquires a value nobody
    /// entered.
    pub fn placeholder(mut self, label: impl Render) -> Self {
        self.placeholder = Some(label.render());
        self
    }

    /// Pre-select the choice with this value.
    ///
    /// A value matching no choice selects nothing, which leaves the placeholder
    /// showing rather than silently picking the first choice. That is the state
    /// a stored value the contract no longer offers arrives in, and it has to be
    /// visible: the alternative is a form that reports a value the project does
    /// not hold.
    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
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

    /// Mark the control required. Only meaningful together with
    /// [`Self::placeholder`], which is what supplies the empty value a browser
    /// refuses.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// The id the label, hint and error point at: the explicit one if set, else
    /// the name.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    /// Whether the placeholder is the current selection: nothing was chosen, or
    /// what was chosen is not on offer.
    fn placeholder_selected(&self) -> bool {
        match &self.selected {
            None => true,
            Some(value) => !self.choices.iter().any(|choice| &choice.value == value),
        }
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
        let placeholder_selected = self.placeholder_selected();
        let control = html! {
            select
                class="field-input field-select"
                id=(id)
                name=(self.name)
                aria-describedby=[described_by.as_deref()]
                aria-invalid=[shell.aria_invalid()]
                data-testid=[self.test_id.as_deref()]
                required[self.required]
            {
                @if let Some(placeholder) = &self.placeholder {
                    option value="" selected[placeholder_selected] { (placeholder) }
                }
                @for choice in &self.choices {
                    option
                        value=(choice.value)
                        selected[self.selected.as_deref() == Some(choice.value.as_str())]
                    { (choice.label) }
                }
            }
        };
        shell.render(control)
    }
}

impl ComponentBuilder for SelectBuilder {
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

impl Render for SelectBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> SelectBuilder {
        select("status", "Status")
            .option("ongoing", "Ongoing")
            .option("finished", "Finished")
    }

    #[test]
    fn the_label_points_at_the_control_it_labels() {
        let out = status().build().into_string();
        assert!(
            out.contains(r#"<label class="field-label" for="status">Status</label>"#),
            "{out}"
        );
        assert!(out.contains(r#"name="status""#), "{out}");
    }

    #[test]
    fn choices_render_in_the_order_they_were_added() {
        let out = status().build().into_string();
        let ongoing = out.find("Ongoing").expect("first choice");
        let finished = out.find("Finished").expect("second choice");
        assert!(ongoing < finished, "{out}");
    }

    #[test]
    fn options_adds_a_whole_list() {
        let out = select("access", "Access rights")
            .options([("open", "Full Open Access"), ("embargo", "Embargoed Access")])
            .build()
            .into_string();
        assert!(out.contains(r#"<option value="open">Full Open Access</option>"#), "{out}");
        assert!(out.contains(r#"<option value="embargo">Embargoed Access</option>"#), "{out}");
    }

    #[test]
    fn the_selected_value_is_the_one_marked_selected() {
        let out = status().selected("finished").build().into_string();
        assert!(out.contains(r#"<option value="finished" selected>Finished</option>"#), "{out}");
        assert!(out.contains(r#"<option value="ongoing">Ongoing</option>"#), "{out}");
    }

    #[test]
    fn without_a_placeholder_an_untouched_select_shows_its_first_choice() {
        // Stated so the next reader sees it is the browser's behaviour and the
        // reason `placeholder` exists, not an oversight in the tile.
        let out = status().build().into_string();
        assert!(out.contains(r#"<option value="ongoing">Ongoing</option>"#), "{out}");
        assert!(!out.contains("selected"), "{out}");
    }

    #[test]
    fn a_placeholder_is_selected_when_nothing_was_chosen() {
        let out = status().placeholder("Select a status…").build().into_string();
        assert!(out.contains(r#"<option value="" selected>Select a status…</option>"#), "{out}");
    }

    #[test]
    fn a_placeholder_is_not_disabled_so_a_field_can_be_returned_to_unset() {
        // `<option value="" selected disabled>` is the common recipe: it makes
        // an optional field a one-way door, because the empty choice cannot be
        // selected back.
        let out = status().placeholder("Select a status…").build().into_string();
        assert!(out.contains(r#"<option value="" selected>Select a status…</option>"#), "{out}");
        assert!(!out.contains("disabled"), "{out}");
    }

    #[test]
    fn a_placeholder_posts_an_empty_value_so_required_refuses_an_untouched_field() {
        let out = status().placeholder("Select…").required().build().into_string();
        assert!(out.contains(r#"<option value="" selected>"#), "{out}");
        assert!(out.contains("required"), "{out}");
    }

    #[test]
    fn a_stored_value_no_longer_on_offer_leaves_the_placeholder_showing() {
        // The alternative is a form reporting a value the project does not hold:
        // the browser selects the first choice when nothing is marked, so a
        // dropped contract value would silently read as "Ongoing".
        let out = status()
            .placeholder("Select a status…")
            .selected("suspended")
            .build()
            .into_string();
        assert!(out.contains(r#"<option value="" selected>"#), "{out}");
        assert!(!out.contains(r#"<option value="ongoing" selected>"#), "{out}");
    }

    #[test]
    fn there_is_no_multiple_choice_mode() {
        // A `<select multiple>` gives no affordance that more than one item is
        // selectable, and picking a second without a modifier deselects the
        // first. Several values is a checkbox group.
        let out = status().build().into_string();
        assert!(out.contains("<select"), "{out}");
        assert!(!out.contains("multiple"), "{out}");
    }

    #[test]
    fn an_error_marks_the_control_invalid_and_describes_it_by_the_message() {
        let out = status()
            .hint("Required even for a draft.")
            .error("Choose a status.")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-invalid="true""#), "{out}");
        assert!(out.contains(r#"aria-describedby="status-hint status-error""#), "{out}");
        assert!(out.contains("Choose a status."), "{out}");
    }

    #[test]
    fn the_error_region_is_rendered_empty_when_valid() {
        let out = status().build().into_string();
        assert!(
            out.contains(r#"<p class="field-error" id="status-error" aria-live="polite"></p>"#),
            "{out}"
        );
        assert!(!out.contains("aria-invalid"), "{out}");
    }

    #[test]
    fn a_choice_label_accepts_markup_and_a_value_is_escaped() {
        let out = select("kind", "Kind")
            .option(
                "a&b",
                html! {
                    em { "Both" }
                },
            )
            .build()
            .into_string();
        assert!(out.contains("<em>Both</em>"), "{out}");
        assert!(out.contains(r#"value="a&amp;b""#), "{out}");
    }

    #[test]
    fn an_explicit_id_moves_the_label_hint_and_error_with_it() {
        let out = status()
            .with_id("project-status")
            .hint("Ongoing or finished.")
            .error("Choose one.")
            .build()
            .into_string();
        assert!(out.contains(r#"for="project-status""#), "{out}");
        assert!(out.contains(r#"id="project-status-hint""#), "{out}");
        assert!(out.contains(r#"id="project-status-error""#), "{out}");
        assert!(out.contains(r#"name="status""#), "{out}");
    }

    #[test]
    fn test_id_lands_on_the_control_not_the_wrapper() {
        let out = status().with_test_id("status-select").build().into_string();
        let testid_at = out.find("data-testid").expect("test id missing");
        let control_at = out.find("<select").expect("select missing");
        assert!(control_at < testid_at, "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = status().required().build().into_string();
        let spliced = html! {
            (status().required())
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
