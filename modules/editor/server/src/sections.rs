//! The project form: `GET` and `POST /projects/{shortcode}/sections/{section}`.
//!
//! `/projects/{shortcode}` redirects here ([`crate::projects::detail`]).
//!
//! Four invariants, each of which fails silently if changed:
//!
//! - **The enhanced path answers 200 even when it refuses.** Datastar processes a response body
//!   only on a 200, so a `409` would drop the refusal it is carrying. The outcome goes on the span
//!   instead.
//! - **The plain path redirects after a save**, or a `POST` left in the history re-posts on
//!   refresh. A *refusal* re-renders on both paths, because a redirect would throw away what was
//!   typed.
//! - **The draft key is `normalize_shortcode`**, not the path segment: `drafts.shortcode` is
//!   exact-match while the published lookup and `may_reach` both fold, so keying as typed gives
//!   `/projects/080c` and `/projects/080C` a row each for one project.
//! - **`Section::fields_for` is the only gate**, deciding both what renders and what is applied, so
//!   a depositor cannot write an RDU-only field by naming it. A field with no declared shape is
//!   never applied, so its stored value rides through untouched (REQ-1.7).

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, Utc};
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody};
use editor_core::records::{
    normalize_shortcode, DraftRecord, ReviewOutcome, ReviewRound, Submission, SubmissionState, User,
};
use editor_core::repository::{
    DraftRepository, RepositoryError, ReviewRoundRepository, SubmissionRepository, Transition,
};
use editor_core::review::{diff, FieldDiff, ReviewState};
use editor_core::submission::unresolved_temporal_coverage;
use editor_web::form::registry::{self, Audience, Section};
use editor_web::form::INTENT;
use editor_web::pages::section as page;
use platform_metadata::is_valid_shortcode;
use uuid::Uuid;

use crate::auth::guard::Authenticated;
use crate::AppState;

/// The header the vendored Datastar bundle sets on every fetch it makes.
///
/// Presence is the whole test: the bundle sends the literal `true`, and a
/// request that carries the name at all came from it. A hand-built request can
/// set it too, which changes nothing that matters — both renderings are the same
/// data, and neither is a permission.
const DATASTAR_REQUEST: &str = "datastar-request";

const SAVE_REFUSED_LOCKED: &str = "This project is in review, so the draft cannot be changed. Nothing was saved.";
const SAVE_REFUSED_STORAGE: &str = "The draft could not be saved. Nothing was changed — try again, and if it keeps \
                                    happening the service needs attention.";
const SUBMIT_REFUSED_INCOMPLETE: &str = "This project cannot be submitted yet: some fields the repository requires \
                                         have no value. Your draft has been saved — fill the remaining fields and \
                                         submit again.";
// Deliberately says nothing about *where* the fields are. Validation is
// whole-project while the form is sectioned, so the fields at fault are
// routinely on another page — a message promising "the fields below" is read
// out beside an empty form on exactly those refusals.
const SUBMIT_REFUSED_INVALID: &str = "This project cannot be submitted yet. Your draft has been saved, and each \
                                      field that needs changing is named below with a link to it.";
const SUBMIT_REFUSED_UNCHANGED: &str = "There is nothing to submit: this project's values are identical to what is \
                                        published. Change something first — a submission with no changes would sit \
                                        in RDU's queue with nothing to review.";
const SUBMIT_REFUSED_RACED: &str = "This project already has a submission waiting for review — somebody submitted \
                                    it while this form was open. Your draft has been saved.";
const SUBMIT_REFUSED_STORAGE: &str = "The submission could not be recorded, so this project is not in RDU's queue. \
                                      Your draft has been saved — try again, and if it keeps happening the service \
                                      needs attention.";
const WITHDRAW_REFUSED_GONE: &str = "There is no submission to take back — RDU finished reviewing it, or it was \
                                     already withdrawn, while this page was open.";
const WITHDRAW_REFUSED_STORAGE: &str = "The submission could not be taken back, so it is still in RDU's queue. Try \
                                        again, and if it keeps happening the service needs attention.";

/// The field-level error an unresolvable `temporalCoverage` entry renders as.
///
/// Says what to do, not only what is wrong: the `Reference` variant always
/// resolves, which is the escape route REQ-1.15's refusal decision rests on —
/// without naming it, a depositor whose period the table does not know is
/// simply stuck.
fn unresolved_message(name: &str) -> String {
    format!(
        "\"{name}\" cannot be matched to a date range, and the repository needs one for every period. Either pick \
         the period from ChronOntology instead of typing it, or ask RDU to add it to the date table."
    )
}

/// Which fields and sections this reader sees.
fn audience_of(user: &User) -> Audience {
    if user.is_rdu() {
        Audience::RduOnly
    } else {
        Audience::Everyone
    }
}

/// Everything both handlers need, once the request is known to be allowed.
struct Context<'a> {
    section: &'static Section,
    audience: Audience,
    /// The draft as it stands: the stored one, or the published project
    /// pre-filled (REQ-1.1), or empty for a project with neither (REQ-2.3).
    draft: ProjectDraft,
    /// The stored row, `None` when nothing has been saved over the published
    /// metadata yet. Its `created_at` is preserved across a save.
    record: Option<DraftRecord>,
    /// Set while a submission is awaiting or under review.
    locked: Option<page::Locked>,
    /// The pending submission, when there is one. The same read `locked` comes
    /// from, kept rather than reduced to a flag: withdrawing needs its id, and
    /// a second lookup could see a different row.
    submission: Option<Submission>,
    /// Fields RDU accepted in the round being answered, which are therefore
    /// fixed until the project is submitted again (REQ-4.5).
    ///
    /// The applier skip reads this, not just the renderer: a control that does
    /// not render still posts nothing, but a hand-built body can name the field
    /// anyway, and the whole point is that an accepted value cannot re-enter
    /// review altered.
    accepted_fields: Vec<String>,
    /// The latest finished review round, or `None` for a project nobody has
    /// reviewed.
    ///
    /// From `review_rounds` rather than from the draft, because the note has to
    /// be read beside the outcome it belongs to and only the round carries
    /// both. A save therefore cannot disturb it, which is what the column on
    /// `drafts` needed a rule to guarantee.
    ///
    /// Every outcome shows, not only the one that asked for changes: a
    /// rejection is otherwise invisible (REQ-4.6 discards and notifications are
    /// out of scope), and an approval is where the depositor is shown what RDU
    /// substituted for their values. It shows until the next submission, which
    /// starts the next cycle.
    round: Option<RoundStrings>,
    /// The published project's name, for the heading and the tab title.
    project_name: Option<&'a str>,
}

/// One round's owned strings and values, so the view can borrow them.
struct RoundStrings {
    outcome: ReviewOutcome,
    note: Option<String>,
    at: String,
    substitutions: Vec<(String, serde_json::Value)>,
}

impl Context<'_> {
    /// The submission a withdrawal would take back, if there is one.
    ///
    /// `Approved` is excluded: that record is on its way to a pull request and
    /// is no longer the depositor's to take back — deleting it is exactly the
    /// terminal-state failure a reject landing after an approve would cause.
    /// The renderer and the handler both read this, so the control cannot be
    /// offered where the write would refuse.
    fn pending_submission(&self) -> Option<&Submission> {
        self.submission.as_ref().filter(|s| s.state != SubmissionState::Approved)
    }
}

/// Resolve a request, or the response that refuses it.
///
/// The order is deliberate and matches [`crate::projects::detail`]: shape, then
/// authorization, then anything that reads state. A 404 for an unpublished
/// shortcode and a 403 for a published one would make the pair an oracle for
/// which projects exist, to a reader who is not allowed to know.
async fn context<'a>(
    state: &'a AppState,
    user: &User,
    shortcode: &str,
    section_id: &str,
) -> Result<Context<'a>, Response> {
    if !is_valid_shortcode(shortcode) {
        return Err(crate::not_found(State(state.clone())).await);
    }
    if !user.may_reach(shortcode) {
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused a project that is not assigned to this account"
        );
        return Err(crate::forbidden(state, user, crate::projects::NOT_ASSIGNED));
    }

    let audience = audience_of(user);
    // A section this reader does not see is a 404 rather than a 403: unlike the
    // project above, the reader invented the segment, and there is no assignment
    // question a 403 would be answering. `legal` is RDU-only, and a depositor's
    // rail does not link to it at all.
    let Some(section) = registry::section(section_id)
        .filter(|section| registry::sections_for(audience).any(|visible| visible.id == section.id))
    else {
        return Err(crate::not_found(State(state.clone())).await);
    };

    let key = normalize_shortcode(shortcode);
    let record = match DraftRepository::find(&*state.db, &key).await {
        Ok(record) => record,
        Err(error) => return Err(storage_error(state, user, "read this project's draft", &error)),
    };
    // Everything short of `Approved` locks the form. An approved record is
    // waiting to be collected into a pull request and is no longer the
    // depositor's to wait on, so editing again starts the next cycle rather
    // than disturbing a review in progress.
    let submission = match SubmissionRepository::find_by_shortcode(&*state.db, &key).await {
        Ok(submission) => submission,
        Err(error) => return Err(storage_error(state, user, "read this project's submission", &error)),
    };
    let locked = submission.as_ref().and_then(|submission| match submission.state {
        SubmissionState::Submitted => Some(page::Locked::Submitted),
        SubmissionState::InReview => Some(page::Locked::InReview),
        SubmissionState::Approved => None,
    });

    // Newest first, so the head is the round that decides what the form says.
    // Read even while a submission is pending: the note describes the round the
    // depositor is answering, and the form is read-only rather than blank.
    let latest_round = match ReviewRoundRepository::list_for_shortcode(&*state.db, &key).await {
        Ok(mut rounds) => rounds.drain(..).next(),
        Err(error) => return Err(storage_error(state, user, "read this project's review history", &error)),
    };
    // One parse of the round's decisions, read for two different things: which
    // fields are fixed, and which values RDU put in place of the depositor's.
    // A second parse could disagree with the first.
    let decisions = latest_round.as_ref().map(|round| {
        let (decisions, error) = ReviewState::parse(round.review_state.as_deref());
        if let Some(error) = error {
            // Read as "nothing decided", never as a refusal: the depositor can
            // still edit and resubmit, where a 500 would strand the project
            // until somebody edited the database. It fails open on purpose —
            // the alternative locks fields nobody can prove were accepted.
            tracing::error!(
                error = %error,
                project.shortcode = %shortcode,
                "a stored review state could not be parsed; no field is locked"
            );
        }
        decisions
    });

    // Only a round that asked for changes fixes fields. A reject's or a
    // withdrawal's decisions are moot — the submission they were recorded
    // against is gone, so nothing is being answered — and an approval's belong
    // to work that has left the depositor's hands.
    let accepted_fields = latest_round
        .as_ref()
        .filter(|round| round.outcome == ReviewOutcome::ChangesRequested)
        .and(decisions.as_ref())
        .map(|decisions| decisions.accepted_fields().into_iter().map(str::to_string).collect())
        .unwrap_or_default();

    let round = latest_round.as_ref().map(|round| RoundStrings {
        outcome: round.outcome,
        note: round.note.clone(),
        at: crate::format_instant(round.at),
        substitutions: decisions
            .as_ref()
            .map(|decisions| {
                decisions
                    .substitutions()
                    .into_iter()
                    .map(|(field, value)| (field.to_string(), value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    });

    let published = state.published.get(shortcode);
    let draft = match &record {
        // A stored draft supersedes the published metadata: REQ-1.1 pre-fills
        // from what is published, and REQ-1.10 keeps what was saved over it.
        Some(record) => serde_json::from_str(&record.payload).unwrap_or_else(|error| {
            // A payload this build cannot parse is a stored-state problem, and
            // falling back to the published project would silently discard the
            // depositor's work the moment they saved. Empty is the honest
            // answer: the form renders blank, nothing is pre-filled from
            // somewhere else, and a save writes only what is entered.
            tracing::error!(
                error = %error,
                project.shortcode = %shortcode,
                "a stored draft payload could not be parsed"
            );
            ProjectDraft::default()
        }),
        None => published.map(ProjectDraft::from_raw).unwrap_or_default(),
    };

    Ok(Context {
        section,
        audience,
        draft,
        record,
        locked,
        submission,
        accepted_fields,
        round,
        project_name: published.map(|project| project.name.as_str()),
    })
}

/// `GET /projects/{shortcode}/sections/{section}`.
pub(crate) async fn show(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path((shortcode, section_id)): Path<(String, String)>,
) -> Response {
    let context = match context(&state, &user, &shortcode, &section_id).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    render_page(&state, &user, &shortcode, &context, Rendering::default())
}

/// `POST /projects/{shortcode}/sections/{section}` — save the draft (REQ-1.10),
/// submit it (REQ-1.12), or take a pending submission back (REQ-4.7).
///
/// One URL and one `GET` for all three, discriminated by the `intent` pair the
/// activated submit control carries — the same shape the review surface uses,
/// and for the same reason: a second write URL would need a `GET` of its own or
/// strand a refused write on a bare 405.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "project section action",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.section = tracing::field::Empty,
        form.intent = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn act(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path((shortcode, section_id)): Path<(String, String)>,
    headers: HeaderMap,
    // Last, because it consumes the body. `Vec<(String, String)>` rather than a
    // struct: `serde_urlencoded` errors on a repeated key and cannot deserialize
    // a struct holding a `Vec` at all — see `editor_core::form`.
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(&shortcode));
    span.record("form.section", tracing::field::display(&section_id));

    let mut context = match context(&state, &user, &shortcode, &section_id).await {
        Ok(context) => context,
        Err(response) => {
            span.record("form.outcome", "refused");
            return response;
        }
    };

    let body = FormBody::from_pairs(pairs);
    // Anything this build does not know falls back to `save`: submit and
    // withdraw are not undoable by the depositor and a save is, so a body
    // naming an unknown verb must take the recoverable branch.
    let intent = match body.get(INTENT) {
        Some(page::SUBMIT) => Intent::Submit,
        Some(page::WITHDRAW) => Intent::Withdraw,
        Some(page::WITHDRAW_CONFIRM) => Intent::ConfirmWithdrawal,
        _ => Intent::Save,
    };
    span.record("form.intent", tracing::field::display(intent.as_str()));

    if intent == Intent::Withdraw || intent == Intent::ConfirmWithdrawal {
        return withdraw(&state, &user, &shortcode, &context, headers, intent).await;
    }

    // Re-checked here and not only when the form was rendered: the render is a
    // `GET`, so nothing stops a `POST` arriving without one — or arriving after
    // a reviewer picked the project up in the meantime.
    if context.locked.is_some() {
        span.record("form.outcome", "locked");
        tracing::info!("refused a write against a project that is in review");
        return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_LOCKED);
    }

    let mut applied = 0;
    for field in context.section.fields_for(context.audience) {
        // An accepted field is skipped here, which is the gate. Not rendering
        // its control stops an ordinary browser posting it; only this stops a
        // hand-built body, and REQ-4.5's point is that an accepted value
        // cannot re-enter review altered.
        if context.accepted_fields.iter().any(|accepted| accepted == field.id) {
            continue;
        }
        if let Some(shape) = field.shape {
            apply(shape, &body, &mut context.draft, field.id);
            applied += 1;
        }
    }

    let now = Utc::now();
    let record = DraftRecord {
        shortcode: normalize_shortcode(&shortcode),
        payload: match serde_json::to_string(&context.draft) {
            Ok(payload) => payload,
            Err(error) => {
                span.record("form.outcome", "serialize_failed");
                tracing::error!(error = %error, "a draft could not be serialized");
                return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_STORAGE);
            }
        },
        updated_by: Some(user.id),
        // Preserved from the stored row: "created" keeps meaning when the draft
        // was first saved rather than when it was last. The reviewer's note
        // needs no such care — it lives on the round, which nothing here
        // writes, so a save cannot disturb it.
        created_at: context.record.as_ref().map_or(now, |record| record.created_at),
        updated_at: now,
    };

    // The draft is written on both intents, and first. A submit that stored
    // only the submission would lose whatever was typed in the same post if
    // validation then refused it — and the draft is what the depositor comes
    // back to when RDU returns the project (REQ-1.13).
    if let Err(error) = DraftRepository::upsert(&*state.db, &record).await {
        span.record("form.outcome", "store_failed");
        tracing::error!(error = %error, "could not save a project draft");
        return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_STORAGE);
    }
    context.record = Some(record.clone());

    if intent == Intent::Save {
        span.record("form.outcome", "saved");
        tracing::info!(fields.applied = applied, "saved a project draft");
        return saved(&shortcode, &context, headers);
    }

    submit(&state, &user, &shortcode, &context, headers, &record, now).await
}

/// What a `POST` to this route is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Save,
    Submit,
    Withdraw,
    ConfirmWithdrawal,
}

impl Intent {
    /// The span value. Not [`page`]'s constants: those are a wire vocabulary
    /// the browser sends, and an attribute keyed on them would report whatever
    /// a hand-built body typed. This reports what the server decided to do.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Submit => "submit",
            Self::Withdraw => "withdraw",
            Self::ConfirmWithdrawal => "withdraw-confirm",
        }
    }
}

/// Record the draft as the project's pending submission (REQ-1.12).
///
/// The draft has already been written, so every branch here leaves the
/// depositor's work in place — a refusal costs them the submission, never the
/// editing.
async fn submit(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    record: &DraftRecord,
    now: DateTime<Utc>,
) -> Response {
    let span = tracing::Span::current();

    // Type-level first: a draft that cannot become a `ProjectRaw` is missing a
    // member the contract requires, and no per-field rule below can say
    // anything useful about a shape that does not exist.
    let raw = match context.draft.to_raw() {
        Ok(raw) => raw,
        Err(error) => {
            span.record("form.outcome", "incomplete");
            tracing::info!(error = %error, "refused a submission whose draft is not a complete project");
            return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_INCOMPLETE, &[]);
        }
    };

    // REQ-1.14, through the same function `dpe-server validate` and
    // `dpe-api-oai` apply, so the three cannot disagree about what counts as a
    // gap. Re-run on every submit, which is what makes a resubmission
    // revalidated rather than trusted because it was reviewed once.
    let unresolved = unresolved_temporal_coverage(&raw, &state.temporal.periods, &state.temporal.enrichment);
    if !unresolved.is_empty() {
        span.record("form.outcome", "invalid");
        tracing::info!(
            fields.invalid = unresolved.len(),
            "refused a submission with unresolvable periods"
        );
        let errors: Vec<(String, String)> = unresolved
            .iter()
            .map(|entry| (entry.field_path(), unresolved_message(&entry.name)))
            .collect();
        return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_INVALID, &errors);
    }

    // Nothing to review. Allowed through, this locks the depositor's own form
    // on a submission a reviewer can only clear by rejecting it. The comparison
    // is `editor_core::review::diff`, the one the review surface itself renders
    // from, so "changes nothing" means here exactly what it means there.
    let published = state.published.get(shortcode).map(ProjectDraft::from_raw);
    if !diff(published.as_ref(), &context.draft).iter().any(FieldDiff::changed) {
        span.record("form.outcome", "unchanged");
        tracing::info!("refused a submission identical to the published project");
        return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_UNCHANGED, &[]);
    }

    let submission = Submission {
        id: Uuid::new_v4(),
        shortcode: record.shortcode.clone(),
        // The draft as it now stands, copied rather than referenced: the
        // depositor keeps editing their draft the moment RDU returns the
        // project, and a review has to be against what was sent.
        payload: record.payload.clone(),
        state: SubmissionState::Submitted,
        submitted_by: Some(user.id),
        submitted_at: now,
        reviewed_by: None,
        reviewed_at: None,
        reviewer_note: None,
        review_state: None,
    };

    match SubmissionRepository::create(&*state.db, &submission).await {
        Ok(()) => {
            span.record("form.outcome", "submitted");
            tracing::info!(
                submission.id = %submission.id,
                auth.role = %user.role,
                "recorded a pending submission"
            );
            phase_changed(
                state,
                user,
                shortcode,
                context.section.id,
                context,
                headers,
                page::Notice::Submitted,
            )
            .await
        }
        // The unique index on `shortcode`: a submission was made for this
        // project between the read that rendered the form and this write.
        // Reported rather than replacing the first, which is what the
        // "one pending submission per project" constraint is for.
        Err(RepositoryError::Conflict { .. }) => {
            span.record("form.outcome", "already_pending");
            tracing::info!("refused a second pending submission for one project");
            refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_RACED, &[])
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not record a pending submission");
            refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_STORAGE, &[])
        }
    }
}

/// Take a pending submission back (REQ-4.7), or show the confirmation that
/// posts it.
///
/// The form is read-only while a submission is pending, so this is the one
/// write here that runs *because* the project is locked rather than in spite of
/// it.
async fn withdraw(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    intent: Intent,
) -> Response {
    let span = tracing::Span::current();

    // No pending submission: it was reviewed or withdrawn while this page was
    // open. The terminal-state guard would say the same, but saying it here
    // avoids inventing a round id for a transition that cannot happen.
    let Some(submission) = context.pending_submission() else {
        span.record("form.outcome", "gone");
        tracing::info!("refused a withdrawal of a submission that is no longer pending");
        return refused_with(state, user, shortcode, context, headers, WITHDRAW_REFUSED_GONE, &[]);
    };

    if intent == Intent::ConfirmWithdrawal {
        span.record("form.outcome", "confirming");
        return confirming(state, user, shortcode, context, headers);
    }

    let round = ReviewRound {
        id: Uuid::new_v4(),
        shortcode: submission.shortcode.clone(),
        submission_id: submission.id,
        outcome: ReviewOutcome::Withdrawn,
        // Nobody to address: the person who withdrew it is the person who would
        // read the note.
        note: None,
        // Whatever a reviewer had recorded, kept as evidence rather than
        // dropped — it is the only remaining answer to what had been decided on
        // a submission the depositor then took back.
        review_state: submission.review_state.clone(),
        actor: Some(user.id),
        at: Utc::now(),
    };

    match ReviewRoundRepository::discard(&*state.db, submission.id, &round).await {
        Ok(Transition::Applied) => {
            span.record("form.outcome", "withdrawn");
            tracing::info!(submission.id = %submission.id, "withdrew a pending submission");
            phase_changed(
                state,
                user,
                shortcode,
                context.section.id,
                context,
                headers,
                page::Notice::Withdrawn,
            )
            .await
        }
        Ok(Transition::AlreadyReviewed) => {
            span.record("form.outcome", "gone");
            tracing::info!("a withdrawal lost the race with a review");
            refused_with(state, user, shortcode, context, headers, WITHDRAW_REFUSED_GONE, &[])
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not withdraw a pending submission");
            refused_with(state, user, shortcode, context, headers, WITHDRAW_REFUSED_STORAGE, &[])
        }
    }
}

/// Whether this request came from the Datastar bundle.
fn is_enhanced(headers: &HeaderMap) -> bool {
    headers.contains_key(DATASTAR_REQUEST)
}

/// A successful save: a redirect on the plain path, the patched region on the
/// enhanced one.
fn saved(shortcode: &str, context: &Context<'_>, headers: HeaderMap) -> Response {
    if !is_enhanced(&headers) {
        // POST-redirect-GET: a `POST` left in the history re-posts on refresh,
        // and the reloaded `GET` reads the row that was just written, so the
        // "last saved" line is the confirmation rather than a flash message
        // that has to survive a redirect.
        return redirect_here(shortcode, context);
    }
    region(
        shortcode,
        context,
        Rendering { notice: Some(page::Notice::Saved), ..Rendering::default() },
    )
}

/// A write that moved the project between phases: a submission recorded, or
/// one taken back.
///
/// The plain path redirects, for the reason a save does — and the reloaded
/// `GET` renders the new phase, which is a more durable confirmation than a
/// flash message. The enhanced path patches the region, and **re-resolves the
/// request first**: submitting locks every field and withdrawing unlocks them,
/// so the context that answered the `POST` describes the phase that has just
/// ended. Overriding two of its fields would leave the rest — `may_withdraw`,
/// the accepted set, the round — describing the old one, silently.
async fn phase_changed(
    state: &AppState,
    user: &User,
    shortcode: &str,
    section_id: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    notice: page::Notice<'_>,
) -> Response {
    if !is_enhanced(&headers) {
        return redirect_here(shortcode, context);
    }
    match self::context(state, user, shortcode, section_id).await {
        Ok(fresh) => region(shortcode, &fresh, Rendering { notice: Some(notice), ..Rendering::default() }),
        // The write landed; only the re-read did not. A redirect is the
        // fail-safe answer — the browser follows it and finds the new phase,
        // where a refusal would report a failure that did not happen.
        Err(_) => redirect_here(shortcode, context),
    }
}

/// The withdrawal confirmation, rendered over the read-only form.
fn confirming(state: &AppState, user: &User, shortcode: &str, context: &Context<'_>, headers: HeaderMap) -> Response {
    let rendering = Rendering { confirming_withdrawal: true, ..Rendering::default() };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

fn redirect_here(shortcode: &str, context: &Context<'_>) -> Response {
    Redirect::to(&format!("/projects/{shortcode}/sections/{}", context.section.id)).into_response()
}

/// A refused save, re-rendered with what was typed still in the form.
fn refused(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    message: &str,
) -> Response {
    refused_with(state, user, shortcode, context, headers, message, &[])
}

/// A refused write, with any field-level errors beside the controls they name.
///
/// Re-rendered rather than redirected on both paths, which is REQ-1.13: a
/// redirect would throw away what was typed, and the invalid values are exactly
/// what the depositor has to see to fix them.
fn refused_with(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    message: &str,
    errors: &[(String, String)],
) -> Response {
    let rendering = Rendering {
        notice: Some(page::Notice::Refused(message)),
        errors,
        confirming_withdrawal: false,
    };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

/// What a rendering adds on top of the resolved request.
///
/// A struct rather than four arguments: three of them are `Option`s of similar
/// types and the fourth is a `bool`, which is exactly the set a positional list
/// lets a call site scramble silently. `Default` is "nothing to report", which
/// is what a plain `GET` renders.
#[derive(Default)]
struct Rendering<'a> {
    notice: Option<page::Notice<'a>>,
    errors: &'a [(String, String)],
    confirming_withdrawal: bool,
}

/// The section region, as the enhanced path's `datastar-patch-elements`.
///
/// 200 always: Datastar processes a response body only on a 200, so a status
/// carrying the refusal would lose the message it is carrying.
fn region(shortcode: &str, context: &Context<'_>, rendering: Rendering<'_>) -> Response {
    let stored = saved_at(context);
    let view = view(shortcode, context, stored.as_deref(), rendering);
    (StatusCode::OK, axum::response::Html(page::region(&view).into_string())).into_response()
}

/// The whole page, inside the document shell.
fn render_page(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    rendering: Rendering<'_>,
) -> Response {
    let stored = saved_at(context);
    let view = view(shortcode, context, stored.as_deref(), rendering);
    // The published name in the tab title where there is one: a browser with
    // eleven tabs open shows about twenty characters, and five of them being
    // "Proje" helps nobody.
    let title = match context.project_name {
        Some(name) => format!("{} — {name} — DaSCH Metadata Editor", context.section.title),
        None => format!("{} — Project {shortcode} — DaSCH Metadata Editor", context.section.title),
    };
    crate::render(state, &title, StatusCode::OK, Some(user), page::page(&view))
}

fn saved_at(context: &Context<'_>) -> Option<String> {
    context.record.as_ref().map(|record| crate::format_instant(record.updated_at))
}

fn view<'a>(
    shortcode: &'a str,
    context: &'a Context<'a>,
    saved_at: Option<&'a str>,
    rendering: Rendering<'a>,
) -> page::SectionView<'a> {
    page::SectionView {
        shortcode,
        project_name: context.project_name,
        section: context.section,
        audience: context.audience,
        draft: &context.draft,
        locked: context.locked,
        accepted_fields: &context.accepted_fields,
        // RDU too, not only the assigned depositors: `may_reach` is already
        // true for every project for an RDU account, and a submission nobody
        // can take back is one only a reject can clear. The same predicate the
        // write applies, so the control is never offered where it is refused.
        may_withdraw: context.pending_submission().is_some(),
        confirming_withdrawal: rendering.confirming_withdrawal,
        errors: rendering.errors,
        round: context.round.as_ref().map(|round| page::RoundSummary {
            outcome: round.outcome,
            note: round.note.as_deref(),
            at: &round.at,
            substitutions: &round.substitutions,
        }),
        saved_at,
        notice: rendering.notice,
    }
}

/// Storage would not answer, so the page cannot show what it should.
fn storage_error(state: &AppState, user: &User, what: &str, error: &RepositoryError) -> Response {
    tracing::error!(error = %error, operation = what, "the project form could not reach storage");
    crate::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(user),
        editor_web::pages::problem::unavailable(
            "The editor could not reach its database, so this form is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::Request;
    use editor_core::canonical::write_draft;
    use editor_core::records::Role;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::{
        a_session, a_user, body_string, count_rows, get, location, open_test_db, post, state_over, test_app,
        test_state, with_cookie, Faults, FaultyDatabase, RecordingMailer,
    };

    /// Percent-encode a form value. Only the three characters the fixtures
    /// actually carry — a general encoder would be a dependency for one call.
    fn urlencoding(value: &str) -> String {
        value.replace('%', "%25").replace('&', "%26").replace(' ', "+")
    }

    const OVERVIEW: &str = "/projects/0801d/sections/overview";

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

    #[tokio::test]
    async fn a_depositor_opens_the_form_for_a_project_assigned_to_them() {
        let (state, _) = test_state("section-open").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, get(OVERVIEW), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        // REQ-1.1: the form opens pre-filled from the published metadata, with
        // nothing saved over it yet.
        assert!(body.contains("Basler Edition der Bernoulli-Briefwechsel"), "{body}");
        assert!(body.contains(&format!(r#"action="{OVERVIEW}""#)), "{body}");
    }

    #[tokio::test]
    async fn a_section_this_reader_does_not_see_is_a_404_rather_than_a_403() {
        // The reader invented the segment, so there is no assignment question a
        // 403 would be answering — and `legal` is absent from a depositor's rail
        // entirely, so nothing linked them there.
        let (state, _) = test_state("section-audience").await;
        let depositor = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let app = test_app(&state);

        let session = a_session(&state, depositor.id).await;
        let refused = as_session(&app, get("/projects/0801d/sections/legal"), &session).await;
        assert_eq!(refused.status(), StatusCode::NOT_FOUND);

        let session = a_session(&state, rdu.id).await;
        let allowed = as_session(&app, get("/projects/0801d/sections/legal"), &session).await;
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_section_that_does_not_exist_is_a_404() {
        let (state, _) = test_state("section-unknown").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        for uri in [
            "/projects/0801d/sections/nope",
            "/projects/0801d/sections/OVERVIEW",
            "/projects/not%20a%20code/sections/overview",
        ] {
            let response = as_session(&app, get(uri), &session).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn a_project_that_is_not_assigned_is_a_403_on_the_form_too() {
        // REQ-1.3, checked before anything is read: a 404 for an unpublished
        // shortcode beside a 403 for a published one would make the pair an
        // oracle for which projects exist.
        let (state, _) = test_state("section-forbidden").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let read = as_session(&app, get("/projects/0803/sections/overview"), &session).await;
        assert_eq!(read.status(), StatusCode::FORBIDDEN);
        let write = as_session(&app, post("/projects/0803/sections/overview", "name=Mine"), &session).await;
        assert_eq!(write.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_signed_out_visitor_is_sent_to_login_and_back_to_the_section() {
        let (state, _) = test_state("section-anonymous").await;
        let app = test_app(&state);

        let response = app.clone().oneshot(get(OVERVIEW)).await.expect("completes");
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some(&format!("/login?next={OVERVIEW}")[..]));
    }

    #[tokio::test]
    async fn a_save_stores_the_draft_and_redirects_to_the_get() {
        // REQ-1.10, and POST-redirect-GET: a `POST` left in the history
        // re-posts on refresh.
        let (state, _) = test_state("section-save").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let saved = as_session(&app, post(OVERVIEW, "name=A+New+Title"), &session).await;
        assert_eq!(saved.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&saved).as_deref(), Some(OVERVIEW));

        let reloaded = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(reloaded.contains("A New Title"), "{reloaded}");
        // The confirmation is the stored row read back, not a flash message that
        // had to survive a redirect.
        assert!(reloaded.contains("Draft last saved"), "{reloaded}");
    }

    #[tokio::test]
    async fn the_enhanced_path_answers_with_the_region_rather_than_a_document() {
        // Datastar treats a `text/html` response as a `datastar-patch-elements`
        // and matches by `id` in `outer` mode, so a whole document would try to
        // patch `<html>`.
        let (state, _) = test_state("section-datastar").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, enhanced(OVERVIEW, "name=Enhanced+Title"), &session).await;
        // 200, not a redirect and not a 4xx: Datastar processes a body only on a
        // 200, so any other status loses what the response was carrying.
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(!body.contains("<!DOCTYPE"), "{body}");
        assert!(body.starts_with(&format!(r#"<section id="{}""#, page::REGION_ID)), "{body}");
        assert!(body.contains("Draft saved."), "{body}");
        assert!(body.contains("Enhanced Title"), "{body}");
        // The rail comes back with it, or a save that answers the last required
        // field leaves the rail still saying something is missing.
        assert!(body.contains(r#"aria-label="Form sections""#), "{body}");
    }

    #[tokio::test]
    async fn both_paths_write_the_same_draft() {
        // The point of one handler with two renderings: the difference is the
        // response, never the effect.
        let (state, _) = test_state("section-paths").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d", "0801a"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=Same+Title"), &session).await;
        as_session(&app, enhanced("/projects/0801a/sections/overview", "name=Same+Title"), &session).await;

        let plain = DraftRepository::find(&*state.db, "0801d").await.expect("read").expect("row");
        let enhanced_row = DraftRepository::find(&*state.db, "0801a").await.expect("read").expect("row");
        let name_of = |payload: &str| {
            serde_json::from_str::<ProjectDraft>(payload)
                .expect("a stored payload parses")
                .get("name")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        assert_eq!(name_of(&plain.payload).as_deref(), Some("Same Title"));
        assert_eq!(name_of(&enhanced_row.payload).as_deref(), Some("Same Title"));
    }

    #[tokio::test]
    async fn one_project_has_one_draft_however_its_shortcode_is_capitalised() {
        // `drafts.shortcode` is exact-match while the published lookup and the
        // assignment check both fold ASCII case, so keying on the path segment
        // as typed would give `/080c` and `/080C` a row each — and two people
        // editing one project would each keep half the edits with nothing to say
        // so.
        let db = std::sync::Arc::new(open_test_db("section-case").await);
        let state = state_over(db.clone(), RecordingMailer::new(), |auth| {
            auth.cooldown = std::time::Duration::ZERO;
        });
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["080C"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post("/projects/080C/sections/overview", "name=Upper"), &session).await;
        as_session(&app, post("/projects/080c/sections/overview", "name=Lower"), &session).await;

        assert_eq!(count_rows(&db, "drafts").await, 1);
        let row = DraftRepository::find(&*state.db, "080c").await.expect("read").expect("row");
        assert_eq!(row.shortcode, "080c");
        // Last write wins, which is the documented concurrency model — the point
        // is that both writes reached the same row.
        assert!(row.payload.contains("Lower"), "{}", row.payload);
    }

    #[tokio::test]
    async fn a_save_touches_only_the_fields_the_posted_section_owns() {
        // A section posts its own fields, and an applier reads an absent name as
        // "this section did not carry that field" — so saving Overview must
        // leave the Dataset section's fields exactly as they were, even though
        // the body could name them.
        let (state, _) = test_state("section-scope").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // `provenance` lives in the Dataset section and is a field the form
        // reads, so it is the strongest case: a name the decoder knows, posted
        // to a section that does not own it.
        as_session(&app, post(OVERVIEW, "name=A+New+Title&provenance=Injected"), &session).await;

        let row = DraftRepository::find(&*state.db, "0801d").await.expect("read").expect("row");
        let draft: ProjectDraft = serde_json::from_str(&row.payload).expect("parses");
        assert_eq!(draft.get("name").and_then(|v| v.as_str()), Some("A New Title"));
        assert!(draft.get("provenance").is_none(), "{}", row.payload);
    }

    #[tokio::test]
    async fn a_depositor_cannot_write_an_rdu_only_field_by_posting_it() {
        // The audience check has one home — the registry, which the decoder
        // consults too — so this is closed by the same call that decides what
        // the form renders, not by a second check here.
        let (state, _) = test_state("section-audience-write").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let before = state.published.get("0801d").expect("0801d").url.clone();
        as_session(&app, post(OVERVIEW, "name=Fine&url=https%3A%2F%2Fevil.example"), &session).await;

        let row = DraftRepository::find(&*state.db, "0801d").await.expect("read").expect("row");
        let draft: ProjectDraft = serde_json::from_str(&row.payload).expect("parses");
        assert_eq!(draft.get("name").and_then(|v| v.as_str()), Some("Fine"));
        // Unchanged, and specifically still the published value rather than the
        // posted one — `url` is RDU-only and has no declared shape either way.
        assert_eq!(draft.get("url"), before.as_ref());
    }

    /// Record a finished review round on `0801d`, as the transition that ended
    /// it would have.
    async fn a_round(state: &AppState, actor: Uuid, outcome: ReviewOutcome, note: &str) {
        a_round_with(state, actor, outcome, Some(note), None).await;
    }

    /// The same, with the note and the decision snapshot chosen.
    async fn a_round_with(
        state: &AppState,
        actor: Uuid,
        outcome: ReviewOutcome,
        note: Option<&str>,
        review_state: Option<&str>,
    ) {
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: "0801d".to_string(),
            payload: "{}".to_string(),
            state: SubmissionState::InReview,
            submitted_by: Some(actor),
            submitted_at: Utc::now(),
            reviewed_by: Some(actor),
            reviewed_at: Some(Utc::now()),
            reviewer_note: None,
            review_state: review_state.map(str::to_string),
        };
        SubmissionRepository::create(&*state.db, &submission).await.unwrap();
        let round = ReviewRound {
            id: Uuid::new_v4(),
            shortcode: submission.shortcode.clone(),
            submission_id: submission.id,
            outcome,
            note: note.map(str::to_string),
            review_state: submission.review_state.clone(),
            actor: Some(actor),
            at: Utc::now(),
        };
        let applied = ReviewRoundRepository::discard(&*state.db, submission.id, &round).await.unwrap();
        assert_eq!(applied, Transition::Applied, "the fixture's round should be recorded");
    }

    #[tokio::test]
    async fn a_returned_project_shows_the_depositor_what_rdu_asked_for() {
        // The reviewer note has no other home. The form is where the depositor
        // acts on it, so it is where it is shown — and it rides inside the
        // region, so a save does not clear it.
        let (state, _) = test_state("section-note").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_round(
            &state,
            user.id,
            ReviewOutcome::ChangesRequested,
            "Please add a German description.",
        )
        .await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(body.contains("RDU asked for changes"), "{body}");
        assert!(body.contains("Please add a German description."), "{body}");
    }

    #[tokio::test]
    async fn saving_a_draft_does_not_clear_the_reviewer_note() {
        // The note describes the round the depositor is answering, so it has to
        // survive them saving their answer to it — otherwise it disappears the
        // moment they start acting on it. It now lives on an append-only table
        // no save writes to, which is what makes that structural rather than a
        // rule the upsert has to remember; the assertion is on the *rendered*
        // form, since that is where the guarantee is observable.
        let (state, _) = test_state("section-note-save").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_round(
            &state,
            user.id,
            ReviewOutcome::ChangesRequested,
            "Please add a German description.",
        )
        .await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=A%20name"), &session).await;

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(body.contains("Please add a German description."), "{body}");
        let stored = DraftRepository::find(&*state.db, "0801d").await.unwrap().unwrap();
        assert!(stored.payload.contains("A name"), "the save landed: {}", stored.payload);
    }

    #[tokio::test]
    async fn each_outcome_gets_its_own_wording() {
        // One banner reading "RDU asked for changes" whatever happened would
        // tell a depositor whose work was rejected to go and answer a round
        // that is closed, and tell one whose work was approved to edit it.
        for (outcome, heading) in [
            (ReviewOutcome::ChangesRequested, "RDU asked for changes"),
            (ReviewOutcome::Rejected, "RDU rejected this submission"),
            (ReviewOutcome::Approved, "RDU approved this project"),
            (ReviewOutcome::Withdrawn, "The submission was taken back"),
        ] {
            let (state, _) = test_state(&format!("section-wording-{outcome}")).await;
            let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
            let session = a_session(&state, user.id).await;
            a_round(&state, user.id, outcome, "The note.").await;
            let app = test_app(&state);

            let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
            assert!(body.contains(heading), "{outcome}: {body}");
            for (_, other) in [
                (ReviewOutcome::ChangesRequested, "RDU asked for changes"),
                (ReviewOutcome::Rejected, "RDU rejected this submission"),
                (ReviewOutcome::Approved, "RDU approved this project"),
                (ReviewOutcome::Withdrawn, "The submission was taken back"),
            ] {
                if other != heading {
                    assert!(!body.contains(other), "{outcome} also rendered {other:?}");
                }
            }
        }
    }

    #[tokio::test]
    async fn a_rejected_submission_is_visible_to_the_depositor_with_its_reason() {
        // The gap this closes: REQ-4.6 discards the submission, notifications
        // are out of scope, and REQ-2.1's states have no Rejected — so without
        // this the depositor's work vanishes with no signal whatever, and
        // repeated reject cycles leave no trace anywhere.
        let (state, _) = test_state("section-rejected").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_round(&state, user.id, ReviewOutcome::Rejected, "Out of scope for this repository.").await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;

        assert!(body.contains("RDU rejected this submission"), "{body}");
        assert!(body.contains("Out of scope for this repository."), "{body}");
        // And the other half they need: their work is not gone.
        assert!(body.contains("Your draft is still here"), "{body}");
        assert!(body.contains("Submit for review"), "the form is editable again: {body}");
    }

    #[tokio::test]
    async fn an_approval_shows_the_depositor_the_value_rdu_put_in_place_of_theirs() {
        // REQ-4.3 lets a reviewer edit before accepting and REQ-4.4 waives the
        // second approver, so a substituted value is seen by nobody unless the
        // depositor is shown it here.
        let (state, _) = test_state("section-substitution").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_round_with(
            &state,
            user.id,
            ReviewOutcome::Approved,
            None,
            Some(r#"{"name":{"decision":"accept","value":"The title RDU wrote"}}"#),
        )
        .await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;

        assert!(body.contains("Values RDU changed before deciding"), "{body}");
        assert!(body.contains("The title RDU wrote"), "{body}");
        // Named by its label, not its member name: the depositor knows the
        // field by what the form calls it.
        assert!(body.contains("<strong>Name</strong>"), "{body}");
    }

    #[tokio::test]
    async fn a_field_accepted_as_submitted_is_not_listed_as_changed_by_rdu() {
        // Accepting the depositor's own value is not a substitution, and
        // listing it would tell them RDU had rewritten something it had not.
        let (state, _) = test_state("section-no-substitution").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_round_with(
            &state,
            user.id,
            ReviewOutcome::Approved,
            None,
            Some(r#"{"name":{"decision":"accept"}}"#),
        )
        .await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!body.contains("Values RDU changed before deciding"), "{body}");
    }

    #[tokio::test]
    async fn a_returned_draft_is_distinguishable_from_one_never_submitted() {
        // REQ-2.1 fixes the state list at five and has no "returned", so the
        // difference has to be the round rather than a sixth state.
        let (state, _) = test_state("section-returned-vs-fresh").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // Never submitted: a plain editable form and nothing about a review.
        let fresh = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!fresh.contains("RDU asked for changes"), "{fresh}");

        a_round(
            &state,
            user.id,
            ReviewOutcome::ChangesRequested,
            "Please add a German description.",
        )
        .await;

        let returned = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(returned.contains("RDU asked for changes"), "{returned}");
        assert!(returned.contains("This project is a draft again"), "{returned}");
    }

    /// A saved draft that differs from the published project, so a submit has
    /// something to send. `name` is the field the overview form owns and every
    /// committed project has, which keeps the change one field wide.
    async fn a_changed_draft(state: &AppState, app: &axum::Router, session: &str) {
        let saved = as_session(app, post(OVERVIEW, "name=A+New+Title"), session).await;
        assert_eq!(saved.status(), StatusCode::SEE_OTHER, "the fixture's save should be accepted");
        let stored = DraftRepository::find(&*state.db, "0801d").await.unwrap();
        assert!(stored.is_some(), "the fixture should leave a draft");
    }

    #[tokio::test]
    async fn submitting_a_draft_creates_the_pending_submission() {
        // REQ-1.12. The submission carries the draft as it stands, and the
        // plain path redirects so a refresh cannot re-post it.
        let (state, _) = test_state("section-submit").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some(OVERVIEW));

        let submission = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("a pending submission");
        assert_eq!(submission.state, SubmissionState::Submitted);
        assert_eq!(submission.submitted_by, Some(user.id));
        assert!(submission.payload.contains("A New Title"), "{}", submission.payload);
        assert_eq!(submission.review_state, None);
        assert_eq!(submission.reviewed_by, None);
    }

    #[tokio::test]
    async fn submitting_records_no_review_round() {
        // A round is written by what *ends* one. A submission that recorded one
        // would put an outcome on the depositor's form describing a review
        // nobody has done.
        let (state, _) = test_state("section-submit-no-round").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn submitting_leaves_the_draft_in_place() {
        // REQ-1.13, and what makes request-changes and withdraw work: the
        // depositor comes back to the draft, so submit must not consume it.
        let (state, _) = test_state("section-submit-keeps-draft").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        let draft = DraftRepository::find(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the draft survives the submission");
        assert!(draft.payload.contains("A New Title"), "{}", draft.payload);
    }

    #[tokio::test]
    async fn submitting_an_unresolvable_period_is_refused_with_a_field_error() {
        // REQ-1.14 and Success Criterion 2, applied through the same function
        // `dpe-server validate` and `dpe-api-oai` use. Re-run on every submit,
        // which is what makes a resubmission revalidated rather than trusted
        // because it was reviewed once.
        let (state, _) = test_state("section-submit-invalid").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // Written straight to the draft: `temporalCoverage` has no control yet,
        // so there is no body that could carry it.
        let mut draft = ProjectDraft::from_raw(state.published.get("0801d").expect("0801d"));
        draft.set("temporalCoverage", json!([{ "en": "A period nobody has enriched" }]));
        DraftRepository::upsert(
            &*state.db,
            &DraftRecord {
                shortcode: "0801d".to_string(),
                payload: serde_json::to_string(&draft).unwrap(),
                updated_by: Some(user.id),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        )
        .await
        .unwrap();

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a refusal re-renders rather than redirecting"
        );
        let body = body_string(response).await;
        assert!(
            body.contains("A period nobody has enriched"),
            "the offending value is named: {body}"
        );
        assert!(body.contains("ChronOntology"), "the escape route is named: {body}");
        // The form is sectioned and validation is whole-project, so the field
        // at fault is routinely not on the page the depositor submitted from.
        // Named per field only, the message would be nowhere at all and the
        // refusal would read as a dead end.
        assert!(
            body.contains("/projects/0801d/sections/dataset"),
            "the section holding the field is linked: {body}"
        );

        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
        let draft = DraftRepository::find(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the draft survives");
        assert!(
            draft.payload.contains("A New Title"),
            "the refused post was still saved: {}",
            draft.payload
        );
    }

    #[tokio::test]
    async fn submitting_a_project_identical_to_what_is_published_is_refused() {
        // Allowed through, this locks the depositor's own form on a submission
        // a reviewer can only clear by rejecting it. The comparison is the one
        // the review surface renders from, so "changes nothing" means the same
        // thing in both places.
        let (state, _) = test_state("section-submit-unchanged").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let published = state.published.get("0801d").expect("0801d").name.clone();
        let body = format!("name={}&intent=submit", urlencoding(&published));
        let response = as_session(&app, post(OVERVIEW, &body), &session).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body_string(response).await.contains("nothing to submit"),
            "the reason is stated"
        );
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn submitting_while_a_submission_is_pending_is_refused() {
        // The form renders read-only, but the render is a `GET` and nothing
        // stops a `POST` arriving without one — the Back button onto a stale
        // form is the ordinary way in.
        let (state, _) = test_state("section-submit-locked").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_submission(&state, "0801d", user.id, SubmissionState::InReview).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("in review"), "the reason is stated");
        // Still the one that was already there, not a second one and not a
        // replacement.
        let submission = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the pending submission");
        assert_eq!(submission.payload, "{}");
    }

    #[tokio::test]
    async fn a_submit_that_loses_the_race_is_reported_rather_than_replacing_the_first() {
        // The unique index on `shortcode` is what enforces "one pending
        // submission per project". Reaching it means a submission arrived
        // between the read that rendered the form and this write, which is a
        // refusal to report — not a 500, and not a silent replacement.
        let db = Arc::new(open_test_db("section-submit-raced").await);
        let state = state_over(db.clone(), RecordingMailer::new(), |_| {});
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        // Inserted behind the handler's back, after the form was rendered.
        let racing = Submission {
            id: Uuid::new_v4(),
            shortcode: "0801d".to_string(),
            payload: r#"{"name":"somebody else"}"#.to_string(),
            // `Approved` so the lock check above does not catch it first: this
            // test is about the write, not about the form being read-only.
            state: SubmissionState::Approved,
            submitted_by: Some(user.id),
            submitted_at: Utc::now(),
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        };
        SubmissionRepository::create(&*db, &racing).await.unwrap();

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        assert_eq!(response.status(), StatusCode::OK, "a refusal, not a 500");
        assert!(
            body_string(response).await.contains("already has a submission"),
            "the reason is stated"
        );
        let stored = SubmissionRepository::find_by_shortcode(&*db, "0801d").await.unwrap().unwrap();
        assert_eq!(stored.id, racing.id, "the first submission is untouched");
    }

    #[tokio::test]
    async fn a_submit_refused_by_storage_leaves_the_draft_saved() {
        // The draft is written before the submission and by a different call,
        // so a storage failure on the submission must not cost the depositor
        // their editing as well as their submission.
        let db = Arc::new(open_test_db("section-submit-storage").await);
        let sound = state_over(db.clone(), RecordingMailer::new(), |_| {});
        let user = a_user(&sound, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&sound, user.id).await;
        let faulty = state_over(
            Arc::new(FaultyDatabase::new(
                db.clone(),
                Faults { submission_create: true, ..Faults::default() },
            )),
            RecordingMailer::new(),
            |_| {},
        );
        let app = test_app(&faulty);

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body_string(response).await.contains("could not be recorded"),
            "the reason is stated"
        );
        let draft = DraftRepository::find(&*db, "0801d")
            .await
            .unwrap()
            .expect("the draft was still written");
        assert!(draft.payload.contains("A New Title"), "{}", draft.payload);
        assert_eq!(SubmissionRepository::find_by_shortcode(&*db, "0801d").await.unwrap(), None);
    }

    #[tokio::test]
    async fn an_rdu_member_submitting_produces_a_submission_of_the_same_shape() {
        // REQ-4.8. `may_reach` is already true for every project for an RDU
        // account, so the direct-editing half was there; this is the half that
        // makes the result reviewable. Identical in shape means it is
        // `Submitted` with no reviewer — an RDU submission is not
        // self-approving, which REQ-4.8 requires by asking for a *pending* one.
        let (state, _) = test_state("section-submit-rdu").await;
        let rdu = a_user(&state, "rdu@example.test", "An RDU Member", Role::Rdu, &[]).await;
        let session = a_session(&state, rdu.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "name=Edited+by+RDU&intent=submit"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        let submission = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("an RDU submission is a pending submission");
        assert_eq!(submission.state, SubmissionState::Submitted);
        assert_eq!(submission.submitted_by, Some(rdu.id));
        assert_eq!(submission.reviewed_by, None);
        assert_eq!(submission.reviewed_at, None);
    }

    #[tokio::test]
    async fn withdrawing_deletes_the_submission_and_records_the_round() {
        // REQ-4.7. The draft is left, so the depositor keeps editing and can
        // submit again (REQ-1.13).
        let (state, _) = test_state("section-withdraw").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;
        as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;

        let response = as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None
        );
        let rounds = ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d").await.unwrap();
        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].outcome, ReviewOutcome::Withdrawn);
        assert_eq!(rounds[0].actor, Some(user.id));
        assert!(DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn the_form_is_editable_again_after_a_withdrawal() {
        // The observable point of REQ-4.7: the depositor got their form back.
        // A withdrawal that left the lock in place would be indistinguishable
        // from one that failed.
        let (state, _) = test_state("section-withdraw-unlocks").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;
        as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;
        let locked = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(locked.contains("Submitted for review"), "the premise of this test");

        as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!body.contains("Submitted for review"), "{body}");
        assert!(body.contains("Submit for review"), "the submit control is back: {body}");
    }

    #[tokio::test]
    async fn withdrawing_asks_first() {
        // A withdrawal cannot be undone by the depositor: the submission's
        // place in the queue and whatever a reviewer recorded on it both go.
        let (state, _) = test_state("section-withdraw-confirm").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let submission = a_submission(&state, "0801d", user.id, SubmissionState::Submitted).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, post(OVERVIEW, "intent=withdraw-confirm"), &session).await).await;
        assert!(body.contains("Yes, take it back"), "{body}");

        assert_eq!(
            SubmissionRepository::find(&*state.db, submission).await.unwrap().map(|s| s.id),
            Some(submission),
            "asking must not withdraw"
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn declining_a_withdrawal_writes_nothing_and_says_nothing_was_refused() {
        // Backing out of an irreversible action has to be inert. As a button
        // with no intent it fell through to `save`, which this form refuses
        // while a submission is pending — so declining a withdrawal answered
        // "This project is in review ... Nothing was saved", the opposite of
        // the reassurance the control is for.
        let (state, _) = test_state("section-withdraw-decline").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let submission = a_submission(&state, "0801d", user.id, SubmissionState::Submitted).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, post(OVERVIEW, "intent=withdraw-confirm"), &session).await).await;
        assert!(body.contains("Yes, take it back"), "the premise: the prompt is showing");
        // An anchor, not a submit. Asserted on the element and not on the href,
        // which the form's own `action` already carries either way.
        assert!(body.contains(">Keep waiting</a>"), "{body}");
        assert!(!body.contains(">Keep waiting</button>"), "{body}");

        // What the button shape would have posted, and what it would have got:
        // the form is read-only while a submission is pending, so backing out
        // of a withdrawal answered a refusal.
        let as_a_button = body_string(as_session(&app, post(OVERVIEW, ""), &session).await).await;
        assert!(as_a_button.contains("Nothing was saved"), "{as_a_button}");

        assert!(
            SubmissionRepository::find(&*state.db, submission).await.unwrap().is_some(),
            "the submission is untouched throughout"
        );
    }

    #[tokio::test]
    async fn withdrawing_a_submission_that_is_no_longer_pending_is_reported() {
        // The terminal-state guard, at the surface a depositor reaches it
        // through: RDU finished the review while this page was open.
        let (state, _) = test_state("section-withdraw-gone").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body_string(response).await.contains("no submission to take back"),
            "the reason is stated"
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn an_approved_submission_cannot_be_taken_back() {
        // The terminal-state rule from the depositor's side: an approved record
        // is on its way to a pull request and is no longer theirs to take back.
        // Allowed, this is the failure the issue names from the other
        // direction — the PR open, the editor record gone, and the project
        // silently back at its published state.
        let (state, _) = test_state("section-withdraw-approved").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let submission = a_submission(&state, "0801d", user.id, SubmissionState::Approved).await;
        let app = test_app(&state);

        // Not offered.
        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!body.contains("Take the submission back"), "{body}");

        // And refused when posted anyway.
        let response = as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("no submission to take back"));
        assert!(
            SubmissionRepository::find(&*state.db, submission).await.unwrap().is_some(),
            "the approved record survives"
        );
        assert!(ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_project_mate_may_withdraw_the_submission_somebody_else_made() {
        // REQ-4.7's "their own" reads as the project's, which is how the rest
        // of the service scopes access: one draft per project, and a
        // project-mate can already overwrite the draft the submission was made
        // from. Scoped to the submitter, a submission would be unwithdrawable
        // whenever that account is away — or removed, which nulls
        // `submitted_by`.
        let (state, _) = test_state("section-withdraw-mate").await;
        let author = a_user(&state, "a@example.test", "First Depositor", Role::Depositor, &["0801d"]).await;
        let mate = a_user(&state, "b@example.test", "Second Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, mate.id).await;
        a_submission(&state, "0801d", author.id, SubmissionState::Submitted).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        let rounds = ReviewRoundRepository::list_for_shortcode(&*state.db, "0801d").await.unwrap();
        assert_eq!(
            rounds[0].actor,
            Some(mate.id),
            "the round names who withdrew, not who submitted"
        );
    }

    #[tokio::test]
    async fn a_depositor_cannot_withdraw_a_submission_on_a_project_they_are_not_assigned() {
        // The project-scoped rule is `may_reach`, which `context` applies
        // before anything reads state — so this is a 403 rather than a refusal
        // message, like every other write here.
        let (state, _) = test_state("section-withdraw-unassigned").await;
        let owner = a_user(&state, "a@example.test", "Assigned", Role::Depositor, &["0801d"]).await;
        let outsider = a_user(&state, "b@example.test", "Unassigned", Role::Depositor, &["0803"]).await;
        let session = a_session(&state, outsider.id).await;
        a_submission(&state, "0801d", owner.id, SubmissionState::Submitted).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "intent=withdraw"), &session).await;

        assert!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
                .await
                .unwrap()
                .is_some(),
            "the submission survives"
        );
    }

    /// Return the project to the depositor with `field` accepted, as
    /// request-changes will.
    async fn returned_with_accepted(state: &AppState, user: Uuid, field: &str) {
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: "0801d".to_string(),
            payload: "{}".to_string(),
            state: SubmissionState::InReview,
            submitted_by: Some(user),
            submitted_at: Utc::now(),
            reviewed_by: Some(user),
            reviewed_at: Some(Utc::now()),
            reviewer_note: None,
            review_state: Some(format!(r#"{{"{field}":{{"decision":"accept"}}}}"#)),
        };
        SubmissionRepository::create(&*state.db, &submission).await.unwrap();
        let round = ReviewRound {
            id: Uuid::new_v4(),
            shortcode: submission.shortcode.clone(),
            submission_id: submission.id,
            outcome: ReviewOutcome::ChangesRequested,
            note: Some("Please add a German description.".to_string()),
            review_state: submission.review_state.clone(),
            actor: Some(user),
            at: Utc::now(),
        };
        let applied = ReviewRoundRepository::request_changes(
            &*state.db,
            submission.id,
            &DraftRecord {
                shortcode: submission.shortcode.clone(),
                payload: serde_json::to_string(&ProjectDraft::from_raw(state.published.get("0801d").expect("0801d")))
                    .unwrap(),
                updated_by: Some(user),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            &round,
        )
        .await
        .unwrap();
        assert_eq!(applied, Transition::Applied, "the fixture's return should be recorded");
    }

    #[tokio::test]
    async fn an_accepted_field_is_not_written_by_a_save() {
        // REQ-4.5 retains the per-field state, and nothing stopped the
        // depositor altering an accepted field — which then re-entered review
        // still flagged accepted. The gate is this applier skip: not rendering
        // the control stops an ordinary browser, and only this stops a
        // hand-built body.
        let (state, _) = test_state("section-accepted-lock").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        returned_with_accepted(&state, user.id, "name").await;
        let app = test_app(&state);
        let published = state.published.get("0801d").expect("0801d").name.clone();

        // `officialName` is not accepted, so the same post proves the skip is
        // per field rather than a whole-form lock in disguise.
        as_session(
            &app,
            post(OVERVIEW, "name=Altered+behind+RDU&officialName=A+new+official+name"),
            &session,
        )
        .await;

        let draft: ProjectDraft = serde_json::from_str(
            &DraftRepository::find(&*state.db, "0801d")
                .await
                .unwrap()
                .expect("a draft")
                .payload,
        )
        .unwrap();
        assert_eq!(
            draft.get("name").and_then(Value::as_str),
            Some(published.as_str()),
            "the accepted field is unchanged"
        );
        assert_eq!(
            draft.get("officialName").and_then(Value::as_str),
            Some("A new official name"),
            "an undecided field is still writable"
        );
    }

    #[tokio::test]
    async fn an_accepted_multilingual_field_cannot_be_written_through_its_language_keys() {
        // The plausible bypass: a multilingual field posts under `description.de`,
        // not under `description`, so a skip keyed on the field id looks like it
        // would miss it. It does not — `apply` derives every body key it reads
        // from the `field` argument, and the skip means `apply` is never called
        // for that field at all. Worth pinning rather than reasoning about,
        // because the two names genuinely differ.
        let (state, _) = test_state("section-accepted-multilingual").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        returned_with_accepted(&state, user.id, "description").await;
        let app = test_app(&state);
        let published = ProjectDraft::from_raw(state.published.get("0801d").expect("0801d"));

        as_session(
            &app,
            post(OVERVIEW, "description.de=Altered+behind+RDU&description.en=And+this+too"),
            &session,
        )
        .await;

        let draft: ProjectDraft = serde_json::from_str(
            &DraftRepository::find(&*state.db, "0801d")
                .await
                .unwrap()
                .expect("a draft")
                .payload,
        )
        .unwrap();
        assert_eq!(
            draft.get("description"),
            published.get("description"),
            "no language key of an accepted field was written"
        );
    }

    #[tokio::test]
    async fn an_accepted_field_renders_as_a_value_with_its_reason() {
        // A whole-form lock and an accepted field are both read-only and lift
        // on different events, so the reader has to be told which applies.
        let (state, _) = test_state("section-accepted-render").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        returned_with_accepted(&state, user.id, "name").await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;

        assert!(body.contains("RDU accepted this value"), "{body}");
        assert!(!body.contains(r#"name="name""#), "the accepted field posts nothing: {body}");
        assert!(
            body.contains(r#"name="officialName""#),
            "an undecided field still has a control"
        );
    }

    #[tokio::test]
    async fn the_lock_applies_only_while_the_latest_round_asked_for_changes() {
        // A rejected or withdrawn round's decisions are moot: the submission
        // they were recorded against is gone, so nothing is being answered and
        // locking a field would fix a value nobody accepted for this round.
        let (state, _) = test_state("section-accepted-only-returned").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        returned_with_accepted(&state, user.id, "name").await;
        // A later round that discards, on top of the one that returned.
        let submission = a_submission(&state, "0801d", user.id, SubmissionState::InReview).await;
        ReviewRoundRepository::discard(
            &*state.db,
            submission,
            &ReviewRound {
                id: Uuid::new_v4(),
                shortcode: "0801d".to_string(),
                submission_id: submission,
                outcome: ReviewOutcome::Rejected,
                note: Some("Not this time.".to_string()),
                // Accepting `name` here too: with an empty snapshot the
                // assertion below would hold whatever the outcome filter did,
                // so the test would pass against no filter at all.
                review_state: Some(r#"{"name":{"decision":"accept"}}"#.to_string()),
                actor: Some(user.id),
                at: Utc::now() + chrono::Duration::seconds(1),
            },
        )
        .await
        .unwrap();
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=Editable+again"), &session).await;

        let draft: ProjectDraft = serde_json::from_str(
            &DraftRepository::find(&*state.db, "0801d")
                .await
                .unwrap()
                .expect("a draft")
                .payload,
        )
        .unwrap();
        assert_eq!(draft.get("name").and_then(Value::as_str), Some("Editable again"));
    }

    #[tokio::test]
    async fn an_unknown_intent_saves_rather_than_submitting_or_withdrawing() {
        // Submit and withdraw are not undoable by the depositor and a save is,
        // so a body naming a verb this build does not know has to take the
        // recoverable branch.
        let (state, _) = test_state("section-unknown-intent").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=approve"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
        let draft = DraftRepository::find(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the draft was saved");
        assert!(draft.payload.contains("A New Title"), "{}", draft.payload);
    }

    /// Put a pending submission in front of the project.
    async fn a_submission(state: &AppState, shortcode: &str, user: Uuid, submission_state: SubmissionState) -> Uuid {
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: "{}".to_string(),
            state: submission_state,
            submitted_by: Some(user),
            submitted_at: Utc::now(),
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        };
        SubmissionRepository::create(&*state.db, &submission)
            .await
            .expect("the submission should store");
        submission.id
    }

    #[tokio::test]
    async fn a_project_in_review_is_read_only_and_a_save_against_it_is_refused() {
        for submission_state in [SubmissionState::Submitted, SubmissionState::InReview] {
            let label = format!("section-locked-{submission_state}");
            let db = std::sync::Arc::new(open_test_db(&label).await);
            let state = state_over(db.clone(), RecordingMailer::new(), |auth| {
                auth.cooldown = std::time::Duration::ZERO;
            });
            let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
            let session = a_session(&state, user.id).await;
            a_submission(&state, "0801d", user.id, submission_state).await;
            let app = test_app(&state);

            let read = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
            assert!(!read.contains("Save draft"), "{submission_state}: {read}");
            assert!(!read.contains(r#"name="name""#), "{submission_state}: {read}");

            // Re-checked at the write, not only at the render: the render is a
            // `GET`, so nothing stops a `POST` arriving without one — or
            // arriving after a reviewer picked the project up in the meantime.
            let refused = as_session(&app, post(OVERVIEW, "name=Sneaked+In"), &session).await;
            assert_eq!(refused.status(), StatusCode::OK, "{submission_state}");
            let body = body_string(refused).await;
            assert!(body.contains("cannot be changed"), "{submission_state}: {body}");
            assert_eq!(count_rows(&db, "drafts").await, 0, "{submission_state}: nothing may be written");
        }
    }

    #[tokio::test]
    async fn an_approved_submission_does_not_lock_the_form() {
        // An approved record is waiting to be collected into a pull request and
        // is no longer the depositor's to wait on, so editing again starts the
        // next cycle rather than disturbing a review in progress.
        let (state, _) = test_state("section-approved").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_submission(&state, "0801d", user.id, SubmissionState::Approved).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(body.contains("Save draft"), "{body}");
        let saved = as_session(&app, post(OVERVIEW, "name=Next+Cycle"), &session).await;
        assert_eq!(saved.status(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn an_unpublished_project_opens_blank_without_reading_as_an_error() {
        // REQ-2.3: absent from the published set is not "does not exist", and
        // REQ-1.1's "current published metadata" is then empty.
        let (state, _) = test_state("section-unpublished").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["9999"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, get("/projects/9999/sections/overview"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("nothing to pre-fill"), "{body}");
        assert!(body.contains("Save draft"), "{body}");
    }

    #[tokio::test]
    async fn a_save_with_no_sec_fetch_site_never_reaches_the_handler() {
        // The CSRF control is the outermost layer, and every write in this
        // service depends on it. Asserted here because a new write route is
        // exactly where the assumption would go untested.
        let (state, _) = test_state("section-csrf").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let mut request = Request::builder()
            .method("POST")
            .uri(OVERVIEW)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("name=Cross+Site"))
            .expect("the request should build");
        request.headers_mut().append(
            axum::http::header::COOKIE,
            format!("{}={session}", crate::auth::cookie::SESSION).parse().expect("a cookie"),
        );
        let response = app.clone().oneshot(request).await.expect("completes");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// The projects the end-to-end round-trip drives, and the trap each carries.
    ///
    /// Chosen rather than sampled: a project with no trap would let this pass on
    /// the strength of the plumbing alone. The whole-corpus version of the same
    /// check is `editor-web`'s `untouched_form_round_trip`; what these two add
    /// is the handler layer, and what the handler layer can break is specific to
    /// the values a project holds.
    const ROUND_TRIP_PROJECTS: &[(&str, &str)] = &[
        // `shortDescription` ends in a space, and `endDate` is the `MISSING`
        // sentinel — the trimming and placeholder traps.
        ("0816", "0816_vitrocentre.json"),
        // `description.ar` begins with a newline, which the HTML parser eats
        // after a `<textarea>` start tag. The tile compensates; this is the only
        // check that the compensation survives the section handler and the
        // `payload` column rather than only the tile's own unit test.
        ("0820", "0820_lhtt.json"),
    ];

    #[tokio::test]
    async fn saving_a_section_nobody_edited_leaves_the_committed_file_byte_identical() {
        // The end-to-end form of `editor-web`'s `untouched_form_round_trip`,
        // through the real routes: that test builds the body a control would
        // post, and this one drives every section of a real project with the
        // values the rendered form actually carries, then writes the stored
        // draft back through the canonical writer.
        //
        // What it adds over the unit test is the handler layer — the section
        // scoping, the draft that starts as the published project, and the
        // storage round-trip through the `payload` column. A bug in any of those
        // rewrites a published file while every unit test stays green.
        let (state, _) = test_state("section-round-trip").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        for (shortcode, filename) in ROUND_TRIP_PROJECTS {
            let published = state
                .published
                .get(shortcode)
                .unwrap_or_else(|| panic!("{shortcode} is committed"));
            let committed = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../dpe/server/data/projects")
                    .join(filename),
            )
            .unwrap_or_else(|error| panic!("reading {filename}: {error}"));

            // Every section, as a reader who opened each one and pressed save.
            for section in registry::sections_for(Audience::RduOnly) {
                let body = untouched_body(&ProjectDraft::from_raw(published), section);
                let uri = format!("/projects/{shortcode}/sections/{}", section.id);
                let response = as_session(&app, post(&uri, &body), &session).await;
                assert_eq!(response.status(), StatusCode::SEE_OTHER, "{shortcode} {}", section.id);
            }

            let row = DraftRepository::find(&*state.db, &normalize_shortcode(shortcode))
                .await
                .expect("read")
                .unwrap_or_else(|| panic!("{shortcode} should have a draft row"));
            let stored: ProjectDraft = serde_json::from_str(&row.payload).expect("a stored payload parses");
            let written = write_draft(&stored).expect("the draft should write");
            assert_eq!(written, committed, "saving every untouched section rewrote {filename}");
        }
    }

    #[tokio::test]
    async fn the_round_trip_projects_still_carry_the_traps_they_were_chosen_for() {
        // A positive canary for the test above, which asserts an *absence* of
        // change: over a corpus with the traps edited out it would pass while
        // proving nothing, and nobody could tell. Named per trap so a data
        // change says which project to replace.
        let (state, _) = test_state("section-round-trip-canary").await;
        let vitrocentre = state.published.get("0816").expect("0816 is committed");
        assert!(
            vitrocentre.short_description.ends_with(' '),
            "0816 was chosen for a trailing space in shortDescription"
        );
        assert!(
            platform_metadata::is_placeholder(&vitrocentre.end_date),
            "0816 was chosen for a MISSING endDate"
        );
        let lhtt = state.published.get("0820").expect("0820 is committed");
        assert!(
            lhtt.description.get("ar").is_some_and(|text| text.starts_with('\n')),
            "0820 was chosen for a description.ar beginning with a newline"
        );
    }

    /// The urlencoded body an untouched render of `section` would post.
    ///
    /// Built from the registry rather than hand-listed, so a field that gains a
    /// shape is carried here without this helper being edited — and a control
    /// holding a placeholder sentinel posts empty, which is the trap the
    /// round-trip above exists to catch.
    fn untouched_body(draft: &ProjectDraft, section: &Section) -> String {
        use editor_core::form::Shape;
        use editor_core::multilingual::UI_LANGUAGES;

        let mut pairs: Vec<(String, String)> = Vec::new();
        for field in section.fields_for(Audience::RduOnly) {
            match field.shape {
                Some(Shape::Text(_)) => {
                    let rendered = draft
                        .get(field.id)
                        .and_then(|value| value.as_str())
                        .filter(|text| !platform_metadata::is_placeholder(text))
                        .unwrap_or_default();
                    pairs.push((field.id.to_string(), rendered.to_string()));
                }
                Some(Shape::Multilingual) => {
                    let stored = draft.multilingual(field.id);
                    let extra: Vec<&str> = stored.extra_tags().collect();
                    for tag in UI_LANGUAGES.iter().copied().chain(extra) {
                        pairs.push((format!("{}.{tag}", field.id), stored.get(tag).unwrap_or_default().to_string()));
                    }
                }
                None => {}
            }
        }
        pairs
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}={}",
                    crate::test_support::urlencode(name),
                    crate::test_support::urlencode(value)
                )
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}
