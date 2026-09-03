//! Which control each field renders — the `FIELD_RENDERERS` half of the
//! prototype's split, keyed by the same ids [`registry`](super::registry) is.
//!
//! The dispatch is a `match` on the field **id**, not on the shape: two fields
//! can share a shape and want different controls (`provenance` and
//! `dataManagementPlan` are both an `Option<String>`, one a paragraph and one a
//! URL).
//!
//! A field renders in one of three states, and which one decides whether it
//! **posts**: a control posts even when empty, a value and a note post nothing
//! at all. That is load-bearing, because a section posts only its own fields and
//! an applier reads an absent name as "this section did not carry it" — so a
//! display-only or not-yet-read field must submit no name, or an empty control
//! would clear a value the save was never meant to touch (REQ-1.5, REQ-1.7).
//!
//! A note rather than silence for a field whose widget has not landed: a
//! depositor who cannot find "Keywords" in the section the published page shows
//! it in would otherwise conclude the form lost it.

use editor_core::draft::ProjectDraft;
use editor_core::form::Shape;
use editor_core::multilingual::{DraftMultilingual, UI_LANGUAGES};
use maud::{html, Markup};
use mosaic_tiles::text_field::{text_field, InputType};
use mosaic_tiles::textarea::textarea;
use platform_metadata::is_placeholder;
use serde_json::Value;

use super::registry::{Field, Obligation};

/// Whether the form is open for editing.
///
/// A named type rather than a `bool` argument: `field_row(field, draft, true)`
/// at a call site says nothing about which way round `true` is, and the two
/// renderings differ by whether a save can change the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Controls, and a save that writes.
    Editable,
    /// Values only — the project has a submission in review, so nothing may
    /// change under the reviewer (REQ-4.x).
    ReadOnly,
}

/// What a reader is told about a field whose widget has not landed.
const NOT_READ_YET: &str = "This field is not editable here yet. Its current value is kept unchanged when you \
                            save, and the control arrives in a later release.";

/// What is shown in place of a value the project does not have.
const NO_VALUE: &str = "Not set";

/// One field: its label, its obligation, and whichever of the three renderings
/// applies.
pub fn field_row(field: &Field, draft: &ProjectDraft, mode: Mode) -> Markup {
    match (mode, field.shape) {
        (Mode::Editable, Some(shape)) => control(field, draft, shape),
        // A locked field and a display-only one render the same way, which is
        // the point: neither posts, so neither can be cleared.
        (Mode::ReadOnly, _) | (_, None) => stated(field, draft),
    }
}

/// A field rendered as a value or as a note, with its own label above it.
///
/// Not a `<label>`: there is no control for one to point at, and a `for`
/// naming nothing is worse than no `for` at all. The heading and the value are
/// tied by proximity and by the same `field-*` treatment the tiles use, so a
/// locked form reads as the same form.
fn stated(field: &Field, draft: &ProjectDraft) -> Markup {
    let editable_but_unbuilt = !field.display_only && field.shape.is_none();
    html! {
        div class="field" {
            p class="field-label" { (labelled(field)) }
            @if editable_but_unbuilt {
                (value_display(field, draft))
                p class="field-hint" { (NOT_READ_YET) }
            } @else {
                (value_display(field, draft))
                @if let Some(hint) = field.hint {
                    p class="field-hint" { (hint) }
                }
            }
        }
    }
}

/// The stored value, rendered for reading.
///
/// A placeholder sentinel is *not* shown: `MISSING` and `CALCULATED` are the
/// platform's "no value yet" markers, filtered out of DPE's UI and of OAI-PMH's
/// output, so showing one here would be the only place in the platform that
/// presents an internal marker as a value.
fn value_display(field: &Field, draft: &ProjectDraft) -> Markup {
    let stored = draft.get(field.id);
    html! {
        @match stored {
            Some(Value::String(text)) if !is_placeholder(text) && !text.trim().is_empty() => {
                p class="whitespace-pre-line text-neutral-900" { (text) }
            }
            Some(value) if is_language_map(value) => {
                (language_list(&draft.multilingual(field.id)))
            }
            Some(Value::Array(items)) if !items.is_empty() => {
                p class="text-neutral-900" { (count_summary(items.len())) }
            }
            Some(Value::Object(members)) if !members.is_empty() => {
                p class="text-neutral-900" { (count_summary(members.len())) }
            }
            _ => {
                p class="italic text-neutral-600" { (NO_VALUE) }
            }
        }
    }
}

/// Whether a stored object is a language map rather than a structured value.
///
/// Every member being a short lowercase tag holding a string is what separates
/// `{"en": "…"}` from `{"type": …, "url": …}`. A structured value is summarised
/// by its member count instead, which is honest about not rendering it rather
/// than showing a half-parsed version of it.
fn is_language_map(value: &Value) -> bool {
    value.as_object().is_some_and(|members| {
        !members.is_empty()
            && members
                .iter()
                .all(|(key, text)| text.is_string() && key.len() <= 3 && key.chars().all(|c| c.is_ascii_lowercase()))
    })
}

/// A read-only language map: one line per language, in the form's own order.
fn language_list(value: &DraftMultilingual) -> Markup {
    html! {
        dl class="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1" {
            @for (tag, text) in value.iter() {
                @if !text.trim().is_empty() {
                    dt class="text-sm font-bold text-neutral-600" { (language_name(tag)) }
                    dd class="whitespace-pre-line text-neutral-900" { (text) }
                }
            }
        }
    }
}

/// "3 entries" — what a structured field this module does not render yet can
/// still say truthfully.
fn count_summary(count: usize) -> String {
    if count == 1 {
        "1 entry".to_string()
    } else {
        format!("{count} entries")
    }
}

/// The language a tag names, or the tag itself.
///
/// Covers every tag in the committed corpus: `en` (940 values), `de` (244),
/// `fr` (124), `ar` (18) and `it` (7). Anything else falls back to the tag,
/// which is honest — a wrong name is worse than a raw code, and the set of tags
/// is deliberately open ([`UI_LANGUAGES`] is what the form *offers*, not what it
/// accepts).
fn language_name(tag: &str) -> &str {
    match tag {
        "de" => "German",
        "en" => "English",
        "fr" => "French",
        "it" => "Italian",
        "ar" => "Arabic",
        other => other,
    }
}

/// The control for one editable field, dispatched by id.
///
/// The `match` is exhaustive over the ids the registry declares a shape for, and
/// the fallback is not a silent default: it renders the same note an unbuilt
/// field gets, so a field given a shape without a control here is visible rather
/// than posting under a name with no way to enter a value.
fn control(field: &Field, draft: &ProjectDraft, shape: Shape) -> Markup {
    match field.id {
        "name" | "officialName" => text(field, draft, InputType::Text),
        // `type="text"`, not `type="url"`: a draft may hold a value that does
        // not validate (REQ-1.9), and a browser refusing to submit a half-typed
        // address would block the save REQ-1.10 asks for — the same reason
        // `text` below never sets `required`.
        "dataManagementPlan" => text(field, draft, InputType::Text),
        "startDate" | "endDate" => text(field, draft, InputType::Date),
        "dataPublicationYear" => year(field, draft),
        "shortDescription" => long_text(field, draft, 2, Some(SHORT_DESCRIPTION_MAX)),
        "provenance" | "imageCredit" => long_text(field, draft, 3, None),
        "description" => multilingual(field, draft, 5),
        "abstract" => multilingual(field, draft, 2),
        // Unreachable while the registry and this dispatch agree, which
        // `tests::every_shaped_field_has_a_control` pins. Rendered rather than
        // panicked: a missing control is a gap in this file, and taking the
        // whole section down for it would hide every other field too.
        _ => {
            debug_assert!(false, "{} declares {shape:?} but no control", field.id);
            stated(field, draft)
        }
    }
}

/// The character bound the prototype's screens put on the teaser, and the
/// registry's hint states ("Up to 200 characters"), so the control enforces what
/// the reader was told.
const SHORT_DESCRIPTION_MAX: u32 = 200;

/// The value a scalar control shows: the stored string, with a placeholder
/// sentinel rendered as empty.
///
/// This is the rule the whole untouched-save guarantee rests on. 131 sentinels
/// sit across 8 paths in the 85 committed files, 24 of them `endDate`; each one
/// renders empty here and posts empty, and `apply_text` is what recognises that
/// an empty submit against a stored sentinel is not a clear.
fn scalar_value<'a>(field: &Field, draft: &'a ProjectDraft) -> &'a str {
    draft
        .get(field.id)
        .and_then(Value::as_str)
        .filter(|text| !is_placeholder(text))
        .unwrap_or_default()
}

fn text(field: &Field, draft: &ProjectDraft, input_type: InputType) -> Markup {
    let mut control = text_field(field.id, labelled(field))
        .input_type(input_type)
        .value(scalar_value(field, draft));
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    // `required` is deliberately absent even on a `Required` field: a draft may
    // be missing anything (REQ-1.9), and a browser refusing to save one is the
    // opposite of what REQ-1.10 asks for. The obligation is stated in words
    // beside the field, and enforced at submit.
    html! {
        (control)
    }
}

fn year(field: &Field, draft: &ProjectDraft) -> Markup {
    let mut control = text_field(field.id, labelled(field)).year().value(scalar_value(field, draft));
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    html! {
        (control)
    }
}

fn long_text(field: &Field, draft: &ProjectDraft, rows: u32, maxlength: Option<u32>) -> Markup {
    let mut control = textarea(field.id, labelled(field)).rows(rows).value(scalar_value(field, draft));
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    if let Some(maxlength) = maxlength {
        control = control.maxlength(maxlength);
    }
    html! {
        (control)
    }
}

/// A language map: one labelled control per language, inside a group named by
/// the field.
///
/// The tags rendered are [`UI_LANGUAGES`] plus whatever the value already
/// carries. Offering only a closed set would drop `ar` — live in two committed
/// files — on the first save, because a tag with no control posts nothing and a
/// map rebuilt from the body would not carry it.
///
/// A `<fieldset>` rather than a bare `<div>`: the field's own name has to reach
/// assistive technology, and each control's `<label>` is the language, so the
/// group's name can only be a `<legend>`. Same reasoning as the checkbox and
/// radio tiles, which is why the markup matches theirs.
fn multilingual(field: &Field, draft: &ProjectDraft, rows: u32) -> Markup {
    let value = draft.multilingual(field.id);
    let tags: Vec<&str> = UI_LANGUAGES.iter().copied().chain(value.extra_tags()).collect();
    let hint_id = field.hint.map(|_| format!("{}-hint", field.id));
    html! {
        fieldset class="field field-group" id=(field.id) aria-describedby=[hint_id.as_deref()] {
            legend class="field-label" { (labelled(field)) }
            div class="flex flex-col gap-3" {
                @for tag in &tags {
                    ({
                        textarea(format!("{}.{tag}", field.id), language_name(tag))
                            .rows(rows)
                            .value(value.get(tag).unwrap_or_default())
                    })
                }
            }
            @if let Some(hint) = field.hint {
                p class="field-hint" id=[hint_id.as_deref()] { (hint) }
            }
        }
    }
}

/// A field's label: its name, and its obligation as a pill **inside** it.
///
/// Inside, because no input here carries `required` or `aria-required` — a draft
/// may be missing anything (REQ-1.9) — which leaves the accessible name as the
/// only channel the tier has. As a sibling the pill was visible and nothing
/// else: a reader tabbing to the control heard "Name, edit text".
fn labelled(field: &Field) -> Markup {
    html! {
        (field.label)
        @if let Some(obligation) = field.obligation {
            " "
            span class=(pill_class(obligation)) { (obligation.label()) }
        }
    }
}

/// The pill's classes, as a complete literal string per tier.
///
/// Not assembled from the tier's name: `@import 'tailwindcss'` collects classes
/// by scanning source text, so a class built at runtime is a class the build
/// never sees, and the pill renders unstyled with no error anywhere. Same reason
/// `AlertVariant::css_class` spells each one out.
const fn pill_class(obligation: Obligation) -> &'static str {
    match obligation {
        Obligation::Required => "w-fit rounded bg-warning-50 px-2 py-0.5 text-xs font-bold text-warning-800",
        Obligation::Recommended => "w-fit rounded bg-info-50 px-2 py-0.5 text-xs font-bold text-info-800",
        Obligation::Optional => "w-fit rounded bg-neutral-100 px-2 py-0.5 text-xs font-bold text-neutral-700",
    }
}

#[cfg(test)]
mod tests {
    use editor_core::draft::ProjectDraft;

    use super::*;
    use crate::form::registry::{sections_for, Audience, Section, FIELDS, SECTIONS};

    /// A draft over a real committed project, so a control renders against the
    /// values the corpus actually holds.
    fn published_draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    /// Every field of `section` this audience sees, rendered.
    fn render(section: &Section, draft: &ProjectDraft, audience: Audience) -> String {
        section
            .fields_for(audience)
            .map(|field| field_row(field, draft, Mode::Editable).into_string())
            .collect()
    }

    #[test]
    fn every_shaped_field_has_a_control() {
        // The test `control`'s fallback arm names. Without it that arm's
        // `debug_assert!` is the only guard, and it is compiled out of a release
        // build — so a field given a shape in the registry with no arm here
        // renders the "not editable yet" note instead of a control, silently,
        // and a `Required` field becomes unfillable in production with nothing
        // failing in CI.
        //
        // Every section for the audience that sees the most, because rendering
        // one section covers a third of the shaped fields: `dataManagementPlan`
        // is in `access`, `provenance` and `dataPublicationYear` in `dataset`,
        // `imageCredit` in `image`.
        let draft = published_draft();
        let mut unrendered: Vec<&str> = Vec::new();
        for section in SECTIONS {
            let out = render(section, &draft, Audience::RduOnly);
            for field in section.fields_for(Audience::RduOnly) {
                let posts = match field.shape {
                    None => continue,
                    // A scalar posts under the field's own name.
                    Some(Shape::Text(_)) => out.contains(&format!(r#"name="{}""#, field.id)),
                    // A language map posts under `{field}.{tag}`, one control per
                    // offered language.
                    Some(Shape::Multilingual) => UI_LANGUAGES
                        .iter()
                        .all(|tag| out.contains(&format!(r#"name="{}.{tag}""#, field.id))),
                };
                if !posts {
                    unrendered.push(field.id);
                }
            }
        }
        assert!(
            unrendered.is_empty(),
            "these fields declare a shape but render no control that posts under their name: {unrendered:?}"
        );
    }

    #[test]
    fn every_shaped_field_is_reached_by_this_test_at_all() {
        // The canary for the test above, which asserts an *absence*: it would
        // pass just as well if `SECTIONS` reached none of the shaped fields, at
        // which point it proves nothing and nobody can tell.
        let shaped: Vec<&str> = FIELDS
            .iter()
            .filter(|field| field.is_editable())
            .map(|field| field.id)
            .collect();
        assert_eq!(shaped.len(), 11, "{shaped:?}");
        let reached: Vec<&str> = SECTIONS
            .iter()
            .flat_map(|section| section.fields_for(Audience::RduOnly))
            .filter(|field| field.is_editable())
            .map(|field| field.id)
            .collect();
        assert_eq!(reached.len(), shaped.len(), "reached {reached:?} of {shaped:?}");
    }

    #[test]
    fn every_field_states_its_obligation_inside_its_own_label() {
        // Nothing here is `required` or `aria-required` (REQ-1.9/REQ-1.10), so a
        // field's label is the only channel its obligation has: a pill rendered
        // beside the label is visible and nothing else, and a reader who tabs to
        // the control hears "Name, edit text".
        //
        // Asserted per field over every section rather than on one example,
        // because each control builder composes its own label — `text`, `year`,
        // `long_text`, `multilingual` and `stated` are five places to forget it,
        // and forgetting it in one renders identically to a sighted reader.
        let draft = published_draft();
        let mut silent: Vec<&str> = Vec::new();
        for section in SECTIONS {
            for field in section.fields_for(Audience::RduOnly) {
                let Some(obligation) = field.obligation else { continue };
                let out = field_row(field, &draft, Mode::Editable).into_string();
                // Inside the labelling element, not merely somewhere in the
                // field: the accessible name is what has to carry it.
                let named = ["label", "legend", "p"].iter().any(|tag| {
                    out.split(&format!("<{tag} "))
                        .skip(1)
                        .filter_map(|rest| rest.split_once('>'))
                        .filter_map(|(_, body)| body.split_once(&format!("</{tag}>")))
                        .any(|(body, _)| body.contains(field.label) && body.contains(obligation.label()))
                });
                if !named {
                    silent.push(field.id);
                }
            }
        }
        assert!(
            silent.is_empty(),
            "these fields carry an obligation their label does not state: {silent:?}"
        );
    }

    #[test]
    fn a_field_with_no_shape_renders_no_control_in_any_section() {
        // The other direction, and the REQ-1.7 guarantee: a field no applier
        // reads must post nothing at all, or an empty control would clear a
        // value the save was never meant to touch.
        let draft = published_draft();
        let mut posting: Vec<&str> = Vec::new();
        for section in SECTIONS {
            let out = render(section, &draft, Audience::RduOnly);
            for field in section.fields_for(Audience::RduOnly) {
                if field.shape.is_none() && out.contains(&format!(r#"name="{}""#, field.id)) {
                    posting.push(field.id);
                }
            }
        }
        assert!(posting.is_empty(), "these fields post without a declared shape: {posting:?}");
    }

    #[test]
    fn a_locked_field_posts_nothing_whatever_its_shape() {
        // Read-only means the save cannot change it, and the only way to
        // guarantee that from the markup is for the field to submit no name.
        let draft = published_draft();
        for section in sections_for(Audience::RduOnly) {
            for field in section.fields_for(Audience::RduOnly) {
                let out = field_row(field, &draft, Mode::ReadOnly).into_string();
                assert!(!out.contains("<input"), "{}: {out}", field.id);
                assert!(!out.contains("<textarea"), "{}: {out}", field.id);
                assert!(!out.contains("name="), "{}: {out}", field.id);
            }
        }
    }
}
