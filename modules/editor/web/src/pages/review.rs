//! The review queue and the field-by-field diff surface (US-4).
//!
//! [`queue`] is `GET /review`; [`page`] is `GET /review/{shortcode}` and
//! [`region`] is the part a decision save replaces, the same split — and for
//! the same reasons — as [`section`](super::section).
//!
//! ## The diff is one form, not one request per field
//!
//! The surface offers accept, revert and edit-in-place *per field*; that does
//! not mean a request per field, and making each one its own fetch would put
//! partial failure back where the plan wanted it out of: a reviewer who accepts
//! eight fields and loses the ninth to a dropped connection has a submission
//! half-decided with nothing saying which half. One form posting one body is
//! the batching, natively — every decision and every substituted value arrives
//! together and is written in one transaction.
//!
//! It also keeps the surface working without JavaScript, which every other
//! authenticated surface in this service does. The enhanced path is the same
//! `@post(..., {contentType: 'form'})` the section form uses, so a save patches
//! the region instead of reloading the page, and the plain path posts and is
//! redirected.
//!
//! **The submit button's name reaches the server on both paths.** A native
//! submit includes the activated button's name and value; Datastar 1.0.2's form
//! mode appends them too, from `SubmitEvent.submitter`. That is what lets
//! "Accept all" and "Save decisions" be two buttons on one form rather than two
//! forms or a signal, and [`tests::accept_all_is_a_named_submit_on_the_same_form`]
//! pins the markup half of it.
//!
//! ## Two namespaces, kept apart
//!
//! A decision posts under `decision.{field}`; a substituted value posts under
//! the field's own name — `{field}` for a scalar, `{field}.{tag}` for one
//! language of a map — which is exactly what the section form posts and
//! therefore exactly what `editor_core::form`'s appliers read. No registry id
//! begins with `decision.`, so the two cannot collide, and the reviewer's edit
//! goes through the same trimming, newline and placeholder rules a depositor's
//! does rather than a second set that agrees with them by inspection.
//!
//! ## A project with no published counterpart
//!
//! A project can exist only locally, while the comparison assumes a published
//! value per field. Rather than degenerate quietly, the surface says
//! there is nothing to compare against and **offers no revert**: reverting means
//! keeping the published value, and there is no published value — the choice
//! would silently unset a field the contract requires. Accept and
//! edit-in-place still apply, which is the whole of what a reviewer can
//! meaningfully do to a record that is new.

use editor_core::draft::ProjectDraft;
use editor_core::records::SubmissionState;
use editor_core::review::Decision;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::badge::{badge, BadgeVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::radio_group::radio_group;
use mosaic_tiles::table::{table, table_cell, table_head_cell};
use serde_json::Value;

use crate::form::registry::Field;
use crate::form::widgets::{control, value_markup};

/// The id the enhanced path's patch targets, and the anchor a save returns to.
pub const REGION_ID: &str = "review-surface";

/// The name every decision control on the diff form posts under, before the
/// field id. Stated once, because the renderer and the decoder both spell it.
pub const DECISION_PREFIX: &str = "decision";

/// The name every control that says *what a POST is for* posts under.
pub const INTENT: &str = "intent";

/// Store the decisions and substitutions the body carries.
pub const SAVE: &str = "save";

/// Store them, and accept every row nobody has decided yet.
pub const ACCEPT_ALL: &str = "accept-all";

/// Take the submission over, without deciding anything.
pub const CLAIM: &str = "claim";

// --- The queue ------------------------------------------------------------

/// One pending submission, as the queue renders it.
pub struct QueueRow<'a> {
    /// The published project's shortcode as written in its file, or the stored
    /// (folded) key for a project the published set does not hold.
    pub shortcode: &'a str,
    /// `None` for a project that exists only locally.
    pub project_name: Option<&'a str>,
    /// The submitter's name, `None` once that account is removed — the row
    /// survives, so this reads as unknown rather than dangling.
    pub last_editor: Option<&'a str>,
    /// Already formatted; the server owns the format.
    pub submitted_at: &'a str,
    /// Who has the submission open, and `None` while nobody has picked it up.
    pub reviewer: Option<&'a str>,
    pub state: SubmissionState,
}

/// One draft, as the queue renders it.
pub struct DraftRow<'a> {
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    pub last_editor: Option<&'a str>,
    /// Already formatted.
    pub updated_at: &'a str,
}

/// `GET /review` — every pending submission, then every draft.
///
/// Both tables, not just the first: every RDU member sees every pending
/// submission *and* every draft, so that RDU can help a depositor who is stuck
/// before submitting. A draft is not reviewable,
/// which is why it is a separate table rather than a row with no controls —
/// one list mixing the two would invite a reviewer to look for an action that
/// does not exist.
pub fn queue(pending: &[QueueRow<'_>], drafts: &[DraftRow<'_>]) -> Markup {
    html! {
        div class="py-8" {
            h1 class="font-display text-2xl mb-2" { "Review queue" }
            p class="text-gray-600 mb-6" {
                "Oldest first. Every RDU member sees every pending submission and every draft — access here is \
                 role-based, not per project."
            }
            @if pending.is_empty() {
                p class="text-gray-600 mb-8" { "No submissions are waiting for review." }
            } @else { (pending_table(pending)) }
            h2 class="font-display text-xl mt-10 mb-2" { "Drafts in progress" }
            p class="text-gray-600 mb-4" {
                "Work a depositor has saved but not submitted. Nothing here is waiting on you; open one to help \
                 somebody who is stuck."
            }
            @if drafts.is_empty() {
                p class="text-gray-600" { "No drafts have been saved." }
            } @else { (drafts_table(drafts)) }
        }
    }
}

fn pending_table(rows: &[QueueRow<'_>]) -> Markup {
    let actions = html! {
        span class="sr-only" { "Actions" }
    };
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Project"))
            (table_head_cell("Last editor"))
            (table_head_cell("Submitted"))
            (table_head_cell("Status"))
            (table_head_cell(actions))
        }
    };
    let body = html! {
        @for row in rows { (pending_row(row)) }
    };
    html! {
        (table("Pending submissions").head(head).body(body))
    }
}

fn pending_row(row: &QueueRow<'_>) -> Markup {
    let name = html! {
        @match row.project_name {
            Some(name) => (name)
            None => span class="italic text-neutral-600" { "Not in the published set" }
        }
    };
    html! {
        tr {
            ({
                table_cell(
                    html! {
                        span class = "font-mono text-sm" { (row.shortcode) }
                    },
                )
            })
            (table_cell(name))
            (table_cell(editor_name(row.last_editor)))
            ({
                table_cell(
                    html! {
                        span class = "text-sm" { (row.submitted_at) }
                    },
                )
            })
            (table_cell(queue_status(row)))
            (table_cell(review_control(row)))
        }
    }
}

/// What the queue says about who holds a submission.
fn queue_status(row: &QueueRow<'_>) -> Markup {
    html! {
        @match (row.state, row.reviewer) {
            (SubmissionState::InReview, Some(reviewer)) => {
                (badge(format!("With {reviewer}")).variant(BadgeVariant::Info))
            }
            (SubmissionState::InReview, None) => (badge("In review").variant(BadgeVariant::Info))
            _ => (badge("Waiting").variant(BadgeVariant::Warning))
        }
    }
}

/// The control that opens a submission.
///
/// A `POST`, not a link: opening a submission claims it, and a claim is a state
/// change — a `GET` that changed state is one the `Sec-Fetch-Site` control
/// cannot cover, because a navigation from anywhere is a `GET`. It posts to the
/// submission's own review URL, which answers `GET`, so a refused claim
/// re-renders somewhere a reader can stay.
fn review_control(row: &QueueRow<'_>) -> Markup {
    let action = format!("/review/{}", row.shortcode);
    let label = if row.state == SubmissionState::InReview {
        "Open"
    } else {
        "Start review"
    };
    html! {
        form method="post" action=(action) {
            input type="hidden" name=(INTENT) value=(CLAIM);
            ({
                button(label)
                    .button_type(ButtonType::Submit)
                    .variant(ButtonVariant::Primary)
                    .aria_label(format!("{label} {}", row.shortcode))
            })
        }
    }
}

fn drafts_table(rows: &[DraftRow<'_>]) -> Markup {
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Project"))
            (table_head_cell("Last editor"))
            (table_head_cell("Last saved"))
        }
    };
    let body = html! {
        @for row in rows { (draft_row(row)) }
    };
    html! {
        (table("Drafts in progress").head(head).body(body))
    }
}

fn draft_row(row: &DraftRow<'_>) -> Markup {
    let name = html! {
        @match row.project_name {
            Some(name) => (link(name, format!("/projects/{}", row.shortcode)))
            None => (link("Not in the published set", format!("/projects/{}", row.shortcode)))
        }
    };
    html! {
        tr {
            ({
                table_cell(
                    html! {
                        span class = "font-mono text-sm" { (row.shortcode) }
                    },
                )
            })
            (table_cell(name))
            (table_cell(editor_name(row.last_editor)))
            ({
                table_cell(
                    html! {
                        span class = "text-sm" { (row.updated_at) }
                    },
                )
            })
        }
    }
}

/// An account name, or what is shown once the account is gone.
///
/// The row outlives its author by design — removing an account must not destroy
/// a project's work — so this states the absence rather than rendering an empty
/// cell that reads as a bug.
fn editor_name(name: Option<&str>) -> Markup {
    html! {
        @match name {
            Some(name) => (name)
            None => span class="italic text-neutral-600" { "Account removed" }
        }
    }
}

// --- The diff surface -----------------------------------------------------

/// The name the diff form posts its current filter under, so a save comes back
/// showing what the reviewer was looking at.
pub const SHOW: &str = "show";

/// The value [`Filter::All`] posts and reads.
pub const SHOW_ALL: &str = "all";

/// Which rows the surface is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Only the fields the submission changes. The default: a record has around
    /// thirty members and a submission usually changes three, so showing every
    /// row by default buries the ones that need a decision.
    Changed,
    /// Every field, changed or not.
    All,
}

impl Filter {
    /// The query string this filter is reached by, empty for the default.
    #[must_use]
    pub const fn query(self) -> &'static str {
        match self {
            Self::Changed => "",
            Self::All => "?show=all",
        }
    }
}

/// What the `POST` that led to this rendering did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice<'a> {
    /// The decisions were stored.
    Saved,
    /// The reviewer took the submission over from somebody else.
    Claimed,
    /// Nothing was stored, and why.
    Refused(&'a str),
}

/// One field, as the diff renders it.
pub struct ReviewRow<'a> {
    /// The project member name. The key for the decision control, the
    /// substituted value's control, and the stored decision.
    pub field: &'a str,
    /// The registry entry, or `None` for a member the form does not know —
    /// which is what a field added to the contract without an editor change
    /// looks like from here. It is what decides both the wording and the control, so the review
    /// surface and the depositor's form cannot render a field differently.
    pub registry: Option<&'static Field>,
    pub published: Option<&'a Value>,
    /// The submitted value, as the depositor sent it. Never replaced by the
    /// reviewer's substitute: the two are shown together, because there is no
    /// second approver and nobody else would ever see the change.
    pub submitted: Option<&'a Value>,
    /// What the reviewer put in place of it, or `None` where they left it.
    pub substitute: Option<&'a Value>,
    pub decision: Option<Decision>,
    pub changed: bool,
}

impl ReviewRow<'_> {
    /// The value that would be committed if this row is accepted.
    fn outgoing(&self) -> Option<&Value> {
        self.substitute.or(self.submitted)
    }

    /// The registry's wording, falling back to the member's own name.
    fn label(&self) -> &str {
        self.registry.map_or(self.field, |field| field.label)
    }

    /// Whether the reviewer can edit this field in place — true exactly where
    /// the depositor's form has a control for it, since it is that control.
    fn editable(&self) -> bool {
        self.registry.is_some_and(|field| field.shape.is_some())
    }
}

/// Everything one rendering of the diff surface needs.
pub struct ReviewView<'a> {
    /// As the URL spells it, which is what every control posts back to.
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    /// False for a local-only project: there is no published side, so there is
    /// nothing to revert to and no revert is offered.
    pub published: bool,
    pub submitted_by: Option<&'a str>,
    /// Already formatted.
    pub submitted_at: &'a str,
    /// Who holds the submission, `None` while nobody does.
    pub reviewer: Option<&'a str>,
    /// Whether the reader is the one holding it.
    pub held_by_viewer: bool,
    pub rows: &'a [ReviewRow<'a>],
    pub filter: Filter,
    pub notice: Option<Notice<'a>>,
}

impl ReviewView<'_> {
    fn action(&self) -> String {
        format!("/review/{}", self.shortcode)
    }

    fn changed(&self) -> usize {
        self.rows.iter().filter(|row| row.changed).count()
    }

    fn decided(&self, decision: Decision) -> usize {
        self.rows
            .iter()
            .filter(|row| row.changed && row.decision == Some(decision))
            .count()
    }

    fn visible(&self) -> impl Iterator<Item = &ReviewRow<'_>> {
        self.rows.iter().filter(move |row| self.filter == Filter::All || row.changed)
    }
}

/// The whole page: the submission's heading, then the region a save replaces.
pub fn page(view: &ReviewView<'_>) -> Markup {
    html! {
        div class="max-w-5xl py-8" { (heading(view)) (region(view)) }
    }
}

/// The summary, the status region and the diff form — everything a save can
/// change.
///
/// The counts are inside it deliberately: a save that accepted four fields and
/// left the tally at zero would be the review surface's version of the section
/// rail going stale, which is the failure the form's region boundary exists to
/// prevent.
pub fn region(view: &ReviewView<'_>) -> Markup {
    html! {
        section id=(REGION_ID) {
            (status(view))
            (unpublished_banner(view))
            (claim_banner(view))
            (summary(view))
            (diff_form(view))
        }
    }
}

fn heading(view: &ReviewView<'_>) -> Markup {
    html! {
        div class="mb-6" {
            p class="mb-2" {
                a href="/review" class="underline" { "Back to the review queue" }
            }
            @match view.project_name {
                Some(name) => {
                    h1 class="font-display text-2xl mb-1" { (name) }
                    p class="font-mono text-sm text-gray-600" { (view.shortcode) }
                }
                None => {
                    h1 class="font-display text-2xl mb-1" { "Project " (view.shortcode) }
                }
            }
            p class="text-gray-600 mt-2" {
                "Submitted by "
                @match view.submitted_by {
                    Some(name) => strong { (name) }
                    None => span class="italic" { "an account that has since been removed" }
                }
                " on "
                (view.submitted_at)
                "."
            }
        }
    }
}

/// The live region a save writes its outcome into.
///
/// Present from the first load and empty: an `aria-live` region announces a
/// *change* to content it already holds, and one morphed in together with its
/// text is widely reported not to announce at all. `empty:hidden` is what keeps
/// an always-present region free of a stray line box.
///
/// `AlertVariant::Warning` rather than `Danger` for a refusal, for the reason
/// the section form's status region carries: `Danger` has `role="alert"`, an
/// implicit assertive region, and screen readers do not agree on which
/// politeness wins when one is nested inside a polite one.
fn status(view: &ReviewView<'_>) -> Markup {
    html! {
        div class="empty:hidden" aria-live="polite" {
            @match view.notice {
                Some(Notice::Saved) => {
                    ({
                        alert("Review decisions saved.")
                            .variant(AlertVariant::Success)
                            .class("mb-4")
                    })
                }
                Some(Notice::Claimed) => {
                    ({
                        alert("You are now reviewing this submission.")
                            .variant(AlertVariant::Success)
                            .class("mb-4")
                    })
                }
                Some(Notice::Refused(message)) => {
                    (alert(message).variant(AlertVariant::Warning).class("mb-4"))
                }
                None => {}
            }
        }
    }
}

/// What a project with no published counterpart means for this review.
///
/// Stated once at the top rather than beside each field. It is a fact about the
/// *record*, not about any one field, so repeating it per row says the same
/// thing thirty times — and a paragraph rendered next to a control is not part
/// of that control's accessible description, so a reader tabbing straight to
/// the input never hears it at all. The per-row half is the "Not published yet"
/// column, which every reader gets.
fn unpublished_banner(view: &ReviewView<'_>) -> Markup {
    if view.published {
        return html! {};
    }
    html! {
        ({
            alert(
                    "This project is not in the published set this deployment carries, so there is nothing to compare \
                 against. Every field below is new, and no field can be reverted — there is no published value to \
                 revert to.",
                )
                .variant(AlertVariant::Info)
                .title("Nothing published to compare against")
                .class("mb-4")
        })
    }
}

/// Who holds the submission, and the way to take it over.
///
/// Visible rather than blocking, which is the whole of this service's answer to
/// two reviewers on one submission: there is no lock, decisions are
/// last-write-wins like a draft, and the one thing that must not happen
/// silently is a second reviewer overwriting the first without either of them
/// knowing. A lock would need a release path and a stale-lock timeout, and
/// would strand a submission whenever somebody closed a tab.
fn claim_banner(view: &ReviewView<'_>) -> Markup {
    let Some(reviewer) = view.reviewer else {
        return html! {};
    };
    if view.held_by_viewer {
        return html! {};
    }
    let message = format!(
        "{reviewer} picked this submission up. You can still read it and record decisions, and the last save wins — \
         take it over to make that visible in the queue."
    );
    let take_over = html! {
        form method="post" action=(view.action()) class="mt-2" {
            input type="hidden" name=(INTENT) value=(CLAIM);
            // The same reason the diff form carries it: a take-over that
            // dropped the filter would return a reviewer from "every field" to
            // the changed-only view, as if rows had disappeared.
            @if view.filter == Filter::All {
                input type="hidden" name=(SHOW) value=(SHOW_ALL);
            }
            (button("Take over the review").button_type(ButtonType::Submit))
        }
    };
    let body = html! {
        p { (message) }
        (take_over)
    };
    html! {
        ({
            alert(body)
                .variant(AlertVariant::Warning)
                .title("Somebody else is reviewing this")
                .class("mb-4")
        })
    }
}

/// The tally, and the control that switches between changed and all fields.
fn summary(view: &ReviewView<'_>) -> Markup {
    let changed = view.changed();
    let accepted = view.decided(Decision::Accept);
    let reverted = view.decided(Decision::Revert);
    let undecided = changed - accepted - reverted;
    let (other, other_label) = match view.filter {
        Filter::Changed => (Filter::All, "Show every field"),
        Filter::All => (Filter::Changed, "Show only changed fields"),
    };
    let other_href = format!("/review/{}{}", view.shortcode, other.query());
    html! {
        div class="flex flex-wrap items-center gap-4 mb-4" {
            p class="text-sm text-neutral-700" {
                strong { (accepted) " accepted" }
                @if undecided > 0 {
                    " · "
                    strong { (undecided) " to review" }
                }
                @if reverted > 0 {
                    " · "
                    strong { (reverted) " reverted" }
                }
                " of "
                (changed)
                (if changed == 1 { " change" } else { " changes" })
            }
            (link(other_label, other_href))
        }
    }
}

/// Every visible row, and the two controls that save them.
fn diff_form(view: &ReviewView<'_>) -> Markup {
    let action = view.action();
    let rows: Vec<&ReviewRow<'_>> = view.visible().collect();
    html! {
        // `contentType: 'form'` posts the body as `application/x-www-form-urlencoded`,
        // which is what `editor_core::form` reads; Datastar sends no signals on
        // a form-content-type request. No `__prevent`: the bundle calls
        // `preventDefault` unconditionally for a `submit` event on a form.
        form
            id="review-form"
            method="post"
            action=(action)
            class="flex flex-col gap-4"
            data-on:submit={ "@post('" (action) "', {contentType: 'form'})" }
        {
            // The filter travels with the save, or a reviewer looking at every
            // field is silently returned to the changed-only view by their own
            // save — on the enhanced path without even a navigation to explain
            // it.
            @if view.filter == Filter::All {
                input type="hidden" name=(SHOW) value=(SHOW_ALL);
            }
            @if rows.is_empty() {
                p class="text-gray-600" {
                    "This submission changes nothing. Its values are identical to what is published."
                }
            }
            @for row in &rows { (diff_row(view, row)) }
            @if view.changed() > 0 {
                div class="flex items-center gap-4 mt-2" {
                    ({
                        button("Save review decisions")
                            .variant(ButtonVariant::Primary)
                            .name_value(INTENT, SAVE)
                    })
                    ({
                        button("Accept all remaining")
                            .variant(ButtonVariant::Secondary)
                            .name_value(INTENT, ACCEPT_ALL)
                    })
                }
            }
        }
    }
}

/// One field: the header with its state, the two values, and the decision.
fn diff_row(view: &ReviewView<'_>, row: &ReviewRow<'_>) -> Markup {
    html! {
        div class="rounded border border-neutral-300 bg-white" {
            div class="flex flex-wrap items-center gap-3 border-b border-neutral-200 px-4 py-2" {
                strong { (row.label()) }
                (row_state(row))
            }
            (row_values(view, row))
            @if row.changed { (decision_control(view, row)) }
        }
    }
}

/// The badge naming where the row stands.
fn row_state(row: &ReviewRow<'_>) -> Markup {
    if !row.changed {
        return html! {
            (badge("Unchanged").variant(BadgeVariant::Secondary))
        };
    }
    html! {
        @match (row.decision, row.substitute.is_some()) {
            (Some(Decision::Accept), true) => (badge("Accepted · edited").variant(BadgeVariant::Success))
            (Some(Decision::Accept), false) => (badge("Accepted").variant(BadgeVariant::Success))
            (Some(Decision::Revert), _) => (badge("Reverted — keeps published").variant(BadgeVariant::Secondary))
            (None, _) => (badge("Needs review").variant(BadgeVariant::Warning))
        }
    }
}

/// The two columns: what is published, and what would be.
fn row_values(view: &ReviewView<'_>, row: &ReviewRow<'_>) -> Markup {
    html! {
        div class="grid gap-4 border-t border-neutral-200 p-4 md:grid-cols-2" {
            div {
                p class="mb-2 text-xs font-bold uppercase tracking-wide text-neutral-600" {
                    "Published"
                }
                @if view.published { (value_markup(row.published)) } @else {
                    // Not the same as a published project holding no value for
                    // this field: there is no published project at all, and a
                    // reader told "Not set" would look for the field rather
                    // than for the record.
                    p class="italic text-neutral-600" { "Not published yet" }
                }
            }
            div {
                p class="mb-2 text-xs font-bold uppercase tracking-wide text-neutral-600" {
                    "Submitted"
                }
                (submitted_side(row))
            }
        }
    }
}

/// The submitted value, as a control where the reviewer may edit it and as a
/// reading rendering everywhere else.
///
/// A row the reviewer has reverted renders read-only whatever its shape: the
/// value they would be editing is not the one that would be committed, and a
/// control that posts into a decision it contradicts is a trap.
fn submitted_side(row: &ReviewRow<'_>) -> Markup {
    let reverted = row.decision == Some(Decision::Revert);
    if !row.changed || reverted || !row.editable() {
        return html! {
            (value_markup(row.outgoing()))
            @if row.substitute.is_some() && !reverted { (submitted_original(row)) }
        };
    }
    html! {
        (editor(row))
        (submitted_original(row))
    }
}

/// What the depositor actually sent, shown beneath the reviewer's replacement.
///
/// Only where the two differ. There is no second approver, so a value RDU
/// substituted is seen by nobody unless the submitted one stays beside it — and
/// the depositor is shown the same pair later.
fn submitted_original(row: &ReviewRow<'_>) -> Markup {
    if row.substitute.is_none() {
        return html! {};
    }
    html! {
        details class="mt-2 text-sm" {
            summary class="cursor-pointer text-neutral-700" { "What the depositor submitted" }
            div class="mt-1" { (value_markup(row.submitted)) }
        }
    }
}

/// The in-place editor for one row.
///
/// **The form's own control**, over the value that would be committed, rather
/// than a second dispatch here. The second one diverged the moment it existed:
/// it keyed off whether the value happened to contain a newline, so `startDate`
/// rendered as free text where the form gives a date picker, and
/// `shortDescription` lost the 200-character cap its own hint promises — with
/// nothing server-side to catch either, because the cap is an HTML attribute.
///
/// The control posts under the field's own name, so `editor_core::form`'s
/// appliers read a reviewer's edit exactly as they read a depositor's: the same
/// trimming, the same newline normalisation, the same placeholder rules.
fn editor(row: &ReviewRow<'_>) -> Markup {
    let Some(field) = row.registry else { return html! {} };
    let Some(shape) = field.shape else { return html! {} };
    // A one-member draft holding what would be committed. `set` with a
    // `Value::Null` removes the member, which is exactly a substitution that
    // cleared the field: the control then renders empty, as the form's does.
    let mut outgoing = ProjectDraft::default();
    if let Some(value) = row.outgoing() {
        outgoing.set(field.id, value.clone());
    }
    control(field, &outgoing, shape)
}

/// Accept / revert / undecided for one field.
///
/// A radio group and not a pair of buttons, because the three states have to be
/// distinguishable and reversible: a button that has been pressed says nothing
/// about a field being *back* to undecided, and a revert is not the absence of
/// an accept. The tile's own documentation is the reason the third
/// choice is explicit — a radio group cannot be returned to unset, so "not
/// reviewed yet" has to be a choice of its own.
///
/// Revert is absent entirely where nothing is published: it means "keep the
/// published value", and there is none.
fn decision_control(view: &ReviewView<'_>, row: &ReviewRow<'_>) -> Markup {
    let legend = format!("{} — decision", row.label());
    let selected = row.decision.map_or("", Decision::as_str);
    let mut group = radio_group(format!("{DECISION_PREFIX}.{}", row.field), legend)
        .inline()
        .option(Decision::Accept.as_str(), "Accept")
        .selected(selected);
    if view.published {
        group = group.option(Decision::Revert.as_str(), "Revert");
    }
    group = group.option("", "Not reviewed yet");
    html! {
        div class="border-t border-neutral-200 px-4 py-3" { (group) }
    }
}

/// The reviewer's note, as the depositor's form shows it.
///
/// Here rather than in [`section`](super::section) because it belongs to the
/// review round: the note is written by request-changes and read on the form,
/// and putting both in the module that owns the review vocabulary keeps the
/// wording in one place.
#[must_use]
pub fn reviewer_note(note: &str) -> Markup {
    html! {
        ({
            alert(
                    html! {
                        p class = "whitespace-pre-line" { (note) }
                    },
                )
                .variant(AlertVariant::Info)
                .title("RDU asked for changes")
                .class("mb-4")
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn queue_row() -> QueueRow<'static> {
        QueueRow {
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
            last_editor: Some("A Depositor"),
            submitted_at: "2026-09-03 08:15 UTC",
            reviewer: None,
            state: SubmissionState::Submitted,
        }
    }

    fn draft_row() -> DraftRow<'static> {
        DraftRow {
            shortcode: "080C",
            project_name: Some("Anton Webern"),
            last_editor: Some("A Depositor"),
            updated_at: "2026-09-02 11:00 UTC",
        }
    }

    static NAME: &str = "New name";
    static OLD_NAME: &str = "Old name";

    fn rows() -> Vec<ReviewRow<'static>> {
        static NEW: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| json!(NAME));
        static OLD: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| json!(OLD_NAME));
        static NEW_ABSTRACT: std::sync::LazyLock<Value> =
            std::sync::LazyLock::new(|| json!({ "en": "A new abstract.", "de": "Ein neues Abstract." }));
        static OLD_ABSTRACT: std::sync::LazyLock<Value> =
            std::sync::LazyLock::new(|| json!({ "en": "An old abstract." }));
        static UNCHANGED: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| json!("0801d"));
        vec![
            ReviewRow {
                field: "name",
                registry: crate::form::registry::field("name"),
                published: Some(&OLD),
                submitted: Some(&NEW),
                substitute: None,
                decision: None,
                changed: true,
            },
            ReviewRow {
                field: "abstract",
                registry: crate::form::registry::field("abstract"),
                published: Some(&OLD_ABSTRACT),
                submitted: Some(&NEW_ABSTRACT),
                substitute: None,
                decision: None,
                changed: true,
            },
            ReviewRow {
                field: "shortcode",
                registry: crate::form::registry::field("shortcode"),
                published: Some(&UNCHANGED),
                submitted: Some(&UNCHANGED),
                substitute: None,
                decision: None,
                changed: false,
            },
        ]
    }

    fn view<'a>(rows: &'a [ReviewRow<'a>]) -> ReviewView<'a> {
        ReviewView {
            shortcode: "0801d",
            project_name: Some("Bernoulli-Euler Online"),
            published: true,
            submitted_by: Some("A Depositor"),
            submitted_at: "2026-09-03 08:15 UTC",
            reviewer: None,
            held_by_viewer: false,
            rows,
            filter: Filter::Changed,
            notice: None,
        }
    }

    #[test]
    fn the_queue_shows_what_req_4_1_asks_for() {
        let out = queue(&[queue_row()], &[draft_row()]).into_string();
        assert!(out.contains("0801d"), "{out}");
        assert!(out.contains("A Depositor"), "{out}");
        assert!(out.contains("2026-09-03 08:15 UTC"), "{out}");
    }

    #[test]
    fn the_queue_lists_drafts_as_well_as_submissions() {
        // RDU sees every draft, so it can help a depositor who is stuck before
        // submitting.
        let out = queue(&[], &[draft_row()]).into_string();
        assert!(out.contains("Drafts in progress"), "{out}");
        assert!(out.contains(r#"href="/projects/080C""#), "{out}");
        assert!(out.contains("No submissions are waiting for review"), "{out}");
    }

    #[test]
    fn opening_a_submission_is_a_post_and_not_a_link() {
        // A claim changes state, and the same-origin CSRF control exempts `GET`
        // by necessity — a navigation from anywhere is a `GET`.
        let out = queue(&[queue_row()], &[]).into_string();
        assert!(out.contains(r#"<form method="post" action="/review/0801d""#), "{out}");
        assert!(out.contains(r#"value="claim""#), "{out}");
        assert!(!out.contains(r#"<a href="/review/0801d""#), "{out}");
    }

    #[test]
    fn a_submission_somebody_holds_says_who_has_it() {
        let mut row = queue_row();
        row.state = SubmissionState::InReview;
        row.reviewer = Some("A Reviewer");
        let out = queue(&[row], &[]).into_string();
        assert!(out.contains("With A Reviewer"), "{out}");
        assert!(out.contains("Open"), "{out}");
    }

    #[test]
    fn a_removed_account_reads_as_removed_rather_than_blank() {
        // The row outlives its author by design; an empty cell reads as a bug.
        let mut row = queue_row();
        row.last_editor = None;
        let out = queue(&[row], &[]).into_string();
        assert!(out.contains("Account removed"), "{out}");
    }

    #[test]
    fn the_diff_posts_the_whole_form_rather_than_one_request_per_field() {
        // The batching the per-field controls need: every decision and
        // every substituted value arrives in one body, so partial failure is one
        // server-side transaction rather than N independent ones.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains(r#"action="/review/0801d""#), "{out}");
        assert!(out.contains(r#"method="post""#), "{out}");
        assert_eq!(out.matches("<form").count(), 1, "one form, not one per row: {out}");
    }

    #[test]
    fn the_enhanced_path_posts_the_form_body_rather_than_signals() {
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(
            out.contains(r#"data-on:submit="@post('/review/0801d', {contentType: 'form'})""#),
            "{out}"
        );
        // Keyed plugin attributes use `:`, not `-`. The hyphen form is a console
        // error and an inert control, and a test asserting presence passes either way.
        assert!(!out.contains("data-on-submit"), "{out}");
    }

    #[test]
    fn accept_all_is_a_named_submit_on_the_same_form() {
        // Both paths carry it: a native submit posts the activated button's
        // name and value, and Datastar 1.0.2's form mode appends the
        // `SubmitEvent`'s submitter to the body it builds. A `formaction` would
        // work on the plain path only — the bundle posts to the URL in
        // `@post`, so the second destination would be silently ignored.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains(r#"name="intent" value="save""#), "{out}");
        assert!(out.contains(r#"name="intent" value="accept-all""#), "{out}");
    }

    #[test]
    fn only_changed_fields_are_shown_by_default() {
        // Around thirty members, usually three of them changed: showing every
        // row by default buries the ones that need a decision.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains("Name"), "{out}");
        assert!(!out.contains("Shortcode"), "{out}");
        assert!(out.contains("Show every field"), "{out}");
    }

    #[test]
    fn showing_every_field_includes_the_unchanged_ones_without_a_decision() {
        let rows = rows();
        let mut view = view(&rows);
        view.filter = Filter::All;
        let out = page(&view).into_string();
        assert!(out.contains("Shortcode"), "{out}");
        assert!(out.contains("Unchanged"), "{out}");
        assert!(!out.contains("decision.shortcode"), "{out}");
    }

    #[test]
    fn a_changed_field_offers_accept_revert_and_not_reviewed_yet() {
        // Three states, all reversible. A radio group cannot be returned to
        // unset, so "not reviewed yet" has to be a choice of its own.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains(r#"name="decision.name""#), "{out}");
        assert!(out.contains(r#"value="accept""#), "{out}");
        assert!(out.contains(r#"value="revert""#), "{out}");
        assert!(out.contains("Not reviewed yet"), "{out}");
    }

    #[test]
    fn an_editable_field_posts_under_its_own_name() {
        // The same name the section form posts, so `editor_core::form`'s
        // appliers read a reviewer's edit exactly as they read a depositor's.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains(r#"name="name""#), "{out}");
        assert!(out.contains(r#"value="New name""#), "{out}");
    }

    #[test]
    fn a_multilingual_field_posts_one_control_per_tag_on_either_side() {
        // The tags rendered are the ones the two sides carry, not a closed set:
        // a tag with no control posts nothing, and the map is rebuilt from the
        // body, so a language only the published value has would be dropped.
        let rows = rows();
        let out = page(&view(&rows)).into_string();
        assert!(out.contains(r#"name="abstract.en""#), "{out}");
        assert!(out.contains(r#"name="abstract.de""#), "{out}");
    }

    #[test]
    fn a_reverted_field_renders_read_only() {
        // The value the reviewer would be editing is not the one that would be
        // committed, so a control posting into a decision that discards it is a
        // trap.
        let mut rows = rows();
        rows[0].decision = Some(Decision::Revert);
        let out = page(&view(&rows)).into_string();
        assert!(out.contains("Reverted — keeps published"), "{out}");
        assert!(!out.contains(r#"name="name""#), "{out}");
    }

    #[test]
    fn a_substituted_value_is_shown_beside_what_the_depositor_sent() {
        // There is no second approver, so a value RDU put in place of the
        // depositor's is seen by nobody unless the submitted one stays beside
        // it.
        let substitute = json!("A reviewer's wording");
        let mut rows = rows();
        rows[0].decision = Some(Decision::Accept);
        rows[0].substitute = Some(&substitute);
        let out = page(&view(&rows)).into_string();
        assert!(out.contains("Accepted · edited"), "{out}");
        assert!(out.contains("What the depositor submitted"), "{out}");
        assert!(out.contains("New name"), "{out}");
        assert!(out.contains(r#"value="A reviewer's wording""#), "{out}");
    }

    #[test]
    fn an_unpublished_project_offers_no_revert_and_says_why() {
        // A local-only project: revert means keeping the published value, and
        // there is none — offering it would silently unset a field the contract
        // requires.
        let rows = rows();
        let mut view = view(&rows);
        view.published = false;
        view.project_name = None;
        let out = page(&view).into_string();
        assert!(out.contains("Not published yet"), "{out}");
        // Said once, at the top: it is a fact about the record, not about any
        // one field, and a paragraph beside a control is not part of that
        // control's accessible description.
        assert_eq!(out.matches("Nothing published to compare against").count(), 1, "{out}");
        assert!(out.contains("no field can be reverted"), "{out}");
        assert!(!out.contains(r#"value="revert""#), "{out}");
        assert!(out.contains(r#"value="accept""#), "{out}");
    }

    #[test]
    fn a_second_reviewer_is_told_who_has_it_and_is_not_blocked() {
        // No lock: a lock needs a release path and a stale-lock timeout, and
        // would strand a submission whenever somebody closed a tab. What must
        // not happen silently is one reviewer overwriting another.
        let rows = rows();
        let mut view = view(&rows);
        view.reviewer = Some("Another Reviewer");
        let out = page(&view).into_string();
        assert!(out.contains("Another Reviewer picked this submission up"), "{out}");
        assert!(out.contains("Take over the review"), "{out}");
        assert!(out.contains("Save review decisions"), "{out}");
    }

    #[test]
    fn the_holder_is_not_told_somebody_else_has_it() {
        let rows = rows();
        let mut view = view(&rows);
        view.reviewer = Some("The Viewer");
        view.held_by_viewer = true;
        let out = page(&view).into_string();
        assert!(!out.contains("Take over the review"), "{out}");
    }

    #[test]
    fn the_tally_counts_the_changed_rows_only() {
        let mut rows = rows();
        rows[0].decision = Some(Decision::Accept);
        let out = page(&view(&rows)).into_string();
        assert!(out.contains("1 accepted"), "{out}");
        assert!(out.contains("1 to review"), "{out}");
        assert!(out.contains("of 2 changes"), "{out}");
    }

    #[test]
    fn a_submission_changing_nothing_says_so_rather_than_rendering_an_empty_form() {
        let rows = rows();
        let unchanged: Vec<ReviewRow<'_>> = rows.into_iter().filter(|row| !row.changed).collect();
        let out = page(&view(&unchanged)).into_string();
        assert!(out.contains("This submission changes nothing"), "{out}");
        assert!(!out.contains("Save review decisions"), "{out}");
    }

    #[test]
    fn the_status_region_is_present_and_empty_from_the_first_load() {
        // An `aria-live` region announces a change to content it already holds;
        // one morphed in together with its text is widely reported not to
        // announce at all.
        let rows = rows();
        let out = region(&view(&rows)).into_string();
        assert!(out.contains(r#"<div class="empty:hidden" aria-live="polite"></div>"#), "{out}");
    }

    #[test]
    fn a_refusal_is_a_warning_and_not_a_danger_alert() {
        // `Danger` carries `role="alert"`, an implicit assertive region, and
        // screen readers do not agree on which politeness wins when one is
        // nested inside a polite one.
        let rows = rows();
        let mut view = view(&rows);
        view.notice = Some(Notice::Refused("Nothing was saved."));
        let out = region(&view).into_string();
        assert!(out.contains("alert-warning"), "{out}");
        assert!(!out.contains(r#"role="alert""#), "{out}");
    }

    #[test]
    fn the_reviewer_note_names_what_it_is() {
        // The note has no other home, and a bare paragraph on the form would
        // read as one more hint.
        let out = reviewer_note("Please add a German description.").into_string();
        assert!(out.contains("RDU asked for changes"), "{out}");
        assert!(out.contains("Please add a German description."), "{out}");
    }

    #[test]
    fn snapshot_the_review_queue() {
        insta::assert_snapshot!("review_queue", queue(&[queue_row()], &[draft_row()]).into_string());
    }

    #[test]
    fn snapshot_the_diff_surface_including_a_multi_language_field() {
        let rows = rows();
        insta::assert_snapshot!("review_diff", page(&view(&rows)).into_string());
    }

    #[test]
    fn snapshot_a_project_with_no_published_counterpart() {
        let rows = rows();
        let mut view = view(&rows);
        view.published = false;
        view.project_name = None;
        insta::assert_snapshot!("review_diff_unpublished", page(&view).into_string());
    }

    #[test]
    fn snapshot_a_saved_region_as_the_enhanced_path_sends_it() {
        let mut rows = rows();
        rows[0].decision = Some(Decision::Accept);
        let mut view = view(&rows);
        view.notice = Some(Notice::Saved);
        insta::assert_snapshot!("review_diff_saved", region(&view).into_string());
    }
}
