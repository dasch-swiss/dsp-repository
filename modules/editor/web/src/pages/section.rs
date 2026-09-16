//! One form section: the rail, the fields, and the save control.
//!
//! [`page`] is the whole page; [`region`] is the part a save replaces. Both come
//! from one [`SectionView`], so the plain and enhanced paths cannot drift.
//!
//! The region is the rail, the status and the form together, under one id:
//! patching only the `<form>` leaves the rail showing the counts from before the
//! save. The status region is rendered empty from the first load: an `aria-live`
//! region announces a change to content it already holds, and one morphed in
//! with its text is widely reported not to announce. No field is `required` and
//! the form is not `novalidate`: a draft may be missing anything, and validation
//! stays on because `type="date"` cannot hold a half-typed date, so with it off,
//! fiddling the year of a real date and saving would clear it.

use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::FormBody;
use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation};
use editor_core::records::ReviewOutcome;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use serde_json::Value;

use crate::form::obligation::{section_progress, SectionProgress};
use crate::form::registry::{sections_for, Audience, Section};
use crate::form::widgets::{field_row, Mode, Rows};
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
/// intent for every picker because every search box posts with the form; it
/// re-renders with the posted body kept, which carries the query back.
pub const FIND_AGENT: &str = "find-agent";

/// Start a proposal for a new person.
pub const PROPOSE_PERSON: &str = "propose-person";

/// Start a proposal for a new organisation.
pub const PROPOSE_ORGANIZATION: &str = "propose-organization";

/// Propose a change to an entity this project already references. The entity
/// rides in the intent value (`propose-changes:person-417`) via
/// [`propose_changes_intent`] and [`proposed_entity`].
///
/// It must not move to a field of its own: a section renders every resolved
/// agent row inside one `<form>`, so a per-row field shares one name across all
/// of them and `FormBody::get` takes the first. Only the activated button posts
/// its name and value, natively and through `SubmitEvent.submitter`.
pub const PROPOSE_CHANGES: &str = "propose-changes";

/// The separator between [`PROPOSE_CHANGES`] and the entity id: no entity id
/// contains a colon.
const PROPOSE_CHANGES_SEPARATOR: char = ':';

/// The intent value a "Propose changes" control on `entity_id`'s row posts.
#[must_use]
pub fn propose_changes_intent(entity_id: &str) -> String {
    format!("{PROPOSE_CHANGES}{PROPOSE_CHANGES_SEPARATOR}{entity_id}")
}

/// The entity id a posted intent carries, or `None` when it is not a
/// propose-changes intent. An empty id answers `None`: naming the verb with no
/// entity has asked for nothing.
#[must_use]
pub fn proposed_entity(intent: &str) -> Option<&str> {
    intent
        .strip_prefix(PROPOSE_CHANGES)?
        .strip_prefix(PROPOSE_CHANGES_SEPARATOR)
        .filter(|entity_id| !entity_id.is_empty())
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
    /// What the reader is told, and what they can do about it.
    const fn message(self) -> &'static str {
        match self {
            Self::Submitted => {
                "This project has been submitted for review, so the form is read-only until RDU \
                                picks it up or returns it to you. Ask RDU if you need to change something in \
                                the meantime."
            }
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

impl RoundSummary<'_> {
    /// The heading, and what it means for the depositor now.
    fn wording(&self) -> (&'static str, &'static str) {
        match self.outcome {
            ReviewOutcome::ChangesRequested => (
                "RDU asked for changes",
                "This project is a draft again. Fields RDU accepted are fixed until you submit it again; \
                 everything else is yours to edit.",
            ),
            // A rejection discards the submission and notifications are out of
            // scope, so this is the only thing that says why.
            ReviewOutcome::Rejected => (
                "RDU rejected this submission",
                "The submission was discarded and the published project is unchanged. Your draft is still here, \
                 so you can change it and submit again.",
            ),
            // Depositor-facing, so the mechanism is not named; the wait matches
            // ProjectState::Approved, the normative wording.
            ReviewOutcome::Approved => (
                "RDU approved this project",
                "Your changes will appear on the public site with the next repository release — usually within a \
                 few weeks. Editing again starts the next round.",
            ),
            ReviewOutcome::Withdrawn => (
                "The submission was taken back",
                "It is no longer in RDU's queue. Your draft is unchanged, so you can keep editing and submit \
                 again.",
            ),
        }
    }

    fn variant(&self) -> AlertVariant {
        match self.outcome {
            ReviewOutcome::Approved => AlertVariant::Success,
            ReviewOutcome::Rejected => AlertVariant::Warning,
            ReviewOutcome::ChangesRequested | ReviewOutcome::Withdrawn => AlertVariant::Info,
        }
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
    /// A proposal was started or a change proposed; links to the entity form, which
    /// is where the depositor fills it in.
    Proposed {
        kind: ProposalKind,
        operation: ProposalOperation,
        entity_id: &'a str,
    },
    /// The write was refused, and why: the whole-form kind. Field-level errors are
    /// [`SectionView::errors`], which name a control the reader can fix.
    Refused(&'a str),
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
    /// reviewed. On the form because that is where a depositor acts on it, and
    /// inside the region so a save leaves it in place. It shows until the depositor
    /// submits again, which is what makes a returned draft distinguishable from
    /// one never submitted without a sixth lifecycle state.
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
    /// depositor added but has not filled in, which is never stored. `None` after a
    /// successful save, since a row with no text is not data; `Some` for a refusal
    /// and for add/remove.
    pub posted: Option<&'a FormBody>,
    /// The field one more blank row was just asked for.
    pub adding_row: Option<&'a str>,
    /// The agents an id field may refer to, borrowed from `AppState`'s snapshot.
    pub agents: Option<&'a AgentScope<'a>>,
    /// Base URL a repeatable field's add and remove controls submit to: `/fields`
    /// under the section's own URL, built by the caller because the route shape
    /// belongs to the router, so a row action resolves through the same audience
    /// gate, lock check and shortcode fold the save does.
    pub rows_action: String,
    /// Whether an approved change is waiting for the repository release that
    /// carries it. Not a [`Locked`] variant: approve is the only outcome that does
    /// not hand the project back, so the form stays editable and what the
    /// depositor types is the next cycle.
    pub awaiting_release: bool,
}

impl SectionView<'_> {
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
                let section = crate::form::registry::section_of(id)?;
                let label = crate::form::registry::field(id).map_or(id, |field| field.label);
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
    /// remove controls submit to; see [`Self::rows_action`].
    fn rows(&self) -> Rows<'_> {
        Rows {
            posted: self.posted,
            action: &self.rows_action,
            adding: self.adding_row,
            agents: self.agents,
            // The section form dispatches all three propose intents, unlike the entity
            // form's affiliations picker, which reuses these widgets.
            propose: true,
        }
    }
}

/// The whole page: the project heading, then the region a save replaces.
pub fn page(view: &SectionView<'_>) -> Markup {
    html! {
        div class="max-w-5xl py-8" { (heading(view)) (region(view)) }
    }
}

/// The rail, the status region and the form — everything a save can change.
///
/// Rendered on its own for the enhanced path's patch, and spliced into [`page`]
/// for the plain one, so the two cannot drift.
pub fn region(view: &SectionView<'_>) -> Markup {
    html! {
        section id=(REGION_ID) class="grid gap-6 md:grid-cols-[16rem_1fr] items-start" {
            (rail(view))
            div { (status(view)) (proposals_summary(view)) (form(view)) }
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
                        "This project is not in the published set this deployment carries, so there was nothing \
                         to pre-fill. Anything you enter is kept as a draft."
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
        "flex flex-col rounded bg-neutral-100 px-3 py-2 font-bold focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    } else {
        "flex flex-col rounded px-3 py-2 hover:bg-neutral-100 focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    }
}

/// The section rail: a `<nav>` with a list, so a screen reader user can jump to
/// it and count the items; the current section carries `aria-current="page"`.
fn rail(view: &SectionView<'_>) -> Markup {
    html! {
        nav class="md:sticky md:top-4" aria-label="Form sections" {
            ol class="flex flex-col gap-1" {
                @for section in sections_for(view.audience) {
                    @let progress = section_progress(section, view.audience, view.draft);
                    @let current = section.id == view.section.id;
                    li {
                        a   href={ "/projects/" (view.shortcode) "/sections/" (section.id) }
                            class=(rail_link_class(current))
                            aria-current=[current.then_some("page")]
                            aria-label=[rail_link_label(section.title, &progress)]
                        {
                            span { (section.title) }
                            @if progress.has_requirements() {
                                // Both numbers, in words: a rail showing only what is outstanding
                                // cannot tell a finished section from an empty one.
                                span class="text-xs text-neutral-600" { (progress.summary()) }
                            }
                        }
                    }
                }
            }
        }
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
fn status(view: &SectionView<'_>) -> Markup {
    html! {
        div class="empty:hidden sticky top-0 z-10" aria-live="polite" {
            @match view.notice {
                Some(Notice::Saved) => { (alert("Draft saved.").variant(AlertVariant::Success)) }
                Some(Notice::Submitted) => {
                    ({
                        alert(
                                "Submitted for review. The form is read-only until RDU returns the project to \
                                 you, and your draft is kept either way.",
                            )
                            .variant(AlertVariant::Success)
                            .title("Sent to RDU")
                    })
                }
                Some(Notice::Withdrawn) => {
                    ({
                        alert(
                                "The submission has been taken back and is no longer in RDU's queue. Your draft \
                                 is unchanged, so you can keep editing and submit again.",
                            )
                            .variant(AlertVariant::Success)
                            .title("Submission withdrawn")
                    })
                }
                Some(Notice::Discarded) => {
                    ({
                        alert(
                                "The draft has been discarded. This form is showing the project's published \
                                 metadata again, and nothing you save from here is submitted until you say so.",
                            )
                            .variant(AlertVariant::Success)
                            .title("Draft discarded")
                    })
                }
                Some(Notice::Changed { by, at }) => { (changed_notice(by, at)) }
                Some(Notice::Proposed { kind, operation, entity_id }) => {
                    (proposed_notice(view, kind, operation, entity_id))
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

/// The section's fields, and the control that saves them.
fn form(view: &SectionView<'_>) -> Markup {
    // Only on a form a save can change: a read-only one has no controls for
    // change to fire from, and advertising a save the handler would refuse is
    // worse than none.
    let autosave = view
        .locked
        .is_none()
        .then(|| format!("@post('{}', {{contentType: 'form'}})", view.action()));
    let action = view.action();
    html! {
        @if let Some(round) = view.round.as_ref() { (review_round(round)) }
        @if view.awaiting_release { (awaiting_release_notice()) }
        @if let Some(locked) = view.locked {
            ({
                alert(locked.message())
                    .variant(AlertVariant::Warning)
                    .title(locked.heading())
            })
        }
        h2 class="font-display text-xl mb-4" { (view.section.title) }
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
            class="flex flex-col gap-6"
            data-on:submit={
                "@post(evt.submitter?.formAction || '"
                (action)
                "', {contentType: 'form'})"
            }
            "data-on:change__debounce.1s"=[autosave.as_deref()]
        {
            // The revision this form was rendered from; absent with no stored draft.
            @if let Some(baseline) = view.baseline {
                input type="hidden" name=(BASELINE) value=(baseline);
            }
            @for field in view.section.fields_for(view.audience) {
                // No obligation pill here: it is inside the field's own label, the only
                // way it reaches a screen reader now that nothing is required.
                (field_row(field, view.draft, view.mode_of(field.id), view.rows()))
                @for message in view.errors_for(field.id) {
                    (alert(message).variant(AlertVariant::Warning))
                }
            }
            (controls(view))
        }
    }
}

/// The latest review round: what RDU decided, when, its note, and anything RDU
/// put in place of the depositor's own values.
fn review_round(round: &RoundSummary<'_>) -> Markup {
    let (heading, meaning) = round.wording();
    html! {
        ({
            let body = html! {
                p { (meaning) } p class = "text-sm mt-2" { (round.at) } @ if let
                Some(note) = round.note { div class = "mt-3" { p class =
                "text-xs font-bold uppercase tracking-wide" { "What RDU wrote" } p class
                = "whitespace-pre-line" { (note) } } } @ if ! round.substitutions
                .is_empty() { div class = "mt-3" { p class =
                "text-xs font-bold uppercase tracking-wide" {
                "Values RDU changed before deciding" } ul class =
                "flex flex-col gap-2 mt-1" { @ for (field, value) in round.substitutions
                { li { strong { (crate ::form::registry::field(field).map_or(field
                .as_str(), | f | f.label)) } ": "(crate
                ::form::widgets::value_markup(Some(value))) } } } } }
            };
            alert(body).variant(round.variant()).title(heading)
        })
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

/// An approved change is waiting for the release that carries it.
/// Informational: nothing is wrong and there is nothing to do. The wording comes
/// from [`ProjectState::Approved`](editor_core::status::ProjectState), which the
/// list column and the `/states` page also read, so the three cannot drift.
fn awaiting_release_notice() -> Markup {
    html! {
        ({
            alert(editor_core::status::ProjectState::Approved.explanation())
                .variant(AlertVariant::Info)
                .title("Waiting for the next release")
        })
    }
}

/// A proposal was just started, named by kind and id, with a link onward. Its
/// own function per this repo's rule on a nested `html!` passed as a call
/// argument (`maudfmt` skips it and `cargo fmt` mangles it).
fn proposed_notice(
    view: &SectionView<'_>,
    kind: ProposalKind,
    operation: ProposalOperation,
    entity_id: &str,
) -> Markup {
    let noun = kind.label().to_lowercase();
    let lead = match operation {
        ProposalOperation::New => format!("A new {noun} has been started as {entity_id}."),
        ProposalOperation::Change => format!("A change to {entity_id} has been started."),
    };
    let body = html! {
        p { (lead) }
        p class="mt-2" {
            a href={ "/projects/" (view.shortcode) "/entities/" (entity_id) } class="underline" {
                "Open it to fill in the details"
            }
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
/// the repo's rule on nested `html!` (`modules/dpe/CLAUDE.md`): `maudfmt` skips a
/// block passed as a call argument and `cargo fmt` then flattens it, with `just
/// check` green because it only verifies `maudfmt` is a no-op.
fn proposal_findings_list(view: &SectionView<'_>) -> Markup {
    html! {
        ul class="flex flex-col gap-2" {
            @for (entity_id, kind, message) in view.proposal_findings {
                li {
                    a   href={ "/projects/" (view.shortcode) "/entities/" (entity_id) }
                        class="underline font-bold"
                    { (kind.label()) " " (entity_id) }
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
    html! {
        div class="rounded border border-neutral-300 bg-white p-4 mb-6" {
            // h2, not h3: this panel renders above the section form, whose title is
            // an h2, and an h3 here breaks the outline for a reader navigating by
            // level.
            h2 class="font-display text-base mb-2" { "Proposed persons and organisations" }
            ul class="flex flex-col gap-1" {
                @for proposal in &live {
                    @let resolved = view
                        .agents
                        .and_then(|agents| agents.get(&proposal.entity_id));
                    li {
                        span class="text-xs font-bold uppercase tracking-wide text-neutral-600" {
                            (proposal.kind.label())
                        }
                        " "
                        a   href={ "/projects/" (view.shortcode) "/entities/" (proposal.entity_id) }
                            class="underline font-bold"
                        {
                            @match resolved {
                                Some(agent) => (agent.label)
                                None => (proposal.entity_id)
                            }
                        }
                        @if resolved.is_some() { " (" (proposal.entity_id) ")" }
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
/// named function per the repo's rule on nested `html!` (`modules/dpe/CLAUDE.md`).
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

/// The write controls, which depend on where the project sits in the cycle.
/// Every one is a named submit on the same form: a native submit posts the
/// activated button's name and value and Datastar's form mode appends them from
/// `SubmitEvent.submitter`, while a `formaction` would be silently ignored on the
/// enhanced path.
fn controls(view: &SectionView<'_>) -> Markup {
    html! {
        @if view.confirming == Some(Confirmation::Discard) {
            div class="rounded border border-neutral-300 bg-white p-4 flex flex-col gap-3" {
                p {
                    "Discarding the draft removes every change that has not been submitted. It cannot be undone. \
                     The project itself is not affected — the form re-opens showing its published metadata."
                }
                div class="flex items-center gap-4" {
                    ({
                        button("Yes, discard the draft")
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, DISCARD)
                    })
                    // A link, as for declining a withdrawal: backing out must write
                    // nothing, and a button with no intent falls through to save.
                    (link("Keep the draft", view.action()))
                }
            }
        } @else if view.confirming == Some(Confirmation::Withdrawal) {
            div class="rounded border border-neutral-300 bg-white p-4 flex flex-col gap-3" {
                p {
                    "Taking the submission back removes it from RDU's queue, along with anything a reviewer has \
                     already recorded on it. Your draft is kept, so you can keep editing and submit again."
                }
                div class="flex items-center gap-4" {
                    ({
                        button("Yes, take it back")
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, WITHDRAW)
                    })
                    // A link, not a button: a button with no intent falls through to save,
                    // which this form refuses while a submission is pending, so declining
                    // would answer "nothing was saved".
                    (link("Keep waiting", view.action()))
                }
            }
        } @else if view.locked.is_some() {
            @if view.may_withdraw {
                div class="flex items-center gap-4" {
                    ({
                        button("Take the submission back")
                            .variant(ButtonVariant::Secondary)
                            .button_type(ButtonType::Submit)
                            .name_value(INTENT, WITHDRAW_CONFIRM)
                    })
                }
            }
        } @else {
            div class="flex flex-wrap items-center gap-4" {
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
                @if let Some(saved_at) = view.saved_at {
                    p class="text-sm text-gray-600" {
                        "Draft last saved "
                        (saved_at)
                        // Named only when it was somebody else: "saved by you"
                        // is the ordinary case and reads as noise.
                        @if let Some(editor) = view.last_editor { " by " (editor) }
                        "."
                    }
                } @else {
                    p class="text-sm text-gray-600" {
                        "Nothing saved yet — this form shows the project's published metadata."
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_core::agents::{AgentScope, Agents};
    use editor_core::draft::ProjectDraft;
    use serde_json::json;

    use super::*;
    use crate::form::registry::section;

    /// The committed agent set, loaded once for the whole test binary.
    fn published_agents() -> &'static Agents {
        static AGENTS: std::sync::OnceLock<Agents> = std::sync::OnceLock::new();
        AGENTS.get_or_init(|| {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data");
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

    /// A draft over a real committed project, so the fields under test hold what the corpus
    /// actually holds rather than what a fixture assumes.
    pub(super) fn published_draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
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
            adding_row: None,
            rows_action: format!("/projects/0801d/sections/{section_id}/fields"),
            awaiting_release: false,
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
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

    #[test]
    fn an_editable_form_autosaves_on_change_and_a_locked_one_does_not() {
        // change rather than input: the response patches the whole region, so a
        // mid-keystroke trigger would patch the field being typed into. Nothing on
        // a read-only form.
        let draft = published_draft();
        let editable = page(&view(&draft, "overview", Audience::Everyone)).into_string();
        assert!(editable.contains("data-on:change__debounce.1s"), "{editable}");
        assert!(!editable.contains("data-on:input"), "never on every keystroke: {editable}");

        let mut locked = view(&draft, "overview", Audience::Everyone);
        locked.locked = Some(Locked::Submitted);
        let out = page(&locked).into_string();
        assert!(!out.contains("data-on:change"), "{out}");
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
    fn no_page_ships_the_whole_agent_set_and_every_picker_can_be_searched() {
        // Nothing may ship the whole agent set as a <datalist>: the check is for a
        // datalist anywhere, not an id. Over every section and both audiences,
        // because funding sits alone in the access section.
        let mut draft = published_draft();
        draft.set("contactPoint", serde_json::json!(["organization-008"]));
        for audience in [Audience::Everyone, Audience::RduOnly] {
            for section in sections_for(audience) {
                let out = page(&view(&draft, section.id, audience)).into_string();
                assert_eq!(
                    out.matches("<datalist").count(),
                    0,
                    "{} ({audience:?}) ships a datalist again",
                    section.id
                );
                assert!(!out.contains("list=\"agent-suggestions\""), "{} ({audience:?})", section.id);
            }
        }

        // On a section that has an agent field, every picker carries its own search box and the
        // id rides in a hidden input, which is what keeps an untouched save byte-exact.
        let contributors = page(&view(&draft, "contributors", Audience::Everyone)).into_string();
        assert!(
            contributors.contains(r#"<input type="hidden" name="contactPoint.r0" value="organization-008">"#),
            "the id posts from a hidden input: {contributors}"
        );
        assert!(
            contributors.contains(r#"name="contactPoint.r0.q""#),
            "the picker offers a search: {contributors}"
        );
        assert!(
            contributors.contains(r#"value="find-agent""#),
            "and a control to run it: {contributors}"
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
