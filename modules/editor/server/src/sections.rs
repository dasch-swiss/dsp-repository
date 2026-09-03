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
use editor_core::records::{normalize_shortcode, DraftRecord, SubmissionState, User};
use editor_core::repository::{DraftRepository, RepositoryError, SubmissionRepository};
use editor_web::form::registry::{self, Audience, Section};
use editor_web::pages::section as page;
use platform_metadata::is_valid_shortcode;

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
    /// The published project's name, for the heading and the tab title.
    project_name: Option<&'a str>,
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
    let locked = match SubmissionRepository::find_by_shortcode(&*state.db, &key).await {
        Ok(submission) => submission.and_then(|submission| match submission.state {
            SubmissionState::Submitted => Some(page::Locked::Submitted),
            SubmissionState::InReview => Some(page::Locked::InReview),
            SubmissionState::Approved => None,
        }),
        Err(error) => return Err(storage_error(state, user, "read this project's submission", &error)),
    };

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
    render_page(&state, &user, &shortcode, &context, None)
}

/// `POST /projects/{shortcode}/sections/{section}` — save the draft (REQ-1.10).
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "project section save",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.section = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn save(
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

    // Re-checked here and not only when the form was rendered: the render is a
    // `GET`, so nothing stops a `POST` arriving without one — or arriving after
    // a reviewer picked the project up in the meantime.
    if context.locked.is_some() {
        span.record("form.outcome", "locked");
        tracing::info!("refused a save against a project that is in review");
        return refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_LOCKED);
    }

    let body = FormBody::from_pairs(pairs);
    let mut applied = 0;
    for field in context.section.fields_for(context.audience) {
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
        // Preserved from the stored row so "created" keeps meaning when the
        // draft was first saved rather than when it was last.
        created_at: context.record.as_ref().map_or(now, |record| record.created_at),
        updated_at: now,
    };

    match DraftRepository::upsert(&*state.db, &record).await {
        Ok(()) => {
            span.record("form.outcome", "saved");
            tracing::info!(fields.applied = applied, "saved a project draft");
            saved(&shortcode, &context, headers, now)
        }
        Err(error) => {
            span.record("form.outcome", "store_failed");
            tracing::error!(error = %error, "could not save a project draft");
            refused(&state, &user, &shortcode, &context, headers, SAVE_REFUSED_STORAGE)
        }
    }
}

/// Whether this request came from the Datastar bundle.
fn is_enhanced(headers: &HeaderMap) -> bool {
    headers.contains_key(DATASTAR_REQUEST)
}

/// A successful save: a redirect on the plain path, the patched region on the
/// enhanced one.
fn saved(shortcode: &str, context: &Context<'_>, headers: HeaderMap, at: DateTime<Utc>) -> Response {
    if !is_enhanced(&headers) {
        // POST-redirect-GET: a `POST` left in the history re-posts on refresh,
        // and the reloaded `GET` reads the row that was just written, so the
        // "last saved" line is the confirmation rather than a flash message
        // that has to survive a redirect.
        return Redirect::to(&format!("/projects/{shortcode}/sections/{}", context.section.id)).into_response();
    }
    let saved_at = crate::format_instant(at);
    region(shortcode, context, Some(saved_at.as_str()), Some(page::Notice::Saved))
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
    let notice = Some(page::Notice::Refused(message));
    if is_enhanced(&headers) {
        return region(shortcode, context, saved_at(context).as_deref(), notice);
    }
    render_page(state, user, shortcode, context, notice)
}

/// The section region, as the enhanced path's `datastar-patch-elements`.
///
/// 200 always: Datastar processes a response body only on a 200, so a status
/// carrying the refusal would lose the message it is carrying.
fn region(
    shortcode: &str,
    context: &Context<'_>,
    saved_at: Option<&str>,
    notice: Option<page::Notice<'_>>,
) -> Response {
    let view = view(shortcode, context, saved_at, notice);
    (StatusCode::OK, axum::response::Html(page::region(&view).into_string())).into_response()
}

/// The whole page, inside the document shell.
fn render_page(
    state: &AppState,
    user: &User,
    shortcode: &str,
    context: &Context<'_>,
    notice: Option<page::Notice<'_>>,
) -> Response {
    let stored = saved_at(context);
    let view = view(shortcode, context, stored.as_deref(), notice);
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
    notice: Option<page::Notice<'a>>,
) -> page::SectionView<'a> {
    page::SectionView {
        shortcode,
        project_name: context.project_name,
        section: context.section,
        audience: context.audience,
        draft: &context.draft,
        locked: context.locked,
        saved_at,
        notice,
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
    use axum::body::Body;
    use axum::http::Request;
    use editor_core::canonical::write_draft;
    use editor_core::records::{Role, Submission};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;
    use crate::test_support::{
        a_session, a_user, body_string, count_rows, get, location, open_test_db, post, state_over, test_app,
        test_state, with_cookie, RecordingMailer,
    };

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

    /// Put a pending submission in front of the project.
    async fn a_submission(state: &AppState, shortcode: &str, user: Uuid, submission_state: SubmissionState) {
        SubmissionRepository::create(
            &*state.db,
            &Submission {
                id: Uuid::new_v4(),
                shortcode: shortcode.to_string(),
                payload: "{}".to_string(),
                state: submission_state,
                submitted_by: Some(user),
                submitted_at: Utc::now(),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
            },
        )
        .await
        .expect("the submission should store");
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
