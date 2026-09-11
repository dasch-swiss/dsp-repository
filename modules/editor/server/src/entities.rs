//! The entity form: `GET` and `POST /projects/{shortcode}/entities/{proposal}`.
//!
//! `{proposal}` is a proposal's **`entity_id`** (`person-417`, `organization-143`), not its row
//! `id` — the same value `pages::section`'s `proposed_notice` and `proposals_summary` already
//! link to, and the value the propose controls post back under `propose.entity`. A
//! project may hold more than one row for one `entity_id` over time (a withdrawn proposal, then a
//! fresh one for the same entity), but never more than one *live* one
//! (`entity_proposals_live_per_entity`); [`proposal_for`] resolves the live one where there is
//! one, and otherwise the most recently touched, so a stale link still shows something coherent.
//!
//! Same shape as `sections.rs`: [`context`] resolves a request in the order that module's own
//! docs argue for — shape, then authorization, then anything that reads state — and a proposal
//! under the wrong shortcode is a 404 for the same reason an unknown section id is: the reader
//! invented the pairing, not an assignment question a 403 would answer.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::{DateTime, Utc};
use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody, Shape, WhenCleared};
use editor_core::proposals::{EntityProposal, ProposalKind, ProposalStatus};
use editor_core::records::{normalize_shortcode, User};
use editor_core::repository::{EntityProposalRepository, RepositoryError};
use editor_web::entity as page;
use editor_web::form::INTENT;
use platform_metadata::is_valid_shortcode;
use serde_json::Value;

use crate::auth::guard::Authenticated;
use crate::sections::is_enhanced;
use crate::AppState;

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
    /// despite its name, `get`/`set`/`remove`/`multilingual`/`set_multilingual` touch nothing
    /// project-specific — only `from_raw`/`to_raw` and the URL/funding-shape helpers do, and none
    /// of those are called on a person or organisation. Reusing it is what lets this form reuse
    /// `editor_core::form::apply`'s appliers and `editor_web::form::widgets`'s row and
    /// multilingual composers verbatim, instead of a second implementation of row-key bookkeeping
    /// and empty/unchanged handling built for entities alone.
    draft: ProjectDraft,
    agents: AgentScope<'a>,
    posted: Option<&'a FormBody>,
    signed_out_at: DateTime<Utc>,
}

/// Resolve a request, or the response that refuses it. See the module docs for the ordering.
async fn context<'a>(
    state: &'a AppState,
    user: &User,
    shortcode: &str,
    entity_id: &str,
    signed_out_at: DateTime<Utc>,
) -> Result<Context<'a>, Response> {
    if !is_valid_shortcode(shortcode) {
        return Err(crate::not_found(State(state.clone())).await);
    }
    if !user.may_reach(shortcode) {
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused an entity form for a project that is not assigned to this account"
        );
        return Err(crate::forbidden(state, user, crate::projects::NOT_ASSIGNED));
    }

    let key = normalize_shortcode(shortcode);
    let proposals = match EntityProposalRepository::list_for_shortcode(&*state.db, &key).await {
        Ok(proposals) => proposals,
        Err(error) => return Err(storage_error(state, user, "read this project's entity proposals", &error)),
    };
    // The reader invented the pairing if this comes back empty — a proposal id under the wrong
    // shortcode is exactly that, so it is a 404 rather than a 403.
    let Some(proposal) = proposal_for(&proposals, entity_id) else {
        return Err(crate::not_found(State(state.clone())).await);
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

    Ok(Context { proposal, draft, agents, posted: None, signed_out_at })
}

/// This shortcode's proposal for `entity_id`: the live one if there is one, else the most
/// recently touched. `None` when this shortcode has never proposed this entity at all.
fn proposal_for(proposals: &[EntityProposal], entity_id: &str) -> Option<EntityProposal> {
    proposals
        .iter()
        .filter(|proposal| proposal.entity_id == entity_id)
        .max_by_key(|proposal| (proposal.is_live(), proposal.updated_at))
        .cloned()
}

/// `GET /projects/{shortcode}/entities/{proposal}`.
pub(crate) async fn show(
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, entity_id)): Path<(String, String)>,
) -> Response {
    let context = match context(&state, &user, &shortcode, &entity_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    render_page(&state, &user, &shortcode, &context, Rendering::default())
}

/// `POST /projects/{shortcode}/entities/{proposal}` — save the proposal, or discard it.
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
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, entity_id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    span.record("project.shortcode", tracing::field::display(&shortcode));
    span.record("form.entity", tracing::field::display(&entity_id));

    let body = FormBody::from_pairs(pairs);
    let mut context = match context(&state, &user, &shortcode, &entity_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => {
            span.record("form.outcome", "refused");
            return response;
        }
    };
    context.posted = Some(&body);

    // Anything this build does not know falls back to Save, matching
    // `sections.rs::act`'s own fallback: discarding is not undoable by the
    // depositor, and saving is.
    let intent = match body.get(INTENT) {
        Some(page::DISCARD) => Intent::Discard,
        Some(page::DISCARD_CONFIRM) => Intent::ConfirmDiscard,
        _ => Intent::Save,
    };
    span.record("form.intent", tracing::field::display(intent.as_str()));

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

    match EntityProposalRepository::update_payload(&*state.db, context.proposal.id, &payload, Utc::now()).await {
        Ok(()) => {
            span.record("form.outcome", "saved");
            tracing::info!("saved an entity proposal");
            saved(&shortcode, &context, headers)
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
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, entity_id, field_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(
        &state,
        &user,
        &shortcode,
        &entity_id,
        &field_id,
        headers,
        pairs,
        None,
        signed_out_at,
    )
    .await
}

/// `POST /projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove` — drop one row.
pub(crate) async fn remove_row(
    State(state): State<AppState>,
    Authenticated(user, signed_out_at): Authenticated,
    Path((shortcode, entity_id, field_id, key)): Path<(String, String, String, String)>,
    headers: HeaderMap,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Response {
    row_action(
        &state,
        &user,
        &shortcode,
        &entity_id,
        &field_id,
        headers,
        pairs,
        Some(&key),
        signed_out_at,
    )
    .await
}

/// Add or remove a row: the two differ only in what they do to the posted body, exactly the way
/// `sections.rs::row_action` argues for — removing drops the row's key from `{field}.row` before
/// anything is applied, so the save that follows simply does not see it.
#[allow(clippy::too_many_arguments)]
async fn row_action(
    state: &AppState,
    user: &User,
    shortcode: &str,
    entity_id: &str,
    field_id: &str,
    headers: HeaderMap,
    pairs: Vec<(String, String)>,
    removing: Option<&str>,
    signed_out_at: DateTime<Utc>,
) -> Response {
    let body = FormBody::from_pairs(match removing {
        Some(key) => FormBody::pairs_without_row(pairs, field_id, key),
        None => pairs,
    });

    let mut context = match context(state, user, shortcode, entity_id, signed_out_at).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    context.posted = Some(&body);

    // The field has to be one this proposal's kind actually renders, checked
    // against the same list the render side reads — an id naming another
    // field, or one this kind does not have, is a way to write outside the
    // contract this form declares.
    let Some(field) = editor_web::entity::fields_for(context.proposal.kind).iter().find(|field| {
        field.id == field_id
            && matches!(
                field.shape,
                Some(Shape::StringRows | Shape::AgentRows | Shape::ReferenceRows(_))
            )
    }) else {
        return crate::not_found(State(state.clone())).await;
    };

    if !context.proposal.is_live() {
        return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_NOT_LIVE);
    }

    apply_posted(&mut context.draft, context.proposal.kind, &body);
    let payload = serde_json::to_string(&context.draft).unwrap_or_default();
    if let Err(error) =
        EntityProposalRepository::update_payload(&*state.db, context.proposal.id, &payload, Utc::now()).await
    {
        tracing::error!(error = %error, "could not save an entity proposal row action");
        return refused(state, user, shortcode, &context, headers, SAVE_REFUSED_STORAGE);
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
    state: &AppState,
    user: &User,
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

    match EntityProposalRepository::withdraw(&*state.db, context.proposal.id, Utc::now()).await {
        Ok(()) => {
            tracing::info!(proposal.entity_id = %context.proposal.entity_id, "withdrew an entity proposal");
            // Re-resolve so the rendering reflects the withdrawal it just
            // made, the way `sections.rs::phase_changed` re-resolves after a
            // write that moves the record between phases — overriding only
            // the notice here would leave everything else describing the
            // proposal as still live.
            // The plain path **redirects**, like every other phase-changing write here and in
            // `sections.rs::phase_changed`: a `POST` left in the history re-posts on refresh, and
            // this one would then find the proposal already withdrawn and answer "no longer open,
            // so there is nothing to discard" — a refusal surfacing from an ordinary reload, which
            // is the reading the 303-after-write rule exists to prevent. The enhanced path still
            // renders the region, because Datastar processes a body only on a 200 and would merge
            // a followed redirect's whole page into it.
            if !is_enhanced(&headers) {
                return redirect_here(shortcode, context);
            }
            match self::context(state, user, shortcode, &context.proposal.entity_id, context.signed_out_at).await {
                Ok(fresh) => {
                    let rendering = Rendering {
                        notice: Some(page::Notice::Discarded),
                        ..Rendering::default()
                    };
                    region(shortcode, &fresh, rendering)
                }
                // The write landed; only the re-read did not. Render what is
                // already in hand rather than a redirect, which would send
                // the reader to a `GET` this route always answers anyway.
                Err(_) => {
                    let rendering = Rendering {
                        notice: Some(page::Notice::Discarded),
                        ..Rendering::default()
                    };
                    if is_enhanced(&headers) {
                        region(shortcode, context, rendering)
                    } else {
                        render_page(state, user, shortcode, context, rendering)
                    }
                }
            }
        }
        Err(error) => {
            tracing::error!(error = %error, "could not withdraw an entity proposal");
            refused(state, user, shortcode, context, headers, DISCARD_REFUSED_STORAGE)
        }
    }
}

/// What a `POST` to this route is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Save,
    Discard,
    ConfirmDiscard,
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
/// and an *empty* one as a person with no job title, which 59 of the 416
/// committed persons already are. `apply(Shape::StringRows, …)` cannot tell
/// the two apart on its own: it removes the field whenever no row survives,
/// which is right for every field of this shape except this one.
fn keep_job_titles_present(body: &FormBody, draft: &mut ProjectDraft) {
    if body.has("jobTitles.row") && draft.get("jobTitles").is_none() {
        draft.set("jobTitles", Value::Array(Vec::new()));
    }
}

fn apply_organization(body: &FormBody, draft: &mut ProjectDraft) {
    apply(Shape::Text(WhenCleared::Drop), body, draft, "name");
    apply(Shape::Text(WhenCleared::Drop), body, draft, "url");
    apply(Shape::Multilingual, body, draft, "alternativeName");
    for (member, _) in page::ADDRESS_MEMBERS {
        apply(Shape::Text(WhenCleared::Drop), body, draft, &format!("address.{member}"));
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

fn saved(shortcode: &str, context: &Context<'_>, headers: HeaderMap) -> Response {
    if !is_enhanced(&headers) {
        return redirect_here(shortcode, context);
    }
    region(
        shortcode,
        context,
        Rendering { notice: Some(page::Notice::Saved), ..Rendering::default() },
    )
}

fn confirming(state: &AppState, user: &User, shortcode: &str, context: &Context<'_>, headers: HeaderMap) -> Response {
    let rendering = Rendering {
        confirming: Some(page::Confirmation::Discard),
        ..Rendering::default()
    };
    if is_enhanced(&headers) {
        return region(shortcode, context, rendering);
    }
    render_page(state, user, shortcode, context, rendering)
}

fn redirect_here(shortcode: &str, context: &Context<'_>) -> Response {
    Redirect::to(&format!("/projects/{shortcode}/entities/{}", context.proposal.entity_id)).into_response()
}

fn refused(
    state: &AppState,
    user: &User,
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
    /// refusal and a row action, and deliberately not for a successful save,
    /// matching `sections.rs::Rendering::keep_posted`'s own reasoning.
    keep_posted: bool,
}

fn region(shortcode: &str, context: &Context<'_>, rendering: Rendering<'_>) -> Response {
    let view = view(shortcode, context, rendering);
    (
        StatusCode::OK,
        axum::response::Html(editor_web::entity::region(&view).into_string()),
    )
        .into_response()
}

fn render_page(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    rendering: Rendering<'_>,
) -> Response {
    let title = format!(
        "{} {} — DaSCH Metadata Editor",
        context.proposal.kind.label(),
        context.proposal.entity_id
    );
    let view = view(shortcode, context, rendering);
    crate::render(state, &title, StatusCode::OK, Some(user), editor_web::entity::page(&view))
}

fn view<'a>(
    shortcode: &'a str,
    context: &'a Context<'a>,
    rendering: Rendering<'a>,
) -> editor_web::entity::EntityView<'a> {
    editor_web::entity::EntityView {
        shortcode,
        proposal: &context.proposal,
        over: over_of(context.proposal.status),
        draft: &context.draft,
        confirming: rendering.confirming,
        notice: rendering.notice,
        posted: rendering.keep_posted.then_some(context.posted).flatten(),
        adding_row: rendering.adding_row,
        agents: Some(&context.agents),
        rows_action: format!("/projects/{shortcode}/entities/{}/fields", context.proposal.entity_id),
    }
}

fn storage_error(state: &AppState, user: &User, what: &str, error: &RepositoryError) -> Response {
    tracing::error!(error = %error, operation = what, "the entity form could not reach storage");
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
    use axum::body::Body;
    use axum::http::Request;
    use editor_core::records::Role;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::test_support::{a_session, a_user, body_string, get, post, test_app, test_state, with_cookie};

    async fn as_session(app: &axum::Router, request: Request<Body>, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(request, crate::auth::cookie::SESSION, session))
            .await
            .expect("the request should complete")
    }

    /// Every proposal stored for `shortcode`, for asserting on what a save or discard wrote —
    /// mirrors `sections.rs::tests::proposals_for`.
    async fn proposals_for(state: &AppState, shortcode: &str) -> Vec<EntityProposal> {
        EntityProposalRepository::list_for_shortcode(&*state.db, shortcode)
            .await
            .expect("proposals should read")
    }

    /// Start a proposal the way a depositor would — through the section form's own propose
    /// control — and return its entity id. This route never creates one; that is `sections.rs`'s
    /// job, so every test here needs a proposal already in hand before it can open this form.
    async fn a_proposal(app: &axum::Router, state: &AppState, session: &str, shortcode: &str, intent: &str) -> String {
        let uri = format!("/projects/{shortcode}/sections/overview");
        as_session(app, post(&uri, &format!("intent={intent}")), session).await;
        proposals_for(state, shortcode)
            .await
            .into_iter()
            .next()
            .expect("a proposal")
            .entity_id
    }

    async fn a_person_proposal(app: &axum::Router, state: &AppState, session: &str, shortcode: &str) -> String {
        a_proposal(app, state, session, shortcode, "propose-person").await
    }

    async fn an_organization_proposal(app: &axum::Router, state: &AppState, session: &str, shortcode: &str) -> String {
        a_proposal(app, state, session, shortcode, "propose-organization").await
    }

    async fn stored_payload(state: &AppState, shortcode: &str) -> Value {
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
            &*state.db,
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
    async fn emptying_every_address_member_drops_the_whole_group() {
        let (state, _) = test_state("entity-address-empty").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = an_organization_proposal(&app, &state, &session, "0801d").await;

        let proposal = proposals_for(&state, "0801d").await.into_iter().next().expect("a proposal");
        EntityProposalRepository::update_payload(
            &*state.db,
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
            &*state.db,
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
        assert!(body_string(confirm).await.contains("cannot be undone"));
        assert_eq!(
            proposals_for(&state, "0801d").await[0].status,
            ProposalStatus::Draft,
            "confirming must not itself withdraw"
        );

        // A 303 on the plain path, like every other phase-changing write: a `POST` left in the
        // history re-posts on refresh, and this one would then find the proposal already withdrawn
        // and answer "no longer open, so there is nothing to discard" — a refusal surfacing from
        // an ordinary reload.
        let discard = as_session(&app, post(&uri, "intent=discard"), &session).await;
        assert_eq!(discard.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            discard.headers().get("location").and_then(|value| value.to_str().ok()),
            Some(uri.as_str())
        );
        assert_eq!(proposals_for(&state, "0801d").await[0].status, ProposalStatus::Withdrawn);
    }

    #[tokio::test]
    async fn a_discard_on_the_datastar_path_renders_the_region_rather_than_redirecting() {
        // The other half of the two-rendering contract: Datastar processes a body only on a 200, so
        // a redirect it followed would merge the whole page into the region.
        let (state, _) = test_state("entity-discard-enhanced").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        let entity_id = a_person_proposal(&app, &state, &session, "0801d").await;

        // Built from the shared `post` helper so the CSRF headers match every other test's, with
        // only the header Datastar's bundle adds on top.
        let mut request = post(&format!("/projects/0801d/entities/{entity_id}"), "intent=discard");
        request
            .headers_mut()
            .insert("datastar-request", "true".parse().expect("a header value"));
        let response = as_session(&app, request, &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(proposals_for(&state, "0801d").await[0].status, ProposalStatus::Withdrawn);
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
        assert!(body_string(save).await.contains("no longer open"));

        let discard_again = as_session(&app, post(&uri, "intent=discard"), &session).await;
        assert!(body_string(discard_again).await.contains("no longer open"));
        assert_eq!(
            proposals_for(&state, "0801d").await[0].status,
            ProposalStatus::Withdrawn,
            "still withdrawn, not re-decided"
        );
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
