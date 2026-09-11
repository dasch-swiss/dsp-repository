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
//! would clear a value the save was never meant to touch.
//!
//! A note rather than silence for a field whose widget has not landed: a
//! depositor who cannot find "Keywords" in the section the published page shows
//! it in would otherwise conclude the form lost it.

use editor_core::agents::{Agent, AgentKind, AgentMatches, AgentScope};
use editor_core::draft::{ProjectDraft, UrlSlot};
use editor_core::form::{ChoiceSet, FormBody, Shape, GRANTS_KIND, REFERENCE_KIND};
use editor_core::multilingual::{DraftMultilingual, UI_LANGUAGES};
use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::checkbox_group::checkbox_group;
use mosaic_tiles::radio_group::radio_group;
use mosaic_tiles::repeatable_list::repeatable_list;
use mosaic_tiles::select::select;
use mosaic_tiles::text_field::{text_field, InputType};
use mosaic_tiles::textarea::textarea;
use mosaic_tiles::ComponentBuilder;
use platform_metadata::is_placeholder;
use platform_metadata::project::CONTRIBUTOR_ROLES;
use serde_json::Value;

use super::registry::{Field, Obligation};
use crate::form::INTENT;
use crate::pages::section::{propose_changes_intent, FIND_AGENT, PROPOSE_ORGANIZATION, PROPOSE_PERSON};

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
    /// change under the reviewer.
    ReadOnly,
    /// Values only, because RDU accepted this field in the round being
    /// answered.
    ///
    /// A third variant rather than [`Self::ReadOnly`] with a flag beside it,
    /// because the reader has to be told *which* of two reasons applies: a
    /// whole-form lock lifts when the review finishes, and this one lifts when
    /// the field is submitted again. Told the wrong one, a depositor waits for
    /// the wrong event.
    Accepted,
}

/// What a reader is told about a field RDU has already accepted.
///
/// It renders as a value with no control, so nothing about it posts and the
/// applier that would write it is not run either — the note explains a gate
/// that is enforced server-side, rather than being the gate.
const ACCEPTED_BY_RDU: &str = "RDU accepted this value, so it is fixed while you answer this review. It is submitted \
                               unchanged, and becomes editable again after the next review round.";

/// What a reader is told about a field whose widget has not landed.
const NOT_READ_YET: &str = "This field is not editable here yet. Its current value is kept unchanged when you \
                            save, and the control arrives in a later release.";

/// What is shown in place of a value the project does not have.
pub(crate) const NO_VALUE: &str = "Not set";

/// One field: its label, its obligation, and whichever of the three renderings
/// applies.
pub fn field_row(field: &Field, draft: &ProjectDraft, mode: Mode, rows: Rows<'_>) -> Markup {
    match (mode, field.shape) {
        (Mode::Editable, Some(shape)) => control(field, draft, shape, rows),
        (Mode::Accepted, _) => stated(field, draft, Some(ACCEPTED_BY_RDU)),
        // A locked field and a display-only one render the same way, which is
        // the point: neither posts, so neither can be cleared.
        (Mode::ReadOnly, _) | (_, None) => stated(field, draft, None),
    }
}

/// A field rendered as a value or as a note, with its own label above it.
///
/// Not a `<label>`: there is no control for one to point at, and a `for`
/// naming nothing is worse than no `for` at all. The heading and the value are
/// tied by proximity and by the same `field-*` treatment the tiles use, so a
/// locked form reads as the same form.
///
/// `pub(crate)`: `crate::entity` reuses this verbatim for a proposal that is
/// no longer live, rather than a second read-only renderer that could show a
/// person or organisation's values differently from a project's.
pub(crate) fn stated(field: &Field, draft: &ProjectDraft, why: Option<&str>) -> Markup {
    let editable_but_unbuilt = !field.display_only && field.shape.is_none();
    html! {
        div class="field" {
            p class="field-label" { (labelled(field)) }
            (value_display(field, draft))
            @if editable_but_unbuilt {
                p class="field-hint" { (NOT_READ_YET) }
            } @else if let Some(why) = why {
                // Instead of the hint, not beside it: the hint says what to
                // type, which is not what this reader can do.
                p class="field-hint" { (why) }
            } @else if let Some(hint) = field.hint {
                p class="field-hint" { (hint) }
            }
        }
    }
}

/// The stored value, rendered for reading.
fn value_display(field: &Field, draft: &ProjectDraft) -> Markup {
    value_markup(draft.get(field.id))
}

/// Any project member, rendered for reading.
///
/// Takes a `Value` rather than a field and a draft because the review surface
/// renders two of them side by side — a published one and a submitted one — and
/// neither is "the draft's". One rendering for both, so a reviewer comparing a
/// value against the form that produced it is comparing like with like.
///
/// A placeholder sentinel is *not* shown: `MISSING` and `CALCULATED` are the
/// platform's "no value yet" markers, filtered out of DPE's UI and of OAI-PMH's
/// output, so showing one here would be the only place in the platform that
/// presents an internal marker as a value.
pub(crate) fn value_markup(value: Option<&Value>) -> Markup {
    html! {
        @match value {
            Some(Value::String(text)) if !is_placeholder(text) && !text.trim().is_empty() => {
                p class="whitespace-pre-line text-neutral-900" { (text) }
            }
            Some(value) if is_language_map(value) => { (language_list(&as_multilingual(value))) }
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

/// A language map as the form's editing view over it, in UI-tag order.
///
/// Same construction as [`ProjectDraft::multilingual`], for a value already in
/// hand rather than one looked up by field id — a row of a repeatable field is
/// exactly that, which is why this is `pub`: `untouched_form_round_trip` builds
/// the body a row would post and has to read a row the same way the widget
/// does, or it tests a body no browser would send.
pub fn as_multilingual(value: &Value) -> DraftMultilingual {
    let contract = serde_json::from_value(value.clone()).unwrap_or_default();
    DraftMultilingual::from_contract(&contract)
}

/// Whether a stored object is a language map rather than a structured value.
///
/// Every member being a short lowercase tag holding a string is what separates
/// `{"en": "…"}` from `{"type": …, "url": …}`. A structured value is summarised
/// by its member count instead, which is honest about not rendering it rather
/// than showing a half-parsed version of it.
pub(crate) fn is_language_map(value: &Value) -> bool {
    value.as_object().is_some_and(|members| {
        !members.is_empty()
            && members
                .iter()
                .all(|(key, text)| text.is_string() && key.len() <= 3 && key.chars().all(|c| c.is_ascii_lowercase()))
    })
}

/// A read-only language map: one line per language, in the form's own order.
pub(crate) fn language_list(value: &DraftMultilingual) -> Markup {
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
pub(crate) fn language_name(tag: &str) -> &str {
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
/// `pub(crate)` because the review surface renders the *same* control over the
/// value it would commit. A second dispatch there diverged silently — a date
/// rendered as free text, and `shortDescription` lost the 200-character cap its
/// own hint promises, with nothing server-side to catch either.
///
/// The `match` is exhaustive over the ids the registry declares a shape for, and
/// the fallback is not a silent default: it renders the same note an unbuilt
/// field gets, so a field given a shape without a control here is visible rather
/// than posting under a name with no way to enter a value.
pub(crate) fn control(field: &Field, draft: &ProjectDraft, shape: Shape, rows: Rows<'_>) -> Markup {
    match field.id {
        "name" | "officialName" => text(field, draft, InputType::Text),
        // `type="text"`, not `type="url"`: a draft may hold a value that does
        // not validate, and a browser refusing to submit a half-typed
        // address would block a save that must always be possible — the same reason
        // `text` below never sets `required`.
        "dataManagementPlan" => text(field, draft, InputType::Text),
        "startDate" | "endDate" | "accessRights.embargoDate" => text(field, draft, InputType::Date),
        // Two choices, so both are visible at once and picking one is a single
        // action. A `<select>` for two options hides half the answer behind a
        // click and reads worse to a screen reader.
        "status" => radio(field, draft, choices(shape)),
        // `type="text"`, like `dataManagementPlan` and for the same reason: a draft may hold a value that does not
        // validate, and a browser refusing to submit a half-typed address would block the save.
        "url" | "secondaryUrl" => url(field, draft, slot(shape)),
        "typeOfData" | "dataLanguage" => string_list(field, draft, set(shape)),
        "keywords" | "alternativeNames" => multilingual_rows(field, draft, rows),
        "additionalMaterial" | "documentationMaterial" => string_rows(field, draft, rows),
        "contactPoint" => agent_rows(field, draft, rows),
        "attributions" => attribution_rows(field, draft, rows),
        "temporalCoverage" | "disciplines" => text_or_reference_rows(field, draft, rows, reference_types(shape)),
        "spatialCoverage" => reference_rows(field, draft, rows, reference_types(shape)),
        "publications" => publication_rows(field, draft, rows),
        "funding" => funding(field, draft, rows),
        // Four choices whose labels run to "Open Access with Restrictions", and
        // exactly one is current. Radios would be four long lines competing
        // with the fields around them.
        "accessRights.accessRights" => dropdown(field, draft, choices(shape)),
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
            stated(field, draft, None)
        }
    }
}

/// What a repeatable field needs beyond the draft: the body that was posted, and
/// where its add and remove controls submit to.
///
/// A named type rather than two arguments, because both are `Option`-ish and
/// only repeatable fields read either — a positional pair would let every other
/// control's call site scramble them silently.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rows<'a> {
    /// The posted body, when this render is answering a `POST`.
    ///
    /// **This is what keeps a freshly added row alive.** A row with no text in
    /// any language is never stored — `apply_multilingual_rows` drops it, and it
    /// must, or a file fills up with empty objects — so an added-but-unfilled
    /// row exists only in the form. The tile renders a hidden `{field}.row` for
    /// every row including a blank one, so the body carries its key back and the
    /// re-render finds it here. Nothing is held server-side between requests.
    pub posted: Option<&'a FormBody>,
    /// Base URL for the add and remove controls: `{base}/add` and
    /// `{base}/{key}/remove`.
    pub action: &'a str,
    /// The field one more blank row was just asked for.
    pub adding: Option<&'a str>,
    /// The agents an id field may refer to, for resolving an id to a name and
    /// for the shared suggestion list.
    ///
    /// `None` renders the id bare, which is what a deployment with no data
    /// directory looks like — honest, and it still round-trips.
    pub agents: Option<&'a AgentScope<'a>>,
    /// Whether an unresolved or resolved agent id also renders the controls that
    /// start an entity proposal.
    ///
    /// Defaults to `false` (via `Rows`'s `Default`) rather than being inferred
    /// from `agents.is_some()`: the entity form's own `affiliations` field reuses
    /// this same row widget for its organisation picker, and posting one of
    /// these buttons there would name an `intent` the entity route does not
    /// dispatch — a control that silently falls through to a save nothing asked
    /// for. Only the project section form, which does dispatch all three
    /// intents, turns this on.
    pub propose: bool,
}

/// The keys of the rows to render for `field`, in display order.
///
/// From the posted body when there is one, so what a depositor typed and any
/// blank row they added both survive the round trip. From the stored list
/// otherwise, keyed **positionally** — a stored list is positional, so `r0` is
/// its first row, and the key only has to stay stable for the life of one
/// rendered form: the body carries the keys back in DOM order and the applier
/// rebuilds the list from that, never from a number.
fn row_keys(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Vec<String> {
    let row_name = format!("{}.row", field.id);
    let mut keys: Vec<String> = match rows.posted {
        Some(body) if body.has(&row_name) => body.rows(field.id).into_iter().map(str::to_string).collect(),
        _ => (0..stored_rows(field, draft).len()).map(|n| format!("r{n}")).collect(),
    };
    if rows.adding == Some(field.id) {
        keys.push(next_row_key(&keys));
    }
    keys
}

/// A row key not already in use.
///
/// The smallest free `rN` rather than the next after the highest, because keys
/// arrive from the body and a removed middle row leaves a gap: appending after
/// the highest would work too, but this keeps the set dense and the accessible
/// "Remove keyword 2" labels matching what a reader counts on the page.
fn next_row_key(keys: &[String]) -> String {
    (0..=keys.len())
        .map(|n| format!("r{n}"))
        .find(|candidate| !keys.contains(candidate))
        .unwrap_or_else(|| format!("r{}", keys.len()))
}

/// The stored rows of a list-of-maps field.
fn stored_rows<'a>(field: &Field, draft: &'a ProjectDraft) -> Vec<&'a Value> {
    draft
        .get(field.id)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .collect()
}

/// One row's language map, from the body when this is a re-render and from the
/// stored list otherwise.
///
/// The two never mix: a posted body is the whole truth about what the form
/// currently holds, and a stored list is the whole truth about what a fresh
/// `GET` shows. Reading texts from one and keys from the other is how a
/// re-render ends up showing a value the depositor did not type.
fn row_value(field: &Field, draft: &ProjectDraft, rows: Rows<'_>, key: &str, position: usize) -> DraftMultilingual {
    let row_name = format!("{}.row", field.id);
    match rows.posted {
        Some(body) if body.has(&row_name) => {
            let mut value = DraftMultilingual::new();
            for (tag, text) in body.entries(&format!("{}.{key}", field.id)) {
                value.set(&tag.to_ascii_lowercase(), text);
            }
            value
        }
        _ => stored_rows(field, draft)
            .get(position)
            .map(|row| as_multilingual(row))
            .unwrap_or_default(),
    }
}

/// A list of single strings: one text control per row, added and removed by
/// server round-trip.
///
/// Shares every piece of the row protocol with [`multilingual_rows`] — the
/// hidden `{field}.row` key, the empty marker, the add and remove actions — and
/// differs only in what a row contains.
pub(crate) fn string_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let value = match rows.posted {
            Some(body) if body.has(&row_name) => body.get(&format!("{}.{key}", field.id)).unwrap_or_default(),
            _ => stored.get(position).and_then(|row| row.as_str()).unwrap_or_default(),
        };
        list = list.row(
            key,
            html! {
                ({
                    text_field(
                            format!("{}.{key}", field.id),
                            row_label(ROW_VALUE_LABEL, position),
                        )
                        .input_type(InputType::Text)
                        .value(value)
                })
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// The suffix a picker's search box posts under, appended to the control's unique name.
///
/// Read by nothing in `editor_core::form`: an applier reads the names it knows, so the query
/// rides along in the body and is never stored. That is the whole reason it needs no field of its
/// own anywhere.
const QUERY_SUFFIX: &str = "q";

/// The label on a picker's search box, and on the button that runs it.
const FIND_AGENT_LABEL: &str = "Find a person or organisation";
const FIND_AGENT_HINT: &str = "Search by name, then choose from the matches. Part of a name is enough.";
const FIND_AGENT_BUTTON: &str = "Search";

/// One agent picker: what the row refers to now, and the way to change it.
///
/// ## Why this is a search and not a list
///
/// It replaces an `<input list=>` pointed at one shared `<datalist>` of all 558 agents. That
/// control was reported as "seems to be a pull-down menu, but when I click it, nothing opens",
/// and both halves were true: Chromium draws a dropdown arrow for any `input[list]`, and it
/// filters the options against what the box already holds — which was the full id, so the only
/// thing left to offer was the value already there. A depositor also had to know `person-417` to
/// type it, and the datalist matched ids rather than the names it displayed.
///
/// A `<select>` of everything is what the datalist existed to avoid: one committed project has 56
/// contributors, so at 31.7 KB a list that would be 1.8 MB on one page. A search narrows first and
/// then offers a real `<select>`, which opens because it is a menu rather than looking like one.
///
/// ## What posts
///
/// `value_name` always, and nothing else the appliers read. Before a search it is a hidden input
/// holding the stored id, so an untouched save round-trips byte-for-byte with no lookup in
/// between — the property the old text input had and the reason its value was the id. After a
/// search the `<select>` posts under that same name, with the current choice first and selected,
/// so leaving it alone is still the identity. No applier changed for any of this.
///
/// `value_name` is shared by every funder control in a grant, which is how `apply_funding`
/// collects them into one list; `unique` is what distinguishes them for element ids and for the
/// search box, and is the same string for a control that holds only one value.
fn agent_picker(value_name: &str, unique: &str, label: &str, id: &str, row_context: &str, rows: Rows<'_>) -> Markup {
    let id = id.trim();
    let resolved = rows.agents.and_then(|agents| agents.get(id));
    let query_name = format!("{unique}.{QUERY_SUFFIX}");
    let query = rows
        .posted
        .and_then(|body| body.get(&query_name))
        .unwrap_or_default()
        .to_string();
    let matches = rows
        .agents
        .filter(|_| !query.trim().is_empty())
        .map(|agents| agents.search(&query));

    html! {
        div class="flex flex-col gap-2" {
            @match matches.as_ref().filter(|found| !found.is_empty()) {
                Some(found) => {
                    ({
                        let mut chooser = select(value_name, label)
                            .with_id(unique)
                            .selected(id);
                        chooser = match resolved {
                            Some(agent) => {
                                chooser
                                    .option(
                                        id,
                                        format!("Keep {} ({})", agent.label, agent.kind.label()),
                                    )
                            }
                            None if id.is_empty() => chooser.option("", "Nobody chosen"),
                            None => {
                                chooser
                                    .option(id, format!("Keep {id} ({UNRESOLVED_AGENT})"))
                            }
                        };
                        for agent in found
                            .offered()
                            .iter()
                            .filter(|agent| agent.id != id)
                        {
                            chooser = chooser
                                .option(
                                    &agent.id,
                                    format!("{} ({})", agent.label, agent.kind.label()),
                                );
                        }
                        chooser
                            .hint(
                                match found.truncated() {
                                    true => {
                                        format!(
                                            "{} matches, showing the first {}. Add a word to narrow it.",
                                            found.total(),
                                            found.offered().len(),
                                        )
                                    }
                                    false => {
                                        format!(
                                            "{} match{}.",
                                            found.total(),
                                            if found.total() == 1 { "" } else { "es" },
                                        )
                                    }
                                },
                            )
                    })
                }
                None => {
                    input type="hidden" name=(value_name) value=(id);
                    p class="text-sm" {
                        (label)
                        ": "
                        @match resolved {
                            Some(agent) => {
                                strong { (agent.label) }
                                " ("
                                (agent.kind.label())
                                ")"
                            }
                            None if id.is_empty() => {
                                span class="text-neutral-600" { "nobody chosen yet" }
                            }
                            None => {
                                strong { (id) }
                                " — "
                                (UNRESOLVED_AGENT)
                            }
                        }
                    }
                }
            }
            div class="flex flex-wrap items-end gap-2" {
                ({
                    text_field(&query_name, FIND_AGENT_LABEL)
                        .input_type(InputType::Text)
                        .value(&query)
                        .hint(FIND_AGENT_HINT)
                        .with_id(format!("{unique}-{QUERY_SUFFIX}"))
                })
                ({
                    button(FIND_AGENT_BUTTON)
                        .variant(ButtonVariant::Secondary)
                        .button_type(ButtonType::Submit)
                        .name_value(INTENT, FIND_AGENT)
                        .aria_label(format!("{FIND_AGENT_LABEL} ({row_context})"))
                })
            }
            // Said here and not only at submit, and said for a search as well as for a stored id:
            // the form is where it can be fixed.
            @if matches.as_ref().is_some_and(AgentMatches::is_empty) {
                p class="text-sm" {
                    "Nothing matches “"
                    (query)
                    "”. Check the spelling, or propose a new entry below."
                }
            }
        }
    }
}

/// A list of agent ids as editable rows, each resolved to a name through [`agent_picker`].
pub(crate) fn agent_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let id = match rows.posted {
            Some(body) if body.has(&row_name) => body.get(&format!("{}.{key}", field.id)).unwrap_or_default(),
            _ => stored.get(position).and_then(|row| row.as_str()).unwrap_or_default(),
        };
        let name = format!("{}.{key}", field.id);
        let resolved = rows.agents.and_then(|agents| agents.get(id.trim()));
        let row_context = format!("row {}", position + 1);
        let body = html! {
            div class="flex flex-col gap-2" {
                ({
                    agent_picker(
                        &name,
                        &name,
                        &row_label(AGENT_ID_LABEL, position),
                        id,
                        &row_context,
                        rows,
                    )
                })
                @if rows.propose { (propose_controls(resolved, id, &row_context)) }
            }
        };
        list = list.row(key, body);
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// The controls that start an entity proposal, beside an agent picker whose id does not
/// resolve or does: named submits on the section's own form, exactly like
/// `save`/`submit` — `sections.rs` already dispatches all three intents, so
/// none of this needs a route of its own.
///
/// `id` empty renders nothing: an unfilled picker is not "no match", it is
/// "nothing typed yet", and offering to propose an entity for it would be
/// offering to propose nobody.
///
/// `row_context` is spliced into each button's accessible name, the same
/// reason [`row_label`] numbers a row's own control: several rows' propose
/// buttons would otherwise share one indistinguishable name.
/// What the propose-changes button says, which names **the entity** and never the row it sits in.
///
/// A bare "Propose changes" read as an offer to propose whatever else the row held. On a
/// contributor row that is the roles, and a reviewer took it exactly that way: the roles are
/// project data, saved by "Save draft" like any other field, and `proposals::check_person` goes as
/// far as refusing a project-role word in a person's `jobTitles` precisely because a role is not
/// part of the person. Nothing behaved wrongly; the button's name was the whole problem.
const fn propose_changes_label(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Person => "Propose changes to this person's details",
        AgentKind::Organization => "Propose changes to this organisation's details",
    }
}

fn propose_controls(resolved: Option<&Agent>, id: &str, row_context: &str) -> Markup {
    if id.trim().is_empty() {
        return html! {};
    }
    html! {
        @match resolved {
            None => {
                div class="flex flex-wrap gap-2" {
                    ({
                        button("Propose a new person")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, PROPOSE_PERSON)
                            .aria_label(format!("Propose a new person ({row_context})"))
                    })
                    ({
                        button("Propose a new organisation")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, PROPOSE_ORGANIZATION)
                            .aria_label(
                                format!("Propose a new organisation ({row_context})"),
                            )
                    })
                }
            }
            Some(agent) => {
                @let label = propose_changes_label(agent.kind);
                div class="flex flex-wrap items-center gap-2" {
                    ({
                        button(label)
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, propose_changes_intent(id))
                            .aria_label(format!("{label} ({row_context})"))
                    })
                }
            }
        }
    }
}

/// A list of contributor rows: an agent picker plus that contributor's roles.
///
/// The roles are a checkbox group over the offered vocabulary **unioned with whatever this row
/// already holds**, plus a text input for one more. Same shape as `dataLanguage` and for the same
/// reason: the corpus spells the same role several ways, so a closed offer would drop a project's
/// own wording on the first save.
///
/// Both the group and the "add another" input post under
/// `{field}.{key}.role`, so `FormBody::all` collects them together and no
/// second wire name is needed.
fn attribution_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let posted = rows.posted.filter(|body| body.has(&row_name));
        let prefix = format!("{}.{key}", field.id);
        let contributor = match posted {
            Some(body) => body.get(&format!("{prefix}.contributor")).unwrap_or_default().to_string(),
            None => stored
                .get(position)
                .and_then(|row| row.get("contributor"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
        // Blanks are dropped, because the "add another role" input below posts under this same
        // name and an untouched one sends an empty string. Read back as a role this row holds, it
        // joined the option union as an unlabelled checkbox — ticked, since it is also in `held` —
        // which is what a depositor saw appear after any re-render that keeps the posted body.
        // `form::resolve_against` already discards it on the way into the draft, so this is the
        // render catching up with what is actually stored.
        let held: Vec<String> = match posted {
            Some(body) => body
                .all(&format!("{prefix}.role"))
                .filter(|role| !role.trim().is_empty())
                .map(str::to_string)
                .collect(),
            None => stored
                .get(position)
                .and_then(|row| row.get("contributorType"))
                .and_then(Value::as_array)
                .map(|roles| roles.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default(),
        };
        let mut options: Vec<&str> = CONTRIBUTOR_ROLES.to_vec();
        for role in &held {
            if !options.contains(&role.as_str()) {
                options.push(role);
            }
        }
        let contributor_name = format!("{prefix}.contributor");
        let resolved = rows.agents.and_then(|agents| agents.get(contributor.trim()));
        let row_context = format!("row {}", position + 1);
        // The propose controls go **below** the role controls, not between them and the picker.
        // Sandwiched there they read as an offer to propose the roles underneath them, which is
        // how a reviewer read them — and the roles are project data that "Save draft" stores, with
        // nothing to propose. Last in the row they follow everything the row is about, which is
        // what they act on: the entity the picker names.
        let body = html! {
            div class="flex flex-col gap-3" {
                ({
                    agent_picker(
                        &contributor_name,
                        &contributor_name,
                        &row_label(AGENT_ID_LABEL, position),
                        &contributor,
                        &row_context,
                        rows,
                    )
                })
                ({
                    checkbox_group(format!("{prefix}.role"), ROLE_GROUP_LABEL)
                        .options(options.iter().map(|role| (*role, *role)))
                        .checked(held.iter().map(String::as_str))
                })
                ({
                    text_field(format!("{prefix}.role"), ADD_ROLE_LABEL)
                        .input_type(InputType::Text)
                        .hint(ADD_ROLE_HINT)
                        .with_id(format!("{prefix}.role-add"))
                })
                @if rows.propose { (propose_controls(resolved, &contributor, &row_context)) }
            }
        };
        list = list.row(key, body);
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// The reference types a shape offers, or none for any other shape.
const fn reference_types(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::TextOrReferenceRows(types) | Shape::ReferenceRows(types) => types,
        _ => &[],
    }
}

/// A list of rows that are each an authority reference or free text.
///
/// **Both branches are always in the DOM, and the inactive one is `hidden`
/// rather than `disabled`.** That is the whole mechanism: a `hidden` input is
/// still submitted, so the server receives both candidates plus the
/// discriminant and can keep the unchosen one — a depositor who switches to a
/// reference and back finds their text still there. `disabled` would submit
/// nothing and lose it.
///
/// Switching branches is a **server round-trip**: the chosen branch follows what the row holds, so
/// picking the other radio and saving re-renders with it visible. The control then behaves
/// identically with and without JavaScript, and this form stays free of signal bindings.
fn text_or_reference_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>, types: &[&str]) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let posted = rows.posted.filter(|body| body.has(&row_name));
        let prefix = format!("{}.{key}", field.id);
        let held = stored.get(position);

        // A stored row is a reference when it has a URL; that is the same test
        // `ProjectDraft::coverage_shapes` applies, and serde's own attempt
        // order agrees — `AuthorityFileReference` is declared first.
        let is_reference = match posted {
            Some(body) => body.get(&format!("{prefix}.kind")) == Some(REFERENCE_KIND),
            None => held.and_then(|row| row.get("url")).is_some(),
        };
        let reference_value = |member: &str| -> String {
            match posted {
                Some(body) => body.get(&format!("{prefix}.ref.{member}")).unwrap_or_default().to_string(),
                None => held
                    .and_then(|row| row.get(if member == "label" { "text" } else { member }))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }
        };
        let texts = match posted {
            Some(body) => {
                let mut value = DraftMultilingual::new();
                for (tag, text) in body.entries(&format!("{prefix}.text")) {
                    value.set(&tag.to_ascii_lowercase(), text);
                }
                value
            }
            None => held
                .filter(|_| !is_reference)
                .map(|row| as_multilingual(row))
                .unwrap_or_default(),
        };
        let current = if is_reference { REFERENCE_KIND } else { TEXT_KIND };

        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-3" {
                    ({
                        radio_group(format!("{prefix}.kind"), VARIANT_LABEL)
                            .options([
                                (REFERENCE_KIND, "A recognised entry"),
                                (TEXT_KIND, "My own wording"),
                            ])
                            .selected(current)
                            .hint(VARIANT_HINT)
                            .inline()
                    })
                    div hidden[!is_reference] class="flex flex-col gap-2" {
                        ({
                            select(format!("{prefix}.ref.type"), REFERENCE_TYPE_LABEL)
                                .options(types.iter().map(|kind| (*kind, *kind)))
                                .selected(reference_value("type"))
                        })
                        ({
                            text_field(format!("{prefix}.ref.url"), REFERENCE_URL_LABEL)
                                .input_type(InputType::Text)
                                .value(reference_value("url"))
                        })
                        ({
                            text_field(
                                    format!("{prefix}.ref.label"),
                                    REFERENCE_LABEL_LABEL,
                                )
                                .input_type(InputType::Text)
                                .value(reference_value("label"))
                        })
                    }
                    div hidden[is_reference] class="flex flex-col gap-2" {
                        @for tag in UI_LANGUAGES.iter().copied().chain(texts.extra_tags()) {
                            ({
                                text_field(
                                        format!("{prefix}.text.{tag}"),
                                        language_name(tag),
                                    )
                                    .input_type(InputType::Text)
                                    .value(texts.get(tag).unwrap_or_default())
                            })
                        }
                    }
                }
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// One row's three reference controls, shared by every field that renders a
/// reference so their names and labels cannot drift.
fn reference_controls(prefix: &str, types: &[&str], value: impl Fn(&str) -> String) -> Markup {
    html! {
        ({
            select(format!("{prefix}.ref.type"), REFERENCE_TYPE_LABEL)
                .options(types.iter().map(|kind| (*kind, *kind)))
                .selected(value("type"))
        })
        ({
            text_field(format!("{prefix}.ref.url"), REFERENCE_URL_LABEL)
                .input_type(InputType::Text)
                .value(value("url"))
        })
        ({
            text_field(format!("{prefix}.ref.label"), REFERENCE_LABEL_LABEL)
                .input_type(InputType::Text)
                .value(value("label"))
        })
    }
}

/// One row's stored or posted member, for a row of flat members.
fn row_member(field: &Field, stored: Option<&Value>, rows: Rows<'_>, prefix: &str, member: &str) -> String {
    let row_name = format!("{}.row", field.id);
    match rows.posted.filter(|body| body.has(&row_name)) {
        Some(body) => body.get(&format!("{prefix}.{member}")).unwrap_or_default().to_string(),
        None => stored
            .and_then(|row| row.get(member))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

/// A list of authority references - the reference half of
/// [`text_or_reference_rows`], with no variant to choose.
pub(crate) fn reference_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>, types: &[&str]) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let prefix = format!("{}.{key}", field.id);
        let held = stored.get(position).copied();
        let body = reference_controls(&prefix, types, |member| {
            row_member(field, held, rows, &prefix, if member == "label" { "text" } else { member })
        });
        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-2" { (body) }
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// A list of bibliographic references: a citation and an optional identifier.
fn publication_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let prefix = format!("{}.{key}", field.id);
        let held = stored.get(position).copied();
        let row_name = format!("{}.row", field.id);
        let text = row_member(field, held, rows, &prefix, "text");
        // The identifier is nested one deeper in the contract, so it cannot go
        // through `row_member`.
        let pid = match rows.posted.filter(|body| body.has(&row_name)) {
            Some(body) => body.get(&format!("{prefix}.pid")).unwrap_or_default().to_string(),
            None => held
                .and_then(|row| row.get("pid")?.get("url"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-2" {
                    ({
                        textarea(
                                format!("{prefix}.text"),
                                row_label(CITATION_LABEL, position),
                            )
                            .rows(2)
                            .value(text)
                    })
                    ({
                        text_field(
                                format!("{prefix}.pid"),
                                row_label(PID_LABEL, position),
                            )
                            .input_type(InputType::Text)
                            .value(pid)
                            .hint(PID_HINT)
                    })
                }
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// Funding: either a list of grants or one free text.
///
/// The discriminant is on the **field**, not per row, so the chooser sits above
/// the list rather than inside each row. Both branches stay in the DOM with the
/// inactive one `hidden`, for the reason [`text_or_reference_rows`] gives.
fn funding(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let stored = stored_rows(field, draft);
    let is_grants = match rows.posted.filter(|body| body.has(&format!("{}.kind", field.id))) {
        Some(body) => body.get(&format!("{}.kind", field.id)) == Some(GRANTS_KIND),
        // A string is the free-text variant; anything else, including absent, renders as grants, which is what
        // almost every project holds.
        None => !draft.get(field.id).is_some_and(Value::is_string),
    };
    let free_text = match rows.posted {
        Some(body) => body.get(&format!("{}.text", field.id)).unwrap_or_default().to_string(),
        None => draft.get(field.id).and_then(Value::as_str).unwrap_or_default().to_string(),
    };

    let keys = row_keys(field, draft, rows);
    let action = format!("{}/{}", rows.action, field.id);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun("grant");
    for (position, key) in keys.iter().enumerate() {
        let prefix = format!("{}.{key}", field.id);
        let held = stored.get(position).copied();
        let row_name = format!("{}.row", field.id);
        // Blanks are dropped for the reason `attribution_rows` gives, and here they *accumulated*:
        // the render appends its own trailing blank below, so reading the posted one back as a
        // held funder grew the row by one control on every re-render that keeps the body.
        let funders: Vec<String> = match rows.posted.filter(|body| body.has(&row_name)) {
            Some(body) => body
                .all(&format!("{prefix}.funder"))
                .filter(|id| !id.trim().is_empty())
                .map(str::to_string)
                .collect(),
            None => held
                .and_then(|grant| grant.get("funders"))
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default(),
        };
        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-2" {
                    // One control per stored funder plus a blank, all posting
                    // under one name so `FormBody::all` collects them. Adding
                    // several funders takes a save each, which is the same
                    // price adding a row costs.
                    @for (index, id) in funders.iter().chain(std::iter::once(&String::new())).enumerate() {
                        ({
                            funder_control(
                                &prefix,
                                index,
                                id,
                                rows.agents.and_then(|agents| agents.get(id.trim())),
                                &format!("funder {} of grant {}", index + 1, position + 1),
                                rows,
                            )
                        })
                    }
                    ({
                        text_field(format!("{prefix}.number"), GRANT_NUMBER_LABEL)
                            .input_type(InputType::Text)
                            .value(row_member(field, held, rows, &prefix, "number"))
                    })
                    ({
                        text_field(format!("{prefix}.name"), GRANT_NAME_LABEL)
                            .input_type(InputType::Text)
                            .value(row_member(field, held, rows, &prefix, "name"))
                    })
                    ({
                        text_field(format!("{prefix}.url"), GRANT_URL_LABEL)
                            .input_type(InputType::Text)
                            .value(row_member(field, held, rows, &prefix, "url"))
                    })
                }
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        div class="field field-group" {
            ({
                radio_group(format!("{}.kind", field.id), FUNDING_VARIANT_LABEL)
                    .options([
                        (GRANTS_KIND, "Grants"),
                        (FUNDING_TEXT_KIND, "A note instead"),
                    ])
                    .selected(if is_grants { GRANTS_KIND } else { FUNDING_TEXT_KIND })
                    .hint(FUNDING_VARIANT_HINT)
                    .inline()
            })
            div hidden[!is_grants] { (list) }
            div hidden[is_grants] {
                ({
                    text_field(format!("{}.text", field.id), FUNDING_TEXT_LABEL)
                        .input_type(InputType::Text)
                        .value(free_text)
                        .hint(FUNDING_TEXT_HINT)
                })
            }
        }
    }
}

/// One funder control, plus the propose controls beside it when `propose` is set.
///
/// Its own function rather than inline in the funder loop, per this repo's rule on a nested
/// `html!` `maudfmt` cannot see (a block passed where a call argument is expected).
fn funder_control(
    prefix: &str,
    index: usize,
    id: &str,
    resolved: Option<&Agent>,
    row_context: &str,
    rows: Rows<'_>,
) -> Markup {
    let label = match id.trim().is_empty() && index > 0 {
        true => ADD_FUNDER_LABEL,
        false => FUNDER_LABEL,
    };
    // Every funder in a grant posts under one `{prefix}.funder`, which is how `apply_funding`
    // collects them into a list — so the value name is shared and only the `unique` differs,
    // which is what keeps each control's own search box and element ids apart.
    let value_name = format!("{prefix}.funder");
    let unique = format!("{prefix}.funder-{index}");
    html! {
        div class="flex flex-col gap-2" {
            (agent_picker(&value_name, &unique, label, id, row_context, rows))
            @if rows.propose { (propose_controls(resolved, id, row_context)) }
        }
    }
}

/// Labels for the publication, funding and grant controls.
const CITATION_LABEL: &str = "Reference";
const PID_LABEL: &str = "Persistent identifier";
const PID_HINT: &str = "A DOI or other stable link, if the publication has one.";
const FUNDER_LABEL: &str = "Funder";
const ADD_FUNDER_LABEL: &str = "Add another funder";
const GRANT_NUMBER_LABEL: &str = "Grant number";
const GRANT_NAME_LABEL: &str = "Programme";
const GRANT_URL_LABEL: &str = "Link";
const FUNDING_VARIANT_LABEL: &str = "How funding is recorded";
const FUNDING_VARIANT_HINT: &str = "Grants are listed one by one, each with its funder. A note is a single line, \
                                    for a project with nothing to list. Switching saves the form, and neither is \
                                    lost.";
const FUNDING_TEXT_LABEL: &str = "Funding note";
const FUNDING_TEXT_HINT: &str = "For example, \"No funding\".";

/// The discriminant value meaning "a note" for funding, the counterpart of
/// [`GRANTS_KIND`].
const FUNDING_TEXT_KIND: &str = "text";

/// The discriminant value meaning "free text", the counterpart of
/// [`REFERENCE_KIND`].
const TEXT_KIND: &str = "text";

/// The legend on a row's variant chooser.
const VARIANT_LABEL: &str = "How this is recorded";

/// Why the choice exists, in the terms a depositor can act on.
const VARIANT_HINT: &str = "A recognised entry carries a link the repository can resolve, which is what makes the \
                            value comparable across projects. Your own wording is kept as typed, per language. \
                            Switching saves the form, and nothing you have entered in either is lost.";

/// The labels on a reference branch's three controls.
const REFERENCE_TYPE_LABEL: &str = "Source";
const REFERENCE_URL_LABEL: &str = "Link";
const REFERENCE_LABEL_LABEL: &str = "Shown as";

/// The legend on a contributor row's role group.
const ROLE_GROUP_LABEL: &str = "Roles";

/// The label on the input that adds a role the offer does not carry.
const ADD_ROLE_LABEL: &str = "Add another role";

/// Why that input is there. Worth saying, because the group above it already
/// looks like the whole answer.
const ADD_ROLE_HINT: &str = "For a role the list does not offer. Type it and save; it joins the list above.";

/// The label on an agent row's control.
const AGENT_ID_LABEL: &str = "Person or organisation";

/// What is shown beside an id that resolves to nobody.
///
/// Said here as well as at submit, because the form is where it can be fixed:
/// told only at submit, a depositor would have to find which of 56 rows the
/// refusal meant.
const UNRESOLVED_AGENT: &str = "No person or organisation with this id — pick one from the list.";

/// A row control's label, with the row's own position in it.
///
/// Five rows whose value controls are all called "Link" are five
/// indistinguishable entries in a screen reader's form-field list, and stepping
/// through the list in document order is the only way to tell row 3 from row 5.
/// The repeatable-list tile already numbers its Remove buttons for exactly this
/// reason; this gives the row's own control the same treatment.
fn row_label(label: &str, position: usize) -> String {
    format!("{label} {}", position + 1)
}

/// The label on a single-value row's control.
///
/// A row's own label, not the field's: the field is named by the list's
/// `<legend>`, and every control still needs an accessible name of its own —
/// five rows all called "Additional material" would read identically.
const ROW_VALUE_LABEL: &str = "Link";

/// What one row of a repeatable field is called, for the accessible "Remove
/// <noun> 2" labels the tile builds.
///
/// The field's label singularised crudely, because the alternative is a second
/// vocabulary in the registry for the sake of one word per field.
fn row_noun(field: &Field) -> String {
    let label = field.label.to_lowercase();
    label.strip_suffix('s').map_or(label.clone(), str::to_string)
}

/// A list of language maps: one multilingual group per row, added and removed by
/// server round-trip.
fn multilingual_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    for (position, key) in keys.iter().enumerate() {
        let value = row_value(field, draft, rows, key, position);
        let prefix = format!("{}.{key}", field.id);
        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-2" {
                    @for tag in UI_LANGUAGES.iter().copied().chain(value.extra_tags()) {
                        ({
                            text_field(format!("{prefix}.{tag}"), language_name(tag))
                                .input_type(InputType::Text)
                                .value(value.get(tag).unwrap_or_default())
                        })
                    }
                }
            },
        );
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    html! {
        (list)
    }
}

/// The values a [`Shape::Choice`] offers, or none for any other shape.
///
/// Unreachable while the registry and [`control`]'s dispatch agree, which
/// `tests::every_shaped_field_has_a_control` pins. Empty rather than a panic,
/// for the same reason `control`'s own fallback arm renders a note: a group with
/// no options is a visible gap in this file, and taking the section down for it
/// would hide every other field too.
const fn choices(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::Choice(values) => values,
        _ => &[],
    }
}

/// Which choice set a shape names, defaulting to an empty closed one.
///
/// Unreachable while the registry and [`control`]'s dispatch agree, which
/// `tests::every_shaped_field_has_a_control` pins. Closed-and-empty is the
/// fail-safe of the two: it renders a group with no options, which is a visible
/// gap, where an open one would accept anything a hand-built body sent.
const fn set(shape: Shape) -> ChoiceSet {
    match shape {
        Shape::StringList(set) => set,
        _ => ChoiceSet::Closed(&[]),
    }
}

/// The values stored in a list field, in the order the project holds them.
fn stored_list<'a>(field: &Field, draft: &'a ProjectDraft) -> Vec<&'a str> {
    draft
        .get(field.id)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// A list of strings as a checkbox group, plus a way in for a value the offer
/// does not carry.
///
/// **The options are the offer unioned with whatever the project already holds**, the same rule the
/// multilingual widget follows and for the same reason: a value with no control posts nothing, so a
/// list rebuilt from the body would drop it — and `dataLanguage` holds far more tags than the UI
/// offers.
///
/// An open set also gets a text input **named after the field itself**, so a typed value arrives as
/// one more repeated value and `apply_string_list` reads it with the rest. No route and no
/// client-side splicing: the next render finds it in the stored list and gives it a checkbox.
fn string_list(field: &Field, draft: &ProjectDraft, set: ChoiceSet) -> Markup {
    let stored = stored_list(field, draft);
    let mut options: Vec<&str> = set.offered().to_vec();
    for value in &stored {
        if !options.contains(value) {
            options.push(value);
        }
    }

    let mut group = checkbox_group(field.id, labelled(field))
        .options(options.iter().map(|value| (*value, label_for(field, value))))
        .checked(stored.iter().copied());
    if let Some(hint) = field.hint {
        group = group.hint(hint);
    }
    html! {
        (group)
        @if matches!(set, ChoiceSet::Open(_)) {
            ({
                text_field(field.id, ADD_ANOTHER)
                    .input_type(InputType::Text)
                    .hint(ADD_ANOTHER_HINT)
                    .with_id(format!("{}-add", field.id))
            })
        }
    }
}

/// What the "add one more" input is called on an open list.
const ADD_ANOTHER: &str = "Add another";

/// Why that input is there, and what happens to what goes in it.
const ADD_ANOTHER_HINT: &str = "Type one more value and save — it is added to the list above, where you can then \
                                untick it.";

/// A list value's label. Language tags get their language name, because `cop`
/// and `gez` are not readable as they stand; anything else is already prose.
fn label_for(field: &Field, value: &str) -> String {
    if field.id == "dataLanguage" {
        let name = language_name(value);
        if name == value {
            return value.to_string();
        }
        return format!("{name} ({value})");
    }
    value.to_string()
}

/// Which URL slot a shape names, defaulting to the secondary.
///
/// Unreachable while the registry and [`control`]'s dispatch agree, which
/// `tests::every_shaped_field_has_a_control` pins. The secondary is the safer
/// fallback of the two: it is the depositor's own field, so a control wired to
/// it by mistake cannot show or overwrite the RDU-only DaSCH address.
const fn slot(shape: Shape) -> UrlSlot {
    match shape {
        Shape::Url(slot) => slot,
        _ => UrlSlot::Secondary,
    }
}

/// One of the project's two URLs, as a plain text control.
///
/// Reads through [`ProjectDraft::url_slot`], so it renders whichever
/// representation the project stores the pair in without knowing which. A
/// placeholder sentinel renders empty, for the same reason [`scalar_value`]
/// does it — two committed projects hold `url: ["MISSING"]`.
fn url(field: &Field, draft: &ProjectDraft, slot: UrlSlot) -> Markup {
    let value = draft.url_slot(slot).filter(|text| !is_placeholder(text)).unwrap_or_default();
    let mut control = text_field(field.id, labelled(field)).input_type(InputType::Text).value(value);
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    html! {
        (control)
    }
}

/// A closed choice as a radio group: every option visible, one current.
///
/// The wire value doubles as the label, because both vocabularies are already
/// depositor-facing prose — `Ongoing`, `Open Access with Restrictions`. A
/// separate label table would be a second thing to keep in step with
/// `platform_metadata`'s slices for no gain.
fn radio(field: &Field, draft: &ProjectDraft, values: &[&str]) -> Markup {
    let mut control = radio_group(field.id, labelled(field))
        .options(values.iter().map(|value| (*value, *value)))
        .inline();
    if let Some(current) = draft.get(field.id).and_then(Value::as_str) {
        control = control.selected(current);
    }
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    html! {
        (control)
    }
}

/// A closed choice as a `<select>`.
///
/// **No placeholder option.** The contract types these as enums with no unset
/// variant, so "nothing chosen" is not a state a project can hold — and
/// `apply_choice` ignores a value outside the offered set, so a placeholder
/// would be an option that silently does nothing when picked. A project that
/// somehow holds no value renders with nothing selected, which the browser
/// shows as the first option; the stored value is unchanged until a real pick
/// posts one.
fn dropdown(field: &Field, draft: &ProjectDraft, values: &[&str]) -> Markup {
    let mut control = select(field.id, labelled(field)).options(values.iter().map(|value| (*value, *value)));
    if let Some(current) = draft.get(field.id).and_then(Value::as_str) {
        control = control.selected(current);
    }
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    html! {
        (control)
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
    // be missing anything, and a browser refusing to save one is the
    // opposite of a save that must always be possible. The obligation is stated in words
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
pub(crate) fn multilingual(field: &Field, draft: &ProjectDraft, rows: u32) -> Markup {
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
/// may be missing anything — which leaves the accessible name as the
/// only channel the tier has. As a sibling the pill was visible and nothing
/// else: a reader tabbing to the control heard "Name, edit text".
///
/// `pub(crate)`: `crate::entity`'s own scalar control reuses this so a person or
/// organisation's fields carry the pill the same way a project's do.
pub(crate) fn labelled(field: &Field) -> Markup {
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
            .map(|field| field_row(field, draft, Mode::Editable, Rows::default()).into_string())
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
                    // A group posts under the field's name from each of its
                    // controls, so the name appearing at all is the same
                    // evidence a scalar gives.
                    Some(Shape::Choice(_)) => out.contains(&format!(r#"name="{}""#, field.id)),
                    // A URL slot posts under the field's own name, like a
                    // scalar — the slot decides where it is *stored*, not what
                    // it is called on the wire.
                    Some(Shape::Url(_) | Shape::StringList(_)) => out.contains(&format!(r#"name="{}""#, field.id)),
                    // A repeatable field posts its rows under `{field}.row`,
                    // and posts that name even when the list is empty — the
                    // marker is what lets a depositor clear the last row.
                    Some(
                        Shape::MultilingualRows
                        | Shape::StringRows
                        | Shape::AgentRows
                        | Shape::AttributionRows
                        | Shape::TextOrReferenceRows(_)
                        | Shape::ReferenceRows(_)
                        | Shape::PublicationRows,
                    ) => out.contains(&format!(r#"name="{}.row""#, field.id)),
                    // Funding's discriminant is on the field rather than the
                    // row, and the row marker appears only on the grants
                    // branch, so the discriminant is what always posts.
                    Some(Shape::FundingRows) => out.contains(&format!(r#"name="{}.kind""#, field.id)),
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
        // Every editable field, since none is left without a control: the count
        // is `FIELDS` minus the six display-only ones.
        // A count rather than "more than none": this is the canary for a test
        // that asserts an absence, so it has to move deliberately as shapes
        // land rather than drifting.
        assert_eq!(shaped.len(), 29, "{shaped:?}");
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
        // Nothing here is `required` or `aria-required`, so a
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
                let out = field_row(field, &draft, Mode::Editable, Rows::default()).into_string();
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
        // The other direction, and the guarantee that a draft carries what the editor does not
        // manage: a field no applier reads must post nothing at all, or an empty control would
        // clear a value the save was never meant to touch.
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
                let out = field_row(field, &draft, Mode::ReadOnly, Rows::default()).into_string();
                assert!(!out.contains("<input"), "{}: {out}", field.id);
                assert!(!out.contains("<textarea"), "{}: {out}", field.id);
                assert!(!out.contains("name="), "{}: {out}", field.id);
            }
        }
    }

    /// The committed agent set, for a picker that actually has something to resolve against.
    fn agent_corpus() -> editor_core::agents::Agents {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data");
        let (agents, errors) = editor_core::agents::Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert!(errors.is_empty(), "the committed agent set should load: {errors:?}");
        agents
    }

    #[test]
    fn propose_controls_appear_beside_an_unresolved_id_and_not_beside_a_resolved_one() {
        // The picker cannot know which kind was meant, so both propose buttons offer
        // beside an id that resolves to nobody; a resolved id offers only "propose changes".
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008", "person-999999"]));
        let field = crate::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: true,
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains("Propose a new person"), "{out}");
        assert!(out.contains("Propose a new organisation"), "{out}");
        assert!(out.contains("Propose changes"), "{out}");
        // The resolved id rides in the button's own value, so only the activated one posts it.
        assert!(out.contains(r#"value="propose-changes:organization-008""#), "{out}");
        // And nothing carries it in a field every row would post regardless of which button was
        // clicked — the shape that made row 12's button propose a change to row 1's entity.
        assert!(!out.contains(r#"name="propose.entity""#), "{out}");
    }

    #[test]
    fn propose_controls_are_absent_when_the_row_widget_does_not_opt_in() {
        // `Rows::propose` defaults to `false` — the entity form's own `affiliations` field reuses
        // this same widget for its organisation picker, and must not offer a control its route
        // does not dispatch.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["person-999999"]));
        let field = crate::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: false,
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(!out.contains("Propose"), "{out}");
    }

    /// Exactly what a browser posts for one contributor row: the hidden row key, the id, the
    /// ticked role, and the untouched "add another role" input, which sends an empty string under
    /// the same name.
    fn posted_contributor_row() -> editor_core::form::FormBody {
        editor_core::form::FormBody::from_pairs(vec![
            ("attributions.row".to_string(), "r0".to_string()),
            ("attributions.r0.contributor".to_string(), "person-001".to_string()),
            ("attributions.r0.role".to_string(), "Author".to_string()),
            ("attributions.r0.role".to_string(), String::new()),
        ])
    }

    #[test]
    fn a_picker_shows_no_menu_until_a_search_and_then_keeps_the_current_choice_first() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = crate::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let fresh = Rows {
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: true,
        };

        // Unsearched: the id posts from a hidden input, so an untouched save is the identity, and
        // the row says in words who it refers to rather than showing a bare id.
        let quiet = agent_rows(field, &draft, fresh).into_string();
        assert!(
            quiet.contains(r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#),
            "{quiet}"
        );
        assert!(quiet.contains("Dokumentationsbibliothek St. Moritz"), "{quiet}");
        assert!(!quiet.contains("<select"), "no menu before a search: {quiet}");

        // Searched: a real `<select>` posting under the same name, with what the row already
        // holds first and selected — so submitting without touching it still changes nothing.
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("contactPoint.r0.q".to_string(), "Universität".to_string()),
        ]);
        let searched = agent_rows(field, &draft, Rows { posted: Some(&body), ..fresh }).into_string();
        assert!(
            searched
                .contains(r#"<select class="field-input field-select" id="contactPoint.r0" name="contactPoint.r0""#),
            "a real menu, posting under the name the applier already reads: {searched}"
        );
        assert!(
            searched.contains(r#"<option value="organization-008" selected>Keep Dokumentationsbibliothek St. Moritz (Organisation)</option>"#),
            "the current choice is first and selected: {searched}"
        );
        // And the query survives the round trip, or the box would clear under the matches.
        assert!(searched.contains(r#"value="Universität""#), "{searched}");
        assert!(!searched.contains("<datalist"), "{searched}");
    }

    #[test]
    fn a_search_that_matches_nobody_says_so_rather_than_offering_an_empty_menu() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = crate::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("contactPoint.r0.q".to_string(), "zzzznobodyzzzz".to_string()),
        ]);
        let out = agent_rows(
            field,
            &draft,
            Rows {
                posted: Some(&body),
                action: "/x",
                adding: None,
                agents: Some(&scope),
                propose: true,
            },
        )
        .into_string();
        assert!(!out.contains("<select"), "an empty menu is worse than a sentence: {out}");
        assert!(out.contains("Nothing matches"), "{out}");
        // The stored id is still what posts, so a fruitless search loses nothing.
        assert!(
            out.contains(r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#),
            "{out}"
        );
    }

    #[test]
    fn a_contributor_rows_propose_control_follows_its_roles_and_names_the_entity() {
        // Between the picker and the roles, a bare "Propose changes" read as an offer to propose
        // the roles below it. It is not: roles are project data that a save stores, and
        // `proposals::check_person` refuses a project-role word in a person's `jobTitles` for
        // exactly that reason. So the button goes last and says what it acts on.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let field = crate::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows {
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: true,
        };

        let out = attribution_rows(field, &draft, rows).into_string();
        let roles = out.find(ADD_ROLE_LABEL).expect("the add-another-role control");
        let propose = out.find("Propose changes").expect("the propose control");
        assert!(propose > roles, "the propose control must follow the role controls: {out}");
        // And it names the entity's kind, so the scope of the button is in the button.
        assert!(out.contains("Propose changes to this person's details"), "{out}");
    }

    #[test]
    fn a_propose_control_beside_an_organisation_says_organisation() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = crate::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: true,
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains("Propose changes to this organisation's details"), "{out}");
        assert!(!out.contains("person's details"), "{out}");
    }

    #[test]
    fn the_add_another_role_input_does_not_come_back_as_an_unlabelled_checkbox() {
        // It shares its wire name with the checkbox group, so its empty value was read back as a
        // role this row holds and rendered as a ticked checkbox with no label. A depositor saw one
        // appear on every re-render that keeps the posted body: a refusal, a row action, or the
        // conflict notice.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let body = posted_contributor_row();
        let field = crate::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows {
            posted: Some(&body),
            action: "/x",
            adding: None,
            agents: Some(&scope),
            propose: true,
        };

        let out = attribution_rows(field, &draft, rows).into_string();
        assert!(
            !out.contains(r#"name="attributions.r0.role" value="""#),
            "no role control may carry an empty value: {out}"
        );
        // The role that was actually posted still renders, and still ticked.
        assert!(out.contains(r#"value="Author" checked"#), "{out}");
    }

    #[test]
    fn a_grants_row_keeps_one_blank_funder_however_often_it_is_re_rendered() {
        // The row renders one trailing blank so a second funder can be typed. Reading the posted
        // blank back as a held funder meant the render appended a *second* one, so the row grew by
        // a control every round trip: 2, 3, 4, 5.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "funding",
            serde_json::json!([{ "funders": ["organization-002"], "number": "1" }]),
        );
        let field = crate::form::registry::field("funding").expect("funding is a known field");

        let mut posted: Vec<(String, String)> = vec![
            ("funding.row".to_string(), "r0".to_string()),
            ("funding.kind".to_string(), "grants".to_string()),
            ("funding.r0.funder".to_string(), "organization-002".to_string()),
            ("funding.r0.funder".to_string(), String::new()),
        ];
        for round in 0..3 {
            let body = editor_core::form::FormBody::from_pairs(posted.clone());
            let rows = Rows {
                posted: Some(&body),
                action: "/x",
                adding: None,
                agents: Some(&scope),
                propose: true,
            };
            let out = funding(field, &draft, rows).into_string();
            let controls = out.matches(r#"name="funding.r0.funder""#).count();
            assert_eq!(controls, 2, "round {round} rendered {controls} funder controls: {out}");

            // Post back exactly what that render emitted, as the browser would.
            posted.retain(|(name, _)| name != "funding.r0.funder");
            posted.push(("funding.r0.funder".to_string(), "organization-002".to_string()));
            for _ in 1..controls {
                posted.push(("funding.r0.funder".to_string(), String::new()));
            }
        }
    }
}
