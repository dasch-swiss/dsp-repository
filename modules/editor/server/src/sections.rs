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
//!   never applied, so its stored value rides through untouched.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, Utc};
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody, Shape};
use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
use editor_core::records::{
    normalize_shortcode, DraftRecord, ReviewOutcome, ReviewRound, Submission, SubmissionState, User,
};
use editor_core::repository::{
    DraftRepository, EntityProposalRepository, RepositoryError, ReviewRoundRepository, SubmissionRepository,
    Transition, UserRepository,
};
use editor_core::review::{diff, FieldDiff, ReviewState};
use editor_core::submission::unresolved_temporal_coverage;
use editor_web::form::obligation::unsatisfied_required;
use editor_web::form::registry::{self, Audience, Section};
use editor_web::form::submit::{over_cap, typed_sentinels, unresolved_agents};
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
/// The refusal an unanswered required field renders as.
///
/// Distinct from [`SUBMIT_REFUSED_INCOMPLETE`], which answers a draft that is
/// not a `ProjectRaw` at all and can therefore name no field: this one always
/// carries per-field errors, so it promises them.
const SUBMIT_REFUSED_UNANSWERED: &str = "This project cannot be submitted yet: some required fields have no value. \
                                         Your draft has been saved, and each one is named below with a link to it.";
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
/// resolves, which is the escape route the decision to refuse rests on —
/// without naming it, a depositor whose period the table does not know is
/// simply stuck.
/// The refusal a body over the per-field cap renders as.
///
/// A write refusal rather than a submit one: applied to a save too, because an
/// over-cap save would otherwise truncate, store, and leave the submit that
/// follows looking at a draft already within the cap.
const WRITE_REFUSED_OVER_CAP: &str = "One field carried more values than this form accepts, so nothing was saved. \
                                      Each one is named below.";

/// The field-level error a field over the cap renders as.
fn over_cap_message(label: &str) -> String {
    format!(
        "\"{label}\" carried more than {} values. Nothing was saved for this field — remove some and save again.",
        editor_core::form::MAX_VALUES_PER_FIELD
    )
}

const DISCARD_REFUSED_GONE: &str = "There is no draft to discard — this form is already showing the project's \
                                    published metadata.";
const DISCARD_REFUSED_LOCKED: &str = "This project is in review, so its draft cannot be discarded. Take the \
                                      submission back first if you want to start over.";
const DISCARD_REFUSED_STORAGE: &str = "The draft could not be discarded, so it is still there. Try again, and if \
                                       it keeps happening the service needs attention.";

/// The refusal an unresolvable agent reference renders as.
const SUBMIT_REFUSED_UNKNOWN_AGENT: &str = "This project cannot be submitted yet: it refers to a person or \
                                            organisation the repository does not have. Your draft has been saved, \
                                            and each one is named below with a link to it.";

/// The field-level error an unresolvable agent reference renders as.
fn unknown_agent_message(id: &str) -> String {
    format!(
        "\"{id}\" is not a person or organisation the repository knows. Pick one from the suggestions, or ask RDU \
         to add it."
    )
}

/// The refusal a typed placeholder sentinel renders as.
const SUBMIT_REFUSED_SENTINEL: &str = "This project cannot be submitted yet: a field holds a word the repository \
                                       reserves for \"no value yet\". Your draft has been saved, and each one is \
                                       named below with a link to it.";

/// The refusal an unfinished entity proposal of this project's own renders as.
const SUBMIT_REFUSED_PROPOSAL_INCOMPLETE: &str = "This project cannot be submitted yet: a person or organisation \
                                                   you started has not been finished. Your draft has been saved, \
                                                   and each one is named below with a link to it.";

/// The refusal `PROPOSE_CHANGES` renders as when the posted entity is missing or names nobody.
const PROPOSE_REFUSED_UNKNOWN_ENTITY: &str = "No person or organisation was picked, or the one picked is not one \
                                              the repository knows, so nothing was started.";

/// The refusal a `Conflict` from [`EntityProposalRepository::create_new`] renders as: two proposals
/// of the same kind computed the same next id at once, which is rare but not impossible.
const PROPOSE_REFUSED_RACED: &str = "Something else claimed the same id at the same moment. Nothing was started \
                                     — try again.";

/// The refusal a `Conflict` from [`EntityProposalRepository::create_change`] renders as: an
/// ordinary double-click, since this project already holds a live proposal for that entity.
const PROPOSE_REFUSED_ALREADY_LIVE: &str = "This project already has a change proposed for that person or \
                                            organisation. Nothing new was started — open the existing proposal \
                                            instead.";

/// The refusal a storage failure renders as for any of the three propose intents.
const PROPOSE_REFUSED_STORAGE: &str = "The proposal could not be recorded. Nothing was started — try again, and \
                                       if it keeps happening the service needs attention.";

/// The field-level error a typed placeholder sentinel renders as.
///
/// Names the words, because the control shows the field as *empty* once one is
/// stored: told only that the value is wrong, a depositor looks at a blank box.
/// Says what to do with it too — the way out is a real value or a genuine
/// clear, and for an optional field those are different acts.
const SENTINEL_MESSAGE: &str = "\"MISSING\" and \"CALCULATED\" are reserved here for a value that is not filled in \
                                yet, so this field reads as empty to the rest of the platform. Enter the real value, \
                                or clear the field if there is nothing to record.";

/// The field-level error an unanswered required field renders as.
///
/// Names neither the field nor its section, because both renderings supply
/// them: in the reader's own section the message sits directly under the
/// field's label, and elsewhere `errors_elsewhere` prefixes it with the label
/// and a link to the section that holds it.
const UNANSWERED_MESSAGE: &str = "Required before this project can be submitted, and it has no value yet.";

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
    /// pre-filled, or empty for a project with neither.
    draft: ProjectDraft,
    /// The stored row, `None` when nothing has been saved over the published
    /// metadata yet. Its `created_at` is preserved across a save.
    record: Option<DraftRecord>,
    /// Set while a submission is awaiting or under review.
    locked: Option<page::Locked>,
    /// Whether an approved change is waiting for the release that carries it
    /// (REQ-2.5). Not a lock: approve is the only outcome that does not hand
    /// the project back, so the form stays editable.
    awaiting_release: bool,
    /// The pending submission, when there is one. The same read `locked` comes
    /// from, kept rather than reduced to a flag: withdrawing needs its id, and
    /// a second lookup could see a different row.
    submission: Option<Submission>,
    /// Fields RDU accepted in the round being answered, which are therefore
    /// fixed until the project is submitted again.
    ///
    /// The applier skip reads this, not just the renderer: a control that does
    /// not render still posts nothing, but a hand-built body can name the field
    /// anyway, and the whole point is that an accepted value cannot re-enter
    /// review altered.
    accepted_fields: Vec<String>,
    /// When this reader's session ends, whichever of the two deadlines comes
    /// first.
    ///
    /// On the context because every rendering wants it and it costs nothing:
    /// the guard already read the session row it comes from.
    signed_out_at: DateTime<Utc>,
    /// Who last saved the draft, when that was somebody other than this reader.
    ///
    /// Resolved here rather than in the view, and only when it is somebody
    /// else: a draft this reader saved themselves is the ordinary case, so the
    /// name would be noise, and skipping it avoids a user lookup on every
    /// ordinary form render.
    last_editor: Option<String>,
    /// The agents an id field may refer to.
    ///
    /// On the context rather than reached through `state` in the view, because
    /// `view` is given the resolved request and not the whole application —
    /// which is what keeps a renderer from quietly acquiring a second source of
    /// truth about the project.
    agents: editor_core::agents::AgentScope<'a>,
    /// This project's own entity proposals, every status.
    ///
    /// Read once here rather than filtered at each use: [`Self::agents`] is built from it (only the
    /// referenceable ones contribute), the submit gate below walks the live ones, and the summary
    /// panel renders the live ones too — three readers of one list rather than three queries that
    /// could disagree about which proposals exist.
    proposals: Vec<EntityProposal>,
    /// The body that was posted, or `None` on a `GET`.
    ///
    /// Part of the resolved request rather than an argument threaded through
    /// the thirteen refusal call sites, and only repeatable fields read it.
    /// What they need is the one thing the draft cannot hold: a row a depositor
    /// added but has not filled in. Such a row is never stored — an empty row
    /// must not reach a published file — so it lives in the form, and the body
    /// is where a re-render finds it again.
    posted: Option<&'a FormBody>,
    /// The latest finished review round, or `None` for a project nobody has
    /// reviewed.
    ///
    /// From `review_rounds` rather than from the draft, because the note has to
    /// be read beside the outcome it belongs to and only the round carries
    /// both. A save therefore cannot disturb it, which is what the column on
    /// `drafts` needed a rule to guarantee.
    ///
    /// Every outcome shows, not only the one that asked for changes: a
    /// rejection is otherwise invisible (a rejection discards the submission and notifications are
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
    signed_out_at: DateTime<Utc>,
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
    // Only when somebody else saved it: two members of one project team is the
    // normal case, and this is what turns a silent overwrite into a named one.
    // A draft this reader saved themselves needs no name, which also keeps this
    // lookup off every ordinary render. An account since removed leaves the row
    // with a null author, and reads as unknown rather than dangling.
    let last_editor = match record.as_ref().map(|record| record.updated_by) {
        Some(Some(editor)) if editor != user.id => match UserRepository::find_by_id(&*state.db, editor).await {
            Ok(found) => found.map(|found| found.name),
            // Not worth refusing the whole page over: the name is a courtesy
            // beside the timestamp, and the timestamp is what says the draft
            // moved.
            Err(error) => {
                tracing::warn!(error = %error, "could not read the draft's last editor");
                None
            }
        },
        _ => None,
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

    // REQ-2.5. An approved record that the published set does not yet carry is
    // waiting for a release; one it does carry is already Online, and the
    // startup pass will discard it. Read here rather than derived from `locked`
    // because an approval leaves no submission row to read it from.
    let awaiting_release = match crate::projects::approved_comparison(state, &key).await {
        // A record the published set already carries is Online, not waiting —
        // the startup pass will discard it.
        Ok((held, comparison)) => held && !comparison.permits_online(),
        Err(error) => return Err(storage_error(state, user, "read this project's approved records", &error)),
    };

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

    // Every status, not only the live ones: `AgentScope::with_proposals` also resolves an
    // `Accepted` proposal, which the referential-integrity gate needs, and the submit gate
    // below needs `is_live` per row to decide which to check.
    let proposals = match EntityProposalRepository::list_for_shortcode(&*state.db, &key).await {
        Ok(proposals) => proposals,
        Err(error) => return Err(storage_error(state, user, "read this project's entity proposals", &error)),
    };

    let published = state.published.get(shortcode);
    let draft = match &record {
        // A stored draft supersedes the published metadata: the form pre-fills from what is
        // published, and what was saved wins over it.
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
        awaiting_release,
        submission,
        accepted_fields,
        round,
        last_editor,
        signed_out_at,
        // Built from `proposals` before it moves into the struct below: `with_proposals` copies out
        // the labels it needs into its own `Vec<Agent>`, so the scope does not borrow the vector and
        // the two can sit on `Context` side by side.
        agents: editor_core::agents::AgentScope::with_proposals(&state.agents, &proposals),
        proposals,
        posted: None,
        project_name: published.map(|project| project.name.as_str()),
    })
}

/// `GET /projects/{shortcode}/sections/{section}`.
pub(crate) async fn show(
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, section_id)): Path<(String, String)>,
) -> Response {
    let context = match context(&state, &user, &shortcode, &section_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    render_page(&state, &user, &shortcode, &context, Rendering::default())
}

/// `POST /projects/{shortcode}/sections/{section}` — save the draft,
/// submit it, or take a pending submission back.
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
    Authenticated(user, signed_out_at): Authenticated,
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

    // Declared before the context so it outlives it: the context borrows it, so
    // that every render reached from here can preserve the editing state
    // without the body being threaded through each refusal.
    let body = FormBody::from_pairs(pairs);

    let mut context = match context(&state, &user, &shortcode, &section_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => {
            span.record("form.outcome", "refused");
            return response;
        }
    };
    context.posted = Some(&body);
    // Anything this build does not know falls back to `save`: submit and
    // withdraw are not undoable by the depositor and a save is, so a body
    // naming an unknown verb must take the recoverable branch.
    let intent = match body.get(INTENT) {
        Some(page::SUBMIT) => Intent::Submit,
        Some(page::WITHDRAW) => Intent::Withdraw,
        Some(page::WITHDRAW_CONFIRM) => Intent::ConfirmWithdrawal,
        Some(page::DISCARD) => Intent::Discard,
        Some(page::DISCARD_CONFIRM) => Intent::ConfirmDiscard,
        Some(page::FIND_AGENT) => Intent::FindAgent,
        Some(page::PROPOSE_PERSON) => Intent::ProposePerson,
        Some(page::PROPOSE_ORGANIZATION) => Intent::ProposeOrganization,
        // The entity rides in the value, so this matches the prefix rather than the whole
        // string; `page::PROPOSE_CHANGES`'s docs say why it is not a hidden input.
        Some(value) if page::proposed_entity(value).is_some() => Intent::ProposeChanges,
        _ => Intent::Save,
    };
    span.record("form.intent", tracing::field::display(intent.as_str()));

    if intent == Intent::Withdraw || intent == Intent::ConfirmWithdrawal {
        return withdraw(&state, &user, &shortcode, &context, headers, intent).await;
    }

    // Before the lock check and before anything is applied: discarding is not a
    // write to the draft but the removal of it, so running the appliers first
    // would store a body only to delete it.
    if intent == Intent::Discard || intent == Intent::ConfirmDiscard {
        return discard(&state, &user, &shortcode, &context, headers, intent).await;
    }

    // Re-checked here and not only when the form was rendered: the render is a
    // `GET`, so nothing stops a `POST` arriving without one — or arriving after
    // a reviewer picked the project up in the meantime.
    if context.locked.is_some() {
        span.record("form.outcome", "locked");
        tracing::info!("refused a write against a project that is in review");
        return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_LOCKED);
    }

    // Did the draft move under this form? The draft is one row and `upsert` is last-write-wins, so
    // without this a save silently replaces work somebody else did while this form was open.
    // Refused **once**: the re-render carries what was typed and a refreshed baseline, so
    // saving again keeps this depositor's version.
    //
    // It closes the human-scale race — two people with the form open for
    // minutes — and not the instant between this read and the write below,
    // which needs a transaction rather than a baseline. Worth being exact
    // about, because the wider race is the one that actually loses work.
    if let Some(conflict) = changed_underneath(&context, &body) {
        span.record("form.outcome", "changed_underneath");
        tracing::info!("refused a write against a draft that changed underneath the form");
        // In memory only, and before rendering: a scalar control renders from
        // the draft, so without this the refusal would show the other person's
        // values under a notice saying the page still holds yours. Nothing is
        // stored, which is what makes "nothing has been saved just now" true.
        apply_posted(&mut context, &body);
        let rendering = Rendering {
            notice: Some(page::Notice::Changed { by: context.last_editor.as_deref(), at: &conflict }),
            keep_posted: true,
            ..Rendering::default()
        };
        return if is_enhanced(&headers) {
            region(&shortcode, &context, rendering)
        } else {
            render_page(&state, &user, &shortcode, &context, rendering)
        };
    }

    // Before any applier runs, and on a save as much as a submit: an applier
    // silently truncates at `MAX_VALUES_PER_FIELD`, so deferring this to
    // submit would let an over-cap save store the truncated value and leave
    // submit looking at a draft already within the cap. Nothing is written on
    // this branch, which is what makes the refusal honest about "nothing was
    // saved".
    let over = over_cap(context.audience, context.section, &body);
    if !over.is_empty() {
        span.record("form.outcome", "over_cap");
        tracing::info!(fields.over_cap = over.len(), "refused a write carrying too many values");
        let errors: Vec<(String, String)> = over
            .iter()
            .map(|field| (field.id.to_string(), over_cap_message(field.label)))
            .collect();
        return refused_with(&state, &user, &shortcode, &context, headers, WRITE_REFUSED_OVER_CAP, &errors);
    }

    let record = match apply_and_store(&state, &user, &shortcode, &mut context, &headers, &body).await {
        Ok(record) => record,
        Err(response) => return response,
    };

    if intent == Intent::Save {
        span.record("form.outcome", "saved");
        tracing::info!("saved a project draft");
        return saved(&shortcode, &context, headers);
    }

    // A search stores the body exactly as a save does — so nothing typed is at risk of being
    // lost to a lookup — and then re-renders it, which is the whole of what it does. It keeps the
    // posted body, unlike a save: the query lives only in the form, so a render that dropped it
    // would clear the box and the matches with it. `saved` redirects for the same reason it must
    // not here.
    if intent == Intent::FindAgent {
        span.record("form.outcome", "searched");
        let rendering = Rendering { keep_posted: true, ..Rendering::default() };
        return if is_enhanced(&headers) {
            region(&shortcode, &context, rendering)
        } else {
            render_page(&state, &user, &shortcode, &context, rendering)
        };
    }

    // Same footing as submit below: the draft above is already written, so a proposal refused past
    // this point costs the depositor only the proposal, never whatever they just typed.
    if matches!(
        intent,
        Intent::ProposePerson | Intent::ProposeOrganization | Intent::ProposeChanges
    ) {
        return propose(&state, &user, &shortcode, &section_id, &context, headers, &body, intent).await;
    }

    // The record's own timestamp rather than a second `Utc::now()`: the
    // submission and the draft write it came from must agree.
    let submitted_at = record.updated_at;
    submit(&state, &user, &shortcode, &context, headers, &record, submitted_at).await
}

/// How long before the session ends a form starts saying so.
///
/// Half an hour: long enough to finish a paragraph and save, short enough that
/// the warning is not permanently on screen being ignored.
const SIGN_OUT_WARNING: chrono::Duration = chrono::Duration::minutes(30);

/// The deadline, if it is close enough to be worth telling a reader about.
fn nearly_signed_out(signed_out_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
    (signed_out_at - Utc::now() <= SIGN_OUT_WARNING).then_some(signed_out_at)
}

/// When the stored draft is a different revision from the one this form was
/// rendered from, formatted for a reader.
///
/// `None` when they agree, when there is no stored draft, and when the body
/// carries no baseline at all. That last case is deliberate: a body with no
/// baseline was not posted from a form this service rendered, so there is no
/// revision it could have been looking at, and refusing it would protect
/// nothing. This is a courtesy between people rather than a control — the row
/// is still last-write-wins.
fn changed_underneath(context: &Context<'_>, body: &FormBody) -> Option<String> {
    let baseline = body.get(page::BASELINE)?;
    let record = context.record.as_ref()?;
    let stored = record.updated_at.to_rfc3339();
    (stored != baseline).then(|| crate::format_instant(record.updated_at))
}

/// Apply the posted body to the **in-memory** draft, returning how many fields
/// it touched.
///
/// Split from the store beside it because one path needs exactly this and not
/// the write: a save refused because the draft changed underneath has to
/// re-render what the depositor typed, and a scalar control renders
/// from the draft rather than from the body — so without applying first, the
/// refusal would show the *other* person's values under a notice claiming the
/// page still holds yours.
fn apply_posted(context: &mut Context<'_>, body: &FormBody) -> usize {
    let mut applied = 0;
    for field in context.section.fields_for(context.audience) {
        // An accepted field is skipped here, which is the gate. Not rendering
        // its control stops an ordinary browser posting it; only this stops a
        // hand-built body, and the point of retaining per-field state is that an accepted
        // value cannot re-enter review altered.
        if context.accepted_fields.iter().any(|accepted| accepted == field.id) {
            continue;
        }
        if let Some(shape) = field.shape {
            apply(shape, body, &mut context.draft, field.id);
            applied += 1;
        }
    }
    applied
}

/// Apply the posted body to the draft and write it, or the response that refuses
/// the write.
///
/// Shared by the save/submit handler and the row actions, which is what keeps a
/// row action from being a second, weaker write path: the accepted-field gate,
/// the `fields_for` audience gate, the `created_at` preservation and the
/// serialize/store refusals are all here, once.
///
/// The draft is written **before** submit validation runs, and on every intent.
/// A submit that stored only the submission would lose whatever was typed in the
/// same post if validation then refused it — and the draft is what the depositor
/// comes back to when RDU returns the project.
async fn apply_and_store(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &mut Context<'_>,
    headers: &HeaderMap,
    body: &FormBody,
) -> Result<DraftRecord, Response> {
    let span = tracing::Span::current();
    let applied = apply_posted(context, body);

    let now = Utc::now();
    let record = DraftRecord {
        shortcode: normalize_shortcode(shortcode),
        payload: match serde_json::to_string(&context.draft) {
            Ok(payload) => payload,
            Err(error) => {
                span.record("form.outcome", "serialize_failed");
                tracing::error!(error = %error, "a draft could not be serialized");
                return Err(refused(state, user, shortcode, context, headers.clone(), SAVE_REFUSED_STORAGE));
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

    if let Err(error) = DraftRepository::upsert(&*state.db, &record).await {
        span.record("form.outcome", "store_failed");
        tracing::error!(error = %error, "could not save a project draft");
        return Err(refused(state, user, shortcode, context, headers.clone(), SAVE_REFUSED_STORAGE));
    }
    context.record = Some(record.clone());
    tracing::debug!(fields.applied = applied, "applied a posted form body");
    Ok(record)
}

/// `POST /projects/{shortcode}/sections/{section}/fields/{field}/add` — one more
/// blank row.
///
/// The whole form body comes with it, because the tile's add control is a submit
/// button carrying a `formaction`. So nothing typed elsewhere is lost, and this
/// handler's job is only to save what arrived and re-render with one row more.
///
/// **The blank row is not stored.** `apply_multilingual_rows` drops a row with
/// no text in any language, and it must — a file full of empty objects is not
/// data. The extra row is a *rendering*, and it survives the next round trip
/// because the tile emits a hidden `{field}.row` for it, so the body carries
/// its key back. Nothing is held server-side between requests.
pub(crate) async fn add_row(
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, section_id, field_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(
        &state,
        &user,
        &shortcode,
        &section_id,
        &field_id,
        headers,
        pairs,
        None,
        signed_out_at,
    )
    .await
}

/// `POST /projects/{shortcode}/sections/{section}/fields/{field}/{key}/remove` —
/// drop one row.
///
/// The key comes from the URL rather than from a submit button's name and value,
/// which is the tile's own decision and the right one: a form submitted
/// programmatically does not include the submitter's name and value unless it is
/// passed explicitly, so a named button would work on the plain path and vanish
/// on the enhanced one.
pub(crate) async fn remove_row(
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, section_id, field_id, key)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(
        &state,
        &user,
        &shortcode,
        &section_id,
        &field_id,
        headers,
        pairs,
        Some(&key),
        signed_out_at,
    )
    .await
}

/// Add or remove a row: the two differ only in what they do to the posted body.
///
/// Removing drops the row's key from `{field}.row` **before** anything is
/// applied, so the applier simply does not see it and the stored list comes out
/// without it. That is the whole removal — there is no separate delete, which is
/// what keeps this in step with a save: one applier, one set of rules.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "project section row action",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.section = tracing::field::Empty,
        form.field = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
#[allow(clippy::too_many_arguments)]
async fn row_action(
    state: &AppState,
    user: &User,
    shortcode: &str,
    section_id: &str,
    field_id: &str,
    headers: HeaderMap,
    pairs: Vec<(String, String)>,
    removing: Option<&str>,
    signed_out_at: DateTime<Utc>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(shortcode));
    span.record("form.section", tracing::field::display(section_id));
    span.record("form.field", tracing::field::display(field_id));

    let body = FormBody::from_pairs(match removing {
        Some(key) => FormBody::pairs_without_row(pairs, field_id, key),
        None => pairs,
    });

    let mut context = match context(state, user, shortcode, section_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => {
            span.record("form.outcome", "refused");
            return response;
        }
    };
    context.posted = Some(&body);

    // The field has to be one this reader may write in this section, checked
    // through `fields_for` like every other write: without it the URL is a way
    // to name an RDU-only or a display-only field.
    //
    // Whether the field has rows comes from `Shape::has_rows`, which is exhaustive, and not from
    // a list of shapes written out here. The renderer emits an add control for every row shape,
    // so an allowlist that misses one serves a `404` to a button the page itself drew — which is
    // what four of the eight row shapes did.
    let Some(field) = context
        .section
        .fields_for(context.audience)
        .find(|field| field.id == field_id && field.shape.is_some_and(Shape::has_rows))
    else {
        span.record("form.outcome", "unknown_field");
        return crate::not_found(State(state.clone())).await;
    };

    if context.locked.is_some() {
        span.record("form.outcome", "locked");
        return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_LOCKED);
    }

    let over = over_cap(context.audience, context.section, &body);
    if !over.is_empty() {
        span.record("form.outcome", "over_cap");
        let errors: Vec<(String, String)> = over
            .iter()
            .map(|field| (field.id.to_string(), over_cap_message(field.label)))
            .collect();
        return refused_with(state, user, shortcode, &context, headers, WRITE_REFUSED_OVER_CAP, &errors);
    }

    if let Err(response) = apply_and_store(state, user, shortcode, &mut context, &headers, &body).await {
        return response;
    }

    span.record("form.outcome", if removing.is_some() { "row_removed" } else { "row_added" });
    let rendering = Rendering {
        adding_row: removing.is_none().then_some(field.id),
        keep_posted: true,
        ..Rendering::default()
    };
    if is_enhanced(&headers) {
        return region(shortcode, &context, rendering);
    }
    render_page(state, user, shortcode, &context, rendering)
}

/// What a `POST` to this route is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Save,
    Submit,
    Withdraw,
    ConfirmWithdrawal,
    Discard,
    ConfirmDiscard,
    /// Run the agent pickers' searches and re-render. Stores the posted body like a save, and
    /// changes nothing else.
    FindAgent,
    /// Start a proposal for a new person.
    ProposePerson,
    /// Start a proposal for a new organisation.
    ProposeOrganization,
    /// Propose a change to an entity this project already references.
    ProposeChanges,
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
            Self::Discard => "discard",
            Self::ConfirmDiscard => "discard-confirm",
            Self::FindAgent => "find-agent",
            Self::ProposePerson => "propose-person",
            Self::ProposeOrganization => "propose-organization",
            Self::ProposeChanges => "propose-changes",
        }
    }
}

/// Record the draft as the project's pending submission.
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

    // The obligation gate reads the draft, so it needs nothing from the contract conversion below
    // and must stay ahead of it: every way a required value can be missing — absent, `[]`,
    // `{}`, `""`, a placeholder sentinel — is reachable from the form, and the conversion can
    // only report the first of them, without naming a field. Presence is read through
    // `obligation::is_satisfied`, the same function the section rail counts with, so the gate
    // cannot refuse a submission the rail has just called complete.
    let unanswered = unsatisfied_required(context.audience, &context.draft);
    if !unanswered.is_empty() {
        span.record("form.outcome", "unanswered");
        tracing::info!(
            fields.unanswered = unanswered.len(),
            "refused a submission with unanswered required fields"
        );
        let errors: Vec<(String, String)> = unanswered
            .iter()
            .map(|field| (field.id.to_string(), UNANSWERED_MESSAGE.to_string()))
            .collect();
        return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_UNANSWERED, &errors);
    }

    // Do not move this ahead of the obligation gate. Emptying a required list to zero rows removes
    // the member — `keywords`, `attributions`, `disciplines`, `temporalCoverage` and
    // `spatialCoverage` are non-`Option` `Vec`s and the row appliers use absent as their empty
    // state — so the conversion fails first with a generic "missing field" and this branch
    // reports it naming no field at all. What is left for it is what no field-level rule can
    // explain: a member of the wrong JSON kind, which the form cannot produce.
    let raw = match context.draft.to_raw() {
        Ok(raw) => raw,
        Err(error) => {
            span.record("form.outcome", "incomplete");
            tracing::info!(error = %error, "refused a submission whose draft is not a complete project");
            return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_INCOMPLETE, &[]);
        }
    };

    // Every agent reference must resolve. The applier stores whatever arrives,
    // because a draft may hold a value that does not validate — this
    // is what stops an unresolvable id reaching a published file, where it
    // renders as a bare `person-001` on the public project page.
    let unknown = unresolved_agents(context.audience, &context.draft, &context.agents);
    if !unknown.is_empty() {
        span.record("form.outcome", "unknown_agent");
        tracing::info!(
            fields.unknown_agents = unknown.len(),
            "refused a submission with an unresolvable agent"
        );
        let errors: Vec<(String, String)> = unknown
            .iter()
            .map(|(field, id)| (field.id.to_string(), unknown_agent_message(id)))
            .collect();
        return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_UNKNOWN_AGENT, &errors);
    }

    // This project's own live proposals must satisfy the same rules the entity form enforces: a
    // `Draft`/`Submitted` proposal riding into review with an incomplete payload would let RDU
    // accept it into a broken committed file, with no field-level check left to stop
    // it once this project is approved.
    let mut proposal_findings: Vec<(String, ProposalKind, String)> = Vec::new();
    for proposal in context.proposals.iter().filter(|proposal| proposal.is_live()) {
        let payload: serde_json::Value = match serde_json::from_str(&proposal.payload) {
            Ok(payload) => payload,
            // Cannot happen through this form — every writer here stores what `serde_json::Value`
            // itself produced — but a stored payload this build cannot parse is a reason to refuse,
            // not to skip the check silently.
            Err(_) => {
                proposal_findings.push((
                    proposal.entity_id.clone(),
                    proposal.kind,
                    "its stored data could not be read".to_string(),
                ));
                continue;
            }
        };
        let findings = match proposal.kind {
            ProposalKind::Person => editor_core::proposals::check_person(&payload),
            // The published side is what makes the address carve-out work: an incomplete address
            // inherited unchanged from a published organisation is passed through, one the
            // depositor wrote is refused. `None` for a `New` proposal, which has nothing to
            // inherit — `published_body` answers only for the published store, so a proposal's own
            // payload can never grandfather itself.
            ProposalKind::Organization => {
                editor_core::proposals::check_organization(&payload, context.agents.published_body(&proposal.entity_id))
            }
        };
        for finding in findings {
            let field = match finding.index {
                Some(index) => format!("{}[{index}]", finding.field),
                None => finding.field.to_string(),
            };
            proposal_findings.push((
                proposal.entity_id.clone(),
                proposal.kind,
                format!("{field}: {}", finding.message),
            ));
        }
    }
    if !proposal_findings.is_empty() {
        span.record("form.outcome", "proposal_incomplete");
        tracing::info!(
            proposals.findings = proposal_findings.len(),
            "refused a submission with an unfinished entity proposal"
        );
        return refused_with_proposal_findings(state, user, shortcode, context, headers, &proposal_findings);
    }

    // A sentinel a depositor typed, which the shape is what recognises: a
    // sentinel is the *correct* stored value wherever clearing the field writes
    // one, so this refuses only the shapes where clearing means something else.
    // Left through, the field reads as empty, an empty submit will not clear it,
    // and the platform reads the stored word as no value at all.
    let sentinels = typed_sentinels(context.audience, &context.draft);
    if !sentinels.is_empty() {
        span.record("form.outcome", "sentinel");
        tracing::info!(
            fields.sentinel = sentinels.len(),
            "refused a submission holding a typed placeholder sentinel"
        );
        let errors: Vec<(String, String)> = sentinels
            .iter()
            .map(|field| (field.id.to_string(), SENTINEL_MESSAGE.to_string()))
            .collect();
        return refused_with(state, user, shortcode, context, headers, SUBMIT_REFUSED_SENTINEL, &errors);
    }

    // Every `temporalCoverage` entry has to resolve to a structured date, checked through the
    // same function `dpe-server validate` and `dpe-api-oai` apply, so the three cannot disagree
    // about what counts as a gap. Re-run on every submit, which is what makes a resubmission
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

/// Start an entity proposal, or refuse without creating one.
///
/// The draft has already been written by the time this runs — `act` saves the posted body before
/// dispatching on intent, on this path exactly as it does on submit — so a refusal here costs the
/// depositor only the proposal, never whatever they just typed.
#[allow(clippy::too_many_arguments)]
async fn propose(
    state: &AppState,
    user: &User,
    shortcode: &str,
    section_id: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    body: &FormBody,
    intent: Intent,
) -> Response {
    let span = tracing::Span::current();
    let now = Utc::now();

    let outcome = match intent {
        Intent::ProposePerson | Intent::ProposeOrganization => {
            let kind = if intent == Intent::ProposePerson {
                ProposalKind::Person
            } else {
                ProposalKind::Organization
            };
            let proposal = EntityProposal {
                id: Uuid::new_v4(),
                shortcode: normalize_shortcode(shortcode),
                // Filled in by `create_new`, inside the write transaction that allocates it — nothing
                // out here can compute it without racing every other proposal of this kind.
                entity_id: String::new(),
                kind,
                operation: ProposalOperation::New,
                payload: "{}".to_string(),
                status: ProposalStatus::Draft,
                proposed_by: Some(user.id),
                created_at: now,
                updated_at: now,
                decision: None,
                decided_by: None,
                decided_at: None,
            };
            let published_floor = state.agents.highest_id_number(kind);
            EntityProposalRepository::create_new(&*state.db, &proposal, published_floor)
                .await
                .map(|created| (created.entity_id, kind, ProposalOperation::New))
        }
        Intent::ProposeChanges => {
            // Both "no id was posted" and "the posted id resolves to nobody" are the same refusal:
            // either way there is nothing this project may propose a change to.
            //
            // The id comes from the intent value, which is the activated button's — so it names
            // the row that was clicked and no other. Read from a shared hidden input it named
            // whichever resolved row happened to render first.
            let resolved = body
                .get(INTENT)
                .and_then(page::proposed_entity)
                .and_then(|id| context.agents.get(id).map(|agent| (id, agent)));
            let Some((entity_id, agent)) = resolved else {
                span.record("form.outcome", "propose_unresolved_entity");
                tracing::info!("refused a change proposal naming no resolvable entity");
                return refused(state, user, shortcode, context, headers, PROPOSE_REFUSED_UNKNOWN_ENTITY);
            };
            let kind = match agent.kind {
                editor_core::agents::AgentKind::Person => ProposalKind::Person,
                editor_core::agents::AgentKind::Organization => ProposalKind::Organization,
            };
            // `published_body` and not "whatever this scope resolves": a proposal must be seeded
            // from the published entity, never from another proposal's payload. The agent resolved
            // above, and only the published store answers `seed_payload`, so a `None` here means
            // the id resolved through a proposal rather than a file — which `create_change` has
            // nothing to change.
            let Some(seed) = context.agents.seed_payload(entity_id) else {
                span.record("form.outcome", "unknown_entity");
                tracing::info!(proposal.entity_id = %entity_id, "refused a change to an unpublished entity");
                return refused(state, user, shortcode, context, headers, PROPOSE_REFUSED_UNKNOWN_ENTITY);
            };
            let proposal = EntityProposal {
                id: Uuid::new_v4(),
                shortcode: normalize_shortcode(shortcode),
                entity_id: entity_id.to_string(),
                kind,
                operation: ProposalOperation::Change,
                // Seeded with the whole published entity, `id` stripped. Not a nicety: accepting
                // this proposal writes its payload as the entity file, so a payload holding only
                // the members a form renders would silently drop `affiliations`, `sameAs`,
                // `email`, `alternativeName`, `canton` and `additional`. This is the property
                // `ProjectDraft` gives a project — carry every member the editor does not manage,
                // unchanged — and an entity needs it for the same reason.
                //
                // `seed_payload` is what removes `id`; see its docs and `EntityProposal::payload`
                // for why the payload must not carry one.
                payload: seed.to_string(),
                status: ProposalStatus::Draft,
                proposed_by: Some(user.id),
                created_at: now,
                updated_at: now,
                decision: None,
                decided_by: None,
                decided_at: None,
            };
            EntityProposalRepository::create_change(&*state.db, &proposal)
                .await
                .map(|()| (proposal.entity_id, kind, ProposalOperation::Change))
        }
        _ => unreachable!("propose is dispatched to only for the three propose intents"),
    };

    match outcome {
        Ok((entity_id, kind, operation)) => {
            span.record("form.outcome", "proposed");
            tracing::info!(proposal.entity_id = %entity_id, proposal.kind = %kind, "started an entity proposal");
            proposed(
                state, user, shortcode, section_id, context, headers, kind, operation, &entity_id,
            )
            .await
        }
        // `entity_proposals_allocated_id` (only reachable from `ProposePerson`/`ProposeOrganization`,
        // since it applies to `operation = 'new'` rows alone) is a genuine allocation race;
        // `entity_proposals_live_per_entity` (only reachable from `ProposeChanges`) is an ordinary
        // double-click. Both are refusals, not 500s.
        Err(RepositoryError::Conflict { .. }) => {
            span.record("form.outcome", "propose_conflict");
            let message = if intent == Intent::ProposeChanges {
                PROPOSE_REFUSED_ALREADY_LIVE
            } else {
                PROPOSE_REFUSED_RACED
            };
            tracing::info!("refused a proposal that raced an existing one");
            refused(state, user, shortcode, context, headers, message)
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not record an entity proposal");
            refused(state, user, shortcode, context, headers, PROPOSE_REFUSED_STORAGE)
        }
    }
}

/// A proposal was started: named with its kind and id, and linked to the entity form it will one
/// day have.
///
/// Neither path redirects, unlike a save or a phase change. Datastar processes a body only on a
/// 200, so a redirect the enhanced path followed would merge the entity form's whole page into the
/// section region; re-rendering answers both renderings identically, the same way a refusal does,
/// and the notice carries the link instead.
#[allow(clippy::too_many_arguments)]
async fn proposed(
    state: &AppState,
    user: &User,
    shortcode: &str,
    section_id: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    kind: ProposalKind,
    operation: ProposalOperation,
    entity_id: &str,
) -> Response {
    let notice = page::Notice::Proposed { kind, operation, entity_id };
    let signed_out_at = context.signed_out_at;
    // Re-resolved like `phase_changed`, so the picker and the proposals summary this same render
    // shows already carry what was just started — without it a depositor could not yet name the id
    // they just allocated anywhere else on the page they are looking at.
    match self::context(state, user, shortcode, section_id, signed_out_at).await {
        Ok(fresh) => {
            let rendering = Rendering { notice: Some(notice), ..Rendering::default() };
            if is_enhanced(&headers) {
                region(shortcode, &fresh, rendering)
            } else {
                render_page(state, user, shortcode, &fresh, rendering)
            }
        }
        // The write landed; only the re-read did not. Unlike `phase_changed`, this must never
        // redirect — for the reason this function's own docs give about the enhanced path — so the
        // fallback re-renders what is already in hand instead.
        Err(_) => {
            let rendering = Rendering { notice: Some(notice), ..Rendering::default() };
            if is_enhanced(&headers) {
                region(shortcode, context, rendering)
            } else {
                render_page(state, user, shortcode, context, rendering)
            }
        }
    }
}

/// Discard the draft, or show the confirmation that posts it.
///
/// The only thing in the service that removes a draft: a review that rejects
/// one and a depositor who withdraws a submission both **keep** it, so without
/// this an abandoned draft sits in RDU's list for good, and a project hand-edited
/// outside the editor keeps a stale draft shadowing it.
///
/// Removing the draft is not the same as removing the project. The published
/// metadata is untouched, and the form re-opens pre-filled from it —
/// which is why the control and the confirmation both say "discard" rather than
/// "delete".
async fn discard(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    intent: Intent,
) -> Response {
    let span = tracing::Span::current();

    // Nothing stored: the form is already showing published metadata, so there
    // is nothing to discard and the control is not offered. Only a hand-built
    // body or a stale page reaches this.
    if context.record.is_none() {
        span.record("form.outcome", "gone");
        tracing::info!("refused a discard of a project with no draft");
        return refused_with(state, user, shortcode, context, headers, DISCARD_REFUSED_GONE, &[]);
    }

    // Re-checked here as well as when the form was rendered, for the reason the
    // save path re-checks it: the render is a `GET`, so nothing stops a `POST`
    // arriving without one, or arriving after a reviewer picked the project up.
    // The draft is what the depositor comes back to when RDU returns the
    // project, so it must not vanish from under a live review.
    if context.locked.is_some() {
        span.record("form.outcome", "locked");
        tracing::info!("refused a discard against a project that is in review");
        return refused_with(state, user, shortcode, context, headers, DISCARD_REFUSED_LOCKED, &[]);
    }

    if intent == Intent::ConfirmDiscard {
        span.record("form.outcome", "confirming");
        return confirming(state, user, shortcode, context, headers, page::Confirmation::Discard);
    }

    match DraftRepository::delete(&*state.db, &normalize_shortcode(shortcode)).await {
        // `false` means it went between the read above and this write, which is
        // the outcome the depositor asked for either way.
        Ok(_) => {
            span.record("form.outcome", "discarded");
            tracing::info!(auth.role = %user.role, "discarded a project draft");
            phase_changed(
                state,
                user,
                shortcode,
                context.section.id,
                context,
                headers,
                page::Notice::Discarded,
            )
            .await
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not discard a project draft");
            refused_with(state, user, shortcode, context, headers, DISCARD_REFUSED_STORAGE, &[])
        }
    }
}

/// Take a pending submission back, or show the confirmation that
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
        return confirming(state, user, shortcode, context, headers, page::Confirmation::Withdrawal);
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
///
/// `pub(crate)` so the entity form (`entities.rs`) answers the same two ways
/// this form does, from one definition — a second copy could drift on which
/// header name it checks.
pub(crate) fn is_enhanced(headers: &HeaderMap) -> bool {
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
    // The re-resolve below needs the same deadline the request came in with;
    // taking it off the context rather than as an argument keeps it in step
    // with whatever `context()` was given.
    let signed_out_at = context.signed_out_at;
    if !is_enhanced(&headers) {
        return redirect_here(shortcode, context);
    }
    match self::context(state, user, shortcode, section_id, signed_out_at).await {
        Ok(fresh) => region(shortcode, &fresh, Rendering { notice: Some(notice), ..Rendering::default() }),
        // The write landed; only the re-read did not. A redirect is the
        // fail-safe answer — the browser follows it and finds the new phase,
        // where a refusal would report a failure that did not happen.
        Err(_) => redirect_here(shortcode, context),
    }
}

/// The withdrawal confirmation, rendered over the read-only form.
fn confirming(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    which: page::Confirmation,
) -> Response {
    let rendering = Rendering { confirming: Some(which), ..Rendering::default() };
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
/// Re-rendered rather than redirected on both paths: a redirect would throw away what was
/// typed, and the invalid values are exactly
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
        proposal_findings: &[],
        confirming: None,
        adding_row: None,
        // A refusal re-renders what was typed, which for a repeatable field
        // includes a row that has been added but not filled in.
        keep_posted: true,
    };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

/// A submission refused because one of this project's own proposals is not finished yet.
///
/// Its own function rather than another `refused_with` argument: every other call site passes an
/// empty proposal-findings slice, and a ninth optional-shaped parameter next to `errors` is exactly
/// the kind of adjacent-arguments mistake this file's own `Rendering` struct exists to rule out.
fn refused_with_proposal_findings(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    findings: &[(String, ProposalKind, String)],
) -> Response {
    let rendering = Rendering {
        notice: Some(page::Notice::Refused(SUBMIT_REFUSED_PROPOSAL_INCOMPLETE)),
        proposal_findings: findings,
        keep_posted: true,
        ..Rendering::default()
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
    /// Findings against this project's own live proposals (the submit gate below), each keyed by
    /// which proposal and what needs finishing.
    ///
    /// A parallel channel to `errors` rather than sharing it: a proposal is not a registry field,
    /// so `errors_elsewhere`'s field-id lookup would resolve to nothing and silently drop every
    /// one of these.
    proposal_findings: &'a [(String, ProposalKind, String)],
    confirming: Option<page::Confirmation>,
    /// The field one more blank row was just asked for.
    adding_row: Option<&'a str>,
    /// Whether this render must preserve the posted editing state.
    ///
    /// Set for a refusal and for a row action, and deliberately **not** for a
    /// successful save: a save commits every row with any text in it, and a row
    /// that still has none is not data, so dropping it is the point rather than
    /// a loss.
    keep_posted: bool,
}

/// The section region, as the enhanced path's `datastar-patch-elements`.
///
/// 200 always: Datastar processes a response body only on a 200, so a status
/// carrying the refusal would lose the message it is carrying.
fn region(shortcode: &str, context: &Context<'_>, rendering: Rendering<'_>) -> Response {
    let stored = stored_of(context);
    let view = view(shortcode, context, &stored, rendering);
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
    let stored = stored_of(context);
    let view = view(shortcode, context, &stored, rendering);
    // The published name in the tab title where there is one: a browser with
    // eleven tabs open shows about twenty characters, and five of them being
    // "Proje" helps nobody.
    let title = match context.project_name {
        Some(name) => format!("{} — {name} — DaSCH Metadata Editor", context.section.title),
        None => format!("{} — Project {shortcode} — DaSCH Metadata Editor", context.section.title),
    };
    crate::render(state, &title, StatusCode::OK, Some(user), page::page(&view))
}

/// What the stored draft row contributes to a rendering: when it was last
/// written, for a reader, and which revision that is, for the form to post back.
///
/// One struct rather than two `Option<String>` arguments: both are derived from
/// the same row and adjacent optionals of one type are silently swappable —
/// which here would show a depositor an RFC 3339 timestamp and post a
/// human-formatted one as the revision, so every save would report a conflict.
struct Stored {
    /// Formatted for a reader.
    saved_at: Option<String>,
    /// When this reader's session ends, formatted, and only when that is close
    /// enough to be worth saying.
    signed_out_at: Option<String>,
    /// The same instant as RFC 3339, which is what [`page::BASELINE`] carries.
    /// Machine-readable on purpose: it is compared, never shown.
    baseline: Option<String>,
}

fn stored_of(context: &Context<'_>) -> Stored {
    Stored {
        saved_at: context.record.as_ref().map(|record| crate::format_instant(record.updated_at)),
        baseline: context.record.as_ref().map(|record| record.updated_at.to_rfc3339()),
        signed_out_at: nearly_signed_out(context.signed_out_at).map(crate::format_instant),
    }
}

fn view<'a>(
    shortcode: &'a str,
    context: &'a Context<'a>,
    stored: &'a Stored,
    rendering: Rendering<'a>,
) -> page::SectionView<'a> {
    page::SectionView {
        shortcode,
        project_name: context.project_name,
        section: context.section,
        audience: context.audience,
        draft: &context.draft,
        locked: context.locked,
        awaiting_release: context.awaiting_release,
        accepted_fields: &context.accepted_fields,
        // RDU too, not only the assigned depositors: `may_reach` is already
        // true for every project for an RDU account, and a submission nobody
        // can take back is one only a reject can clear. The same predicate the
        // write applies, so the control is never offered where it is refused.
        may_withdraw: context.pending_submission().is_some(),
        // A draft to discard, and no live review to pull it out from under.
        // Both halves matter: without a draft the form already shows published
        // metadata, and while a submission is pending the draft is what the
        // depositor comes back to.
        may_discard: context.record.is_some() && context.locked.is_none(),
        confirming: rendering.confirming,
        errors: rendering.errors,
        proposal_findings: rendering.proposal_findings,
        proposals: &context.proposals,
        round: context.round.as_ref().map(|round| page::RoundSummary {
            outcome: round.outcome,
            note: round.note.as_deref(),
            at: &round.at,
            substitutions: &round.substitutions,
        }),
        saved_at: stored.saved_at.as_deref(),
        last_editor: context.last_editor.as_deref(),
        // The revision this render is of, so a save posted from it can tell
        // whether the draft moved underneath. Refreshed on every render,
        // including the one that reports a conflict — otherwise a depositor who
        // decides to keep their version would be refused for ever.
        baseline: stored.baseline.as_deref(),
        signed_out_at: stored.signed_out_at.as_deref(),
        notice: rendering.notice,
        agents: Some(&context.agents),
        posted: rendering.keep_posted.then_some(context.posted).flatten(),
        adding_row: rendering.adding_row,
        rows_action: format!("/projects/{shortcode}/sections/{}/fields", context.section.id),
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
    use editor_core::repository::ApprovedRecordRepository;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::{
        a_session, a_user, body_string, count_rows, get, location, open_test_db, post, state_over, test_app,
        test_state, with_cookie, Faults, FaultyDatabase, RecordingMailer,
    };

    /// Percent-encode a form value.
    ///
    /// Delegates to `test_support`'s encoder rather than keeping a second one. A second encoder
    /// that missed a character silently truncates a posted value and compares the truncation
    /// against the file: a draft revision reads `…45.345567+00:00`, so an unescaped `+` arrives
    /// as a space and a baseline compares unequal to itself.
    fn urlencoding(value: &str) -> String {
        crate::test_support::urlencode(value)
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
        // The form opens pre-filled from the published metadata, with
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
        // The access check runs before anything is read: a 404 for an unpublished
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
        // A save must always be possible, and POST-redirect-GET: a `POST` left in the history
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
        // The gap this closes: a rejection discards the submission, notifications
        // are out of scope, and the lifecycle states have no Rejected — so without
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
        // A reviewer may edit before accepting and there is no second approver, which waives the
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
        // The state list is fixed at five and has no "returned", so the
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
        // The submission carries the draft as it stands, and the
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
        // What makes request-changes and withdraw work: the
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
    async fn submitting_with_a_required_field_emptied_is_refused_and_names_it() {
        // `shortDescription` is emptied through the form rather than written straight to the draft,
        // because that is the only way a depositor reaches this state: the control renders,
        // they clear it, and the value is a valid `ProjectRaw` member the whole way — which
        // is why `to_raw` cannot catch it.
        let (state, _) = test_state("section-submit-unanswered").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(
            &app,
            post(OVERVIEW, "name=A+New+Title&shortDescription=&intent=submit"),
            &session,
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a refusal re-renders rather than redirecting"
        );
        let body = body_string(response).await;
        assert!(body.contains(UNANSWERED_MESSAGE), "the field-level error shows: {body}");
        assert!(
            body.contains("some required fields have no value"),
            "the refusal promises named fields: {body}"
        );
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
        // A refusal costs the submission, never the editing.
        let draft = DraftRepository::find(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("the draft survives a refused submission");
        assert!(draft.payload.contains("A New Title"), "{}", draft.payload);
    }

    #[tokio::test]
    async fn a_required_field_in_another_section_is_refused_with_a_link_to_it() {
        // The gate is whole-project while the form is sectioned, so the field at
        // fault is routinely not on the page the depositor submitted from.
        // Without the link the refusal names a field the reader cannot find
        // among six sections.
        let (state, _) = test_state("section-submit-unanswered-elsewhere").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // Emptied **through the route a depositor uses**, because hand-building `keywords: []` is a
        // state the real path does not produce: removing the last row drops the member
        // entirely, `keywords` being a non-`Option` `Vec` whose applier uses absent as its
        // empty state. Only the real path reaches what the gate must catch.
        as_session(
            &app,
            post(
                "/projects/0801d/sections/dataset/fields/keywords/r0/remove",
                "keywords.row=r0&keywords.r0.en=the+only+keyword",
            ),
            &session,
        )
        .await;
        let stored = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(
            !stored.payload.contains("\"keywords\""),
            "the member is absent: {}",
            stored.payload
        );

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=submit"), &session).await;
        let body = body_string(response).await;
        assert!(
            body.contains("some required fields have no value"),
            "the named refusal, not the generic contract one: {body}"
        );
        assert!(
            body.contains("/projects/0801d/sections/dataset"),
            "the section holding the field is linked: {body}"
        );
        assert!(body.contains("Keywords"), "the field is named: {body}");
    }

    #[tokio::test]
    async fn submitting_a_published_project_unchanged_is_not_refused_by_the_obligation_gate() {
        // The corpus-wide guarantee at the route rather than in the unit test:
        // bounding the required tier by what the published corpus answers is
        // what keeps the gate from refusing a live project. If it did refuse
        // one, the refusal here would be the obligation gate and not the
        // "nothing to submit" one that a genuinely unchanged draft earns.
        let (state, _) = test_state("section-submit-unanswered-published").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=submit"), &session).await;
        let body = body_string(response).await;
        assert!(
            !body.contains(UNANSWERED_MESSAGE),
            "a published project answers every required field: {body}"
        );
        assert!(
            body.contains("identical to what is published"),
            "the refusal it does earn is the unchanged one: {body}"
        );
    }

    const DATASET: &str = "/projects/0801d/sections/dataset";

    /// A depositor with the dataset section open.
    /// The stored draft for `shortcode`, as the repository holds it.
    async fn stored_draft(state: &AppState, shortcode: &str) -> ProjectDraft {
        let row = DraftRepository::find(&*state.db, shortcode)
            .await
            .expect("read")
            .expect("a stored draft");
        serde_json::from_str(&row.payload).expect("parses")
    }

    async fn a_depositor_on(name: &str) -> (AppState, axum::Router, String) {
        let (state, _) = test_state(name).await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        (state, app, session)
    }

    #[tokio::test]
    async fn adding_a_row_renders_one_more_and_stores_nothing_extra() {
        // The blank row is a rendering, not a stored value:
        // `apply_multilingual_rows` drops a row with no text in any language,
        // and it must, or a published file fills with empty objects. It survives
        // the next round trip because the tile emits a hidden `{field}.row` for
        // it and the body carries the key back.
        let (state, app, session) = a_depositor_on("section-add-row").await;

        let response = as_session(
            &app,
            post(
                &format!("{DATASET}/fields/keywords/add"),
                "keywords.row=r0&keywords.r0.en=manuscripts",
            ),
            &session,
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a row action re-renders rather than redirecting"
        );
        let body = body_string(response).await;

        // The row that was filled in comes back, and one blank row is added
        // under a fresh key.
        assert!(body.contains("manuscripts"), "the filled row survives: {body}");
        assert!(body.contains(r#"name="keywords.r1.en""#), "the new row renders: {body}");

        // Only the filled row reached storage. The blank one is a rendering.
        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        let stored: serde_json::Value = serde_json::from_str(&draft.payload).unwrap();
        assert_eq!(
            stored.get("keywords"),
            Some(&serde_json::json!([{ "en": "manuscripts" }])),
            "{}",
            draft.payload
        );
    }

    #[tokio::test]
    async fn removing_a_row_drops_it_from_the_stored_list() {
        // The removal *is* the applier seeing one fewer key: there is no
        // separate delete, which is what keeps a row action in step with a save.
        let (state, app, session) = a_depositor_on("section-remove-row").await;
        let published = state.published.get("0801d").expect("0801d");
        assert!(published.keywords.len() >= 2, "0801d was chosen for having several keywords");

        // Post every row, then remove the first.
        let mut form = String::new();
        for (position, keyword) in published.keywords.iter().enumerate() {
            form.push_str(&format!("&keywords.row=r{position}"));
            for (tag, text) in keyword.iter() {
                form.push_str(&format!("&keywords.r{position}.{tag}={}", urlencoding(text)));
            }
        }
        let response = as_session(
            &app,
            post(&format!("{DATASET}/fields/keywords/r0/remove"), form.trim_start_matches('&')),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        let stored: serde_json::Value = serde_json::from_str(&draft.payload).unwrap();
        assert_eq!(
            stored.get("keywords").and_then(|k| k.as_array()).map(Vec::len),
            Some(published.keywords.len() - 1),
            "{}",
            draft.payload
        );
    }

    #[tokio::test]
    async fn removing_the_last_row_clears_the_field_rather_than_leaving_it() {
        // The marker case. With no `{field}.row` left in the body the applier
        // reads the field as absent and leaves it alone, so removing the only
        // row would not stick — the handler re-adds the empty marker for
        // exactly that reason.
        let (state, app, session) = a_depositor_on("section-remove-last-row").await;

        let response = as_session(
            &app,
            post(
                &format!("{DATASET}/fields/keywords/r0/remove"),
                "keywords.row=r0&keywords.r0.en=only",
            ),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        let stored: serde_json::Value = serde_json::from_str(&draft.payload).unwrap();
        assert!(
            stored.get("keywords").is_none(),
            "the last removal must stick: {}",
            draft.payload
        );
    }

    #[tokio::test]
    async fn the_row_routes_serve_a_string_row_field_too() {
        // One row protocol for both row shapes: `additionalMaterial` holds
        // plain strings rather than language maps, and shares the tile, the
        // hidden `{field}.row` key, the empty marker and these two routes. A
        // second set of routes for it would be a second place for the audience
        // gate and the lock check to drift.
        let (state, app, session) = a_depositor_on("section-string-rows").await;

        let added = as_session(
            &app,
            post(
                &format!("{DATASET}/fields/additionalMaterial/add"),
                "additionalMaterial.row=r0&additionalMaterial.r0=https%3A%2F%2Ffirst.example%2F",
            ),
            &session,
        )
        .await;
        assert_eq!(added.status(), StatusCode::OK);
        let body = body_string(added).await;
        assert!(body.contains("https://first.example/"), "the filled row survives: {body}");
        assert!(body.contains(r#"name="additionalMaterial.r1""#), "a blank row is added: {body}");

        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        let stored: serde_json::Value = serde_json::from_str(&draft.payload).unwrap();
        assert_eq!(
            stored.get("additionalMaterial"),
            Some(&serde_json::json!(["https://first.example/"])),
            "{}",
            draft.payload
        );

        let removed = as_session(
            &app,
            post(
                &format!("{DATASET}/fields/additionalMaterial/r0/remove"),
                "additionalMaterial.row=r0&additionalMaterial.r0=https%3A%2F%2Ffirst.example%2F",
            ),
            &session,
        )
        .await;
        assert_eq!(removed.status(), StatusCode::OK);
        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        let stored: serde_json::Value = serde_json::from_str(&draft.payload).unwrap();
        assert!(
            stored.get("additionalMaterial").is_none(),
            "the last removal sticks: {}",
            draft.payload
        );
    }

    #[tokio::test]
    async fn a_depositor_cannot_reach_the_rdu_only_row_field() {
        // `documentationMaterial` is a string-row field like the one above, and
        // RDU-only. The URL names the field, so `fields_for` is the only thing
        // between a depositor and writing it.
        let (state, app, session) = a_depositor_on("section-rdu-row-field").await;
        let response = as_session(
            &app,
            post(
                &format!("{DATASET}/fields/documentationMaterial/add"),
                "documentationMaterial.row=r0&documentationMaterial.r0=https%3A%2F%2Fdocs.example%2F",
            ),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was written"
        );
    }

    #[tokio::test]
    async fn a_row_action_on_a_field_the_reader_may_not_write_is_a_404() {
        // The URL names the field, so without the `fields_for` check it is a way
        // to reach an RDU-only or a display-only field. `documentationMaterial`
        // is RDU-only; `howToCite` is display-only and in another section.
        let (_state, app, session) = a_depositor_on("section-row-forbidden").await;
        for field in ["documentationMaterial", "howToCite", "provenance", "nonsense"] {
            let response = as_session(&app, post(&format!("{DATASET}/fields/{field}/add"), ""), &session).await;
            assert_eq!(
                response.status(),
                StatusCode::NOT_FOUND,
                "{field} should not be reachable as a repeatable field"
            );
        }
    }

    #[tokio::test]
    async fn a_search_offers_matches_stores_the_body_and_changes_no_reference() {
        // The picker's whole round trip: type a name, press Search, choose from the menu, save.
        // The search itself must store what was typed like a save does — so nothing is lost to a
        // lookup — while changing no reference of its own.
        let (state, app, session) = a_depositor_on("section-agent-search").await;
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";

        // A contact point that resolves, saved through the form.
        as_session(
            &app,
            post(CONTRIBUTORS, "contactPoint.row=r0&contactPoint.r0=organization-008"),
            &session,
        )
        .await;

        // Search from that row. The query rides in the body; the button names the intent.
        let searched = body_string(
            as_session(
                &app,
                post(
                    CONTRIBUTORS,
                    "contactPoint.row=r0&contactPoint.r0=organization-008\
                     &contactPoint.r0.q=Bernoulli&intent=find-agent",
                ),
                &session,
            )
            .await,
        )
        .await;
        assert!(searched.contains("<select"), "the search offers a real menu: {searched}");
        assert!(searched.contains(r#"value="Bernoulli""#), "the query comes back in the box");
        // A search decides nothing: the stored reference is still the one that was there.
        let after_search = stored_draft(&state, "0801d").await;
        assert_eq!(
            after_search.get("contactPoint"),
            Some(&serde_json::json!(["organization-008"])),
            "a search must not change a reference"
        );

        // Now choose from the menu, which posts under the same name the applier already reads.
        as_session(
            &app,
            post(CONTRIBUTORS, "contactPoint.row=r0&contactPoint.r0=organization-002"),
            &session,
        )
        .await;
        assert_eq!(
            stored_draft(&state, "0801d").await.get("contactPoint"),
            Some(&serde_json::json!(["organization-002"])),
            "the chosen id is what lands in the draft"
        );
    }

    #[tokio::test]
    async fn every_add_and_remove_control_the_form_renders_resolves() {
        // Posted to the URLs the pages themselves draw, over every section a depositor sees,
        // rather than to hand-written paths for the fields someone remembered.
        //
        // The narrower version of this — naming `keywords` and `alternativeNames` — passed while
        // eleven of the twenty-three controls answered `404` and threw the form away with it:
        // `disciplines`, `spatialCoverage`, `temporalCoverage`, `publications` and `funding` all
        // rendered an add button that `row_action`'s shape allowlist did not accept. `has_rows`
        // is exhaustive so the shapes cannot drift apart again, and this is what checks that the
        // route agrees with the markup.
        let (_state, app, session) = a_depositor_on("section-row-actions-resolve").await;

        let mut checked = 0;
        for section in registry::sections_for(Audience::Everyone) {
            let url = format!("/projects/0801d/sections/{}", section.id);
            let rendered = body_string(as_session(&app, get(&url), &session).await).await;
            let mut actions: Vec<&str> = rendered
                .split(r#"formaction=""#)
                .skip(1)
                .filter_map(|rest| rest.split('"').next())
                .collect();
            actions.sort_unstable();
            actions.dedup();
            for action in actions {
                let status = as_session(&app, post(action, ""), &session).await.status();
                assert_eq!(
                    status,
                    StatusCode::OK,
                    "{} renders {action}, which answers {status}",
                    section.id
                );
                checked += 1;
            }
        }
        // The count is asserted so a registry change that stops rendering row controls entirely
        // cannot make this test vacuous.
        assert!(checked >= 20, "only {checked} row controls were found to post to");
    }

    #[tokio::test]
    async fn a_row_action_against_a_project_in_review_is_refused() {
        // The lock is re-checked here and not only on the save path: a row
        // action is a write, and it resolves through the same `context()`.
        let (state, _) = test_state("section-row-locked").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_submission(&state, "0801d", user.id, SubmissionState::Submitted).await;

        let response = as_session(
            &app,
            post(&format!("{DATASET}/fields/keywords/add"), "keywords.row=r0"),
            &session,
        )
        .await;
        let body = body_string(response).await;
        assert!(body.contains("in review"), "{body}");
        assert_eq!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was written"
        );
    }

    #[tokio::test]
    async fn a_save_carrying_more_values_than_the_cap_is_refused_and_writes_nothing() {
        // The cap a depositor can see, and the reason it is on the save path
        // and not only on submit: an applier truncates at
        // `MAX_VALUES_PER_FIELD`, so an over-cap save would store the
        // truncated value and the submit after it would see a draft already
        // within the cap and pass. Refusing the write is what makes "nothing
        // was saved" true.
        let (state, _) = test_state("section-save-over-cap").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let mut form = String::from("name=A+New+Title");
        for n in 0..=editor_core::form::MAX_VALUES_PER_FIELD {
            form.push_str(&format!("&description.l{n}=text"));
        }
        let response = as_session(&app, post(OVERVIEW, &form), &session).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a refusal re-renders rather than redirecting"
        );
        let body = body_string(response).await;
        assert!(body.contains("more values than this form accepts"), "{body}");
        assert!(body.contains("Description"), "the field is named: {body}");
        assert_eq!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was written, including the name that was fine"
        );
    }

    #[tokio::test]
    async fn a_save_at_exactly_the_cap_is_accepted() {
        // The boundary in the direction that matters: a cap that refused at its
        // own limit would be off by one against the number the refusal states.
        let (state, _) = test_state("section-save-at-cap").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let mut form = String::from("name=A+New+Title");
        for n in 0..editor_core::form::MAX_VALUES_PER_FIELD {
            form.push_str(&format!("&description.l{n}=text"));
        }
        let response = as_session(&app, post(OVERVIEW, &form), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "a save at the cap is accepted");
        assert!(DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn submitting_an_agent_reference_that_resolves_to_nobody_is_refused() {
        // The applier stores whatever id arrives, because a draft may hold a
        // value that does not validate. This is what stops it
        // reaching a published file, where the public project page would render
        // a bare `person-99999`.
        let (state, _) = test_state("section-unknown-agent").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";

        let response = as_session(
            &app,
            post(CONTRIBUTORS, "contactPoint.row=r0&contactPoint.r0=person-99999&intent=submit"),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "a refusal re-renders");
        let body = body_string(response).await;
        assert!(body.contains("person-99999"), "the offending id is named: {body}");
        assert!(body.contains("not a person or organisation the repository knows"), "{body}");
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
    }

    #[tokio::test]
    async fn submitting_a_reference_that_resolves_is_accepted() {
        // The other half: a real id passes the gate, so the refusal above is
        // about resolution and not about the field being touched at all.
        let (state, _) = test_state("section-known-agent").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";

        let response = as_session(
            &app,
            post(
                CONTRIBUTORS,
                "contactPoint.row=r0&contactPoint.r0=organization-008&intent=submit",
            ),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "the submission is recorded");
        let submission = SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
            .await
            .unwrap()
            .expect("a pending submission");
        assert!(submission.payload.contains("organization-008"), "{}", submission.payload);
    }

    #[tokio::test]
    async fn submitting_a_typed_placeholder_sentinel_is_refused_with_a_field_error() {
        // The dead end the architecture doc carried as an open `[!NOTE]`: typing
        // the word stores something the platform reads as "no value", the
        // control then renders empty, and an empty submit will not clear it.
        // Typed through the form, because that is the only way a depositor
        // reaches it — and `provenance` is `WhenCleared::Drop`, where clearing
        // removes the member, so the sentinel cannot have come from clearing.
        let (state, _) = test_state("section-submit-sentinel").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(DATASET, "provenance=MISSING&intent=submit"), &session).await;
        assert_eq!(response.status(), StatusCode::OK, "a refusal re-renders");
        let body = body_string(response).await;
        assert!(body.contains("reserved here for a value that is not filled in"), "{body}");
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
        // The draft keeps it, so the depositor sees what they typed and can fix
        // it — a refusal costs the submission, never the editing.
        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(draft.payload.contains("MISSING"), "{}", draft.payload);
    }

    #[tokio::test]
    async fn submitting_a_project_whose_end_date_is_a_sentinel_is_not_refused_for_it() {
        // The other half, and the reason the check reads the declared shape rather than the value:
        // `endDate` is `WhenCleared::Placeholder`, so `"MISSING"` is what the editor writes
        // when a depositor clears it, and published projects hold it. Refusing it would
        // make every ongoing project unsubmittable.
        let (state, _) = test_state("section-submit-sentinel-legit").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "name=A+New+Title&endDate=&intent=submit"), &session).await;
        let body = body_string(response).await;
        assert!(!body.contains("reserved here for a value"), "{body}");
        assert!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
                .await
                .unwrap()
                .is_some(),
            "the submission should be recorded: {body}"
        );
    }

    #[tokio::test]
    async fn submitting_an_unresolvable_period_is_refused_with_a_field_error() {
        // Every `temporalCoverage` entry has to resolve, applied through the same function
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
        // `may_reach` is already true for every project for an RDU account, so the
        // direct-editing half was there; this is the half that makes the result reviewable.
        // Identical in shape means it is `Submitted` with no reviewer — an RDU submission is
        // not self-approving, because what it has to produce is a *pending* submission.
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
        // A withdrawal leaves the draft, so the depositor keeps editing and can
        // submit again.
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
        // The observable point of a withdrawal: the depositor got their form back.
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

    /// The two accounts a concurrency test needs: both assigned to one project,
    /// which is the normal case rather than an edge one.
    async fn two_project_mates(state: &AppState) -> (User, String, User, String) {
        let first = a_user(state, "one@example.test", "Ada One", Role::Depositor, &["0801d"]).await;
        let second = a_user(state, "two@example.test", "Bo Two", Role::Depositor, &["0801d"]).await;
        let first_session = a_session(state, first.id).await;
        let second_session = a_session(state, second.id).await;
        (first, first_session, second, second_session)
    }

    /// The baseline the form currently renders, as a body would post it.
    async fn baseline_of(state: &AppState, shortcode: &str) -> String {
        let record = DraftRepository::find(&*state.db, shortcode)
            .await
            .unwrap()
            .expect("a draft to take a baseline from");
        urlencoding(&record.updated_at.to_rfc3339())
    }

    #[test]
    fn only_a_deadline_within_the_warning_window_reaches_a_reader() {
        // The filter itself, which the view cannot test: a rendering is handed
        // the deadline already filtered, so a test that injects one proves the
        // markup and not the threshold. Removing the filter passed both of
        // those, which is what this closes.
        let now = Utc::now();
        assert!(nearly_signed_out(now + chrono::Duration::minutes(29)).is_some());
        assert!(nearly_signed_out(now + chrono::Duration::minutes(31)).is_none());
        // A session that has just run out still warrants saying so, rather than
        // going quiet at the moment it matters most.
        assert!(nearly_signed_out(now - chrono::Duration::minutes(1)).is_some());
    }

    #[tokio::test]
    async fn a_fresh_session_is_not_warned_about_signing_out() {
        // The default session is twelve hours absolute and two hours idle, so
        // an ordinary form must be quiet. A warning permanently on screen is a
        // warning nobody reads.
        let (state, _) = test_state("section-no-warning").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!body.contains("about to end"), "{body}");
    }

    #[tokio::test]
    async fn an_autosave_carries_no_intent_and_is_treated_as_a_save() {
        // What the autosave trigger actually posts: the form body with no
        // submitter, so no `intent` at all. The handler reads an unknown or
        // absent verb as `save` — the recoverable branch, deliberately — so an
        // autosave can never submit or withdraw.
        let (state, _) = test_state("section-autosave").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, enhanced(OVERVIEW, "name=Typed+then+left+the+field"), &session).await;
        assert_eq!(response.status(), StatusCode::OK, "the enhanced path patches the region");

        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(draft.payload.contains("Typed then left the field"), "{}", draft.payload);
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "an autosave must never submit"
        );
    }

    #[tokio::test]
    async fn a_save_over_a_draft_that_changed_underneath_is_refused_and_names_who() {
        // Two members of one project team, both with the form open. Without
        // this the second save silently replaces the first person's work.
        let (state, _) = test_state("section-concurrent").await;
        let (_first, first_session, _second, second_session) = two_project_mates(&state).await;
        let app = test_app(&state);

        // Ada saves, so a draft exists; Bo's form was rendered from it.
        as_session(&app, post(OVERVIEW, "name=Ada+was+here"), &first_session).await;
        let bo_baseline = baseline_of(&state, "0801d").await;

        // Ada saves again, moving the draft under Bo's open form.
        as_session(&app, post(OVERVIEW, "name=Ada+again"), &first_session).await;

        let response = as_session(
            &app,
            post(OVERVIEW, &format!("name=Bo+was+here&baseline={bo_baseline}")),
            &second_session,
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "a refusal re-renders rather than redirecting"
        );
        let body = body_string(response).await;
        assert!(body.contains("changed while you were editing"), "{body}");
        assert!(body.contains("Ada One"), "the other editor is named: {body}");
        // What was typed comes back, so nothing has to be retyped.
        assert!(body.contains("Bo was here"), "the typed value survives: {body}");

        let stored = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(
            stored.payload.contains("Ada again"),
            "nothing was overwritten: {}",
            stored.payload
        );
        assert!(!stored.payload.contains("Bo was here"), "{}", stored.payload);
    }

    #[tokio::test]
    async fn saving_again_after_the_warning_keeps_this_depositors_version() {
        // Refused once, not for ever: the re-render carries a refreshed baseline, so a depositor
        // who decides to keep their version can. The row is still last-write-wins; what
        // this adds is that the overwrite is visible.
        let (state, _) = test_state("section-concurrent-again").await;
        let (_first, first_session, _second, second_session) = two_project_mates(&state).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=Ada+was+here"), &first_session).await;
        let stale = baseline_of(&state, "0801d").await;
        as_session(&app, post(OVERVIEW, "name=Ada+again"), &first_session).await;

        // Refused once.
        as_session(
            &app,
            post(OVERVIEW, &format!("name=Bo+was+here&baseline={stale}")),
            &second_session,
        )
        .await;

        // Saving again, with the baseline the refusal re-rendered.
        let fresh = baseline_of(&state, "0801d").await;
        let response = as_session(
            &app,
            post(OVERVIEW, &format!("name=Bo+was+here&baseline={fresh}")),
            &second_session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "the second save is accepted");
        let stored = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(stored.payload.contains("Bo was here"), "{}", stored.payload);
    }

    #[tokio::test]
    async fn a_body_with_no_baseline_is_saved_without_complaint() {
        // Deliberate: a body with no baseline was not posted from a form this
        // service rendered, so there is no revision it could have been looking
        // at and refusing it would protect nothing. It is also what keeps every
        // other test in this file — none of which posts one — testing the save
        // path rather than this check.
        let (state, _) = test_state("section-no-baseline").await;
        let (_first, first_session, _second, second_session) = two_project_mates(&state).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=Ada+was+here"), &first_session).await;
        let response = as_session(&app, post(OVERVIEW, "name=Bo+was+here"), &second_session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn the_form_names_the_other_editor_but_not_the_reader_themselves() {
        // "Saved by you" is the ordinary case and reads as noise, which is also
        // why the name is only looked up when it is somebody else.
        let (state, _) = test_state("section-last-editor").await;
        let (_first, first_session, _second, second_session) = two_project_mates(&state).await;
        let app = test_app(&state);
        as_session(&app, post(OVERVIEW, "name=Ada+was+here"), &first_session).await;

        let own = body_string(as_session(&app, get(OVERVIEW), &first_session).await).await;
        assert!(own.contains("Draft last saved"), "{own}");
        assert!(!own.contains("by Ada One"), "a reader is not told they are themselves: {own}");

        let mate = body_string(as_session(&app, get(OVERVIEW), &second_session).await).await;
        assert!(mate.contains("by Ada One"), "the other editor is named: {mate}");
    }

    #[tokio::test]
    async fn the_form_posts_the_revision_it_was_rendered_from() {
        // The mechanism, asserted at the markup: without the hidden field the
        // check above has nothing to compare and every concurrent save is
        // silent again.
        let (state, _) = test_state("section-baseline-rendered").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // No draft yet: nothing to have moved, so nothing to post.
        let blank = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!blank.contains(r#"name="baseline""#), "{blank}");

        a_changed_draft(&state, &app, &session).await;
        let opened = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(opened.contains(r#"name="baseline""#), "{opened}");
        let record = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(
            opened.contains(&record.updated_at.to_rfc3339()),
            "the stored revision, not a formatted time: {opened}"
        );
    }

    #[tokio::test]
    async fn discarding_asks_first_and_writes_nothing_until_confirmed() {
        // The draft is the only copy of whatever has not been submitted, and
        // the delete cannot be undone.
        let (state, _) = test_state("section-discard-confirm").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        let response = as_session(&app, post(OVERVIEW, "intent=discard-confirm"), &session).await;
        assert_eq!(response.status(), StatusCode::OK, "the confirmation re-renders");
        let body = body_string(response).await;
        assert!(body.contains("cannot be undone"), "{body}");
        assert!(body.contains("Yes, discard the draft"), "{body}");
        // Backing out has to write nothing, so it is a link and not a button
        // that would fall through to `save`.
        assert!(body.contains("Keep the draft"), "{body}");
        assert!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some(),
            "asking must not delete"
        );
    }

    #[tokio::test]
    async fn discarding_removes_the_draft_and_leaves_the_published_project() {
        // The one thing in the service that removes a draft — reject and
        // withdraw both keep it — and it removes only the draft: the form
        // re-opens pre-filled from the published metadata.
        let (state, _) = test_state("section-discard").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        let response = as_session(&app, post(OVERVIEW, "intent=discard"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "the plain path redirects to the GET");
        assert_eq!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap(),
            None,
            "the draft is gone"
        );

        // The form still works and shows the published name, not the discarded
        // draft's title.
        let reopened = as_session(&app, get(OVERVIEW), &session).await;
        assert_eq!(reopened.status(), StatusCode::OK);
        let body = body_string(reopened).await;
        assert!(body.contains("Basler Edition der Bernoulli-Briefwechsel"), "{body}");
        assert!(!body.contains("A New Title"), "the discarded value is gone: {body}");
        assert!(body.contains("Nothing saved yet"), "{body}");
    }

    #[tokio::test]
    async fn the_enhanced_path_re_renders_a_discard_without_the_control_it_just_used() {
        // `phase_changed` re-resolves the request rather than patching two
        // fields of the old context, and this is the case that needs it: after
        // the draft is gone there is nothing to discard, so a region rendered
        // from the pre-delete context would still offer the control and still
        // claim a last-saved time.
        let (state, _) = test_state("section-discard-enhanced").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;

        let response = as_session(&app, enhanced(OVERVIEW, "intent=discard"), &session).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "the enhanced path patches rather than redirecting"
        );
        let body = body_string(response).await;
        assert!(!body.starts_with("<!DOCTYPE html>"), "a region, not a document: {body}");
        assert!(body.contains("Draft discarded"), "{body}");
        assert!(!body.contains("Discard draft"), "the control is gone with the draft: {body}");
        assert!(body.contains("Nothing saved yet"), "{body}");
        assert_eq!(DraftRepository::find(&*state.db, "0801d").await.unwrap(), None);
    }

    #[tokio::test]
    async fn discarding_a_project_with_no_draft_is_refused_and_the_control_is_not_offered() {
        // Nothing to discard: the form is already showing published metadata.
        let (state, _) = test_state("section-discard-none").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let opened = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!opened.contains("Discard draft"), "no draft, no control: {opened}");

        let response = as_session(&app, post(OVERVIEW, "intent=discard"), &session).await;
        let body = body_string(response).await;
        assert!(body.contains("no draft to discard"), "{body}");
    }

    #[tokio::test]
    async fn discarding_a_draft_under_review_is_refused() {
        // The draft is what the depositor comes back to when RDU returns the
        // project, so it must not vanish from under a live review, and the
        // refusal says what to do instead.
        let (state, _) = test_state("section-discard-locked").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_changed_draft(&state, &app, &session).await;
        a_submission(&state, "0801d", user.id, SubmissionState::Submitted).await;

        let opened = body_string(as_session(&app, get(OVERVIEW), &session).await).await;
        assert!(!opened.contains("Discard draft"), "locked, so no control: {opened}");

        let response = as_session(&app, post(OVERVIEW, "intent=discard"), &session).await;
        let body = body_string(response).await;
        assert!(body.contains("cannot be discarded"), "{body}");
        assert!(body.contains("Take the submission back first"), "the way out is named: {body}");
        assert!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some(),
            "the draft survives"
        );
    }

    #[tokio::test]
    async fn a_depositor_cannot_discard_a_draft_on_a_project_they_are_not_assigned() {
        // The same gate every write here goes through: `context()` resolves
        // shape, then authorization, before anything reads state.
        let (state, _) = test_state("section-discard-forbidden").await;
        let owner = a_user(&state, "own@example.test", "An Owner", Role::Depositor, &["0801d"]).await;
        let app = test_app(&state);
        let owner_session = a_session(&state, owner.id).await;
        a_changed_draft(&state, &app, &owner_session).await;

        let outsider = a_user(&state, "out@example.test", "An Outsider", Role::Depositor, &["0803"]).await;
        let session = a_session(&state, outsider.id).await;
        let response = as_session(&app, post(OVERVIEW, "intent=discard"), &session).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(
            DraftRepository::find(&*state.db, "0801d").await.unwrap().is_some(),
            "the draft survives"
        );
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
        // "Their own" submission reads as the project's, which is how the rest
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
        // Request-changes retains the per-field state, and nothing stopped the
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
        // Absent from the published set is not "does not exist", and the published metadata
        // the form pre-fills from is then empty.
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

    /// Every proposal stored for `shortcode`, for asserting on what a propose intent wrote.
    async fn proposals_for(state: &AppState, shortcode: &str) -> Vec<EntityProposal> {
        EntityProposalRepository::list_for_shortcode(&*state.db, shortcode)
            .await
            .expect("proposals should read")
    }

    #[tokio::test]
    async fn propose_person_allocates_against_the_committed_corpus_and_names_the_id() {
        let (state, _) = test_state("section-propose-person").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("person-417"), "{body}");

        let proposals = proposals_for(&state, "0801d").await;
        assert_eq!(proposals.len(), 1, "{proposals:?}");
        assert_eq!(proposals[0].entity_id, "person-417");
        assert_eq!(proposals[0].kind, ProposalKind::Person);
        assert_eq!(proposals[0].operation, ProposalOperation::New);
        assert_eq!(proposals[0].status, ProposalStatus::Draft);
        assert_eq!(proposals[0].proposed_by, Some(user.id));
    }

    #[tokio::test]
    async fn propose_organization_allocates_against_the_committed_corpus() {
        let (state, _) = test_state("section-propose-organization").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=propose-organization"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("organization-143"));

        let proposals = proposals_for(&state, "0801d").await;
        assert_eq!(proposals.len(), 1, "{proposals:?}");
        assert_eq!(proposals[0].entity_id, "organization-143");
        assert_eq!(proposals[0].kind, ProposalKind::Organization);
    }

    #[tokio::test]
    async fn proposing_twice_allocates_two_different_ids() {
        let (state, _) = test_state("section-propose-twice").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;
        as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;

        let mut ids: Vec<String> = proposals_for(&state, "0801d").await.into_iter().map(|p| p.entity_id).collect();
        ids.sort();
        assert_eq!(ids, vec!["person-417".to_string(), "person-418".to_string()]);
    }

    #[tokio::test]
    async fn a_propose_intent_saves_the_posted_body_first() {
        // Same reasoning as submit's own comment: starting a proposal must not cost the depositor
        // whatever they had just typed.
        let (state, _) = test_state("section-propose-saves-first").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "name=A+New+Title&intent=propose-person"), &session).await;

        let draft = DraftRepository::find(&*state.db, "0801d").await.unwrap().expect("a draft");
        assert!(draft.payload.contains("A New Title"), "{}", draft.payload);
    }

    #[tokio::test]
    async fn propose_changes_with_an_id_that_resolves_to_nobody_creates_nothing_and_refuses() {
        let (state, _) = test_state("section-propose-changes-unknown").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=propose-changes:person-999"), &session).await;
        assert_eq!(response.status(), StatusCode::OK, "a refusal, not a 500");
        assert!(
            body_string(response).await.contains("nothing was started"),
            "the reason is stated"
        );
        assert!(proposals_for(&state, "0801d").await.is_empty());
    }

    /// The bug two reviewers found independently, driven through the **rendered markup** rather
    /// than a hand-built body.
    ///
    /// A section renders every one of its fields inside one `<form>`, so a project with two
    /// resolved agent references renders two "Propose changes" controls. Both a native submit and
    /// Datastar's form mode post every *field* regardless of which button was clicked, so when the
    /// entity rode in a hidden input the body carried both ids and `FormBody::get` took the first:
    /// clicking the second row's button proposed a change to the first row's entity, silently. Only
    /// the activated **button** posts its name and value, which is why the id lives there.
    ///
    /// The old tests could not catch this: each hand-crafted a body with one `propose.entity`
    /// value, so the collision never existed in them.
    #[tokio::test]
    async fn propose_changes_targets_the_row_whose_button_was_clicked() {
        let (state, _) = test_state("section-propose-changes-second-row").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";

        // Two resolved contributors, saved through the form itself so the stored draft is exactly
        // what a depositor would have produced — and so the render below is the real markup.
        as_session(
            &app,
            post(
                CONTRIBUTORS,
                "attributions.row=r0&attributions.r0.contributor=person-001&attributions.r0.role=Author\
                 &attributions.row=r1&attributions.r1.contributor=organization-008&attributions.r1.role=Funder",
            ),
            &session,
        )
        .await;

        let rendered = body_string(as_session(&app, get(CONTRIBUTORS), &session).await).await;
        assert!(
            rendered.contains(r#"value="propose-changes:person-001""#)
                && rendered.contains(r#"value="propose-changes:organization-008""#),
            "both rows must offer their own control: {rendered}"
        );
        // Neither row may put the id in a field every submit carries.
        assert!(!rendered.contains(r#"name="propose.entity""#), "{rendered}");

        // Click the **second** row's control, exactly as the markup posts it.
        as_session(&app, post(CONTRIBUTORS, "intent=propose-changes:organization-008"), &session).await;

        let proposals = proposals_for(&state, "0801d").await;
        assert_eq!(proposals.len(), 1, "{proposals:?}");
        assert_eq!(
            proposals[0].entity_id, "organization-008",
            "the clicked row's entity, not whichever rendered first"
        );
    }

    #[tokio::test]
    async fn propose_changes_twice_for_one_entity_refuses_the_second_without_a_500() {
        let (state, _) = test_state("section-propose-changes-twice").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let body = "intent=propose-changes:organization-008";

        let first = as_session(&app, post(OVERVIEW, body), &session).await;
        assert_eq!(first.status(), StatusCode::OK);

        let second = as_session(&app, post(OVERVIEW, body), &session).await;
        assert_eq!(second.status(), StatusCode::OK, "a refusal, not a 500");
        assert!(body_string(second).await.contains("already has a change proposed"));

        let proposals = proposals_for(&state, "0801d").await;
        assert_eq!(proposals.len(), 1, "the second attempt created nothing: {proposals:?}");
    }

    #[tokio::test]
    async fn a_change_proposal_is_seeded_with_the_whole_published_entity_minus_its_id() {
        // Accepting a change proposal writes its payload as the entity file, so an unseeded
        // payload is data loss: `organization-001` carries a four-member `address` that no
        // organisation form would have to render for the member to survive. This is the property
        // `ProjectDraft` gives a project, applied to an entity.
        let (state, _) = test_state("section-propose-changes-seeded").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "intent=propose-changes:organization-001"), &session).await;

        let proposals = proposals_for(&state, "0801d").await;
        let payload: serde_json::Value =
            serde_json::from_str(&proposals[0].payload).expect("the seeded payload is JSON");
        assert_eq!(payload["name"], "Université de Lausanne");
        assert_eq!(payload["url"], "https://www.unil.ch/");
        assert_eq!(
            payload["address"]["locality"], "Lausanne",
            "a member no form has to render survives"
        );
        assert!(
            payload.get("id").is_none(),
            "the payload must not carry `id` — it lives in entity_id"
        );
        assert_eq!(proposals[0].entity_id, "organization-001");
    }

    /// Decision 5 on the issue, end to end.
    ///
    /// `organization-065` (Tanta University) is committed with no `postalCode`. A depositor
    /// proposing any other change to it must not be made to invent one — "all four members or
    /// omit `address`" applies to what they wrote, not to what they inherited. The same carve-out
    /// `typed_sentinels` already makes for a reference `url` because `0110_h-steiner` holds
    /// `MISSING`.
    ///
    /// This is the test that would have caught the carve-out shipping as dead code: it only passes
    /// once this layer can hand `check_organization` the published entity to compare against.
    #[tokio::test]
    async fn a_change_to_an_organisation_with_an_already_incomplete_address_can_be_submitted() {
        let (state, _) = test_state("section-propose-changes-grandfathered").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "intent=propose-changes:organization-065"), &session).await;

        let proposals = proposals_for(&state, "0801d").await;
        let payload: serde_json::Value = serde_json::from_str(&proposals[0].payload).expect("JSON");
        assert!(
            payload["address"]["postalCode"].as_str().unwrap_or_default().is_empty(),
            "the seed really is incomplete, or this test proves nothing: {payload}"
        );

        let response = as_session(&app, post(OVERVIEW, "intent=submit"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(
            !body.contains("has not been finished"),
            "an inherited incomplete address must not refuse the submission: {body}"
        );
    }

    #[tokio::test]
    async fn a_propose_intent_against_a_locked_project_is_refused() {
        let (state, _) = test_state("section-propose-locked").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        a_submission(&state, "0801d", user.id, SubmissionState::InReview).await;
        let app = test_app(&state);

        let response = as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("in review"));
        assert!(proposals_for(&state, "0801d").await.is_empty());
    }

    #[tokio::test]
    async fn submit_is_refused_when_a_live_proposal_has_findings_and_names_it() {
        let (state, _) = test_state("section-submit-proposal-findings").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // Started with an empty payload, which is what `propose-person` always creates — nothing
        // has filled it in yet, so `check_person` has plenty to find.
        as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;

        let response = as_session(&app, post(OVERVIEW, "intent=submit"), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("has not been finished"), "{body}");
        assert!(body.contains("person-417"), "names the proposal: {body}");
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d").await.unwrap(),
            None,
            "nothing was submitted"
        );
    }

    #[tokio::test]
    async fn submit_passes_once_the_proposals_payload_satisfies_its_checks() {
        let (state, _) = test_state("section-submit-proposal-satisfied").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // Alongside a genuine change to the draft: the project otherwise equals what is published,
        // and submit's own "nothing to review" gate would refuse it for that reason instead —
        // proving nothing about the proposal gate this test is for.
        as_session(&app, post(OVERVIEW, "name=Updated+Title&intent=propose-person"), &session).await;
        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("the proposal");
        EntityProposalRepository::update_payload(
            &*state.db,
            proposal.id,
            r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#,
            Utc::now(),
        )
        .await
        .unwrap();

        // A successful submit redirects on the plain path (POST-redirect-GET), unlike every
        // refusal above, which re-renders at `200` — so the status code alone is the first proof
        // the proposal gate let this through.
        let response = as_session(&app, post(OVERVIEW, "name=Updated+Title&intent=submit"), &session).await;
        assert_eq!(
            response.status(),
            StatusCode::SEE_OTHER,
            "a refusal would re-render at 200 instead"
        );
        assert_eq!(
            SubmissionRepository::find_by_shortcode(&*state.db, "0801d")
                .await
                .unwrap()
                .map(|submission| submission.state),
            Some(SubmissionState::Submitted)
        );
    }

    #[tokio::test]
    async fn a_just_allocated_id_resolves_in_the_picker_once_its_payload_is_filled_in() {
        // `payload: "{}"` is what `propose-person` always stores, and it does not deserialize into
        // a `Person` — so it cannot resolve yet, exactly like an entity form nobody has saved. This
        // fills it in directly through the repository, standing in for the entity form this chunk
        // does not build, and checks the plumbing this chunk does own: `AgentScope::with_proposals`
        // wired into the section context.
        let (state, _) = test_state("section-propose-resolves").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        as_session(&app, post(OVERVIEW, "intent=propose-person"), &session).await;
        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("the proposal");
        EntityProposalRepository::update_payload(
            &*state.db,
            proposal.id,
            r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#,
            Utc::now(),
        )
        .await
        .unwrap();

        // The summary panel renders only on a section holding an agent field, which "overview" is
        // not: `contactPoint`/`attributions` are "contributors".
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";
        let opened = body_string(as_session(&app, get(CONTRIBUTORS), &session).await).await;
        assert!(opened.contains("person-417"), "the summary names it: {opened}");

        // And the picker's own search finds it by the name that payload gives it, which is what
        // a depositor has to type to reference it. Searched through the picker's query name, the
        // same round trip the "Search" button posts.
        let searched = body_string(
            as_session(
                &app,
                post(
                    CONTRIBUTORS,
                    "attributions.row=r0&attributions.r0.contributor=&attributions.r0.contributor.q=Lovelace\
                     &intent=find-agent",
                ),
                &session,
            )
            .await,
        )
        .await;
        assert!(
            searched.contains(r#"value="person-417""#),
            "the search must offer the proposal it just allocated: {searched}"
        );

        // And a field naming it is no longer refused as unresolvable.
        as_session(
            &app,
            post(CONTRIBUTORS, "contactPoint.row=0&contactPoint.0=person-417"),
            &session,
        )
        .await;
        let refusal = as_session(&app, post(CONTRIBUTORS, "intent=submit"), &session).await;
        assert!(
            !body_string(refusal)
                .await
                .contains("is not a person or organisation the repository knows"),
            "person-417 must resolve now that it has a name"
        );
    }

    #[tokio::test]
    async fn the_proposals_summary_lists_a_live_proposal_and_is_absent_with_none() {
        // The summary is gated on a section holding an agent field — "contributors"
        // (`contactPoint`/`attributions`), not "overview".
        const CONTRIBUTORS: &str = "/projects/0801d/sections/contributors";
        let (state, _) = test_state("section-propose-summary").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let before = body_string(as_session(&app, get(CONTRIBUTORS), &session).await).await;
        assert!(
            !before.contains("Proposed persons and organisations"),
            "an empty panel reads as broken: {before}"
        );

        as_session(&app, post(OVERVIEW, "intent=propose-organization"), &session).await;

        let after = body_string(as_session(&app, get(CONTRIBUTORS), &session).await).await;
        assert!(after.contains("Proposed persons and organisations"), "{after}");
        // At `h2`, not `h3`. This panel renders above the section form, whose own title is an
        // `h2`, under the page's single `h1` — as an `h3` the outline ran 1 -> 3 -> 2, so a reader
        // navigating by heading level got a broken tree and one jumping from the `h1` to the next
        // `h2` skipped the panel although it comes first in reading order.
        assert!(
            after.contains(r#"<h2 class="font-display text-base mb-2">Proposed persons and organisations</h2>"#),
            "the summary heading must be an h2: {after}"
        );
        assert!(after.contains("organization-143"), "{after}");
        assert!(after.contains("Organisation"), "the kind is named: {after}");
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
            // The first differing line rather than two whole files: a failure
            // here names a place, and dumping 200 lines of Arabic description
            // to say a grant number moved is unreadable.
            if written != committed {
                let place = committed
                    .lines()
                    .zip(written.lines())
                    .enumerate()
                    .find(|(_, (before, after))| before != after)
                    .map_or_else(
                        || {
                            format!(
                                "every shared line matches; lengths differ: committed {} bytes, written {}",
                                committed.len(),
                                written.len()
                            )
                        },
                        |(line, (before, after))| {
                            format!("line {}:\n  committed: {before}\n  written:   {after}", line + 1)
                        },
                    );
                panic!("saving every untouched section rewrote {filename}\n{place}");
            }
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
                Some(Shape::StringList(_)) => {
                    for value in draft
                        .get(field.id)
                        .and_then(|value| value.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|value| value.as_str())
                    {
                        pairs.push((field.id.to_string(), value.to_string()));
                    }
                }
                Some(Shape::ReferenceRows(_)) => {
                    let rows = draft.get(field.id).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    if rows.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        let prefix = format!("{}.{key}", field.id);
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        for (member, name) in [("type", "type"), ("url", "url"), ("text", "label")] {
                            pairs.push((
                                format!("{prefix}.ref.{name}"),
                                row.get(member).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            ));
                        }
                    }
                }
                Some(Shape::PublicationRows) => {
                    let rows = draft.get(field.id).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    if rows.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        let prefix = format!("{}.{key}", field.id);
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        pairs.push((
                            format!("{prefix}.text"),
                            row.get("text").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        ));
                        pairs.push((
                            format!("{prefix}.pid"),
                            row.get("pid")
                                .and_then(|pid| pid.get("url"))
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        ));
                    }
                }
                Some(Shape::FundingRows) => {
                    // The discriminant is on the field, so both branches post once
                    // rather than per row.
                    let held = draft.get(field.id);
                    let is_grants = !held.is_some_and(|v| v.is_string());
                    pairs.push((
                        format!("{}.kind", field.id),
                        if is_grants { "grants" } else { "text" }.to_string(),
                    ));
                    pairs.push((
                        format!("{}.text", field.id),
                        held.and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    ));
                    let grants = held.and_then(|v| v.as_array()).cloned().unwrap_or_default();
                    if grants.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, grant) in grants.iter().enumerate() {
                        let key = format!("r{position}");
                        let prefix = format!("{}.{key}", field.id);
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        for id in grant
                            .get("funders")
                            .and_then(|v| v.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str())
                        {
                            pairs.push((format!("{prefix}.funder"), id.to_string()));
                        }
                        // The trailing blank funder control.
                        pairs.push((format!("{prefix}.funder"), String::new()));
                        for member in ["number", "name", "url"] {
                            pairs.push((
                                format!("{prefix}.{member}"),
                                grant.get(member).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            ));
                        }
                    }
                }
                Some(Shape::TextOrReferenceRows(_)) => {
                    // A row is a variant. Both branches are in the DOM and the
                    // inactive one is only `hidden`, so an untouched form posts
                    // *both* candidates plus the discriminant — which is exactly
                    // what the applier must narrow with the discriminant alone.
                    let rows = draft
                        .get(field.id)
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if rows.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        let prefix = format!("{}.{key}", field.id);
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        let is_reference = row.get("url").is_some();
                        pairs.push((
                            format!("{prefix}.kind"),
                            if is_reference { "reference" } else { "text" }.to_string(),
                        ));
                        // The reference branch, empty on a text row.
                        for (member, name) in [("type", "type"), ("url", "url"), ("text", "label")] {
                            pairs.push((
                                format!("{prefix}.ref.{name}"),
                                row.get(member).and_then(|value| value.as_str()).unwrap_or_default().to_string(),
                            ));
                        }
                        // The text branch, empty on a reference row. A control per
                        // offered language plus whatever tags the value carries.
                        let texts = if is_reference {
                            editor_core::multilingual::DraftMultilingual::default()
                        } else {
                            editor_web::form::widgets::as_multilingual(row)
                        };
                        for tag in UI_LANGUAGES.iter().copied().chain(texts.extra_tags()) {
                            pairs
                                .push((format!("{prefix}.text.{tag}"), texts.get(tag).unwrap_or_default().to_string()));
                        }
                    }
                }
                Some(Shape::AttributionRows) => {
                    let rows = draft
                        .get(field.id)
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if rows.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        pairs.push((
                            format!("{}.{key}.contributor", field.id),
                            row.get("contributor").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        ));
                        for role in row
                            .get("contributorType")
                            .and_then(|v| v.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str())
                        {
                            pairs.push((format!("{}.{key}.role", field.id), role.to_string()));
                        }
                        pairs.push((format!("{}.{key}.role", field.id), String::new()));
                    }
                }
                Some(Shape::AgentRows | Shape::StringRows) => {
                    let rows = draft
                        .get(field.id)
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if rows.is_empty() {
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        pairs.push((format!("{}.{key}", field.id), row.as_str().unwrap_or_default().to_string()));
                    }
                }
                Some(Shape::MultilingualRows) => {
                    let rows = draft
                        .get(field.id)
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if rows.is_empty() {
                        // The empty marker, without which the field reads as
                        // absent and the last removal would not stick.
                        pairs.push((format!("{}.row", field.id), String::new()));
                    }
                    for (position, row) in rows.iter().enumerate() {
                        let key = format!("r{position}");
                        pairs.push((format!("{}.row", field.id), key.clone()));
                        let stored = editor_web::form::widgets::as_multilingual(row);
                        for tag in UI_LANGUAGES.iter().copied().chain(stored.extra_tags()) {
                            pairs.push((
                                format!("{}.{key}.{tag}", field.id),
                                stored.get(tag).unwrap_or_default().to_string(),
                            ));
                        }
                    }
                }
                Some(Shape::Url(slot)) => {
                    let rendered = draft
                        .url_slot(slot)
                        .filter(|text| !platform_metadata::is_placeholder(text))
                        .unwrap_or_default();
                    pairs.push((field.id.to_string(), rendered.to_string()));
                }
                // Only where the project holds one: a radio group with nothing
                // checked and a `<select>` on a project with no value both
                // submit no name, which is what leaves the field alone.
                Some(Shape::Choice(_)) => {
                    if let Some(value) = draft.get(field.id).and_then(|value| value.as_str()) {
                        pairs.push((field.id.to_string(), value.to_string()));
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

    #[tokio::test]
    async fn an_approved_change_not_yet_published_says_it_is_waiting_for_a_release() {
        // REQ-2.5. Informational, not a lock: approve is the only outcome that
        // does not hand the project back, so the form has to stay editable.
        let (state, _) = test_state("section-awaiting-release").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let published = state.published.get("0801d").expect("a published fixture");
        let mut approved = ProjectDraft::from_raw(published);
        approved.set("name", serde_json::json!("What RDU Approved"));
        ApprovedRecordRepository::create(
            &*state.db,
            &editor_core::records::ApprovedRecord {
                id: uuid::Uuid::new_v4(),
                shortcode: "0801d".to_string(),
                payload: serde_json::to_string(&approved).expect("serializes"),
                approved_by: Some(user.id),
                approved_at: chrono::Utc::now(),
                collected_at: None,
            },
        )
        .await
        .expect("create");

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;

        assert!(
            body.contains("Waiting for the next release"),
            "REQ-2.5 notice is missing: {body}"
        );
        assert!(body.contains("few weeks"), "REQ-2.6 wait must be stated with it: {body}");
        assert!(
            body.contains("Save draft"),
            "the form must stay editable after an approval: {body}"
        );
    }

    #[tokio::test]
    async fn an_approved_change_already_published_does_not_claim_to_be_waiting() {
        // The same record, but matching published data: it is Online, and the
        // startup pass will discard it. Saying "waiting" here would be the
        // stale label this phase exists to remove.
        let (state, _) = test_state("section-awaiting-online").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let published = state.published.get("0801d").expect("a published fixture");
        ApprovedRecordRepository::create(
            &*state.db,
            &editor_core::records::ApprovedRecord {
                id: uuid::Uuid::new_v4(),
                shortcode: "0801d".to_string(),
                payload: serde_json::to_string(&ProjectDraft::from_raw(published)).expect("serializes"),
                approved_by: Some(user.id),
                approved_at: chrono::Utc::now(),
                collected_at: Some(chrono::Utc::now()),
            },
        )
        .await
        .expect("create");

        let body = body_string(as_session(&app, get(OVERVIEW), &session).await).await;

        assert!(!body.contains("Waiting for the next release"), "already published: {body}");
    }
}
