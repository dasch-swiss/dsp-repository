//! The review surfaces: `GET /review`, and `GET`/`POST /review/{shortcode}`.
//!
//! Every handler here takes [`Rdu`](crate::auth::guard::Rdu), which puts the
//! access rule in one place: RDU access is role-based rather than per-project,
//! so there is no assignment to check and no per-project 403 to render. A depositor's
//! session gets the 403 page from the extractor, and no session is redirected
//! to login.
//!
//! Four things fail quietly if changed:
//!
//! - **One `POST` URL, three intents.** Claim, save and accept-all all post to
//!   `/review/{shortcode}`, discriminated by the `intent` pair the submit control carries. A second
//!   write URL would need a `GET` of its own, or a refused write would strand a reviewer on a bare
//!   405.
//! - **The submitted payload is never rewritten.** A reviewer's substitution goes to
//!   `review_state`; overwriting `payload` would destroy the depositor's own value, which is the
//!   one thing the absence of a second approver makes it necessary to keep.
//! - **A substitution is computed by running the form's own applier** over a clone of the submitted
//!   draft. That is what makes a reviewer's edit obey the same trimming, newline and placeholder
//!   rules a depositor's does — rules whose whole purpose is that an untouched value writes no
//!   bytes.
//! - **The enhanced path answers 200 even when it refuses**, and the plain path redirects after a
//!   write. Same two rules as the project form, for the same two reasons.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, Utc};
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody};
use editor_core::records::{
    normalize_shortcode, ApprovedRecord, DraftRecord, ReviewOutcome, ReviewRound, Submission, SubmissionState, User,
};
use editor_core::repository::{
    DraftRepository, RepositoryError, ReviewRoundRepository, SubmissionRepository, Transition, UserRepository,
};
use editor_core::review::{diff, Decision, FieldDiff, FieldReview, ReviewState};
use editor_web::form::{registry, INTENT};
use editor_web::pages::review as page;
use platform_metadata::is_valid_shortcode;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::guard::Rdu;
use crate::AppState;

/// The header the vendored Datastar bundle sets on every fetch it makes; see
/// [`crate::sections`], which reads it for the same reason.
const DATASTAR_REQUEST: &str = "datastar-request";

const NO_SUBMISSION: &str = "There is no submission waiting for review on this project. It may have just been \
                             withdrawn by the depositor, or reviewed by another RDU member.";
const SAVE_REFUSED_STORAGE: &str = "The review decisions could not be saved. Nothing was changed — try again, and if \
                                    it keeps happening the service needs attention.";
const SAVE_REFUSED_GONE: &str = "This submission is no longer waiting for review — somebody withdrew or finished it \
                                 while this page was open. Nothing was saved.";
const APPROVE_REFUSED_UNDECIDED: &str = "Every change has to be decided before this can be approved. Approving an \
                                         undecided field would commit a value nobody looked at — use \"Accept all \
                                         remaining\" if that is what you mean.";
const NOTE_REQUIRED: &str = "A note is required: it is the only thing the depositor will see. Nothing was changed.";
const FINISH_REFUSED_STORAGE: &str = "The review could not be recorded, so this submission is still in the queue. \
                                      Nothing was changed — try again, and if it keeps happening the service needs \
                                      attention.";
const FINISH_REFUSED_UNWRITABLE: &str = "The approved record could not be built from this submission, so nothing \
                                         was changed. The service needs attention before this project can be \
                                         approved.";

/// `GET /review` — the queue and the drafts.
pub(crate) async fn queue(State(state): State<AppState>, Rdu(user): Rdu) -> Response {
    let submissions = match SubmissionRepository::list(&*state.db).await {
        Ok(submissions) => submissions,
        Err(error) => return storage_error(&state, &user, "read the review queue", &error),
    };
    let drafts = match DraftRepository::list(&*state.db).await {
        Ok(drafts) => drafts,
        Err(error) => return storage_error(&state, &user, "read the drafts", &error),
    };
    let names = match account_names(&state).await {
        Ok(names) => names,
        Err(error) => return storage_error(&state, &user, "read the accounts", &error),
    };

    // Owned first, borrowed second: a row borrows its formatted instant and its
    // account name, and neither can be produced inside the `map` that builds it.
    let pending: Vec<PendingRow> = submissions
        .iter()
        // The queue lists *pending* submissions. An approved record is waiting to
        // be collected into a pull request, not for a reviewer, so a row for it
        // here would be a queue entry nobody can clear.
        .filter(|submission| submission.state != SubmissionState::Approved)
        .map(|submission| PendingRow {
            shortcode: shortcode_as_published(&state, &submission.shortcode),
            project_name: project_name(&state, &submission.shortcode).map(str::to_string),
            last_editor: name_of(&names, submission.submitted_by).map(str::to_string),
            submitted_at: crate::format_instant(submission.submitted_at),
            reviewer: name_of(&names, submission.reviewed_by).map(str::to_string),
            state: submission.state,
        })
        .collect();
    let draft_rows: Vec<DraftStrings> = drafts
        .iter()
        .map(|draft| DraftStrings {
            shortcode: shortcode_as_published(&state, &draft.shortcode),
            project_name: project_name(&state, &draft.shortcode).map(str::to_string),
            last_editor: name_of(&names, draft.updated_by).map(str::to_string),
            updated_at: crate::format_instant(draft.updated_at),
        })
        .collect();

    let pending_view: Vec<page::QueueRow<'_>> = pending
        .iter()
        .map(|row| page::QueueRow {
            shortcode: &row.shortcode,
            project_name: row.project_name.as_deref(),
            last_editor: row.last_editor.as_deref(),
            submitted_at: &row.submitted_at,
            reviewer: row.reviewer.as_deref(),
            state: row.state,
        })
        .collect();
    let drafts_view: Vec<page::DraftRow<'_>> = draft_rows
        .iter()
        .map(|row| page::DraftRow {
            shortcode: &row.shortcode,
            project_name: row.project_name.as_deref(),
            last_editor: row.last_editor.as_deref(),
            updated_at: &row.updated_at,
        })
        .collect();

    crate::render(
        &state,
        "Review queue — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user),
        page::queue(&pending_view, &drafts_view),
    )
}

/// One pending row's owned strings, so the view can borrow them.
struct PendingRow {
    shortcode: String,
    project_name: Option<String>,
    last_editor: Option<String>,
    submitted_at: String,
    reviewer: Option<String>,
    state: SubmissionState,
}

/// The same for a draft, which has neither a reviewer nor a submission state.
struct DraftStrings {
    shortcode: String,
    project_name: Option<String>,
    last_editor: Option<String>,
    updated_at: String,
}

/// Which rows `GET /review/{shortcode}` shows.
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct ShowParams {
    /// `all` shows every field; anything else, including absence, shows only
    /// the changed ones.
    #[serde(default)]
    show: Option<String>,
}

/// `GET /review/{shortcode}` — the field-by-field diff.
pub(crate) async fn show(
    State(state): State<AppState>,
    Rdu(user): Rdu,
    Path(shortcode): Path<String>,
    Query(params): Query<ShowParams>,
) -> Response {
    let context = match context(&state, &user, &shortcode).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let filter = if params.show.as_deref() == Some("all") {
        page::Filter::All
    } else {
        page::Filter::Changed
    };
    render_page(&state, &user, &context, filter, None)
}

/// `POST /review/{shortcode}` — claim the submission, or record decisions.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "review decision",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        review.intent = tracing::field::Empty,
        review.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn act(
    State(state): State<AppState>,
    Rdu(user): Rdu,
    Path(shortcode): Path<String>,
    headers: HeaderMap,
    // Last, because it consumes the body. A pair list rather than a struct, for
    // the reason `editor_core::form` gives: `serde_urlencoded` errors on a
    // repeated key and cannot deserialize a struct holding a `Vec`.
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(&shortcode));

    let mut context = match context(&state, &user, &shortcode).await {
        Ok(context) => context,
        Err(response) => {
            span.record("review.outcome", "refused");
            return response;
        }
    };

    let body = FormBody::from_pairs(pairs);
    // Carried in the body rather than re-derived: the form knows which view it
    // was rendered into, and a save that silently returned a reviewer from
    // "every field" to "changed only" would look like rows disappearing.
    let filter = if body.get(page::SHOW) == Some(page::SHOW_ALL) {
        page::Filter::All
    } else {
        page::Filter::Changed
    };
    let intent = body.get(INTENT).unwrap_or(page::SAVE);
    span.record("review.intent", tracing::field::display(intent));

    // The decisions on screen are read before any branch, so a terminating
    // action acts on what the reviewer is looking at rather than on what was
    // last saved. Approve especially: the two differ exactly when somebody
    // decided a row and went straight to Approve without saving first.
    if matches!(intent, page::APPROVE | page::REQUEST_CHANGES | page::REJECT) {
        context.state = decisions_from(&body, &context.rows, &context.submitted, false, context.published.is_some());
        return finish(&state, &user, &context, &body, filter, headers, intent).await;
    }

    let now = Utc::now();
    let mut submission = context.submission.clone();
    let notice = if intent == page::CLAIM {
        page::Notice::Claimed
    } else {
        let accept_all = intent == page::ACCEPT_ALL;
        context.state = decisions_from(
            &body,
            &context.rows,
            &context.submitted,
            accept_all,
            context.published.is_some(),
        );
        submission.review_state = match serde_json::to_string(&context.state) {
            Ok(_) if context.state.is_empty() => None,
            Ok(stored) => Some(stored),
            Err(error) => {
                span.record("review.outcome", "serialize_failed");
                tracing::error!(error = %error, "review decisions could not be serialized");
                return refused(&state, &user, &context, filter, headers, SAVE_REFUSED_STORAGE);
            }
        };
        // The note as typed, kept on the pending submission. It is the *working*
        // note — the one the round takes a copy of when it ends — and it is
        // saved here because backing out of a confirmation posts a plain save:
        // without this, declining a reject discards a carefully written reason
        // and the reviewer has to write it again from memory. Absent from the
        // body leaves the stored one alone, so a save from the diff form (which
        // renders no note control) cannot clear it.
        if let Some(note) = body.get(page::NOTE) {
            submission.reviewer_note = (!note.trim().is_empty()).then(|| note.trim().to_string());
        }
        page::Notice::Saved
    };

    // Recording anything about a submission is reviewing it, so both intents
    // claim it. That is what gives `SubmissionState::InReview` a producer, and
    // what makes a second reviewer's banner appear without a lock: nothing is
    // blocked, the last save wins, and the queue says who touched it last.
    submission.state = SubmissionState::InReview;
    submission.reviewed_by = Some(user.id);
    submission.reviewed_at = Some(now);

    match SubmissionRepository::update(&*state.db, &submission).await {
        Ok(()) => {
            span.record("review.outcome", "saved");
            tracing::info!(
                review.decided = context.state.count(&context.changed_fields(), Decision::Accept)
                    + context.state.count(&context.changed_fields(), Decision::Revert),
                "recorded a review decision"
            );
            // Only now: on a refusal nothing was written, so a page claiming
            // this reader holds the submission would hide the take-over banner
            // and name them as the reviewer while the row still says somebody
            // else has it.
            context.submission = submission;
            context.reviewer_name = Some(user.name.clone());
            context.held_by_viewer = true;
            saved(&context, filter, headers, notice)
        }
        // The submission went while this page was open — withdrawn, or finished
        // by another reviewer. Reported rather than retried: there is nothing
        // left to decide, and re-creating it would resurrect a record somebody
        // deliberately removed.
        Err(RepositoryError::NotFound { .. }) => {
            span.record("review.outcome", "gone");
            refused(&state, &user, &context, filter, headers, SAVE_REFUSED_GONE)
        }
        Err(error) => {
            span.record("review.outcome", "store_failed");
            tracing::error!(error = %error, "could not record a review decision");
            refused(&state, &user, &context, filter, headers, SAVE_REFUSED_STORAGE)
        }
    }
}

/// End the review round: approve (REQ-4.4), request changes (REQ-4.5) or
/// reject (REQ-4.6).
///
/// Each asks for confirmation first, on the same URL, so a refused write
/// re-renders somewhere that still answers `GET`. The confirmation is also
/// where the note is collected — required for the two outcomes the depositor
/// has to be told about, and enforced here and not only by the `required`
/// attribute the control carries.
async fn finish(
    state: &AppState,
    user: &User,
    context: &Context<'_>,
    body: &FormBody,
    filter: page::Filter,
    headers: HeaderMap,
    intent: &str,
) -> Response {
    let span = tracing::Span::current();

    // Not the second post yet: render the prompt, carrying whatever is typed.
    if body.get(page::CONFIRMED).is_none() {
        span.record("review.outcome", "confirming");
        let prompt = Prompt { intent, note: working_note(context, body), refusal: None };
        return asking(state, user, context, filter, headers, prompt);
    }

    let note = working_note(context, body);
    let outcome = match intent {
        page::APPROVE => ReviewOutcome::Approved,
        page::REQUEST_CHANGES => ReviewOutcome::ChangesRequested,
        _ => ReviewOutcome::Rejected,
    };

    if outcome != ReviewOutcome::Approved && note.trim().is_empty() {
        span.record("review.outcome", "note_required");
        tracing::info!(review.outcome = %outcome, "refused a review round with no note");
        // The prompt again *with the reason*: re-rendered silently it looks
        // like the button did nothing, which is the one reading that leaves a
        // reviewer with no next step.
        let prompt = Prompt { intent, note, refusal: Some(NOTE_REQUIRED) };
        return asking(state, user, context, filter, headers, prompt);
    }

    // Approving an undecided change commits bytes nobody looked at, which is
    // the one thing a field-by-field surface exists to prevent. Only approve:
    // request-changes and reject do not commit anything, and an undecided row
    // is a perfectly ordinary state to return or discard a submission in.
    if outcome == ReviewOutcome::Approved {
        let changed = context.changed_fields();
        let undecided = changed.len()
            - context.state.count(&changed, Decision::Accept)
            - context.state.count(&changed, Decision::Revert);
        if undecided > 0 {
            span.record("review.outcome", "undecided");
            tracing::info!(review.undecided = undecided, "refused an approval with undecided changes");
            return refused(state, user, context, filter, headers, APPROVE_REFUSED_UNDECIDED);
        }
    }

    let round = ReviewRound {
        id: Uuid::new_v4(),
        shortcode: context.submission.shortcode.clone(),
        submission_id: context.submission.id,
        outcome,
        note: (!note.trim().is_empty()).then(|| note.trim().to_string()),
        // The decisions as they stand on screen, snapshotted: the submission
        // row carrying them is deleted by the same transaction, so this is the
        // only remaining answer to what was accepted and what RDU put in place
        // of the depositor's values.
        review_state: match serde_json::to_string(&context.state) {
            Ok(_) if context.state.is_empty() => None,
            Ok(stored) => Some(stored),
            Err(error) => {
                span.record("review.outcome", "serialize_failed");
                tracing::error!(error = %error, "review decisions could not be serialized");
                return refused(state, user, context, filter, headers, SAVE_REFUSED_STORAGE);
            }
        },
        actor: Some(user.id),
        at: Utc::now(),
    };

    let transition = match outcome {
        ReviewOutcome::Approved => {
            let Some(record) = approved_record(context, user, round.at) else {
                span.record("review.outcome", "serialize_failed");
                return refused(state, user, context, filter, headers, FINISH_REFUSED_UNWRITABLE);
            };
            ReviewRoundRepository::approve(&*state.db, context.submission.id, &record, &round).await
        }
        ReviewOutcome::ChangesRequested => {
            let Some(draft) = returned_draft(context, user, round.at) else {
                span.record("review.outcome", "serialize_failed");
                return refused(state, user, context, filter, headers, FINISH_REFUSED_UNWRITABLE);
            };
            ReviewRoundRepository::request_changes(&*state.db, context.submission.id, &draft, &round).await
        }
        // Reject leaves both the draft and the published metadata alone
        // (REQ-4.6, REQ-1.13): the note is what the depositor gets, and their
        // work is still theirs to resubmit.
        ReviewOutcome::Rejected | ReviewOutcome::Withdrawn => {
            ReviewRoundRepository::discard(&*state.db, context.submission.id, &round).await
        }
    };

    match transition {
        Ok(Transition::Applied) => {
            span.record("review.outcome", outcome.as_str());
            tracing::info!(
                review.outcome = %outcome,
                submission.id = %context.submission.id,
                review.accepted = context.state.count(&context.changed_fields(), Decision::Accept),
                "finished a review round"
            );
            finished(state, user, context, outcome)
        }
        // The terminal-state guard. Somebody else finished this submission
        // while the page was open, and nothing was written — which is what
        // stops a reject destroying a record already served into a pull
        // request, and a request-changes resurrecting one as a draft.
        Ok(Transition::AlreadyReviewed) => {
            span.record("review.outcome", "already_reviewed");
            tracing::info!("refused a review round on a submission somebody else had finished");
            refused(state, user, context, filter, headers, SAVE_REFUSED_GONE)
        }
        Err(error) => {
            span.record("review.outcome", "store_failed");
            tracing::error!(error = %error, "could not record a review round");
            refused(state, user, context, filter, headers, FINISH_REFUSED_STORAGE)
        }
    }
}

/// The note the reviewer is working on: what this body carries, or what a
/// previous save stored.
///
/// The fallback is what makes the prompt survive being declined: the diff form
/// renders no note control, so backing out of a confirmation and opening it
/// again posts nothing under [`page::NOTE`] and would otherwise show an empty
/// box where a written reason had been.
fn working_note<'a>(context: &'a Context<'_>, body: &'a FormBody) -> &'a str {
    body.get(page::NOTE)
        .or(context.submission.reviewer_note.as_deref())
        .unwrap_or_default()
}

/// What an approval commits: the submitted draft with every decision applied.
///
/// Not the submitted payload. An accepted field takes the reviewer's substitute
/// where there is one, and a reverted field goes back to the published value —
/// so the record is what RDU decided, while `submissions.payload` stayed the
/// depositor's own until the moment it was deleted.
///
/// `None` only if the result will not serialize, which cannot happen for a
/// draft that already did.
fn approved_record(context: &Context<'_>, user: &User, at: DateTime<Utc>) -> Option<ApprovedRecord> {
    let mut decided = context.submitted.clone();
    for row in context.rows.iter().filter(|row| row.changed()) {
        match context.state.decision(&row.field) {
            Some(Decision::Accept) => {
                if let Some(substitute) = context.state.substitute(&row.field) {
                    // `Null` is a reviewer clearing an optional field, which is
                    // a removal and not a stored null — the canonical writer
                    // strips nulls, so writing one would differ from what the
                    // depositor's own clear produces.
                    if substitute.is_null() {
                        decided.remove(&row.field);
                    } else {
                        decided.set(&row.field, substitute.clone());
                    }
                }
            }
            // Back to what is published. Absent there means the submission
            // added the member, so reverting removes it again.
            Some(Decision::Revert) => match row.published.as_ref() {
                Some(published) => decided.set(&row.field, published.clone()),
                None => {
                    decided.remove(&row.field);
                }
            },
            // Unreachable: an approval with an undecided change is refused
            // above. Left as the submitted value rather than guessed at, so if
            // the guard ever regresses the record is at least what was sent.
            None => {}
        }
    }
    Some(ApprovedRecord {
        id: Uuid::new_v4(),
        shortcode: context.submission.shortcode.clone(),
        payload: serde_json::to_string(&decided).ok()?,
        approved_by: Some(user.id),
        approved_at: at,
        collected_at: None,
    })
}

/// The draft a request-changes hands back.
///
/// The submitted payload, not the draft row as it was: the depositor resumes
/// from what they sent, which is what the note and the per-field decisions are
/// about. What RDU decided rides on the round, so nothing about the review is
/// written into the draft itself.
fn returned_draft(context: &Context<'_>, user: &User, at: DateTime<Utc>) -> Option<DraftRecord> {
    Some(DraftRecord {
        shortcode: context.submission.shortcode.clone(),
        payload: serde_json::to_string(&context.submitted).ok()?,
        updated_by: Some(user.id),
        // The submission time, not now: a returned draft was started when the
        // depositor started it, and `created_at` is the only thing that says
        // so once the original row is overwritten.
        created_at: context.submission.submitted_at,
        updated_at: at,
    })
}

/// What a confirmation prompt renders: the verb it will perform, the note as it
/// stands, and why the last attempt was refused.
///
/// A struct rather than three more arguments, for the reason `sections`'
/// `Rendering` is one: two of the three are string-ish and one is an `Option`
/// of the same, which a positional list lets a call site scramble silently.
struct Prompt<'a> {
    intent: &'a str,
    note: &'a str,
    refusal: Option<&'a str>,
}

/// The confirmation step for a terminating action.
fn asking(
    state: &AppState,
    user: &User,
    context: &Context<'_>,
    filter: page::Filter,
    headers: HeaderMap,
    prompt: Prompt<'_>,
) -> Response {
    let rows = review_rows(context);
    let mut view = view(context, &rows, filter, prompt.refusal.map(page::Notice::Refused));
    view.confirming = Some(prompt.intent);
    view.note = prompt.note;
    if is_enhanced(&headers) {
        return (StatusCode::OK, axum::response::Html(page::region(&view).into_string())).into_response();
    }
    let title = page_title(context);
    crate::render(state, &title, StatusCode::OK, Some(user), page::page(&view))
}

/// The round is over, so there is no submission left to diff.
fn finished(state: &AppState, user: &User, context: &Context<'_>, outcome: ReviewOutcome) -> Response {
    let finished = match outcome {
        ReviewOutcome::Approved => page::Finished::Approved,
        ReviewOutcome::ChangesRequested => page::Finished::ChangesRequested,
        ReviewOutcome::Rejected | ReviewOutcome::Withdrawn => page::Finished::Rejected,
    };
    // The same body on both paths, and a full document on the enhanced one
    // too: the region this surface patches is the diff, and there is no diff
    // any more. Datastar matches a patch by `id`, so a body with no
    // `review-surface` in it patches nothing — a full page replaces it.
    crate::render(
        state,
        &page_title(context),
        StatusCode::OK,
        Some(user),
        page::finished(context.project_name, context.shortcode, finished),
    )
}

/// Everything a rendering needs, once the request is known to be allowed.
struct Context<'a> {
    /// The path segment as the reader typed it, which every control posts back
    /// to.
    shortcode: &'a str,
    submission: Submission,
    /// The submission's payload, parsed.
    submitted: ProjectDraft,
    /// The published project, `None` for a local-only one.
    published: Option<ProjectDraft>,
    project_name: Option<&'a str>,
    rows: Vec<FieldDiff>,
    state: ReviewState,
    submitter_name: Option<String>,
    reviewer_name: Option<String>,
    /// The submission time, formatted once.
    submitted_at: String,
    /// Whether the reader is the account currently holding the submission.
    held_by_viewer: bool,
}

impl Context<'_> {
    fn changed_fields(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| row.changed())
            .map(|row| row.field.clone())
            .collect()
    }
}

/// Resolve a request, or the response that refuses it.
///
/// Shape, then the record — the same order as everywhere else in this service.
/// There is no authorization step between them here: [`Rdu`] already decided,
/// and role-based access leaves nothing per-project to check.
async fn context<'a>(state: &'a AppState, user: &User, shortcode: &'a str) -> Result<Context<'a>, Response> {
    if !is_valid_shortcode(shortcode) {
        return Err(crate::not_found(State(state.clone())).await);
    }
    let key = normalize_shortcode(shortcode);
    let submission = match SubmissionRepository::find_by_shortcode(&*state.db, &key).await {
        Ok(Some(submission)) => submission,
        Ok(None) => return Err(no_submission(state, user)),
        Err(error) => return Err(storage_error(state, user, "read this project's submission", &error)),
    };

    let submitted: ProjectDraft = match serde_json::from_str(&submission.payload) {
        Ok(draft) => draft,
        Err(error) => {
            // Not an empty draft, which is what the project form falls back to:
            // there, empty means "nothing pre-filled" and a save writes only
            // what is typed. Here it would render as the submission deleting
            // every published field, and a reviewer accepting that diff would
            // be approving a record nobody wrote.
            tracing::error!(
                error = %error,
                project.shortcode = %shortcode,
                "a stored submission payload could not be parsed"
            );
            return Err(unreadable_submission(state, user));
        }
    };

    let (review_state, parse_error) = ReviewState::parse(submission.review_state.as_deref());
    if let Some(error) = parse_error {
        tracing::error!(
            error = %error,
            project.shortcode = %shortcode,
            "a stored review state could not be parsed; the fields read as undecided"
        );
    }

    let published_raw = state.published.get(shortcode);
    let published = published_raw.map(ProjectDraft::from_raw);
    let rows = diff(published.as_ref(), &submitted);

    let names = match account_names(state).await {
        Ok(names) => names,
        Err(error) => return Err(storage_error(state, user, "read the accounts", &error)),
    };

    Ok(Context {
        shortcode,
        submitter_name: name_of(&names, submission.submitted_by).map(str::to_string),
        reviewer_name: name_of(&names, submission.reviewed_by).map(str::to_string),
        submitted_at: crate::format_instant(submission.submitted_at),
        held_by_viewer: submission.reviewed_by == Some(user.id),
        submission,
        submitted,
        published,
        project_name: published_raw.map(|project| project.name.as_str()),
        rows,
        state: review_state,
    })
}

/// Read every decision and substitution the body carries.
///
/// Only rows the submission actually changes are considered: an unchanged field
/// has no decision control rendered, so a decision naming one came from a
/// hand-built body, and honouring it would store a decision the surface can
/// never show.
fn decisions_from(
    body: &FormBody,
    rows: &[FieldDiff],
    submitted: &ProjectDraft,
    accept_all: bool,
    published: bool,
) -> ReviewState {
    let mut state = ReviewState::new();
    for row in rows.iter().filter(|row| row.changed()) {
        let posted = body.get(&format!("{}.{}", page::DECISION_PREFIX, row.field));
        let decision = match posted.and_then(Decision::parse) {
            // A revert on a project with no published counterpart is a decision
            // the surface never offers, because there is nothing to revert *to*.
            // Stored anyway it renders "Reverted — keeps published" beside a
            // "Not published yet" column, in a radio group with no matching
            // option — so it shows as undecided and cannot be cleared.
            Some(Decision::Revert) if !published => None,
            Some(decision) => Some(decision),
            None if accept_all => Some(Decision::Accept),
            None => None,
        };
        // A reverted row renders read-only, so it posts no value control and
        // there is nothing to read — and reading one anyway would let a
        // hand-built body attach a substitute to a decision that discards it.
        let value = if decision == Some(Decision::Revert) {
            None
        } else {
            substitute(body, submitted, &row.field)
        };
        state.set(&row.field, FieldReview { decision, value });
    }
    state
}

/// What the reviewer put in place of the submitted value, or `None` where they
/// left it alone.
///
/// Computed by running the field's own applier over a clone of the submitted
/// draft rather than by comparing strings. That is the only way the reviewer's
/// edit obeys the rules a depositor's does: a value differing only in
/// surrounding whitespace or in how a newline was encoded is not a change, and
/// a stored `MISSING` renders empty and must survive an empty submit. A
/// second comparison here would agree with those rules only by inspection.
///
/// `Some(Value::Null)` is a reviewer clearing a field the contract types as an
/// `Option` — a real substitution, and the one the `or`-shaped alternative
/// (`substitute.or(submitted)`) could not express if absence meant "unchanged".
fn substitute(body: &FormBody, submitted: &ProjectDraft, field: &str) -> Option<Value> {
    let shape = registry::field(field).and_then(|field| field.shape)?;
    let mut edited = submitted.clone();
    apply(shape, body, &mut edited, field);
    let before = submitted.get(field);
    let after = edited.get(field);
    if before == after {
        return None;
    }
    Some(after.cloned().unwrap_or(Value::Null))
}

/// Whether this request came from the Datastar bundle.
fn is_enhanced(headers: &HeaderMap) -> bool {
    headers.contains_key(DATASTAR_REQUEST)
}

/// A stored decision: a redirect on the plain path, the patched region on the
/// enhanced one.
fn saved(context: &Context<'_>, filter: page::Filter, headers: HeaderMap, notice: page::Notice<'_>) -> Response {
    if !is_enhanced(&headers) {
        // POST-redirect-GET: a `POST` left in the history re-posts on refresh,
        // and the reloaded `GET` reads what was just written. The filter rides
        // along, or the redirect is where the reviewer's view silently changes.
        return Redirect::to(&format!("/review/{}{}", context.shortcode, filter.query())).into_response();
    }
    region(context, filter, Some(notice))
}

/// A refused write, re-rendered with what was typed still in place.
fn refused(
    state: &AppState,
    user: &User,
    context: &Context<'_>,
    filter: page::Filter,
    headers: HeaderMap,
    message: &str,
) -> Response {
    let notice = Some(page::Notice::Refused(message));
    if is_enhanced(&headers) {
        return region(context, filter, notice);
    }
    render_page(state, user, context, filter, notice)
}

/// The region the enhanced path patches.
///
/// 200 always: Datastar processes a response body only on a 200, so a status
/// carrying the refusal would lose the message it is carrying. The outcome is
/// on the span instead, which is where alerting reads it from.
fn region(context: &Context<'_>, filter: page::Filter, notice: Option<page::Notice<'_>>) -> Response {
    let rows = review_rows(context);
    let view = view(context, &rows, filter, notice);
    (StatusCode::OK, axum::response::Html(page::region(&view).into_string())).into_response()
}

fn render_page(
    state: &AppState,
    user: &User,
    context: &Context<'_>,
    filter: page::Filter,
    notice: Option<page::Notice<'_>>,
) -> Response {
    let rows = review_rows(context);
    let view = view(context, &rows, filter, notice);
    crate::render(state, &page_title(context), StatusCode::OK, Some(user), page::page(&view))
}

/// What a review page is called. One decision, shared by the diff, the
/// confirmation and the finished page.
fn page_title(context: &Context<'_>) -> String {
    match context.project_name {
        Some(name) => format!("Review {name} — DaSCH Metadata Editor"),
        None => format!("Review project {} — DaSCH Metadata Editor", context.shortcode),
    }
}

/// Turn the comparison into what the page renders, adding the registry's
/// wording and whether the form has a control for the field.
///
/// A field the registry does not know keeps its member name as its label — a
/// field added to the contract without an editor change. A row labelled by its
/// raw name is far better than no row at all, which would let the change
/// through unseen.
fn review_rows<'a>(context: &'a Context<'a>) -> Vec<page::ReviewRow<'a>> {
    context
        .rows
        .iter()
        .map(|row| page::ReviewRow {
            field: &row.field,
            // The registry entry rather than a label and a pair of flags read
            // off it here: the page renders the depositor's own control from
            // it, so a second reading of the same table could not diverge.
            registry: registry::field(&row.field),
            published: row.published.as_ref(),
            submitted: row.submitted.as_ref(),
            substitute: context.state.substitute(&row.field),
            decision: context.state.decision(&row.field),
            changed: row.changed(),
        })
        .collect()
}

fn view<'a>(
    context: &'a Context<'a>,
    rows: &'a [page::ReviewRow<'a>],
    filter: page::Filter,
    notice: Option<page::Notice<'a>>,
) -> page::ReviewView<'a> {
    page::ReviewView {
        shortcode: context.shortcode,
        project_name: context.project_name,
        published: context.published.is_some(),
        submitted_by: context.submitter_name.as_deref(),
        submitted_at: &context.submitted_at,
        reviewer: match context.submission.state {
            SubmissionState::InReview => context.reviewer_name.as_deref(),
            _ => None,
        },
        held_by_viewer: context.held_by_viewer,
        rows,
        filter,
        notice,
        // The two the terminating flow sets; a diff rendering carries neither.
        note: "",
        confirming: None,
    }
}

/// Every account's name by id, for the "last editor" and "who has it" columns.
///
/// One query rather than one per row: the queue reads two tables and would
/// otherwise issue a lookup per submission and per draft.
async fn account_names(state: &AppState) -> Result<HashMap<Uuid, String>, RepositoryError> {
    Ok(UserRepository::list(&*state.db)
        .await?
        .into_iter()
        .map(|user| (user.id, user.name))
        .collect())
}

/// The name behind an id, `None` for an account that has been removed.
fn name_of(names: &HashMap<Uuid, String>, id: Option<Uuid>) -> Option<&str> {
    id.and_then(|id| names.get(&id)).map(String::as_str)
}

/// The published project's shortcode as its file spells it, falling back to the
/// stored key.
///
/// The stored key is folded (`080c`), and the published set mixes `080C` with
/// `0801a` — so a queue rendering the key would show a shortcode that appears
/// nowhere else, in a column a reviewer matches against a file name.
fn shortcode_as_published<'a>(state: &'a AppState, stored: &'a str) -> String {
    state
        .published
        .get(stored)
        .map_or_else(|| stored.to_string(), |project| project.shortcode.clone())
}

fn project_name<'a>(state: &'a AppState, shortcode: &str) -> Option<&'a str> {
    state.published.get(shortcode).map(|project| project.name.as_str())
}

/// No submission on this project.
///
/// A 404 with a page rather than the bare shell: a reviewer arriving from a
/// queue somebody else has already cleared has done nothing wrong, and the
/// status is still the honest one.
fn no_submission(state: &AppState, user: &User) -> Response {
    let content = maud::html! {
        h1 class="font-display text-2xl mb-2" { "Nothing to review" }
        p class="mb-4" { (NO_SUBMISSION) }
        p {
            a href="/review" class="underline" { "Back to the review queue" }
        }
    };
    crate::render(
        state,
        "Nothing to review — DaSCH Metadata Editor",
        StatusCode::NOT_FOUND,
        Some(user),
        content,
    )
}

/// A submission whose stored payload this build cannot parse.
fn unreadable_submission(state: &AppState, user: &User) -> Response {
    crate::render(
        state,
        "Submission unreadable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(user),
        editor_web::pages::problem::unavailable(
            "This submission's stored record could not be read, so there is nothing to compare against what is \
             published. Nothing has been changed. The service needs attention before this project can be reviewed.",
        ),
    )
}

/// Storage would not answer, so the page cannot show what it should.
fn storage_error(state: &AppState, user: &User, what: &str, error: &RepositoryError) -> Response {
    tracing::error!(error = %error, operation = what, "the review surface could not reach storage");
    crate::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(user),
        editor_web::pages::problem::unavailable(
            "The editor could not reach its database, so this page is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use editor_core::records::Role;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::{
        a_session, a_user, body_string, get, location, open_test_db, post, state_over, test_app, test_state,
        with_cookie, Faults, FaultyDatabase, RecordingMailer,
    };

    async fn as_session(app: &axum::Router, request: Request<Body>, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(request, crate::auth::cookie::SESSION, session))
            .await
            .expect("the request should complete")
    }

    /// The same `POST`, as the Datastar bundle sends it.
    fn enhanced(uri: &str, form: &str) -> Request<Body> {
        let mut request = post(uri, form);
        request
            .headers_mut()
            .insert(DATASTAR_REQUEST, "true".parse().expect("a header value"));
        request
    }

    /// An RDU account with a live session.
    async fn a_reviewer(state: &AppState, email: &str, name: &str) -> (User, String) {
        let user = a_user(state, email, name, Role::Rdu, &[]).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    /// The body a terminating action's second post carries: the intent, the
    /// confirmation pair, the decisions and the note.
    fn finishing(intent: &str, decisions: &[(&str, &str)], note: &str) -> String {
        let mut body = format!("intent={intent}&{}=1", page::CONFIRMED);
        for (field, decision) in decisions {
            body.push_str(&format!("&{}.{field}={decision}", page::DECISION_PREFIX));
        }
        if !note.is_empty() {
            body.push_str(&format!("&{}={}", page::NOTE, note.replace(' ', "+")));
        }
        body
    }

    /// The one approved record for a project.
    async fn the_record(state: &AppState, shortcode: &str) -> editor_core::records::ApprovedRecord {
        let mut records = editor_core::repository::ApprovedRecordRepository::find_by_shortcode(&*state.db, shortcode)
            .await
            .expect("read");
        assert_eq!(records.len(), 1, "exactly one approved record: {records:?}");
        records.remove(0)
    }

    /// The one round recorded for a project.
    async fn the_round(state: &AppState, shortcode: &str) -> editor_core::records::ReviewRound {
        let mut rounds = ReviewRoundRepository::list_for_shortcode(&*state.db, shortcode)
            .await
            .expect("read");
        assert_eq!(rounds.len(), 1, "exactly one round: {rounds:?}");
        rounds.remove(0)
    }

    #[tokio::test]
    async fn approving_creates_the_approved_record_and_clears_the_queue() {
        // REQ-4.4. The submission leaves the queue and an `approved_records`
        // row takes its place, on its way to a pull request.
        let (state, _) = test_state("review-approve").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        let body = body_string(
            as_session(
                &app,
                post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
                &session,
            )
            .await,
        )
        .await;
        assert!(body.contains("Approved"), "{body}");

        assert_eq!(SubmissionRepository::find(&*state.db, submission.id).await.unwrap(), None);
        let record = the_record(&state, "0801d").await;
        assert!(record.payload.contains("A New Title"), "{}", record.payload);
        assert_eq!(record.collected_at, None, "uncollected, so the endpoint will serve it");
        assert_eq!(the_round(&state, "0801d").await.outcome, ReviewOutcome::Approved);
    }

    #[tokio::test]
    async fn approving_commits_the_reviewers_substitution_and_not_the_submitted_value() {
        // REQ-4.3 permits editing before acceptance. The substitute is what
        // gets committed; `submissions.payload` stayed the depositor's own
        // until the moment it was deleted, which is what lets their form show
        // them what was changed on their behalf.
        let (state, _) = test_state("review-approve-substitute").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        a_submission(&state, "0801d", None, json!({ "name": "The depositor's title" })).await;
        let app = test_app(&state);

        // The substitute posts under the field's own name, exactly as the
        // depositor's form posts it.
        let body = format!("{}&name=RDU%27s+title", finishing(page::APPROVE, &[("name", "accept")], ""));
        as_session(&app, post("/review/0801d", &body), &session).await;

        let record = the_record(&state, "0801d").await;
        assert!(record.payload.contains("RDU's title"), "{}", record.payload);
        assert!(!record.payload.contains("The depositor's title"), "{}", record.payload);
        let round = the_round(&state, "0801d").await;
        let (stored, error) = ReviewState::parse(round.review_state.as_deref());
        assert!(error.is_none());
        assert_eq!(
            stored.substitutions(),
            [("name", &json!("RDU's title"))],
            "the round keeps the evidence"
        );
    }

    #[tokio::test]
    async fn approving_a_reverted_field_commits_the_published_value() {
        // Revert means keep what is published, so the record must carry the
        // published value — not the submitted one, and not nothing.
        let (state, _) = test_state("review-approve-revert").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let published = state.published.get("0801d").expect("0801d").name.clone();
        a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "revert")], "")),
            &session,
        )
        .await;

        let record = the_record(&state, "0801d").await;
        let committed: ProjectDraft = serde_json::from_str(&record.payload).unwrap();
        assert_eq!(committed.get("name").and_then(Value::as_str), Some(published.as_str()));
    }

    #[tokio::test]
    async fn reverting_a_member_the_submission_added_removes_it_again() {
        // Absent from the published side means the submission added it, so
        // reverting has to remove it. Left in place it would commit a field
        // the reviewer explicitly declined.
        let (state, _) = test_state("review-approve-revert-added").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        // `provenance` is an `Option` the file leaves out, so `from_raw`
        // strips it and the published side genuinely has no such member —
        // unlike `endDate`, which 0801d carries as the `MISSING` placeholder.
        assert!(
            state.published.get("0801d").expect("0801d").provenance.is_none(),
            "the premise: 0801d has no `provenance`"
        );
        a_submission(&state, "0801d", None, json!({ "provenance": "Added by the depositor" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("provenance", "revert")], "")),
            &session,
        )
        .await;

        let committed: ProjectDraft = serde_json::from_str(&the_record(&state, "0801d").await.payload).unwrap();
        assert_eq!(committed.get("provenance"), None);
    }

    #[tokio::test]
    async fn approving_with_an_undecided_change_is_refused() {
        // Approving an undecided row commits bytes nobody looked at, which is
        // the one thing a field-by-field surface exists to prevent. "Accept all
        // remaining" makes clearing it one click, so this is never a dead end.
        let (state, _) = test_state("review-approve-undecided").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(
            &state,
            "0801d",
            None,
            json!({ "name": "A New Title", "officialName": "Another" }),
        )
        .await;
        let app = test_app(&state);

        // One decided, one not.
        let response = as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
            &session,
        )
        .await;

        assert!(
            body_string(response).await.contains("has to be decided"),
            "the reason is stated"
        );
        assert!(
            SubmissionRepository::find(&*state.db, submission.id).await.unwrap().is_some(),
            "the submission is still in the queue"
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn an_rdu_member_can_approve_the_submission_they_made_themselves() {
        // REQ-4.4: no second approver for an RDU member's own submission. What
        // that means here is that nothing distinguishes their own submission
        // from anyone else's — there is no second-approver step to waive, and
        // no self-approval check to add.
        let (state, _) = test_state("review-approve-own").await;
        let (reviewer, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        a_submission(&state, "0801d", Some(reviewer.id), json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
            &session,
        )
        .await;

        assert_eq!(the_round(&state, "0801d").await.actor, Some(reviewer.id));
        assert_eq!(the_record(&state, "0801d").await.approved_by, Some(reviewer.id));
    }

    #[tokio::test]
    async fn requesting_changes_returns_the_project_with_the_note_and_the_decisions() {
        // REQ-4.5: the submission becomes a draft, and both the note and the
        // per-field accepted state are retained — the latter on the round,
        // which is what layer-3's field lock reads.
        let (state, _) = test_state("review-request-changes").await;
        let (reviewer, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        let body = body_string(
            as_session(
                &app,
                post(
                    "/review/0801d",
                    &finishing(page::REQUEST_CHANGES, &[("name", "accept")], "Add a German description"),
                ),
                &session,
            )
            .await,
        )
        .await;
        assert!(body.contains("Returned to the depositor"), "{body}");

        assert_eq!(SubmissionRepository::find(&*state.db, submission.id).await.unwrap(), None);
        let draft = DraftRepository::find(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the submission became a draft");
        assert!(
            draft.payload.contains("A New Title"),
            "the depositor resumes from what they sent"
        );
        assert_eq!(draft.updated_by, Some(reviewer.id));

        let round = the_round(&state, "0801d").await;
        assert_eq!(round.outcome, ReviewOutcome::ChangesRequested);
        assert_eq!(round.note.as_deref(), Some("Add a German description"));
        let (stored, _) = ReviewState::parse(round.review_state.as_deref());
        assert_eq!(stored.accepted_fields(), ["name"], "the lock has something to read");
    }

    #[tokio::test]
    async fn rejecting_discards_the_submission_and_leaves_the_draft() {
        // REQ-4.6 discards and leaves published metadata unchanged; REQ-1.13
        // preserves the draft, so the depositor's work is still theirs.
        let (state, _) = test_state("review-reject").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        DraftRepository::upsert(
            &*state.db,
            &DraftRecord {
                shortcode: "0801d".to_string(),
                payload: submission.payload.clone(),
                updated_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        let body = body_string(
            as_session(
                &app,
                post(
                    "/review/0801d",
                    &finishing(page::REJECT, &[], "Out of scope for this repository"),
                ),
                &session,
            )
            .await,
        )
        .await;
        assert!(body.contains("Rejected"), "{body}");

        assert_eq!(SubmissionRepository::find(&*state.db, submission.id).await.unwrap(), None);
        assert!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some(),
            "the draft survives"
        );
        assert!(
            editor_core::repository::ApprovedRecordRepository::find_by_shortcode(&*state.db, "0801d")
                .await
                .unwrap()
                .is_empty(),
            "nothing was approved"
        );
        let round = the_round(&state, "0801d").await;
        assert_eq!(round.outcome, ReviewOutcome::Rejected);
        assert_eq!(round.note.as_deref(), Some("Out of scope for this repository"));
    }

    #[tokio::test]
    async fn a_reject_with_no_note_is_refused() {
        // The whole rejection signal. REQ-4.6 discards the submission and
        // notifications are out of scope, so without a note the depositor's
        // work vanishes with nothing whatever saying why.
        let (state, _) = test_state("review-reject-no-note").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        for note in ["", "   "] {
            let body = format!("intent={}&{}=1&{}={note}", page::REJECT, page::CONFIRMED, page::NOTE);
            let response = as_session(&app, post("/review/0801d", &body), &session).await;
            let rendered = body_string(response).await;
            assert!(rendered.contains("only thing the depositor will see"), "{rendered}");
            assert!(
                SubmissionRepository::find(&*state.db, submission.id).await.unwrap().is_some(),
                "nothing was discarded for note {note:?}"
            );
        }
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_request_changes_with_no_note_is_refused() {
        // REQ-4.5 retains "the reviewer note", and it is the only thing telling
        // the depositor what to do — a returned draft with no note is a form
        // they cannot act on.
        let (state, _) = test_state("review-changes-no-note").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        let body = format!("intent={}&{}=1", page::REQUEST_CHANGES, page::CONFIRMED);
        as_session(&app, post("/review/0801d", &body), &session).await;

        assert!(SubmissionRepository::find(&*state.db, submission.id).await.unwrap().is_some());
        assert_eq!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap(),
            None,
            "no draft was written"
        );
    }

    #[tokio::test]
    async fn an_approval_needs_no_note() {
        // Approve is the one outcome that leaves a record somewhere else, and
        // the depositor is shown what changed rather than told about it. A
        // required note here would be ceremony on the common path.
        let (state, _) = test_state("review-approve-no-note").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
            &session,
        )
        .await;

        assert_eq!(the_round(&state, "0801d").await.note, None);
    }

    #[tokio::test]
    async fn a_note_survives_backing_out_of_the_confirmation() {
        // Declining a confirmation posts a plain save, and the diff form
        // renders no note control — so without the working copy on the
        // submission, a carefully written rejection reason is gone and has to
        // be typed again from memory.
        let (state, _) = test_state("review-note-survives").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        // Open the reject prompt and type a reason, then decline: "Not now"
        // carries no intent, so it posts as a save.
        let typed = format!("intent={}&{}=Out+of+scope", page::REJECT, page::NOTE);
        as_session(&app, post("/review/0801d", &typed), &session).await;
        let declined = format!("{}=Out+of+scope", page::NOTE);
        as_session(&app, post("/review/0801d", &declined), &session).await;

        let stored = SubmissionRepository::find(&*state.db, submission.id).await.unwrap().unwrap();
        assert_eq!(stored.reviewer_note.as_deref(), Some("Out of scope"));

        // Re-opening the prompt shows it back.
        let reopened =
            body_string(as_session(&app, post("/review/0801d", &format!("intent={}", page::REJECT)), &session).await)
                .await;
        assert!(reopened.contains("Out of scope"), "{reopened}");
    }

    #[tokio::test]
    async fn a_save_from_the_diff_form_does_not_clear_a_stored_note() {
        // The diff form renders no note control, so an ordinary "Save review
        // decisions" posts nothing under `note`. Read as an empty note that
        // would erase the stored one on every save.
        let (state, _) = test_state("review-note-not-cleared").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);
        let typed = format!("intent={}&{}=Out+of+scope", page::REJECT, page::NOTE);
        as_session(&app, post("/review/0801d", &typed), &session).await;
        as_session(&app, post("/review/0801d", &format!("{}=Out+of+scope", page::NOTE)), &session).await;

        // A save carrying decisions and no note at all.
        let body = format!("intent={}&{}.name=accept", page::SAVE, page::DECISION_PREFIX);
        as_session(&app, post("/review/0801d", &body), &session).await;

        let stored = SubmissionRepository::find(&*state.db, submission.id).await.unwrap().unwrap();
        assert_eq!(stored.reviewer_note.as_deref(), Some("Out of scope"));
    }

    #[tokio::test]
    async fn the_round_takes_the_stored_note_when_the_confirming_body_carries_none() {
        // The reviewer typed a reason, backed out, came back and confirmed from
        // a page whose control was pre-filled from storage. If only the body
        // counted, the note requirement would refuse them — or worse, record a
        // round with no note at all.
        let (state, _) = test_state("review-note-from-storage").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);
        let stored_note = format!("{}=Out+of+scope", page::NOTE);
        as_session(&app, post("/review/0801d", &stored_note), &session).await;

        // Confirming with no note in the body at all.
        let body = format!("intent={}&{}=1", page::REJECT, page::CONFIRMED);
        as_session(&app, post("/review/0801d", &body), &session).await;

        assert_eq!(the_round(&state, "0801d").await.note.as_deref(), Some("Out of scope"));
    }

    #[tokio::test]
    async fn a_terminating_action_asks_before_it_writes() {
        // All three are irreversible from this surface — the submission row is
        // gone whichever is chosen — so each asks once. The first post renders
        // the prompt and writes nothing.
        let (state, _) = test_state("review-confirm").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        for (intent, expected) in [
            (page::APPROVE, "Yes, approve"),
            (page::REQUEST_CHANGES, "Yes, request changes"),
            (page::REJECT, "Yes, reject"),
        ] {
            let body =
                body_string(as_session(&app, post("/review/0801d", &format!("intent={intent}")), &session).await).await;
            assert!(body.contains(expected), "{intent}: {body}");
            assert!(
                SubmissionRepository::find(&*state.db, submission.id).await.unwrap().is_some(),
                "{intent} wrote something on the first post"
            );
        }
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_terminating_action_on_an_already_finished_submission_is_reported() {
        // The terminal-state guard, at the surface it is reached through. Two
        // reviewers hold the page open: the second must not destroy a record
        // the collection endpoint has already served into a pull request, nor
        // resurrect one as a draft.
        for intent in [page::APPROVE, page::REQUEST_CHANGES, page::REJECT] {
            let (state, _) = test_state(&format!("review-race-{intent}")).await;
            let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
            a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
            let app = test_app(&state);

            // The first reviewer approves.
            as_session(
                &app,
                post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
                &session,
            )
            .await;

            // The second acts on a page rendered before that.
            let response = as_session(
                &app,
                post("/review/0801d", &finishing(intent, &[("name", "accept")], "Too late")),
                &session,
            )
            .await;

            // The submission is gone, so the request never gets past resolving
            // it: the surface says there is nothing to review rather than
            // inventing a second round.
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{intent}");
            assert!(body_string(response).await.contains("Nothing to review"), "{intent}");
            let rounds = ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d").await.unwrap();
            assert_eq!(rounds.len(), 1, "{intent}: exactly the first round");
            assert_eq!(rounds[0].outcome, ReviewOutcome::Approved, "{intent}");
            assert_eq!(
                editor_core::repository::ApprovedRecordRepository::find_by_shortcode(&*state.db, "0801d")
                    .await
                    .unwrap()
                    .len(),
                1,
                "{intent}: no second record"
            );
            assert_eq!(DraftRepository::find(&*state.db, "0801d").await.unwrap(), None, "{intent}");
        }
    }

    #[tokio::test]
    async fn a_terminating_action_refused_by_storage_leaves_the_submission_in_the_queue() {
        // Nothing partial: the three writes are one transaction, so a failure
        // leaves the submission exactly where it was and says so.
        let db = std::sync::Arc::new(open_test_db("review-finish-storage").await);
        let sound = state_over(db.clone(), RecordingMailer::new(), |_| {});
        let (_, session) = a_reviewer(&sound, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&sound, "0801d", None, json!({ "name": "A New Title" })).await;
        let faulty = state_over(
            std::sync::Arc::new(FaultyDatabase::new(
                db.clone(),
                Faults { review_transition: true, ..Faults::default() },
            )),
            RecordingMailer::new(),
            |_| {},
        );
        let app = test_app(&faulty);

        let response = as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
            &session,
        )
        .await;

        assert!(
            body_string(response).await.contains("still in the queue"),
            "the reason is stated"
        );
        assert!(SubmissionRepository::find(&*db, submission.id).await.unwrap().is_some());
        assert!(
            editor_core::repository::ApprovedRecordRepository::find_by_shortcode(&*db, "0801d")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_terminating_action_uses_the_decisions_on_screen_not_the_ones_last_saved() {
        // The controls sit inside the diff form, so the decisions post with the
        // action. Read from storage instead, approving straight after deciding
        // a row would commit the previous state — a difference nothing on the
        // page would explain.
        let (state, _) = test_state("review-finish-unsaved").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        assert_eq!(submission.review_state, None, "the premise: nothing was saved");
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "revert")], "")),
            &session,
        )
        .await;

        // A revert that was never saved still decided the outcome.
        let published = state.published.get("0801d").expect("0801d").name.clone();
        let committed: ProjectDraft = serde_json::from_str(&the_record(&state, "0801d").await.payload).unwrap();
        assert_eq!(committed.get("name").and_then(Value::as_str), Some(published.as_str()));
    }

    #[tokio::test]
    async fn a_depositor_cannot_finish_a_review_round() {
        // The `Rdu` extractor closes every handler here, so this is the 403
        // page rather than a refusal message. Asserted on the write, not only
        // on the render: the extractor runs before the body is read.
        let (state, _) = test_state("review-finish-depositor").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let submission = a_submission(&state, "0801d", Some(user.id), json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        let response = as_session(
            &app,
            post("/review/0801d", &finishing(page::APPROVE, &[("name", "accept")], "")),
            &session,
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(SubmissionRepository::find(&*state.db, submission.id).await.unwrap().is_some());
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn an_unknown_intent_saves_rather_than_finishing_the_round() {
        // Every terminating action is irreversible; a save is not. A body
        // naming a verb this build does not know has to take the recoverable
        // branch.
        let (state, _) = test_state("review-unknown-intent").await;
        let (_, session) = a_reviewer(&state, "rdu@x.test", "An RDU Member").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A New Title" })).await;
        let app = test_app(&state);

        let body = format!("intent=publish&{}=1&{}.name=accept", page::CONFIRMED, page::DECISION_PREFIX);
        as_session(&app, post("/review/0801d", &body), &session).await;

        let stored = SubmissionRepository::find(&*state.db, submission.id)
            .await
            .unwrap()
            .expect("the submission is still in the queue");
        assert_eq!(
            stored.state,
            SubmissionState::InReview,
            "it was claimed and saved, not finished"
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    /// A pending submission whose payload is the published project with
    /// `changes` applied on top, so the diff is exactly `changes`.
    async fn a_submission(state: &AppState, shortcode: &str, author: Option<Uuid>, changes: Value) -> Submission {
        let mut draft = state.published.get(shortcode).map(ProjectDraft::from_raw).unwrap_or_default();
        for (field, value) in changes.as_object().expect("an object of changes") {
            draft.set(field, value.clone());
        }
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: normalize_shortcode(shortcode),
            payload: serde_json::to_string(&draft).expect("a draft serializes"),
            state: SubmissionState::Submitted,
            submitted_by: author,
            submitted_at: Utc::now(),
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        };
        SubmissionRepository::create(&*state.db, &submission)
            .await
            .expect("the submission should store");
        submission
    }

    async fn stored_review_state(state: &AppState, shortcode: &str) -> ReviewState {
        let submission = SubmissionRepository::find_by_shortcode(&*state.db, &normalize_shortcode(shortcode))
            .await
            .expect("the lookup should succeed")
            .expect("the submission should still be there");
        ReviewState::parse(submission.review_state.as_deref()).0
    }

    #[tokio::test]
    async fn a_depositor_cannot_reach_the_review_surfaces() {
        // Review access is role-based. The `Rdu` extractor is what performs the
        // check, so a handler that omits it is visibly public.
        let (state, _) = test_state("review-depositor").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_submission(&state, "0801d", Some(user.id), json!({})).await;
        let app = test_app(&state);

        for uri in ["/review", "/review/0801d"] {
            let response = as_session(&app, get(uri), &session).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
        }
    }

    #[tokio::test]
    async fn the_queue_lists_every_pending_submission_oldest_first() {
        // Oldest first, and every RDU member sees every pending submission —
        // neither account below is assigned to either project.
        let (state, _) = test_state("review-queue-order").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let author = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &[]).await;
        let mut first = a_submission(&state, "0801d", Some(author.id), json!({ "name": "New" })).await;
        let second = a_submission(&state, "080C", Some(author.id), json!({ "name": "Other" })).await;
        // Push the first one into the past, so the order is not the insert
        // order by accident.
        first.submitted_at = Utc::now() - chrono::Duration::hours(3);
        SubmissionRepository::update(&*state.db, &first).await.unwrap();
        let _ = second;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review"), &session).await).await;
        let first_at = body.find("0801d").expect("the older submission is listed");
        let second_at = body.find("080C").expect("the newer submission is listed");
        assert!(first_at < second_at, "oldest first: {body}");
        assert!(body.contains("A Depositor"), "{body}");
    }

    #[tokio::test]
    async fn the_queue_names_the_project_and_shows_its_published_shortcode() {
        // `submissions.shortcode` is folded, and the published set mixes `080C`
        // with `0801a` — a queue rendering the stored key would show a
        // shortcode that appears in no file name.
        let (state, _) = test_state("review-queue-shortcode").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "080C", None, json!({ "name": "New" })).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review"), &session).await).await;
        assert!(body.contains("080C"), "{body}");
        assert!(!body.contains(">080c<"), "{body}");
    }

    #[tokio::test]
    async fn the_queue_shows_every_draft_as_well() {
        // Drafts are visible to RDU so it can help a depositor who is stuck
        // before submitting. The account below is assigned nothing.
        let (state, _) = test_state("review-queue-drafts").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let author = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        DraftRepository::upsert(
            &*state.db,
            &DraftRecord {
                shortcode: "0801d".to_string(),
                payload: "{}".to_string(),
                updated_by: Some(author.id),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review"), &session).await).await;
        assert!(body.contains("Drafts in progress"), "{body}");
        assert!(body.contains(r#"href="/projects/0801d""#), "{body}");
    }

    #[tokio::test]
    async fn an_approved_record_is_not_a_queue_entry() {
        // It is waiting to be collected into a pull request, not for a
        // reviewer, so a row here would be an entry nobody can clear.
        let (state, _) = test_state("review-queue-approved").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let mut submission = a_submission(&state, "0801d", None, json!({ "name": "New" })).await;
        submission.state = SubmissionState::Approved;
        SubmissionRepository::update(&*state.db, &submission).await.unwrap();
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review"), &session).await).await;
        assert!(body.contains("No submissions are waiting for review"), "{body}");
    }

    #[tokio::test]
    async fn the_diff_shows_only_the_fields_the_submission_changes() {
        let (state, _) = test_state("review-diff").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        // Read off the corpus rather than spelled here: a fixture name that
        // drifts from the committed file makes this test pass on a value the
        // page never rendered.
        let published = state.published.get("0801d").expect("0801d is in the corpus").name.clone();
        let body = body_string(as_session(&app, get("/review/0801d"), &session).await).await;
        assert!(body.contains("A new name"), "{body}");
        assert!(body.contains(&published), "the published value is shown beside it: {body}");
        assert!(body.contains(r#"name="decision.name""#), "{body}");
        // `shortDescription` is unchanged, so it carries no decision control.
        assert!(!body.contains(r#"name="decision.shortDescription""#), "{body}");
    }

    #[tokio::test]
    async fn a_project_with_no_published_counterpart_offers_no_revert() {
        // A local-only project. Revert means keeping the published
        // value, and there is none — the choice would silently unset a field
        // the contract requires.
        let (state, _) = test_state("review-unpublished").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        assert!(state.published.get("9999").is_none(), "the premise of this test");
        a_submission(&state, "9999", None, json!({ "name": "A brand new project" })).await;
        let app = test_app(&state);

        let response = as_session(&app, get("/review/9999"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("Not published yet"), "{body}");
        assert!(body.contains(r#"value="accept""#), "{body}");
        assert!(!body.contains(r#"value="revert""#), "{body}");
    }

    #[tokio::test]
    async fn a_project_with_no_pending_submission_says_so_with_a_way_back() {
        // A reviewer arriving from a queue somebody else has already cleared
        // has done nothing wrong; the status is still the honest one.
        let (state, _) = test_state("review-missing").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let app = test_app(&state);

        let response = as_session(&app, get("/review/0801d"), &session).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = body_string(response).await;
        assert!(body.contains("Nothing to review"), "{body}");
        assert!(body.contains(r#"href="/review""#), "{body}");
    }

    #[tokio::test]
    async fn a_path_that_could_never_name_a_project_is_a_404() {
        let (state, _) = test_state("review-shape").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let app = test_app(&state);

        for uri in ["/review/not%20a%20code", "/review/a-b"] {
            let response = as_session(&app, get(uri), &session).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn claiming_a_submission_records_who_has_it() {
        // The concurrency answer: no lock, but who last touched it is visible.
        // This is also the only producer of `SubmissionState::InReview`, which
        // the project form already reads as a reason to lock the depositor out.
        let (state, _) = test_state("review-claim").await;
        let (reviewer, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "New" })).await;
        let app = test_app(&state);

        let response = as_session(&app, post("/review/0801d", "intent=claim"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/review/0801d"));

        let stored = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.state, SubmissionState::InReview);
        assert_eq!(stored.reviewed_by, Some(reviewer.id));
    }

    #[tokio::test]
    async fn a_second_reviewer_is_told_who_has_it_and_can_take_it_over() {
        let (state, _) = test_state("review-takeover").await;
        let (_, first) = a_reviewer(&state, "one@dasch.swiss", "First Reviewer").await;
        let (second_user, second) = a_reviewer(&state, "two@dasch.swiss", "Second Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "New" })).await;
        let app = test_app(&state);

        as_session(&app, post("/review/0801d", "intent=claim"), &first).await;
        let body = body_string(as_session(&app, get("/review/0801d"), &second).await).await;
        assert!(body.contains("First Reviewer picked this submission up"), "{body}");
        assert!(body.contains("Take over the review"), "{body}");

        as_session(&app, post("/review/0801d", "intent=claim"), &second).await;
        let stored = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.reviewed_by, Some(second_user.id));
    }

    #[tokio::test]
    async fn a_claim_that_could_not_be_stored_does_not_render_as_though_it_had() {
        // The failure the concurrency story turns on. Nothing was written, so
        // the reader does not hold the submission — a page that named them as
        // the reviewer would also suppress the take-over banner, telling them
        // the opposite of what the database says at exactly the moment it
        // matters.
        let db = std::sync::Arc::new(open_test_db("review-claim-fails").await);
        let sound = state_over(db.clone(), RecordingMailer::new(), |_| {});
        let faulty = state_over(
            std::sync::Arc::new(FaultyDatabase::new(
                db.clone(),
                Faults { submission_update: true, ..Faults::default() },
            )),
            RecordingMailer::new(),
            |_| {},
        );
        let (holder, holder_session) = a_reviewer(&sound, "one@dasch.swiss", "First Reviewer").await;
        let (_, session) = a_reviewer(&sound, "two@dasch.swiss", "Second Reviewer").await;
        a_submission(&sound, "0801d", None, json!({ "name": "A new name" })).await;
        as_session(&test_app(&sound), post("/review/0801d", "intent=claim"), &holder_session).await;

        let body =
            body_string(as_session(&test_app(&faulty), post("/review/0801d", "intent=claim"), &session).await).await;
        assert!(body.contains("could not be saved"), "the refusal is reported: {body}");
        assert!(
            body.contains("First Reviewer picked this submission up"),
            "the holder is still named: {body}"
        );
        assert!(body.contains("Take over the review"), "{body}");

        let after = SubmissionRepository::find_by_shortcode(&*sound.db, "0801d")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.reviewed_by, Some(holder.id), "nothing was written");
    }

    #[tokio::test]
    async fn a_revert_is_refused_on_a_project_with_no_published_counterpart() {
        // The surface never offers it, so one arriving came from a hand-built
        // body. Stored, it renders "Reverted — keeps published" beside a "Not
        // published yet" column, in a radio group with no matching option — so
        // it shows as undecided and cannot be cleared.
        let (state, _) = test_state("review-revert-unpublished").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "9999", None, json!({ "name": "A brand new project" })).await;
        let app = test_app(&state);

        as_session(&app, post("/review/9999", "intent=save&decision.name=revert"), &session).await;

        assert_eq!(stored_review_state(&state, "9999").await.decision("name"), None);
    }

    #[tokio::test]
    async fn taking_a_review_over_keeps_the_reviewers_filter() {
        // The same rule as a save: a take-over that dropped the filter would
        // return a reviewer from "every field" to changed-only, as if rows had
        // disappeared.
        let (state, _) = test_state("review-takeover-filter").await;
        let (_, first) = a_reviewer(&state, "one@dasch.swiss", "First Reviewer").await;
        let (_, second) = a_reviewer(&state, "two@dasch.swiss", "Second Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(&app, post("/review/0801d", "intent=claim"), &first).await;
        let body = body_string(as_session(&app, get("/review/0801d?show=all"), &second).await).await;
        // Two hidden filter inputs: the diff form's and the take-over form's.
        assert_eq!(body.matches(r#"name="show" value="all""#).count(), 2, "{body}");

        let response = as_session(&app, post("/review/0801d", "intent=claim&show=all"), &second).await;
        assert_eq!(location(&response).as_deref(), Some("/review/0801d?show=all"));
    }

    #[tokio::test]
    async fn the_in_place_editor_is_the_control_the_depositor_form_renders() {
        // Not a second dispatch. The first one keyed off whether the value
        // happened to hold a newline, so `startDate` came out as free text
        // where the form gives a date picker, and `shortDescription` lost the
        // 200-character cap its own hint promises — with nothing server-side to
        // catch either, because the cap is an HTML attribute.
        let (state, _) = test_state("review-controls").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(
            &state,
            "0801d",
            None,
            json!({ "startDate": "2020-01-01", "shortDescription": "A new teaser." }),
        )
        .await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review/0801d"), &session).await).await;
        assert!(body.contains(r#"id="startDate" name="startDate" type="date""#), "{body}");
        assert!(body.contains(r#"maxlength="200""#), "{body}");
    }

    #[tokio::test]
    async fn a_decision_is_stored_and_comes_back_on_the_next_load() {
        let (state, _) = test_state("review-decide").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        let response = as_session(
            &app,
            post("/review/0801d", "intent=save&decision.name=accept&name=A%20new%20name"),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        let stored = stored_review_state(&state, "0801d").await;
        assert_eq!(stored.decision("name"), Some(Decision::Accept));
        assert_eq!(stored.substitute("name"), None, "nothing was edited");

        let body = body_string(as_session(&app, get("/review/0801d"), &session).await).await;
        assert!(body.contains("1 accepted"), "{body}");
    }

    #[tokio::test]
    async fn accept_all_decides_the_rows_nobody_has_looked_at() {
        // The batched select-all: one request over the whole selection, so a
        // partial failure is one server-side transaction rather than N.
        let (state, _) = test_state("review-accept-all").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(
            &state,
            "0801d",
            None,
            json!({ "name": "A new name", "provenance": "New provenance" }),
        )
        .await;
        let app = test_app(&state);

        as_session(&app, post("/review/0801d", "intent=accept-all&decision.name=revert"), &session).await;

        let stored = stored_review_state(&state, "0801d").await;
        // An explicit decision is not overwritten: "accept all remaining" means
        // the rows nobody has decided, not every row.
        assert_eq!(stored.decision("name"), Some(Decision::Revert));
        assert_eq!(stored.decision("provenance"), Some(Decision::Accept));
    }

    #[tokio::test]
    async fn editing_a_field_in_place_stores_a_substitute_and_keeps_what_was_submitted() {
        // Edit-in-place. A depositor's submission needs no second approver, so
        // their own value has to survive beside the substitution or nobody ever
        // sees what changed.
        let (state, _) = test_state("review-edit").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        let submission = a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post(
                "/review/0801d",
                "intent=save&decision.name=accept&name=A%20reviewer%27s%20wording",
            ),
            &session,
        )
        .await;

        let stored = stored_review_state(&state, "0801d").await;
        assert_eq!(stored.substitute("name"), Some(&json!("A reviewer's wording")));

        let after = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.payload, submission.payload, "the submitted payload is never rewritten");
    }

    #[tokio::test]
    async fn a_value_differing_only_in_whitespace_is_not_a_substitution() {
        // The reviewer's edit goes through the form's own applier, so it obeys
        // the rules that make an untouched value write no bytes. A second
        // comparison here would agree with them only by inspection.
        let (state, _) = test_state("review-untouched").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", "intent=save&decision.name=accept&name=%20A%20new%20name%20"),
            &session,
        )
        .await;

        assert_eq!(stored_review_state(&state, "0801d").await.substitute("name"), None);
    }

    #[tokio::test]
    async fn a_reverted_row_stores_no_substitute_even_if_a_value_is_posted() {
        // A reverted row renders read-only and posts no value control, so a
        // value arriving with one came from a hand-built body — and attaching
        // it to a decision that discards it would show a substitution the
        // surface never offered.
        let (state, _) = test_state("review-revert").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post("/review/0801d", "intent=save&decision.name=revert&name=Something%20else"),
            &session,
        )
        .await;

        let stored = stored_review_state(&state, "0801d").await;
        assert_eq!(stored.decision("name"), Some(Decision::Revert));
        assert_eq!(stored.substitute("name"), None);
    }

    #[tokio::test]
    async fn a_decision_on_a_field_the_submission_does_not_change_is_ignored() {
        // An unchanged field renders no decision control, so one naming it came
        // from a hand-built body. Honouring it would store a decision the
        // surface can never show, and therefore never undo.
        let (state, _) = test_state("review-unchanged").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(
            &app,
            post(
                "/review/0801d",
                "intent=save&decision.name=accept&decision.shortDescription=revert",
            ),
            &session,
        )
        .await;

        let stored = stored_review_state(&state, "0801d").await;
        assert_eq!(stored.decision("shortDescription"), None);
    }

    #[tokio::test]
    async fn an_unknown_decision_leaves_the_field_undecided() {
        // Silently reading one as `Accept` would approve a change on the
        // strength of a value this build does not understand.
        let (state, _) = test_state("review-unknown-decision").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        as_session(&app, post("/review/0801d", "intent=save&decision.name=approve"), &session).await;

        assert_eq!(stored_review_state(&state, "0801d").await.decision("name"), None);
    }

    #[tokio::test]
    async fn a_save_comes_back_showing_what_the_reviewer_was_looking_at() {
        // The form carries its own filter. Without it a reviewer on "every
        // field" is silently returned to the changed-only view by their own
        // save — on the enhanced path without even a navigation to explain it.
        let (state, _) = test_state("review-filter").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        let all = body_string(as_session(&app, get("/review/0801d?show=all"), &session).await).await;
        assert!(all.contains(r#"name="show" value="all""#), "{all}");

        let plain = as_session(
            &app,
            post("/review/0801d", "intent=save&show=all&decision.name=accept"),
            &session,
        )
        .await;
        assert_eq!(location(&plain).as_deref(), Some("/review/0801d?show=all"));

        let region = body_string(
            as_session(
                &app,
                enhanced("/review/0801d", "intent=save&show=all&decision.name=accept"),
                &session,
            )
            .await,
        )
        .await;
        assert!(region.contains("Show only changed fields"), "still on every field: {region}");
    }

    #[tokio::test]
    async fn the_enhanced_path_answers_the_region_rather_than_a_redirect() {
        // A 303 followed by a full document would hand Datastar an `<html>` to
        // patch; the enhanced path never navigated, so it needs no redirect.
        let (state, _) = test_state("review-enhanced").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        let response = as_session(&app, enhanced("/review/0801d", "intent=save&decision.name=accept"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.starts_with(r#"<section id="review-surface""#), "{body}");
        assert!(!body.contains("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("Review decisions saved."), "{body}");
    }

    #[tokio::test]
    async fn no_get_on_this_surface_changes_anything_and_every_write_url_answers_get() {
        // A state-changing `GET` is the one thing the `Sec-Fetch-Site` control
        // cannot cover, because a navigation from anywhere is a `GET`. Opening
        // the diff is the tempting place to break that — claiming on open is
        // one line — and it would also make a shared link claim the submission
        // for whoever followed it.
        let (state, _) = test_state("review-method").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "name": "A new name" })).await;
        let app = test_app(&state);

        // The write URL answers `GET`, so a refused write re-renders somewhere
        // a reader can stay rather than at a bare 405.
        let response = as_session(&app, get("/review/0801d"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);

        let after = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.state, SubmissionState::Submitted, "opening it must not claim it");
        assert_eq!(after.reviewed_by, None);
        assert_eq!(after.review_state, None);
    }

    #[tokio::test]
    async fn a_multilingual_field_is_edited_one_language_at_a_time() {
        // The insta snapshots cover the markup; this covers the round trip,
        // which is where a language map's names and its applier have to agree.
        let (state, _) = test_state("review-multilingual").await;
        let (_, session) = a_reviewer(&state, "rdu@dasch.swiss", "A Reviewer").await;
        a_submission(&state, "0801d", None, json!({ "abstract": { "en": "A submitted abstract." } })).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get("/review/0801d"), &session).await).await;
        assert!(body.contains(r#"name="abstract.en""#), "{body}");

        as_session(
            &app,
            post(
                "/review/0801d",
                "intent=save&decision.abstract=accept&abstract.en=A%20reviewed%20abstract.",
            ),
            &session,
        )
        .await;

        let stored = stored_review_state(&state, "0801d").await;
        assert_eq!(stored.substitute("abstract"), Some(&json!({ "en": "A reviewed abstract." })));
    }
}
