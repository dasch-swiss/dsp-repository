//! `GET /projects`, and the `/projects/{shortcode}` redirect into the form.
//!
//! The editing surface is the project form's work. What lands here is the
//! **scope**: a depositor is confined to the shortcodes assigned to them, and anything else is
//! a 403. Both routes now read the published
//! set for the projects' names, so the list is a real list. Both take
//! [`Authenticated`](crate::auth::guard::Authenticated), so an unauthenticated
//! request never reaches this module.
//!
//! The 403 is answered before the record is read, and it always will be: an
//! authorization check that runs after a lookup leaks the project's existence
//! through the difference between 403 and 404. Here there is nothing to look up
//! yet, so the ordering costs nothing to establish now and would cost a rewrite
//! to establish later.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use editor_core::draft::ProjectDraft;
use editor_core::records::{normalize_shortcode, User};
use editor_core::repository::{ApprovedRecordRepository, DraftRepository, RepositoryError, SubmissionRepository};
use editor_core::status::{depositor_state, Comparison, ProjectState};
use editor_web::pages::projects::AssignedProject;
use platform_metadata::is_valid_shortcode;

use crate::auth::guard::Authenticated;
use crate::AppState;

/// What a depositor is told about a project that is not theirs.
///
/// Safe to say plainly: they named the shortcode to get here, so it tells them
/// nothing they did not supply, and a vaguer message would send them to RDU
/// without knowing what to ask for.
pub(crate) const NOT_ASSIGNED: &str =
    "This project is not assigned to your account. RDU assigns projects to depositors; ask \
                            them if you should have access to it.";

/// `GET /projects` — what this account may edit.
pub(crate) async fn list(State(state): State<AppState>, Authenticated(user, _): Authenticated) -> Response {
    // Not one page with a branch inside it: an RDU account's `shortcodes` is
    // empty by design, so rendering it through the depositor's list
    // would tell an administrator they have no projects.
    let content = if user.is_rdu() {
        let rows: Vec<_> = state.published.summaries().collect();
        editor_web::pages::projects::rdu_overview(&rows)
    } else {
        // The rows are the intersection of the assignments and the published
        // set; the count is the assignments themselves. Both are passed because
        // the difference is a distinct state with a distinct message — a
        // depositor whose projects are merely unpublished must not be told
        // nobody assigned them anything.
        let summaries: Vec<_> = state.published.summaries_for(&user.shortcodes).collect();
        let mut rows = Vec::with_capacity(summaries.len());
        for summary in summaries {
            // One read set per row rather than one for the page: a depositor
            // holds a handful of assignments, and the bulk query this would
            // otherwise need is three new ports for a list that is never long.
            match project_state(&state, summary.shortcode).await {
                Ok(project) => rows.push(AssignedProject { summary, state: project }),
                Err(error) => return storage_error(&state, &user, "read this project's state", &error),
            }
        }
        editor_web::pages::projects::assigned(&rows, user.shortcodes.len())
    };
    crate::render(
        &state,
        "Projects — DaSCH Metadata Editor",
        axum::http::StatusCode::OK,
        Some(&user),
        content,
    )
}

/// `GET /projects/{shortcode}` — a redirect to the first form section.
///
/// A redirect rather than a page, so exactly one place decides where a project
/// link lands, and the scheme in `docs/src/editor/architecture.md` keeps the
/// form's own URLs section-scoped and bookmarkable. It is therefore absent from
/// `page_url.rs`'s `KNOWN_ROUTES`: a redirect renders no beacon script, so no
/// beacon can report it.
///
/// The 404-then-403 order is the same as everywhere else in this module, and the
/// redirect target is deliberately the *same* section for both audiences — a
/// destination that depended on the role is one more thing to get wrong in a
/// link shared between a depositor and a reviewer.
pub(crate) async fn detail(
    State(state): State<AppState>,
    Authenticated(user, _): Authenticated,
    Path(shortcode): Path<String>,
) -> Response {
    // Shape first, so a path segment that could never name a project is a 404
    // rather than a 403. A 403 for `/projects/../etc/passwd` would assert that
    // such a project exists and is merely closed to this account.
    if !is_valid_shortcode(&shortcode) {
        return crate::not_found(State(state)).await;
    }
    // The authorization check runs before anything is read, and has to:
    // answering 404 for a shortcode that is not published and 403 for one that
    // is would make the pair an oracle for which projects exist, to a reader who
    // is not allowed to know. Redirecting an unassigned reader to the section
    // URL would merely move the 403 one request later, and leak the same thing
    // through the redirect.
    //
    // Nothing here answers 404 for an unknown shortcode either, and that is not
    // an oversight: a project may exist only locally, so
    // "absent from the published set" is not "does not exist" — the section
    // handler opens such a project blank rather than refusing it.
    if !user.may_reach(&shortcode) {
        // Logged because a depositor repeatedly reaching for projects
        // that are not theirs is worth seeing, and the two identifiers here are
        // both non-personal: an opaque account id and a shortcode.
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused a project that is not assigned to this account"
        );
        return crate::forbidden(&state, &user, NOT_ASSIGNED);
    }
    let audience = if user.is_rdu() {
        editor_web::form::registry::Audience::RduOnly
    } else {
        editor_web::form::registry::Audience::Everyone
    };
    let section = editor_web::form::registry::first_section(audience);
    axum::response::Redirect::to(&format!("/projects/{shortcode}/sections/{}", section.id)).into_response()
}

/// `GET /states` — what each state means and how long Online takes (REQ-2.6).
///
/// Behind [`Authenticated`] like the rest: it explains a depositor's own
/// projects, and the editor has no public pages besides the login flow.
pub(crate) async fn states(State(state): State<AppState>, Authenticated(user, _): Authenticated) -> Response {
    crate::render(
        &state,
        "What the states mean — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user),
        editor_web::pages::states::explanation(),
    )
}

/// The state to show a depositor for one project (REQ-2.1).
///
/// Three reads and the published set. The comparison is made against the
/// **approved record** where there is one, because that is the only local row
/// whose publication is in question: a draft differing from published data is
/// just an edit in progress, and comparing it would make every unsaved change
/// look like a pending release.
async fn project_state(state: &AppState, shortcode: &str) -> Result<ProjectState, RepositoryError> {
    // The three record tables key on the *normalized* shortcode, while the
    // summary carries the published set's own spelling — and 24 of the 85
    // published shortcodes are mixed case. Querying `080C` against rows stored
    // as `080c` finds nothing, which would read as "no local record" and so
    // report those projects Online no matter what their depositor had pending.
    let key = normalize_shortcode(shortcode);
    let submission = SubmissionRepository::find_by_shortcode(&*state.db, &key)
        .await?
        .map(|s| s.state);
    let has_draft = DraftRepository::find(&*state.db, &key).await?.is_some();
    let (approved, comparison) = approved_comparison(state, &key).await?;

    Ok(depositor_state(submission, approved, has_draft, &comparison))
}

/// How a project's newest approved record compares against the published set,
/// and whether it holds one at all.
///
/// Shared with the form (`crate::sections`), which needs the same answer for
/// REQ-2.5's waiting-for-release notice. One function because the rule it
/// encodes is not obvious and must not drift: **the newest record is the one
/// whose publication is in question** — `find_by_shortcode` orders oldest
/// first, and a project holds more than one whenever collection has lagged.
/// Two call sites deriving that separately would silently disagree the moment
/// the selection changed.
///
/// `key` is a normalized shortcode. A record whose payload cannot be parsed is
/// treated as no record rather than as an error: the page's job is to render a
/// state, and refusing the whole list over one unreadable row would take every
/// other project down with it. The startup pass is where an unreadable payload
/// is reported.
pub(crate) async fn approved_comparison(state: &AppState, key: &str) -> Result<(bool, Comparison), RepositoryError> {
    let approved = ApprovedRecordRepository::find_by_shortcode(&*state.db, key).await?;
    let local = approved
        .last()
        .and_then(|record| serde_json::from_str::<ProjectDraft>(&record.payload).ok());
    Ok((
        !approved.is_empty(),
        Comparison::classify(state.published.get(key), local.as_ref()),
    ))
}

/// Storage would not answer, so the page cannot show what it should.
fn storage_error(state: &AppState, viewer: &User, what: &str, error: &RepositoryError) -> Response {
    tracing::error!(error = %error, operation = what, "the project list could not reach storage");
    crate::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(viewer),
        editor_web::pages::problem::unavailable(
            "The editor could not reach its database, so this page is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use editor_core::records::{normalize_shortcode, DraftRecord, Role};
    use editor_core::repository::DraftRepository;
    use tower::ServiceExt;

    use crate::test_support::{
        a_session, a_user, body_string, capture_logs, get, location, test_app, test_state, with_cookie,
    };

    /// `GET uri` as `session`.
    async fn as_session(app: &axum::Router, uri: &str, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(get(uri), crate::auth::cookie::SESSION, session))
            .await
            .expect("the request should complete")
    }

    #[tokio::test]
    async fn test_a_depositor_opening_a_project_lands_on_its_first_form_section() {
        let (state, _) = test_state("project-allowed").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801", "080C"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, "/projects/0801", &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/projects/0801/sections/overview"));
    }

    #[tokio::test]
    async fn test_the_redirect_target_does_not_depend_on_the_role() {
        // A destination that differed by role is one more thing to get wrong in
        // a link shared between a depositor and a reviewer, and both audiences
        // see `overview` first.
        let (state, _) = test_state("project-first-section").await;
        let depositor = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801"]).await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let app = test_app(&state);

        for user in [depositor, rdu] {
            let session = a_session(&state, user.id).await;
            let response = as_session(&app, "/projects/0801", &session).await;
            assert_eq!(
                location(&response).as_deref(),
                Some("/projects/0801/sections/overview"),
                "{}",
                user.name
            );
        }
    }

    #[tokio::test]
    async fn test_a_depositor_reaching_an_unassigned_project_gets_a_403_page_with_a_way_back() {
        // The requirement asks for the status. The page is because a bare 403 is a dead
        // end in a browser — the reader is signed in and has nothing to press.
        let (state, _) = test_state("project-forbidden").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, "/projects/0803", &session).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("not assigned to your account"), "{body}");
        assert!(body.contains(r#"<a href="/projects""#), "{body}");
    }

    #[tokio::test]
    async fn test_the_assignment_check_ignores_case_so_a_typed_shortcode_still_works() {
        // The published set mixes `080C` with `0801a`; an RDU member typing an
        // assignment cannot be expected to get the case right, and getting it
        // wrong would deny a depositor their own project with no visible cause.
        let (state, _) = test_state("project-case").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["080c"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        assert_eq!(
            as_session(&app, "/projects/080C", &session).await.status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            as_session(&app, "/projects/080c", &session).await.status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            as_session(&app, "/projects/080E", &session).await.status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn test_rdu_opens_any_project_without_an_assignment() {
        // RDU access is role-based, not per-project, which is why an
        // RDU account's assignment set is empty.
        let (state, _) = test_state("project-rdu").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        assert_eq!(
            as_session(&app, "/projects/0803", &session).await.status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            as_session(&app, "/projects/0801a", &session).await.status(),
            StatusCode::SEE_OTHER
        );
    }

    #[tokio::test]
    async fn test_a_path_that_could_never_name_a_project_is_a_404_and_not_a_403() {
        // A 403 here would assert that such a project exists and is merely closed
        // to this account, which is a claim about a path the reader invented.
        let (state, _) = test_state("project-shape").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        for uri in ["/projects/not%20a%20code", "/projects/a-b", "/projects/%2e%2e"] {
            let response = as_session(&app, uri, &session).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn test_the_list_shows_a_depositor_their_assignments_and_nothing_else() {
        // `0801d` and `080C` are real published shortcodes, so both are rows.
        let (state, _) = test_state("list-depositor").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d", "080C"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains(r#"href="/projects/0801d""#), "{body}");
        assert!(body.contains(r#"href="/projects/080C""#), "{body}");
        assert!(!body.contains("/projects/0803"), "{body}");
    }

    #[tokio::test]
    async fn test_the_list_names_each_project_rather_than_only_its_shortcode() {
        // The reason the published set is read at all: a depositor recognises
        // their project by name, not by a four-character code.
        let (state, _) = test_state("list-names").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let expected = state
            .published
            .get("0801d")
            .expect("0801d is in the committed corpus")
            .name
            .clone();
        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains(&expected), "the list should name the project: {body}");
    }

    #[tokio::test]
    async fn test_an_assignment_with_no_published_project_is_not_a_blank_row() {
        // A project assigned before it is published, and a project that exists only locally,
        // are both this state. It has to be distinguishable from
        // having no assignments at all, or the depositor asks the wrong person.
        let (state, _) = test_state("list-unpublished").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["9999"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains("none of them is in the published set"), "{body}");
        assert!(!body.contains("No projects are assigned"), "{body}");
        assert!(!body.contains("<table"), "{body}");
    }

    #[tokio::test]
    async fn test_the_rdu_overview_lists_the_whole_published_set() {
        let (state, _) = test_state("list-rdu-set").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        // Every project in the corpus is a row, not just the reader's own.
        for shortcode in ["0801a", "0801d", "080C"] {
            assert!(
                body.contains(&format!(r#"href="/projects/{shortcode}""#)),
                "{shortcode}: {body}"
            );
        }
    }

    #[tokio::test]
    async fn test_the_list_does_not_tell_an_rdu_member_they_have_no_projects() {
        // An RDU account's assignment set is empty by design, so rendering it
        // through the depositor's list would read as "you have none".
        let (state, _) = test_state("list-rdu").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains("role-based"), "{body}");
        assert!(!body.contains("No projects are assigned"), "{body}");
        assert!(!body.contains("none of them is in the published set"), "{body}");
    }

    #[tokio::test]
    async fn test_a_signed_out_visitor_is_sent_to_login_and_back_again() {
        let (state, _) = test_state("project-anonymous").await;
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(get("/projects/0801"))
            .await
            .expect("the request should complete");
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/login?next=/projects/0801"));
    }

    #[tokio::test]
    async fn test_a_refusal_is_logged_without_anything_personal_in_it() {
        // The two identifiers are an opaque account id and a
        // shortcode, and a depositor repeatedly reaching for projects that are
        // not theirs is worth being able to see.
        let (state, _) = test_state("project-log").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let (logs, guard) = capture_logs();
        let _ = as_session(&app, "/projects/0803", &session).await;
        drop(guard);

        let lines = logs.lines();
        assert!(
            lines
                .iter()
                .any(|line| line.contains("0803") && line.contains(&user.id.to_string())),
            "the refusal must be traceable: {lines:?}"
        );
        assert!(
            !lines.iter().any(|line| line.contains("d@example.test")),
            "no address may reach a log or a span: {lines:?}"
        );
    }

    #[tokio::test]
    async fn a_mixed_case_assignment_still_finds_its_local_records() {
        // The regression this exists for: `drafts` keys on the normalized
        // shortcode, the published set spells `080C` with a capital, and 24 of
        // the 85 published shortcodes are mixed case. Querying the stored
        // spelling finds no draft, so the project reports Online while its
        // depositor has unsaved work — a wrong answer for a quarter of the set.
        let (state, _) = test_state("project-state-case").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["080C"]).await;
        let session = a_session(&state, user.id).await;
        DraftRepository::upsert(
            &*state.db,
            &DraftRecord {
                shortcode: normalize_shortcode("080C"),
                payload: r#"{"name":"Work in progress"}"#.to_string(),
                updated_by: Some(user.id),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .expect("upsert");

        let app = test_app(&state);
        let body = body_string(as_session(&app, "/projects", &session).await).await;

        assert!(
            body.contains("Draft"),
            "a project with a draft row must not read as Online: {body}"
        );
    }

    #[tokio::test]
    async fn a_published_project_with_nothing_pending_reads_online() {
        // REQ-2.1 on the list, and the only place a depositor ever sees the
        // result of REQ-2.4's discard.
        let (state, _) = test_state("project-state-online").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;

        assert!(body.contains("Online"), "{body}");
        assert!(body.contains("Your changes"), "the state column must be labelled: {body}");
    }

    #[tokio::test]
    async fn the_state_explanation_page_is_reachable_and_linked_from_the_list() {
        // REQ-2.6. A page nothing links to does not explain anything, so the
        // link is part of the requirement rather than a nicety.
        let (state, _) = test_state("project-state-explained").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let list = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(
            list.contains(r#"href="/states""#),
            "the list must link to the explanation: {list}"
        );

        let response = as_session(&app, "/states", &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        for label in ["Draft", "Submitted", "In review", "Approved", "Online"] {
            assert!(body.contains(label), "{label} is missing from the explanation page: {body}");
        }
        assert!(body.contains("few weeks"), "REQ-2.6 requires the expected wait: {body}");
    }

    #[tokio::test]
    async fn the_state_explanation_page_is_not_public() {
        // Every page but the login flow is behind `Authenticated`.
        let (state, _) = test_state("project-state-guarded").await;
        let app = test_app(&state);
        let response = app.oneshot(get("/states")).await.expect("response");
        assert_ne!(
            response.status(),
            StatusCode::OK,
            "/states must not answer an unauthenticated request"
        );
    }
}
