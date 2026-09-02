//! Radio group tile: a legend, one radio per choice, and an optional hint and
//! error — a field with exactly one value, chosen from a short visible list.
//!
//! `radio_group(name, legend)` returns a [`RadioGroupBuilder`]; add choices with
//! [`RadioGroupBuilder::option`], mark the current one with
//! [`RadioGroupBuilder::selected`], then splice it into `html!` (it implements
//! [`Render`]) or call `.build()`.
//!
//! ## When this rather than a select
//!
//! A [`select`](super::select) hides its choices until opened, which suits a
//! closed list nobody needs to compare — a status, an access-rights value. A
//! radio group shows all of them at once, which is what a *discriminant* wants:
//! the editor's variant choosers ("is this coverage an authority reference or
//! free text?") change which fields below them apply, so the reader has to see
//! the alternatives to understand the question. Keep the list short; a long one
//! is a select.
//!
//! ## A radio group cannot be returned to unset
//!
//! Once a radio in a group is checked, no interaction unchecks it — that is the
//! control's behaviour, not an omission. So a radio group is for a field that
//! always has a value. For one that may be unset, either give it an explicit
//! "none" choice or use a select with a placeholder.

use maud::{html, Markup, Render};

use super::choice::{choice_controls, push_choice, Choice, ChoiceKind};
use super::shell::group_shell;
use crate::builder::ComponentBuilder;

/// Builder for a labelled set of radios. Construct with [`radio_group`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct RadioGroupBuilder {
    name: String,
    legend: Markup,
    choices: Vec<Choice>,
    selected: Option<String>,
    inline: bool,
    hint: Option<Markup>,
    error: Option<Markup>,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a radio group posting under `name`, named by `legend`.
pub fn radio_group(name: impl Into<String>, legend: impl Render) -> RadioGroupBuilder {
    RadioGroupBuilder {
        name: name.into(),
        legend: legend.render(),
        choices: Vec::new(),
        selected: None,
        inline: false,
        hint: None,
        error: None,
        id: None,
        test_id: None,
    }
}

impl RadioGroupBuilder {
    /// Add a choice, in the order it should appear.
    pub fn option(mut self, value: impl Into<String>, label: impl Render) -> Self {
        push_choice(&mut self.choices, value, label);
        self
    }

    /// Add every choice from an iterator of `(value, label)` pairs.
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

    /// Mark the current value. A value matching no choice checks nothing.
    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }

    /// Add help text below the group, announced with it via `aria-describedby`.
    pub fn hint(mut self, hint: impl Render) -> Self {
        self.hint = Some(hint.render());
        self
    }

    /// Mark the group invalid and say why.
    pub fn error(mut self, message: impl Render) -> Self {
        self.error = Some(message.render());
        self
    }

    /// Lay the choices out in a row rather than a column.
    ///
    /// For a two- or three-way discriminant, where a row reads as one question
    /// with alternatives and a column reads as a list of unrelated settings.
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// The id the legend, hint and error hang off, and the stem each choice's own
    /// id is derived from.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let controls = choice_controls(ChoiceKind::Radio, id, &self.name, &self.choices, |value| {
            self.selected.as_deref() == Some(value)
        });
        let layout = if self.inline {
            "field-choices field-choices-inline"
        } else {
            "field-choices"
        };
        let controls = html! {
            div class=(layout) data-testid=[self.test_id.as_deref()] { (controls) }
        };
        group_shell(id, &self.legend, self.hint.as_ref(), self.error.as_ref(), controls)
    }
}

impl ComponentBuilder for RadioGroupBuilder {
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

impl Render for RadioGroupBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant() -> RadioGroupBuilder {
        radio_group("temporalCoverage.k1.kind", "How is this period recorded?")
            .option("reference", "An authority reference")
            .option("text", "Free text, per language")
    }

    #[test]
    fn the_group_is_named_by_a_legend_and_each_choice_by_its_own_label() {
        let out = variant().build().into_string();
        assert!(
            out.contains(r#"<legend class="field-label">How is this period recorded?</legend>"#),
            "{out}"
        );
        assert!(out.contains(r#"for="temporalCoverage.k1.kind-0""#), "{out}");
        assert!(out.contains(r#"for="temporalCoverage.k1.kind-1""#), "{out}");
    }

    #[test]
    fn every_choice_shares_the_name_which_is_what_makes_them_one_group() {
        // Radios only exclude each other when they share a name. Distinct names
        // render as radios and behave as independent toggles.
        let out = variant().build().into_string();
        assert_eq!(out.matches(r#"name="temporalCoverage.k1.kind""#).count(), 2, "{out}");
        assert_eq!(out.matches(r#"type="radio""#).count(), 2, "{out}");
    }

    #[test]
    fn the_selected_value_is_the_checked_one() {
        let out = variant().selected("text").build().into_string();
        assert!(out.contains(r#"value="text" checked"#), "{out}");
        assert!(out.contains(r#"value="reference""#), "{out}");
        assert!(!out.contains(r#"value="reference" checked"#), "{out}");
    }

    #[test]
    fn a_selected_value_that_is_not_on_offer_checks_nothing() {
        let out = variant().selected("grants").build().into_string();
        assert_eq!(out.matches("<input").count(), 2, "{out}");
        assert!(!out.contains("checked"), "{out}");
    }

    #[test]
    fn an_error_is_described_by_the_fieldset_not_repeated_on_every_control() {
        let out = variant()
            .hint("Changes which fields below apply.")
            .error("Choose how the period is recorded.")
            .build()
            .into_string();
        assert!(out.contains("-hint temporalCoverage.k1.kind-error\""), "{out}");
        assert!(!out.contains("aria-invalid"), "{out}");
        assert!(out.contains(r#"data-invalid="true""#), "{out}");
        assert_eq!(out.matches("Choose how the period is recorded.").count(), 1, "{out}");
    }

    #[test]
    fn inline_lays_the_choices_out_in_a_row() {
        let column = variant().build().into_string();
        let row = variant().inline().build().into_string();
        assert!(!column.contains("field-choices-inline"), "{column}");
        assert!(row.contains("field-choices-inline"), "{row}");
    }

    #[test]
    fn an_explicit_id_moves_the_choice_ids_with_it() {
        let out = variant().with_id("coverage-kind").build().into_string();
        assert!(out.contains(r#"for="coverage-kind-0""#), "{out}");
        assert!(out.contains(r#"id="coverage-kind-error""#), "{out}");
        assert!(out.contains(r#"name="temporalCoverage.k1.kind""#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = variant().selected("text").build().into_string();
        let spliced = html! {
            (variant().selected("text"))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
