//! One form section: the rail, the fields, and the save control.
//!
//! [`page`] is the whole page; [`region`] is the part a save replaces. Both come
//! from one [`SectionView`], so the plain and enhanced paths cannot drift.
//!
//! Three invariants:
//!
//! - **The region is the rail, the status and the form together**, under one id. Patching only the
//!   `<form>` leaves the rail showing the counts from before the save, so answering the last
//!   required field goes quiet while the rail still says something is missing.
//! - **The status region is rendered empty from the first load.** An `aria-live` region announces a
//!   *change* to content it already holds; one morphed in together with its text is widely reported
//!   not to announce at all. `empty:hidden` is what keeps that free.
//! - **No field is `required`, and the form is not `novalidate`.** A draft may be missing anything
//!   (REQ-1.9) and saving one must always work (REQ-1.10), so nothing is `required`. Validation
//!   stays on because `startDate`/`endDate` are `type="date"`, which cannot hold a half-typed date
//!   — the value comes back empty, so with validation off, fiddling the year of a real date and
//!   saving would clear it. Datastar gates its form path on the same flag.

use editor_core::draft::ProjectDraft;
use editor_core::records::ReviewOutcome;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use serde_json::Value;

use crate::form::obligation::{section_progress, SectionProgress};
use crate::form::registry::{sections_for, Audience, Section};
use crate::form::widgets::{field_row, Mode};
use crate::form::INTENT;

/// The id the enhanced path's patch targets. Also the anchor a save returns to.
pub const REGION_ID: &str = "project-section";

/// Store the draft, changing nothing about the review cycle (REQ-1.10).
///
/// Also what an unknown intent falls back to. A body naming a verb this build
/// does not know must not submit or withdraw by typo: those two are not
/// undoable by the depositor, and saving is.
pub const SAVE: &str = "save";

/// Validate the draft and record it as the project's pending submission
/// (REQ-1.12).
pub const SUBMIT: &str = "submit";

/// Take a pending submission back, leaving the draft (REQ-4.7).
pub const WITHDRAW: &str = "withdraw";

/// Show the withdrawal confirmation, which posts [`WITHDRAW`].
///
/// Two steps rather than one, because a withdrawal cannot be undone by the
/// depositor — the submission's place in the queue and whatever a reviewer has
/// already recorded on it both go. It shares this URL rather than taking one of
/// its own, for the reason every other write here does: a refused post
/// re-renders somewhere that still answers `GET`.
pub const WITHDRAW_CONFIRM: &str = "withdraw-confirm";

/// Why the form is read-only.
///
/// A reason rather than a `bool`, because the page has to say which it is: a
/// depositor whose work is queued and one whose work a reviewer has open should
/// not be told the same thing, and only one of them can expect it back soon.
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

/// The latest finished review round, as the depositor's form shows it.
///
/// Everything here is about *communicating* the round. The one thing the round
/// also governs mechanically — which fields are fixed — is
/// [`SectionView::accepted_fields`], separately, because the applier skip that
/// enforces it is not a rendering concern.
pub struct RoundSummary<'a> {
    pub outcome: ReviewOutcome,
    /// What RDU wrote. Required for a rejection and a request-changes, absent
    /// for an approval and a withdrawal.
    pub note: Option<&'a str>,
    /// Already formatted; the server owns the format.
    pub at: &'a str,
    /// Each field RDU put its own value in place of, with that value rendered.
    ///
    /// The other half of the reason this surface exists. REQ-4.3 lets a
    /// reviewer edit before accepting and REQ-4.4 waives the second approver,
    /// so a substituted value is seen by nobody unless the depositor is shown
    /// it here.
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
            // REQ-4.6 discards the submission and notifications are out of
            // scope, so without this the work vanishes with nothing saying
            // why. The draft is kept (REQ-1.13), which is the other half of
            // what the depositor needs to know.
            ReviewOutcome::Rejected => (
                "RDU rejected this submission",
                "The submission was discarded and the published project is unchanged. Your draft is still here, \
                 so you can change it and submit again.",
            ),
            ReviewOutcome::Approved => (
                "RDU approved this project",
                "It is recorded for a pull request against the repository, and the published page updates once \
                 that is merged. Editing again starts the next round.",
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
    /// The write was refused, and why. The whole-form kind: a live submission,
    /// nothing to submit, or storage that would not take the write. Field-level
    /// errors are [`SectionView::errors`], which is a different thing — they
    /// name a control the reader can go and fix.
    Refused(&'a str),
}

/// Everything one section rendering needs.
///
/// A struct rather than a long argument list: half of these are `Option`s of
/// similar types, and adjacent optional arguments of one type are silently
/// swappable — the same reason the form tiles' shell is a struct.
pub struct SectionView<'a> {
    pub shortcode: &'a str,
    /// The published project's name, or `None` for a shortcode the published set
    /// does not hold (REQ-2.3).
    pub project_name: Option<&'a str>,
    pub section: &'static Section,
    /// Which fields and sections this reader sees. Not a permission check in
    /// itself — the registry is consulted by the decoder too, so the form and
    /// the save cannot disagree about which fields a depositor owns.
    pub audience: Audience,
    pub draft: &'a ProjectDraft,
    /// `None` while the project is editable.
    pub locked: Option<Locked>,
    /// Field ids RDU accepted in the round being answered, which are therefore
    /// fixed until it is submitted again (REQ-4.5).
    ///
    /// Ids rather than a per-field flag on the registry, because the set is
    /// per-round data and the registry is a constant. Empty whenever the latest
    /// round did not return the project — an approval's decisions are not a
    /// lock on work that has left the depositor's hands.
    pub accepted_fields: &'a [String],
    /// Whether this reader may take the pending submission back (REQ-4.7), and
    /// so whether the withdrawal control is offered at all.
    pub may_withdraw: bool,
    /// Set while the withdrawal confirmation is showing.
    pub confirming_withdrawal: bool,
    /// Per-field submit errors, keyed by the path the field posts under
    /// (`temporalCoverage[0]`, not `temporalCoverage`), in field order.
    ///
    /// Separate from [`Notice::Refused`]: these name a control the reader can
    /// go and fix, and a refusal names the form as a whole.
    pub errors: &'a [(String, String)],
    /// The latest finished review round, or `None` for a project nobody has
    /// reviewed.
    ///
    /// On the form rather than on a page of its own: REQ-4.5 retains the note
    /// and names nowhere to read it, and the place a depositor acts on it is
    /// the form they act on it *in*. It rides inside the region, so a save
    /// leaves it in place.
    ///
    /// It shows until the depositor submits again, which starts the next
    /// cycle. That is also what makes a returned draft distinguishable from one
    /// never submitted, without adding a state beyond REQ-2.1's five.
    pub round: Option<RoundSummary<'a>>,
    /// When the stored draft was last written, formatted. `None` when the form
    /// is showing published metadata that nobody has saved over yet (REQ-1.1).
    pub saved_at: Option<&'a str>,
    pub notice: Option<Notice<'a>>,
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

    /// Errors naming a field this section does not show, with the section it
    /// is in.
    ///
    /// The form is sectioned and submit validation is whole-project, so a
    /// refusal is routinely about a field the reader is not looking at.
    /// Rendered per field only, the message would be nowhere on the page and
    /// the refusal would read as a dead end.
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

    fn action(&self) -> String {
        format!("/projects/{}/sections/{}", self.shortcode, self.section.id)
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
            div { (status(view)) (form(view)) }
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
                    // REQ-2.3: a project may exist only locally, and REQ-1.1's
                    // "current published metadata" is then empty. Said plainly,
                    // because a blank form with no explanation reads as a
                    // failure to load.
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

/// The rail link's accessible name, when its two lines would otherwise run
/// together.
///
/// The title and the progress are adjacent `<span>`s with no whitespace between
/// them, because a flex column is what puts them on two lines — so the name
/// computation concatenates them into "Overview5 of 5 required". A separator in
/// the markup would either show as stray punctuation or depend on a
/// whitespace-only flex item not being rendered, so the name is stated instead.
///
/// `None` for a section with no requirements, where the visible title is already
/// the whole name and an `aria-label` repeating it would be one more string to
/// keep in step. The label always *starts* with the visible title, which is what
/// WCAG 2.5.3 (Label in Name) asks of an `aria-label` over visible text.
fn rail_link_label(title: &str, progress: &SectionProgress) -> Option<String> {
    progress.has_requirements().then(|| format!("{title}, {}", progress.summary()))
}

/// One rail link's classes.
///
/// Two complete literal strings rather than a base plus a conditional suffix:
/// Tailwind collects classes by scanning source text, so a class assembled at
/// runtime is one the build never emits — and the failure is silent, an
/// unstyled link with no error anywhere.
const fn rail_link_class(current: bool) -> &'static str {
    if current {
        "flex flex-col rounded bg-neutral-100 px-3 py-2 font-bold focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    } else {
        "flex flex-col rounded px-3 py-2 hover:bg-neutral-100 focus-visible:outline-2 \
         focus-visible:outline-primary-600"
    }
}

/// The section rail: every section this reader sees, with its obligation state.
///
/// A `<nav>` with a list, because it is navigation and a screen reader user
/// needs to be able to jump to it and count the items. The current section
/// carries `aria-current="page"` rather than only a colour.
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
                                // Both numbers, always, and in words: a rail
                                // that showed only what is outstanding cannot
                                // tell a finished section from an empty one, and
                                // one that marked completion with a colour or a
                                // tick alone would not say it to everyone.
                                span class="text-xs text-neutral-600" { (progress.summary()) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The live region a save writes its outcome into. Always present; see the
/// module docs. `empty:hidden` is what keeps an always-present region free —
/// without it every section carries an empty block's line box, the same reason
/// the form tiles' error region carries `.field-error:empty`.
fn status(view: &SectionView<'_>) -> Markup {
    html! {
        div class="empty:hidden" aria-live="polite" {
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
                Some(Notice::Refused(message)) => { (alert(message).variant(AlertVariant::Warning)) }
                None => {}
            }
            // Inside the live region, beside the refusal it details rather than
            // above the form heading: the refusal is what gets announced, and
            // it is about fields the reader cannot see, so the list of them has
            // to announce with it. Outside, the announcement said what needed
            // changing and the detail was silent.
            (errors_elsewhere(view))
        }
    }
}

/// The section's fields, and the control that saves them.
fn form(view: &SectionView<'_>) -> Markup {
    let action = view.action();
    html! {
        @if let Some(round) = view.round.as_ref() { (review_round(round)) }
        @if let Some(locked) = view.locked {
            ({
                alert(locked.message())
                    .variant(AlertVariant::Warning)
                    .title(locked.heading())
            })
        }
        h2 class="font-display text-xl mb-4" { (view.section.title) }
        // The `action` is the URL this form was fetched from, so a rejected save
        // re-renders somewhere that still answers `GET`. `data-on:submit` needs
        // no `__prevent`: Datastar 1.0.2 calls `preventDefault` unconditionally
        // for a `submit` event on a form element, so adding one would be noise.
        // With no script it is an ordinary POST and the server redirects.
        form
            id="section-form"
            method="post"
            action=(action)
            class="flex flex-col gap-6"
            data-on:submit={ "@post('" (action) "', {contentType: 'form'})" }
        {
            @for field in view.section.fields_for(view.audience) {
                // No wrapper and no obligation pill here: the pill is inside the
                // field's own label, which is the only way it reaches a screen
                // reader now that nothing is `required`. See `widgets::labelled`.
                (field_row(field, view.draft, view.mode_of(field.id)))
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

/// Errors about fields another section shows, each a link to the section that
/// shows it.
///
/// A link and not just a name: the field is one navigation away and the reader
/// has no other way to know which of six sections holds it.
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

/// The write controls, which depend on where the project sits in the cycle.
///
/// Every one of them is a named submit on the *same* form, as the review
/// surface's pair is: a native submit posts the activated button's name and
/// value, and Datastar 1.0.2's form mode appends them from
/// `SubmitEvent.submitter`. A second form would have to carry the fields again
/// to submit them, and `formaction` would be honoured on the plain path and
/// silently ignored on the enhanced one, where the bundle posts to the URL in
/// `@post`.
fn controls(view: &SectionView<'_>) -> Markup {
    html! {
        @if view.confirming_withdrawal {
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
                    // A link, not a button. Backing out has to write nothing,
                    // and a button with no intent falls through to `save` —
                    // which this form refuses while a submission is pending,
                    // so declining a withdrawal would answer "nothing was
                    // saved", the opposite of the reassurance it is for.
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
                @if let Some(saved_at) = view.saved_at {
                    p class="text-sm text-gray-600" { "Draft last saved " (saved_at) "." }
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
    use editor_core::draft::ProjectDraft;
    use serde_json::json;

    use super::*;
    use crate::form::registry::{field, section};

    /// A draft over a real committed project, so the fields under test hold what
    /// the corpus actually holds rather than what a fixture assumes.
    pub(super) fn published_draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    pub(super) fn view<'a>(draft: &'a ProjectDraft, section_id: &str, audience: Audience) -> SectionView<'a> {
        SectionView {
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
            section: section(section_id).expect("a known section"),
            audience,
            draft,
            locked: None,
            accepted_fields: &[],
            may_withdraw: false,
            confirming_withdrawal: false,
            errors: &[],
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
        // Whole-project submit validation against a sectioned form means the
        // field at fault is routinely not on this page. The refusal is what
        // gets announced, so the list of those fields has to be inside the
        // `aria-live` region with it — rendered above the form heading instead,
        // the announcement said something needed changing and the detail was
        // silent.
        //
        // Asserted on `status` alone, which is the region: slicing it out of a
        // whole rendering cannot distinguish "inside" from "just after", since
        // the alert it holds is itself a `<div>`.
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
        // The two renderings are exclusive: a field with a control on this page
        // gets its error beside that control, and listing it in the
        // other-sections block as well would send the reader away from the
        // input they are looking at.
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
    fn the_form_posts_to_the_url_it_was_fetched_from() {
        // A write posting to a path with no `GET` strands a rejected save on a
        // bare 405 — the dead end `POST /depositors/{id}` briefly was.
        let out = overview(&published_draft());
        assert!(out.contains(r#"action="/projects/0801d/sections/overview""#), "{out}");
        assert!(out.contains(r#"method="post""#), "{out}");
    }

    #[test]
    fn the_enhanced_path_posts_the_form_body_rather_than_signals() {
        // `contentType: 'form'` is what makes the body
        // `application/x-www-form-urlencoded`; Datastar transmits no signals on a
        // form-content-type request, which is what `editor_core::form` reads.
        // No `__prevent`: the bundle calls `preventDefault` unconditionally for a
        // `submit` event on a form element.
        let out = overview(&published_draft());
        assert!(
            out.contains(r#"data-on:submit="@post('/projects/0801d/sections/overview', {contentType: 'form'})""#),
            "{out}"
        );
        assert!(!out.contains("submit__prevent"), "{out}");
        // Keyed plugin attributes use `:`, not `-`. The hyphen form is a console
        // error and an inert control, and a snapshot asserting the attribute is
        // present passes either way.
        assert!(!out.contains("data-on-submit"), "{out}");
    }

    #[test]
    fn a_display_only_field_renders_its_value_and_no_control() {
        // REQ-1.5. A control here would post, and an empty one would clear a
        // value the reader was never able to change.
        let out = page(&view(&published_draft(), "overview", Audience::RduOnly)).into_string();
        assert!(out.contains("Shortcode"), "{out}");
        assert!(!out.contains(r#"name="shortcode""#), "{out}");
        assert!(!out.contains(r#"name="pid""#), "{out}");
        assert!(!out.contains(r#"name="id""#), "{out}");
    }

    #[test]
    fn a_field_the_form_cannot_read_yet_says_so_and_posts_nothing() {
        // Silently absent would leave a depositor who cannot find "Status" in
        // the section the published page shows it in concluding the form lost
        // it. Posting nothing is what keeps its stored value untouched
        // (REQ-1.7), because no applier names the field.
        let draft = published_draft();
        let out = overview(&draft);
        assert!(field("status").expect("status").shape.is_none(), "the premise of this test");
        assert!(out.contains("Status"), "{out}");
        assert!(out.contains("not editable here yet"), "{out}");
        assert!(!out.contains(r#"name="status""#), "{out}");
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
        // The rule the whole untouched-save guarantee rests on: `MISSING` is
        // filtered out of DPE and of OAI-PMH, so showing it here would make this
        // the one place in the platform that presents an internal marker as a
        // value — and the depositor would then have to delete it by hand.
        let mut draft = published_draft();
        draft.set("endDate", json!("MISSING"));
        let out = overview(&draft);
        assert!(out.contains(r#"name="endDate""#), "{out}");
        assert!(!out.contains("MISSING"), "{out}");
    }

    #[test]
    fn a_language_map_renders_the_offered_languages_plus_any_the_value_carries() {
        // Offering only a closed set would drop `ar` — live in two committed
        // files — on the first save: a tag with no control posts nothing, and a
        // map rebuilt from the body would not carry it.
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
        // A depositor's overview is all answered for this project, so the two
        // renderings must differ — which they cannot if only what is outstanding
        // is shown.
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
        // An `aria-live` region announces a *change* to content it already
        // holds; one inserted together with its text is widely reported not to
        // announce at all, and the enhanced path is exactly that case.
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
        // REQ-2.3: a project may exist only locally, and REQ-1.1's "current
        // published metadata" is then empty. A blank form with no explanation
        // reads as a page that failed to load.
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
        // Datastar matches a `text/html` response by `id` in `outer` mode, so the
        // id has to be on the region's own root — and the region has to include
        // the rail, or a save that answers the last required field leaves the
        // rail still saying something is missing.
        let out = region(&view(&published_draft(), "overview", Audience::Everyone)).into_string();
        assert!(out.starts_with(&format!(r#"<section id="{REGION_ID}""#)), "{out}");
        assert!(out.contains(r#"aria-label="Form sections""#), "{out}");
        assert!(out.contains("<form"), "{out}");
        // A fragment, not a document: patching `<html>` would replace the page.
        assert!(!out.contains("<!DOCTYPE"), "{out}");
    }

    #[test]
    fn no_field_is_required_or_the_browser_would_refuse_to_save_a_draft() {
        // A draft may be missing anything (REQ-1.9) and saving one must always
        // work (REQ-1.10). `required` on the name field would make an unfinished
        // draft unsaveable on both paths — Datastar runs the same
        // `checkValidity()` the browser does.
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
        // Nothing here is `required` or `aria-required` (REQ-1.9/REQ-1.10), so
        // the label is the only channel the obligation has. Rendered as a
        // sibling span it was visible and nothing else: a reader tabbing to the
        // control heard "Name, edit text".
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
        // `AlertVariant::Danger` renders `role="alert"`, an implicit assertive
        // live region; nested inside this polite one, screen readers disagree
        // about which politeness wins and some interrupt. The region announces;
        // the alert only styles.
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
        // The title and the progress are adjacent spans with no whitespace
        // between them, so the name computation would give "Overview5 of 5
        // required".
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

/// Snapshots of the rendered form.
///
/// Separate from the assertions above because they answer a different question.
/// Each test up there names one rule and fails with it; these pin the *whole*
/// markup, so a change nobody was thinking about — a control quietly becoming a
/// value, a posted name changing, an `aria-*` attribute going missing — shows up
/// as a diff rather than as nothing. Neither replaces the other: a snapshot
/// cannot say which rule broke, and a rule cannot notice what it does not
/// mention.
///
/// Deterministic by construction: the data is a committed project, and every
/// timestamp reaches the view as a string the caller formats, so nothing here
/// reads a clock.
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
