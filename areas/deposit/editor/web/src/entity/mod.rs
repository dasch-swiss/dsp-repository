//! The entity form: one person or organisation proposal, rendered inside the
//! same page shell the project form uses.
//!
//! Mirrors `pages::section`: [`page`] is the whole page, [`region`] the part a
//! save replaces, both from one [`EntityView`] so the plain and enhanced paths
//! cannot drift. No rail, since an entity is one page. Every field control is
//! [`crate::form::widgets`]'s own, so this form behaves like the project form's
//! fields of the same shape; the one new shape is `address_group`, a flat group
//! of scalar members.

pub mod origin;

use deposit_shell::impact_card;
use editor_core::agents::AgentScope;
use editor_core::countries::{takes_canton, CANTONS, COUNTRIES, SWITZERLAND};
use editor_core::draft::ProjectDraft;
use editor_core::form::format::{self, Finding};
use editor_core::form::registry::{Audience, Field, Obligation};
use editor_core::form::{FormBody, Shape};
use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation};
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::badge::{badge, BadgeVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::select::select;
use mosaic_tiles::text_field::{text_field, InputType};
use origin::Origin;
use serde_json::Value;
use shared_datastar as datastar;

use crate::form::widgets::{
    agent_rows, formatted_text, multilingual, reference_rows, stated, string_rows, Mode, ReferenceWording, Rows,
};
use crate::form::INTENT;

/// The id the enhanced path's patch targets, and the anchor a save returns to
/// — the [`pages::section`](crate::pages::section) counterpart for this page.
pub const REGION_ID: &str = "entity-form";

/// Store the draft (Save), matching [`crate::pages::section::SAVE`]'s wire word.
pub const SAVE: &str = "save";

/// Show the discard confirmation, which posts [`DISCARD`]. Two-step because a
/// discarded proposal's id is never handed out again; see
/// [`editor_core::proposals::next_entity_id`].
pub const DISCARD_CONFIRM: &str = "discard-confirm";

/// Withdraw the proposal, once confirmed.
pub const DISCARD: &str = "discard";

/// The authority sources a `sameAs` reference may come from, on both a person
/// and an organisation: the committed corpus carries `ORCID`, `GND` and `URL`.
pub const SAME_AS_TYPES: &[&str] = &["ORCID", "GND", "URL"];

/// The wording on a `sameAs` row's controls: what an authority record is for.
pub(crate) const SAME_AS_WORDING: ReferenceWording = ReferenceWording {
    source_hint: "ORCID for an ORCID iD, GND for the German National Library's authority file, URL for any other web \
                  address.",
    link_lead: Some("The whole link to the record, not just its ID."),
    link_example: Some("https://d-nb.info/gnd/118540238"),
    shown_as_hint: "A name for this record, such as the person's or organisation's name.",
};

/// A person's fields, in display order. Exactly `Person`'s members
/// (`shared/metadata/src/person.rs`) and no others.
const PERSON_FIELDS: &[Field] = &[
    Field {
        id: "givenNames",
        label: "Given names",
        hint: Some("The person's given name(s), in the order they should be shown."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::StringRows),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "familyNames",
        label: "Family names",
        hint: Some("The person's family name(s)."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::StringRows),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "jobTitles",
        label: "Job titles",
        hint: Some(
            "The person's occupation, such as \"Senior lecturer\" or \"Archivist\". A project role such as \
             \"Project Leader\" belongs in the project's own contributors, not here — record it there instead.",
        ),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::StringRows),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "affiliations",
        label: "Affiliations",
        hint: Some("Organisations this person is affiliated with."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::AgentRows),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "sameAs",
        label: "Authority records",
        hint: Some("Identifiers for this person elsewhere, such as an ORCID iD."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::ReferenceRows(SAME_AS_TYPES)),
        format: format::REFERENCE_LINK,
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "email",
        label: "Email",
        hint: Some("A contact address for this person, if there is one to record."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        // type="text", not type="email": a payload may hold a value that does not
        // validate, and a browser refusing a half-typed address would block the save.
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
];

/// An organisation's fields, in display order. Exactly `Organization`'s members
/// (`shared/metadata/src/organization.rs`) except `address`, a flat
/// group of scalars that `address_group` renders and applies.
const ORGANIZATION_FIELDS: &[Field] = &[
    Field {
        id: "name",
        label: "Name",
        hint: Some("The organisation's full name."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "url",
        label: "URL",
        hint: Some("The organisation's website."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
        format: format::LINK,
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "alternativeName",
        label: "Alternative name",
        hint: Some("Other names this organisation is known by, one per language."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Multilingual),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "sameAs",
        label: "Authority records",
        hint: Some("Identifiers for this organisation elsewhere."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::ReferenceRows(SAME_AS_TYPES)),
        format: format::REFERENCE_LINK,
        vocabulary: None,
        max_chars: None,
    },
    Field {
        id: "email",
        label: "Email",
        hint: Some("A contact address for this organisation, if there is one to record."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
        format: &[],
        vocabulary: None,
        max_chars: None,
    },
];

/// One of `address`'s flat members: what [`address_group`] renders for it.
pub struct AddressMember {
    pub name: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    /// Text members only; a choice carries its own prompt.
    pub placeholder: Option<&'static str>,
    pub control: MemberControl,
    /// Wrapped in the `data-show` that follows the Country select, but always in the HTML.
    pub swiss_only: bool,
}

pub enum MemberControl {
    Text,
    Choice { options: &'static [&'static str], prompt: &'static str },
}

/// The Country member's name: the control [`address_group`]'s `data-on:change` listens for.
const COUNTRY: &str = "country";

/// `address`'s six flat members, in display order.
/// The first four are the ones [`editor_core::proposals::check_organization`] asks
/// for together or not at all; `canton` and `additional` are always optional, on the contract as
/// well as here.
pub const ADDRESS_MEMBERS: &[AddressMember] = &[
    AddressMember {
        name: "street",
        label: "Street",
        hint: None,
        placeholder: None,
        control: MemberControl::Text,
        swiss_only: false,
    },
    AddressMember {
        name: "postalCode",
        label: "Postal code",
        hint: None,
        placeholder: None,
        control: MemberControl::Text,
        swiss_only: false,
    },
    AddressMember {
        name: "locality",
        label: "Locality",
        hint: None,
        placeholder: None,
        control: MemberControl::Text,
        swiss_only: false,
    },
    AddressMember {
        name: COUNTRY,
        label: "Country",
        hint: Some("The country's English name."),
        placeholder: None,
        control: MemberControl::Choice { options: COUNTRIES, prompt: "Choose a country" },
        swiss_only: false,
    },
    AddressMember {
        name: "canton",
        label: "Canton",
        hint: Some("Switzerland only: cleared on save when the country is not Switzerland."),
        placeholder: None,
        control: MemberControl::Choice { options: CANTONS, prompt: "Choose a canton" },
        swiss_only: true,
    },
    AddressMember {
        name: "additional",
        label: "Additional address line",
        hint: Some(
            "A line after the street, such as c/o, the building or the floor. Leave it blank if the street says it all.",
        ),
        placeholder: Some("c/o Department of History"),
        control: MemberControl::Text,
        swiss_only: false,
    },
];

/// The rule is about the group, so it is stated once rather than on each control.
const ADDRESS_HINT: &str =
    "Street, postal code, locality and country are needed together: fill in all four, or leave the whole address \
     blank. Canton and the additional line are always optional.";

/// The id [`ADDRESS_HINT`]'s paragraph carries. Fixed rather than generated: the
/// group renders at most once per form, so it cannot duplicate.
const ADDRESS_HINT_ID: &str = "address-hint";

/// This kind's fields, in display order. The entity form's row actions (`handlers/entities.rs`)
/// check that a posted field id is one this kind renders.
#[must_use]
pub fn fields_for(kind: ProposalKind) -> &'static [Field] {
    match kind {
        ProposalKind::Person => PERSON_FIELDS,
        ProposalKind::Organization => ORGANIZATION_FIELDS,
    }
}

/// The address group's legend, which has no [`Field`] of its own.
const ADDRESS_LABEL: &str = "Address";

/// What a proposal finding on `member` (an [`editor_core::proposals::EntityFinding::field`]) is
/// named by: the form's label for it, or `None` for a member this kind's form does not show.
#[must_use]
pub fn finding_label(kind: ProposalKind, member: &str) -> Option<&'static str> {
    if member == "address" {
        return (kind == ProposalKind::Organization).then_some(ADDRESS_LABEL);
    }
    fields_for(kind)
        .iter()
        .find(|field| field.id == member)
        .map(|field| field.label)
}

/// Why the form is not offered: a proposal that is no longer
/// [`EntityProposal::is_live`]. Only the two terminal statuses reach this, since
/// a submitted proposal is still the depositor's to finish. A named type rather
/// than a `bool` because the two readings say different things about what
/// happens next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// RDU accepted it; it is on its way into the repository as a file.
    Accepted,
    /// RDU rejected it, or the depositor discarded it.
    Terminal,
}

impl Over {
    const fn message(self) -> &'static str {
        match self {
            Self::Accepted => {
                "RDU accepted this proposal. It is on its way into the repository, so nothing here can change any \
                 more."
            }
            Self::Terminal => {
                "This proposal is no longer open — it was rejected or discarded — so nothing here can change any \
                 more."
            }
        }
    }

    const fn heading(self) -> &'static str {
        match self {
            Self::Accepted => "Accepted",
            Self::Terminal => "Closed",
        }
    }
}

/// What the `POST` that led to this rendering did. A discard has no notice here: it returns to the
/// section the proposal was opened from, which says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice<'a> {
    Saved,
    /// The write was refused, and why.
    Refused(&'a str),
}

/// Short form of [`Notice::Saved`], shared between its alert text in [`status`]
/// and [`Notice::title_prefix`] so a wording change cannot update one and not
/// the other.
const SAVED_TITLE: &str = "Saved";
/// The prefix for a write that did not land, [`Notice::Refused`].
const NOT_SAVED_TITLE: &str = "Not saved";

impl Notice<'_> {
    /// The prefix a full page load puts in front of the document `<title>`, matching
    /// [`crate::pages::section::Notice::title_prefix`].
    pub fn title_prefix(&self) -> Option<&'static str> {
        match self {
            Notice::Saved => Some(SAVED_TITLE),
            Notice::Refused(_) => Some(NOT_SAVED_TITLE),
        }
    }
}

/// Which confirmation is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    Discard,
}

/// Everything one rendering of the entity form needs.
pub struct EntityView<'a> {
    pub shortcode: &'a str,
    pub proposal: &'a EntityProposal,
    /// `None` while this proposal [`EntityProposal::is_live`], `Some` naming
    /// why it is read-only otherwise.
    pub over: Option<Over>,
    /// The proposed entity, as the fields above read and write it.
    pub draft: &'a ProjectDraft,
    pub confirming: Option<Confirmation>,
    pub notice: Option<Notice<'a>>,
    /// The body that was posted, when this render is answering a `POST` — see
    /// [`crate::pages::section::SectionView::posted`] for why a repeatable
    /// field needs it and a scalar one does not.
    pub posted: Option<&'a FormBody>,
    pub adding_row: Option<&'a str>,
    pub agents: Option<&'a AgentScope<'a>>,
    /// Findings on the proposed entity, each keyed by the control it renders
    /// on, as the section form's are.
    pub format_findings: Vec<Finding>,
    /// Base URL a repeatable field's add and remove controls submit to —
    /// `/projects/{shortcode}/entities/{entity_id}/fields`.
    pub rows_action: String,
    /// The section this proposal was opened from, which "Back to the project" returns to and the
    /// form posts back so a save's redirect keeps it.
    pub origin: Option<Origin>,
}

impl EntityView<'_> {
    fn fields(&self) -> &'static [Field] {
        fields_for(self.proposal.kind)
    }

    fn mode(&self) -> Mode {
        if self.over.is_some() {
            Mode::ReadOnly
        } else {
            Mode::Editable
        }
    }

    /// `search_path` is the agent search a picker runs as its combobox is typed in
    /// ([`Self::agents_path`]); a read-only rendering has no picker to type in.
    fn rows<'a>(&'a self, search_path: Option<&'a str>) -> Rows<'a> {
        Rows {
            posted: self.posted,
            submitted: None,
            action: &self.rows_action,
            adding: self.adding_row,
            agents: self.agents,
            format_findings: &self.format_findings,
            // Never the propose controls: this route dispatches none of the propose
            // intents, so they would be dead controls; see Rows::propose.
            propose: false,
            search_path,
            vocabulary_path: None,
            focus: None,
            without_language: &[],
            label_hidden: false,
        }
    }

    fn action(&self) -> String {
        format!("/projects/{}/entities/{}", self.shortcode, self.proposal.entity_id)
    }

    /// The path of this proposal's type-ahead search, `GET {action}/agents`.
    fn agents_path(&self) -> String {
        format!("{}/agents", self.action())
    }
}

/// The whole page: heading, then the region a save replaces.
pub fn page(view: &EntityView<'_>) -> Markup {
    html! {
        div class="max-w-3xl py-8" { (heading(view)) (region(view)) }
    }
}

/// Which side of a `change` proposal supplies its name.
#[derive(Clone, Copy)]
pub(crate) enum Precedence {
    /// The published name wins: RDU's review surface passes this and shows the proposed one beside
    /// it.
    Published,
    /// The proposal's own name wins: depositor views pass this, since a depositor who saved a
    /// rename expects to see it.
    Proposed,
}

fn resolved_name(agents: Option<&AgentScope<'_>>, entity_id: &str, precedence: Precedence) -> Option<String> {
    let agents = agents?;
    let agent = match precedence {
        Precedence::Published => agents.get(entity_id),
        Precedence::Proposed => agents.get_proposed_first(entity_id),
    }?;
    usable_name(&agent.label, entity_id)
}

fn usable_name(label: &str, entity_id: &str) -> Option<String> {
    let name = label.trim();
    (!name.is_empty() && name != entity_id).then(|| name.to_string())
}

fn compose_label(kind: ProposalKind, entity_id: &str, name: Option<&str>, with_id: bool) -> String {
    let noun = kind.label().to_lowercase();
    match (name, with_id) {
        (Some(name), false) => format!("{name} ({noun})"),
        (Some(name), true) => format!("{name} ({noun}, {entity_id})"),
        (None, _) => format!("{} {entity_id}", kind.label()),
    }
}

/// The one owner of entity display text: what every view calls a person or organisation, so no
/// screen shows a bare id. Depositor views pass [`Precedence::Proposed`], RDU's review passes
/// [`Precedence::Published`].
///
/// A view listing several entities goes through [`entity_labels`], which tells equal names apart.
pub(crate) fn entity_label(
    kind: ProposalKind,
    entity_id: &str,
    agents: Option<&AgentScope<'_>>,
    precedence: Precedence,
) -> String {
    let name = resolved_name(agents, entity_id, precedence);
    compose_label(kind, entity_id, name.as_deref(), false)
}

/// [`entity_label`] for each of several entities shown together, in order. Where two would read
/// the same, those carry their id as well ("Ada Lovelace (person, person-042)"); the rest do not.
pub(crate) fn entity_labels<'a>(
    entities: impl IntoIterator<Item = (ProposalKind, &'a str)>,
    agents: Option<&AgentScope<'_>>,
    precedence: Precedence,
) -> Vec<String> {
    let named: Vec<_> = entities
        .into_iter()
        .map(|(kind, id)| (kind, id, resolved_name(agents, id, precedence)))
        .collect();
    let plain: Vec<String> = named
        .iter()
        .map(|(kind, id, name)| compose_label(*kind, id, name.as_deref(), false))
        .collect();
    named
        .iter()
        .zip(&plain)
        .map(|((kind, id, name), label)| {
            if plain.iter().filter(|other| *other == label).count() > 1 {
                compose_label(*kind, id, name.as_deref(), true)
            } else {
                label.clone()
            }
        })
        .collect()
}

fn heading(view: &EntityView<'_>) -> Markup {
    let label = entity_label(view.proposal.kind, &view.proposal.entity_id, view.agents, Precedence::Proposed);
    let back = match view.origin {
        Some(origin) => origin.section_url(view.shortcode, None, Some(&view.proposal.entity_id)),
        None => format!("/projects/{}", view.shortcode),
    };
    html! {
        div class="mb-6 flex flex-col gap-2" {
            p {
                a href=(back) class="underline" { "Back to the project" }
            }
            div class="flex flex-wrap items-center gap-3" {
                h1 class="font-display text-2xl" { (label) }
                ({
                    badge(kind_badge(view.proposal.kind, view.proposal.operation))
                        .variant(BadgeVariant::Secondary)
                })
            }
        }
    }
}

/// What is being proposed, beside the heading, which names only the entity.
const fn kind_badge(kind: ProposalKind, operation: ProposalOperation) -> &'static str {
    match (operation, kind) {
        (ProposalOperation::New, ProposalKind::Person) => "New person",
        (ProposalOperation::New, ProposalKind::Organization) => "New organisation",
        (ProposalOperation::Change, ProposalKind::Person) => "Change to a person",
        (ProposalOperation::Change, ProposalKind::Organization) => "Change to an organisation",
    }
}

/// The status region and the form — everything a save can change.
pub fn region(view: &EntityView<'_>) -> Markup {
    html! {
        section id=(REGION_ID) {
            (status(view))
            @match view.over {
                Some(over) => (locked(view, over))
                None => (form(view))
            }
        }
    }
}

/// `sticky` for the reason the section form's own status region gives: the controls that produce
/// a notice are below it, and the enhanced path patches in place without moving the scroll.
fn status(view: &EntityView<'_>) -> Markup {
    html! {
        div class="empty:hidden sticky top-0 z-10" aria-live="polite" {
            @match view.notice {
                Some(Notice::Saved) => {
                    (alert(format!("{SAVED_TITLE}.")).variant(AlertVariant::Success))
                }
                Some(Notice::Refused(message)) => { (alert(message).variant(AlertVariant::Warning)) }
                None => {}
            }
        }
    }
}

/// A proposal that is no longer [`EntityProposal::is_live`]: a value-only
/// rendering of every field, and the reason nothing here can be changed.
fn locked(view: &EntityView<'_>, over: Over) -> Markup {
    html! {
        div class="flex flex-col gap-6" {
            (alert(over.message()).variant(AlertVariant::Info).title(over.heading()))
            div class=(FIELDS_CARD) {
                @for field in view.fields() {
                    ({
                        entity_field_row(
                            field,
                            view.draft,
                            Mode::ReadOnly,
                            view.rows(None),
                        )
                    })
                }
                @if view.proposal.kind == ProposalKind::Organization { (address_group(view, true)) }
            }
        }
    }
}

/// The section form's card, without its heading.
const FIELDS_CARD: &str = "flex flex-col gap-6 rounded-md border border-neutral-200 bg-white p-4";

/// The proposal's fields, and the controls that save or discard it.
fn form(view: &EntityView<'_>) -> Markup {
    let action = view.action();
    let agents_path = view.agents_path();
    html! {
        // Posts the submitter's formAction: this form carries add and remove
        // controls, and posting the form's action would make each a plain save.
        form
            id="entity-form"
            method="post"
            action=(action)
            class="flex flex-col gap-6 mt-6"
            data-on:submit=(datastar::post(&action).submitter_first().form())
        {
            @if let Some(origin) = view.origin {
                input type="hidden" name=(origin::SECTION) value=(origin.section.id);
                @if let Some(field) = origin.field {
                    input type="hidden" name=(origin::FIELD) value=(field.id);
                }
            }
            div class=(FIELDS_CARD) {
                @for field in view.fields() {
                    ({
                        entity_field_row(
                            field,
                            view.draft,
                            view.mode(),
                            view.rows(Some(&agents_path)),
                        )
                    })
                }
                @if view.proposal.kind == ProposalKind::Organization { (address_group(view, false)) }
            }
            (controls(view))
        }
    }
}

/// One field's control, dispatched on its own [`Shape`] rather than on
/// [`crate::form::widgets::control`]'s per-id dispatch: that one is keyed on the
/// project's field ids, where `"url"` means a URL slot, and an organisation's
/// plain `url` member would silently take that branch.
fn entity_field_row(field: &Field, draft: &ProjectDraft, mode: Mode, rows: Rows<'_>) -> Markup {
    if mode != Mode::Editable {
        // stated renders whatever Value is stored, whichever shape the field has.
        return stated(field, draft, None, false);
    }
    match field.shape {
        Some(Shape::Text(_)) => scalar_text(field, draft, rows),
        Some(Shape::Multilingual) => multilingual(field, draft, 2),
        Some(Shape::StringRows) => string_rows(field, draft, rows),
        Some(Shape::AgentRows) => agent_rows(field, draft, rows),
        Some(Shape::ReferenceRows(types)) => reference_rows(field, draft, rows, types, &SAME_AS_WORDING),
        other => unreachable!("every entity field declares one of the shapes matched above, got {other:?}"),
    }
}

/// A plain scalar text control: `name`, `url`, `email`. Not
/// `crate::form::widgets::control`'s dispatch, for the reason
/// [`entity_field_row`] gives.
fn scalar_text(field: &Field, draft: &ProjectDraft, rows: Rows<'_>) -> Markup {
    let value = draft.get(field.id).and_then(Value::as_str).unwrap_or_default();
    // type="text": a payload may hold a value that does not validate, and a
    // browser refusing a half-typed one would block the save.
    formatted_text(field, value, rows)
}

/// `address`: a flat group of six scalar members, which is why it is not one of
/// [`Shape`]'s shapes. `read_only` renders values instead of controls.
fn address_group(view: &EntityView<'_>, read_only: bool) -> Markup {
    let member = |name: &str| -> String {
        let path = format!("address.{name}");
        view.draft.get(&path).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    // Canton is always in the HTML and only hidden client-side, so a depositor
    // without JavaScript can still fill it in, and the save can clear it.
    // `_swiss` is a local signal: it is never posted, the form body is. It holds a
    // server-computed boolean, never the stored country: Datastar compiles signal
    // values and expressions as JavaScript.
    let live = !read_only;
    let swiss_signal = live.then(|| takes_canton(&member(COUNTRY)).to_string());
    // The client-side form of `editor_core::countries::takes_canton`, which the save applies.
    let track_country = format!(
        "evt.target.name === 'address.{COUNTRY}' && ($_swiss = evt.target.value.trim().toLowerCase() === '{}')",
        SWITZERLAND.to_ascii_lowercase()
    );
    // aria-describedby on the fieldset: a sibling paragraph is not part of a control's accessible
    // description. mosaic_tiles' `group_shell` does the same but is pub(super).
    html! {
        fieldset class="field field-group" aria-describedby=(ADDRESS_HINT_ID) {
            legend class="field-label" { (ADDRESS_LABEL) }
            div class="flex flex-col gap-3"
                data-signals:_swiss=[swiss_signal.as_deref()]
                data-on:change=[live.then_some(track_country.as_str())]
            {
                @for m in ADDRESS_MEMBERS {
                    @let value = member(m.name);
                    @let control = || match &m.control {
                        MemberControl::Choice { options, prompt } => {
                            choice_select(m, options, prompt, &value)
                        }
                        MemberControl::Text => {
                            let mut field = text_field(
                                    format!("address.{}", m.name),
                                    m.label,
                                )
                                .input_type(InputType::Text)
                                .value(value.clone());
                            if let Some(hint) = m.hint {
                                field = field.hint(hint);
                            }
                            if let Some(placeholder) = m.placeholder {
                                field = field.placeholder(placeholder);
                            }
                            html! {
                                (field)
                            }
                        }
                    };
                    @if read_only {
                        div class="field" {
                            p class="field-label" { (m.label) }
                            ({
                                crate::form::widgets::value_markup(
                                    Some(&Value::String(value.clone())),
                                )
                            })
                        }
                    } @else if m.swiss_only {
                        div data-show="$_swiss" { (control()) }
                    } @else { (control()) }
                }
            }
            p class="field-hint" id=(ADDRESS_HINT_ID) { (ADDRESS_HINT) }
        }
    }
}

/// A single-choice select over a bundled list. A stored `value` outside the list is added as its
/// own option, so it keeps rendering and saving as stored instead of being cleared.
fn choice_select(member: &AddressMember, list: &'static [&'static str], placeholder: &str, value: &str) -> Markup {
    let stored_outside_list = (!value.trim().is_empty() && !list.contains(&value)).then_some(value);
    let mut control = select(format!("address.{}", member.name), member.label)
        .placeholder(placeholder)
        .selected(value);
    if let Some(hint) = member.hint {
        control = control.hint(hint);
    }
    if let Some(stored) = stored_outside_list {
        control = control.option(stored, stored);
    }
    html! {
        (control.options(list.iter().map(|name| (*name, *name))))
    }
}

/// Save, and Discard behind its confirmation — the two controls this entity form has, each a
/// named submit on this one form, like the section form's own.
fn controls(view: &EntityView<'_>) -> Markup {
    if view.confirming == Some(Confirmation::Discard) {
        return discard_confirmation(view);
    }
    html! {
        div class="flex flex-wrap items-center gap-4" {
            ({
                button("Save draft")
                    .button_type(ButtonType::Submit)
                    .name_value(INTENT, SAVE)
            })
            ({
                button("Discard this proposal")
                    .variant(ButtonVariant::Secondary)
                    .button_type(ButtonType::Submit)
                    .name_value(INTENT, DISCARD_CONFIRM)
            })
        }
    }
}

/// The question before discarding the proposal, in place of Save and Discard.
fn discard_confirmation(view: &EntityView<'_>) -> Markup {
    let label = entity_label(view.proposal.kind, &view.proposal.entity_id, view.agents, Precedence::Proposed);
    let panel = html! {
        p {
            "Discarding the proposal for "
            (label)
            " removes it. It cannot be undone. The record stays on file as a placeholder, so nothing published \
             while this proposal was live can end up naming a different person or organisation."
            @if view.proposal.operation == ProposalOperation::New {
                " It is also taken out of this project's contributors, contact point and funding."
            }
        }
        div class="mt-2 flex items-center gap-4" {
            ({
                button("Yes, discard this proposal")
                    .variant(ButtonVariant::Danger)
                    .button_type(ButtonType::Submit)
                    .name_value(INTENT, DISCARD)
            })
            (link("Keep it", view.action()))
        }
    };
    impact_card(panel)
}

#[cfg(test)]
mod tests {
    use editor_core::agents::Agents;
    use editor_core::proposals::{ProposalOperation, ProposalStatus};
    use shared_metadata::models::AuthorityFileReference;
    use shared_metadata::organization::Address;
    use shared_metadata::utils::Multilingual;
    use shared_metadata::{Organization, Person};

    use super::*;

    fn a_proposal(kind: ProposalKind, entity_id: &str, payload: &str) -> EntityProposal {
        EntityProposal {
            id: Default::default(),
            shortcode: "0801d".to_string(),
            entity_id: entity_id.to_string(),
            kind,
            operation: ProposalOperation::New,
            payload: payload.to_string(),
            status: ProposalStatus::Draft,
            decision: None,
            proposed_by: None,
            created_at: Default::default(),
            updated_at: Default::default(),
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    const ADA: &str = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;

    #[test]
    fn a_name_that_is_empty_blank_or_the_id_is_not_a_name() {
        assert_eq!(usable_name("", "person-042"), None);
        assert_eq!(usable_name("  \t", "person-042"), None);
        assert_eq!(usable_name("person-042", "person-042"), None);
        assert_eq!(usable_name("  Ada Lovelace ", "person-042").as_deref(), Some("Ada Lovelace"));
    }

    #[test]
    fn a_label_is_the_name_and_kind_else_kind_and_id() {
        let published = Agents::default();
        let proposals = [
            a_proposal(ProposalKind::Person, "person-042", ADA),
            a_proposal(
                ProposalKind::Organization,
                "organization-007",
                r#"{"name":"Acme","url":"https://acme.example/"}"#,
            ),
            // Resolves to the id itself, as a person with no names does.
            a_proposal(
                ProposalKind::Person,
                "person-043",
                r#"{"givenNames":[],"familyNames":[],"jobTitles":[]}"#,
            ),
            a_proposal(
                ProposalKind::Organization,
                "organization-008",
                r#"{"name":"   ","url":"https://x.example/"}"#,
            ),
        ];
        let scope = AgentScope::with_proposals(&published, &proposals);
        assert_eq!(
            entity_label(ProposalKind::Person, "person-042", Some(&scope), Precedence::Published),
            "Ada Lovelace (person)"
        );
        assert_eq!(
            entity_label(
                ProposalKind::Organization,
                "organization-007",
                Some(&scope),
                Precedence::Published
            ),
            "Acme (organisation)"
        );
        assert_eq!(
            entity_label(ProposalKind::Person, "person-043", Some(&scope), Precedence::Published),
            "Person person-043"
        );
        assert_eq!(
            entity_label(
                ProposalKind::Organization,
                "organization-008",
                Some(&scope),
                Precedence::Published
            ),
            "Organisation organization-008"
        );
        assert_eq!(
            entity_label(ProposalKind::Person, "person-999", Some(&scope), Precedence::Published),
            "Person person-999"
        );
        assert_eq!(
            entity_label(ProposalKind::Person, "person-042", None, Precedence::Published),
            "Person person-042"
        );
    }

    #[test]
    fn equal_labels_gain_their_id_and_distinct_ones_do_not() {
        let published = Agents::default();
        let proposals = [
            a_proposal(ProposalKind::Person, "person-042", ADA),
            a_proposal(ProposalKind::Person, "person-043", ADA),
            a_proposal(
                ProposalKind::Organization,
                "organization-007",
                r#"{"name":"Acme","url":"https://acme.example/"}"#,
            ),
        ];
        let scope = AgentScope::with_proposals(&published, &proposals);
        let labels = entity_labels(
            proposals.iter().map(|proposal| (proposal.kind, proposal.entity_id.as_str())),
            Some(&scope),
            Precedence::Published,
        );
        assert_eq!(
            labels,
            [
                "Ada Lovelace (person, person-042)",
                "Ada Lovelace (person, person-043)",
                "Acme (organisation)"
            ]
        );
    }

    /// Every member of the contract type, read off a serialized instance so there
    /// is no second list to forget.
    fn members(value: &serde_json::Value) -> Vec<String> {
        let mut names: Vec<String> = value
            .as_object()
            .expect("a contract entity serializes as an object")
            .keys()
            .filter(|name| *name != "id")
            .cloned()
            .collect();
        names.sort();
        names
    }

    fn declared(fields: &[Field]) -> Vec<String> {
        let mut ids: Vec<String> = fields.iter().map(|field| field.id.to_string()).collect();
        ids.sort();
        ids
    }

    /// The form's field list must cover every member of `Person`, and no others.
    /// Add a member to `Person` and nothing fails to compile: the field list, the
    /// applier and `check_person` all keep silently ignoring it. `id` is excluded
    /// on both sides: it lives in `EntityProposal::entity_id`.
    #[test]
    fn the_person_field_list_covers_every_contract_member() {
        let person = Person {
            id: "person-417".to_string(),
            given_names: vec!["Ada".to_string()],
            family_names: vec!["Lovelace".to_string()],
            job_titles: vec![],
            affiliations: vec!["organization-001".to_string()],
            same_as: vec![AuthorityFileReference {
                type_: "ORCID".to_string(),
                url: "https://orcid.org/0000".to_string(),
                text: None,
            }],
            email: Some("a@x.test".to_string()),
        };
        let serialized = serde_json::to_value(&person).expect("a person serializes");
        assert_eq!(declared(PERSON_FIELDS), members(&serialized));
    }

    /// The same for `Organization`, with `address` standing for its own six members.
    #[test]
    fn the_organisation_field_list_covers_every_contract_member() {
        let organization = Organization {
            id: "organization-143".to_string(),
            name: "A University".to_string(),
            same_as: vec![],
            url: "https://example.test/".to_string(),
            address: Some(Address {
                street: "A Street".to_string(),
                postal_code: "1000".to_string(),
                locality: "A Town".to_string(),
                country: "A Country".to_string(),
                canton: None,
                additional: None,
            }),
            email: Some("a@x.test".to_string()),
            alternative_name: Some(Multilingual::from([("en".to_string(), "AU".to_string())])),
        };
        let serialized = serde_json::to_value(&organization).expect("an organization serializes");

        // address is deliberately not a Field; named here rather than filtered out
        // so the test still fails if the group is ever dropped.
        let mut covered = declared(ORGANIZATION_FIELDS);
        covered.push("address".to_string());
        covered.sort();
        assert_eq!(covered, members(&serialized));
    }

    /// `address` is rendered as six flat controls, so the group has to cover `Address`'s own
    /// members — a new one there would otherwise be as invisible as a new one on `Organization`.
    #[test]
    fn the_address_group_covers_every_address_member() {
        let address = Address {
            street: "A Street".to_string(),
            postal_code: "1000".to_string(),
            locality: "A Town".to_string(),
            country: "A Country".to_string(),
            canton: Some("A Canton".to_string()),
            additional: Some("An Extra Line".to_string()),
        };
        let serialized = serde_json::to_value(&address).expect("an address serializes");
        let mut declared: Vec<String> = ADDRESS_MEMBERS.iter().map(|member| member.name.to_string()).collect();
        declared.sort();
        assert_eq!(declared, members(&serialized));
    }

    fn person_proposal() -> EntityProposal {
        let now = chrono::Utc::now();
        EntityProposal {
            id: uuid::Uuid::nil(),
            shortcode: "0801d".to_string(),
            entity_id: "person-001".to_string(),
            kind: ProposalKind::Person,
            operation: editor_core::proposals::ProposalOperation::New,
            payload: "{}".to_string(),
            status: editor_core::proposals::ProposalStatus::Draft,
            proposed_by: None,
            created_at: now,
            updated_at: now,
            decision: None,
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    fn view_of<'a>(proposal: &'a EntityProposal, draft: &'a ProjectDraft, over: Option<Over>) -> EntityView<'a> {
        EntityView {
            shortcode: "0801d",
            proposal,
            over,
            draft,
            confirming: None,
            notice: None,
            posted: None,
            adding_row: None,
            agents: None,
            format_findings: Vec::new(),
            rows_action: "/projects/0801d/entities/person-001/fields".to_string(),
            origin: None,
        }
    }

    fn organization_address_html(address: Value, read_only: bool) -> String {
        let mut proposal = person_proposal();
        proposal.kind = ProposalKind::Organization;
        proposal.entity_id = "organization-900".to_string();
        let mut draft = ProjectDraft::default();
        draft.set("address", address);
        address_group(&view_of(&proposal, &draft, None), read_only).into_string()
    }

    /// The `<select …>…</select>` named `name`, as text.
    fn select_named<'a>(out: &'a str, name: &str) -> &'a str {
        let start = out
            .find(&format!(r#"<select class="field-input field-select" id="{name}""#))
            .expect("the select");
        &out[start..start + out[start..].find("</select>").expect("a closed select")]
    }

    #[test]
    fn a_stored_country_outside_the_list_is_the_selected_option() {
        let out = organization_address_html(serde_json::json!({ "country": "Atlantis" }), false);
        let country = select_named(&out, "address.country");
        assert!(
            country.contains(r#"<option value="Atlantis" selected>Atlantis</option>"#),
            "{country}"
        );
        assert_eq!(country.matches(" selected").count(), 1, "{country}");

        let listed = organization_address_html(serde_json::json!({ "country": "Tanzania" }), false);
        let country = select_named(&listed, "address.country");
        assert!(
            country.contains(r#"<option value="Tanzania" selected>Tanzania</option>"#),
            "{country}"
        );
        assert_eq!(country.matches(r#"value="Tanzania""#).count(), 1, "{country}");
        assert!(country.contains(r#"<option value="">Choose a country</option>"#), "{country}");

        let blank = organization_address_html(serde_json::json!({ "country": "  " }), false);
        assert!(!select_named(&blank, "address.country").contains(r#"value="  ""#), "{blank}");
    }

    #[test]
    fn a_stored_canton_outside_the_list_is_the_selected_option() {
        let out = organization_address_html(serde_json::json!({ "country": "Switzerland", "canton": "ZH" }), false);
        let canton = select_named(&out, "address.canton");
        assert!(canton.contains(r#"<option value="ZH" selected>ZH</option>"#), "{canton}");
        assert!(canton.contains(r#"<option value="Zurich">Zurich</option>"#), "{canton}");
    }

    /// Without JavaScript nothing hides Canton, so a depositor can still use it and the save can
    /// clear it; with JavaScript the `data-show` follows the Country select without a round trip.
    #[test]
    fn canton_is_rendered_for_a_non_swiss_country_and_hidden_by_datastar_alone() {
        let out = organization_address_html(serde_json::json!({ "country": "Germany" }), false);
        let canton = select_named(&out, "address.canton");
        assert!(canton.contains(r#"<option value="Bern">Bern</option>"#), "{canton}");
        assert!(out.contains(r#"data-show="$_swiss""#), "{out}");
        assert!(out.contains(r#"data-signals:_swiss="false""#), "{out}");
        assert!(out.contains("data-on:change="), "{out}");
        assert!(!out.contains("display:none") && !out.contains(" hidden"), "{out}");
    }

    #[test]
    fn the_swiss_signal_is_true_for_switzerland_however_it_is_stored() {
        let out = organization_address_html(serde_json::json!({ "country": " switzerland " }), false);
        assert!(out.contains(r#"data-signals:_swiss="true""#), "{out}");
    }

    /// Datastar compiles signal values and expressions as JavaScript, so a stored country must
    /// reach no `data-*` attribute: only its escaped text inside the select's option.
    #[test]
    fn a_stored_country_reaches_no_datastar_attribute() {
        let payload = "\u{1F595}JS_$`_DS\u{1F680}+alert(document.domain)+\u{1F595}JS_$'_DS\u{1F680}";
        let out = organization_address_html(serde_json::json!({ "country": payload }), false);
        for chunk in out.split(" data-").skip(1) {
            let value = chunk.split('"').nth(1).unwrap_or_default();
            assert!(!value.contains("alert("), "{chunk}");
        }
        assert!(
            out.contains("alert(document.domain)"),
            "the payload shows as option text: {out}"
        );
        assert!(out.contains("data-signals:_swiss=\"false\""), "{out}");
    }

    /// A renamed Country select would leave the Datastar listener comparing against a name nothing
    /// has.
    #[test]
    fn the_datastar_listener_names_the_country_select() {
        let out = organization_address_html(serde_json::json!({ "country": "Germany" }), false);
        let attribute = out.split(r#"data-on:change=""#).nth(1).expect("the listener");
        let expression = attribute.split('"').next().expect("a closed attribute");
        let listened_for = expression
            .split("evt.target.name === '")
            .nth(1)
            .and_then(|rest| rest.split('\'').next())
            .unwrap_or_else(|| panic!("no control name in {expression}"));
        let country = select_named(&out, listened_for);
        assert!(country.contains(&format!(r#"name="{listened_for}""#)), "{country}");
        assert!(country.contains("Choose a country"), "{country}");
    }

    #[test]
    fn the_read_only_address_has_no_controls() {
        let out = organization_address_html(serde_json::json!({ "country": "Germany" }), true);
        assert!(!out.contains("<select") && !out.contains("data-show"), "{out}");
    }

    #[test]
    fn every_address_member_renders_its_declared_control_and_hint() {
        let out = organization_address_html(serde_json::json!({}), false);
        for member in ADDRESS_MEMBERS {
            let id = format!("address.{}", member.name);
            let id_at = out.find(&format!(r#"id="{id}""#)).expect("the control");
            let tag_start = out[..id_at].rfind('<').expect("a tag start");
            let control_tag = &out[tag_start..][..out[tag_start..].find('>').expect("a tag end")];
            let expected = match member.control {
                MemberControl::Choice { .. } => "<select",
                MemberControl::Text => "<input",
            };
            assert!(control_tag.starts_with(expected), "{id}: {control_tag}");
            let Some(hint) = member.hint else { continue };
            let described_by = control_tag
                .split(r#"aria-describedby=""#)
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .unwrap_or_else(|| panic!("{id} has no aria-describedby: {control_tag}"));
            let hint_id = described_by.split(' ').next().expect("an id");
            let paragraph = &out[out.find(&format!(r#"id="{hint_id}""#)).expect("the hint element")..];
            let paragraph = &paragraph[..paragraph.find("</").expect("a closed element")];
            assert!(paragraph.contains(hint), "{id}: {paragraph}");
        }
        assert!(out.contains(r#"placeholder="c/o Department of History""#), "{out}");
    }

    /// Without the submitter's `formaction`, every add and remove control acts as a plain save
    /// once JavaScript is on.
    #[test]
    fn the_enhanced_path_posts_the_submitters_formaction_and_the_form_body() {
        let proposal = person_proposal();
        let draft = ProjectDraft::default();
        let view = view_of(&proposal, &draft, None);
        let out = form(&view).into_string();
        assert!(
            out.contains(
                r#"data-on:submit="@post(evt.submitter?.formAction || '/projects/0801d/entities/person-001', {contentType: 'form'})""#
            ),
            "{out}"
        );
    }

    #[test]
    fn a_format_finding_marks_the_organisations_url_and_same_as_link() {
        let now = chrono::Utc::now();
        let proposal = EntityProposal {
            id: uuid::Uuid::nil(),
            shortcode: "0801d".to_string(),
            entity_id: "organization-900".to_string(),
            kind: ProposalKind::Organization,
            operation: editor_core::proposals::ProposalOperation::New,
            payload: "{}".to_string(),
            status: editor_core::proposals::ProposalStatus::Draft,
            proposed_by: None,
            created_at: now,
            updated_at: now,
            decision: None,
            decided_by: None,
            decided_at: None,
            retired_at: None,
        };
        let mut draft = ProjectDraft::default();
        draft.set("name", serde_json::json!("DaSCH"));
        draft.set("url", serde_json::json!("www.dasch.swiss"));
        draft.set(
            "sameAs",
            serde_json::json!([
                { "type": "URL", "url": "https://www.dasch.swiss" },
                { "type": "URL", "url": "dasch.swiss" },
            ]),
        );
        let view = EntityView {
            shortcode: "0801d",
            proposal: &proposal,
            over: None,
            draft: &draft,
            confirming: None,
            notice: None,
            posted: None,
            adding_row: None,
            agents: None,
            format_findings: format::findings(ORGANIZATION_FIELDS.iter(), &draft, None, None),
            rows_action: "/projects/0801d/entities/organization-900/fields".to_string(),
            origin: None,
        };
        let out = form(&view).into_string();
        let marked: Vec<&str> = out
            .split("<input")
            .filter(|input| input.contains(r#"aria-invalid="true""#))
            .filter_map(|input| input.split(r#"name=""#).nth(1)?.split('"').next())
            .collect();
        assert_eq!(marked, ["url", "sameAs.r1.ref.url"], "{out}");
    }

    /// A payload holding `value` where `member` of `field` sits.
    fn holding(field: &Field, member: &format::Member, value: &str) -> serde_json::Value {
        let leaf = member
            .stored
            .iter()
            .rev()
            .fold(serde_json::json!(value), |inner, key| serde_json::json!({ *key: inner }));
        let mut payload = serde_json::json!({ "name": "DaSCH", "url": "https://www.dasch.swiss" });
        payload[field.id] = match field.shape {
            Some(shape) if shape.has_rows() => serde_json::json!([leaf]),
            _ => leaf,
        };
        payload
    }

    #[test]
    fn every_entity_format_member_is_refused_at_submit_where_the_form_marks_it() {
        // The form reads `Field::format`, the proposal gate its own rules; a member
        // declared on one and not the other would show an error submit lets through.
        for kind in [ProposalKind::Person, ProposalKind::Organization] {
            for field in fields_for(kind) {
                for member in field.format {
                    let refused = |value: &str| {
                        let payload = holding(field, member, value);
                        let findings = match kind {
                            ProposalKind::Person => editor_core::proposals::check_person(&payload),
                            ProposalKind::Organization => editor_core::proposals::check_organization(&payload, None),
                        };
                        findings.iter().any(|finding| finding.field == field.id)
                    };
                    assert!(refused("www.example.org"), "{kind:?} {}", field.id);
                    assert!(!refused("https://www.example.org"), "{kind:?} {}", field.id);
                }
            }
        }
    }

    #[test]
    fn every_entity_link_the_gate_refuses_is_one_the_form_marks() {
        // The other direction: a gate rule on a field with no format would refuse a
        // submission over a value its form shows no error for.
        for kind in [ProposalKind::Person, ProposalKind::Organization] {
            let mut payload = serde_json::json!({});
            for field in fields_for(kind) {
                payload[field.id] = match field.shape {
                    Some(Shape::ReferenceRows(_)) => serde_json::json!([{ "type": "URL", "url": "www.example.org" }]),
                    Some(Shape::Multilingual) => serde_json::json!({ "en": "www.example.org" }),
                    Some(shape) if shape.has_rows() => serde_json::json!(["www.example.org"]),
                    _ => serde_json::json!("www.example.org"),
                };
            }
            let findings = match kind {
                ProposalKind::Person => editor_core::proposals::check_person(&payload),
                ProposalKind::Organization => editor_core::proposals::check_organization(&payload, None),
            };
            for finding in findings.iter().filter(|finding| finding.message.contains("web address")) {
                let field = fields_for(kind).iter().find(|field| field.id == finding.field);
                assert!(field.is_some_and(|field| !field.format.is_empty()), "{kind:?}: {finding:?}");
            }
            assert!(
                findings.iter().any(|finding| finding.message.contains("web address")),
                "{kind:?}: {findings:?}"
            );
        }
    }

    #[test]
    fn every_member_a_proposal_finding_names_has_a_label() {
        // A payload failing every rule; a member without a label would reach the
        // depositor as its JSON name.
        let person = serde_json::json!({ "jobTitles": ["Project Leader"], "sameAs": [{ "type": "URL", "url": "x" }] });
        let organization = serde_json::json!({ "address": {}, "sameAs": [{ "type": "URL", "url": "x" }] });
        let mut findings: Vec<(ProposalKind, editor_core::proposals::EntityFinding)> = Vec::new();
        findings.extend(
            editor_core::proposals::check_person(&person)
                .into_iter()
                .map(|f| (ProposalKind::Person, f)),
        );
        findings.extend(
            editor_core::proposals::check_organization(&organization, None)
                .into_iter()
                .map(|f| (ProposalKind::Organization, f)),
        );
        let members: std::collections::BTreeSet<&str> = findings.iter().map(|(_, finding)| finding.field).collect();
        assert_eq!(
            members,
            [
                "address",
                "familyNames",
                "givenNames",
                "jobTitles",
                "name",
                "sameAs",
                "url"
            ]
            .into(),
            "every rule fired; a member added here needs a label in `finding_label`"
        );
        for (kind, finding) in &findings {
            assert!(finding_label(*kind, finding.field).is_some(), "{kind:?}: {finding:?}");
        }
    }

    #[test]
    fn a_finished_proposal_says_so_in_an_info_alert_and_not_a_warning() {
        let proposal = person_proposal();
        let draft = ProjectDraft::default();
        for over in [Over::Accepted, Over::Terminal] {
            let view = view_of(&proposal, &draft, Some(over));
            let out = locked(&view, over).into_string();
            assert!(out.contains("alert-info"), "{out}");
            assert!(!out.contains("alert-warning"), "{out}");
        }
    }
}
