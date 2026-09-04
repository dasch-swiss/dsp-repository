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
use chrono::Utc;
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody};
use editor_core::records::{normalize_shortcode, Submission, SubmissionState, User};
use editor_core::repository::{DraftRepository, RepositoryError, SubmissionRepository, UserRepository};
use editor_core::review::{diff, Decision, FieldDiff, FieldReview, ReviewState};
use editor_web::form::registry;
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
    let intent = body.get(page::INTENT).unwrap_or(page::SAVE);
    span.record("review.intent", tracing::field::display(intent));

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
    let title = match context.project_name {
        Some(name) => format!("Review {name} — DaSCH Metadata Editor"),
        None => format!("Review project {} — DaSCH Metadata Editor", context.shortcode),
    };
    crate::render(state, &title, StatusCode::OK, Some(user), page::page(&view))
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
    use editor_core::records::{DraftRecord, Role};
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
                reviewer_note: None,
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
