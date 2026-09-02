//! Checkbox group tile: a legend, one checkbox per choice, and an optional hint
//! and error — a field whose value is a set.
//!
//! `checkbox_group(name, legend)` returns a [`CheckboxGroupBuilder`]; add
//! choices with [`CheckboxGroupBuilder::option`], mark the current ones with
//! [`CheckboxGroupBuilder::checked`], then splice it into `html!` (it implements
//! [`Render`]) or call `.build()`.
//!
//! ## Every checkbox posts under the group's name
//!
//! That is what makes the group one field rather than several: two checked
//! choices send the name twice, and the form decoder reads repeated keys as a
//! list. It is also why nothing here uses an index in the name — `typeOfData`
//! twice is a list of two; `typeOfData[0]` and `typeOfData[1]` are two fields
//! whose indices go stale as soon as a choice list changes.
//!
//! ## An empty set posts nothing at all
//!
//! Unchecked checkboxes are not submitted, so a group with nothing checked is
//! *absent* from the body rather than present and empty. A decoder that reads
//! "absent" as "leave the stored value alone" therefore cannot tell "the
//! depositor cleared this field" from "this section did not carry it" — the
//! caller has to render a same-named empty marker alongside the group, or scope
//! the clear to the fields the section is known to own. Stated here because the
//! tile cannot fix it: it is how HTML forms work.

use maud::{html, Markup, Render};

use super::choice::{choice_controls, push_choice, Choice, ChoiceKind};
use super::shell::group_shell;
use crate::builder::ComponentBuilder;

/// Builder for a labelled set of checkboxes. Construct with [`checkbox_group`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct CheckboxGroupBuilder {
    name: String,
    legend: Markup,
    choices: Vec<Choice>,
    checked: Vec<String>,
    hint: Option<Markup>,
    error: Option<Markup>,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a checkbox group posting under `name`, named by `legend`.
pub fn checkbox_group(name: impl Into<String>, legend: impl Render) -> CheckboxGroupBuilder {
    CheckboxGroupBuilder {
        name: name.into(),
        legend: legend.render(),
        choices: Vec::new(),
        checked: Vec::new(),
        hint: None,
        error: None,
        id: None,
        test_id: None,
    }
}

impl CheckboxGroupBuilder {
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

    /// Mark the current values.
    ///
    /// A value matching no choice is ignored rather than added: the group offers
    /// what it offers, and inventing a control for a stored value would let a
    /// field the contract no longer has ride back out through the form. Making
    /// such a value *visible* is the caller's decision, because only the caller
    /// knows whether it is a retired option to warn about or data to carry
    /// silently.
    pub fn checked<V: Into<String>>(mut self, values: impl IntoIterator<Item = V>) -> Self {
        self.checked = values.into_iter().map(Into::into).collect();
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

    /// The id the legend, hint and error hang off, and the stem each choice's own
    /// id is derived from.
    fn resolved_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    fn markup(&self) -> Markup {
        let id = self.resolved_id();
        let controls = choice_controls(ChoiceKind::Checkbox, id, &self.name, &self.choices, |value| {
            self.checked.iter().any(|current| current == value)
        });
        let controls = html! {
            div class="field-choices" data-testid=[self.test_id.as_deref()] { (controls) }
        };
        group_shell(id, &self.legend, self.hint.as_ref(), self.error.as_ref(), controls)
    }
}

impl ComponentBuilder for CheckboxGroupBuilder {
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

impl Render for CheckboxGroupBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds() -> CheckboxGroupBuilder {
        checkbox_group("typeOfData", "Type of data")
            .option("Text", "Text")
            .option("Image", "Image")
            .option("Audio", "Audio")
    }

    #[test]
    fn the_group_is_named_by_a_legend_and_each_choice_by_its_own_label() {
        let out = kinds().build().into_string();
        assert!(out.contains(r#"<legend class="field-label">Type of data</legend>"#), "{out}");
        assert!(out.contains(r#"for="typeOfData-0""#), "{out}");
        assert!(out.contains(r#"for="typeOfData-2""#), "{out}");
    }

    #[test]
    fn every_choice_posts_under_the_group_name_with_no_index() {
        // Repeated keys under one name are what the decoder reads as a list. An
        // index in the name would go stale the moment the choice list changes.
        let out = kinds().build().into_string();
        assert_eq!(out.matches(r#"name="typeOfData""#).count(), 3, "{out}");
        assert!(!out.contains("typeOfData[0]"), "{out}");
    }

    #[test]
    fn the_current_values_are_the_checked_ones() {
        let out = kinds().checked(["Image", "Audio"]).build().into_string();
        assert!(out.contains(r#"value="Image" checked"#), "{out}");
        assert!(out.contains(r#"value="Audio" checked"#), "{out}");
        assert!(out.contains(r#"value="Text""#), "{out}");
        assert!(!out.contains(r#"value="Text" checked"#), "{out}");
    }

    #[test]
    fn a_current_value_that_is_not_on_offer_adds_no_control() {
        // Inventing a control would let a value the contract no longer has ride
        // back out through the form as though it had been offered.
        let out = kinds().checked(["Software"]).build().into_string();
        assert!(!out.contains("Software"), "{out}");
        assert_eq!(out.matches("<input").count(), 3, "{out}");
        assert!(!out.contains("checked"), "{out}");
    }

    #[test]
    fn nothing_checked_renders_every_choice_unchecked() {
        let out = kinds().build().into_string();
        assert_eq!(out.matches("<input").count(), 3, "{out}");
        assert!(!out.contains("checked"), "{out}");
    }

    #[test]
    fn an_error_is_described_by_the_fieldset_not_repeated_on_every_control() {
        // One group error announced once per choice is the failure this avoids.
        let out = kinds()
            .hint("Pick every kind the dataset holds.")
            .error("Pick at least one kind of data.")
            .build()
            .into_string();
        assert!(out.contains(r#"aria-describedby="typeOfData-hint typeOfData-error""#), "{out}");
        assert!(!out.contains("aria-invalid"), "{out}");
        assert!(out.contains(r#"data-invalid="true""#), "{out}");
        assert_eq!(out.matches("Pick at least one kind of data.").count(), 1, "{out}");
    }

    #[test]
    fn the_error_region_is_rendered_empty_when_valid() {
        let out = kinds().build().into_string();
        assert!(
            out.contains(r#"<p class="field-error" id="typeOfData-error" aria-live="polite"></p>"#),
            "{out}"
        );
        assert!(!out.contains("data-invalid"), "{out}");
    }

    #[test]
    fn an_explicit_id_moves_the_choice_ids_with_it() {
        // Two groups collecting the same field on one page would otherwise share
        // every choice id, and a duplicate id mislabels one of them.
        let out = kinds().with_id("dataset-kinds").build().into_string();
        assert!(out.contains(r#"for="dataset-kinds-0""#), "{out}");
        assert!(out.contains(r#"id="dataset-kinds-error""#), "{out}");
        // The submitted name is unchanged — only the ids moved.
        assert!(out.contains(r#"name="typeOfData""#), "{out}");
    }

    #[test]
    fn options_adds_a_whole_list() {
        let out = checkbox_group("dataLanguage", "Data languages")
            .options([("de", "German"), ("en", "English")])
            .checked(["en"])
            .build()
            .into_string();
        assert!(out.contains(r#"value="de""#), "{out}");
        assert!(out.contains(r#"value="en" checked"#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = kinds().checked(["Image"]).build().into_string();
        let spliced = html! {
            (kinds().checked(["Image"]))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
