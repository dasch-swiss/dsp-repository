//! The label / hint / error wrapper every form tile renders around its control.
//!
//! Private to `form`: it is not a tile and has no showcase. What it owns is the
//! part that is identical for an input, a textarea and a select and wrong in a
//! way nothing catches if one of them differs — the derived ids and the
//! `aria-describedby` that ties them to the control.
//!
//! The failure it prevents is specific. Each tile needs `id`, the label's `for`,
//! the hint's `id`, the error's `id`, and a `aria-describedby` naming whichever
//! of the last two exist. Written out per tile that is five places to keep in
//! agreement, and a mismatch is silent: the field still renders, it just stops
//! being announced correctly, and a snapshot test asserting the attribute is
//! present still passes. Deriving them once from the field name is the same
//! argument the label being inside `text_field` rests on, one level up.
//!
//! It is a struct rather than a function taking five arguments because three of
//! them are `Option<&Markup>` and adjacent optional arguments of one type are
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
    /// The hint's id, or `None` when there is no hint to point at.
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
    /// Hint before error, matching the order they appear in. The error is the
    /// more urgent of the two, but "project shortcodes, separated by commas —
    /// 'nope!' is not a shortcode" reads the way a person would say it, where
    /// the reverse states a problem about a field not yet described.
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
    /// `aria-live` region announces a *change* to content it already contains; a
    /// region inserted into the DOM together with its text is widely reported
    /// not to announce at all, and the editor's validation path is exactly that
    /// case — a rejected submit re-renders the form and Datastar morphs it in,
    /// so an error paragraph that only exists when errored arrives as a new node
    /// and says nothing. Rendering it empty from the start means the morph
    /// writes into a region the assistive technology is already watching.
    ///
    /// The cost is an empty `<p>` per field; `.field-error:empty` collapses it,
    /// so it occupies no space and shows nothing. [`Self::described_by`] still
    /// names it only when there is a message, because describing a field by an
    /// empty element is noise.
    ///
    /// `control` is a `Markup` rather than `impl Render` on purpose: this is an
    /// internal seam whose one caller is the tile that just built its own
    /// control, not a public content slot.
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
/// A `<label for>` needs a single control to point at, and a checkbox or radio
/// group has one per choice — each already carrying its own label. The group's
/// accessible name is therefore a `<legend>` inside a `<fieldset>`, and the
/// hint and error hang off the fieldset via its `aria-describedby` rather than
/// off any one control.
///
/// `aria-invalid` deliberately does **not** go on the fieldset: it is not a
/// valid attribute there, and putting it on every member control would have a
/// screen reader announce the same group error once per choice. The red rule
/// and the message carry the state visually and in the description, and
/// `data-invalid` is what the CSS selects on.
///
/// `controls` arrives already wrapped in its own layout container, so a tile
/// that lays its choices out in a row rather than a column does not need a
/// parameter threaded through here.
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
        // A `for` needs one control to point at, and a group has one per choice.
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
        // `aria-invalid` is not valid on a fieldset, and repeating it on every
        // member would announce one group error once per choice.
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
