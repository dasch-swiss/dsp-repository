//! The entity form: one person or organisation proposal (US-3), rendered
//! inside the same page shell the project form uses.
//!
//! Mirrors `pages::section` in shape — [`page`] is the whole page, [`region`]
//! is the part a save replaces, and both come from one [`EntityView`] so the
//! plain and enhanced paths cannot drift — but answers for one proposal
//! rather than one project section, and has no rail: a person or organisation
//! is one page, not several.
//!
//! Every field control here is [`crate::form::widgets`]'s own — `text_field`,
//! the agent rows, the reference rows, the multilingual group — reused rather
//! than rebuilt, which is what keeps this form behaving identically to the
//! project form's fields of the same shape (the same suggestion list, the
//! same "no person or organisation with this id" wording, the same
//! add/remove-by-round-trip protocol). The one shape genuinely new here is
//! [`address_group`]: a flat group of scalar members, which nothing in the
//! project form's registry needed before.

use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::{FormBody, Shape};
use editor_core::proposals::{EntityProposal, ProposalKind};
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::text_field::{text_field, InputType};
use serde_json::Value;

use crate::form::registry::{Audience, Field, Obligation};
use crate::form::widgets::{agent_rows, labelled, multilingual, reference_rows, stated, string_rows, Mode, Rows};
use crate::form::INTENT;

/// The id the enhanced path's patch targets, and the anchor a save returns to
/// — the [`pages::section`](crate::pages::section) counterpart for this page.
pub const REGION_ID: &str = "entity-form";

/// Store the draft (Save), matching [`crate::pages::section::SAVE`]'s wire word.
pub const SAVE: &str = "save";

/// Show the discard confirmation, which posts [`DISCARD`].
///
/// Its own two-step shape for the reason the project form's withdrawal and
/// discard are: the row's allocated id is never handed out again once
/// discarded (it survives as a tombstone — see
/// [`editor_core::proposals::next_entity_id`]'s module docs), so the control
/// asks first.
pub const DISCARD_CONFIRM: &str = "discard-confirm";

/// Withdraw the proposal, once confirmed.
pub const DISCARD: &str = "discard";

/// The authority sources a `sameAs` reference may come from, on both a person
/// and an organisation — the committed corpus carries `ORCID`, `GND` and
/// `URL` for a person and none yet for an organisation, so the same three are
/// offered to both rather than inventing a narrower list an organisation's
/// first `sameAs` would then have to be widened for.
pub const SAME_AS_TYPES: &[&str] = &["ORCID", "GND", "URL"];

/// A person's fields, in display order. Exactly `Person`'s members
/// (`modules/platform/metadata/src/person.rs`) and no others.
const PERSON_FIELDS: &[Field] = &[
    Field {
        id: "givenNames",
        label: "Given names",
        hint: Some("The person's given name(s), in the order they should be shown."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::StringRows),
    },
    Field {
        id: "familyNames",
        label: "Family names",
        hint: Some("The person's family name(s)."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::StringRows),
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
    },
    Field {
        id: "affiliations",
        label: "Affiliations",
        hint: Some("Organisations this person is affiliated with."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::AgentRows),
    },
    Field {
        id: "sameAs",
        label: "Authority records",
        hint: Some("Identifiers for this person elsewhere, such as an ORCID iD."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::ReferenceRows(SAME_AS_TYPES)),
    },
    Field {
        id: "email",
        label: "Email",
        hint: Some("A contact address for this person, if there is one to record."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        // `type="text"`, not `type="email"`, for the reason every scalar
        // control in the project form is: a payload may hold a value that
        // does not validate, and a browser refusing to submit a half-typed
        // address would block the save nothing here requires.
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
    },
];

/// An organisation's fields, in display order. Exactly `Organization`'s
/// members (`modules/platform/metadata/src/organization.rs`) except
/// `address`, which [`address_group`] renders and applies separately — it is
/// a flat group of scalars, not one of [`Shape`]'s repeatable or scalar
/// shapes.
const ORGANIZATION_FIELDS: &[Field] = &[
    Field {
        id: "name",
        label: "Name",
        hint: Some("The organisation's full name."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
    },
    Field {
        id: "url",
        label: "URL",
        hint: Some("The organisation's website."),
        obligation: Some(Obligation::Required),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
    },
    Field {
        id: "alternativeName",
        label: "Alternative name",
        hint: Some("Other names this organisation is known by, one per language."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Multilingual),
    },
    Field {
        id: "sameAs",
        label: "Authority records",
        hint: Some("Identifiers for this organisation elsewhere."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::ReferenceRows(SAME_AS_TYPES)),
    },
    Field {
        id: "email",
        label: "Email",
        hint: Some("A contact address for this organisation, if there is one to record."),
        obligation: Some(Obligation::Optional),
        display_only: false,
        audience: Audience::Everyone,
        shape: Some(Shape::Text(editor_core::form::WhenCleared::Drop)),
    },
];

/// `address`'s six flat members, in display order, paired with their label.
/// The first four are the ones REQ-3.4 and [`editor_core::proposals::check_organization`] ask
/// for together or not at all; `canton` and `additional` are always optional, on the contract as
/// well as here.
pub const ADDRESS_MEMBERS: &[(&str, &str)] = &[
    ("street", "Street"),
    ("postalCode", "Postal code"),
    ("locality", "Locality"),
    ("country", "Country"),
    ("canton", "Canton"),
    ("additional", "Additional address line"),
];

/// Why the four are asked for together, stated once rather than on each of the
/// four controls — the rule is about the group, not about any one member of
/// it.
const ADDRESS_HINT: &str =
    "Street, postal code, locality and country are needed together: fill in all four, or leave the whole address \
     blank. Canton and the additional line are always optional.";

/// The id [`ADDRESS_HINT`]'s paragraph carries, so the fieldset can point at it.
///
/// A fixed id rather than a generated one: the address group renders at most
/// once per entity form, so a document-wide constant cannot duplicate — and a
/// duplicate id would be the bug this is here to avoid rather than a risk it
/// takes.
const ADDRESS_HINT_ID: &str = "address-hint";

/// This kind's fields, in display order.
///
/// `pub`: `editor-server`'s row actions (add/remove a row) need it too, to
/// check that a posted field id is one this proposal's kind actually renders
/// before applying anything against it — the same check the section form
/// makes against its own registry.
#[must_use]
pub fn fields_for(kind: ProposalKind) -> &'static [Field] {
    match kind {
        ProposalKind::Person => PERSON_FIELDS,
        ProposalKind::Organization => ORGANIZATION_FIELDS,
    }
}

/// Why the form is not offered — a proposal that is no longer
/// [`EntityProposal::is_live`].
///
/// Only the two terminal statuses reach this: `Draft` and `Submitted` are both
/// live per [`EntityProposal::is_live`] (a submitted proposal is still the
/// depositor's to finish — its own status does not lock it the way a
/// submitted *project* draft locks the section form), so there is no
/// "submitted" reading here to confuse with these.
///
/// A named type rather than a `bool`, for the reason `pages::section::Locked`
/// is: the two readings say different things about what happens next, and a
/// depositor reading the wrong one waits for an event that is not coming.
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

/// What the `POST` that led to this rendering did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice<'a> {
    Saved,
    Discarded,
    /// The write was refused, and why.
    Refused(&'a str),
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
    /// Base URL a repeatable field's add and remove controls submit to —
    /// `/projects/{shortcode}/entities/{entity_id}/fields`.
    pub rows_action: String,
}

impl EntityView<'_> {
    fn fields(&self) -> &'static [Field] {
        fields_for(self.proposal.kind)
    }

    /// Whether this kind's own fields offer the shared agent suggestion list — true for a person
    /// (`affiliations`), false for an organisation, which has no field that names another agent.
    /// Gated the same way `SectionView::has_agent_field` gates it: the list carries every
    /// committed agent, so rendering it on a form with no field that needs it is pure weight.
    fn has_agent_field(&self) -> bool {
        self.fields()
            .iter()
            .any(|field| field.shape.is_some_and(Shape::offers_agent_suggestions))
    }

    fn mode(&self) -> Mode {
        if self.over.is_some() {
            Mode::ReadOnly
        } else {
            Mode::Editable
        }
    }

    fn rows(&self) -> Rows<'_> {
        Rows {
            posted: self.posted,
            action: &self.rows_action,
            adding: self.adding_row,
            agents: self.agents,
            // Never REQ-3.1/3.2's propose controls here: `affiliations` reuses
            // `agent_rows`, the same widget `contactPoint` uses, but this
            // route dispatches none of the three propose intents — see
            // `Rows::propose`'s own doc for why that would be a dead control.
            propose: false,
        }
    }

    fn action(&self) -> String {
        format!("/projects/{}/entities/{}", self.shortcode, self.proposal.entity_id)
    }
}

/// The whole page: heading, then the region a save replaces.
pub fn page(view: &EntityView<'_>) -> Markup {
    html! {
        div class="max-w-3xl py-8" { (heading(view)) (region(view)) }
    }
}

fn heading(view: &EntityView<'_>) -> Markup {
    html! {
        div class="mb-6" {
            h1 class="font-display text-2xl mb-1" {
                (view.proposal.kind.label())
                " "
                (view.proposal.entity_id)
            }
            p class="text-gray-600" {
                a href={ "/projects/" (view.shortcode) } class="underline" { "Back to the project" }
            }
        }
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

fn status(view: &EntityView<'_>) -> Markup {
    html! {
        div class="empty:hidden" aria-live="polite" {
            @match view.notice {
                Some(Notice::Saved) => { (alert("Saved.").variant(AlertVariant::Success)) }
                Some(Notice::Discarded) => {
                    ({
                        alert("This proposal has been discarded and is no longer live.")
                            .variant(AlertVariant::Success)
                            .title("Discarded")
                    })
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
        (alert(over.message()).variant(AlertVariant::Warning).title(over.heading()))
        div class="flex flex-col gap-6 mt-6" {
            @for field in view.fields() {
                (entity_field_row(field, view.draft, Mode::ReadOnly, view.rows()))
            }
            @if view.proposal.kind == ProposalKind::Organization { (address_group(view, true)) }
        }
    }
}

/// The proposal's fields, and the controls that save or discard it.
fn form(view: &EntityView<'_>) -> Markup {
    let action = view.action();
    html! {
        form
            id="entity-form"
            method="post"
            action=(action)
            class="flex flex-col gap-6 mt-6"
            data-on:submit={ "@post('" (action) "', {contentType: 'form'})" }
        {
            @for field in view.fields() {
                (entity_field_row(field, view.draft, view.mode(), view.rows()))
            }
            @if view.proposal.kind == ProposalKind::Organization { (address_group(view, false)) }
            (controls(view))
            @if view.has_agent_field() {
                @if let Some(agents) = view.agents {
                    (crate::form::widgets::agent_suggestions(agents))
                }
            }
        }
    }
}

/// One field: its control, dispatched on its own [`Shape`] rather than on
/// [`crate::form::widgets::control`]'s per-id dispatch.
///
/// That dispatch is keyed on the *project's* field ids — `"url"` there means
/// one of the project's two URL slots, read through `ProjectDraft::url_slot`
/// — and an organisation's plain `url: String` member would silently take
/// that branch and read the wrong thing. Every id here is this module's own,
/// so matching on the shape it declares is exhaustive and cannot collide.
fn entity_field_row(field: &Field, draft: &ProjectDraft, mode: Mode, rows: Rows<'_>) -> Markup {
    if mode != Mode::Editable {
        // `stated` does not care which shape a field has — it renders
        // whatever `Value` is stored — so a proposal that is no longer live
        // needs nothing beyond the field itself and the widget every other
        // read-only field on this form already uses.
        return stated(field, draft, None);
    }
    match field.shape {
        Some(Shape::Text(_)) => scalar_text(field, draft),
        Some(Shape::Multilingual) => multilingual(field, draft, 2),
        Some(Shape::StringRows) => string_rows(field, draft, rows),
        Some(Shape::AgentRows) => agent_rows(field, draft, rows),
        Some(Shape::ReferenceRows(types)) => reference_rows(field, draft, rows, types),
        other => unreachable!("every entity field declares one of the shapes matched above, got {other:?}"),
    }
}

/// A plain scalar text control — `name`, `url`, `email`. Not
/// `crate::form::widgets::control`'s dispatch, for the reason
/// [`entity_field_row`] gives; this is the same handful of lines that
/// dispatch would have run for a `Shape::Text` field, so nothing about the
/// control itself is new.
fn scalar_text(field: &Field, draft: &ProjectDraft) -> Markup {
    let value = draft.get(field.id).and_then(Value::as_str).unwrap_or_default();
    // `type="text"`, not `type="url"` or `type="email"`: a payload may hold a
    // value that does not validate, and a browser refusing to submit a
    // half-typed one would block the save nothing here requires.
    let mut control = text_field(field.id, labelled(field)).input_type(InputType::Text).value(value);
    if let Some(hint) = field.hint {
        control = control.hint(hint);
    }
    html! {
        (control)
    }
}

/// `address`: a flat group of six scalar members, which is why it is not one
/// of [`Shape`]'s repeatable or scalar shapes — nothing else in the project
/// form's registry needed a shape like it. `read_only` renders values instead
/// of controls, the way [`entity_field_row`]'s [`Mode::ReadOnly`] branch does
/// for every other field on this form.
fn address_group(view: &EntityView<'_>, read_only: bool) -> Markup {
    let member = |name: &str| -> String {
        let path = format!("address.{name}");
        view.draft.get(&path).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    // `aria-describedby` on the fieldset, not a bare sibling paragraph.
    // [`ADDRESS_HINT`] is the only place the "all four together or none" rule is
    // stated, and a paragraph rendered beside a control is not part of that
    // control's accessible description — so as a sibling it was announced to
    // nobody: a reader tabbing onto Street heard "Street, edit text" and
    // nothing about the rule whose breach `check_organization` then refuses.
    // The same argument the agent controls' `.hint()`/`.error()` already make.
    // `mosaic_tiles`' `group_shell` does this for `radio_group` and
    // `checkbox_group`, but it is `pub(super)` to that crate and one caller does
    // not justify a public tile.
    html! {
        fieldset class="field field-group" aria-describedby=(ADDRESS_HINT_ID) {
            legend class="field-label" { "Address" }
            div class="flex flex-col gap-3" {
                @for (name, label) in ADDRESS_MEMBERS.iter().copied() {
                    @let value = member(name);
                    @if read_only {
                        div class="field" {
                            p class="field-label" { (label) }
                            ({
                                crate::form::widgets::value_markup(
                                    Some(&Value::String(value)),
                                )
                            })
                        }
                    } @else {
                        ({
                            text_field(format!("address.{name}"), label)
                                .input_type(InputType::Text)
                                .value(value)
                        })
                    }
                }
            }
            p class="field-hint" id=(ADDRESS_HINT_ID) { (ADDRESS_HINT) }
        }
    }
}

/// Save, and Discard behind its confirmation — the two controls REQ-3.x's
/// entity form has, each a named submit on this one form, like the section
/// form's own.
fn controls(view: &EntityView<'_>) -> Markup {
    html! {
        @if view.confirming == Some(Confirmation::Discard) {
            div class="rounded border border-neutral-300 bg-white p-4 flex flex-col gap-3" {
                p {
                    "Discarding this proposal removes it. It cannot be undone, and the id "
                    (view.proposal.entity_id)
                    " is not reused for anything else — the row stays on record as a placeholder so nothing \
                     collected while this proposal was live can end up naming a different entity."
                }
                div class="flex items-center gap-4" {
                    ({
                        button("Yes, discard this proposal")
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, DISCARD)
                    })
                    (link("Keep it", view.action()))
                }
            }
        } @else {
            div class="flex flex-wrap items-center gap-4" {
                ({
                    button("Save")
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
}

#[cfg(test)]
mod tests {
    use platform_metadata::models::AuthorityFileReference;
    use platform_metadata::organization::Address;
    use platform_metadata::utils::Multilingual;
    use platform_metadata::{Organization, Person};

    use super::*;

    /// Every member of the contract type, as serde spells it.
    ///
    /// Read off a serialized instance rather than listed here: a second list would be one more
    /// thing to forget, which is the whole failure this test exists to catch.
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
    ///
    /// Prose cannot hold this. Add a member to `Person` and nothing fails to compile: the field
    /// list, `entities.rs`'s applier and `proposals::check_person` all keep silently ignoring it,
    /// so a depositor has no way to set it, review never shows it, and a change proposal that
    /// round-trips through the form is the only place the loss would ever surface. The project
    /// form's own registry is pinned to the data for exactly this reason — see the architecture
    /// doc on why two registry tests hold it "to the data rather than to a list repeated in
    /// prose".
    ///
    /// `id` is excluded on both sides: it lives in `EntityProposal::entity_id`, and the payload
    /// deliberately does not carry one.
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

        // `address` is deliberately not a `Field`: it is a flat group of six controls rendered by
        // `address_group`, because the contract nests it while the form does not. Named here
        // rather than filtered out of the contract side, so the test still fails if the group is
        // ever dropped — every member has to be accounted for somewhere, and this says where.
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
        let mut declared: Vec<String> = ADDRESS_MEMBERS.iter().map(|(name, _)| (*name).to_string()).collect();
        declared.sort();
        assert_eq!(declared, members(&serialized));
    }
}
