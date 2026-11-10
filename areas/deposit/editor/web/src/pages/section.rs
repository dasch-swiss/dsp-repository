//! One form section: the rail, the fields, and the save control.
//!
//! [`page`] is the whole page; [`region`] is the part a save replaces. Both come
//! from one [`SectionView`], so the plain and enhanced paths cannot drift.
//!
//! The region is the rail (state card, round or lock box, section list), the status and the form
//! together, under one id:
//! patching only the `<form>` leaves the rail showing the counts from before the
//! save. The status region is rendered empty from the first load for the
//! enhanced path: an `aria-live` region announces a change to content it
//! already holds, and one morphed in with its text is widely reported not to
//! announce. The plain path instead renders the notice already filled, from a
//! redirect's marker or a refusal's re-render: a live region filled at load is
//! no more reliably announced, so [`Notice::title_prefix`] puts the short form
//! at the front of the document `<title>` instead, which a navigation does
//! announce. No field is `required` and the form is not `novalidate`: a draft
//! may be missing anything, and validation stays on because `type="date"`
//! cannot hold a half-typed date, so with it off, fiddling the year of a real
//! date and saving would clear it.

use deposit_shell::impact_card;
use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::format::Finding;
use editor_core::form::obligation::{section_progress, SectionProgress};
use editor_core::form::registry::{sections_for, Audience, Field, Section};
use editor_core::form::submit::LineWithoutLanguage;
use editor_core::form::FormBody;
use editor_core::proposals::{EntityProposal, ProposalKind};
use editor_core::records::ReviewOutcome;
use editor_core::status::ProjectState;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use serde_json::Value;
use shared_datastar as datastar;

use crate::entity::origin::{entity_url, Origin};
use crate::entity::{entity_labels, Precedence};
use crate::form::widgets::{field_row, finding_message, Mode, Rows, FOCUS_AUTOFOCUS};
use crate::form::INTENT;

/// The id the enhanced path's patch targets. Also the anchor a save returns to.
pub const REGION_ID: &str = "project-section";

/// Store the draft, changing nothing about the review cycle. Also what an
/// unknown intent falls back to: a typo must not submit or withdraw, which the
/// depositor cannot undo, and saving is.
pub const SAVE: &str = "save";

/// Validate the draft and record it as the project's pending submission.
pub const SUBMIT: &str = "submit";

/// Take a pending submission back, leaving the draft.
pub const WITHDRAW: &str = "withdraw";

/// Show the withdrawal confirmation, which posts [`WITHDRAW`]. Two steps because
/// a withdrawal cannot be undone by the depositor; it shares this URL so a
/// refused post re-renders somewhere that still answers `GET`.
pub const WITHDRAW_CONFIRM: &str = "withdraw-confirm";

/// Discarding the draft, once confirmed. "Discard" rather than "delete": what
/// goes is the draft, never the project, and "delete the draft" reads to a
/// depositor like deleting their work from the repository.
pub const DISCARD: &str = "discard";

/// Asking first. The draft is the only copy of whatever has not been submitted,
/// and `DraftRepository::delete` cannot be undone.
pub const DISCARD_CONFIRM: &str = "discard-confirm";

/// Run the agent pickers' searches and re-render, changing nothing else. One
/// intent for every Search button because each posts with the form; it
/// re-renders with the posted body kept, which carries the query back.
pub const FIND_AGENT: &str = "find-agent";

/// The Search button of a vocabulary combobox: handled like [`FIND_AGENT`].
pub const FIND_ENTRY: &str = "find-entry";

/// Start a proposal for a new person.
pub const PROPOSE_PERSON: &str = "propose-person";

/// Start a proposal for a new organisation.
pub const PROPOSE_ORGANIZATION: &str = "propose-organization";

/// Propose a change to an entity this project already references. The card's field and the entity
/// ride in the intent value (`propose-changes:attributions:person-417`) via
/// [`propose_changes_intent`] and [`proposed_entity`]; the field is where the entity form returns
/// to.
///
/// It must not move to a field of its own: a section renders every resolved
/// agent row inside one `<form>`, so a per-row field shares one name across all
/// of them and `FormBody::get` takes the first. Only the activated button posts
/// its name and value, natively and through `SubmitEvent.submitter`.
pub const PROPOSE_CHANGES: &str = "propose-changes";

/// The separator between [`PROPOSE_CHANGES`], the field id and the entity id: neither a field id
/// nor an entity id contains a colon.
const PROPOSE_CHANGES_SEPARATOR: char = ':';

/// The intent value a "Propose changes" control on `entity_id`'s card in `field` posts.
#[must_use]
pub fn propose_changes_intent(field: &str, entity_id: &str) -> String {
    format!("{PROPOSE_CHANGES}{PROPOSE_CHANGES_SEPARATOR}{field}{PROPOSE_CHANGES_SEPARATOR}{entity_id}")
}

/// The field and entity id a posted intent carries, or `None` when it is not a
/// propose-changes intent. An empty entity id answers `None`: naming the verb with no
/// entity has asked for nothing.
#[must_use]
pub fn proposed_entity(intent: &str) -> Option<(&str, &str)> {
    intent
        .strip_prefix(PROPOSE_CHANGES)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)?
        .split_once(PROPOSE_CHANGES_SEPARATOR)
        .filter(|(_, entity_id)| !entity_id.is_empty())
}

/// The intent a replace-search option posts, ahead of the picker's `unique`
/// and the chosen agent id: `choose-agent:{unique}:{id}`.
const CHOOSE_AGENT: &str = "choose-agent";

/// The intent value choosing `agent_id` in the picker named `unique` posts. No
/// `unique` contains a colon, and [`PROPOSE_CHANGES_SEPARATOR`] says no entity id does.
#[must_use]
pub fn choose_agent_intent(unique: &str, agent_id: &str) -> String {
    format!("{CHOOSE_AGENT}{PROPOSE_CHANGES_SEPARATOR}{unique}{PROPOSE_CHANGES_SEPARATOR}{agent_id}")
}

/// The picker and agent id a posted intent carries, or `None` when it is not a
/// choice or either part is empty.
#[must_use]
pub fn chosen_agent(intent: &str) -> Option<(&str, &str)> {
    intent
        .strip_prefix(CHOOSE_AGENT)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)?
        .split_once(PROPOSE_CHANGES_SEPARATOR)
        .filter(|(unique, agent_id)| !unique.is_empty() && !agent_id.is_empty())
}

/// The intent an append search's option posts: `add-agent:{field}:{agent_id}`.
const ADD_AGENT: &str = "add-agent";

/// The intent value appending a new row holding `agent_id` to the picker field `field` posts. No
/// field id contains a colon, and [`PROPOSE_CHANGES_SEPARATOR`] says no entity id does.
#[must_use]
pub fn add_agent_intent(field: &str, agent_id: &str) -> String {
    format!("{ADD_AGENT}{PROPOSE_CHANGES_SEPARATOR}{field}{PROPOSE_CHANGES_SEPARATOR}{agent_id}")
}

/// The field and agent id a posted intent carries, or `None` when it is not an add or either
/// part is empty.
#[must_use]
pub fn added_agent(intent: &str) -> Option<(&str, &str)> {
    intent
        .strip_prefix(ADD_AGENT)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)?
        .split_once(PROPOSE_CHANGES_SEPARATOR)
        .filter(|(field, agent_id)| !field.is_empty() && !agent_id.is_empty())
}

/// The intent a vocabulary combobox's option posts: `pick:{prefix}:{value}`.
const PICK: &str = "pick";

/// The intent picking `value` from the vocabulary combobox of the row `prefix` (`disciplines.r0`).
#[must_use]
pub fn pick_intent(prefix: &str, value: &str) -> String {
    format!("{PICK}{PROPOSE_CHANGES_SEPARATOR}{prefix}{PROPOSE_CHANGES_SEPARATOR}{value}")
}

/// The row prefix and value a posted pick carries, or `None` when it is not a pick or either part
/// is empty. Splits at the first colon after the verb: a row prefix never holds one, a url does.
#[must_use]
pub fn picked_entry(intent: &str) -> Option<(&str, &str)> {
    intent
        .strip_prefix(PICK)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)?
        .split_once(PROPOSE_CHANGES_SEPARATOR)
        .filter(|(prefix, value)| !prefix.is_empty() && !value.is_empty())
}

/// The intent a card's Change control posts: `change-agent:{unique}`. It opens the search on an
/// existing card and, like [`FIND_AGENT`], stores the body and re-renders without a redirect.
const CHANGE_AGENT: &str = "change-agent";

/// The intent value opening the search on the card of the picker named `unique`.
#[must_use]
pub fn change_agent_intent(unique: &str) -> String {
    format!("{CHANGE_AGENT}{PROPOSE_CHANGES_SEPARATOR}{unique}")
}

/// The picker a posted intent opens the search on, or `None` when it is not a change or the
/// picker is empty.
#[must_use]
pub fn changed_agent(intent: &str) -> Option<&str> {
    intent
        .strip_prefix(CHANGE_AGENT)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)
        .filter(|unique| !unique.is_empty())
}

/// The intent a contributor role chip's Remove posts: `remove-role:{prefix}:{role}`.
const REMOVE_ROLE: &str = "remove-role";

/// The intent value removing the role `role` of the contributor row `prefix`
/// (`attributions.r0`). The role rides as text, not as a position: the chips follow the held
/// order and the posted `{prefix}.role` pairs follow the checkboxes' document order, so an index
/// would name a different role in each.
#[must_use]
pub fn remove_role_intent(prefix: &str, role: &str) -> String {
    format!("{REMOVE_ROLE}{PROPOSE_CHANGES_SEPARATOR}{prefix}{PROPOSE_CHANGES_SEPARATOR}{role}")
}

/// The row prefix and role a posted intent carries, or `None` when it is not a role removal or
/// either part is empty. Split on the first colon after the verb: a prefix never holds one, a
/// custom role may.
#[must_use]
pub fn removed_role(intent: &str) -> Option<(&str, &str)> {
    intent
        .strip_prefix(REMOVE_ROLE)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)?
        .split_once(PROPOSE_CHANGES_SEPARATOR)
        .filter(|(prefix, role)| !prefix.is_empty() && !role.is_empty())
}

/// Whether a posted intent keeps the body on the enhanced path of a write: the choice, the pick,
/// the append and the role removal edit the form's rows, and the rest of the form may hold a row
/// that is not storable yet. Find and change re-render the body in any case.
#[must_use]
pub fn keeps_posted(intent: &str) -> bool {
    chosen_agent(intent).is_some()
        || picked_entry(intent).is_some()
        || added_agent(intent).is_some()
        || removed_role(intent).is_some()
}

/// The name a form posts the draft revision it was rendered from under. The
/// draft is one row and `upsert` is last-write-wins, so without this a save
/// silently replaces somebody else's work. A courtesy check between people, not
/// a security control: a body that omits it is saved without complaint.
pub const BASELINE: &str = "baseline";

/// Why the form is read-only. A reason rather than a `bool`: a depositor whose
/// work is queued and one whose work a reviewer has open should not be told the
/// same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locked {
    /// Submitted and waiting for a reviewer to pick it up.
    Submitted,
    /// A reviewer has it open.
    InReview,
}

impl Locked {
    /// RDU's wording is [`rdu_locked_body`].
    const fn message(self) -> &'static str {
        match self {
            Self::Submitted => "In review. You can't edit until RDU has looked at it.",
            Self::InReview => {
                "RDU is reviewing this project, so the form is read-only. It becomes editable \
                               again when the review finishes or the record is returned to you."
            }
        }
    }

    const fn heading(self) -> &'static str {
        match self {
            Self::Submitted => "Submitted for review",
            Self::InReview => "In review",
        }
    }
}

/// The latest finished review round, as the depositor's form shows it. This
/// communicates the round; which fields are fixed is [`SectionView::accepted_fields`].
pub struct RoundSummary<'a> {
    pub outcome: ReviewOutcome,
    /// What RDU wrote. Required for a rejection and a request-changes, absent
    /// for an approval and a withdrawal.
    pub note: Option<&'a str>,
    /// Already formatted; the server owns the format.
    pub at: &'a str,
    /// Each field RDU put its own value in place of, with that value. There is no
    /// second approver, so a substituted value is seen by nobody unless shown here.
    pub substitutions: &'a [(String, Value)],
}

/// The heading of a round box, what it means for the viewer now, and for a
/// depositor's approval what editing does next, which is a second paragraph.
fn round_wording(outcome: ReviewOutcome, audience: Audience) -> (&'static str, &'static str, Option<&'static str>) {
    let rdu = audience == Audience::RduOnly;
    match outcome {
        ReviewOutcome::ChangesRequested => (
            if rdu {
                "A reviewer asked for changes"
            } else {
                "RDU asked for changes"
            },
            if rdu {
                "The depositor can change the project and submit it again; fields RDU accepted stay fixed until \
                 then."
            } else {
                "This project is a draft again. Fields RDU accepted are fixed until you submit it again; \
                 everything else is yours to edit."
            },
            None,
        ),
        // This box is the only place that says a rejection discarded the submission.
        ReviewOutcome::Rejected => (
            if rdu {
                "A reviewer rejected this submission"
            } else {
                "RDU rejected this submission"
            },
            if rdu {
                "The submission was discarded and the published project is unchanged. The depositor's draft is \
                 kept, so they can change it and submit again."
            } else {
                "The submission was discarded and the published project is unchanged. Your draft is still here, \
                 so you can change it and submit again."
            },
            None,
        ),
        // Depositor-facing, so the mechanism is not named; the wait is read
        // from ProjectState::Approved, the normative wording.
        ReviewOutcome::Approved => (
            "RDU approved this project",
            editor_core::status::ProjectState::Approved.explanation_for(audience),
            (!rdu).then_some("You can keep editing; your next changes go to RDU as a new submission."),
        ),
        // `persistent_box` shows no Withdrawn round: the withdraw flash says it.
        ReviewOutcome::Withdrawn => (
            "The submission was taken back",
            "It is no longer in RDU's queue. Your draft is unchanged, so you can keep editing and submit \
             again.",
            None,
        ),
    }
}

fn round_variant(outcome: ReviewOutcome) -> AlertVariant {
    match outcome {
        ReviewOutcome::Approved => AlertVariant::Success,
        ReviewOutcome::Rejected => AlertVariant::Warning,
        ReviewOutcome::ChangesRequested | ReviewOutcome::Withdrawn => AlertVariant::Info,
    }
}

/// What the `POST` that led to this rendering did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice<'a> {
    /// The draft was stored.
    Saved,
    /// The draft became the project's pending submission.
    Submitted,
    /// A pending submission was taken back, and the form is editable again.
    Withdrawn,
    /// The draft was discarded, and the form is showing published metadata
    /// again.
    Discarded,
    /// Somebody else saved the draft while this form was open. Carries their
    /// name where it is known, so the reader can go and ask rather than guess.
    Changed { by: Option<&'a str>, at: &'a str },
    /// A new person or organisation was proposed; links to the entity form, which is where the
    /// depositor fills it in. A change opens that form instead of answering here.
    Proposed { kind: ProposalKind, entity_id: &'a str },
    /// The write was refused, and why: the whole-form kind. Field-level errors are
    /// [`SectionView::errors`], which name a control the reader can fix.
    Refused(&'a str),
    /// One of this project's entity proposals was discarded on its own form, which returned here.
    /// Carries the sentence naming it, built where the withdrawn proposal is in hand.
    ProposalDiscarded(&'a str),
}

/// Short form of [`Notice::Saved`], shared between its alert text in [`status`]
/// and [`Notice::title_prefix`] so a wording change cannot update one and not
/// the other.
const SAVED_TITLE: &str = "Draft saved";
/// Short form of [`Notice::Submitted`]; see [`SAVED_TITLE`].
const SUBMITTED_TITLE: &str = "Sent to RDU";
/// Short form of [`Notice::Withdrawn`]; see [`SAVED_TITLE`].
const WITHDRAWN_TITLE: &str = "Submission withdrawn";
/// Short form of [`Notice::Discarded`]; see [`SAVED_TITLE`].
const DISCARDED_TITLE: &str = "Draft discarded";
/// Short form of [`Notice::ProposalDiscarded`]; see [`SAVED_TITLE`].
const PROPOSAL_DISCARDED_TITLE: &str = "Proposal discarded";
/// The prefix for a write that did not land: [`Notice::Refused`] and [`Notice::Changed`].
const NOT_SAVED_TITLE: &str = "Not saved";

impl Notice<'_> {
    /// The prefix a full page load puts in front of the document `<title>`, so a
    /// screen reader hears the outcome of the reader's write on arrival (see the
    /// module docs). `None` for [`Self::Proposed`], which does not answer one.
    pub fn title_prefix(&self) -> Option<&'static str> {
        match self {
            Notice::Saved => Some(SAVED_TITLE),
            Notice::Submitted => Some(SUBMITTED_TITLE),
            Notice::Withdrawn => Some(WITHDRAWN_TITLE),
            Notice::Discarded => Some(DISCARDED_TITLE),
            Notice::ProposalDiscarded(_) => Some(PROPOSAL_DISCARDED_TITLE),
            Notice::Refused(_) | Notice::Changed { .. } => Some(NOT_SAVED_TITLE),
            Notice::Proposed { .. } => None,
        }
    }
}

/// An action that asks before it acts. Both are irreversible for the person
/// doing them: a withdrawal drops whatever a reviewer had recorded, and a
/// discard drops the only copy of unsubmitted work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// Taking a pending submission back.
    Withdrawal,
    /// Discarding the draft.
    Discard,
}

/// Everything one section rendering needs. A struct rather than a long argument
/// list: half of these are `Option`s of similar types, and adjacent optional
/// arguments of one type are silently swappable.
pub struct SectionView<'a> {
    pub shortcode: &'a str,
    /// The published project's name, or `None` for a shortcode the published set
    /// does not hold.
    pub project_name: Option<&'a str>,
    /// A project RDU created in the editor, which "Discard draft" reseeds rather than empties.
    pub created: bool,
    /// The state the project list shows for this project, in the rail's state card.
    pub state: ProjectState,
    pub section: &'static Section,
    /// Which fields and sections this reader sees. The registry is consulted by
    /// the decoder too, so the form and the save cannot disagree.
    pub audience: Audience,
    pub draft: &'a ProjectDraft,
    /// `None` while the project is editable.
    pub locked: Option<Locked>,
    /// Field ids RDU accepted in the round being answered, fixed until the next
    /// submit. Ids rather than a registry flag because the set is per-round data;
    /// empty when the latest round did not return the project.
    pub accepted_fields: &'a [String],
    /// Whether this reader may take the pending submission back, and
    /// so whether the withdrawal control is offered at all.
    pub may_withdraw: bool,
    /// Whether this reader may discard the draft. False without a stored draft, and
    /// false while a submission is pending: the draft is what the depositor comes
    /// back to when RDU returns the project.
    pub may_discard: bool,
    /// Which confirmation is showing, if any. A named type rather than one `bool`
    /// per action, which allows two nonsense states.
    pub confirming: Option<Confirmation>,
    /// Per-field submit errors, keyed by the path the field posts under
    /// (`temporalCoverage[0]`), in field order. Separate from [`Notice::Refused`]:
    /// these name a control the reader can fix.
    pub errors: &'a [(String, String)],
    /// Findings against this project's own live proposals, from the latest submit
    /// refusal. Separate from [`Self::errors`]: a proposal is not a registry field,
    /// so `errors_elsewhere`'s lookup would drop it.
    pub proposal_findings: &'a [(String, ProposalKind, String)],
    /// This project's own entity proposals, every status: the summary filters to
    /// `is_live()` here, so one place decides which proposals show.
    pub proposals: &'a [EntityProposal],
    /// The latest finished review round, or `None` for a project nobody has
    /// reviewed. In the rail: outside the live region, so a save does not
    /// re-announce it, and inside the patched region, so a save leaves it in place.
    /// It shows until the depositor submits again, which is what makes a returned
    /// draft distinguishable from one never submitted without a sixth lifecycle state.
    pub round: Option<RoundSummary<'a>>,
    /// Who last saved the draft, when that was somebody other than this reader;
    /// `None` for their own saves and for a removed account.
    pub last_editor: Option<&'a str>,
    /// When this reader will be signed out, formatted, and only when that is close
    /// enough to say: a warning permanently on screen is a warning nobody reads.
    pub signed_out_at: Option<&'a str>,
    /// The stored draft's revision, posted back under [`BASELINE`] so a save can
    /// tell whether the draft moved underneath it.
    pub baseline: Option<&'a str>,
    /// When the stored draft was last written, formatted. `None` when the form
    /// is showing published metadata that nobody has saved over yet.
    pub saved_at: Option<&'a str>,
    pub notice: Option<Notice<'a>>,
    /// The body that was posted, when this render is answering a `POST`. Only
    /// repeatable fields read it, for the one thing the draft cannot hold: a row a
    /// depositor added but has not filled in, which is never stored. `None` after an
    /// explicit save, since a row with no text is not data; `Some` for a refusal,
    /// for add/remove, for an agent picker's choice and for an autosave.
    pub posted: Option<&'a FormBody>,
    /// Values without the format their kind needs, each rendered on its own control
    /// on every rendering of the draft that holds it, except the autosave's, which leaves
    /// out a value typed since the last write. Separate from [`Self::errors`], which a format
    /// refusal fills with one line per field holding a finding outside this section, or in it
    /// with no control because RDU accepted it.
    pub format_findings: Vec<Finding>,
    /// On a [`Notice::Saved`] render, the format findings of every other section this
    /// reader sees, each with its section: submit refuses them all, so the saved notice
    /// counts and links them with [`Self::format_findings`]. Empty on any other render.
    pub findings_elsewhere: Vec<(&'static Section, Finding)>,
    /// The body this render answers whether or not `posted` keeps it; see
    /// [`Rows::submitted`].
    pub submitted: Option<&'a FormBody>,
    /// The field one more blank row was just asked for.
    pub adding_row: Option<&'a str>,
    /// Whether the first control holding a format finding takes focus: a refused submit's
    /// re-render, as `/projects/new` focuses its refused field.
    pub focus_finding: bool,
    /// Language-row lines posted with text and no language: every one on the save or submit that
    /// found them, and on a later render that keeps the body, those still unfixed.
    pub without_language: Vec<LineWithoutLanguage>,
    /// The agents an id field may refer to, borrowed from `EditorState`'s snapshot.
    pub agents: Option<&'a AgentScope<'a>>,
    /// Base URL a repeatable field's add and remove controls submit to: `/fields`
    /// under the section's own URL, built by the caller because the route shape
    /// belongs to the router, so a row action resolves through the same audience
    /// gate, lock check and shortcode fold the save does.
    pub rows_action: String,
    /// Whether an approved change is waiting for the repository release that
    /// carries it. Not a [`Locked`] variant: approve is the only outcome that does
    /// not hand the project back, so the form stays editable and what the
    /// depositor types is the next cycle. It is also the only thing that shows an
    /// approved round (see [`persistent_box`]), so the box stops promising a
    /// release once the record is gone.
    pub awaiting_release: bool,
}

impl SectionView<'_> {
    /// The prefix a full page load puts in front of the document `<title>`: the
    /// notice's own, plus on a save how many values still need fixing, which a
    /// screen reader would otherwise never hear about (see the module docs).
    pub fn title_prefix(&self) -> Option<String> {
        let prefix = self.notice?.title_prefix()?;
        match (self.notice, self.values_to_fix()) {
            (Some(Notice::Saved), count) if count > 0 => Some(format!("{prefix}, {}", needs_fixing(count))),
            _ => Some(prefix.to_string()),
        }
    }

    /// Values without their format anywhere in the project and lines with no language, which
    /// submit refuses.
    fn values_to_fix(&self) -> usize {
        self.format_findings.len() + self.findings_elsewhere.len() + self.without_language.len()
    }

    /// Where a link to a finding goes: its control, on this page or on its section's.
    /// A field RDU accepted renders no control, so its link stops at the section.
    fn finding_href(&self, section: &Section, finding: &Finding) -> String {
        let page = format!("/projects/{}/sections/{}", self.shortcode, section.id);
        if self.accepted_fields.iter().any(|accepted| accepted == finding.field.id) {
            return page;
        }
        let page = if section.id == self.section.id {
            String::new()
        } else {
            page
        };
        format!("{page}#{}", finding.control)
    }

    /// How one field renders. Three answers, not two: a whole-form lock and an
    /// accepted field are both read-only and lift on different events, so the
    /// reader has to be told which applies.
    fn mode_of(&self, field: &'static str) -> Mode {
        if self.locked.is_some() {
            Mode::ReadOnly
        } else if self.accepted_fields.iter().any(|accepted| accepted == field) {
            Mode::Accepted
        } else {
            Mode::Editable
        }
    }

    /// Errors against one field, matched on the path a control posts under —
    /// so `temporalCoverage[0]` belongs to `temporalCoverage` and marks the
    /// whole field, which is the granularity this form renders at.
    fn errors_for(&self, field: &str) -> Vec<&str> {
        self.errors
            .iter()
            .filter(|(path, _)| Self::names(path, field))
            .map(|(_, message)| message.as_str())
            .collect()
    }

    /// Whether an error path belongs to a field.
    fn names(path: &str, field: &str) -> bool {
        path == field || path.strip_prefix(field).is_some_and(|rest| rest.starts_with('['))
    }

    /// Errors naming a field this section does not show, with the section it is
    /// in. Submit validation is whole-project, so a refusal is routinely about a
    /// field the reader is not looking at.
    fn errors_elsewhere(&self) -> Vec<(&'static Section, &str, &str)> {
        let shown: Vec<&str> = self.section.fields_for(self.audience).map(|field| field.id).collect();
        self.errors
            .iter()
            .filter(|(path, _)| !shown.iter().any(|field| Self::names(path, field)))
            .filter_map(|(path, message)| {
                // The field id is the path with any `[index]` suffix off.
                let id = path.split_once('[').map_or(path.as_str(), |(id, _)| id);
                let section = editor_core::form::registry::section_of(id)?;
                let label = editor_core::form::registry::field(id).map_or(id, |field| field.label);
                Some((section, label, message.as_str()))
            })
            .collect()
    }

    /// Whether any field this reader sees offers the agent suggestion list. From
    /// `Shape::has_agent_picker`, which is exhaustive, rather than a list of shapes
    /// here that could miss one.
    fn has_agent_picker_field(&self) -> bool {
        self.section
            .fields_for(self.audience)
            .any(|field| field.shape.is_some_and(editor_core::form::Shape::has_agent_picker))
    }

    fn action(&self) -> String {
        format!("/projects/{}/sections/{}", self.shortcode, self.section.id)
    }

    /// What a repeatable field needs: the posted body, and the base URL its add and
    /// remove controls submit to, see [`Self::rows_action`], and the agent search a picker runs as
    /// its combobox is typed in, `search_path`, and the vocabulary search a discipline, place or
    /// period picker runs, `vocabulary_path`.
    fn rows<'a>(&'a self, search_path: &'a str, vocabulary_path: &'a str) -> Rows<'a> {
        let fields: Vec<&Field> = self.section.fields_for(self.audience).collect();
        Rows {
            posted: self.posted,
            submitted: self.submitted,
            action: &self.rows_action,
            adding: self.adding_row,
            agents: self.agents,
            format_findings: &self.format_findings,
            // The section form dispatches all three propose intents, unlike the entity
            // form's affiliations picker, which reuses these widgets.
            propose: true,
            search_path: Some(search_path),
            vocabulary_path: Some(vocabulary_path),
            // A field RDU accepted renders no control to focus.
            // A line without a language goes first: it is what a render that carries one is about.
            focus: self
                .focus_finding
                .then(|| {
                    self.without_language.first().map(|line| line.control.as_str()).or_else(|| {
                        self.format_findings
                            .iter()
                            .find(|finding| !self.accepted_fields.iter().any(|accepted| accepted == finding.field.id))
                            .map(|finding| finding.control.as_str())
                    })
                })
                .flatten(),
            without_language: &self.without_language,
            label_hidden: label_is_heading(self.section, &fields),
        }
    }
}

/// The whole page: the project heading, then the region a save replaces.
pub fn page(view: &SectionView<'_>) -> Markup {
    html! {
        div class="max-w-5xl py-8" { (heading(view)) (region(view)) }
    }
}

/// Sticky only in a viewport at least 30rem tall: at 400% zoom the wrapped row would cover half
/// the screen (WCAG 1.4.10). Keep in step with `scroll-padding-bottom` in `style/main.css`.
const CONTROL_ROW: &str =
    "[@media(min-height:30rem)]:sticky bottom-0 z-10 flex flex-wrap items-center gap-4 bg-white pt-3 pb-3";

/// The rail, the status region and the form — everything a save can change.
///
/// Rendered on its own for the enhanced path's patch, and spliced into [`page`]
/// for the plain one, so the two cannot drift.
pub fn region(view: &SectionView<'_>) -> Markup {
    html! {
        section
            id=(REGION_ID)
            class="grid min-[700px]:grid-cols-[200px_1fr] items-start"
            data-init=[view.focus_finding.then_some(FOCUS_AUTOFOCUS)]
        {
            (rail(view))
            div class="min-[700px]:pl-6 pt-6 md:[&_:is(input,select,textarea,fieldset)]:scroll-mt-[33vh]"
            { (status(view)) (proposals_summary(view)) (form(view)) }
        }
    }
}

/// The project's name and shortcode, and the way back to the list.
fn heading(view: &SectionView<'_>) -> Markup {
    html! {
        div class="mb-6" {
            @match view.project_name {
                Some(name) => {
                    h1 class="font-display text-2xl mb-1" { (name) }
                    p class="font-mono text-sm text-gray-600" { (view.shortcode) }
                }
                None => {
                    h1 class="font-display text-2xl mb-1" { "Project " (view.shortcode) }
                    // A project may exist only locally; a blank form with no explanation
                    // reads as a failure to load.
                    p class="text-gray-600" {
                        "This project has not been published yet, so there was nothing to pre-fill. \
                         Anything you enter is kept as a draft."
                    }
                }
            }
            p class="mt-2" {
                a href="/projects" class="underline" { "Back to your projects" }
            }
        }
    }
}

/// The rail link's accessible name. The title and the progress are adjacent
/// `<span>`s with no whitespace between them, so the name computation would
/// give "Overview5 of 5 required". `None` for a section with no requirements,
/// where the visible title is the whole name. The label starts with the visible
/// title, as WCAG 2.5.3 asks of an `aria-label` over visible text.
fn rail_link_label(title: &str, progress: &SectionProgress) -> Option<String> {
    progress.has_requirements().then(|| format!("{title}, {}", progress.summary()))
}

/// One rail link's classes, as two complete literals: Tailwind collects classes
/// by scanning source text, so a class assembled at runtime is never emitted and
/// the link is silently unstyled.
const fn rail_link_class(current: bool) -> &'static str {
    if current {
        "flex items-center gap-2 rounded bg-primary-50 px-2 py-1.5 text-[12.5px] font-bold text-primary-700 \
         focus-visible:outline-2 focus-visible:outline-primary-600"
    } else {
        "flex items-center gap-2 rounded px-2 py-1.5 text-[12.5px] font-bold text-neutral-700 \
         hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-primary-600"
    }
}

/// One complete class literal per look, for Tailwind's scan. `aria-hidden`: the link's name and
/// summary carry the meaning.
fn rail_glyph(progress: &SectionProgress) -> Markup {
    html! {
        @if !progress.has_requirements() {
            span
                class="size-4 shrink-0 rounded-full border-[1.5px] border-neutral-300"
                aria-hidden="true" {}
        } @else if progress.is_complete() {
            span
                class="flex size-4 shrink-0 items-center justify-center rounded-full bg-success-600 text-[10px] \
                       font-bold text-white"
                aria-hidden="true"
            { "✓" }
        } @else {
            span
                class="flex size-4 shrink-0 items-center justify-center rounded-full bg-danger-600 text-[10px] \
                       font-bold text-white"
                aria-hidden="true"
            { "!" }
        }
    }
}

/// The rule is `unsatisfied_required`'s; this only narrows it to the fields this page shows.
fn missing_here(view: &SectionView<'_>) -> Vec<&'static Field> {
    let shown: Vec<&str> = view.section.fields_for(view.audience).map(|field| field.id).collect();
    editor_core::form::obligation::unsatisfied_required(view.audience, view.draft)
        .into_iter()
        .filter(|field| shown.contains(&field.id))
        .collect()
}

/// `None` where the field renders no control, so the chip is text, not a dead link. Must match
/// [`field_row`]'s editable arm.
fn missing_href(view: &SectionView<'_>, field: &Field) -> Option<String> {
    (view.mode_of(field.id) == Mode::Editable && field.shape.is_some()).then(|| format!("#{}", field.id))
}

fn missing_chips(view: &SectionView<'_>) -> Markup {
    let missing = missing_here(view);
    html! {
        @if !missing.is_empty() {
            ul  class="mt-1 mb-1 ml-8 flex flex-wrap gap-1.5"
                aria-label="Required fields not filled in"
            {
                @for field in missing {
                    li {
                        @if let Some(href) = missing_href(view, field) {
                            a   href=(href)
                                class="inline-block rounded-[3px] border border-danger-200 px-1.5 py-1 text-[10px] \
                                       font-bold text-danger-700 hover:bg-danger-50 focus-visible:outline-2 \
                                       focus-visible:outline-primary-600"
                            { (field.label) }
                        } @else {
                            span
                                class="inline-block rounded-[3px] border border-danger-200 px-1.5 py-1 text-[10px] font-bold text-danger-700"
                            { (field.label) }
                        }
                    }
                }
            }
        }
    }
}

/// Only where a save is possible: a locked form, or one showing a confirmation, has no draft to
/// describe.
fn saved_line(view: &SectionView<'_>) -> Markup {
    let editable = view.section.has_editable_field(view.audience);
    html! {
        @if view.confirming.is_none() && view.locked.is_none() && editable {
            @if let Some(saved_at) = view.saved_at {
                p class="mt-1 text-sm text-neutral-600" {
                    "Draft last saved "
                    (saved_at)
                    // Named only when it was somebody else: "saved by you"
                    // is the ordinary case and reads as noise.
                    @if let Some(editor) = view.last_editor { " by " (editor) }
                    "."
                }
            } @else {
                p class="mt-1 text-sm text-neutral-600" {
                    "Nothing saved yet — this form shows the project's published metadata."
                }
            }
        }
    }
}

fn state_card(view: &SectionView<'_>) -> Markup {
    html! {
        div class="rounded-md border border-neutral-200 bg-neutral-50 p-3" {
            (crate::state_badge::state_badge(view.state))
            (saved_line(view))
        }
    }
}

/// The section list is a `<nav>` with a list, so a screen reader user can jump to it and count
/// the items. The round and lock box sit outside [`status`], the one live region: a save
/// re-renders the whole region and must not announce a standing note again.
fn rail(view: &SectionView<'_>) -> Markup {
    html! {
        // Not sticky itself: a sticky box taller than the viewport hides its own bottom.
        div class="flex flex-col gap-3 self-stretch bg-white p-3 border-b border-neutral-200 min-[700px]:border-b-0 min-[700px]:border-r"
        {
            (state_card(view))
            (persistent_box(view))
            nav aria-label="Form sections" class="min-[700px]:sticky min-[700px]:top-4" {
                p   class="mb-1 px-2 text-[11px] font-bold uppercase tracking-wide text-neutral-600"
                    aria-hidden="true"
                { "Sections" }
                ol class="flex flex-col gap-0.5" {
                    @for section in sections_for(view.audience) {
                        @let progress = section_progress(section, view.audience, view.draft);
                        @let current = section.id == view.section.id;
                        li {
                            a   href={ "/projects/" (view.shortcode) "/sections/" (section.id) }
                                class=(rail_link_class(current))
                                aria-current=[current.then_some("page")]
                                aria-label=[rail_link_label(section.title, &progress)]
                            {
                                (rail_glyph(&progress))
                                span class="flex flex-col" {
                                    span { (section.title) }
                                    @if progress.has_requirements() {
                                        // Both numbers, in words: a rail showing only what is outstanding
                                        // cannot tell a finished section from an empty one.
                                        span class="text-xs font-normal text-neutral-600" {
                                            (progress.summary())
                                        }
                                    }
                                }
                            }
                            @if current { (missing_chips(view)) }
                        }
                    }
                }
            }
        }
    }
}

fn submitted_body(audience: Audience) -> Markup {
    match audience {
        Audience::Everyone => html! {
            "Submitted for review. The form is read-only until RDU returns the project to you, and your draft \
             is kept either way."
        },
        Audience::RduOnly => html! {
            "Sent to the review queue. The form is read-only until a reviewer returns the project, and the \
             draft is kept either way. "
            (review_queue_link())
        },
    }
}

fn withdrawn_body(audience: Audience) -> Markup {
    html! {
        @match audience {
            Audience::Everyone => {
                "The submission has been taken back and is no longer in RDU's queue. Your draft is \
                 unchanged, so you can keep editing and submit again."
            }
            Audience::RduOnly => {
                "The submission has been taken back and is no longer in the review queue. The draft is \
                 unchanged."
            }
        }
    }
}

fn review_queue_link() -> Markup {
    html! {
        a href="/review" class="underline" { "Open the review queue" }
    }
}

/// The live region a save writes its outcome into. Always present, see the module
/// docs; `empty:hidden` keeps an always-present region free of a line box.
///
/// `sticky` because the control that causes a notice is usually below it: the
/// enhanced path patches the region in place without moving the scroll, so a
/// notice rendered at the top of the column is invisible to a depositor who
/// clicked several screens down. Screen readers hear it through `aria-live`;
/// this is the sighted half.
///
/// Capped at a third of the viewport and scrollable (hence focusable) from `md`, with
/// the section column's fields given a matching scroll margin, so a focused field lands
/// below it (WCAG 2.4.11). Not sticky below `md`: a refusal there fills the screen and
/// would cover the field it is about.
fn status(view: &SectionView<'_>) -> Markup {
    html! {
        div class="empty:hidden md:sticky md:top-0 md:z-10 md:max-h-[33vh] md:overflow-y-auto"
            tabindex="0"
            aria-live="polite"
        {
            @match view.notice {
                Some(Notice::Saved) => { (saved_notice(view)) }
                Some(Notice::Submitted) => {
                    ({
                        alert(submitted_body(view.audience))
                            .variant(AlertVariant::Success)
                            .title(SUBMITTED_TITLE)
                    })
                }
                Some(Notice::Withdrawn) => {
                    ({
                        alert(withdrawn_body(view.audience))
                            .variant(AlertVariant::Success)
                            .title(WITHDRAWN_TITLE)
                    })
                }
                Some(Notice::Discarded) => { (discarded_notice(view.created)) }
                Some(Notice::Changed { by, at }) => { (changed_notice(by, at)) }
                Some(Notice::Proposed { kind, entity_id }) => {
                    (proposed_notice(view, kind, entity_id))
                }
                Some(Notice::ProposalDiscarded(message)) => {
                    ({
                        alert(message)
                            .variant(AlertVariant::Success)
                            .title(PROPOSAL_DISCARDED_TITLE)
                    })
                }
                Some(Notice::Refused(message)) => { (alert(message).variant(AlertVariant::Warning)) }
                None => {}
            }
            // Inside the live region, beside the refusal it details: the refusal is what
            // gets announced, and it is about fields the reader cannot see, so the list
            // has to announce with it.
            (errors_elsewhere(view))
            (proposal_findings_elsewhere(view))
            // Not an outcome of anything the reader just did: true of the session
            // whatever the last action was.
            @if let Some(at) = view.signed_out_at { (sign_out_notice(at)) }
        }
    }
}

const AUTOSAVE_NOT_COMBOBOX: &str = "!evt.target.matches('[role=combobox]')";

/// Whether the one field a section shows repeats the section's heading, so its own label is
/// only for a screen reader.
fn label_is_heading(section: &Section, fields: &[&Field]) -> bool {
    matches!(fields, [only] if only.label == section.title)
}

/// The section's fields, and the control that saves them.
fn form(view: &SectionView<'_>) -> Markup {
    // Only on a form a save can change: a read-only one has no controls for
    // change to fire from, and advertising a save the handler would refuse is
    // worse than none.
    //
    // A combobox's `change` is its own: the search it types ahead is not an edit, and its choice
    // posts as an intent, so an autosave in between would only replace the box under the cursor.
    let autosave = view
        .locked
        .is_none()
        .then(|| format!("{AUTOSAVE_NOT_COMBOBOX} && {}", datastar::post(view.action()).form()));
    let action = view.action();
    let agents_path = format!("{action}/agents");
    let vocabulary_path = format!("{action}/vocabulary");
    let fields: Vec<&Field> = view.section.fields_for(view.audience).collect();
    html! {
        // The action is the URL this form was fetched from, so a rejected save
        // re-renders somewhere that answers GET. No __prevent: Datastar calls
        // preventDefault unconditionally on a form's submit. With no script it is an
        // ordinary POST.
        //
        // It posts the submitter's formAction, not the form's action: every add and
        // remove control is a submit button carrying a formaction, and preventDefault
        // discards the submitter's URL with the native submission, so posting the
        // form's action would turn every row control into a plain save. Only a
        // browser with JavaScript can see that. submitter.formAction falls back to
        // the form's action for a button without one, so save, submit and the
        // propose intents are unaffected.
        form
            id="section-form"
            method="post"
            action=(action)
            class="flex flex-col"
            data-on:submit=(datastar::post(&action).submitter_first().form())
            "data-on:change__debounce.1s"=[autosave]
        {
            // The revision this form was rendered from; absent with no stored draft.
            @if let Some(baseline) = view.baseline {
                input type="hidden" name=(BASELINE) value=(baseline);
            }
            div class="flex flex-col gap-6 rounded-md border border-neutral-200 bg-white p-4" {
                h2 class="font-display text-xl" { (view.section.title) }
                // Depositor guidance: RDU is not told to send anything to itself.
                @if let (Some(intro), Audience::Everyone) = (view.section.intro, view.audience) {
                    p class="field-hint" { (intro) }
                }
                @for field in &fields {
                    // No obligation pill here: it is inside the field's own label, the only
                    // way it reaches a screen reader now that nothing is required.
                    ({
                        field_row(
                            field,
                            view.draft,
                            view.mode_of(field.id),
                            view.rows(&agents_path, &vocabulary_path),
                        )
                    })
                    @for message in view.errors_for(field.id) {
                        (alert(message).variant(AlertVariant::Warning))
                    }
                }
            }
            (controls(view))
        }
    }
}

/// A flash right after an action is rendered by [`status`], so none here. An
/// approved round outside the wait would promise a release that is not coming;
/// `Saved` is no flash here because it renders on every autosave and must not
/// hide a standing round note.
///
/// Precedence: the Submitted/Withdrawn/Discarded flash, else the locked box,
/// else a ChangesRequested/Rejected round when it is the latest, else the
/// approved box while a release is awaited.
fn persistent_box(view: &SectionView<'_>) -> Markup {
    if matches!(view.notice, Some(Notice::Submitted | Notice::Withdrawn | Notice::Discarded)) {
        return html! {};
    }
    if let Some(locked) = view.locked {
        return locked_box(locked, view.audience);
    }
    let round = view.round.as_ref();
    if let Some(round) =
        round.filter(|r| matches!(r.outcome, ReviewOutcome::ChangesRequested | ReviewOutcome::Rejected))
    {
        return review_round(round.outcome, Some(round), view.audience);
    }
    if view.awaiting_release {
        let approved = round.filter(|r| r.outcome == ReviewOutcome::Approved);
        return review_round(ReviewOutcome::Approved, approved, view.audience);
    }
    html! {}
}

fn locked_box(locked: Locked, audience: Audience) -> Markup {
    match audience {
        Audience::Everyone => html! {
            ({
                alert(locked.message())
                    .variant(AlertVariant::Info)
                    .title(locked.heading())
            })
        },
        Audience::RduOnly => html! {
            ({
                alert(rdu_locked_body(locked))
                    .variant(AlertVariant::Info)
                    .title(locked.heading())
            })
        },
    }
}

fn rdu_locked_body(locked: Locked) -> Markup {
    let state = match locked {
        Locked::Submitted => editor_core::status::ProjectState::Submitted,
        Locked::InReview => editor_core::status::ProjectState::InReview,
    };
    html! {
        (state.explanation_for(Audience::RduOnly))
        " "
        (review_queue_link())
    }
}

/// `round` adds the date, RDU's note and any values RDU put in place of the
/// depositor's own; it is `None` for an approval waiting on a release whose latest
/// round is a later one.
fn review_round(outcome: ReviewOutcome, round: Option<&RoundSummary<'_>>, audience: Audience) -> Markup {
    let (heading, meaning, next) = round_wording(outcome, audience);
    html! {
        ({
            let body = html! {
                p { (meaning) } @ if let Some(next) = next { p { (next) } } @ if let
                Some(round) = round { (round_details(round)) }
            };
            alert(body).variant(round_variant(outcome)).title(heading)
        })
    }
}

fn round_details(round: &RoundSummary<'_>) -> Markup {
    html! {
        p class="text-sm mt-2" { (round.at) }
        @if let Some(note) = round.note {
            div class="mt-3" {
                p class="text-xs font-bold uppercase tracking-wide" { "What RDU wrote" }
                p class="whitespace-pre-line" { (note) }
            }
        }
        @if !round.substitutions.is_empty() {
            div class="mt-3" {
                p class="text-xs font-bold uppercase tracking-wide" {
                    "Values RDU changed before deciding"
                }
                ul class="flex flex-col gap-2 mt-1" {
                    @for (field, value) in round.substitutions {
                        li {
                            strong {
                                ({
                                    editor_core::form::registry::field(field)
                                        .map_or(field.as_str(), |f| f.label)
                                })
                            }
                            ": "
                            (crate::form::widgets::value_markup(Some(value)))
                        }
                    }
                }
            }
        }
    }
}

/// "N values need fixing", the count the saved notice and its title prefix share.
fn needs_fixing(count: usize) -> String {
    match count {
        1 => "1 value needs fixing".to_string(),
        count => format!("{count} values need fixing"),
    }
}

/// The saved notice. A warning, not a success, while the draft holds a value submit would
/// refuse: "Draft saved." beside a field's format error reads as all being well. Lists every
/// such value, this section's too, each a link to its control, as an error summary does. A line
/// with no language was not stored, so it is named in a sentence, not listed.
fn saved_notice(view: &SectionView<'_>) -> Markup {
    let count = view.values_to_fix();
    if count == 0 {
        return html! {
            (alert(format!("{SAVED_TITLE}.")).variant(AlertVariant::Success))
        };
    }
    let here = view.format_findings.iter().map(|finding| (view.section, finding));
    let elsewhere = view.findings_elsewhere.iter().map(|(section, finding)| (*section, finding));
    let body = html! {
        @if !view.without_language.is_empty() {
            p { "A line has text but no language: it was not saved and is marked below." }
        }
        p { (needs_fixing(count)) " before you can submit for review." }
        @if count > view.without_language.len() {
            ul class="flex flex-col gap-2" {
                @for (section, finding) in here.chain(elsewhere) {
                    li {
                        a href=(view.finding_href(section, finding)) class="underline font-bold" {
                            (finding.field.label)
                        }
                        " ("
                        (section.title)
                        "): "
                        (finding_message(finding))
                    }
                }
            }
        }
    };
    html! {
        (alert(body).variant(AlertVariant::Warning).title(SAVED_TITLE))
    }
}

/// Errors about fields another section shows, each a link to that section: the
/// reader has no other way to know which of six sections holds the field.
fn errors_elsewhere(view: &SectionView<'_>) -> Markup {
    let elsewhere = view.errors_elsewhere();
    if elsewhere.is_empty() {
        return html! {};
    }
    html! {
        ({
            let body = html! {
                ul class = "flex flex-col gap-2" { @ for (section, label, message) in &
                elsewhere { li { a href = { "/projects/"(view.shortcode)
                "/sections/"(section.id) } class = "underline font-bold" { (label) }
                " ("(section.title) "): "(message) } } }
            };
            alert(body)
                .variant(AlertVariant::Warning)
                .title("Fields in other sections need changing")
        })
    }
}

/// The link onward identifies the proposal: a new one is not named by an id, since it has no
/// name yet.
fn proposed_notice(view: &SectionView<'_>, kind: ProposalKind, entity_id: &str) -> Markup {
    let lead = format!("A new {} has been started.", kind.label().to_lowercase());
    let body = html! {
        p { (lead) }
        p class="mt-2" {
            a   href=({
                    entity_url(
                        view.shortcode,
                        entity_id,
                        Some(Origin::section(view.section)),
                    )
                })
                class="underline"
            { "Open it to fill in the details" }
            "."
        }
    };
    html! {
        (alert(body).variant(AlertVariant::Success).title("Proposal started"))
    }
}

/// Findings against this project's own live proposals, each linked to its entity
/// form. Parallel to [`errors_elsewhere`] rather than shared: a proposal is not a
/// registry field, so `section_of` would drop every one of these.
fn proposal_findings_elsewhere(view: &SectionView<'_>) -> Markup {
    if view.proposal_findings.is_empty() {
        return html! {};
    }
    let body = proposal_findings_list(view);
    html! {
        ({
            alert(body)
                .variant(AlertVariant::Warning)
                .title("A person or organisation you started needs finishing")
        })
    }
}

/// The list inside [`proposal_findings_elsewhere`]'s alert. Its own function per
/// the repo's rule on nested `html!` (`areas/access/dpe/CLAUDE.md`): `maudfmt` skips a
/// block passed as a call argument and `cargo fmt` then flattens it, with `just
/// check` green because it only verifies `maudfmt` is a no-op.
fn proposal_findings_list(view: &SectionView<'_>) -> Markup {
    let labels = entity_labels(
        view.proposal_findings.iter().map(|(id, kind, _)| (*kind, id.as_str())),
        view.agents,
        Precedence::Proposed,
    );
    html! {
        ul class="flex flex-col gap-2" {
            @for ((entity_id, _, message), label) in view.proposal_findings.iter().zip(labels) {
                li {
                    a   href={ "/projects/" (view.shortcode) "/entities/" (entity_id) }
                        class="underline font-bold"
                    { (label) }
                    ": "
                    (message)
                }
            }
        }
    }
}

/// This project's own live proposals, each resolved to a label where its payload
/// gives one, linked to its entity form. Gated on
/// [`SectionView::has_agent_picker_field`], since a proposal exists to be
/// referenced from an agent field; renders nothing with no live proposals,
/// because an empty panel reads as broken.
fn proposals_summary(view: &SectionView<'_>) -> Markup {
    if !view.has_agent_picker_field() {
        return html! {};
    }
    let live: Vec<&EntityProposal> = view.proposals.iter().filter(|proposal| proposal.is_live()).collect();
    if live.is_empty() {
        return html! {};
    }
    let labels = entity_labels(
        live.iter().map(|proposal| (proposal.kind, proposal.entity_id.as_str())),
        view.agents,
        Precedence::Proposed,
    );
    html! {
        div class="rounded border border-neutral-300 bg-white p-4 mb-6" {
            // h2, not h3: this panel renders above the section form, whose title is
            // an h2, and an h3 here breaks the outline for a reader navigating by
            // level.
            h2 class="font-display text-base mb-2" { "Proposed persons and organisations" }
            ul class="flex flex-col gap-1" {
                @for (proposal, label) in live.iter().zip(labels) {
                    li {
                        a   href=({
                                entity_url(
                                    view.shortcode,
                                    &proposal.entity_id,
                                    Some(Origin::section(view.section)),
                                )
                            })
                            class="underline font-bold"
                        { (label) }
                    }
                }
            }
        }
    }
}

/// What a reader is told shortly before their sign-in ends.
fn sign_out_notice(at: &str) -> Markup {
    let body = html! {
        p { "You will be signed out at " (at) ". Save your draft before then." }
    };
    let built = alert(body).variant(AlertVariant::Warning).title("This sign-in is about to end");
    html! {
        (built)
    }
}

/// What a reader is told when somebody else saved the draft underneath them. A
/// named function per the repo's rule on nested `html!` (`areas/access/dpe/CLAUDE.md`).
fn changed_notice(by: Option<&str>, at: &str) -> Markup {
    let body = html! {
        p {
            @match by {
                Some(name) => { (name) " saved this draft at " (at) ", after this form was opened." }
                None => { "Somebody else saved this draft at " (at) ", after this form was opened." }
            }
            " Nothing has been saved just now."
        }
        p class="mt-2" {
            "What is on this page is still what you typed. Saving again keeps your version and replaces "
            "theirs, so it is worth checking with them first."
        }
    };
    let built = alert(body)
        .variant(AlertVariant::Warning)
        .title("The draft changed while you were editing");
    html! {
        (built)
    }
}

/// The notice after a discard. A created project has no published metadata to return to: it
/// re-opens with what its creation derived.
fn discarded_notice(created: bool) -> Markup {
    let message = if created {
        "The draft has been discarded. This form is showing the project with only its shortcode and \
         identifiers, and nothing you save from here is submitted until you say so."
    } else {
        "The draft has been discarded. This form is showing the project's published metadata again, and \
         nothing you save from here is submitted until you say so."
    };
    html! {
        (alert(message).variant(AlertVariant::Success).title(DISCARDED_TITLE))
    }
}

/// What the discard confirmation says will happen.
const fn discard_consequence(created: bool) -> &'static str {
    if created {
        "Discarding the draft removes everything entered since the project was created, its name included. It \
         cannot be undone. The project itself is kept, with its shortcode and identifiers."
    } else {
        "Discarding the draft removes every change that has not been submitted. It cannot be undone. The \
         project itself is not affected — the form re-opens showing its published metadata."
    }
}

/// A question before an irreversible action: a group named by its heading and described by
/// `consequence`, so both are announced on entry, with focus on the control carrying `autofocus`
/// (the safe choice), on a page load and where the enhanced path morphs the panel in.
fn confirmation(heading: &str, consequence: &str, choices: Markup) -> Markup {
    let panel = html! {
        h3 id="confirm-heading" class="font-bold" { (heading) }
        p id="confirm-consequence" { (consequence) }
        div class="mt-2 flex items-center gap-4" { (choices) }
    };
    html! {
        div role="group"
            aria-labelledby="confirm-heading"
            aria-describedby="confirm-consequence"
            data-init=(FOCUS_AUTOFOCUS)
        { (impact_card(panel)) }
    }
}

/// The inline question before discarding the draft or taking the submission back.
fn confirmation_of(view: &SectionView<'_>, confirming: Confirmation) -> Markup {
    let (heading, consequence, confirm, intent, keep) = match confirming {
        Confirmation::Discard => (
            "Discard the draft?",
            discard_consequence(view.created),
            "Yes, discard the draft",
            DISCARD,
            "Keep the draft",
        ),
        Confirmation::Withdrawal => (
            "Take the submission back?",
            "Taking the submission back removes it from RDU's queue, along with anything a reviewer has already \
             recorded on it. Your draft is kept, so you can keep editing and submit again.",
            "Yes, take it back",
            WITHDRAW,
            "Keep waiting",
        ),
    };
    let choices = html! {
        ({
            button(confirm)
                .variant(ButtonVariant::Danger)
                .button_type(ButtonType::Submit)
                .name_value(INTENT, intent)
        })
        (link(keep, view.action()).autofocus())
    };
    confirmation(heading, consequence, choices)
}

/// The write controls, which depend on where the project sits in the cycle.
/// Every one is a named submit on the same form: a native submit posts the
/// activated button's name and value and Datastar's form mode appends them from
/// `SubmitEvent.submitter`, while a `formaction` would be silently ignored on the
/// enhanced path.
fn controls(view: &SectionView<'_>) -> Markup {
    if let Some(confirming) = view.confirming {
        return confirmation_of(view, confirming);
    }
    let editable = view.section.has_editable_field(view.audience);
    html! {
        @if view.locked.is_some() {
            @if view.may_withdraw {
                div class=(CONTROL_ROW) {
                    ({
                        button("Take the submission back")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, WITHDRAW_CONFIRM)
                    })
                }
            }
        } @else if editable {
            div class=(CONTROL_ROW) {
                ({
                    button("Save draft")
                        .variant(ButtonVariant::Secondary)
                        .button_type(ButtonType::Submit)
                        .name_value(INTENT, SAVE)
                })
                ({
                    button("Submit for review")
                        .button_type(ButtonType::Submit)
                        .name_value(INTENT, SUBMIT)
                })
                @if view.may_discard {
                    ({
                        button("Discard draft")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, DISCARD_CONFIRM)
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_core::agents::{AgentScope, Agents};
    use editor_core::draft::ProjectDraft;
    use editor_core::form::registry::section;
    use editor_core::proposals::ProposalOperation;
    use serde_json::json;

    use super::*;

    /// The committed agent set, loaded once for the whole test binary.
    fn published_agents() -> &'static Agents {
        static AGENTS: std::sync::OnceLock<Agents> = std::sync::OnceLock::new();
        AGENTS.get_or_init(|| {
            let dir = editor_core::checkout_dpe_data_dir();
            let (agents, errors) = Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
            assert!(errors.is_empty(), "the committed agent set should load: {errors:?}");
            agents
        })
    }

    /// The committed set with no proposals, for a fixture with no project in hand.
    pub(super) fn agent_corpus() -> &'static AgentScope<'static> {
        static SCOPE: std::sync::OnceLock<AgentScope<'static>> = std::sync::OnceLock::new();
        SCOPE.get_or_init(|| AgentScope::published_only(published_agents()))
    }

    #[test]
    fn an_add_agent_intent_round_trips_and_a_garbled_one_is_nothing() {
        let intent = add_agent_intent("attributions", "person-001");
        assert_eq!(intent, "add-agent:attributions:person-001");
        assert_eq!(added_agent(&intent), Some(("attributions", "person-001")));
        assert_eq!(added_agent("add-agent:attributions:"), None);
        assert_eq!(added_agent("add-agent::person-001"), None);
        assert_eq!(added_agent("add-agent:attributions"), None);
        assert_eq!(added_agent("choose-agent:attributions:person-001"), None);
    }

    #[test]
    fn a_change_agent_intent_round_trips_and_a_garbled_one_is_nothing() {
        let intent = change_agent_intent("attributions.r0.contributor");
        assert_eq!(intent, "change-agent:attributions.r0.contributor");
        assert_eq!(changed_agent(&intent), Some("attributions.r0.contributor"));
        assert_eq!(changed_agent("change-agent:"), None);
        assert_eq!(changed_agent("change-agent"), None);
        assert_eq!(changed_agent("add-agent:attributions:person-001"), None);
    }

    #[test]
    fn a_remove_role_intent_round_trips_and_a_garbled_one_is_nothing() {
        let intent = remove_role_intent("attributions.r0", "Author");
        assert_eq!(intent, "remove-role:attributions.r0:Author");
        assert_eq!(removed_role(&intent), Some(("attributions.r0", "Author")));
        // A custom role may hold a colon: the split is on the first one after the prefix.
        let custom = remove_role_intent("attributions.r0", "Editor: volume 2");
        assert_eq!(removed_role(&custom), Some(("attributions.r0", "Editor: volume 2")));
        assert_eq!(removed_role("remove-role::Author"), None);
        assert_eq!(removed_role("remove-role:attributions.r0:"), None);
        assert_eq!(removed_role("remove-role:attributions.r0"), None);
        assert_eq!(removed_role("change-agent:attributions.r0"), None);
    }

    /// A draft over a real committed project, so the fields under test hold what the corpus
    /// actually holds rather than what a fixture assumes.
    pub(super) fn published_draft() -> ProjectDraft {
        let dir = editor_core::checkout_dpe_data_dir().join("projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    pub(super) fn view<'a>(draft: &'a ProjectDraft, section_id: &str, audience: Audience) -> SectionView<'a> {
        SectionView {
            // The real committed set, so a snapshot shows resolved names and
            // the suggestion list a depositor actually gets.
            agents: Some(agent_corpus()),
            posted: None,
            format_findings: Vec::new(),
            findings_elsewhere: Vec::new(),
            submitted: None,
            adding_row: None,
            focus_finding: false,
            without_language: Vec::new(),
            rows_action: format!("/projects/0801d/sections/{section_id}/fields"),
            awaiting_release: false,
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
            created: false,
            state: ProjectState::Draft,
            section: section(section_id).expect("a known section"),
            audience,
            draft,
            locked: None,
            accepted_fields: &[],
            may_withdraw: false,
            may_discard: false,
            confirming: None,
            last_editor: None,
            signed_out_at: None,
            baseline: None,
            errors: &[],
            proposal_findings: &[],
            proposals: &[],
            round: None,
            saved_at: None,
            notice: None,
        }
    }

    fn overview(draft: &ProjectDraft) -> String {
        page(&view(draft, "overview", Audience::Everyone)).into_string()
    }

    #[test]
    fn an_error_about_another_section_announces_with_the_refusal_that_names_it() {
        // The refusal is what gets announced, so the list of fields it is about has
        // to be inside the aria-live region with it. Asserted on status alone, the
        // region itself.
        let draft = published_draft();
        let errors = [(
            "temporalCoverage[0]".to_string(),
            "cannot be matched to a date range".to_string(),
        )];
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.errors = &errors;
        view.notice = Some(Notice::Refused("This project cannot be submitted yet."));

        let region = status(&view).into_string();
        assert!(region.contains(r#"aria-live="polite""#), "{region}");
        assert!(region.contains("cannot be matched to a date range"), "{region}");
        // And it names the section that holds the field, as a link.
        assert!(region.contains("/projects/0801d/sections/dataset"), "{region}");
        assert!(
            region.contains("Temporal coverage"),
            "the field's label, not its member name: {region}"
        );
    }

    /// The opening tag of the `.field-label` element holding `label`.
    fn label_tag<'a>(page: &'a str, label: &str) -> &'a str {
        page.match_indices(r#"class="field-label"#)
            .chain(page.match_indices(r#"class="field-label sr-only"#))
            .find_map(|(at, _)| {
                let end = at + page[at..].find('>')?;
                page[end + 1..].starts_with(label).then(|| &page[at..end])
            })
            .unwrap_or_else(|| panic!("no {label:?} field label: {page}"))
    }

    /// Whether `label` is the opening content of an `sr-only` element.
    fn label_is_screen_reader_only(page: &str, label: &str) -> bool {
        page.match_indices(label).any(|(at, _)| {
            let tag = &page[page[..at].rfind('<').unwrap_or(0)..at];
            tag.ends_with('>') && tag.contains("sr-only")
        })
    }

    #[test]
    fn a_section_whose_only_field_repeats_its_heading_hides_the_label_for_sighted_readers_only() {
        let draft = published_draft();
        let out = page(&view(&draft, "publications", Audience::Everyone)).into_string();
        let fieldset = out
            .find(r#"<fieldset class="field field-group" id="publications""#)
            .expect("the list");
        let legend = out[fieldset..].find("<legend").expect("a legend") + fieldset;
        assert!(
            out[legend..].starts_with(r#"<legend class="field-label"><span class="sr-only">Publications</span>"#),
            "{out}"
        );
        // Still the fieldset's first child, so the group keeps its name.
        let opening_end = out[fieldset..].find('>').expect("end of the fieldset tag") + fieldset + 1;
        assert_eq!(legend, opening_end, "{out}");
    }

    #[test]
    fn the_obligation_pill_stays_in_view_when_the_label_is_hidden() {
        let draft = published_draft();
        let out = page(&view(&draft, "publications", Audience::Everyone)).into_string();
        let legend = out.find("<legend").expect("a legend");
        let legend = &out[legend..legend + out[legend..].find("</legend>").expect("a closed legend")];
        let pill = legend.find("Optional").expect("the pill in the legend");
        let hidden_end = legend.find("</span>").expect("the hidden name closes");
        assert_eq!(legend.matches("sr-only").count(), 1, "{legend}");
        assert!(hidden_end < pill, "the pill sits outside the hidden name: {legend}");
    }

    #[test]
    fn a_field_rdu_accepted_keeps_its_pill_in_view_under_its_heading_too() {
        let draft = published_draft();
        let accepted = ["publications".to_string()];
        let mut view = view(&draft, "publications", Audience::Everyone);
        view.accepted_fields = &accepted;
        let out = page(&view).into_string();
        let label = out.find(r#"id="publications-label""#).expect("the stated label");
        let label = &out[label..label + out[label..].find("</p>").expect("a closed label")];
        assert!(label.contains(r#"<span class="sr-only">Publications</span>"#), "{label}");
        assert!(label.contains("Optional") && !label.contains(r#"sr-only">Optional"#), "{label}");
    }

    #[test]
    fn a_display_only_section_titled_like_its_field_hides_that_label_too() {
        let draft = published_draft();
        let out = page(&view(&draft, "legal", Audience::RduOnly)).into_string();
        assert!(out.contains(r#"<p class="sr-only" id="legalInfo-label">Legal info"#), "{out}");
        assert!(out.contains(r#"aria-labelledby="legalInfo-label""#), "{out}");
    }

    #[test]
    fn a_label_that_differs_from_the_heading_or_shares_it_with_others_stays_visible() {
        let draft = published_draft();
        let image = page(&view(&draft, "image", Audience::Everyone)).into_string();
        assert!(!label_tag(&image, "Project image credit").contains("sr-only"), "{image}");
        let contributors = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(
            !contributors.contains(r#"<legend class="field-label sr-only""#),
            "{contributors}"
        );
    }

    #[test]
    fn a_section_intro_shows_to_depositors_only_and_above_the_fields() {
        let draft = published_draft();
        for section in editor_core::form::registry::SECTIONS {
            let out = page(&view(&draft, section.id, Audience::Everyone)).into_string();
            // Maud does not escape `'`, so the whole sentence matches as written.
            match section.intro {
                Some(intro) => assert!(out.contains(intro), "{} lacks its intro: {out}", section.id),
                None => assert!(
                    !out.contains(r#"<p class="field-hint mb-4">"#),
                    "{} has no intro yet renders one: {out}",
                    section.id
                ),
            }
        }
        let image = page(&view(&draft, "image", Audience::Everyone)).into_string();
        let notice = image
            .find("Image upload isn")
            .unwrap_or_else(|| panic!("the image page carries the notice: {image}"));
        let credit = image.find("Project image credit").expect("the credit field is on the page");
        assert!(notice < credit, "the notice comes before the credit field: {image}");
        let intro = editor_core::form::registry::SECTIONS
            .iter()
            .find(|section| section.id == "image")
            .and_then(|section| section.intro)
            .expect("the image section has an intro");
        let rdu = page(&view(&draft, "image", Audience::RduOnly)).into_string();
        assert!(!rdu.contains(intro), "RDU is not shown depositor guidance: {rdu}");
    }

    #[test]
    fn every_section_whose_single_field_repeats_its_title_renders_that_label_screen_reader_only() {
        // Sweeps every section of this shape, so one whose widget ignores `label_hidden` fails
        // here.
        let draft = published_draft();
        let mut checked = 0;
        for audience in [Audience::Everyone, Audience::RduOnly] {
            for section in editor_core::form::registry::SECTIONS {
                let fields: Vec<_> = section.fields_for(audience).collect();
                let [field] = fields[..] else { continue };
                if field.label != section.title {
                    continue;
                }
                checked += 1;
                let out = page(&view(&draft, section.id, audience)).into_string();
                assert!(
                    label_is_screen_reader_only(&out, field.label),
                    "{} ({audience:?}) renders its label visibly under a heading of the same words: \
                     the widget for shape {:?} must honour `Rows::label_hidden` (keep the name \
                     `sr-only`, leave the obligation pill in view): {out}",
                    section.id,
                    field.shape
                );
            }
        }
        assert!(checked >= 2, "Publications and Legal info are such sections");
    }

    #[test]
    fn an_error_about_a_field_on_this_page_is_not_listed_as_elsewhere() {
        // Exclusive renderings: an error beside its control is not also listed as
        // elsewhere.
        let draft = published_draft();
        let errors = [("name".to_string(), "must not be empty".to_string())];
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.errors = &errors;

        assert!(!status(&view).into_string().contains("must not be empty"));
        assert!(
            region(&view).into_string().contains("must not be empty"),
            "it renders by the control"
        );
    }

    #[test]
    fn no_section_renders_the_same_element_id_twice() {
        // A duplicate id silently breaks every label-for and aria-describedby
        // pointing at it. Easy to produce here because several controls share a
        // name and the tiles default an id to its name. Over every section and
        // both audiences, on a project with data in the repeatable fields.
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008", "person-001"]));
        draft.set(
            "additionalMaterial",
            serde_json::json!(["https://one.example/", "https://two.example/"]),
        );
        draft.set("publications", serde_json::json!([{ "text": "One" }, { "text": "Two" }]));
        draft.set(
            "funding",
            serde_json::json!([{ "funders": ["organization-002", "organization-003"], "number": "1" }]),
        );

        let mut clashes: Vec<String> = Vec::new();
        for audience in [Audience::Everyone, Audience::RduOnly] {
            for section in sections_for(audience) {
                let out = page(&view(&draft, section.id, audience)).into_string();
                let mut seen: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
                for id in out.split(r#" id=""#).skip(1).filter_map(|rest| rest.split('"').next()) {
                    *seen.entry(id).or_default() += 1;
                }
                for (id, count) in seen.iter().filter(|(_, count)| **count > 1) {
                    clashes.push(format!("{} ({audience:?}): {id} x{count}", section.id));
                }
            }
        }
        assert!(clashes.is_empty(), "duplicate element ids: {clashes:?}");
    }

    /// The opening tags in `out` that carry a `data-on:input` trigger. Maud escapes `<` inside an
    /// attribute value, so splitting on it finds only tags.
    fn input_triggers(out: &str) -> Vec<&str> {
        out.split('<')
            .filter(|tag| tag.contains("data-on:input"))
            .map(|tag| tag.split('>').next().unwrap_or(tag))
            .collect()
    }

    #[test]
    fn an_editable_form_autosaves_on_change_and_a_locked_one_does_not() {
        // change rather than input: the response patches the whole region, so a
        // mid-keystroke trigger would patch the field being typed into. The one
        // keystroke triggers are a combobox's search, which reads and morphs only its
        // own results, and a length counter, which only sets a local signal.
        // Nothing on a read-only form.
        let draft = published_draft();
        let overview = page(&view(&draft, "overview", Audience::Everyone)).into_string();
        assert!(overview.contains("data-on:change__debounce.1s"), "{overview}");
        let contributors = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(contributors.contains("data-on:change__debounce.1s"), "{contributors}");
        // A combobox's change never autosaves; every other control's does.
        assert!(
            contributors
                .contains("data-on:change__debounce.1s=\"!evt.target.matches('[role=combobox]') &amp;&amp; @post("),
            "{contributors}"
        );

        for out in [&overview, &contributors] {
            for tag in input_triggers(out) {
                if tag.contains("data-signals=\"{_count_") {
                    assert!(tag.contains("data-on:input=\"$_count_"), "<{tag}>");
                    assert!(tag.contains("data-on:input__debounce.500ms=\"$_announce_"), "<{tag}>");
                    for action in ["@post(", "@get(", "@put(", "@patch(", "@delete("] {
                        assert!(!tag.contains(action), "<{tag}>");
                    }
                    continue;
                }
                let name = tag.split("name=\"").nth(1).and_then(|rest| rest.split('"').next());
                assert!(
                    tag.contains(r#"role="combobox""#) && name.is_some_and(|name| name.ends_with(".q")),
                    "only a combobox search types ahead, never the form or a field control: <{tag}>"
                );
                assert!(tag.contains("data-on:input__debounce.300ms=\"@get("), "and it reads: <{tag}>");
                assert!(!tag.contains("@post("), "<{tag}>");
            }
        }
        assert!(
            input_triggers(&overview)
                .iter()
                .all(|tag| tag.contains("data-signals=\"{_count_")),
            "{overview}"
        );
        assert!(!input_triggers(&contributors).is_empty(), "{contributors}");

        let mut locked = view(&draft, "overview", Audience::Everyone);
        locked.locked = Some(Locked::Submitted);
        let out = page(&locked).into_string();
        assert!(!out.contains("data-on:change"), "{out}");
        assert!(!out.contains("data-on:input"), "{out}");
    }

    #[test]
    fn the_form_warns_about_a_sign_out_only_when_one_is_near() {
        // A warning permanently on screen is a warning nobody reads, which is
        // why the deadline reaches the view already filtered.
        let draft = published_draft();
        let quiet = page(&view(&draft, "overview", Audience::Everyone)).into_string();
        assert!(!quiet.contains("about to end"), "{quiet}");

        let mut soon = view(&draft, "overview", Audience::Everyone);
        soon.signed_out_at = Some("2026-09-09 08:15 UTC");
        let out = page(&soon).into_string();
        assert!(out.contains("This sign-in is about to end"), "{out}");
        assert!(out.contains("2026-09-09 08:15 UTC"), "{out}");
        assert!(out.contains("Save your draft before then"), "the action is named: {out}");
    }

    #[test]
    fn the_form_posts_to_the_url_it_was_fetched_from() {
        // A write posting to a path with no GET strands a rejected save on a 405.
        let out = overview(&published_draft());
        assert!(out.contains(r#"action="/projects/0801d/sections/overview""#), "{out}");
        assert!(out.contains(r#"method="post""#), "{out}");
    }

    #[test]
    fn the_enhanced_path_posts_the_form_body_rather_than_signals() {
        let out = overview(&published_draft());
        assert!(
            out.contains(
                r#"data-on:submit="@post(evt.submitter?.formAction || '/projects/0801d/sections/overview', {contentType: 'form'})""#
            ),
            "{out}"
        );
        assert!(!out.contains("submit__prevent"), "{out}");
        // Keyed plugin attributes use `:`; the hyphen form is inert.
        assert!(!out.contains("data-on-submit"), "{out}");
    }

    #[test]
    fn the_enhanced_path_posts_a_row_action_to_its_own_url_and_not_to_the_form_s() {
        // preventDefault on the form's submit throws the submitter's formaction away,
        // so the expression reads it back off the submitter; the literal action is
        // the fallback for a submission with no submitter.
        let out = overview(&published_draft());
        assert!(out.contains("evt.submitter?.formAction"), "{out}");
        // And the row controls still carry the URL it reads.
        let dataset = page(&view(&published_draft(), "dataset", Audience::Everyone)).into_string();
        assert!(
            dataset.contains(r#"formaction="/projects/0801d/sections/dataset/fields/keywords/add""#),
            "{dataset}"
        );
    }

    #[test]
    fn a_display_only_field_renders_its_value_and_no_control() {
        // A control here would post, and an empty one would clear a value the
        // reader could never change.
        let out = page(&view(&published_draft(), "overview", Audience::RduOnly)).into_string();
        assert!(out.contains("Shortcode"), "{out}");
        assert!(!out.contains(r#"name="shortcode""#), "{out}");
        assert!(!out.contains(r#"name="pid""#), "{out}");
        assert!(!out.contains(r#"name="id""#), "{out}");
    }

    #[test]
    fn a_section_with_nothing_editable_has_no_write_buttons() {
        let draft = published_draft();
        let legal = page(&view(&draft, "legal", Audience::RduOnly)).into_string();
        assert!(legal.contains("CC BY-NC-SA 4.0"), "{legal}");
        for button in ["Save draft", "Submit for review", "Discard draft"] {
            assert!(!legal.contains(button), "{button} in {legal}");
        }

        let overview = page(&view(&draft, "overview", Audience::RduOnly)).into_string();
        assert!(overview.contains("Save draft"), "{overview}");
        assert!(overview.contains("Submit for review"), "{overview}");
    }

    #[test]
    fn a_locked_section_with_nothing_editable_still_offers_the_withdrawal() {
        let draft = published_draft();
        let mut locked = view(&draft, "legal", Audience::RduOnly);
        locked.locked = Some(Locked::Submitted);
        locked.may_withdraw = true;
        let out = page(&locked).into_string();
        assert!(out.contains("Take the submission back"), "{out}");
        assert!(!out.contains("Save draft"), "{out}");

        locked.confirming = Some(Confirmation::Withdrawal);
        let confirming = page(&locked).into_string();
        assert!(confirming.contains("Yes, take it back"), "{confirming}");
        assert!(!confirming.contains("Save draft"), "{confirming}");
    }

    #[test]
    fn the_control_row_buttons_post_with_the_form() {
        let draft = published_draft();
        let inside_form = |html: &str, label: &str| {
            let start = html.find(r#"id="section-form""#).expect("form");
            let end = start + html[start..].find("</form>").expect("form end");
            html[start..end].contains(label)
        };

        let editable = page(&view(&draft, "overview", Audience::RduOnly)).into_string();
        for label in ["Save draft", "Submit for review"] {
            assert!(inside_form(&editable, label), "{label} is outside the form");
        }

        let mut locked = view(&draft, "legal", Audience::RduOnly);
        locked.locked = Some(Locked::Submitted);
        locked.may_withdraw = true;
        let locked = page(&locked).into_string();
        assert!(inside_form(&locked, "Take the submission back"), "outside the form");
    }

    #[test]
    fn no_page_ships_the_whole_agent_set_and_every_picker_can_be_searched() {
        // Nothing may ship the agent set as a <datalist>: a page carries at most one,
        // the language names, and only the dataset section has it. The reasons against
        // an agent one (the set's size, and filtering a box that already holds an id)
        // do not apply to an empty add box or a fixed list of language names.
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        for audience in [Audience::Everyone, Audience::RduOnly] {
            for section in sections_for(audience) {
                let out = page(&view(&draft, section.id, audience)).into_string();
                let datalists = out.matches("<datalist").count();
                assert!(datalists <= 1, "{} ({audience:?}) ships {datalists} datalists", section.id);
                let language_names = out.contains(r#"<datalist id="data-language-names">"#);
                assert_eq!(
                    datalists == 1,
                    section.id == "dataset",
                    "{} ({audience:?}): a datalist belongs on the dataset section only",
                    section.id
                );
                assert!(
                    datalists == 0 || language_names,
                    "{} ({audience:?}): not the language one",
                    section.id
                );
                assert!(!out.contains("list=\"agent-suggestions\""), "{} ({audience:?})", section.id);
            }
        }

        // On a section that has an agent field, a filled slot is a card that opens its search, each
        // list has one append search, and the id rides in a hidden input, which is what keeps an
        // untouched save byte-exact.
        let contributors = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(
            contributors.contains(r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#),
            "the id posts from a hidden input: {contributors}"
        );
        assert!(
            contributors.contains(r#"value="change-agent:contactPoint.r0""#),
            "the card offers the search: {contributors}"
        );
        for list in ["attributions", "contactPoint"] {
            assert!(
                contributors.contains(&format!(r#"name="{list}.new.q""#)),
                "{list} offers an append search: {contributors}"
            );
        }
        assert!(
            contributors.contains(r#"value="find-agent""#),
            "and a control to run it: {contributors}"
        );
        assert!(
            contributors
                .contains("data-on:input__debounce.300ms=\"@get('/projects/0801d/sections/contributors/agents?for="),
            "the section runs the search as the box is typed in: {contributors}"
        );

        let image = page(&view(&draft, "image", Audience::Everyone)).into_string();
        assert!(!image.contains("find-agent"), "no agent field, no picker: {image}");
    }

    #[test]
    fn an_agent_row_shows_the_resolved_name_beside_the_id() {
        // The input holds the id for the round trip, so the name is rendered beside
        // it.
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        let out = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(out.contains(r#"value="organization-008""#), "the id is what posts: {out}");
        assert!(out.contains("Dokumentationsbibliothek St. Moritz"), "the name is shown: {out}");
    }

    #[test]
    fn an_agent_id_that_resolves_to_nobody_says_so_in_the_form() {
        // Said here as well as at submit, because the form is where it can be fixed.
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["person-99999"]));
        let out = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(out.contains("No person or organisation with this id"), "{out}");
    }

    #[test]
    fn no_field_renders_the_not_editable_yet_note_any_more() {
        // The note is kept in widgets::stated as the fallback a new contract field
        // lands on; nothing may reach it today, or that field is silently
        // uneditable.
        let draft = published_draft();
        let mut reached: Vec<&str> = Vec::new();
        for section in sections_for(Audience::RduOnly) {
            let out = page(&view(&draft, section.id, Audience::RduOnly)).into_string();
            if out.contains("not editable here yet") {
                reached.push(section.id);
            }
        }
        assert!(
            reached.is_empty(),
            "sections still rendering the unbuilt-field note: {reached:?}"
        );
    }

    #[test]
    fn an_editable_field_renders_a_control_that_posts_even_when_empty() {
        // The other half of "absent is not empty": a section posts its own
        // fields whether or not they hold anything, so a cleared field arrives
        // present-and-empty rather than absent.
        let mut draft = published_draft();
        draft.remove("officialName");
        let out = overview(&draft);
        assert!(out.contains(r#"name="officialName""#), "{out}");
        assert!(out.contains(r#"name="name""#), "{out}");
    }

    #[test]
    fn a_placeholder_sentinel_renders_as_an_empty_control() {
        // MISSING is filtered out of DPE and OAI-PMH, so showing it here would present
        // an internal marker as a value the depositor has to delete by hand.
        let mut draft = published_draft();
        draft.set("endDate", json!("MISSING"));
        let out = overview(&draft);
        assert!(out.contains(r#"name="endDate""#), "{out}");
        assert!(!out.contains("MISSING"), "{out}");
    }

    #[test]
    fn a_language_map_renders_the_offered_languages_plus_any_the_value_carries() {
        // A closed set would drop ar on the first save: a tag with no control posts
        // nothing.
        let mut draft = published_draft();
        draft.set("description", json!({"en": "English text", "ar": "نص عربي"}));
        let out = overview(&draft);
        for tag in ["de", "en", "fr", "it", "ar"] {
            assert!(out.contains(&format!(r#"name="description.{tag}""#)), "{tag}: {out}");
        }
        // Named in words, not by code, for the tags the corpus actually uses.
        assert!(out.contains("Arabic"), "{out}");
        assert!(out.contains("English"), "{out}");
    }

    #[test]
    fn a_language_group_is_named_by_a_legend_because_each_control_is_a_language() {
        // A `<label for>` needs one control to point at, and this field has one
        // per language, each already labelled — so the field's own name can only
        // reach assistive technology as a `<legend>`.
        let out = overview(&published_draft());
        // The obligation pill rides inside the legend, so the group's accessible
        // name carries it too — see `a_required_field_says_so_inside_its_own_label`.
        assert!(out.contains(r#"<legend class="field-label">Description <span"#), "{out}");
        assert!(out.contains("</span></legend>"), "{out}");
    }

    #[test]
    fn the_rail_marks_the_current_section_in_more_than_a_colour() {
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-current="page""#), "{out}");
        assert_eq!(out.matches(r#"aria-current="page""#).count(), 1, "{out}");
        assert!(out.contains(r#"aria-label="Form sections""#), "{out}");
    }

    #[test]
    fn the_rail_states_both_numbers_so_a_finished_section_cannot_look_empty() {
        let draft = published_draft();
        let filled = overview(&draft);
        let empty = page(&view(&ProjectDraft::default(), "overview", Audience::Everyone)).into_string();
        // The two renderings must differ, which they cannot if only what is
        // outstanding is shown.
        let progress = section_progress(section("overview").expect("overview"), Audience::Everyone, &draft);
        assert!(filled.contains(&progress.summary()), "{filled}");
        assert!(empty.contains(&format!("0 of {} required", progress.required)), "{empty}");
    }

    #[test]
    fn the_round_and_locked_boxes_sit_in_the_rail_and_outside_the_live_region() {
        let draft = published_draft();
        let mut returned = view(&draft, "overview", Audience::Everyone);
        returned.round = Some(RoundSummary {
            outcome: ReviewOutcome::ChangesRequested,
            note: Some("Please add a description."),
            at: "1 January 2026",
            substitutions: &[],
        });
        let (heading, _, _) = round_wording(ReviewOutcome::ChangesRequested, Audience::Everyone);
        let mut locked = view(&draft, "overview", Audience::Everyone);
        locked.locked = Some(Locked::InReview);
        for (view, heading) in [(&returned, heading), (&locked, Locked::InReview.heading())] {
            assert!(rail(view).into_string().contains(heading), "{heading}");
            assert!(!status(view).into_string().contains(heading), "{heading}");
            assert!(!rail(view).into_string().contains("aria-live"), "{heading}");
            let out = page(view).into_string();
            assert_eq!(out.matches(heading).count(), 1, "{heading}: {out}");
            assert!(out.find(heading) < out.find(r#"aria-live="polite""#), "{heading}");
        }
    }

    #[test]
    fn the_saved_at_line_is_in_the_rail_and_not_in_the_controls() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.saved_at = Some("14:02");
        view.last_editor = Some("Daisy Duck");
        let rail = rail(&view).into_string();
        assert!(rail.contains("Draft last saved 14:02 by Daisy Duck."), "{rail}");
        assert!(!controls(&view).into_string().contains("Draft last saved"));
        view.locked = Some(Locked::Submitted);
        assert!(!self::rail(&view).into_string().contains("Draft last saved"));
    }

    #[test]
    fn the_chips_list_the_current_sections_missing_required_fields_and_link_to_them() {
        let empty = ProjectDraft::default();
        let view = view(&empty, "overview", Audience::Everyone);
        let missing = missing_here(&view);
        assert!(!missing.is_empty());
        let rail = rail(&view).into_string();
        assert!(rail.contains(r#"aria-label="Required fields not filled in""#), "{rail}");
        for field in &missing {
            assert!(rail.contains(&format!(r##"href="#{}""##, field.id)), "{}: {rail}", field.id);
        }
        // Another section's gaps are not this section's chips.
        let elsewhere = editor_core::form::obligation::unsatisfied_required(Audience::Everyone, &empty)
            .into_iter()
            .filter(|field| !missing.iter().any(|m| m.id == field.id));
        for field in elsewhere {
            assert!(!rail.contains(&format!(r##"href="#{}""##, field.id)), "{}", field.id);
        }
        // A locked form has no controls, so no chip links.
        let mut locked = self::view(&empty, "overview", Audience::Everyone);
        locked.locked = Some(Locked::Submitted);
        let rail = self::rail(&locked).into_string();
        assert!(rail.contains(r#"aria-label="Required fields not filled in""#), "{rail}");
        assert!(!rail.contains(r##"href="#"##), "{rail}");
        // A finished section shows none.
        let draft = published_draft();
        let done = self::rail(&self::view(&draft, "overview", Audience::Everyone)).into_string();
        assert!(!done.contains("Required fields not filled in"), "{done}");
    }

    #[test]
    fn every_chip_that_links_into_the_page_lands_on_an_element_there() {
        let empty = ProjectDraft::default();
        for audience in [Audience::Everyone, Audience::RduOnly] {
            for section in editor_core::form::registry::SECTIONS {
                let view = view(&empty, section.id, audience);
                let html = page(&view).into_string();
                for field in missing_here(&view) {
                    let href = format!(r##"href="#{}""##, field.id);
                    if html.contains(&href) {
                        assert!(
                            html.contains(&format!(r#"id="{}""#, field.id)),
                            "{}/{}: no target",
                            section.id,
                            field.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_depositor_s_rail_does_not_link_to_the_rdu_only_section() {
        // Present and empty would be a rail entry that goes nowhere.
        let draft = published_draft();
        let depositor = overview(&draft);
        assert!(!depositor.contains("/sections/legal"), "{depositor}");
        let rdu = page(&view(&draft, "overview", Audience::RduOnly)).into_string();
        assert!(rdu.contains("/sections/legal"), "{rdu}");
    }

    #[test]
    fn the_status_region_is_in_the_dom_before_there_is_anything_to_announce() {
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-live="polite""#), "{out}");
    }

    #[test]
    fn a_saved_notice_lands_inside_the_live_region() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Saved);
        let out = region(&view).into_string();
        let live = out.find(r#"aria-live="polite""#).expect("the live region");
        let saved = out.find("Draft saved.").expect("the notice");
        assert!(live < saved, "the notice must be inside the region, not before it: {out}");
    }

    #[test]
    fn a_save_that_dropped_a_line_without_a_language_is_not_announced_as_plain_success() {
        let draft = published_draft();
        let lines = vec![LineWithoutLanguage { line: 0, control: "keywords.r0.lang".to_string() }];
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.without_language = lines;
        view.notice = Some(Notice::Saved);
        let out = region(&view).into_string();
        assert!(out.contains("it was not saved and is marked below"), "{out}");
        assert!(out.contains("1 value needs fixing"), "{out}");
        assert!(!out.contains("Draft saved."), "the success alert says so: {out}");
        assert_eq!(view.title_prefix().as_deref(), Some("Draft saved, 1 value needs fixing"));
    }

    #[test]
    fn a_locked_project_renders_values_and_no_way_to_save() {
        // Nothing may change under a reviewer, and a save button that refuses is
        // worse than no button: the depositor presses it, waits, and is told no.
        let draft = published_draft();
        for locked in [Locked::Submitted, Locked::InReview] {
            let mut view = view(&draft, "overview", Audience::Everyone);
            view.locked = Some(locked);
            let out = page(&view).into_string();
            assert!(!out.contains(r#"name="name""#), "{locked:?}: {out}");
            assert!(!out.contains("Save draft"), "{locked:?}: {out}");
            assert!(out.contains(locked.heading()), "{locked:?}: {out}");
            // The value is still readable — a read-only form is the same form,
            // not a blank page.
            assert!(out.contains("Bernoulli"), "{locked:?}: {out}");
        }
    }

    #[test]
    fn the_two_locked_states_do_not_say_the_same_thing() {
        // Only one of them can expect the record back soon, and a depositor
        // deciding whether to wait or to ask needs to know which.
        assert_ne!(Locked::Submitted.message(), Locked::InReview.message());
        assert_ne!(Locked::Submitted.heading(), Locked::InReview.heading());
    }

    #[test]
    fn an_unpublished_project_opens_without_reading_as_a_failure() {
        // A blank form with no explanation reads as a page that failed to load.
        let draft = ProjectDraft::default();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.project_name = None;
        let out = page(&view).into_string();
        assert!(out.contains("Project 0801d"), "{out}");
        assert!(out.contains("nothing to pre-fill"), "{out}");
        // Still editable: a local-only project is a project.
        assert!(out.contains("Save draft"), "{out}");
    }

    #[test]
    fn the_save_control_says_whether_anything_has_been_saved_yet() {
        let draft = published_draft();
        let fresh = overview(&draft);
        assert!(fresh.contains("published metadata"), "{fresh}");

        let mut view = view(&draft, "overview", Audience::Everyone);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        let saved = page(&view).into_string();
        assert!(saved.contains("Draft last saved 2026-09-03 08:15 UTC."), "{saved}");
    }

    #[test]
    fn the_region_carries_the_id_the_enhanced_path_patches_and_the_rail_with_it() {
        // Datastar matches a text/html response by id in outer mode, so the id has
        // to be on the region's root, and the region has to include the rail.
        let out = region(&view(&published_draft(), "overview", Audience::Everyone)).into_string();
        assert!(out.starts_with(&format!(r#"<section id="{REGION_ID}""#)), "{out}");
        assert!(out.contains(r#"aria-label="Form sections""#), "{out}");
        assert!(out.contains("<form"), "{out}");
        // A fragment, not a document: patching `<html>` would replace the page.
        assert!(!out.contains("<!DOCTYPE"), "{out}");
    }

    #[test]
    fn no_field_is_required_or_the_browser_would_refuse_to_save_a_draft() {
        // required on the name field would make an unfinished draft unsaveable on
        // both paths: Datastar runs the same checkValidity() the browser does.
        let out = overview(&published_draft());
        // The *attribute*, not the word — "Required" is the obligation pill and
        // "5 of 5 required" is the rail, and both must stay.
        let with_attribute: Vec<&str> = out
            .split('<')
            .filter(|tag| tag.contains(" required") && (tag.starts_with("input") || tag.starts_with("textarea")))
            .collect();
        assert!(with_attribute.is_empty(), "{with_attribute:?}");
        assert!(out.contains("Required"), "the obligation is still stated in words: {out}");
        // Validation is still left on, deliberately: see the module docs.
        assert!(!out.contains("novalidate"), "{out}");
    }

    #[test]
    fn a_required_field_says_so_inside_its_own_label() {
        // Nothing is required or aria-required, so the label is the only channel
        // the obligation has; a sibling span reaches nobody.
        let out = overview(&published_draft());
        assert!(
            out.contains(r#"<label class="field-label" for="name">Name <span"#),
            "the pill must be inside the label: {out}"
        );
        let label = out.split(r#"for="name">"#).nth(1).expect("the name label");
        let label = label.split("</label>").next().expect("the label's end");
        assert!(label.contains("Required"), "{label}");
        // Same for a group, whose accessible name can only be its legend.
        assert!(out.contains(r#"<legend class="field-label">Description <span"#), "{out}");
    }

    #[test]
    fn the_refused_notice_carries_no_live_role_of_its_own() {
        // Danger renders role="alert", an assertive region nested in this polite
        // one; the region announces, the alert only styles.
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Refused("Nothing was saved."));
        let out = region(&view).into_string();
        assert!(out.contains(r#"aria-live="polite""#), "{out}");
        assert!(!out.contains(r#"role="alert""#), "{out}");
        assert!(out.contains("Nothing was saved."), "{out}");
    }

    #[test]
    fn a_rail_link_s_accessible_name_does_not_run_its_two_lines_together() {
        let out = overview(&published_draft());
        assert!(out.contains(r#"aria-label="Overview, 5 of 5 required""#), "{out}");
        // A section with no requirements needs no label: the visible title is
        // already the whole name.
        assert!(out.contains(r#"<span>Publications</span>"#), "{out}");
        let publications = out.split(r#"/sections/publications""#).nth(1).expect("the link");
        let publications = publications.split("</a>").next().expect("the link's end");
        assert!(!publications.contains("aria-label"), "{publications}");
    }

    #[test]
    fn a_project_name_and_a_stored_value_are_both_escaped() {
        // The name comes from a project file and the values from a draft, so
        // both are data.
        let hostile = "<script>alert(1)</script>";
        let mut draft = ProjectDraft::default();
        draft.set("name", json!(hostile));
        draft.set("description", json!({"en": hostile}));
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.project_name = Some(hostile);
        let out = page(&view).into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }

    fn a_live_proposal(
        kind: ProposalKind,
        operation: ProposalOperation,
        entity_id: &str,
        payload: &str,
    ) -> EntityProposal {
        EntityProposal {
            id: Default::default(),
            shortcode: "0801d".to_string(),
            entity_id: entity_id.to_string(),
            kind,
            operation,
            payload: payload.to_string(),
            status: editor_core::proposals::ProposalStatus::Draft,
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
    fn the_proposals_box_and_the_findings_tell_two_same_named_proposals_apart() {
        let draft = published_draft();
        let proposals = [
            a_live_proposal(ProposalKind::Person, ProposalOperation::New, "person-900", ADA),
            a_live_proposal(ProposalKind::Person, ProposalOperation::New, "person-901", ADA),
            a_live_proposal(
                ProposalKind::Organization,
                ProposalOperation::New,
                "organization-900",
                r#"{"name":"Acme","url":"https://acme.example/"}"#,
            ),
        ];
        let scope = AgentScope::with_proposals(published_agents(), &proposals);
        let findings: Vec<(String, ProposalKind, String)> = proposals
            .iter()
            .map(|proposal| (proposal.entity_id.clone(), proposal.kind, "needs finishing".to_string()))
            .collect();
        let mut view = view(&draft, "contributors", Audience::Everyone);
        view.agents = Some(&scope);
        view.proposals = &proposals;
        view.proposal_findings = &findings;

        for out in [
            proposals_summary(&view).into_string(),
            proposal_findings_list(&view).into_string(),
        ] {
            assert!(out.contains("Ada Lovelace (person, person-900)"), "{out}");
            assert!(out.contains("Ada Lovelace (person, person-901)"), "{out}");
            assert!(out.contains(">Acme (organisation)<"), "a name held once carries no id: {out}");
            assert!(!out.contains("organization-900<"), "{out}");
        }
    }

    #[test]
    fn a_new_proposal_notice_names_no_id() {
        let draft = published_draft();
        let view = view(&draft, "contributors", Audience::Everyone);
        let new = proposed_notice(&view, ProposalKind::Person, "person-900").into_string();
        assert!(new.contains("A new person has been started."), "{new}");
        // The id is in the link's target, not in anything read.
        assert!(!new.replace("/entities/person-900", "").contains("person-900"), "{new}");
    }
}

/// Snapshots of the rendered form, separate from the rule tests: each of those
/// names one rule, and these pin the whole markup so a change nobody was
/// thinking about shows up as a diff. Deterministic: the data is a committed
/// project and every timestamp arrives as a string.
#[cfg(test)]
mod snapshots {
    use super::tests::{published_draft, view};
    use super::*;

    #[test]
    fn snapshot_a_depositor_s_editable_overview() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        insta::assert_snapshot!("section_overview_depositor", page(&view).into_string());
    }

    #[test]
    fn snapshot_the_rdu_view_which_adds_the_rdu_only_fields_and_section() {
        let draft = published_draft();
        let view = view(&draft, "overview", Audience::RduOnly);
        insta::assert_snapshot!("section_overview_rdu", page(&view).into_string());
    }

    #[test]
    fn snapshot_a_project_locked_for_review() {
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.locked = Some(Locked::InReview);
        view.saved_at = Some("2026-09-03 08:15 UTC");
        insta::assert_snapshot!("section_overview_in_review", page(&view).into_string());
    }

    #[test]
    fn snapshot_a_refused_save() {
        // The region rather than the page, because this is what the enhanced
        // path actually sends back.
        let draft = published_draft();
        let mut view = view(&draft, "overview", Audience::Everyone);
        view.notice = Some(Notice::Refused(
            "This project is in review, so the draft cannot be changed. Nothing was saved.",
        ));
        insta::assert_snapshot!("section_overview_refused", region(&view).into_string());
    }
}
