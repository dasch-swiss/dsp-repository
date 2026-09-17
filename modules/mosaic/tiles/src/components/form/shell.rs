//! The label / hint / error wrapper every form tile renders around its control.
//!
//! Private to `form`: not a tile, and no showcase. It owns the five things every
//! form tile must keep in agreement — `id`, the label's `for`, the hint's and
//! error's `id`, and the `aria-describedby` naming whichever of the last two
//! exist — because a mismatch between them is silent: the field still renders,
//! it stops being announced correctly, and a test asserting the attribute is
//! present still passes.
//!
//! It is a struct rather than a function taking five arguments because three of
//! them are `Option<&Markup>`, and adjacent optional arguments of one type are
//! silently swappable — a hint rendered where an error belongs.

use maud::{html, Markup};

/// One field's label, hint and error, and the ids that tie them together.
pub(super) struct FieldShell<'a> {
    /// The control's `id`, and the stem the hint and error ids are derived from.
    pub id: &'a str,
    pub label: &'a Markup,
    pub hint: Option<&'a Markup>,
    pub error: Option<&'a Markup>,
}

impl<'a> FieldShell<'a> {
    pub fn hint_id(&self) -> Option<String> {
        self.hint.map(|_| format!("{}-hint", self.id))
    }

    /// The error region's id. Always present, because the region always is —
    /// see [`Self::render`].
    pub fn error_id(&self) -> String {
        format!("{}-error", self.id)
    }

    /// What the control's `aria-describedby` should be, or `None` when there is
    /// nothing to describe it by.
    ///
    /// Hint before error, matching the order they appear in: the description
    /// then reads the way a person would say it, where the reverse states a
    /// problem about a field not yet described.
    pub fn described_by(&self) -> Option<String> {
        match (self.hint_id(), self.error) {
            (Some(hint), Some(_)) => Some(format!("{hint} {}", self.error_id())),
            (Some(hint), None) => Some(hint),
            (None, Some(_)) => Some(self.error_id()),
            (None, None) => None,
        }
    }

    /// `aria-invalid`'s value, or `None` when the field is valid. A field is
    /// invalid exactly when it has a message, which is why the two cannot be set
    /// apart from each other.
    pub fn aria_invalid(&self) -> Option<&'static str> {
        self.error.map(|_| "true")
    }

    /// The field: its label, the `control` markup, the hint, and the error
    /// region.
    ///
    /// The error region is rendered **even when there is no error**. An
    /// `aria-live` region announces a *change* to content it already contains,
    /// and a region inserted together with its text is widely reported not to
    /// announce at all — which is exactly the editor's validation path, where a
    /// rejected submit re-renders the form and Datastar morphs it in. The cost
    /// is an empty `<p>` per field, collapsed by `.field-error:empty`;
    /// [`Self::described_by`] names it only when there is a message.
    ///
    /// `control` is a `Markup` rather than `impl Render` on purpose: an internal
    /// seam whose one caller is the tile that just built its own control, not a
    /// public content slot.
    pub fn render(&self, control: Markup) -> Markup {
        let hint_id = self.hint_id();
        html! {
            div class="field" {
                label class="field-label" for=(self.id) { (self.label) }
                (control)
                @if let Some(hint) = self.hint {
                    p class="field-hint" id=[hint_id.as_deref()] { (hint) }
                }
                p class="field-error" id=(self.error_id()) aria-live="polite" {
                    @if let Some(error) = self.error { (error) }
                }
            }
        }
    }
}

/// The same field wrapper for a control that is several elements rather than
/// one.
///
/// A `<label for>` needs a single control to point at and a group has one per
/// choice, so the group's accessible name is a `<legend>` inside a
/// `<fieldset>`, and the hint and error hang off the fieldset.
///
/// `aria-invalid` deliberately does **not** go on the fieldset: it is not valid
/// there, and putting it on every member control would announce the same group
/// error once per choice. `data-invalid` is what the CSS selects on.
///
/// `controls` arrives already wrapped in its own layout container, so a tile
/// laying its choices out in a row needs no parameter threaded through here.
pub(super) fn group_shell(
    id: &str,
    legend: &Markup,
    hint: Option<&Markup>,
    error: Option<&Markup>,
    controls: Markup,
) -> Markup {
    let shell = FieldShell { id, label: legend, hint, error };
    let hint_id = shell.hint_id();
    let described_by = shell.described_by();
    html! {
        fieldset
            class="field field-group"
            id=(id)
            aria-describedby=[described_by.as_deref()]
            data-invalid=[shell.aria_invalid()]
        {
            legend class="field-label" { (legend) }
            (controls)
            @if let Some(hint) = hint {
                p class="field-hint" id=[hint_id.as_deref()] { (hint) }
            }
            p class="field-error" id=(shell.error_id()) aria-live="polite" {
                @if let Some(error) = error { (error) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label() -> Markup {
        html! {
            "Name"
        }
    }

    fn markup(text: &str) -> Markup {
        html! {
            (text)
        }
    }

    #[test]
    fn a_field_with_neither_hint_nor_error_describes_the_control_by_nothing() {
        let shell = FieldShell { id: "name", label: &label(), hint: None, error: None };
        assert_eq!(shell.described_by(), None);
        assert_eq!(shell.aria_invalid(), None);
    }

    #[test]
    fn a_hint_and_an_error_are_named_in_the_order_they_appear() {
        let hint = markup("Separated by commas.");
        let error = markup("Not a shortcode.");
        let shell = FieldShell {
            id: "codes",
            label: &label(),
            hint: Some(&hint),
            error: Some(&error),
        };
        assert_eq!(shell.described_by().as_deref(), Some("codes-hint codes-error"));
        assert_eq!(shell.aria_invalid(), Some("true"));
    }

    #[test]
    fn the_error_region_is_rendered_empty_so_a_live_update_has_a_region_to_change() {
        let shell = FieldShell { id: "name", label: &label(), hint: None, error: None };
        let out = shell
            .render(html! {
                input;
            })
            .into_string();
        assert!(
            out.contains(r#"<p class="field-error" id="name-error" aria-live="polite"></p>"#),
            "{out}"
        );
        assert!(!out.contains("field-hint"), "{out}");
    }

    #[test]
    fn a_group_is_named_by_a_legend_rather_than_a_for_attribute() {
        let hint = markup("Pick any that apply.");
        let controls = html! {
            input type="checkbox";
        };
        let out = group_shell("kinds", &label(), Some(&hint), None, controls).into_string();
        assert!(out.contains("<fieldset"), "{out}");
        assert!(out.contains(r#"<legend class="field-label">Name</legend>"#), "{out}");
        assert!(!out.contains("for="), "{out}");
        assert!(out.contains(r#"aria-describedby="kinds-hint""#), "{out}");
    }

    #[test]
    fn a_group_marks_its_invalid_state_without_aria_invalid_on_the_fieldset() {
        let error = markup("Pick at least one.");
        let controls = html! {
            input type="checkbox";
        };
        let out = group_shell("kinds", &label(), None, Some(&error), controls).into_string();
        assert!(out.contains(r#"data-invalid="true""#), "{out}");
        assert!(!out.contains("aria-invalid"), "{out}");
        assert!(out.contains(r#"aria-describedby="kinds-error""#), "{out}");
        assert!(out.contains("Pick at least one."), "{out}");
    }

    #[test]
    fn the_label_points_at_the_control_id() {
        let shell = FieldShell { id: "name", label: &label(), hint: None, error: None };
        let out = shell
            .render(html! {
                input id="name";
            })
            .into_string();
        assert!(out.contains(r#"<label class="field-label" for="name">Name</label>"#), "{out}");
    }
}
