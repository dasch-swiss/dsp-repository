//! What a checkbox group and a radio group have in common.
//!
//! Private to `form`. The two tiles differ in three things — the input type,
//! whether one or several choices can be current, and what "nothing chosen"
//! means — and agree on everything else: a `<fieldset>` named by a `<legend>`,
//! one label-wrapped control per choice, per-choice ids derived from the group's,
//! and the hint and error on the fieldset. Writing that twice is how a group
//! ends up with a label pointing at the wrong input in one of them.
//!
//! ## Per-choice ids come from the index, not the value
//!
//! A choice's value is contract data — `Full Open Access`, a language tag, a URL
//! — and an `id` has to be unique in the document and is referenced from a
//! `for`. Slugging a value gives collisions (two values differing only in
//! punctuation) and unstable ids (a value edited upstream moves every `for`
//! below it). The index is stable for a given ordered choice list, which is what
//! the tile has.

use maud::{html, Markup, Render};

/// One choice: the value posted, and the text shown beside its control.
pub(super) struct Choice {
    pub value: String,
    pub label: Markup,
}

/// Which control a group renders, and therefore how many choices can be current.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ChoiceKind {
    /// `type="checkbox"` — any number current, all sharing the group's name, so
    /// the body carries the name once per checked choice.
    Checkbox,
    /// `type="radio"` — at most one current.
    Radio,
}

impl ChoiceKind {
    fn input_type(self) -> &'static str {
        match self {
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
        }
    }
}

/// The controls of one group: a label-wrapped input per choice, in order.
///
/// `is_current` decides each choice's checked state, so a checkbox group can ask
/// "is this value in the set" and a radio group "is this the one value" without
/// this function knowing which.
///
/// Each control is *inside* its `<label>` as well as pointed at by the label's
/// `for`. The wrapping is what makes the text a click target; the `for` is what
/// keeps the association explicit for assistive technology, which does not have
/// to rely on ancestry.
pub(super) fn choice_controls(
    kind: ChoiceKind,
    group_id: &str,
    name: &str,
    choices: &[Choice],
    is_current: impl Fn(&str) -> bool,
) -> Markup {
    html! {
        @for (index, choice) in choices.iter().enumerate() {
            @let id = format!("{group_id}-{index}");
            label class="field-choice" for=(id) {
                input
                    class="field-choice-input"
                    type=(kind.input_type())
                    id=(id)
                    name=(name)
                    value=(choice.value)
                    checked[is_current(&choice.value)];
                span class="field-choice-label" { (choice.label) }
            }
        }
    }
}

/// Push a `(value, label)` pair onto a choice list.
pub(super) fn push_choice(choices: &mut Vec<Choice>, value: impl Into<String>, label: impl Render) {
    choices.push(Choice { value: value.into(), label: label.render() });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choices() -> Vec<Choice> {
        let mut choices = Vec::new();
        push_choice(&mut choices, "text", "Text");
        push_choice(&mut choices, "image", "Image");
        choices
    }

    #[test]
    fn every_choice_has_its_own_id_and_its_label_points_at_it() {
        let out = choice_controls(ChoiceKind::Checkbox, "kinds", "kinds", &choices(), |_| false).into_string();
        assert!(out.contains(r#"<label class="field-choice" for="kinds-0">"#), "{out}");
        assert!(out.contains(r#"id="kinds-0""#), "{out}");
        assert!(out.contains(r#"<label class="field-choice" for="kinds-1">"#), "{out}");
        assert!(out.contains(r#"id="kinds-1""#), "{out}");
    }

    #[test]
    fn every_choice_posts_under_the_group_name() {
        // A checkbox group is repeated keys under one name; that is what the
        // form decoder reads as a list.
        let out = choice_controls(ChoiceKind::Checkbox, "kinds", "typeOfData", &choices(), |_| false).into_string();
        assert_eq!(out.matches(r#"name="typeOfData""#).count(), 2, "{out}");
    }

    #[test]
    fn ids_come_from_the_index_so_a_value_with_punctuation_is_still_a_valid_id() {
        let mut choices = Vec::new();
        push_choice(&mut choices, "Full Open Access", "Full Open Access");
        push_choice(&mut choices, "Open Access with Restrictions", "Open Access with Restrictions");
        let out = choice_controls(ChoiceKind::Checkbox, "access", "access", &choices, |_| false).into_string();
        assert!(out.contains(r#"for="access-0""#), "{out}");
        assert!(out.contains(r#"for="access-1""#), "{out}");
        // The value still travels verbatim, escaped.
        assert!(out.contains(r#"value="Full Open Access""#), "{out}");
    }

    #[test]
    fn the_current_predicate_decides_which_controls_are_checked() {
        let out = choice_controls(ChoiceKind::Checkbox, "k", "k", &choices(), |v| v == "image").into_string();
        assert!(out.contains(r#"value="image" checked"#), "{out}");
        assert!(out.contains(r#"value="text""#), "{out}");
        assert!(!out.contains(r#"value="text" checked"#), "{out}");
    }

    #[test]
    fn the_kind_decides_the_input_type() {
        let radio = choice_controls(ChoiceKind::Radio, "k", "k", &choices(), |_| false).into_string();
        assert!(radio.contains(r#"type="radio""#), "{radio}");
        assert!(!radio.contains(r#"type="checkbox""#), "{radio}");
    }

    #[test]
    fn a_choice_label_accepts_markup() {
        let mut choices = Vec::new();
        push_choice(
            &mut choices,
            "ref",
            html! {
                "A reference "
                em { "(recommended)" }
            },
        );
        let out = choice_controls(ChoiceKind::Radio, "k", "k", &choices, |_| false).into_string();
        assert!(out.contains("<em>(recommended)</em>"), "{out}");
    }
}
