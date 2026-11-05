//! The entity form: `GET` and `POST /projects/{shortcode}/entities/{proposal}`.
//!
//! `{proposal}` is a proposal's **`entity_id`** (`person-417`, `organization-143`), not its row
//! `id` — the same value `pages::section` links to and the propose-changes intent carries
//! (`propose-changes:{field}:{entity_id}`). A project may hold more than one row for one
//! `entity_id` over time (a withdrawn proposal, then a fresh one for the same entity), but never
//! more than one *live* one (`entity_proposals_live_per_entity`); [`proposal_for`] picks the row a
//! link resolves to.
//!
//! Same shape as `sections.rs`: [`context`] resolves a request in the order that module's own
//! docs argue for — shape, then authorization, then anything that reads state — and a proposal
//! under the wrong shortcode is a 404 for the same reason an unknown section id is: the reader
//! invented the pairing, not an assigned-project question a 403 would answer.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::Utc;
use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::registry::{first_section, Field};
use editor_core::form::submit::drop_agent;
use editor_core::form::{apply, FormBody, Shape, WhenCleared};
use editor_core::proposals::{proposal_for, EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
use editor_core::records::normalize_shortcode;
use editor_core::repository::{DraftRewrite, EntityProposalRepository, RepositoryError};
use serde_json::Value;
use shared_metadata::is_valid_shortcode;

use crate::entity as page;
use crate::entity::origin::{entity_url, Origin};
use crate::form::widgets::apply_picker_intents;
use crate::form::INTENT;
use crate::handlers::auth::{Account, Authenticated};
use crate::handlers::sections::audience_of;
use crate::handlers::{
    agent_search_response, is_enhanced, navigate, picker_root, storage_error, SearchParams, StorageSurface,
};
use crate::pages::section::{changed_agent, keeps_posted, FIND_AGENT};
use crate::shell::EditorState;

const SAVE_REFUSED_NOT_LIVE: &str = "This proposal is no longer open, so it cannot be changed.";
const SAVE_REFUSED_STORAGE: &str = "Could not be saved. Nothing was changed — try again, and if it keeps happening \
                                    the service needs attention.";
const DISCARD_REFUSED_NOT_LIVE: &str = "This proposal is no longer open, so there is nothing to discard.";
const DISCARD_REFUSED_STORAGE: &str = "Could not be discarded. Try again, and if it keeps happening the service \
                                       needs attention.";

/// Everything both handlers need, once the request is known to be allowed.
struct Context<'a> {
    proposal: EntityProposal,
    /// The proposed entity, read from [`EntityProposal::payload`].
    ///
    /// [`ProjectDraft`] is reused here as a plain JSON-object accessor rather than a second type:
    /// despite its name, `get`/`set`/`remove`/`multilingual` touch nothing project-specific.
    /// Reusing it is what lets this form reuse `editor_core::form::apply`'s appliers and
    /// `crate::form::widgets`'s row and multilingual composers verbatim, instead of a second
    /// implementation of row-key bookkeeping and empty/unchanged handling built for entities
    /// alone.
    draft: ProjectDraft,
    agents: AgentScope<'a>,
    posted: Option<&'a FormBody>,
    /// The section the proposal was opened from: the `GET`'s query, or a `POST`'s hidden inputs.
    origin: Option<Origin>,
}

/// Resolve a request, or the response that refuses it. See the module docs for the ordering.
async fn context<'a>(
    state: &'a EditorState,
    user: &Account,
    shortcode: &str,
    entity_id: &str,
) -> Result<Context<'a>, Response> {
    if !is_valid_shortcode(shortcode) {
        return Err(crate::shell::not_found(State(state.clone())).await);
    }
    if !user.actor.may_reach(shortcode) {
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused an entity form for a project that is not assigned to this account"
        );
        return Err(crate::shell::forbidden(state, user, crate::handlers::projects::NOT_ASSIGNED));
    }

    let key = normalize_shortcode(shortcode);
    let proposals = match EntityProposalRepository::list_for_shortcode(&*state.editor, &key).await {
        Ok(proposals) => proposals,
        Err(error) => {
            return Err(storage_error(
                state,
                user,
                SURFACE,
                "read this project's entity proposals",
                &error,
            ))
        }
    };
    // The reader invented the pairing if this comes back empty — a proposal id under the wrong
    // shortcode is exactly that, so it is a 404 rather than a 403.
    let Some(proposal) = proposal_for(&proposals, entity_id) else {
        return Err(crate::shell::not_found(State(state.clone())).await);
    };

    let draft = serde_json::from_str(&proposal.payload).unwrap_or_else(|error| {
        // Read as empty rather than falling back to something else: there is
        // no published counterpart for a `New` proposal to fall back to, and
        // a `Change` one's seed is already what is stored here — the same
        // reasoning `sections.rs::context` gives for an unparsable draft.
        tracing::error!(
            error = %error,
            proposal.entity_id = %proposal.entity_id,
            "a stored entity payload could not be parsed"
        );
        ProjectDraft::default()
    });
    let agents = AgentScope::with_proposals(&state.agents, &proposals);

    Ok(Context { proposal, draft, agents, posted: None, origin: None })
}

/// Whether this `GET` follows a plain-path save, as `sections.rs::ShowParams` does for a section,
/// and which section it was opened from ([`Origin`]).
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ShowParams {
    saved: Option<String>,
    section: Option<String>,
    field: Option<String>,
}

/// `GET /projects/{shortcode}/entities/{proposal}`.
pub(crate) async fn show(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path((shortcode, entity_id)): Path<(String, String)>,
    Query(params): Query<ShowParams>,
) -> Response {
    let mut context = match context(&state, &user, &shortcode, &entity_id).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    context.origin = Origin::parse(audience_of(&user), params.section.as_deref(), params.field.as_deref());
    let notice = params
        .saved
        .as_deref()
        .filter(|saved| *saved == context.proposal.updated_at.to_rfc3339())
        .map(|_| page::Notice::Saved);
    render_page(
        &state,
        &user,
        &shortcode,
        &context,
        Rendering { notice, ..Rendering::default() },
    )
}

/// The field a search's `for` names, when it is a picker name this proposal's form renders: a
/// [`picker_root`] that is one of its kind's fields and whose shape has an agent picker.
fn picker_field(context: &Context<'_>, name: &str) -> Option<&'static Field> {
    let root = picker_root(name)?;
    page::fields_for(context.proposal.kind)
        .iter()
        .find(|field| field.id == root)
        .filter(|field| field.shape.is_some_and(Shape::has_agent_picker))
}

/// `GET /projects/{shortcode}/entities/{proposal}/agents?for=<picker>&q=<query>`: the results of
/// one type-ahead search, as the fragment the combobox's results container is morphed to.
///
/// Reads and never writes, like `sections.rs::agent_search`, whose answer-200-for-every-`q`
/// contract it keeps. The access checks are [`show`]'s, through [`context`]. No propose buttons:
/// this form dispatches none of the propose intents.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "entity agent search",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.entity = tracing::field::Empty,
        form.picker = tracing::field::Empty,
    )
)]
pub(crate) async fn agent_search(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path((shortcode, entity_id)): Path<(String, String)>,
    Query(params): Query<SearchParams>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(&shortcode));
    span.record("form.entity", tracing::field::display(&entity_id));

    let context = match context(&state, &user, &shortcode, &entity_id).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    let Some((unique, field)) = params
        .r#for
        .as_deref()
        .and_then(|unique| picker_field(&context, unique).map(|field| (unique, field)))
    else {
        return crate::shell::not_found(State(state.clone())).await;
    };
    span.record("form.picker", tracing::field::display(unique));

    agent_search_response(unique, field, &context.draft, &context.agents, params.q.as_deref(), false)
}

/// `POST /projects/{shortcode}/entities/{proposal}` — save the proposal, discard it, or re-render
/// it for an agent picker's find or change.
///
/// A choice, an append and a role removal save like any other write; on the enhanced path they
/// keep the posted body in the answer (see [`Rendering::keep_posted`]).
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "entity form action",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.entity = tracing::field::Empty,
        form.intent = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn act(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path((shortcode, entity_id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(&shortcode));
    span.record("form.entity", tracing::field::display(&entity_id));

    let mut context = match context(&state, &user, &shortcode, &entity_id).await {
        Ok(context) => context,
        Err(response) => {
            span.record("form.outcome", "refused");
            return response;
        }
    };
    // After the context, which says which kind's fields name the shape an appended agent goes into.
    let body = FormBody::from_pairs(apply_picker_intents(pairs, |name| {
        page::fields_for(context.proposal.kind)
            .iter()
            .find(|field| field.id == name)
            .and_then(|field| field.shape)
    }));
    context.posted = Some(&body);
    context.origin = Origin::posted(audience_of(&user), &body);

    // Anything this build does not know falls back to Save, matching
    // `sections.rs::act`'s own fallback: discarding is not undoable by the
    // depositor, and saving is.
    let intent = match body.get(INTENT) {
        Some(page::DISCARD) => Intent::Discard,
        Some(page::DISCARD_CONFIRM) => Intent::ConfirmDiscard,
        Some(FIND_AGENT) => Intent::FindAgent,
        // Opening a card's search is a re-render like a search: a redirect would lose it.
        Some(value) if changed_agent(value).is_some() => Intent::FindAgent,
        _ => Intent::Save,
    };
    span.record("form.intent", tracing::field::display(intent.as_str()));
    // A picker's choice, an appended agent and a removed role keep the posted body on the enhanced
    // path, as the rest of the form may hold a row that is not storable yet.
    let keep_posted = body.get(INTENT).is_some_and(keeps_posted);

    if intent == Intent::Discard || intent == Intent::ConfirmDiscard {
        return discard(&state, &user, &shortcode, &context, headers, intent).await;
    }

    // Re-checked here, not only when the form was rendered and the Save
    // control was or was not offered: the render is a `GET`, so nothing stops
    // a `POST` arriving without one, or arriving after RDU decided the
    // proposal in the meantime.
    if !context.proposal.is_live() {
        span.record("form.outcome", "not_live");
        return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_NOT_LIVE);
    }

    apply_posted(&mut context.draft, context.proposal.kind, &body);
    let payload = serde_json::to_string(&context.draft).unwrap_or_default();

    // One `Utc::now()`, reused as the marker `saved` redirects with: the marker must carry the
    // stamp that landed, and a second call would not match it.
    let now = Utc::now();
    match EntityProposalRepository::update_payload(&*state.editor, context.proposal.id, &payload, now).await {
        Ok(()) => {
            span.record(
                "form.outcome",
                if intent == Intent::FindAgent {
                    "searched"
                } else {
                    "saved"
                },
            );
            tracing::info!("saved an entity proposal");
            context.proposal.updated_at = now;
            // A search stores the body exactly as a save does, then re-renders it with the query
            // still in the box: `saved` redirects on the plain path, which would drop it.
            if intent == Intent::FindAgent {
                let rendering = Rendering { keep_posted: true, ..Rendering::default() };
                return if is_enhanced(&headers) {
                    region(&shortcode, &context, rendering)
                } else {
                    render_page(&state, &user, &shortcode, &context, rendering)
                };
            }
            saved(&shortcode, &context, headers, keep_posted)
        }
        // Decided since `context` read it: the store refuses the write.
        Err(RepositoryError::NotFound { .. }) => {
            span.record("form.outcome", "not_live");
            refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_NOT_LIVE)
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not save an entity proposal");
            refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_STORAGE)
        }
    }
}

/// `POST /projects/{shortcode}/entities/{proposal}/fields/{field}/add` — one more blank row.
pub(crate) async fn add_row(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path((shortcode, entity_id, field_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(&state, &user, &shortcode, &entity_id, &field_id, headers, pairs, None).await
}

/// `POST /projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove` — drop one row.
pub(crate) async fn remove_row(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path((shortcode, entity_id, field_id, key)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(&state, &user, &shortcode, &entity_id, &field_id, headers, pairs, Some(&key)).await
}

/// Add or remove a row: the two differ only in what they do to the posted body, exactly the way
/// `sections.rs::row_action` argues for — removing drops the row's key from `{field}.row` before
/// anything is applied, so the save that follows simply does not see it.
#[allow(clippy::too_many_arguments)]
async fn row_action(
    state: &EditorState,
    user: &Account,
    shortcode: &str,
    entity_id: &str,
    field_id: &str,
    headers: HeaderMap,
    pairs: Vec<(String, String)>,
    removing: Option<&str>,
) -> Response {
    let body = FormBody::from_pairs(match removing {
        Some(key) => FormBody::pairs_without_row(pairs, field_id, key),
        None => pairs,
    });

    let mut context = match context(state, user, shortcode, entity_id).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    context.posted = Some(&body);
    context.origin = Origin::posted(audience_of(user), &body);

    // The field has to be one this proposal's kind actually renders, checked
    // against the same list the render side reads — an id naming another
    // field, or one this kind does not have, is a way to write outside the
    // contract this form declares.
    let Some(field) = crate::entity::fields_for(context.proposal.kind).iter().find(|field| {
        field.id == field_id
            && matches!(
                field.shape,
                Some(Shape::StringRows | Shape::AgentRows | Shape::ReferenceRows(_))
            )
    }) else {
        return crate::shell::not_found(State(state.clone())).await;
    };

    if !context.proposal.is_live() {
        return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_NOT_LIVE);
    }

    apply_posted(&mut context.draft, context.proposal.kind, &body);
    let payload = serde_json::to_string(&context.draft).unwrap_or_default();
    match EntityProposalRepository::update_payload(&*state.editor, context.proposal.id, &payload, Utc::now()).await {
        Ok(()) => {}
        Err(RepositoryError::NotFound { .. }) => {
            return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_NOT_LIVE);
        }
        Err(error) => {
            tracing::error!(error = %error, "could not save an entity proposal row action");
            return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_STORAGE);
        }
    }

    let rendering = Rendering {
        adding_row: removing.is_none().then_some(field.id),
        keep_posted: true,
        ..Rendering::default()
    };
    if is_enhanced(&headers) {
        region(shortcode, &context, rendering)
    } else {
        render_page(state, user, shortcode, &context, rendering)
    }
}

/// Discard the proposal, or show the confirmation that posts it.
async fn discard(
    state: &EditorState,
    user: &Account,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    intent: Intent,
) -> Response {
    // Checked before the confirmation step as well as before the write: a
    // proposal RDU decided while this page was open must not be offered a
    // discard that would then refuse, and the confirmation is not the write
    // itself.
    if !context.proposal.is_live() {
        return refused(state, user, shortcode, context, headers, DISCARD_REFUSED_NOT_LIVE);
    }

    if intent == Intent::ConfirmDiscard {
        return confirming(state, user, shortcode, context, headers);
    }

    match withdraw(state, user, context).await {
        Ok(()) => {
            tracing::info!(proposal.entity_id = %context.proposal.entity_id, "withdrew an entity proposal");
            // The plain path redirects, so a refresh re-issues the section's `GET` rather than
            // re-posting a discard that would now refuse.
            let target = discarded_target(user, shortcode, context);
            if is_enhanced(&headers) {
                return navigate(&target);
            }
            Redirect::to(&target).into_response()
        }
        Err(error) => {
            tracing::error!(error = %error, "could not withdraw an entity proposal");
            refused(state, user, shortcode, context, headers, DISCARD_REFUSED_STORAGE)
        }
    }
}

/// Withdraw the proposal. A `New` one also leaves the project's draft in the same write: its id
/// now resolves to nobody, so every row naming it would render "not found" and refuse the submit.
async fn withdraw(state: &EditorState, user: &Account, context: &Context<'_>) -> Result<(), RepositoryError> {
    let now = Utc::now();
    if context.proposal.operation == ProposalOperation::Change {
        return EntityProposalRepository::withdraw(&*state.editor, context.proposal.id, now).await;
    }
    let entity_id = context.proposal.entity_id.clone();
    let rewrite: DraftRewrite = Box::new(move |payload| {
        let mut draft: ProjectDraft = serde_json::from_str(payload).ok()?;
        (drop_agent(&mut draft, &entity_id) > 0)
            .then(|| serde_json::to_string(&draft).ok())
            .flatten()
    });
    EntityProposalRepository::withdraw_rewriting_draft(&*state.editor, context.proposal.id, now, Some(user.id), rewrite)
        .await
}

/// The section a discard returns to, carrying the marker `sections.rs::show` confirms it by. At
/// its top rather than at the card: the confirmation is in the status region there, which scrolls
/// away below `md`. With no origin, the project's first section.
fn discarded_target(user: &Account, shortcode: &str, context: &Context<'_>) -> String {
    let origin = context.origin.unwrap_or(Origin::section(first_section(audience_of(user))));
    let marker = format!("{DISCARDED_PROPOSAL}={}", context.proposal.entity_id);
    origin.section_url(shortcode, Some(&marker), None)
}

/// The query parameter naming the proposal a discard withdrew, read back by `sections.rs::show`.
pub(crate) const DISCARDED_PROPOSAL: &str = "discarded_proposal";

/// What a `POST` to this route is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Save,
    Discard,
    ConfirmDiscard,
    FindAgent,
}

impl Intent {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Discard => "discard",
            // The same spelling `sections.rs`'s own `ConfirmDiscard` uses, because a dashboard
            // filtering `form.intent = "discard-confirm"` to count discard confirmations across
            // both forms would otherwise silently miss every one from this route.
            Self::ConfirmDiscard => page::DISCARD_CONFIRM,
            Self::FindAgent => "find-agent",
        }
    }
}

/// Merge one applier per field the entity's kind renders into the stored
/// payload. Never rebuilds it: a field this form does not declare is simply
/// never named in an `apply` call, so it survives untouched — the same
/// property `ProjectDraft` gives a project, applied to an entity.
fn apply_posted(draft: &mut ProjectDraft, kind: ProposalKind, body: &FormBody) {
    match kind {
        ProposalKind::Person => apply_person(body, draft),
        ProposalKind::Organization => apply_organization(body, draft),
    }
    // No control on this form posts `id`, so this should already be a no-op —
    // but `EntityProposal::payload` must never carry one (its own docs say
    // why: `entity_id` is the column the allocator and the uniqueness index
    // work on), so it is stripped again here rather than trusted to have
    // stayed absent.
    draft.remove("id");
}

fn apply_person(body: &FormBody, draft: &mut ProjectDraft) {
    apply(Shape::StringRows, body, draft, "givenNames");
    apply(Shape::StringRows, body, draft, "familyNames");
    apply(Shape::StringRows, body, draft, "jobTitles");
    keep_job_titles_present(body, draft);
    apply(Shape::AgentRows, body, draft, "affiliations");
    apply(Shape::ReferenceRows(page::SAME_AS_TYPES), body, draft, "sameAs");
    apply(Shape::Text(WhenCleared::Drop), body, draft, "email");
}

/// `jobTitles` must survive a save as `[]`, never disappear, once this form
/// has rendered it — `check_person` reads an *absent* member as unanswered
/// and an *empty* one as a person with no job title, which many committed
/// persons already are. `apply(Shape::StringRows, …)` cannot tell the two
/// apart on its own: it removes the field whenever no row survives, which is
/// right for every field of this shape except this one.
fn keep_job_titles_present(body: &FormBody, draft: &mut ProjectDraft) {
    if body.has("jobTitles.row") && draft.get("jobTitles").is_none() {
        draft.set("jobTitles", Value::Array(Vec::new()));
    }
}

fn apply_organization(body: &FormBody, draft: &mut ProjectDraft) {
    apply(Shape::Text(WhenCleared::Drop), body, draft, "name");
    apply(Shape::Text(WhenCleared::Drop), body, draft, "url");
    apply(Shape::Multilingual, body, draft, "alternativeName");
    for member in page::ADDRESS_MEMBERS {
        apply(Shape::Text(WhenCleared::Drop), body, draft, &format!("address.{}", member.name));
    }
    // A canton belongs to a Swiss address only. The form always posts it, so
    // this is what clears one left behind by a country that was changed.
    let takes_canton = draft
        .get("address.country")
        .and_then(Value::as_str)
        .is_some_and(editor_core::countries::takes_canton);
    if !takes_canton {
        draft.remove("address.canton");
    }
    // All four of `street`, `postalCode`, `locality` and `country`,
    // or no `address` at all. Each member above already drops itself when
    // cleared; what is left is dropping the container once every member it
    // ever held is gone — `ProjectDraft::remove`'s own docs say a dotted
    // remove leaves an emptied parent in place, on purpose, so a sibling
    // member is not taken with it. Nothing here has a sibling to protect: an
    // organisation has no other member nested under `address`.
    if draft
        .get("address")
        .and_then(Value::as_object)
        .is_some_and(serde_json::Map::is_empty)
    {
        draft.remove("address");
    }
    apply(Shape::ReferenceRows(page::SAME_AS_TYPES), body, draft, "sameAs");
    apply(Shape::Text(WhenCleared::Drop), body, draft, "email");
}

/// Whether this reader is offered the form or a read-only value, and why.
fn over_of(status: ProposalStatus) -> Option<page::Over> {
    match status {
        // Both are live (`EntityProposal::is_live`): a submitted proposal is
        // still the depositor's to finish, unlike a submitted project draft.
        ProposalStatus::Draft | ProposalStatus::Submitted => None,
        ProposalStatus::Accepted => Some(page::Over::Accepted),
        ProposalStatus::Rejected | ProposalStatus::Withdrawn => Some(page::Over::Terminal),
    }
}

/// A successful save: a redirect on the plain path, the patched region on the enhanced one.
/// `act` sets `updated_at` to the stored stamp before calling this, so the marker matches the row.
/// `keep_posted` is for the enhanced path only, see [`Rendering::keep_posted`].
fn saved(shortcode: &str, context: &Context<'_>, headers: HeaderMap, keep_posted: bool) -> Response {
    if !is_enhanced(&headers) {
        return redirect_saved(shortcode, context);
    }
    region(
        shortcode,
        context,
        Rendering {
            notice: Some(page::Notice::Saved),
            keep_posted,
            ..Rendering::default()
        },
    )
}

fn confirming(
    state: &EditorState,
    user: &Account,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
) -> Response {
    let rendering = Rendering {
        confirming: Some(page::Confirmation::Discard),
        ..Rendering::default()
    };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

/// Redirect to the entity form after a save, carrying the stored proposal's `updated_at` for the
/// reloaded `GET` ([`ShowParams`]) and the origin, so "Back to the project" still returns there.
fn redirect_saved(shortcode: &str, context: &Context<'_>) -> Response {
    let url = entity_url(shortcode, &context.proposal.entity_id, context.origin);
    let separator = if context.origin.is_some() { '&' } else { '?' };
    let saved = urlencoding::encode(&context.proposal.updated_at.to_rfc3339()).into_owned();
    Redirect::to(&format!("{url}{separator}saved={saved}")).into_response()
}

fn refused(
    state: &EditorState,
    user: &Account,
    shortcode: &str,
    context: &Context<'_>,
    headers: HeaderMap,
    message: &str,
) -> Response {
    let rendering = Rendering {
        notice: Some(page::Notice::Refused(message)),
        confirming: None,
        adding_row: None,
        keep_posted: true,
    };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

/// What a rendering adds on top of the resolved request. A struct for the
/// reason `sections.rs::Rendering` is one: several of these are `Option`s of
/// similar shape, and a positional list lets a call site scramble them
/// silently.
#[derive(Default)]
struct Rendering<'a> {
    notice: Option<page::Notice<'a>>,
    confirming: Option<page::Confirmation>,
    adding_row: Option<&'a str>,
    /// Whether this render must preserve the posted editing state — set for a
    /// refusal, a row action and a find or a change (which re-render the body, query and
    /// open search included), and for a picker's choice, an append or a role removal on the
    /// enhanced path of a save. Deliberately not for a plain save, matching
    /// `sections.rs::Rendering::keep_posted`'s own reasoning.
    keep_posted: bool,
}

fn region(shortcode: &str, context: &Context<'_>, rendering: Rendering<'_>) -> Response {
    let view = view(shortcode, context, rendering);
    (StatusCode::OK, axum::response::Html(crate::entity::region(&view).into_string())).into_response()
}

fn render_page(
    state: &EditorState,
    user: &Account,
    shortcode: &str,
    context: &Context<'_>,
    rendering: Rendering<'_>,
) -> Response {
    // Read before `rendering` moves into `view`; see `sections.rs::render_page`.
    let prefix = rendering.notice.and_then(|notice| notice.title_prefix());
    let label = crate::entity::entity_label(
        context.proposal.kind,
        &context.proposal.entity_id,
        Some(&context.agents),
        crate::entity::Precedence::Proposed,
    );
    let title = format!("{label} — DaSCH Metadata Editor");
    let title = match prefix {
        Some(prefix) => format!("{prefix} — {title}"),
        None => title,
    };
    let view = view(shortcode, context, rendering);
    crate::shell::render(state, &title, StatusCode::OK, Some(user), crate::entity::page(&view))
}

fn view<'a>(shortcode: &'a str, context: &'a Context<'a>, rendering: Rendering<'a>) -> crate::entity::EntityView<'a> {
    let posted = rendering.keep_posted.then_some(context.posted).flatten();
    // From the stored payload at every render, as the section form does, so a
    // save's redirect and a refusal's re-render show the same findings.
    let fields = crate::entity::fields_for(context.proposal.kind);
    let format_findings = editor_core::form::format::findings(fields.iter(), &context.draft, posted, None);
    crate::entity::EntityView {
        shortcode,
        proposal: &context.proposal,
        over: over_of(context.proposal.status),
        draft: &context.draft,
        confirming: rendering.confirming,
        notice: rendering.notice,
        posted,
        adding_row: rendering.adding_row,
        agents: Some(&context.agents),
        format_findings,
        rows_action: format!("/projects/{shortcode}/entities/{}/fields", context.proposal.entity_id),
        origin: context.origin,
    }
}

const SURFACE: StorageSurface = StorageSurface { log: "the entity form could not reach storage", noun: "form" };

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use editor_core::repository::DraftRepository;
    use editor_ports::Role;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::fixtures::{
        a_session, a_user, body_string, get, location, post, test_app, test_state, title_of, with_cookie,
    };

    fn enhanced(uri: &str, form: &str) -> Request<Body> {
        let mut request = post(uri, form);
        request
            .headers_mut()
            .insert(crate::handlers::DATASTAR_REQUEST, "true".parse().expect("a header value"));
        request
    }

    async fn as_session(app: &axum::Router, request: Request<Body>, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(request, crate::fixtures::SESSION, session))
            .await
            .expect("the request should complete")
    }

    /// Every proposal stored for `shortcode`, for asserting on what a save or discard wrote —
    /// mirrors `sections.rs::tests::proposals_for`.
    async fn proposals_for(state: &EditorState, shortcode: &str) -> Vec<EntityProposal> {
        EntityProposalRepository::list_for_shortcode(&*state.editor, shortcode)
            .await
            .expect("proposals should read")
    }

    /// Start a proposal the way a depositor would — through the section form's own propose
    /// control — and return its entity id. This route never creates one; that is `sections.rs`'s
    /// job, so every test here needs a proposal already in hand before it can open this form.
    async fn a_proposal(
        app: &axum::Router,
        state: &EditorState,
        session: &str,
        shortcode: &str,
        intent: &str,
    ) -> String {
        let uri = format!("/projects/{shortcode}/sections/overview");
        as_session(app, post(&uri, &format!("intent={intent}")), session).await;
        proposals_for(state, shortcode)
            .await
            .into_iter()
            .next()
            .expect("a proposal")
            .entity_id
    }

    async fn a_person_proposal(app: &axum::Router, state: &EditorState, session: &str, shortcode: &str) -> String {
        a_proposal(app, state, session, shortcode, "propose-person").await
    }

    async fn an_organization_proposal(
        app: &axum::Router,
        state: &EditorState,
        session: &str,
        shortcode: &str,
    ) -> String {
        a_proposal(app, state, session, shortcode, "propose-organization").await
    }

    async fn stored_payload(state: &EditorState, shortcode: &str) -> Value {
        let proposal = proposals_for(state, shortcode).await.into_iter().next().expect("a proposal");
        serde_json::from_str(&proposal.payload).expect("the stored payload is JSON")
    }

    #[tokio::test]
    async fn a_depositor_opens_the_person_form() {
        let (state, _) = test_state("entity-open-person").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let response = as_session(&app, get(&format!("/projects/0801d/entities/{entity_id}")), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("Given names"), "{body}");
        assert!(body.contains("Job titles"), "{body}");
        assert!(body.contains(r#"name="givenNames.row""#), "{body}");
    }

    #[tokio::test]
    async fn the_person_form_labels_each_text_row_by_what_it_holds() {
        let (state, _) = test_state("entity-person-row-labels").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        // A fresh proposal renders no rows, so store one value per field first.
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");
        let typed = "givenNames.row=r0&givenNames.r0=Ada&familyNames.row=r0&familyNames.r0=Lovelace\
                     &jobTitles.row=r0&jobTitles.r0=Archivist";
        as_session(&app, post(&uri, typed), &session).await;
        let response = as_session(&app, get(&uri), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;

        assert!(!body.contains("Link 1"), "{body}");
        for (field, label) in [
            ("givenNames", "Given name 1"),
            ("familyNames", "Family name 1"),
            ("jobTitles", "Job title 1"),
        ] {
            let id = format!("{field}.r0");
            let labelled = format!(r#"<label class="field-label sr-only" for="{id}">{label}</label>"#);
            assert!(body.contains(&labelled), "{label} labels {id}: {body}");
            assert!(body.contains(&format!(r#"id="{id}""#)), "{id} is an input: {body}");
        }
    }

    #[tokio::test]
    async fn a_depositor_opens_the_organisation_form() {
        let (state, _) = test_state("entity-open-organization").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let response = as_session(&app, get(&format!("/projects/0801d/entities/{entity_id}")), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains(r#"name="name""#), "{body}");
        assert!(body.contains("Address"), "{body}");
        assert!(body.contains("Alternative name"), "{body}");
    }

    #[tokio::test]
    async fn the_address_rule_is_announced_at_the_address_controls() {
        // `ADDRESS_HINT` is the only place the "all four together or none" rule is stated.
        // As a bare sibling paragraph it was part of no control's accessible description, so a
        // reader tabbing onto Street heard "Street, edit text" and nothing about the rule whose
        // breach `check_organization` then refuses — the failure the codebase's own accessibility
        // decision about `aria-describedby` records, applied to the one group that bypassed the
        // shared shell.
        let (state, _) = test_state("entity-address-described-by").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let response = as_session(&app, get(&format!("/projects/0801d/entities/{entity_id}")), &session).await;
        let body = body_string(response).await;

        assert!(
            body.contains(r#"aria-describedby="address-hint""#),
            "the fieldset must point at the hint: {body}"
        );
        assert!(body.contains(r#"id="address-hint""#), "the hint must carry the id: {body}");
    }

    #[tokio::test]
    async fn a_proposal_under_another_shortcode_is_a_404() {
        // The reader invented the pairing, so this is a 404 rather than a 403 — the same argument
        // `sections.rs::context` makes for an unknown section id.
        let (state, _) = test_state("entity-wrong-shortcode").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d", "0803"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let response = as_session(&app, get(&format!("/projects/0803/entities/{entity_id}")), &session).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_reader_who_cannot_reach_the_project_is_refused() {
        let (state, _) = test_state("entity-forbidden").await;
        let owner = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let owner_session = a_session(&state, owner.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &owner_session, "0801d").await;

        let stranger = a_user(&state, "e@example.test", "Another Depositor", Role::Depositor, &["0803"]).await;
        let stranger_session = a_session(&state, stranger.id).await;
        let response = as_session(&app, get(&format!("/projects/0801d/entities/{entity_id}")), &stranger_session).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn an_unknown_proposal_id_is_a_404() {
        let (state, _) = test_state("entity-unknown").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, get("/projects/0801d/entities/person-999999"), &session).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_save_merges_and_an_unrendered_member_survives() {
        let (state, _) = test_state("entity-merge-unrendered").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;

        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        EntityProposalRepository::update_payload(
            &*state.editor,
            proposal.id,
            r#"{"somethingTheFormDoesNotRender": 42}"#,
            Utc::now(),
        )
        .await
        .expect("the payload should update");

        let uri = format!("/projects/0801d/entities/{entity_id}");
        as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada"), &session).await;

        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["somethingTheFormDoesNotRender"], 42, "{payload}");
        assert_eq!(payload["givenNames"], json!(["Ada"]), "{payload}");
    }

    #[tokio::test]
    async fn a_save_stores_what_was_typed_and_a_rerender_shows_it() {
        let (state, _) = test_state("entity-save-shows").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        as_session(&app, post(&uri, "name=Test+Org&url=https%3A%2F%2Fexample.org%2F"), &session).await;

        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["name"], "Test Org", "{payload}");
        assert_eq!(payload["url"], "https://example.org/", "{payload}");

        let response = as_session(&app, get(&uri), &session).await;
        let body = body_string(response).await;
        assert!(body.contains("Test Org"), "{body}");
        assert!(body.contains("https://example.org/"), "{body}");
    }

    #[tokio::test]
    async fn a_saved_url_without_its_format_is_stored_and_marked_on_the_reloaded_form() {
        let (state, _) = test_state("entity-save-format").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        let saved = as_session(&app, post(&uri, "name=Test+Org&url=www.example.org"), &session).await;
        assert_eq!(saved.status(), StatusCode::SEE_OTHER);
        assert_eq!(stored_payload(&state, "0801d").await["url"], "www.example.org");

        let target = location(&saved).expect("a redirect location");
        let body = body_string(as_session(&app, get(&target), &session).await).await;
        let input = body
            .split("<input")
            .find(|input| input.contains(r#"name="url""#))
            .expect("the url control");
        assert!(input.contains(r#"aria-invalid="true""#), "{input}");
        assert!(body.contains(crate::form::widgets::format_message(editor_core::form::format::ValueKind::Link)));
    }

    #[tokio::test]
    async fn a_renamed_published_organisation_shows_its_new_name_on_its_own_page_after_a_save() {
        let (state, _) = test_state("entity-change-shows-new-name").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        const ID: &str = "organization-008";
        let published_name = state.agents.get(ID).expect("a published agent").label.clone();
        let contributors = "/projects/0801d/sections/contributors";
        as_session(
            &app,
            post(
                contributors,
                &format!("attributions.row=r0&attributions.r0.contributor={ID}&attributions.r0.role=Funder"),
            ),
            &session,
        )
        .await;
        as_session(
            &app,
            post(contributors, &format!("intent=propose-changes:attributions:{ID}")),
            &session,
        )
        .await;
        let uri = format!("/projects/0801d/entities/{ID}");

        let before = body_string(as_session(&app, get(&uri), &session).await).await;
        assert!(
            before.contains(&format!(">{published_name} (organisation)</h1>")),
            "a change starts out under the published name: {before}"
        );

        as_session(&app, post(&uri, "name=Renamed+Organisation"), &session).await;

        let after = body_string(as_session(&app, get(&uri), &session).await).await;
        assert!(after.contains(">Renamed Organisation (organisation)</h1>"), "{after}");
        assert!(after.contains("<title>Renamed Organisation (organisation) — "), "{after}");
        assert!(!after.contains(&format!(">{published_name} (organisation)</h1>")), "{after}");
    }

    #[tokio::test]
    async fn emptying_every_address_member_drops_the_whole_group() {
        let (state, _) = test_state("entity-address-empty").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;

        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        EntityProposalRepository::update_payload(
            &*state.editor,
            proposal.id,
            r#"{"name":"Test Org","url":"https://example.org/","address":{"street":"Bahnhofstrasse 1",
               "postalCode":"3000","locality":"Bern","country":"Switzerland"}}"#,
            Utc::now(),
        )
        .await
        .expect("the payload should update");

        let uri = format!("/projects/0801d/entities/{entity_id}");
        let body = "name=Test+Org&url=https%3A%2F%2Fexample.org%2F&address.street=&address.postalCode=\
                    &address.locality=&address.country=&address.canton=&address.additional=";
        as_session(&app, post(&uri, body), &session).await;

        let payload = stored_payload(&state, "0801d").await;
        assert!(
            payload.get("address").is_none(),
            "an all-empty address is omitted entirely: {payload}"
        );
    }

    #[tokio::test]
    async fn a_canton_is_kept_for_a_swiss_country_and_cleared_for_any_other() {
        let (state, _) = test_state("entity-address-canton").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");
        let address = |country: &str| {
            format!(
                "name=Test+Org&url=https%3A%2F%2Fexample.org%2F&address.street=Main+St&address.postalCode=1000\
                 &address.locality=Town&address.country={country}&address.canton=Zurich&address.additional="
            )
        };

        as_session(&app, post(&uri, &address("Switzerland")), &session).await;
        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["address"]["canton"], "Zurich", "{payload}");

        as_session(&app, post(&uri, &address("Germany")), &session).await;
        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["address"]["country"], "Germany", "{payload}");
        assert!(payload["address"].get("canton").is_none(), "{payload}");
    }

    #[tokio::test]
    async fn a_canton_is_kept_for_a_stored_country_spelled_with_stray_whitespace() {
        let (state, _) = test_state("entity-address-canton-spelling").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;

        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        EntityProposalRepository::update_payload(
            &*state.editor,
            proposal.id,
            r#"{"name":"Test Org","url":"https://example.org/","address":{"street":"Main St",
               "postalCode":"1000","locality":"Town","country":"Switzerland ","canton":"Zurich"}}"#,
            Utc::now(),
        )
        .await
        .expect("the payload should update");

        // The form re-posts the stored country, since its select keeps a stored value selected.
        let uri = format!("/projects/0801d/entities/{entity_id}");
        let body = "name=Renamed+Org&url=https%3A%2F%2Fexample.org%2F&address.street=Main+St&address.postalCode=1000\
                    &address.locality=Town&address.country=Switzerland+&address.canton=Zurich&address.additional=";

        as_session(&app, post(&uri, body), &session).await;
        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["address"]["canton"], "Zurich", "{payload}");
    }

    #[tokio::test]
    async fn an_address_with_one_member_still_typed_is_kept() {
        // The other half of the rule: only clearing *every* member drops the group — leaving
        // even one behind means the depositor is partway through it, and `check_organization`, not
        // this merge, is what judges an incomplete one.
        let (state, _) = test_state("entity-address-partial").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        as_session(
            &app,
            post(&uri, "name=Test+Org&url=https%3A%2F%2Fexample.org%2F&address.street=Main+St"),
            &session,
        )
        .await;

        let payload = stored_payload(&state, "0801d").await;
        assert_eq!(payload["address"]["street"], "Main St", "{payload}");
    }

    #[tokio::test]
    async fn id_never_appears_in_a_stored_payload() {
        let (state, _) = test_state("entity-id-stripped").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;

        // Stands in for a payload that somehow carried one, so the save's own defensive strip is
        // what this test exercises rather than an id this form never had to remove.
        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        EntityProposalRepository::update_payload(
            &*state.editor,
            proposal.id,
            r#"{"id":"person-417","givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#,
            Utc::now(),
        )
        .await
        .expect("the payload should update");

        let uri = format!("/projects/0801d/entities/{entity_id}");
        as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada"), &session).await;

        let payload = stored_payload(&state, "0801d").await;
        assert!(payload.get("id").is_none(), "id must never survive a save: {payload}");
    }

    fn h1_of(body: &str) -> &str {
        let start = body.find("<h1").expect("an h1");
        &body[start..start + body[start..].find("</h1>").expect("h1 closes")]
    }

    #[tokio::test]
    async fn the_heading_and_title_carry_the_name_once_one_resolves() {
        let (state, _) = test_state("entity-heading-name").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        let fresh = body_string(as_session(&app, get(&uri), &session).await).await;
        let fallback = format!("Person {entity_id}");
        assert!(h1_of(&fresh).contains(&fallback), "{}", h1_of(&fresh));
        assert!(title_of(&fresh).starts_with(&fallback), "{}", title_of(&fresh));

        as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada&familyNames.row=r0&familyNames.r0=Lovelace&jobTitles.row=r0&jobTitles.r0=Archivist"), &session)
            .await;
        let named = body_string(as_session(&app, get(&uri), &session).await).await;
        assert!(h1_of(&named).contains("Ada Lovelace (person)"), "{}", h1_of(&named));
        assert!(!h1_of(&named).contains(&entity_id), "{}", h1_of(&named));
        assert!(title_of(&named).starts_with("Ada Lovelace (person) — "), "{}", title_of(&named));
        assert!(!title_of(&named).contains(&entity_id), "{}", title_of(&named));
    }

    #[tokio::test]
    async fn discard_asks_first_and_only_withdraws_once_confirmed() {
        let (state, _) = test_state("entity-discard-confirm").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        let confirm = as_session(&app, post(&uri, "intent=discard-confirm"), &session).await;
        assert_eq!(confirm.status(), StatusCode::OK);
        let confirm = body_string(confirm).await;
        assert!(confirm.contains("cannot be undone"), "{confirm}");
        assert!(
            confirm.contains("also taken out of this project's contributors, contact point and funding"),
            "a new entity's references go with it, and the depositor is told: {confirm}"
        );
        assert!(
            confirm.contains(&format!("proposal for Person {entity_id} removes")),
            "{confirm}"
        );
        assert_eq!(
            proposals_for(&state, "0801d").await[0].status,
            ProposalStatus::Draft,
            "confirming must not itself withdraw"
        );

        // A 303 on the plain path, like every other phase-changing write: a `POST` left in the
        // history re-posts on refresh, and this one would then find the proposal already withdrawn
        // and answer "no longer open, so there is nothing to discard" — a refusal surfacing from
        // an ordinary reload. Opened from no section, it returns to the first one.
        let discard = as_session(&app, post(&uri, "intent=discard"), &session).await;
        assert_eq!(discard.status(), StatusCode::SEE_OTHER);
        let target = location(&discard).expect("a redirect location");
        assert_eq!(
            target,
            format!("/projects/0801d/sections/overview?discarded_proposal={entity_id}")
        );
        assert_eq!(proposals_for(&state, "0801d").await[0].status, ProposalStatus::Withdrawn);

        let reloaded = body_string(as_session(&app, get(&target), &session).await).await;
        let live = reloaded.find(r#"aria-live="polite""#).expect("the live region");
        assert!(
            reloaded[live..].contains("Your proposed new person was discarded."),
            "{reloaded}"
        );
        assert!(
            title_of(&reloaded).starts_with("Proposal discarded — "),
            "{}",
            title_of(&reloaded)
        );
        assert!(
            !reloaded.contains("Proposed persons and organisations"),
            "the entry left the box: {reloaded}"
        );
    }

    /// Start a change to `organization-008` from its contributor card, as the card's control posts
    /// it, and return the entity form's URL the section redirected to.
    async fn a_change_from_the_card(app: &axum::Router, session: &str) -> String {
        let contributors = "/projects/0801d/sections/contributors";
        as_session(
            app,
            post(
                contributors,
                "attributions.row=r0&attributions.r0.contributor=organization-008&attributions.r0.role=Funder",
            ),
            session,
        )
        .await;
        let opened = as_session(
            app,
            post(contributors, "intent=propose-changes:attributions:organization-008"),
            session,
        )
        .await;
        location(&opened).expect("the section opens the entity form")
    }

    #[tokio::test]
    async fn a_discarded_change_returns_to_its_card_with_one_confirmation_naming_it() {
        let (state, _) = test_state("entity-discard-returns").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let name = state.agents.get("organization-008").expect("a published agent").label.clone();
        let opened = a_change_from_the_card(&app, &session).await;
        let uri = "/projects/0801d/entities/organization-008";
        let returned = "/projects/0801d/sections/contributors?discarded_proposal=organization-008";

        let form = body_string(as_session(&app, get(&opened), &session).await).await;
        assert!(form.contains(r#"name="section" value="contributors""#), "{form}");
        assert!(form.contains(r#"name="field" value="attributions""#), "{form}");
        let discard = as_session(
            &app,
            enhanced(uri, "section=contributors&field=attributions&intent=discard"),
            &session,
        )
        .await;
        assert_eq!(discard.status(), StatusCode::OK, "Datastar processes only a 200");
        assert_eq!(body_string(discard).await, format!("window.location.assign(\"{returned}\")"));

        let section = body_string(as_session(&app, get(returned), &session).await).await;
        let escaped = maud::html! {
            (name)
        }
        .into_string();
        assert_eq!(
            section.matches("Your proposed changes to").count(),
            1,
            "one confirmation: {section}"
        );
        assert!(
            section.contains(&format!("Your proposed changes to {escaped} were discarded.")),
            "{section}"
        );
        assert!(
            section.contains(r#"id="attributions-card-organization-008""#),
            "the anchor exists: {section}"
        );
    }

    #[tokio::test]
    async fn a_save_keeps_the_return_target() {
        let (state, _) = test_state("entity-save-keeps-origin").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        a_change_from_the_card(&app, &session).await;
        let uri = "/projects/0801d/entities/organization-008";

        let saved = as_session(
            &app,
            post(uri, "section=contributors&field=attributions&name=Renamed"),
            &session,
        )
        .await;
        let target = location(&saved).expect("a redirect location");
        assert!(target.contains("?section=contributors&field=attributions&saved="), "{target}");
        let reloaded = body_string(as_session(&app, get(&target), &session).await).await;
        assert!(
            reloaded.contains(r#"href="/projects/0801d/sections/contributors#attributions-card-organization-008""#),
            "{reloaded}"
        );
    }

    #[tokio::test]
    async fn discarding_a_new_proposal_drops_the_rows_naming_it() {
        let (state, _) = test_state("entity-discard-new-drops-rows").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let contributors = "/projects/0801d/sections/contributors";
        as_session(
            &app,
            post(
                contributors,
                &format!(
                    "attributions.row=r0&attributions.r0.contributor={entity_id}&attributions.r0.role=Author\
                     &attributions.row=r1&attributions.r1.contributor=person-001&attributions.r1.role=Editor\
                     &contactPoint.row=r0&contactPoint.r0={entity_id}"
                ),
            ),
            &session,
        )
        .await;

        let uri = format!("/projects/0801d/entities/{entity_id}");
        as_session(&app, post(&uri, "intent=discard"), &session).await;

        let draft = DraftRepository::find(&*state.editor, "0801d").await.unwrap().expect("a draft");
        let draft: Value = serde_json::from_str(&draft.payload).expect("JSON");
        assert!(!draft.to_string().contains(&entity_id), "{draft}");
        assert_eq!(draft["attributions"][0]["contributor"], "person-001", "{draft}");
    }

    #[tokio::test]
    async fn a_discard_marker_for_a_proposal_that_is_still_live_confirms_nothing() {
        let (state, _) = test_state("entity-discard-marker-stale").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;

        let uri = format!("/projects/0801d/sections/overview?discarded_proposal={entity_id}");
        let rendered = body_string(as_session(&app, get(&uri), &session).await).await;
        assert!(!rendered.contains("was discarded"), "{rendered}");
        assert!(
            !title_of(&rendered).starts_with("Proposal discarded"),
            "{}",
            title_of(&rendered)
        );
    }

    #[tokio::test]
    async fn a_save_redirects_with_a_marker_matching_the_stored_stamp() {
        let (state, _) = test_state("entity-save-redirect").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        let response = as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada"), &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let target = location(&response).expect("a redirect location");

        let stored = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        let (path, query) = target.split_once('?').expect("a query string");
        assert_eq!(path, uri);
        let encoded = query.strip_prefix("saved=").expect("a saved marker with a value");
        assert_eq!(
            urlencoding::decode(encoded).expect("valid percent-encoding").into_owned(),
            stored.updated_at.to_rfc3339(),
            "the marker should carry the stamp of the row just stored"
        );

        let reloaded = body_string(as_session(&app, get(&target), &session).await).await;
        let live = reloaded.find(r#"aria-live="polite""#).expect("the live region");
        assert!(reloaded[live..].contains("Saved."), "{reloaded}");
        assert!(title_of(&reloaded).starts_with("Saved — "), "{}", title_of(&reloaded));
    }

    #[tokio::test]
    async fn a_stale_saved_marker_renders_no_notice() {
        let (state, _) = test_state("entity-save-notice").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");

        let first_save = as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada"), &session).await;
        let first_target = location(&first_save).expect("a redirect location");

        as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Grace"), &session).await;

        let stale = body_string(as_session(&app, get(&first_target), &session).await).await;
        let live = stale.find(r#"aria-live="polite""#).expect("the live region");
        assert!(!stale[live..].contains("Saved."), "{stale}");
        assert!(!title_of(&stale).starts_with("Saved — "), "{}", title_of(&stale));

        let empty_marker = as_session(&app, get(&format!("{uri}?saved")), &session).await;
        assert_eq!(empty_marker.status(), StatusCode::OK);
        let empty_marker = body_string(empty_marker).await;
        let live = empty_marker.find(r#"aria-live="polite""#).expect("the live region");
        assert!(!empty_marker[live..].contains("Saved."), "{empty_marker}");
        assert!(!title_of(&empty_marker).starts_with("Saved — "), "{}", title_of(&empty_marker));
    }

    #[tokio::test]
    async fn a_non_live_proposal_refuses_both_the_control_and_the_post() {
        let (state, _) = test_state("entity-non-live").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");
        as_session(&app, post(&uri, "intent=discard"), &session).await;
        assert_eq!(
            proposals_for(&state, "0801d").await[0].status,
            ProposalStatus::Withdrawn,
            "the setup"
        );

        let page = as_session(&app, get(&uri), &session).await;
        let body = body_string(page).await;
        assert!(!body.contains("Discard this proposal"), "the control is withdrawn too: {body}");
        assert!(!body.contains(r#"name="givenNames.row""#), "no control posts any more: {body}");

        let save = as_session(&app, post(&uri, "givenNames.row=r0&givenNames.r0=Ada"), &session).await;
        assert_eq!(save.status(), StatusCode::OK, "a refusal, not a 500");
        let refusal = body_string(save).await;
        assert!(refusal.contains("no longer open"), "{refusal}");
        assert!(title_of(&refusal).starts_with("Not saved — "), "{}", title_of(&refusal));

        let discard_again = as_session(&app, post(&uri, "intent=discard"), &session).await;
        assert!(body_string(discard_again).await.contains("no longer open"));
        assert_eq!(
            proposals_for(&state, "0801d").await[0].status,
            ProposalStatus::Withdrawn,
            "still withdrawn, not re-decided"
        );
    }

    const AFFILIATION: &str = "organization-002";

    /// A person proposal holding one affiliation, `organization-008`, and its page URI.
    async fn a_person_affiliated(label: &str) -> (crate::fixtures::TestState, axum::Router, String, String) {
        let (state, _) = test_state(label).await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;
        let uri = format!("/projects/0801d/entities/{entity_id}");
        as_session(
            &app,
            post(&uri, "affiliations.row=r0&affiliations.r0=organization-008"),
            &session,
        )
        .await;
        (state, app, session, uri)
    }

    fn label_query(state: &EditorState, id: &str) -> String {
        crate::fixtures::urlencode(&state.agents.get(id).expect("a published agent").label)
    }

    #[tokio::test]
    async fn a_search_on_the_entity_form_stores_the_body_and_re_renders_without_redirecting() {
        let (state, app, session, uri) = a_person_affiliated("entity-find-agent").await;
        let query = label_query(&state, AFFILIATION);
        let body = format!(
            "affiliations.row=r0&affiliations.r0=organization-008&affiliations.new.q={query}&intent=find-agent"
        );

        for enhanced in [false, true] {
            let mut request = post(&uri, &body);
            if enhanced {
                request
                    .headers_mut()
                    .insert("datastar-request", "true".parse().expect("a header value"));
            }
            let response = as_session(&app, request, &session).await;
            assert_eq!(response.status(), StatusCode::OK, "enhanced={enhanced}");
            assert_eq!(location(&response), None, "enhanced={enhanced}");
            let page = body_string(response).await;
            assert!(
                page.contains(&format!("add-agent:affiliations:{AFFILIATION}")),
                "enhanced={enhanced}: the search offers matches: {page}"
            );
            assert!(
                page.contains(&format!(r#"value="{}""#, state.agents.get(AFFILIATION).unwrap().label)),
                "{page}"
            );
        }
        assert_eq!(
            stored_payload(&state, "0801d").await["affiliations"],
            json!(["organization-008"]),
            "a search stores the body and changes no reference"
        );
    }

    #[tokio::test]
    async fn adding_an_affiliation_appends_it_and_choosing_one_replaces_it() {
        let (state, app, session, uri) = a_person_affiliated("entity-add-choose-agent").await;

        as_session(
            &app,
            post(
                &uri,
                &format!(
                    "affiliations.row=r0&affiliations.r0=organization-008&intent=add-agent:affiliations:{AFFILIATION}"
                ),
            ),
            &session,
        )
        .await;
        assert_eq!(
            stored_payload(&state, "0801d").await["affiliations"],
            json!(["organization-008", AFFILIATION])
        );

        let mut enhanced = post(
            &uri,
            &format!(
                "affiliations.row=r0&affiliations.r0=organization-008&affiliations.r0.q=x\
                 &intent=choose-agent:affiliations.r0:{AFFILIATION}"
            ),
        );
        enhanced
            .headers_mut()
            .insert("datastar-request", "true".parse().expect("a header value"));
        let response = as_session(&app, enhanced, &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            stored_payload(&state, "0801d").await["affiliations"],
            json!([AFFILIATION]),
            "the chosen id replaces the row's"
        );
    }

    #[tokio::test]
    async fn changing_an_affiliation_re_renders_with_the_search_open_and_never_redirects() {
        let (state, app, session, uri) = a_person_affiliated("entity-change-agent").await;

        let response = as_session(
            &app,
            post(
                &uri,
                "affiliations.row=r0&affiliations.r0=organization-008&intent=change-agent:affiliations.r0",
            ),
            &session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "a redirect would lose the open search");
        assert_eq!(location(&response), None);
        let page = body_string(response).await;
        assert!(page.contains(r#"aria-controls="affiliations.r0-results""#), "{page}");
        assert_eq!(
            stored_payload(&state, "0801d").await["affiliations"],
            json!(["organization-008"]),
            "stored like a search"
        );
    }

    #[tokio::test]
    async fn the_entity_agent_search_answers_options_and_writes_nothing() {
        let (state, app, session, uri) = a_person_affiliated("entity-agent-fragment").await;
        let before = proposals_for(&state, "0801d").await;
        let agents = format!("{uri}/agents");

        let held = label_query(&state, "organization-008");
        let response = as_session(&app, get(&format!("{agents}?for=affiliations.new&q={held}")), &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").and_then(|value| value.to_str().ok());
        assert!(
            content_type.is_some_and(|value| value.starts_with("text/html")),
            "{content_type:?}"
        );
        let fragment = body_string(response).await;
        assert!(!fragment.contains("<html"), "the fragment only: {fragment}");
        assert!(
            !fragment.contains("add-agent:affiliations:organization-008"),
            "an affiliation the payload holds is not offered again: {fragment}"
        );

        let other = label_query(&state, AFFILIATION);
        let fragment =
            body_string(as_session(&app, get(&format!("{agents}?for=affiliations.new&q={other}")), &session).await)
                .await;
        assert!(
            fragment.contains(&format!(r#"value="add-agent:affiliations:{AFFILIATION}""#)),
            "{fragment}"
        );

        let card =
            body_string(as_session(&app, get(&format!("{agents}?for=affiliations.r0&q={other}")), &session).await)
                .await;
        assert!(
            card.contains(&format!(r#"value="choose-agent:affiliations.r0:{AFFILIATION}""#)),
            "{card}"
        );

        let nothing =
            body_string(as_session(&app, get(&format!("{agents}?for=affiliations.new&q=zzzzqqqq")), &session).await)
                .await;
        assert!(nothing.contains("Nothing matches"), "{nothing}");
        assert!(
            !nothing.contains("Propose a new"),
            "the entity page offers no propose controls: {nothing}"
        );
        assert!(!nothing.contains(r#"value="propose-"#), "{nothing}");

        assert_eq!(
            proposals_for(&state, "0801d").await,
            before,
            "a search changes neither payload nor stamp"
        );
    }

    #[tokio::test]
    async fn the_entity_agent_search_refuses_a_picker_the_proposal_does_not_have() {
        let (_state, app, session, uri) = a_person_affiliated("entity-agent-fragment-refused").await;
        for target in [
            "for=nope.new",
            "for=a%27b",
            "for=",
            "",
            // A field of this kind that has no agent picker.
            "for=givenNames.new",
        ] {
            let response = as_session(&app, get(&format!("{uri}/agents?{target}&q=org")), &session).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{target}");
        }
        let long = format!("affiliations.{}", "r".repeat(200));
        let response = as_session(&app, get(&format!("{uri}/agents?for={long}&q=org")), &session).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "a name over 200 characters");
    }

    #[tokio::test]
    async fn the_entity_agent_search_has_the_reach_rules_of_the_entity_page() {
        let (state, app, _session, uri) = a_person_affiliated("entity-agent-fragment-reach").await;
        let stranger = a_user(&state, "e@example.test", "Another Depositor", Role::Depositor, &["0803"]).await;
        let stranger_session = a_session(&state, stranger.id).await;
        let response = as_session(
            &app,
            get(&format!("{uri}/agents?for=affiliations.new&q=org")),
            &stranger_session,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn over_of_reads_a_submitted_proposal_as_still_live() {
        // Unlike a submitted *project* draft, a submitted entity proposal is still the
        // depositor's to finish — `EntityProposal::is_live` says so, and this mapping must agree.
        assert_eq!(over_of(ProposalStatus::Draft), None);
        assert_eq!(over_of(ProposalStatus::Submitted), None);
        assert_eq!(over_of(ProposalStatus::Accepted), Some(page::Over::Accepted));
        assert_eq!(over_of(ProposalStatus::Rejected), Some(page::Over::Terminal));
        assert_eq!(over_of(ProposalStatus::Withdrawn), Some(page::Over::Terminal));
    }
}
