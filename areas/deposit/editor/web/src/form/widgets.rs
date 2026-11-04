//! Which control each field renders, keyed by the same ids
//! [`registry`](editor_core::form::registry) is.
//!
//! The dispatch is a `match` on the field id, not on the shape: two fields can
//! share a shape and want different controls (`provenance` and
//! `dataManagementPlan` are both an `Option<String>`, one a paragraph and one a
//! URL).
//!
//! A field renders in one of three states, and which one decides whether it
//! posts: a control posts even when empty, a value and a note post nothing. A
//! section posts only its own fields and an applier reads an absent name as
//! "not carried", so a display-only or not-yet-read field must submit no name,
//! or an empty control would clear a value the save was never meant to touch. A
//! note rather than silence for a field whose widget has not landed, so a
//! depositor does not conclude the form lost it.

use editor_core::agents::{Agent, AgentKind, AgentScope};
use editor_core::draft::{ProjectDraft, UrlSlot};
use editor_core::form::format::{Finding, ValueKind};
use editor_core::form::registry::{Field, Obligation};
use editor_core::form::{
    stored_row_key, ChoiceSet, FormBody, Shape, GRANTS_KIND, MAX_VALUES_PER_FIELD, REFERENCE_KIND,
};
use editor_core::multilingual::{as_multilingual, is_language_map, is_language_tag, DraftMultilingual, UI_LANGUAGES};
use maud::{html, Markup, Render};
use mosaic_tiles::badge::{badge, BadgeVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::checkbox_group::checkbox_group;
use mosaic_tiles::combobox::{combobox, combobox_listbox, combobox_option, combobox_results, combobox_status};
use mosaic_tiles::radio_group::radio_group;
use mosaic_tiles::repeatable_list::repeatable_list;
use mosaic_tiles::select::select;
use mosaic_tiles::text_field::{text_field, InputType, TextFieldBuilder};
use mosaic_tiles::textarea::textarea;
use mosaic_tiles::ComponentBuilder;
use serde_json::Value;
use shared_datastar as datastar;
use shared_metadata::is_placeholder;
use shared_metadata::project::CONTRIBUTOR_ROLES;

use crate::form::INTENT;
use crate::pages::section::{
    add_agent_intent, added_agent, change_agent_intent, changed_agent, choose_agent_intent, chosen_agent,
    propose_changes_intent, remove_role_intent, removed_role, FIND_AGENT, PROPOSE_ORGANIZATION, PROPOSE_PERSON, SUBMIT,
};

/// Whether the form is open for editing. A named type rather than a `bool`
/// argument, which says nothing about which way round `true` is at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Controls, and a save that writes.
    Editable,
    /// Values only — the project has a submission in review, so nothing may
    /// change under the reviewer.
    ReadOnly,
    /// Values only, because RDU accepted this field in the round being answered.
    /// A third variant because the reader has to be told which of two reasons
    /// applies: a whole-form lock lifts when the review finishes, this one when
    /// the field is submitted again.
    Accepted,
}

/// What a reader is told about a field RDU has already accepted. It renders as
/// a value with no control, so nothing posts; the note explains a gate enforced
/// server-side.
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

/// A field rendered as a value or as a note, with its own label above it. Not a
/// `<label>`: there is no control for one to point at, and a `for` naming
/// nothing is worse than none. `pub(crate)` because `crate::entity` reuses this
/// for a proposal that is no longer live.
pub(crate) fn stated(field: &Field, draft: &ProjectDraft, why: Option<&str>) -> Markup {
    let editable_but_unbuilt = !field.display_only && field.shape.is_none();
    let label_id = format!("{}-label", field.id);
    html! {
        div class="field" {
            p class="field-label" id=(label_id) { (labelled(field)) }
            (value_display(field, draft, &label_id))
            @if editable_but_unbuilt {
                p class="field-hint" { (NOT_READ_YET) }
            } @else if let Some(why) = why {
                // Instead of the hint: the hint says what to type, which this reader
                // cannot do.
                p class="field-hint" { (why) }
            } @else if let Some(hint) = field.hint {
                p class="field-hint" { (hint) }
            }
        }
    }
}

/// The stored value, rendered for reading. `label_id` is the id of the field's
/// label, for a rendering that is a group the label names.
fn value_display(field: &Field, draft: &ProjectDraft, label_id: &str) -> Markup {
    match field.id {
        "legalInfo" => legal_info_display(draft.get(field.id), label_id),
        _ => value_markup(draft.get(field.id)),
    }
}

const LICENCE_URI: &str = "Licence URI";

/// The licence, copyright holder and authorship of each legal-info entry, read-only.
///
/// Lives here and not in [`value_markup`]: the review surface shares that and
/// keeps "N entries" for every structured field.
///
/// Members are read from the JSON rather than through `LegalInfo`, which fails a
/// whole entry on one missing member and would make it vanish.
fn legal_info_display(value: Option<&Value>, label_id: &str) -> Markup {
    let entries: Vec<Vec<(&str, String)>> = match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter(|item| item.as_object().is_some_and(|members| !members.is_empty()))
            .map(legal_info_rows)
            .collect(),
        _ => Vec::new(),
    };
    html! {
        div role="group" aria-labelledby=(label_id) {
            @if entries.is_empty() {
                p class="italic text-neutral-600" { (NO_VALUE) }
            } @else if entries.len() == 1 { (legal_info_entry(&entries[0], 1)) } @else {
                ul class="flex flex-col gap-3" {
                    @for (index, rows) in entries.iter().enumerate() {
                        li class="border-t border-neutral-300 pt-3 first:border-t-0 first:pt-0" {
                            (legal_info_entry(rows, index + 1))
                        }
                    }
                }
            }
        }
    }
}

fn legal_info_rows(item: &Value) -> Vec<(&'static str, String)> {
    let license = item.get("license");
    let from_license = |key: &str| license.and_then(|license| member_text(license, key));
    let authorship = item.get("authorship").and_then(Value::as_array).map(|names| {
        names
            .iter()
            .filter_map(Value::as_str)
            .filter(|name| is_shown(name))
            .collect::<Vec<_>>()
            .join(", ")
    });
    [
        ("Licence", from_license("licenseIdentifier")),
        ("Licence date", from_license("licenseDate")),
        (LICENCE_URI, from_license("licenseURI")),
        ("Copyright holder", member_text(item, "copyrightHolder")),
        ("Authorship", authorship.filter(|names| !names.is_empty())),
    ]
    .into_iter()
    .filter_map(|(label, text)| Some((label, text?)))
    .collect()
}

fn member_text(parent: &Value, key: &str) -> Option<String> {
    parent
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| is_shown(text))
        .map(str::to_owned)
}

fn is_shown(text: &str) -> bool {
    !is_placeholder(text) && !text.trim().is_empty()
}

/// The address is a link only when it is a web address: a stored value is not
/// trusted to be one, and `javascript:` in an `href` would run.
fn legal_info_entry(rows: &[(&str, String)], number: usize) -> Markup {
    html! {
        @if rows.is_empty() {
            p class="italic text-neutral-600" { "Entry " (number) " could not be read." }
        } @else {
            dl class="grid grid-cols-1 sm:grid-cols-[10rem_minmax(0,1fr)] gap-x-3 gap-y-1" {
                @for (label, text) in rows {
                    dt class="text-sm font-bold text-neutral-600" { (label) }
                    dd class="break-words text-neutral-900" {
                        @if {
                            *label == LICENCE_URI
                                && (text.starts_with("https://")
                                    || text.starts_with("http://"))
                        } {
                            a class="break-all underline" href=(text) rel="noopener noreferrer" {
                                (text)
                            }
                        } @else { (text) }
                    }
                }
            }
        }
    }
}

/// Any project member, rendered for reading. Takes a `Value` because the review
/// surface renders a published and a submitted one side by side; one rendering
/// for both. A placeholder sentinel is not shown: `MISSING` and `CALCULATED` are
/// filtered out of DPE's UI and OAI-PMH output, so showing one here would present
/// an internal marker as a value.
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

/// The language a tag names, or the tag itself when it is not in the bundled list:
/// a wrong name is worse than a raw code.
pub(crate) fn language_name(tag: &str) -> &str {
    editor_core::languages::name_for_code(tag).unwrap_or(tag)
}

/// The control for one editable field, dispatched by id. `pub(crate)` because the
/// review surface renders the same control over the value it would commit; a
/// second dispatch would diverge silently. The fallback renders the unbuilt-field
/// note rather than a silent default, so a field given a shape without a control
/// here is visible.
pub(crate) fn control(field: &Field, draft: &ProjectDraft, shape: Shape, rows: Rows<'_>) -> Markup {
    match field.id {
        "name" | "officialName" => text(field, draft, InputType::Text),
        // type="text", not type="url": a draft may hold a value that does not
        // validate, and a browser refusing a half-typed address would block a save.
        "dataManagementPlan" => formatted_text(field, scalar_value(field, draft), rows),
        "startDate" | "endDate" | "accessRights.embargoDate" => text(field, draft, InputType::Date),
        // Two choices, both visible at once; a <select> for two options hides half
        // the answer behind a click.
        "status" => radio(field, draft, choices(shape)),
        // type="text", for the reason dataManagementPlan gives.
        "url" | "secondaryUrl" => url(field, draft, slot(shape), rows),
        "typeOfData" | "dataLanguage" => string_list(field, draft, set(shape), rows),
        "keywords" | "alternativeNames" => multilingual_rows(field, draft, rows),
        "additionalMaterial" | "documentationMaterial" => string_rows(field, draft, rows),
        "contactPoint" => agent_rows(field, draft, rows),
        "attributions" => attribution_rows(field, draft, rows),
        "temporalCoverage" | "disciplines" => text_or_reference_rows(field, draft, rows, reference_types(shape)),
        "spatialCoverage" => reference_rows(field, draft, rows, reference_types(shape)),
        "publications" => publication_rows(field, draft, rows),
        "funding" => funding(field, draft, rows),
        // Four choices with long labels and exactly one current: radios would be
        // four long lines competing with the fields around them.
        "accessRights.accessRights" => dropdown(field, draft, choices(shape)),
        "dataPublicationYear" => year(field, draft),
        "shortDescription" => long_text(field, draft, 2, Some(SHORT_DESCRIPTION_MAX)),
        "provenance" | "imageCredit" => long_text(field, draft, 3, None),
        "description" => multilingual(field, draft, 5),
        "abstract" => multilingual(field, draft, 2),
        // Unreachable while the registry and this dispatch agree, which
        // tests::every_shaped_field_has_a_control pins. Rendered rather than
        // panicked, so a missing control does not take the whole section down.
        _ => {
            debug_assert!(false, "{} declares {shape:?} but no control", field.id);
            stated(field, draft, None)
        }
    }
}

/// What a control needs beyond the draft: the posted body, where a repeatable
/// field's add and remove controls submit to, and the format findings. A named type
/// because several are `Option`-ish and a positional list would let call sites
/// scramble them silently.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rows<'a> {
    /// The posted body, when this render is answering a `POST`. What keeps a
    /// freshly added row alive: a row with no text is never stored, so it exists
    /// only in the form, and the hidden `{field}.row` carries its key back.
    pub posted: Option<&'a FormBody>,
    /// The body this render answers, kept or not: unlike [`Self::posted`] it is
    /// also set after an explicit save, which drops the posted rows. Read only to
    /// tell whether an "Add another" box had text in it; see [`add_box_id`].
    pub submitted: Option<&'a FormBody>,
    /// Base URL for the add and remove controls: `{base}/add` and
    /// `{base}/{key}/remove`.
    pub action: &'a str,
    /// The field one more blank row was just asked for.
    pub adding: Option<&'a str>,
    /// The agents an id field may refer to. `None` renders the id bare, which is
    /// what a deployment with no data directory looks like.
    pub agents: Option<&'a AgentScope<'a>>,
    /// Format findings, each keyed by the name of the control it renders on.
    pub format_findings: &'a [Finding],
    /// Whether an agent id also renders the controls that start an entity
    /// proposal. Defaults to `false`: the entity form's `affiliations` field reuses
    /// this widget, and its route does not dispatch the propose intents, so a
    /// button there would fall through to a save. Only the section form turns
    /// this on.
    pub propose: bool,
    /// The section's agent search (`GET {path}?for=<picker>&q=`) a picker's combobox runs as the
    /// depositor types. `None` leaves the combobox a plain search box that the Search button
    /// submits: the route behind it is not wired for the form.
    pub search_path: Option<&'a str>,
    /// The control that takes focus, by name: the first holding a finding on a refused submit's
    /// re-render.
    pub focus: Option<&'a str>,
}

/// The keys of the rows to render for `field`, in display order: from the posted
/// body when there is one, so a typed value and an added blank row survive the
/// round trip; positionally from the stored list otherwise. The applier rebuilds
/// the list from the keys in DOM order, never from a number.
fn row_keys(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Vec<String> {
    let row_name = format!("{}.row", field.id);
    let mut keys: Vec<String> = match rows.posted {
        Some(body) if body.has(&row_name) => body.rows(field.id).into_iter().map(str::to_string).collect(),
        _ => (0..stored_rows(field, draft).len()).map(stored_row_key).collect(),
    };
    if rows.adding == Some(field.id) {
        keys.push(next_row_key(&keys));
    }
    keys
}

/// A text control posting under `name`, for a value of `kind` where it has one
/// (`editor_core::form::format`), with a hint of `lead` then the format. A finding
/// on `name` renders through the tile's own error, so `aria-invalid` and
/// `aria-describedby` come with it, and never in an alert.
pub(crate) fn format_field(
    name: String,
    label: impl maud::Render,
    kind: Option<ValueKind>,
    lead: Option<&str>,
    rows: Rows<'_>,
) -> TextFieldBuilder {
    let finding = rows.format_findings.iter().find(|finding| finding.control == name);
    let hint = [lead, kind.map(format_hint)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let control = text_field(name, label).input_type(InputType::Text);
    let control = if hint.is_empty() { control } else { control.hint(hint) };
    match finding {
        Some(finding) if rows.focus == Some(finding.control.as_str()) => {
            control.error(format_message(finding.kind)).autofocus()
        }
        Some(finding) => control.error(format_message(finding.kind)),
        None => control,
    }
}

/// The kind of `field`'s value its control posts under `posted` (see
/// [`editor_core::form::format::Member::posted`]), read from the registry so the
/// hint, the error and the rule cannot disagree.
pub(crate) fn format_kind(field: &Field, posted: &str) -> Option<ValueKind> {
    field
        .format
        .iter()
        .find(|member| member.posted == posted)
        .map(|member| member.kind)
}

/// The format a value of `kind` needs, as a control's hint.
pub const fn format_hint(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Link => "A full web address, starting with https://",
        ValueKind::DoiOrLink => "A DOI such as 10.1234/abcd, or a full web address starting with https://",
        ValueKind::LinkOrNotAccessible => {
            "A full web address, starting with https://, or “not accessible” if the plan cannot be shared"
        }
    }
}

/// What a value without its kind's format is told, standing alone: a screen reader
/// reads it without the field's name.
pub const fn format_message(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Link => "Enter a full web address, such as https://www.dasch.swiss.",
        ValueKind::DoiOrLink => {
            "Enter a DOI, such as 10.1234/abcd, or a full web address, such as https://doi.org/10.1234/abcd."
        }
        ValueKind::LinkOrNotAccessible => {
            "Enter a full web address, such as https://www.dasch.swiss, or “not accessible”."
        }
    }
}

/// Focuses the `autofocus` element inside where the enhanced path morphs it in (the row an Add
/// just created, a confirmation's safe choice), since a browser honours `autofocus` only at page
/// load.
pub(crate) const FOCUS_AUTOFOCUS: &str = "el.querySelector('[autofocus]')?.focus()";

/// [`FOCUS_AUTOFOCUS`] for a node that replaces one the depositor may still be in:
/// replacing a focused node drops focus to the body, so focus is taken only when
/// it was lost that way. Focus the depositor moved on to stays where it is.
const FOCUS_IF_LOST: &str =
    "(!document.activeElement || document.activeElement === document.body) && el.querySelector('[autofocus]')?.focus()";

fn is_added_row(field: &Field, rows: Rows<'_>, position: usize, count: usize) -> bool {
    rows.adding == Some(field.id) && position + 1 == count
}

/// A row key not already in use: the smallest free `rN`, so the set stays dense
/// and the accessible "Remove keyword 2" labels match what a reader counts.
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

/// One row's lines, `(tag, text)` in order, from the body when this is a re-render
/// and from the stored list otherwise. The two never mix: reading texts from one
/// and tags from the other shows a value the depositor did not type. A `Vec`, not
/// a language map, so a tag repeated within a row stays two lines until the
/// applier splits it.
fn row_lines(field: &Field, draft: &ProjectDraft, rows: Rows<'_>, key: &str, position: usize) -> Vec<(String, String)> {
    let row_name = format!("{}.row", field.id);
    let mut lines: Vec<(String, String)> = match rows.posted {
        Some(body) if body.has(&row_name) => body
            .all(&format!("{}.{key}.lang", field.id))
            .zip(body.all(&format!("{}.{key}.text", field.id)))
            .map(|(tag, text)| (tag.trim().to_ascii_lowercase(), text.to_string()))
            .filter(|(tag, _)| is_language_tag(tag))
            .take(MAX_VALUES_PER_FIELD)
            .collect(),
        _ => stored_rows(field, draft)
            .get(position)
            .map(|row| {
                as_multilingual(row)
                    .iter()
                    .map(|(tag, text)| (tag.to_string(), text.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    };
    if lines.is_empty() {
        lines.push((BLANK_ROW_LANGUAGE.to_string(), String::new()));
    }
    lines
}

const BLANK_ROW_LANGUAGE: &str = "en";

/// A list of single strings: one text control per row, sharing the row protocol
/// with [`multilingual_rows`] and differing only in what a row contains.
pub(crate) fn string_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let name = capitalised(&noun);
    let mut list = repeatable_list(field.id, labelled(field), &action).item_noun(&noun);
    let kind = format_kind(field, "");
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let value = match rows.posted {
            Some(body) if body.has(&row_name) => body.get(&format!("{}.{key}", field.id)).unwrap_or_default(),
            _ => stored.get(position).and_then(|row| row.as_str()).unwrap_or_default(),
        };
        let label = row_label(&name, position);
        let control = format_field(format!("{}.{key}", field.id), label, kind, None, rows).label_hidden();
        list = list.row(
            key,
            html! {
                (control.value(value))
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

/// The suffix a picker's search box posts under. Read by nothing in
/// `editor_core::form`, so the query rides along in the body and is never stored.
const QUERY_SUFFIX: &str = "q";

/// The Search button's accessible name, which the visible "Search" alone would leave
/// identical across every row. It starts with that text (WCAG 2.5.3).
const FIND_AGENT_LABEL: &str = "Search for a person or organisation";
const FIND_AGENT_BUTTON: &str = "Search";

const ADD_AGENT_LABEL: &str = "Add a person or organisation";
const ADD_AGENT_HINT: &str = "Search by name or affiliation.";
const ADD_AGENT_FIND_LABEL: &str = "Search to add a person or organisation";

const CHANGE_AGENT_BUTTON: &str = "Change";

struct Slot<'a> {
    /// The name the current id posts under, from a hidden input. Shared by every funder in a
    /// grant, which is how `apply_funding` collects them.
    value_name: &'a str,
    /// Distinguishes slots for element ids, the search box and the choice; a funder's is the
    /// shared name plus its position ([`funder_control`]).
    unique: &'a str,
    label: &'a str,
    id: &'a str,
    /// Goes into every button's accessible name: two rows can hold the same agent.
    row_context: &'a str,
    /// A row the `/add` URL just created: its search takes focus.
    focus: bool,
    /// Draws the card in the Warning colour; see [`attribution_rows`].
    incomplete: bool,
}

/// One agent slot: a card when it holds an id and is not being changed, a search (a combobox over
/// the agents) when it is empty or being changed. Why a search and not a list of every agent, and
/// how the two states post: `docs/src/editor/project-form.md`.
///
/// Invariants: `value_name` posts always, from a hidden input holding the current id, in both
/// states, so an untouched save round-trips byte-for-byte and a change abandoned with Escape keeps
/// the stored choice. `inside` renders in both states, because it holds controls that post (a
/// contributor's roles). Focus follows the depositor: an opened search takes it, and so does the
/// Change button of a card whose choice was just made, by `autofocus` on a full page and by
/// `data-init` where the enhanced path morphs the slot in, since a browser honours `autofocus`
/// only at page load.
fn agent_picker(slot: Slot<'_>, rows: Rows<'_>, inside: Markup) -> Markup {
    let query_name = format!("{}.{QUERY_SUFFIX}", slot.unique);
    let intent = rows.posted.and_then(|body| body.get(INTENT));
    let opened = intent.and_then(changed_agent) == Some(slot.unique);
    let chosen_here = intent.and_then(chosen_agent).is_some_and(|(picker, _)| picker == slot.unique);
    let searching = slot.id.trim().is_empty() || opened || rows.posted.is_some_and(|body| body.has(&query_name));
    match searching {
        true => slot_search(&slot, &query_name, rows, inside, slot.focus || opened),
        false => slot_card(&slot, rows, inside, chosen_here),
    }
}

fn slot_search(slot: &Slot<'_>, query_name: &str, rows: Rows<'_>, inside: Markup, autofocus: bool) -> Markup {
    let query = rows.posted.and_then(|body| body.get(query_name)).unwrap_or_default();
    let current = slot.id.trim();
    let (results, expanded) = match rows.agents {
        Some(scope) => results_for(slot.unique, query, scope, &[], rows.propose),
        None => (html! {}, false),
    };
    let mut search = combobox(query_name, slot.label)
        .with_id(agent_query_id(slot.unique))
        .results_id(agent_results_id(slot.unique))
        .value(query)
        .search_button(find_button(FIND_AGENT_LABEL, slot.row_context))
        .results(results)
        .expanded(expanded);
    if !current.is_empty() {
        let name = rows
            .agents
            .and_then(|agents| agents.get(current))
            .map_or(current, |agent| agent.label.as_str());
        search = search.hint(format!("Choosing a match replaces {name}."));
    }
    if let Some(path) = rows.search_path {
        search = search.search_action(agent_search_action(path, slot.unique));
    }
    if autofocus {
        search = search.autofocus();
    }
    html! {
        div class="flex flex-col gap-2" data-init=[autofocus.then_some(FOCUS_AUTOFOCUS)] {
            input type="hidden" name=(slot.value_name) value=(slot.id.trim());
            (search)
            (inside)
        }
    }
}

/// The find-agent Search button: the no-JavaScript way to run the search in the box before it.
fn find_button(aria_label: &str, context: &str) -> Markup {
    button(FIND_AGENT_BUTTON)
        .variant(ButtonVariant::Secondary)
        .button_type(ButtonType::Submit)
        .name_value(INTENT, FIND_AGENT)
        .aria_label(format!("{aria_label} ({context})"))
        .build()
}

fn slot_card(slot: &Slot<'_>, rows: Rows<'_>, inside: Markup, focus: bool) -> Markup {
    let id = slot.id.trim();
    let resolved = rows.agents.and_then(|agents| agents.get(id));
    let name = resolved.map_or(id, |agent| agent.label.as_str());
    let border = match slot.incomplete {
        true => "border-warning-600",
        false => "border-neutral-500",
    };
    let class = format!("flex flex-col gap-3 rounded-md border p-3 {border}");
    let change = button(CHANGE_AGENT_BUTTON)
        .variant(ButtonVariant::Secondary)
        .class("px-2 py-1")
        .name_value(INTENT, change_agent_intent(slot.unique))
        .aria_label(format!("{CHANGE_AGENT_BUTTON} {name} ({})", slot.row_context));
    let change = match focus {
        true => change.autofocus(),
        false => change,
    };
    html! {
        div class="flex flex-col gap-2" data-init=[focus.then_some(FOCUS_AUTOFOCUS)] {
            input type="hidden" name=(slot.value_name) value=(id);
            div role="group" aria-label=(slot.label) class=(class) {
                div class="flex flex-wrap items-start justify-between gap-3" {
                    div class="flex min-w-0 flex-col gap-1" {
                        @match resolved {
                            Some(agent) => {
                                p {
                                    span class="font-semibold" { (agent.label) }
                                    " "
                                    (badge(agent.kind.label()).variant(BadgeVariant::Secondary))
                                }
                                @if let Some(detail) = &agent.detail {
                                    p class="text-sm text-neutral-600" { (detail) }
                                }
                            }
                            None => {
                                p {
                                    strong { (id) }
                                    " — "
                                    (UNRESOLVED_AGENT)
                                }
                            }
                        }
                    }
                    (change)
                }
                (inside)
            }
        }
    }
}

fn match_count_line(total: usize, shown: usize, truncated: bool) -> String {
    match truncated {
        true => format!("{total} matches, showing the first {shown}. Add a word to narrow it."),
        false => format!("{total} match{}.", if total == 1 { "" } else { "es" }),
    }
}

/// The id of a picker's search input. Stable for a slot's search; the append search renames its
/// own on a reset ([`append_search`]), which is why the results container is addressed by
/// [`agent_results_id`] instead.
pub(crate) fn agent_query_id(unique: &str) -> String {
    format!("{unique}-{QUERY_SUFFIX}")
}

/// The id of a picker's results container: what the input's `aria-controls` names and what the
/// search endpoint's fragment replaces, whatever the input's own id is.
pub(crate) fn agent_results_id(unique: &str) -> String {
    format!("{unique}-results")
}

/// The longest query [`agent_results`] searches for. The search endpoint answers a longer one with
/// nothing rather than an error: a 4xx on a type-ahead fires the page's telemetry error listener.
const MAX_QUERY_CHARS: usize = 200;

/// The results of one type-ahead search, as the page and the search endpoint both render them.
///
/// `unique` names the picker; one [`append_unique`] makes is the append search under `{field}`,
/// whose options add a row ([`add_agent_intent`]), and any other replaces the agent on its card
/// ([`choose_agent_intent`]). `exclude` ids are not offered. `propose` adds the two propose
/// buttons to an empty result, where the section form dispatches them.
pub(crate) fn agent_results(
    unique: &str,
    query: &str,
    scope: &AgentScope<'_>,
    exclude: &[&str],
    propose: bool,
) -> Markup {
    let (content, expanded) = results_for(unique, query, scope, exclude, propose);
    combobox_results(&agent_results_id(unique), content, expanded)
}

/// What [`agent_results`] puts in the container, and whether it is open. The combobox builder
/// wraps its own results in the container, so a page render passes it this and not the container.
fn results_for(unique: &str, query: &str, scope: &AgentScope<'_>, exclude: &[&str], propose: bool) -> (Markup, bool) {
    if query.trim().is_empty() || query.chars().count() > MAX_QUERY_CHARS {
        return (html! {}, false);
    }
    let found = scope.search(query, exclude);
    let shown = found.offered();
    if shown.is_empty() {
        let content = html! {
            ({
                combobox_status(
                    html! {
                        "Nothing matches “"(query) "”."
                    },
                )
            })
            @if propose {
                p class="text-sm" { "Not in the list? Propose a new person or organisation" }
                div class="flex flex-wrap gap-2" {
                    ({
                        button("Propose a new person")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, PROPOSE_PERSON)
                    })
                    ({
                        button("Propose a new organisation")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, PROPOSE_ORGANIZATION)
                    })
                }
            }
        };
        return (content, false);
    }
    let append_to = appended_field(unique);
    let options = shown.iter().map(|agent| {
        let value = match append_to {
            Some(field) => add_agent_intent(field, &agent.id),
            None => choose_agent_intent(unique, &agent.id),
        };
        let content = html! {
            span class="font-semibold" { (agent.label) }
            " "
            (badge(agent.kind.label()).variant(BadgeVariant::Secondary))
            @if let Some(detail) = &agent.detail {
                span class="block text-sm" { (detail) }
            }
        };
        combobox_option(content, INTENT, value).build()
    });
    let content = html! {
        ({
            combobox_status(
                match_count_line(found.total(), shown.len(), found.truncated()),
            )
        })
        (combobox_listbox(format!("Matches for “{query}”"), options))
    };
    (content, true)
}

/// The `unique` of the append search under the list of agents `field`: how an option knows to
/// append a row ([`add_agent_intent`]) and not to replace an agent.
pub(crate) fn append_unique(field: &str) -> String {
    format!("{field}{APPEND_SUFFIX}")
}

/// The field an append search's `unique` ([`append_unique`]) appends to, or `None` for any other
/// picker. Appending is valid only for `AttributionRows` and `AgentRows`; the caller checks the
/// field's shape.
pub(crate) fn appended_field(unique: &str) -> Option<&str> {
    unique.strip_suffix(APPEND_SUFFIX)
}

const APPEND_SUFFIX: &str = ".new";

/// The Datastar expression a combobox's input runs as the depositor types: a `GET` of `path`
/// (the section's agent search) for the picker `unique`, carrying the input's value as `q`.
pub(crate) fn agent_search_action(path: &str, unique: &str) -> String {
    datastar::get(format!("{path}?for={unique}&q="))
        .plus_js("encodeURIComponent(evt.target.value)")
        .to_string()
}

/// The one search under a list of agent rows, whose options append a row ([`add_agent_intent`]).
///
/// `held` are the ids the rows already hold, which it does not offer again. The box must come
/// back empty after an add, and the Datastar morph keeps typed text unless the node is replaced,
/// so its id changes only on the response to an add (see [`reset_token`]): a response to anything
/// else, an autosave landing mid-typing included, would replace the node and drop keystrokes. Focus
/// stays on it through an add (see [`FOCUS_IF_LOST`]).
fn append_search(field: &Field, held: &[&str], rows: Rows<'_>) -> Markup {
    let unique = append_unique(field.id);
    let query_name = format!("{unique}.{QUERY_SUFFIX}");
    let added_here = rows
        .submitted
        .or(rows.posted)
        .and_then(|body| body.get(INTENT))
        .and_then(added_agent)
        .is_some_and(|(added_to, _)| added_to == field.id);
    let id = add_box_id(&agent_query_id(&unique), held.len(), reset_token(added_here).as_deref());
    let focus = added_here && rows.adding.is_none();
    let query = rows.posted.and_then(|body| body.get(&query_name)).unwrap_or_default();
    let (results, expanded) = match rows.agents {
        Some(scope) => results_for(&unique, query, scope, held, rows.propose),
        None => (html! {}, false),
    };
    let mut search = combobox(&query_name, ADD_AGENT_LABEL)
        .with_id(&id)
        .results_id(agent_results_id(&unique))
        .hint(ADD_AGENT_HINT)
        .value(query)
        .search_button(find_button(ADD_AGENT_FIND_LABEL, field.label))
        .results(results)
        .expanded(expanded);
    if let Some(path) = rows.search_path {
        search = search.search_action(agent_search_action(path, &unique));
    }
    if focus {
        search = search.autofocus();
    }
    focus_wrapper(&id, search, focus)
}

/// Rewrite the raw pairs a posted picker intent describes, before [`FormBody::from_pairs`]
/// reads them. The intent stays in the body: [`agent_picker`] and the handler read it.
///
/// - choose: the chosen id lands in its picker's value and that picker's search closes.
/// - add-agent: one new row holding the agent is appended to the contributor or agent-id rows
///   `shape_of` names for the field; any other shape, or an unknown field, leaves the body alone.
/// - remove-role: every `{prefix}.role` pair equal to the intent's role is dropped.
pub(crate) fn apply_picker_intents(
    mut pairs: Vec<(String, String)>,
    shape_of: impl Fn(&str) -> Option<Shape>,
) -> Vec<(String, String)> {
    let Some(intent) = pairs.iter().find(|(name, _)| name == INTENT).map(|(_, intent)| intent.clone()) else {
        return pairs;
    };
    if let Some((unique, id)) = chosen_agent(&intent) {
        choose_agent(&mut pairs, unique, id);
    } else if let Some((field, id)) = added_agent(&intent) {
        append_agent(&mut pairs, field, id, shape_of(field));
    } else if let Some((prefix, role)) = removed_role(&intent) {
        let role_name = format!("{prefix}.role");
        pairs.retain(|(name, value)| !(*name == role_name && value == role));
    }
    pairs
}

fn choose_agent(pairs: &mut Vec<(String, String)>, unique: &str, id: &str) {
    let (value_name, slot) = value_slot(unique);
    if let Some((_, value)) = pairs.iter_mut().filter(|(name, _)| name == value_name).nth(slot) {
        *value = id.to_string();
    }
    let query_name = format!("{unique}.{QUERY_SUFFIX}");
    pairs.retain(|(name, _)| name != &query_name);
}

/// Append a row holding `id` to `field`, which `shape` must say holds contributors or agent ids.
///
/// The row keeps the keys the body already carries and takes the smallest free one, so the row
/// the picker's own Add would have made and this one cannot collide. An id the field already
/// holds appends nothing: the same agent twice is never what a depositor meant. Nor does a field
/// already at [`MAX_VALUES_PER_FIELD`] rows.
fn append_agent(pairs: &mut Vec<(String, String)>, field: &str, id: &str, shape: Option<Shape>) {
    if !matches!(shape, Some(Shape::AttributionRows | Shape::AgentRows)) {
        return;
    }
    let value_name = |key: &str| match shape {
        Some(Shape::AttributionRows) => format!("{field}.{key}.contributor"),
        _ => format!("{field}.{key}"),
    };
    let query_name = format!("{}.{QUERY_SUFFIX}", append_unique(field));
    pairs.retain(|(name, _)| name != &query_name);
    // The empty-list sentinel: with a real row beside it the applier would read it as a row.
    let row_name = format!("{field}.row");
    pairs.retain(|(name, value)| !(*name == row_name && value.is_empty()));

    let body = FormBody::from_pairs(pairs.clone());
    let keys: Vec<String> = body.rows(field).into_iter().map(str::to_string).collect();
    if keys.len() >= MAX_VALUES_PER_FIELD {
        return;
    }
    let held = keys
        .iter()
        .any(|key| body.all(&value_name(key)).any(|value| value.trim() == id));
    if held {
        return;
    }
    let key = next_row_key(&keys);
    pairs.push((row_name, key.clone()));
    pairs.push((value_name(&key), id.to_string()));
}

/// The name a picker's value posts under, and which of its values is the picker's:
/// a funder's `unique` is the shared name plus its position ([`funder_control`]).
fn value_slot(unique: &str) -> (&str, usize) {
    unique
        .rsplit_once('-')
        .filter(|(name, _)| name.ends_with(".funder"))
        .and_then(|(name, position)| Some((name, position.parse().ok()?)))
        .unwrap_or((unique, 0))
}

/// A list of agent ids as editable rows, each a card (see [`agent_picker`]), with one search under
/// the list that appends a row ([`append_search`]) instead of an Add button.
pub(crate) fn agent_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action)
        .item_noun(&noun)
        .without_add()
        .empty_message(format!("No {} yet. Search below to add one.", field.label.to_lowercase()));
    let mut held: Vec<String> = Vec::new();
    for (position, key) in keys.iter().enumerate() {
        let row_name = format!("{}.row", field.id);
        let id = match rows.posted {
            Some(body) if body.has(&row_name) => body.get(&format!("{}.{key}", field.id)).unwrap_or_default(),
            _ => stored.get(position).and_then(|row| row.as_str()).unwrap_or_default(),
        };
        if !id.trim().is_empty() {
            held.push(id.trim().to_string());
        }
        let name = format!("{}.{key}", field.id);
        let resolved = rows.agents.and_then(|agents| agents.get(id.trim()));
        let row_context = format!("row {}", position + 1);
        let slot = Slot {
            value_name: &name,
            unique: &name,
            label: &row_label(AGENT_ID_LABEL, position),
            id,
            row_context: &row_context,
            focus: is_added_row(field, rows, position, keys.len()),
            incomplete: false,
        };
        let body = html! {
            div class="flex flex-col gap-2" {
                (agent_picker(slot, rows, html! {}))
                @if rows.propose { (propose_controls(resolved, id, &row_context)) }
            }
        };
        list = list.row(key, body);
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    let held: Vec<&str> = held.iter().map(String::as_str).collect();
    html! {
        (list)
        (append_search(field, &held, rows))
    }
}

/// What the propose-changes button says, naming the entity and never the row it
/// sits in: a bare "Propose changes" on a contributor row reads as an offer to
/// propose the roles, which are project data.
const fn propose_changes_label(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Person => "Propose changes to this person's details",
        AgentKind::Organization => "Propose changes to this organisation's details",
    }
}

/// The controls that start an entity proposal, beside an agent picker: named
/// submits on the section's own form, which dispatches all three intents. `id`
/// empty renders nothing, since an unfilled picker is not "no match". `row_context`
/// goes into each button's accessible name, as [`row_label`] numbers a row's
/// control.
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

/// A list of contributor rows, each a card (see [`agent_picker`]) that holds that contributor's
/// roles, with one search under the list that appends a row ([`append_search`]) instead of an Add
/// button. Roles are the checkbox group over the offered vocabulary unioned with what the row
/// holds, plus a text input for one more ([`roles_section`]); open for the reason `dataLanguage`
/// is: the corpus spells the same role several ways. Group and input post under
/// `{field}.{key}.role`, so `FormBody::all` collects them together.
fn attribution_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let keys = row_keys(field, draft, rows);
    let stored = stored_rows(field, draft);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let mut list = repeatable_list(field.id, labelled(field), &action)
        .item_noun(&noun)
        .without_add()
        .empty_message(NO_CONTRIBUTORS);
    let mut agent_ids: Vec<String> = Vec::new();
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
        // Blanks are dropped: the "add another role" input posts under this same
        // name and an untouched one sends an empty string, which would come back
        // as a ticked, unlabelled checkbox.
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
        if !contributor.trim().is_empty() {
            agent_ids.push(contributor.trim().to_string());
        }
        let contributor_name = format!("{prefix}.contributor");
        let resolved = rows.agents.and_then(|agents| agents.get(contributor.trim()));
        let row_context = format!("row {}", position + 1);
        let who = resolved.map_or_else(
            || match contributor.trim().is_empty() {
                true => "this contributor".to_string(),
                false => contributor.trim().to_string(),
            },
            |agent| agent.label.clone(),
        );
        let incomplete = held.is_empty() && !contributor.trim().is_empty() && may_flag_incomplete(rows);
        let roles = roles_section(&prefix, &who, &held, incomplete, rows);
        let slot = Slot {
            value_name: &contributor_name,
            unique: &contributor_name,
            label: &row_label(AGENT_ID_LABEL, position),
            id: &contributor,
            row_context: &row_context,
            focus: is_added_row(field, rows, position, keys.len()),
            incomplete,
        };
        // The propose controls go below the role controls: sandwiched between the
        // picker and the roles they read as an offer to propose the roles.
        let body = html! {
            div class="flex flex-col gap-3" {
                (agent_picker(slot, rows, roles))
                @if rows.propose { (propose_controls(resolved, &contributor, &row_context)) }
            }
        };
        list = list.row(key, body);
    }
    if let Some(hint) = field.hint {
        list = list.hint(hint);
    }
    let held: Vec<&str> = agent_ids.iter().map(String::as_str).collect();
    html! {
        (list)
        (append_search(field, &held, rows))
    }
}

const NO_CONTRIBUTORS: &str = "No contributors yet. Search below to add the people and organisations involved.";

/// A held role's chip: display only, so it has the look of a tag and not of a control.
const ROLE_CHIP: &str =
    "inline-flex items-center gap-1 rounded-full border border-neutral-500 bg-neutral-50 py-0.5 pl-3 pr-1 text-sm";

const ROLE_REQUIRED: &str = "Choose at least one role";

/// Whether this render may tell a contributor with no role to choose one. Only a render of the
/// stored draft (a `GET`, an explicit save) or the answer to a refused submit: the response to the
/// choice or add that just made the contributor would scold a depositor who has not had a chance
/// to pick one.
fn may_flag_incomplete(rows: Rows<'_>) -> bool {
    rows.posted.is_none() || rows.submitted.and_then(|body| body.get(INTENT)) == Some(SUBMIT)
}

/// One contributor's roles: the held ones as display-only chips, each with a Remove, then the
/// controls that actually post them.
///
/// The checkbox group is the only source of truth for roles; a chip's Remove posts
/// [`remove_role_intent`], which [`apply_picker_intents`] applies to the body. The `<details>` is
/// open while no role is held, and keeps whatever state the depositor gave it across a morph
/// (`data-preserve-attr`) while one is: without that, ticking a role would autosave and collapse
/// the group under their hands, and with it on an empty row the server's `open` would never win
/// back after the last role is removed.
fn roles_section(prefix: &str, who: &str, held: &[String], incomplete: bool, rows: Rows<'_>) -> Markup {
    let mut options: Vec<&str> = CONTRIBUTOR_ROLES.to_vec();
    for role in held {
        if !options.contains(&role.as_str()) {
            options.push(role);
        }
    }
    let name = format!("{prefix}.role");
    let summary = match held.is_empty() {
        true => "Choose roles",
        false => "Change roles",
    };
    let required_id = format!("{prefix}-roles-required");
    let focus = removed_role_focus(prefix, held, rows);
    let summary_focus = focus == Some(RoleFocus::Summary);
    // A chip's focus rides on a wrapper whose id changes with every such response: a morph reruns
    // `data-init` only on a node it replaces, and a second Remove would otherwise find the same
    // attribute already in place.
    let chips_id = matches!(focus, Some(RoleFocus::Chip(_)))
        .then(|| reset_token(true).map(|token| format!("{prefix}-roles-{token}")))
        .flatten();
    html! {
        div class="flex flex-col gap-2" {
            @if !held.is_empty() {
                div id=[chips_id.as_deref()]
                    data-init=[chips_id.as_deref().map(|_| FOCUS_AUTOFOCUS)]
                {
                    ul aria-label=(format!("Roles of {who}")) class="flex flex-wrap gap-2" {
                        @for (index, role) in held.iter().enumerate() {
                            li class=(ROLE_CHIP) {
                                span { (role) }
                                ({
                                    let remove = button("×")
                                        .variant(ButtonVariant::Ghost)
                                        .class("min-h-6 min-w-6")
                                        .name_value(INTENT, remove_role_intent(prefix, role))
                                        .aria_label(format!("Remove role {role} from {who}"));
                                    match focus == Some(RoleFocus::Chip(index)) {
                                        true => remove.autofocus(),
                                        false => remove,
                                    }
                                })
                            }
                        }
                    }
                }
            }
            @if incomplete {
                p id=(required_id) class="text-sm text-warning-800" { (ROLE_REQUIRED) }
            }
            details
                open[held.is_empty()]
                data-preserve-attr=[(!held.is_empty()).then_some("open")]
                data-init=[summary_focus.then_some(FOCUS_AUTOFOCUS)]
            {
                summary
                    class="cursor-pointer text-sm"
                    aria-describedby=[incomplete.then_some(required_id.as_str())]
                    autofocus[summary_focus]
                { (summary) }
                div class="mt-2 flex flex-col gap-3" {
                    ({
                        checkbox_group(&name, ROLE_GROUP_LABEL)
                            .options(options.iter().map(|role| (*role, *role)))
                            .checked(held.iter().map(String::as_str))
                    })
                    ({
                        let typed = add_text_posted(rows, &name);
                        let id = add_box_id(
                            &name,
                            held.len(),
                            reset_token(typed).as_deref(),
                        );
                        let input = text_field(&name, ADD_ROLE_LABEL)
                            .input_type(InputType::Text)
                            .hint(ADD_ROLE_HINT)
                            .with_id(&id);
                        add_box(&id, input, typed && rows.adding.is_none())
                    })
                }
            }
        }
    }
}

/// What takes focus on the response to a role chip's Remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleFocus {
    /// The Remove button of the chip at this position.
    Chip(usize),
    /// The `<details>` summary: no chip is left.
    Summary,
}

/// Where focus goes when this response answers a Remove of one of `prefix`'s roles: the chip now
/// at the removed one's position, else the one before it, else the roles summary.
///
/// The body no longer holds the removed role, and a response lists the roles in checkbox order
/// (the vocabulary's, then any other in the order held), so the position is where the removed role
/// sorts among those still held; a custom role sorts after the vocabulary ones.
fn removed_role_focus(prefix: &str, held: &[String], rows: Rows<'_>) -> Option<RoleFocus> {
    let intent = rows.submitted.or(rows.posted)?.get(INTENT)?;
    let (from, removed) = removed_role(intent)?;
    if from != prefix {
        return None;
    }
    let rank = |role: &str| {
        CONTRIBUTOR_ROLES
            .iter()
            .position(|known| *known == role)
            .unwrap_or(CONTRIBUTOR_ROLES.len())
    };
    let removed_rank = rank(removed);
    let at = held.iter().filter(|role| rank(role) < removed_rank).count();
    Some(match (at < held.len(), at) {
        (true, _) => RoleFocus::Chip(at),
        (false, 0) => RoleFocus::Summary,
        (false, _) => RoleFocus::Chip(at - 1),
    })
}

/// The reference types a shape offers, or none for any other shape.
const fn reference_types(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::TextOrReferenceRows(types) | Shape::ReferenceRows(types) => types,
        _ => &[],
    }
}

/// A list of rows that are each an authority reference or free text. Both
/// branches are always in the DOM, the inactive one `hidden` rather than
/// `disabled`: a hidden input is still submitted, so the server receives both
/// candidates plus the discriminant and a depositor who switches and back finds
/// their text still there. Switching branches is a server round-trip, so the
/// control behaves identically with and without JavaScript.
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

        // A stored row is a reference when it has a URL, the same test
        // ProjectDraft::coverage_shapes applies.
        let is_reference = match posted {
            Some(body) => body.get(&format!("{prefix}.kind")) == Some(REFERENCE_KIND),
            None => held.and_then(|row| row.get("url")).is_some(),
        };
        let reference = reference_controls(field, &prefix, types, rows, |member| {
            reference_member(posted, held.copied(), &prefix, member)
        });
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
                    div hidden[!is_reference] class="flex flex-col gap-2" { (reference) }
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
fn reference_controls(
    field: &Field,
    prefix: &str,
    types: &[&str],
    rows: Rows<'_>,
    value: impl Fn(&str) -> String,
) -> Markup {
    html! {
        ({
            select(format!("{prefix}.ref.type"), REFERENCE_TYPE_LABEL)
                .options(types.iter().map(|kind| (*kind, *kind)))
                .selected(value("type"))
        })
        ({
            format_field(
                    format!("{prefix}.ref.url"),
                    REFERENCE_URL_LABEL,
                    format_kind(field, "ref.url"),
                    None,
                    rows,
                )
                .value(value("url"))
        })
        ({
            text_field(format!("{prefix}.ref.label"), REFERENCE_LABEL_LABEL)
                .input_type(InputType::Text)
                .value(value("label"))
        })
    }
}

/// One reference member as its control shows it: the posted `{prefix}.ref.<member>`
/// on a re-render, else the stored row's, which holds `label` as `text`.
fn reference_member(posted: Option<&FormBody>, held: Option<&Value>, prefix: &str, member: &str) -> String {
    match posted {
        Some(body) => body.get(&format!("{prefix}.ref.{member}")).unwrap_or_default().to_string(),
        None => held
            .and_then(|row| row.get(if member == "label" { "text" } else { member }))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
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
    let row_name = format!("{}.row", field.id);
    let posted = rows.posted.filter(|body| body.has(&row_name));
    for (position, key) in keys.iter().enumerate() {
        let prefix = format!("{}.{key}", field.id);
        let held = stored.get(position).copied();
        let body = reference_controls(field, &prefix, types, rows, |member| {
            reference_member(posted, held, &prefix, member)
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
                        format_field(
                                format!("{prefix}.pid"),
                                row_label(PID_LABEL, position),
                                format_kind(field, "pid"),
                                Some(PID_HINT),
                                rows,
                            )
                            .value(pid)
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

/// Funding: either a list of grants or one free text. The discriminant is on the
/// field, so the chooser sits above the list; both branches stay in the DOM with
/// the inactive one `hidden`, as in [`text_or_reference_rows`].
fn funding(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let stored = stored_rows(field, draft);
    let is_grants = match rows.posted.filter(|body| body.has(&format!("{}.kind", field.id))) {
        Some(body) => body.get(&format!("{}.kind", field.id)) == Some(GRANTS_KIND),
        // A string is the free-text variant; anything else renders as grants.
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
        // Blanks are dropped: the render appends its own trailing blank below, so a
        // posted blank read back as a funder would grow the row on every re-render.
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
                    // One control per stored funder plus a blank, all posting under one
                    // name so FormBody::all collects them.
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
                        format_field(
                                format!("{prefix}.url"),
                                GRANT_URL_LABEL,
                                format_kind(field, "url"),
                                None,
                                rows,
                            )
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
/// Its own function per the repo's rule on nested `html!` `maudfmt` cannot see.
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
    // Every funder in a grant posts under one {prefix}.funder, which is how
    // apply_funding collects them; only unique differs, for ids and the search box.
    let value_name = format!("{prefix}.funder");
    let unique = format!("{prefix}.funder-{index}");
    html! {
        div class="flex flex-col gap-2" {
            ({
                let slot = Slot {
                    value_name: &value_name,
                    unique: &unique,
                    label,
                    id,
                    row_context,
                    focus: false,
                    incomplete: false,
                };
                agent_picker(slot, rows, html! {})
            })
            @if rows.propose { (propose_controls(resolved, id, row_context)) }
        }
    }
}

/// Labels for the publication, funding and grant controls.
const CITATION_LABEL: &str = "Reference";
const PID_LABEL: &str = "Persistent identifier";
const PID_HINT: &str = "Only if the publication has one.";
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
const ADD_ROLE_HINT: &str = "For a role the list does not offer. Type it and save; it joins the list above. To remove \
                             it, untick it and save.";

/// The label on an agent row's control.
const AGENT_ID_LABEL: &str = "Person or organisation";

/// What is shown beside an id that resolves to nobody. Said here as well as at
/// submit, because the form is where it can be fixed.
const UNRESOLVED_AGENT: &str = "No person or organisation with this id — pick one from the list.";

/// A row control's label, with the row's own position in it: several controls
/// with one name are indistinguishable in a screen reader's form-field list. The
/// repeatable-list tile numbers its Remove buttons for the same reason.
fn row_label(label: &str, position: usize) -> String {
    format!("{label} {}", position + 1)
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// What one row of a repeatable field is called, for the tile's "Remove <noun> 2"
/// labels: the field's label singularised crudely, rather than a second
/// vocabulary in the registry for one word per field.
fn row_noun(field: &Field) -> String {
    let label = field.label.to_lowercase();
    label.strip_suffix('s').map_or(label.clone(), str::to_string)
}

/// A list of language maps: one row per map, each a text field and a language
/// select per language the map holds, added and removed by server round-trip.
///
/// A select offers [`UI_LANGUAGES`] plus its own line's tag: a tag with no option
/// would post as whichever language the browser then selects. Lines stop at
/// [`MAX_VALUES_PER_FIELD`] across the field, as the applier's do.
fn multilingual_rows(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let mut keys = row_keys(field, draft, rows);
    keys.truncate(MAX_VALUES_PER_FIELD);
    let action = format!("{}/{}", rows.action, field.id);
    let noun = row_noun(field);
    let name = capitalised(&noun);
    let mut budget = MAX_VALUES_PER_FIELD;
    let row_lines: Vec<Vec<(String, String)>> = keys
        .iter()
        .enumerate()
        .map_while(|(position, key)| {
            let mut lines = row_lines(field, draft, rows, key, position);
            lines.truncate(budget);
            budget -= lines.len();
            (!lines.is_empty()).then_some(lines)
        })
        .collect();
    let mut list = repeatable_list(field.id, labelled(field), &action)
        .item_noun(&noun)
        .add_label(format!("Add {noun}"));
    for (position, (key, lines)) in keys.iter().zip(&row_lines).enumerate() {
        let prefix = format!("{}.{key}", field.id);
        let focus = is_added_row(field, rows, position, keys.len());
        list = list.row(
            key,
            html! {
                div class="flex flex-col gap-2" data-init=[focus.then_some(FOCUS_AUTOFOCUS)] {
                    @for (line, (tag, text)) in lines.iter().enumerate() {
                        @let suffix = if line == 0 {
                            String::new()
                        } else {
                            format!(", translation {}", line + 1)
                        };
                        @let id = format!("{}-{key}-{line}", field.id);
                        div class="flex flex-wrap items-end gap-2" {
                            div {
                                ({
                                    select(
                                            format!("{prefix}.lang"),
                                            format!("Language of {noun} {}{suffix}", position + 1),
                                        )
                                        .options(
                                            UI_LANGUAGES
                                                .iter()
                                                .copied()
                                                .chain(
                                                    (!UI_LANGUAGES.contains(&tag.as_str()))
                                                        .then_some(tag.as_str()),
                                                )
                                                .map(|tag| (tag, language_name(tag))),
                                        )
                                        .selected(tag)
                                        .with_id(format!("{id}-lang"))
                                })
                            }
                            div class="grow min-w-0" {
                                ({
                                    let input = text_field(
                                            format!("{prefix}.text"),
                                            format!("{}{suffix}", row_label(&name, position)),
                                        )
                                        .input_type(InputType::Text)
                                        .value(text)
                                        .with_id(format!("{id}-text"));
                                    match focus && line == 0 {
                                        true => input.autofocus(),
                                        false => input,
                                    }
                                })
                            }
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

/// The values a [`Shape::Choice`] offers, or none for any other shape. Unreachable
/// while the registry and [`control`]'s dispatch agree; empty rather than a panic,
/// so a group with no options is a visible gap rather than a dead section.
const fn choices(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::Choice(values) => values,
        _ => &[],
    }
}

/// Which choice set a shape names, defaulting to an empty closed one: the
/// fail-safe of the two, since an open one would accept anything a hand-built
/// body sent.
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
/// does not carry. The options are the offer unioned with what the project
/// holds, as the multilingual widget does: a value with no control posts nothing
/// and would be dropped. An open set also gets a text input named after the
/// field itself, so a typed value arrives as one more repeated value and the next
/// render gives it a checkbox. A language list names held values it does not
/// know in the group's own hint, so a screen reader reads them with the group.
fn string_list(field: &Field, draft: &ProjectDraft, set: ChoiceSet, rows: Rows<'_>) -> Markup {
    let stored = stored_list(field, draft);
    let mut options: Vec<&str> = set.offered().to_vec();
    for value in &stored {
        if !options.contains(value) {
            options.push(value);
        }
    }

    let typed = matches!(set, ChoiceSet::Open(_) | ChoiceSet::Languages(_)) && add_text_posted(rows, field.id);
    let token = reset_token(typed);
    // The token also renames the group, and with it every checkbox: a checkbox the
    // morph reuses keeps the live `checked` the depositor's untick left behind, so
    // an added value taking an unticked one's slot would show unchecked.
    let group_id = match token.as_deref() {
        Some(token) => format!("{}-{token}", field.id),
        None => field.id.to_string(),
    };
    let mut group = checkbox_group(field.id, labelled(field))
        .with_id(group_id)
        .options(options.iter().map(|value| (*value, label_for(field, value))))
        .checked(stored.iter().copied());
    let unlisted: Vec<&str> = match set {
        ChoiceSet::Languages(_) => stored
            .iter()
            .copied()
            .filter(|value| editor_core::languages::canonical_code(value).is_none())
            .collect(),
        _ => Vec::new(),
    };
    if let Some(hint) = group_hint(field, &unlisted) {
        group = group.hint(hint);
    }
    let languages = matches!(set, ChoiceSet::Languages(_));
    html! {
        (group)
        @if matches!(set, ChoiceSet::Open(_) | ChoiceSet::Languages(_)) {
            ({
                let id = add_box_id(field.id, stored.len(), token.as_deref());
                let input = text_field(field.id, ADD_ANOTHER)
                    .input_type(InputType::Text)
                    .hint(if languages { LANGUAGE_ADD_HINT } else { ADD_ANOTHER_HINT })
                    .with_id(&id);
                let input = if languages {
                    input.list(LANGUAGE_NAMES_ID)
                } else {
                    input
                };
                add_box(&id, input, typed && rows.adding.is_none())
            })
        }
        @if languages { (language_names()) }
    }
}

/// A fixed id, unlike the add box's: the list does not change with the stored
/// count and is morphed together with the box that points at it.
const LANGUAGE_NAMES_ID: &str = "data-language-names";

/// The option value is the name, which the server maps back to its code. The
/// browser does not restrict what is typed (and may lack datalist support), so
/// the server's resolution and its hint for an unknown name stay the check.
fn language_names() -> Markup {
    html! {
        datalist id=(LANGUAGE_NAMES_ID) {
            @for language in editor_core::languages::LANGUAGES {
                option value=(language.name) {}
            }
        }
    }
}

/// The field's hint plus a note on held values that are not in the language
/// list, which no checkbox label can say.
fn group_hint(field: &Field, unlisted: &[&str]) -> Option<Markup> {
    if field.hint.is_none() && unlisted.is_empty() {
        return None;
    }
    Some(html! {
        @if let Some(hint) = field.hint { (hint) " " }
        @if !unlisted.is_empty() {
            "Not in the language list: "
            (unlisted.join(", "))
            ". "
            (UNLISTED_ADVICE)
        }
    })
}

const UNLISTED_ADVICE: &str = "Type a language name from the list instead, or untick it and save.";

/// A token that makes this response's add box (and, on a list, its checkbox
/// group) differ from every earlier one, or `None` when the request posted no text
/// in it.
///
/// The box renders no value, so the morph resets its typed text only when it
/// replaces the node, and it replaces a node only when the id changes. The count
/// alone does not change it: a value already held (deduplicated) or one unticked
/// as another is added leaves it unchanged. The clock differs from the previous
/// response's across restarts and instances. A render that posted nothing carries
/// no token, so a `GET` is deterministic.
fn reset_token(typed: bool) -> Option<String> {
    typed.then(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        format!("{nanos:x}")
    })
}

fn add_box_id(base: &str, held: usize, token: Option<&str>) -> String {
    match token {
        Some(token) => format!("{base}-add-{held}-{token}"),
        None => format!("{base}-add-{held}"),
    }
}

/// Whether the request posted text in the "add one more" box named `name`.
///
/// The box shares its name with the checkboxes it follows, and a browser submits
/// in document order, so its value is the last one under that name: always sent,
/// empty when untouched. An open list's typed value is also a ticked value on the
/// next render, so no comparison against the options could tell them apart.
fn add_text_posted(rows: Rows<'_>, name: &str) -> bool {
    rows.submitted
        .or(rows.posted)
        .and_then(|body| body.all(name).last())
        .is_some_and(|text| !text.trim().is_empty())
}

/// The add box in a wrapper that refocuses it where the enhanced path morphs a
/// replacement in. The wrapper's id follows the box's, so it is replaced with it
/// and `data-init` runs again. Only a box that had focus is refocused (see
/// [`FOCUS_IF_LOST`]): a depositor who Tabbed on keeps their place.
fn add_box(id: &str, input: TextFieldBuilder, focus: bool) -> Markup {
    let input = if focus { input.autofocus() } else { input };
    focus_wrapper(id, input, focus)
}

/// `content` in the wrapper [`add_box`] describes; the caller marks the input inside it
/// `autofocus`.
fn focus_wrapper(id: &str, content: impl Render, focus: bool) -> Markup {
    html! {
        div id=(format!("focus-{id}")) data-init=[focus.then_some(FOCUS_IF_LOST)] { (content) }
    }
}

/// What the "add one more" input is called on an open list.
const ADD_ANOTHER: &str = "Add another";

/// Why that input is there, and what happens to what goes in it.
const ADD_ANOTHER_HINT: &str = "Type one more value and save; it joins the list above. To remove a value, untick it \
                                and save.";

const LANGUAGE_ADD_HINT: &str = "Type a language name, for example Hungarian; suggestions may appear as you type.";

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

/// Which URL slot a shape names, defaulting to the secondary: the depositor's own
/// field, so a control wired to it by mistake cannot overwrite the RDU-only DaSCH
/// address.
const fn slot(shape: Shape) -> UrlSlot {
    match shape {
        Shape::Url(slot) => slot,
        _ => UrlSlot::Secondary,
    }
}

/// One of the project's two URLs, as a plain text control. Reads through
/// [`ProjectDraft::url_slot`], so it renders whichever representation the project
/// stores; a placeholder renders empty, as [`scalar_value`] does.
fn url(field: &Field, draft: &ProjectDraft, slot: UrlSlot, rows: Rows<'_>) -> Markup {
    let value = draft.url_slot(slot).filter(|text| !is_placeholder(text)).unwrap_or_default();
    formatted_text(field, value, rows)
}

/// A closed choice as a radio group. The wire value doubles as the label, because
/// both vocabularies are already depositor-facing prose.
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

/// A closed choice as a `<select>`, with no placeholder option: the contract
/// types these as enums with no unset variant, and `apply_choice` ignores a value
/// outside the offered set, so a placeholder would silently do nothing when
/// picked.
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

/// The character bound the registry's hint states ("Up to 200 characters"), so
/// the control enforces what the reader was told.
const SHORT_DESCRIPTION_MAX: u32 = 200;

/// The value a scalar control shows: the stored string, with a placeholder
/// sentinel rendered as empty. The rule the untouched-save guarantee rests on:
/// each sentinel renders empty and posts empty, and `apply_text` recognises that
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
    // required is absent even on a Required field: a draft may be missing
    // anything. The obligation is stated in words and enforced at submit.
    html! {
        (control)
    }
}

/// A field's single text control holding `value`, with its hint, and its format
/// where the registry declares one.
pub(crate) fn formatted_text(field: &Field, value: &str, rows: Rows<'_>) -> Markup {
    let control =
        format_field(field.id.to_string(), labelled(field), format_kind(field, ""), field.hint, rows).value(value);
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
/// the field. The tags are [`UI_LANGUAGES`] plus whatever the value carries;
/// a closed set would drop `ar` on the first save, since a tag with no control
/// posts nothing. A `<fieldset>` because each control's `<label>` is the
/// language, so the field's own name can only be a `<legend>`.
pub(crate) fn multilingual(field: &Field, draft: &ProjectDraft, rows: u32) -> Markup {
    let value = draft.multilingual(field.id);
    let tags: Vec<&str> = UI_LANGUAGES.iter().copied().chain(value.extra_tags()).collect();
    let hint_id = field.hint.map(|_| format!("{}-hint", field.id));
    html! {
        fieldset class="field field-group" id=(field.id) aria-describedby=[hint_id.as_deref()] {
            legend class="field-label" { (labelled(field)) }
            @if let Some(hint) = field.hint {
                p class="field-hint" id=[hint_id.as_deref()] { (hint) }
            }
            div class="flex flex-col gap-3" {
                @for tag in &tags {
                    ({
                        textarea(format!("{}.{tag}", field.id), language_name(tag))
                            .rows(rows)
                            .value(value.get(tag).unwrap_or_default())
                    })
                }
            }
        }
    }
}

/// A field's label: its name, and its obligation as a pill inside it. Inside,
/// because no input carries `required` or `aria-required`, which leaves the
/// accessible name as the only channel the tier has. `pub(crate)` because
/// `crate::entity`'s scalar control reuses it.
pub(crate) fn labelled(field: &Field) -> Markup {
    html! {
        (field.label)
        @if let Some(obligation) = field.obligation {
            " "
            span class=(pill_class(obligation)) { (obligation.label()) }
        }
    }
}

/// The pill's classes, as a complete literal per tier: Tailwind collects classes
/// by scanning source text, so a class built at runtime renders unstyled.
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
    use editor_core::form::registry::{sections_for, Audience, Section, FIELDS, SECTIONS};

    use super::*;

    /// A draft over a real committed project, so a control renders against the
    /// values the corpus actually holds.
    fn published_draft() -> ProjectDraft {
        let dir = editor_core::checkout_dpe_data_dir().join("projects");
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

    fn legal_info_display_of(value: serde_json::Value) -> String {
        let field = editor_core::form::registry::field("legalInfo").expect("legalInfo is a known field");
        let mut draft = ProjectDraft::default();
        draft.set("legalInfo", value);
        value_display(field, &draft, "legalInfo-label").into_string()
    }

    #[test]
    fn a_legal_info_entry_shows_its_licence_holder_and_a_link_not_a_count() {
        let out = legal_info_display_of(serde_json::json!([{
            "license": {
                "licenseIdentifier": "CC BY 4.0",
                "licenseDate": "2024-05-01",
                "licenseURI": "https://creativecommons.org/licenses/by/4.0/"
            },
            "copyrightHolder": "University of Basel",
            "authorship": ["Jane Doe", "John Roe"]
        }]));

        assert!(out.contains("CC BY 4.0"), "{out}");
        assert!(out.contains("2024-05-01"), "{out}");
        assert!(out.contains(r#"href="https://creativecommons.org/licenses/by/4.0/""#), "{out}");
        assert!(out.contains("University of Basel"), "{out}");
        assert!(out.contains("Jane Doe, John Roe"), "{out}");
        assert!(!out.contains("entr"), "{out}");
    }

    #[test]
    fn a_serialised_legal_info_shows_all_five_members() {
        let legal_info = shared_metadata::project::LegalInfo {
            license: shared_metadata::project::License {
                license_identifier: "CC BY 4.0".to_owned(),
                license_date: "2024-05-01".to_owned(),
                license_uri: "https://creativecommons.org/licenses/by/4.0/".to_owned(),
            },
            copyright_holder: "University of Basel".to_owned(),
            authorship: vec!["Jane Doe".to_owned()],
        };
        let value = serde_json::to_value([legal_info]).expect("LegalInfo serialises");

        let out = legal_info_display_of(value);

        for label in ["Licence", "Licence date", LICENCE_URI, "Copyright holder", "Authorship"] {
            assert!(out.contains(label), "{label}: {out}");
        }
        for text in [
            "CC BY 4.0",
            "2024-05-01",
            "https://creativecommons.org/licenses/by/4.0/",
            "University of Basel",
            "Jane Doe",
        ] {
            assert!(out.contains(text), "{text}: {out}");
        }
    }

    #[test]
    fn a_legal_info_placeholder_member_is_left_out_with_its_row() {
        let out = legal_info_display_of(serde_json::json!([{
            "license": {
                "licenseIdentifier": "CC BY 4.0",
                "licenseDate": "MISSING",
                "licenseURI": "https://creativecommons.org/licenses/by/4.0/"
            },
            "copyrightHolder": "University of Basel",
            "authorship": ["CALCULATED"]
        }]));

        assert!(out.contains("CC BY 4.0"), "{out}");
        for absent in ["MISSING", "CALCULATED", "Licence date", "Authorship"] {
            assert!(!out.contains(absent), "{absent} in {out}");
        }
    }

    #[test]
    fn a_legal_info_that_is_only_the_missing_placeholder_reads_as_no_value() {
        let out = legal_info_display_of(serde_json::json!(["MISSING"]));

        assert!(out.contains(NO_VALUE), "{out}");
    }

    #[test]
    fn a_legal_info_address_that_is_not_a_web_address_is_text_not_a_link() {
        let out = legal_info_display_of(serde_json::json!([{
            "license": {
                "licenseIdentifier": "CC BY 4.0",
                "licenseDate": "2024-05-01",
                "licenseURI": "javascript:alert(1)"
            },
            "copyrightHolder": "University of Basel",
            "authorship": []
        }]));

        assert!(out.contains("javascript:alert(1)"), "{out}");
        assert!(!out.contains("href"), "{out}");
    }

    #[test]
    fn every_legal_info_entry_is_shown_in_a_list_and_a_link_opts_out_of_the_opener() {
        let out = legal_info_display_of(serde_json::json!([
            {
                "license": {
                    "licenseIdentifier": "CC BY 4.0",
                    "licenseDate": "2024-05-01",
                    "licenseURI": "https://creativecommons.org/licenses/by/4.0/"
                },
                "copyrightHolder": "University of Basel",
                "authorship": ["Jane Doe"]
            },
            {
                "license": {
                    "licenseIdentifier": "CC0 1.0",
                    "licenseDate": "2023-01-01",
                    "licenseURI": "MISSING"
                },
                "copyrightHolder": "ETH Zurich",
                "authorship": ["John Roe"]
            }
        ]));

        for shown in [
            "CC BY 4.0",
            "University of Basel",
            "Jane Doe",
            "CC0 1.0",
            "ETH Zurich",
            "John Roe",
        ] {
            assert!(out.contains(shown), "{shown} in {out}");
        }
        assert_eq!(out.matches("<li").count(), 2, "{out}");
        assert!(out.contains(r#"rel="noopener noreferrer""#), "{out}");
        assert!(out.contains(r#"aria-labelledby="legalInfo-label""#), "{out}");
    }

    #[test]
    fn a_legal_info_entry_with_only_a_holder_shows_only_the_holder() {
        let out = legal_info_display_of(serde_json::json!([{
            "license": {
                "licenseIdentifier": "MISSING",
                "licenseDate": "MISSING",
                "licenseURI": "MISSING"
            },
            "copyrightHolder": "University of Basel",
            "authorship": ["MISSING"]
        }]));

        assert!(out.contains("University of Basel"), "{out}");
        for absent in ["Licence", "Authorship", "MISSING", "could not be read"] {
            assert!(!out.contains(absent), "{absent} in {out}");
        }
    }

    #[test]
    fn a_legal_info_entry_missing_a_member_still_shows_the_rest() {
        let out = legal_info_display_of(serde_json::json!([{
            "license": {
                "licenseIdentifier": "CC BY 4.0",
                "licenseDate": "2024-05-01",
                "licenseURI": "https://creativecommons.org/licenses/by/4.0/"
            },
            "copyrightHolder": "University of Basel"
        }]));

        assert!(out.contains("CC BY 4.0"), "{out}");
        assert!(out.contains("University of Basel"), "{out}");
        assert!(!out.contains("Authorship"), "{out}");
    }

    #[test]
    fn a_legal_info_entry_with_nothing_readable_is_said_so_not_dropped() {
        let out = legal_info_display_of(serde_json::json!([{ "licence": "CC BY 4.0" }]));

        assert!(out.contains("Entry 1 could not be read."), "{out}");
        assert!(!out.contains(NO_VALUE), "{out}");
    }

    #[test]
    fn a_two_language_keyword_is_one_row_of_two_lines_and_a_held_tag_keeps_its_option() {
        let field = editor_core::form::registry::field("keywords").expect("keywords is a known field");
        let mut draft = ProjectDraft::default();
        draft.set(
            "keywords",
            serde_json::json!([{ "de": "Nomaden", "en": "Nomads" }, { "ar": "x" }]),
        );
        let out = multilingual_rows(field, &draft, Rows { action: "/x", ..Rows::default() }).into_string();

        assert_eq!(out.matches(r#"name="keywords.r0.lang""#).count(), 2, "{out}");
        assert!(out.contains(r#"<option value="de" selected>"#), "{out}");
        assert!(out.contains(r#"<option value="en" selected>"#), "{out}");
        assert!(out.contains(r#"<option value="ar" selected>Arabic</option>"#), "{out}");
    }

    #[test]
    fn the_keyword_row_an_add_just_appended_takes_focus() {
        let field = editor_core::form::registry::field("keywords").expect("keywords is a known field");
        let draft = ProjectDraft::default();
        let rows = Rows { action: "/x", adding: Some("keywords"), ..Rows::default() };
        let added = multilingual_rows(field, &draft, rows).into_string();
        assert_eq!(added.matches(" autofocus").count(), 1, "{added}");
        assert!(added.find(" autofocus") > added.find(r#"name="keywords.r0.text""#), "{added}");
        assert_eq!(added.matches("data-init").count(), 1, "{added}");

        let plain = multilingual_rows(field, &draft, Rows { adding: None, ..rows }).into_string();
        assert!(!plain.contains("autofocus"), "{plain}");
    }

    #[test]
    fn the_add_input_gets_a_new_id_once_a_value_lands() {
        let field = editor_core::form::registry::field("dataLanguage").expect("dataLanguage is a known field");
        let set = set(field.shape.expect("dataLanguage has a shape"));
        let add_id = |draft: &ProjectDraft| {
            let out = string_list(field, draft, set, Rows::default()).into_string();
            let at = out.find(r#"id="dataLanguage-add"#).expect("an open list renders its add input");
            out[at..].split('"').nth(1).expect("the id is quoted").to_string()
        };
        let mut draft = ProjectDraft::default();
        let before = add_id(&draft);
        draft.set("dataLanguage", serde_json::json!(["hu"]));
        assert_ne!(before, add_id(&draft));
    }

    #[test]
    fn the_add_box_is_replaced_and_refocused_when_the_request_posted_text_in_it() {
        let field = editor_core::form::registry::field("dataLanguage").expect("dataLanguage is a known field");
        let set = set(field.shape.expect("dataLanguage has a shape"));
        let mut draft = ProjectDraft::default();
        draft.set("dataLanguage", serde_json::json!(["hu"]));
        let render = |pairs: Vec<(&str, &str)>| {
            let body = editor_core::form::FormBody::from_pairs(
                pairs
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
            );
            // `submitted` and not `posted`: an explicit save keeps no rows but still answers a
            // body.
            string_list(field, &draft, set, Rows { submitted: Some(&body), ..Rows::default() }).into_string()
        };
        let add_id = |out: &str| {
            let at = out.find(r#"id="dataLanguage-add"#).expect("the add box renders");
            out[at..].split('"').nth(1).expect("the id is quoted").to_string()
        };

        let plain = string_list(field, &draft, set, Rows::default()).into_string();
        assert_eq!(add_id(&plain), "dataLanguage-add-1");
        assert!(!plain.contains(" autofocus"), "{plain}");

        // The same count as before: "hu" is already held, so the typed name dedupes.
        let typed = render(vec![("dataLanguage", "hu"), ("dataLanguage", "Hungarian")]);
        assert_ne!(add_id(&typed), add_id(&plain), "{typed}");
        assert_eq!(typed.matches(" autofocus").count(), 1, "{typed}");
        assert_eq!(typed.matches("data-init").count(), 1, "{typed}");
        assert!(
            typed.contains("document.activeElement"),
            "refocus only when focus was lost: {typed}"
        );
        // The checkboxes are renamed with the box, so the morph cannot reuse one the depositor
        // unticked.
        assert!(plain.contains(r#"id="dataLanguage-0""#), "{plain}");
        assert!(!typed.contains(r#"id="dataLanguage-0""#), "{typed}");

        // A ticked value and an untouched add box post nothing to refocus for.
        let untouched = render(vec![("dataLanguage", "hu"), ("dataLanguage", "")]);
        assert_eq!(add_id(&untouched), add_id(&plain));
        assert!(!untouched.contains(" autofocus"), "{untouched}");
    }

    #[test]
    fn the_data_language_add_box_suggests_language_names_and_an_open_list_does_not() {
        let out = data_language_control(&[]);
        assert_eq!(out.matches("<datalist").count(), 1, "{out}");
        assert!(out.contains(&format!(r#"<datalist id="{LANGUAGE_NAMES_ID}">"#)), "{out}");
        assert!(out.contains(r#"<option value="Hungarian"></option>"#), "{out}");
        assert!(out.contains(&format!(r#"list="{LANGUAGE_NAMES_ID}""#)), "{out}");

        // The suggestions outlive a value landing, which only renames the add box.
        let held = data_language_control(&["hu"]);
        assert!(held.contains(&format!(r#"<datalist id="{LANGUAGE_NAMES_ID}">"#)), "{held}");

        let open = string_list(
            editor_core::form::registry::field("typeOfData").expect("typeOfData is a known field"),
            &ProjectDraft::default(),
            ChoiceSet::Open(&[]),
            Rows::default(),
        )
        .into_string();
        assert!(open.contains("-add-"), "an open list keeps its add box: {open}");
        assert!(!open.contains("<datalist"), "{open}");
        assert!(!open.contains("list="), "{open}");
    }

    fn data_language_control(held: &[&str]) -> String {
        let field = editor_core::form::registry::field("dataLanguage").expect("dataLanguage is a known field");
        let mut draft = ProjectDraft::default();
        draft.set("dataLanguage", serde_json::json!(held));
        string_list(
            field,
            &draft,
            set(field.shape.expect("dataLanguage has a shape")),
            Rows::default(),
        )
        .into_string()
    }

    #[test]
    fn a_data_language_preset_keeps_its_label() {
        let out = data_language_control(&[]);
        assert!(out.contains("German (de)"), "{out}");
    }

    #[test]
    fn a_held_language_the_list_does_not_know_is_named_in_the_hint() {
        let out = data_language_control(&["de", "Belgian"]);
        assert!(out.contains("Not in the language list: Belgian."), "{out}");
        assert!(out.contains("untick it"), "{out}");
        assert!(!out.contains("Not in the language list: de"), "{out}");

        let listed = data_language_control(&["de", "hu"]);
        assert!(!listed.contains("Not in the language list"), "{listed}");
    }

    #[test]
    fn every_shaped_field_has_a_control() {
        // The fallback arm's debug_assert! is compiled out of a release build, so
        // a field given a shape with no arm here would render the unbuilt note in
        // production with nothing failing in CI. Every section, for the audience
        // that sees the most.
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
                    // A group posts under the field's name from each control.
                    Some(Shape::Choice(_)) => out.contains(&format!(r#"name="{}""#, field.id)),
                    // A URL slot posts under the field's own name, like a scalar.
                    Some(Shape::Url(_) | Shape::StringList(_)) => out.contains(&format!(r#"name="{}""#, field.id)),
                    // A language list's first stored row posts the line pair the
                    // applier reads, which the marker alone does not show.
                    Some(Shape::MultilingualRows) if !stored_rows(field, &draft).is_empty() => ["lang", "text"]
                        .iter()
                        .all(|part| out.contains(&format!(r#"name="{}.r0.{part}""#, field.id))),
                    // A repeatable field posts {field}.row even when empty: the marker.
                    Some(
                        Shape::MultilingualRows
                        | Shape::StringRows
                        | Shape::AgentRows
                        | Shape::AttributionRows
                        | Shape::TextOrReferenceRows(_)
                        | Shape::ReferenceRows(_)
                        | Shape::PublicationRows,
                    ) => out.contains(&format!(r#"name="{}.row""#, field.id)),
                    // Funding's discriminant is on the field, so it is what always posts.
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
        // The canary for the absence above.
        let shaped: Vec<&str> = FIELDS
            .iter()
            .filter(|field| field.is_editable())
            .map(|field| field.id)
            .collect();
        // A count, so the canary moves deliberately as shapes land.
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
        // The label is the only channel an obligation has, and each control
        // builder composes its own label, so this is asserted per field.
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
        // A field no applier reads must post nothing, or an empty control would
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
        let dir = editor_core::checkout_dpe_data_dir();
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
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            submitted: None,
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            format_findings: &[],
            propose: true,
            focus: None,
            search_path: None,
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains("Propose a new person"), "{out}");
        assert!(out.contains("Propose a new organisation"), "{out}");
        assert!(out.contains("Propose changes"), "{out}");
        // The resolved id rides in the button's own value, so only the activated one posts it.
        assert!(out.contains(r#"value="propose-changes:organization-008""#), "{out}");
        // Nothing carries it in a field every row would post regardless of which
        // button was clicked.
        assert!(!out.contains(r#"name="propose.entity""#), "{out}");
    }

    #[test]
    fn propose_controls_are_absent_when_the_row_widget_does_not_opt_in() {
        // Rows::propose defaults to false; the entity form's affiliations field
        // reuses this widget and must not offer a control its route does not
        // dispatch.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["person-999999"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            submitted: None,
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            format_findings: &[],
            propose: false,
            focus: None,
            search_path: None,
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
    fn a_changed_slot_lists_choose_buttons_below_the_box_and_keeps_the_stored_id_posting() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let fresh = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let hidden = r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#;

        let quiet = agent_rows(field, &draft, fresh).into_string();
        assert!(quiet.contains(hidden), "{quiet}");
        assert!(!quiet.contains("choose-agent"), "no matches before a search: {quiet}");

        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("contactPoint.r0.q".to_string(), "Universität".to_string()),
        ]);
        let out = agent_rows(field, &draft, Rows { posted: Some(&body), ..fresh }).into_string();
        assert!(out.contains(hidden), "{out}");
        let search = out.find(r#"name="contactPoint.r0.q""#).expect("search box");
        let first = out.find(r#"value="choose-agent:contactPoint.r0:"#).expect("choose buttons");
        assert!(search < first, "the matches are below the search box: {out}");
        assert!(out.contains(r#"value="Universität""#), "the query comes back: {out}");
        assert!(!out.contains("Keep "), "no way to keep the old choice but Escape: {out}");
    }

    #[test]
    fn a_choice_is_written_into_its_own_picker_and_closes_its_search() {
        let row = apply_picker_intents(
            pairs(&[
                ("attributions.r1.contributor", ""),
                ("attributions.r1.contributor.q", "Rita"),
                ("intent", "choose-agent:attributions.r1.contributor:person-025"),
            ]),
            shape_of,
        );
        assert!(row.contains(&("attributions.r1.contributor".to_string(), "person-025".to_string())));
        assert!(!row.iter().any(|(name, _)| name == "attributions.r1.contributor.q"), "{row:?}");

        // Funders share one name; the choice lands on the funder searched in, not the first.
        let funders = apply_picker_intents(
            pairs(&[
                ("funding.r0.funder", "organization-002"),
                ("funding.r0.funder", ""),
                ("funding.r0.funder-1.q", "SNF"),
                ("intent", "choose-agent:funding.r0.funder-1:organization-128"),
            ]),
            shape_of,
        );
        let ids: Vec<&str> = funders
            .iter()
            .filter(|(name, _)| name == "funding.r0.funder")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(ids, ["organization-002", "organization-128"]);
    }

    #[test]
    fn after_a_choice_the_picker_shows_the_chosen_agent_as_a_card_with_its_change_button_focused() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let body = editor_core::form::FormBody::from_pairs(apply_picker_intents(
            vec![
                ("contactPoint.row".to_string(), "r0".to_string()),
                ("contactPoint.r0".to_string(), String::new()),
                ("contactPoint.r0.q".to_string(), "Universität".to_string()),
                (
                    "intent".to_string(),
                    "choose-agent:contactPoint.r0:organization-008".to_string(),
                ),
            ],
            shape_of,
        ));
        let out = agent_rows(
            field,
            &published_draft(),
            Rows {
                submitted: None,
                posted: Some(&body),
                action: "/x",
                adding: None,
                agents: Some(&scope),
                format_findings: &[],
                propose: false,
                focus: None,
                search_path: None,
            },
        )
        .into_string();
        assert!(out.contains("Dokumentationsbibliothek St. Moritz"), "{out}");
        assert!(
            out.contains("autofocus>Change</button>"),
            "focus goes to the card's Change button: {out}"
        );
        assert!(!out.contains(r#"name="contactPoint.r0.q""#), "the search is closed: {out}");
    }

    #[test]
    fn only_the_row_an_add_appended_takes_focus() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            action: "/x",
            adding: Some("contactPoint"),
            agents: Some(&scope),
            ..Rows::default()
        };

        let added = agent_rows(field, &draft, rows).into_string();
        // " autofocus" is the attribute; the data-init expression names it as "[autofocus]".
        assert_eq!(added.matches(" autofocus").count(), 1, "{added}");
        assert!(added.find(" autofocus") > added.find(r#"name="contactPoint.r1.q""#), "{added}");
        assert_eq!(
            added.matches(FOCUS_AUTOFOCUS).count(),
            1,
            "the enhanced path focuses it too: {added}"
        );

        let plain = agent_rows(field, &draft, Rows { adding: None, ..rows }).into_string();
        assert!(!plain.contains("autofocus"), "{plain}");
        assert!(!plain.contains(FOCUS_AUTOFOCUS), "{plain}");
    }

    #[test]
    fn a_search_that_matches_nobody_says_so_rather_than_offering_an_empty_menu() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("contactPoint.r0.q".to_string(), "zzzznobodyzzzz".to_string()),
        ]);
        let out = agent_rows(
            field,
            &draft,
            Rows {
                submitted: None,
                posted: Some(&body),
                action: "/x",
                adding: None,
                agents: Some(&scope),
                format_findings: &[],
                propose: true,
                focus: None,
                search_path: None,
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
        // Between the picker and the roles, a bare "Propose changes" reads as an
        // offer to propose the roles, which are project data.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows {
            submitted: None,
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            format_findings: &[],
            propose: true,
            focus: None,
            search_path: None,
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
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            submitted: None,
            posted: None,
            action: "/x",
            adding: None,
            agents: Some(&scope),
            format_findings: &[],
            propose: true,
            focus: None,
            search_path: None,
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains("Propose changes to this organisation's details"), "{out}");
        assert!(!out.contains("person's details"), "{out}");
    }

    #[test]
    fn the_add_another_role_input_does_not_come_back_as_an_unlabelled_checkbox() {
        // It shares its wire name with the checkbox group, so its empty value would
        // come back as a ticked checkbox with no label on every re-render.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let body = posted_contributor_row();
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows {
            submitted: None,
            posted: Some(&body),
            action: "/x",
            adding: None,
            agents: Some(&scope),
            format_findings: &[],
            propose: true,
            focus: None,
            search_path: None,
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
    fn a_reference_row_re_rendered_from_a_body_shows_what_was_posted() {
        // A posted render that read the stored member names would show an empty
        // control, and the next save would post the empty value back.
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("spatialCoverage.row".to_string(), "r0".to_string()),
            ("spatialCoverage.r0.ref.type".to_string(), "Pleiades".to_string()),
            ("spatialCoverage.r0.ref.url".to_string(), "Basel".to_string()),
            ("spatialCoverage.r0.ref.label".to_string(), "Basle".to_string()),
        ]);
        let field = editor_core::form::registry::field("spatialCoverage").expect("spatialCoverage is a known field");
        let rows = Rows { posted: Some(&body), action: "/x", ..Rows::default() };

        let out = reference_rows(field, &published_draft(), rows, &["Geonames", "Pleiades"]).into_string();
        assert!(
            out.contains(r#"name="spatialCoverage.r0.ref.url" type="text" value="Basel""#),
            "{out}"
        );
        assert!(out.contains(r#"value="Basle""#), "{out}");
        assert!(out.contains(r#"<option value="Pleiades" selected>"#), "{out}");
    }

    fn input_named<'a>(out: &'a str, control: &str) -> Option<&'a str> {
        out.split("<input")
            .find(|input| input.contains(&format!(r#"name="{control}""#)))
    }

    #[test]
    fn a_format_finding_marks_only_the_control_it_names() {
        let field = editor_core::form::registry::field("spatialCoverage").expect("spatialCoverage is a known field");
        let mut draft = ProjectDraft::default();
        draft.set(
            "spatialCoverage",
            serde_json::json!([
                { "type": "Geonames", "url": "https://www.geonames.org/1" },
                { "type": "Geonames", "url": "Basel" },
            ]),
        );
        let findings = [Finding {
            field,
            control: "spatialCoverage.r1.ref.url".to_string(),
            kind: ValueKind::Link,
        }];
        let rows = Rows { action: "/x", format_findings: &findings, ..Rows::default() };

        let out = reference_rows(field, &draft, rows, &["Geonames"]).into_string();
        let input = input_named(&out, "spatialCoverage.r1.ref.url").expect("the second row's link control");
        assert!(input.contains(r#"aria-invalid="true""#), "{input}");
        assert_eq!(out.matches("aria-invalid").count(), 1, "only the flagged control: {out}");
        assert!(out.contains(format_message(ValueKind::Link)), "{out}");
        // On the control, never in an alert: an alert inside the polite status
        // region interrupts.
        assert!(!out.contains(r#"role="alert""#), "{out}");
    }

    #[test]
    fn the_reported_values_render_their_message_at_their_control() {
        let mut draft = ProjectDraft::default();
        draft.set("documentationMaterial", serde_json::json!(["www.dasch.swiss"]));
        draft.set(
            "publications",
            serde_json::json!([{ "text": "A paper", "pid": { "url": "doi.asdasd..asdasd" } }]),
        );
        for (id, control, kind) in [
            ("documentationMaterial", "documentationMaterial.r0", ValueKind::Link),
            ("publications", "publications.r0.pid", ValueKind::DoiOrLink),
        ] {
            let field = editor_core::form::registry::field(id).expect("a known field");
            let findings = editor_core::form::format::findings(std::iter::once(field), &draft, None);
            let rows = Rows { action: "/x", format_findings: &findings, ..Rows::default() };
            let out = field_row(field, &draft, Mode::Editable, rows).into_string();
            let input = input_named(&out, control).expect("the control holding the value");
            assert!(input.contains(r#"aria-invalid="true""#), "{id}: {input}");
            assert!(out.contains(format_message(kind)), "{id}: {out}");
        }
    }

    /// Whether `field`, rendered from a body naming one row `r0`, marks the control
    /// a finding on `member` names and gives it the hint naming the member's format.
    fn renders_finding(field: &'static Field, member: &editor_core::form::format::Member) -> bool {
        let control = editor_core::form::format::control_name(field, "r0", member);
        let body = editor_core::form::FormBody::from_pairs(vec![(format!("{}.row", field.id), "r0".to_string())]);
        let findings = [Finding { field, control: control.clone(), kind: member.kind }];
        let rows = Rows {
            posted: Some(&body),
            action: "/x",
            format_findings: &findings,
            ..Rows::default()
        };
        let out = field_row(field, &ProjectDraft::default(), Mode::Editable, rows).into_string();
        let marked = input_named(&out, &control).is_some_and(|input| input.contains(r#"aria-invalid="true""#));
        marked && out.contains(format_hint(member.kind))
    }

    #[test]
    fn every_format_member_renders_its_finding() {
        // The member table names controls by hand; a name no widget posts, or a widget
        // that does not mark its control, would refuse a submission with nothing shown.
        let canary = editor_core::form::format::Member { posted: "ref.url", stored: &["url"], kind: ValueKind::Link };
        let spatial = editor_core::form::registry::field("spatialCoverage").expect("spatialCoverage is a known field");
        assert!(renders_finding(spatial, &canary), "the check itself must see a marked control");

        let unmarked: Vec<String> = FIELDS
            .iter()
            .flat_map(|field| field.format.iter().map(move |member| (field, member)))
            .filter(|(field, member)| !renders_finding(field, member))
            .map(|(field, member)| format!("{} {}", field.id, member.posted))
            .collect();
        assert!(
            unmarked.is_empty(),
            "format members no control marks or hints (render the control through `format_field`): {unmarked:?}"
        );
    }

    #[test]
    fn a_grants_row_keeps_one_blank_funder_however_often_it_is_re_rendered() {
        // Reading the posted blank back as a held funder would append a second
        // blank on every round trip.
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "funding",
            serde_json::json!([{ "funders": ["organization-002"], "number": "1" }]),
        );
        let field = editor_core::form::registry::field("funding").expect("funding is a known field");

        let mut posted: Vec<(String, String)> = vec![
            ("funding.row".to_string(), "r0".to_string()),
            ("funding.kind".to_string(), "grants".to_string()),
            ("funding.r0.funder".to_string(), "organization-002".to_string()),
            ("funding.r0.funder".to_string(), String::new()),
        ];
        for round in 0..3 {
            let body = editor_core::form::FormBody::from_pairs(posted.clone());
            let rows = Rows {
                submitted: None,
                posted: Some(&body),
                action: "/x",
                adding: None,
                agents: Some(&scope),
                format_findings: &[],
                propose: true,
                focus: None,
                search_path: None,
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

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    }

    fn shape_of(name: &str) -> Option<Shape> {
        editor_core::form::registry::field(name).and_then(|field| field.shape)
    }

    #[test]
    fn adding_an_agent_appends_a_contributor_row_under_the_next_free_key() {
        let out = apply_picker_intents(
            pairs(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.row", "r2"),
                ("attributions.r2.contributor", "person-002"),
                ("attributions.new.q", "Rita"),
                ("intent", "add-agent:attributions:person-025"),
            ]),
            shape_of,
        );
        assert!(out.contains(&("attributions.row".to_string(), "r1".to_string())), "{out:?}");
        assert!(
            out.contains(&("attributions.r1.contributor".to_string(), "person-025".to_string())),
            "{out:?}"
        );
        assert!(!out.iter().any(|(name, _)| name == "attributions.new.q"), "{out:?}");
        assert!(out.iter().any(|(name, _)| name == "intent"), "the intent stays: {out:?}");
    }

    #[test]
    fn adding_an_agent_to_an_empty_list_drops_the_placeholder_row() {
        let out = apply_picker_intents(
            pairs(&[
                ("attributions.row", ""),
                ("attributions.new.q", "Rita"),
                ("intent", "add-agent:attributions:person-025"),
            ]),
            shape_of,
        );
        let rows: Vec<&str> = out
            .iter()
            .filter(|(name, _)| name == "attributions.row")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(rows, ["r0"], "{out:?}");
        assert!(out.contains(&("attributions.r0.contributor".to_string(), "person-025".to_string())));
    }

    #[test]
    fn adding_an_agent_appends_an_agent_rows_row() {
        let out = apply_picker_intents(
            pairs(&[
                ("contactPoint.row", "r0"),
                ("contactPoint.r0", "organization-008"),
                ("contactPoint.new.q", "Bern"),
                ("intent", "add-agent:contactPoint:organization-002"),
            ]),
            shape_of,
        );
        assert!(out.contains(&("contactPoint.row".to_string(), "r1".to_string())), "{out:?}");
        assert!(
            out.contains(&("contactPoint.r1".to_string(), "organization-002".to_string())),
            "{out:?}"
        );
        assert!(!out.iter().any(|(name, _)| name == "contactPoint.new.q"), "{out:?}");
    }

    #[test]
    fn adding_an_agent_the_field_already_holds_appends_nothing() {
        let attributions = pairs(&[
            ("attributions.row", "r0"),
            ("attributions.r0.contributor", "person-001"),
            ("intent", "add-agent:attributions:person-001"),
        ]);
        assert_eq!(apply_picker_intents(attributions.clone(), shape_of), attributions);
        let contacts = pairs(&[
            ("contactPoint.row", "r0"),
            ("contactPoint.r0", "organization-008"),
            ("intent", "add-agent:contactPoint:organization-008"),
        ]);
        assert_eq!(apply_picker_intents(contacts.clone(), shape_of), contacts);
    }

    #[test]
    fn adding_an_agent_to_a_field_with_no_agent_rows_changes_nothing() {
        // `keywords` is a rows field of another shape, and `funding` holds its agents nested.
        let keywords = pairs(&[("keywords.row", "r0"), ("intent", "add-agent:keywords:person-001")]);
        assert_eq!(apply_picker_intents(keywords.clone(), shape_of), keywords);
        let funding = pairs(&[("funding.row", "r0"), ("intent", "add-agent:funding:organization-002")]);
        assert_eq!(apply_picker_intents(funding.clone(), shape_of), funding);
        let unknown = pairs(&[
            ("nosuchfield.row", "r0"),
            ("intent", "add-agent:nosuchfield:person-001"),
        ]);
        assert_eq!(apply_picker_intents(unknown.clone(), shape_of), unknown);
    }

    #[test]
    fn removing_a_role_drops_every_pair_of_that_role_in_that_row_only() {
        let out = apply_picker_intents(
            pairs(&[
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "Author"),
                ("attributions.r0.role", "Editor"),
                ("attributions.r0.role", ""),
                ("attributions.r0.role", "Author"),
                ("attributions.r1.role", "Author"),
                ("intent", "remove-role:attributions.r0:Author"),
            ]),
            shape_of,
        );
        let roles: Vec<&str> = out
            .iter()
            .filter(|(name, _)| name == "attributions.r0.role")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(roles, ["Editor", ""], "{out:?}");
        assert!(out.contains(&("attributions.r1.role".to_string(), "Author".to_string())));
    }

    /// The values of the checked checkboxes in `out`, in document order: what a browser posts.
    fn checked_roles(out: &str) -> Vec<String> {
        out.split(r#"value=""#)
            .skip(1)
            .filter_map(|rest| rest.split_once('"'))
            .filter(|(_, after)| after.starts_with(" checked"))
            .map(|(value, _)| value.to_string())
            .collect()
    }

    /// The value of the `nth` role Remove in `out`.
    fn remove_intent(out: &str, nth: usize) -> String {
        out.split(r#"name="intent" value=""#)
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .filter(|value| value.starts_with("remove-role:"))
            .nth(nth)
            .expect("a role chip")
            .to_string()
    }

    /// The opening tag in `out` that holds `needle`.
    fn tag_with<'a>(out: &'a str, needle: &str) -> &'a str {
        out.split('<')
            .find(|tag| tag.contains(needle))
            .unwrap_or_else(|| panic!("no tag holds {needle}: {out}"))
    }

    fn contributor_with_roles(roles: &[&str]) -> ProjectDraft {
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": roles }]),
        );
        draft
    }

    #[test]
    fn a_chip_s_remove_drops_exactly_its_role_when_the_body_posts_roles_in_checkbox_order() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        // Chips follow the stored order, the posted pairs follow the checkboxes: the vocabulary's,
        // then any other role.
        for (stored, chip, removed, left) in [
            (vec!["Author", "Project Leader"], 0, "Author", vec!["Project Leader"]),
            (vec!["Zeta custom", "Author"], 0, "Zeta custom", vec!["Author"]),
            (
                vec!["Author", "Editor: volume 2", "Editor"],
                1,
                "Editor: volume 2",
                vec!["Author", "Editor"],
            ),
        ] {
            let out = attribution_rows(field, &contributor_with_roles(&stored), rows).into_string();
            let intent = remove_intent(&out, chip);
            assert_eq!(intent, format!("remove-role:attributions.r0:{removed}"));

            let mut posted = pairs(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
            ]);
            posted.extend(
                checked_roles(&out)
                    .into_iter()
                    .map(|role| ("attributions.r0.role".to_string(), role)),
            );
            posted.push(("attributions.r0.role".to_string(), String::new()));
            posted.push(("intent".to_string(), intent));

            let rewritten = apply_picker_intents(posted, shape_of);
            let mut roles: Vec<&str> = rewritten
                .iter()
                .filter(|(name, value)| name == "attributions.r0.role" && !value.is_empty())
                .map(|(_, value)| value.as_str())
                .collect();
            roles.sort_unstable();
            let mut left = left;
            left.sort_unstable();
            assert_eq!(roles, left, "stored {stored:?}, chip {chip}");
        }
    }

    #[test]
    fn focus_after_a_role_chip_s_remove_goes_to_the_chip_in_its_place_then_before_it_then_the_summary() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let draft = contributor_with_roles(&[]);
        let answer = |roles: &[&str], removed: &str| {
            let mut posted = pairs(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
            ]);
            posted.extend(roles.iter().map(|role| ("attributions.r0.role".to_string(), role.to_string())));
            posted.push(("intent".to_string(), format!("remove-role:attributions.r0:{removed}")));
            let body = editor_core::form::FormBody::from_pairs(apply_picker_intents(posted, shape_of));
            attribution_rows(field, &draft, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string()
        };
        let focused = |out: &str, role: &str| tag_with(out, &format!("Remove role {role} from ")).contains("autofocus");

        // Posted in checkbox order: Project Leader, Author, Editor.
        let out = answer(&["Project Leader", "Author", "Editor"], "Author");
        assert!(focused(&out, "Editor") && !focused(&out, "Project Leader"), "{out}");
        assert!(out.contains("el.querySelector('[autofocus]')?.focus()"), "{out}");

        let out = answer(&["Project Leader", "Author", "Editor"], "Editor");
        assert!(focused(&out, "Author") && !focused(&out, "Project Leader"), "{out}");

        let out = answer(&["Author"], "Author");
        assert!(!out.contains("Remove role"), "{out}");
        assert!(tag_with(&out, "Choose roles").contains("autofocus"), "{out}");

        // Another row's Remove, and any other response, move nothing.
        let other = {
            let body = editor_core::form::FormBody::from_pairs(pairs(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "Author"),
                ("attributions.r0.role", ""),
                ("intent", "remove-role:attributions.r1:Author"),
            ]));
            attribution_rows(field, &draft, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string()
        };
        assert!(!other.contains("autofocus"), "{other}");
    }

    #[test]
    fn removing_a_role_leaves_the_server_s_open_to_win_once_the_row_holds_none() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let held = attribution_rows(field, &contributor_with_roles(&["Author"]), rows).into_string();
        assert!(held.contains(r#"data-preserve-attr="open""#), "{held}");
        let empty = attribution_rows(field, &contributor_with_roles(&[]), rows).into_string();
        assert!(!empty.contains("data-preserve-attr"), "{empty}");
        assert!(empty.contains("<details open"), "{empty}");
    }

    #[test]
    fn the_roles_summary_is_described_by_the_incomplete_marker_and_a_chip_s_remove_is_24px() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let flagged = attribution_rows(field, &contributor_with_roles(&[]), rows).into_string();
        assert!(flagged.contains(r#"<p id="attributions.r0-roles-required""#), "{flagged}");
        assert!(
            tag_with(&flagged, "Choose roles").contains(r#"aria-describedby="attributions.r0-roles-required""#),
            "{flagged}"
        );

        let held = attribution_rows(field, &contributor_with_roles(&["Author"]), rows).into_string();
        assert!(!held.contains("aria-describedby=\"attributions.r0-roles-required\""), "{held}");
        let remove = tag_with(&held, "Remove role Author from ");
        assert!(remove.contains("min-h-6") && remove.contains("min-w-6"), "{remove}");
    }

    #[test]
    fn every_search_button_names_itself_starting_with_its_visible_text() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "", "contributorType": [] }]),
        );
        let out = attribution_rows(field, &draft, rows).into_string();
        let buttons: Vec<&str> = out.split('<').filter(|tag| tag.contains(r#"value="find-agent""#)).collect();
        assert_eq!(buttons.len(), 2, "the empty row's search and the append search: {out}");
        for tag in buttons {
            assert!(tag.contains(r#"aria-label="Search "#), "{tag}");
        }
    }

    #[test]
    fn a_slot_being_changed_says_whom_a_choice_replaces() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let draft = contributor_with_roles(&["Author"]);
        let name = &agents.get("person-001").expect("a committed person").label;

        let body = editor_core::form::FormBody::from_pairs(pairs(&[
            ("attributions.row", "r0"),
            ("attributions.r0.contributor", "person-001"),
            ("attributions.r0.role", "Author"),
            ("intent", "change-agent:attributions.r0.contributor"),
        ]));
        let changing =
            attribution_rows(field, &draft, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string();
        assert!(changing.contains(&format!("Choosing a match replaces {name}.")), "{changing}");

        let card = attribution_rows(field, &draft, rows).into_string();
        assert!(!card.contains("Choosing a match replaces"), "{card}");
    }

    #[test]
    fn an_append_stops_at_the_cap_on_rows() {
        let mut full: Vec<(String, String)> = Vec::new();
        for n in 0..MAX_VALUES_PER_FIELD {
            full.push(("attributions.row".to_string(), format!("r{n}")));
            full.push((format!("attributions.r{n}.contributor"), format!("person-{n:03}")));
        }
        full.push(("intent".to_string(), "add-agent:attributions:organization-002".to_string()));
        let out = apply_picker_intents(full.clone(), shape_of);
        assert_eq!(out, full, "no pair is added to a full field");
    }

    #[test]
    fn a_body_without_a_picker_intent_is_returned_unchanged() {
        let body = pairs(&[
            ("attributions.row", "r0"),
            ("attributions.r0.role", "Author"),
            ("intent", "save"),
        ]);
        assert_eq!(apply_picker_intents(body.clone(), shape_of), body);
    }

    #[test]
    fn a_blank_query_renders_empty_results_that_say_the_box_is_collapsed() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let out = agent_results("attributions.new", "  ", &scope, &[], true).into_string();
        assert!(out.contains(r#"id="attributions.new-results""#), "{out}");
        assert!(out.contains("setAttribute('aria-expanded','false')"), "{out}");
        assert!(!out.contains("role=\"option\""), "{out}");
        assert!(!out.contains("role=\"status\""), "{out}");
    }

    #[test]
    fn a_query_over_the_cap_renders_empty_results() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let long = "a".repeat(201);
        let out = agent_results("attributions.new", &long, &scope, &[], true).into_string();
        assert!(out.contains("setAttribute('aria-expanded','false')"), "{out}");
        assert!(!out.contains("role=\"option\""), "{out}");
        assert!(!out.contains("Nothing matches"), "{out}");
        let at_cap = "a".repeat(200);
        let out = agent_results("attributions.new", &at_cap, &scope, &[], true).into_string();
        assert!(out.contains("data-combobox-status"), "a query at the cap is searched: {out}");
    }

    #[test]
    fn the_append_search_offers_add_intents_with_a_kind_badge_and_a_detail_line() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let out = agent_results("attributions.new", "person-001", &scope, &[], true).into_string();
        assert!(out.contains(r#"value="add-agent:attributions:person-001""#), "{out}");
        assert!(!out.contains("choose-agent:"), "{out}");
        assert!(out.contains("setAttribute('aria-expanded','true')"), "{out}");
        assert!(out.contains(r#"class="font-semibold""#), "{out}");
        assert!(out.contains("badge-secondary"), "{out}");
        assert!(out.contains(">Person<"), "{out}");
        assert!(out.contains("Université de Lausanne"), "the detail line: {out}");
        assert!(out.contains("data-combobox-status"), "{out}");
        assert!(!out.contains(r#"role="status""#), "the tile owns the live region: {out}");
        assert!(out.contains("1 match."), "{out}");
    }

    #[test]
    fn a_search_on_a_card_offers_choose_intents_for_that_card() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let out = agent_results("attributions.r0.contributor", "person-001", &scope, &[], true).into_string();
        assert!(
            out.contains(r#"value="choose-agent:attributions.r0.contributor:person-001""#),
            "{out}"
        );
        assert!(!out.contains("add-agent:"), "{out}");
    }

    #[test]
    fn excluded_agents_are_not_offered() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let out = agent_results("attributions.new", "person-00", &scope, &["person-001"], true).into_string();
        assert!(!out.contains("add-agent:attributions:person-001\""), "{out}");
        assert!(out.contains("add-agent:attributions:person-002\""), "{out}");
        // Everything a query finds excluded reads as no match.
        let none = agent_results("attributions.new", "person-001", &scope, &["person-001"], true).into_string();
        assert!(none.contains("Nothing matches"), "{none}");
    }

    #[test]
    fn a_search_with_no_match_offers_the_proposals_only_when_asked_to() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let with = agent_results("attributions.new", "zzzzqqqq", &scope, &[], true).into_string();
        assert!(with.contains("Nothing matches “zzzzqqqq”."), "{with}");
        assert!(with.contains("Not in the list? Propose a new person or organisation"), "{with}");
        assert!(with.contains(r#"value="propose-person""#), "{with}");
        assert!(with.contains(r#"value="propose-organization""#), "{with}");
        assert!(with.contains("setAttribute('aria-expanded','false')"), "{with}");
        let without = agent_results("attributions.new", "zzzzqqqq", &scope, &[], false).into_string();
        assert!(without.contains("Nothing matches"), "{without}");
        assert!(!without.contains("propose-"), "{without}");
    }

    #[test]
    fn a_search_hitting_the_cap_says_how_many_it_shows() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let out = agent_results("attributions.new", "person", &scope, &[], true).into_string();
        assert!(out.contains("matches, showing the first 25. Add a word to narrow it."), "{out}");
    }

    #[test]
    fn the_search_action_is_a_get_carrying_the_encoded_input_value() {
        assert_eq!(
            agent_search_action("/projects/0801d/sections/contributors/agents", "attributions.new"),
            "@get('/projects/0801d/sections/contributors/agents?for=attributions.new&q=' + \
             encodeURIComponent(evt.target.value))"
        );
        assert_eq!(agent_query_id("attributions.new"), "attributions.new-q");
    }

    #[test]
    fn a_filled_slot_is_a_card_with_name_kind_detail_the_posting_id_and_a_change_button() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["person-001"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains(r#"class="font-semibold""#), "the name: {out}");
        assert!(out.contains("badge-secondary"), "{out}");
        assert!(out.contains(">Person<"), "{out}");
        assert!(
            out.contains(r#"<p class="text-sm text-neutral-600">Université de Lausanne"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<input type="hidden" name="contactPoint.r0" value="person-001">"#),
            "{out}"
        );
        assert!(out.contains(r#"name="intent" value="change-agent:contactPoint.r0""#), "{out}");
        assert!(out.contains(r#"aria-label="Change "#), "{out}");
        assert!(out.contains(r#" (row 1)">Change</button>"#), "{out}");
        assert!(!out.contains(r#"name="contactPoint.r0.q""#), "a card has no search: {out}");
        assert!(out.contains("border-neutral-500"), "{out}");
    }

    #[test]
    fn an_id_that_resolves_to_nobody_keeps_its_text_and_its_propose_buttons() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["person-999999"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows {
            action: "/x",
            agents: Some(&scope),
            propose: true,
            ..Rows::default()
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert!(out.contains("<strong>person-999999</strong>"), "{out}");
        assert!(out.contains(UNRESOLVED_AGENT), "{out}");
        assert!(out.contains(r#"value="propose-person""#), "{out}");
        assert!(out.contains(r#"value="change-agent:contactPoint.r0""#), "{out}");
    }

    #[test]
    fn a_slot_being_changed_shows_the_search_with_its_query_and_keeps_the_hidden_value() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let hidden = r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#;

        // Just opened: the Change button was the submitter, and no query exists yet.
        let opened = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("intent".to_string(), "change-agent:contactPoint.r0".to_string()),
        ]);
        let out = agent_rows(field, &draft, Rows { posted: Some(&opened), ..rows }).into_string();
        assert!(out.contains(r#"role="combobox""#), "{out}");
        assert!(out.contains(r#"name="contactPoint.r0.q""#), "{out}");
        assert!(
            out.contains(r#"<label class="field-label" for="contactPoint.r0-q">Person or organisation 1"#),
            "{out}"
        );
        assert!(out.contains(hidden), "an abandoned change keeps the stored choice: {out}");
        assert!(out.contains(" autofocus"), "{out}");
        assert!(out.contains(FOCUS_AUTOFOCUS), "{out}");
        assert!(!out.contains(r#"value="change-agent:contactPoint.r0""#), "{out}");

        // Any later render of that body keeps the search open, for as long as it posts its query.
        let typed = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "organization-008".to_string()),
            ("contactPoint.r0.q".to_string(), "Bern".to_string()),
        ]);
        let out = agent_rows(field, &draft, Rows { posted: Some(&typed), ..rows }).into_string();
        assert!(out.contains(r#"value="Bern""#), "{out}");
        assert!(out.contains(hidden), "{out}");
        assert!(!out.contains(" autofocus"), "only a freshly opened search takes focus: {out}");
        assert!(!out.contains("Keep "), "{out}");
        // A render of the stored draft is the card again.
        let stored = agent_rows(field, &draft, rows).into_string();
        assert!(!stored.contains(r#"role="combobox" aria-autocomplete="list" aria-controls="contactPoint.r0-results""#));
    }

    #[test]
    fn an_empty_slot_types_ahead_against_the_section_s_search_only_when_it_has_one() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!([]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let path = "/projects/0801d/sections/contributors/agents";
        let rows = Rows {
            action: "/x",
            adding: Some("contactPoint"),
            agents: Some(&scope),
            search_path: Some(path),
            ..Rows::default()
        };

        let with = agent_rows(field, &draft, rows).into_string();
        assert!(
            with.contains(&format!(
                r#"data-on:input__debounce.300ms="@get('{path}?for=contactPoint.r0&amp;q='"#
            )),
            "{with}"
        );
        assert!(with.contains(r#"aria-controls="contactPoint.r0-results""#), "{with}");
        assert!(with.contains(r#"id="contactPoint.r0-results""#), "{with}");
        // The append search below the list asks about its own picker.
        assert!(
            with.contains(&format!(r#"@get('{path}?for=contactPoint.new&amp;q='"#)),
            "{with}"
        );

        let without = agent_rows(field, &draft, Rows { search_path: None, ..rows }).into_string();
        assert!(!without.contains("data-on:input"), "{without}");
        assert!(without.contains(r#"name="contactPoint.r0.q""#), "{without}");
        assert!(
            without.contains(r#"value="find-agent""#),
            "the Search button still runs it: {without}"
        );
    }

    #[test]
    fn a_no_javascript_search_shows_its_matches_in_the_results_container() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!([]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), String::new()),
            ("contactPoint.r0.q".to_string(), "person-001".to_string()),
            ("intent".to_string(), "find-agent".to_string()),
        ]);
        let rows = Rows {
            action: "/x",
            posted: Some(&body),
            submitted: Some(&body),
            agents: Some(&scope),
            ..Rows::default()
        };

        let out = agent_rows(field, &draft, rows).into_string();
        assert_eq!(
            out.matches(r#"id="contactPoint.r0-results""#).count(),
            1,
            "one container: {out}"
        );
        let container = out.find(r#"id="contactPoint.r0-results""#).expect("container");
        let option = out.find(r#"value="choose-agent:contactPoint.r0:person-001""#).expect("option");
        assert!(container < option, "{out}");
        assert!(out.contains(r#"aria-expanded="true""#), "{out}");
    }

    #[test]
    fn contributors_render_one_append_search_and_no_per_row_search_or_add_button() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([
                { "contributor": "person-001", "contributorType": ["Author"] },
                { "contributor": "organization-008", "contributorType": ["Editor"] },
            ]),
        );
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let out = attribution_rows(field, &draft, rows).into_string();
        assert_eq!(out.matches(r#"name="attributions.new.q""#).count(), 1, "{out}");
        assert!(!out.contains(r#"name="attributions.r0.contributor.q""#), "{out}");
        assert!(!out.contains(r#"name="attributions.r1.contributor.q""#), "{out}");
        assert_eq!(out.matches(r#"role="combobox""#).count(), 1, "{out}");
        assert!(out.contains("Add a person or organisation"), "{out}");
        assert!(out.contains("Search by name or affiliation."), "{out}");
        assert!(out.contains(r#"value="change-agent:attributions.r0.contributor""#), "{out}");
        assert!(out.contains(r#"value="change-agent:attributions.r1.contributor""#), "{out}");
        // Each row keeps its Remove, and nothing posts to the list's add URL.
        assert!(out.contains(r#"formaction="/x/attributions/r0/remove""#), "{out}");
        assert!(out.contains(r#"formaction="/x/attributions/r1/remove""#), "{out}");
        assert!(!out.contains("/x/attributions/add"), "{out}");
        assert!(!out.contains("repeatable-add"), "{out}");

        let mut contacts = published_draft();
        contacts.set("contactPoint", serde_json::json!(["organization-008"]));
        let field = editor_core::form::registry::field("contactPoint").expect("contactPoint is a known field");
        let out = agent_rows(field, &contacts, rows).into_string();
        assert!(!out.contains("/x/contactPoint/add"), "{out}");
        assert!(!out.contains("repeatable-add"), "{out}");
        assert_eq!(out.matches(r#"name="contactPoint.new.q""#).count(), 1, "{out}");
    }

    #[test]
    fn an_empty_list_says_how_to_fill_it_and_the_append_search_does_not_offer_what_a_row_holds() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut empty = published_draft();
        empty.set("attributions", serde_json::json!([]));
        empty.set("contactPoint", serde_json::json!([]));
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let attributions = editor_core::form::registry::field("attributions").expect("a known field");
        let out = attribution_rows(attributions, &empty, rows).into_string();
        assert!(
            out.contains("No contributors yet. Search below to add the people and organisations involved."),
            "{out}"
        );
        let contact = editor_core::form::registry::field("contactPoint").expect("a known field");
        let out = agent_rows(contact, &empty, rows).into_string();
        assert!(out.contains("No contact point yet. Search below to add one."), "{out}");

        // Typing in the box of a list holding person-001 never offers person-001 again.
        let mut held = published_draft();
        held.set("contactPoint", serde_json::json!(["person-001"]));
        let body = editor_core::form::FormBody::from_pairs(vec![
            ("contactPoint.row".to_string(), "r0".to_string()),
            ("contactPoint.r0".to_string(), "person-001".to_string()),
            ("contactPoint.new.q".to_string(), "person-00".to_string()),
            ("intent".to_string(), "find-agent".to_string()),
        ]);
        let out =
            agent_rows(contact, &held, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string();
        assert!(out.contains(r#"value="add-agent:contactPoint:person-002""#), "{out}");
        assert!(!out.contains(r#"value="add-agent:contactPoint:person-001""#), "{out}");
        assert!(out.contains(r#"value="person-00""#), "the query stays in the box: {out}");
    }

    #[test]
    fn roles_render_as_chips_with_remove_buttons_and_a_disclosure_over_the_checkboxes() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([
                { "contributor": "person-001", "contributorType": ["Author", "Editor"] },
                { "contributor": "person-002", "contributorType": [] },
            ]),
        );
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let out = attribution_rows(field, &draft, rows).into_string();
        assert!(out.contains(r#"<ul aria-label="Roles of "#), "{out}");
        assert!(
            out.contains(r#"name="intent" value="remove-role:attributions.r0:Author""#),
            "{out}"
        );
        assert!(
            out.contains(r#"name="intent" value="remove-role:attributions.r0:Editor""#),
            "{out}"
        );
        assert!(!out.contains("remove-role:attributions.r1"), "no chips without a role: {out}");
        assert!(out.contains(r#"aria-label="Remove role Author from "#), "{out}");
        assert!(out.contains(r#"aria-label="Remove role Editor from "#), "{out}");
        // The first row holds roles, so its disclosure is closed and offers to change them; the
        // second holds none, so it is open and offers to choose. Both keep the depositor's own
        // toggle across a morph.
        assert!(
            out.contains(r#"<details data-preserve-attr="open"><summary class="cursor-pointer text-sm">Change roles"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<details open><summary class="cursor-pointer text-sm""#),
            "{out}"
        );
        // The checkbox group is the only thing that posts a role, and the chips post nothing.
        let chips = &out[out.find(r#"<ul aria-label="Roles of "#).expect("chips")..];
        let chips = &chips[..chips.find("</ul>").expect("end of chips")];
        assert!(!chips.contains("<input"), "{chips}");
        assert!(!chips.contains(r#"name="attributions.r0.role""#), "{chips}");
        assert!(out.contains(r#"value="Author" checked"#), "{out}");
        assert!(out.contains(ADD_ROLE_LABEL), "{out}");
    }

    #[test]
    fn a_contributor_with_no_role_is_flagged_on_a_get_and_a_refused_submit_but_not_on_the_choice_that_made_it() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": [] }]),
        );
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };
        let flagged = |out: &str| out.contains("Choose at least one role") && out.contains("border-warning-600");

        assert!(flagged(&attribution_rows(field, &draft, rows).into_string()));

        let body_with = |intent: &str| {
            editor_core::form::FormBody::from_pairs(vec![
                ("attributions.row".to_string(), "r0".to_string()),
                ("attributions.r0.contributor".to_string(), "person-001".to_string()),
                ("attributions.r0.role".to_string(), String::new()),
                ("intent".to_string(), intent.to_string()),
            ])
        };
        let render = |intent: &str| {
            let body = body_with(intent);
            attribution_rows(field, &draft, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string()
        };
        assert!(flagged(&render("submit")), "a refused submit names what is missing");
        let chosen = render("choose-agent:attributions.r0.contributor:person-001");
        assert!(
            !chosen.contains("Choose at least one role") && !chosen.contains("border-warning-600"),
            "{chosen}"
        );
        let added = render("add-agent:attributions:person-001");
        assert!(!added.contains("Choose at least one role"), "{added}");

        // A role held clears it.
        let mut held = draft.clone();
        held.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let out = attribution_rows(field, &held, rows).into_string();
        assert!(
            !out.contains("Choose at least one role") && !out.contains("border-warning-600"),
            "{out}"
        );
    }

    #[test]
    fn the_append_box_gets_a_new_id_after_an_add_and_its_results_container_keeps_its_own() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "attributions",
            serde_json::json!([{ "contributor": "person-001", "contributorType": ["Author"] }]),
        );
        let field = editor_core::form::registry::field("attributions").expect("attributions is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let before = attribution_rows(field, &draft, rows).into_string();
        assert!(before.contains(r#"id="attributions.new-q-add-1""#), "{before}");
        assert!(!before.contains("document.activeElement"), "a GET takes no focus: {before}");

        // The agent is already held, so the row count does not move: only the token can rename it.
        let body = editor_core::form::FormBody::from_pairs(apply_picker_intents(
            pairs(&[
                ("attributions.row", "r0"),
                ("attributions.r0.contributor", "person-001"),
                ("attributions.r0.role", "Author"),
                ("attributions.new.q", "Rita"),
                ("intent", "add-agent:attributions:person-001"),
            ]),
            shape_of,
        ));
        let after =
            attribution_rows(field, &draft, Rows { posted: Some(&body), submitted: Some(&body), ..rows }).into_string();
        assert!(!after.contains(r#"id="attributions.new-q-add-1""#), "{after}");
        assert!(after.contains(r#"id="attributions.new-q-add-1-"#), "{after}");
        assert!(after.contains(r#"value="""#), "the box comes back empty: {after}");
        assert!(after.contains("document.activeElement"), "focus stays on the box: {after}");
        // An autosave landing mid-typing posts the text and no intent: the box keeps its node.
        let typing = editor_core::form::FormBody::from_pairs(pairs(&[
            ("attributions.row", "r0"),
            ("attributions.r0.contributor", "person-001"),
            ("attributions.r0.role", "Author"),
            ("attributions.r0.role", ""),
            ("attributions.new.q", "Rit"),
        ]));
        let autosaved =
            attribution_rows(field, &draft, Rows { posted: Some(&typing), submitted: Some(&typing), ..rows })
                .into_string();
        assert!(autosaved.contains(r#"id="attributions.new-q-add-1""#), "{autosaved}");
        assert!(!autosaved.contains("document.activeElement"), "{autosaved}");
        for out in [&before, &after] {
            assert!(out.contains(r#"id="attributions.new-results""#), "{out}");
            assert!(out.contains(r#"aria-controls="attributions.new-results""#), "{out}");
        }
    }

    #[test]
    fn a_funder_is_a_card_and_a_search_with_no_append_search() {
        let agents = agent_corpus();
        let scope = editor_core::agents::AgentScope::published_only(&agents);
        let mut draft = published_draft();
        draft.set(
            "funding",
            serde_json::json!([{ "funders": ["organization-002"], "number": "1" }]),
        );
        let field = editor_core::form::registry::field("funding").expect("funding is a known field");
        let rows = Rows { action: "/x", agents: Some(&scope), ..Rows::default() };

        let out = funding(field, &draft, rows).into_string();
        assert!(out.contains(r#"value="change-agent:funding.r0.funder-0""#), "{out}");
        assert!(
            out.contains(r#"<input type="hidden" name="funding.r0.funder" value="organization-002">"#),
            "{out}"
        );
        assert!(
            out.contains(r#"name="funding.r0.funder-1.q""#),
            "the blank trailing funder searches: {out}"
        );
        assert!(!out.contains(".new.q"), "{out}");
    }
}
