//! `GET /projects` and `GET /projects/{shortcode}`.
//!
//! The editing surface is the project form's work. What lands here is the
//! **scope**: REQ-1.2 confines a depositor to the shortcodes assigned to them,
//! and REQ-1.3 makes anything else a 403. Both routes take
//! [`Authenticated`](crate::auth::guard::Authenticated), so an unauthenticated
//! request never reaches this module.
//!
//! The 403 is answered before the record is read, and it always will be: an
//! authorization check that runs after a lookup leaks the project's existence
//! through the difference between 403 and 404. Here there is nothing to look up
//! yet, so the ordering costs nothing to establish now and would cost a rewrite
//! to establish later.

use axum::extract::{Path, State};
use axum::response::Response;
use editor_core::records::is_valid_shortcode;

use crate::auth::guard::Authenticated;
use crate::AppState;

/// What a depositor is told about a project that is not theirs.
///
/// Safe to say plainly: they named the shortcode to get here, so it tells them
/// nothing they did not supply, and a vaguer message would send them to RDU
/// without knowing what to ask for.
const NOT_ASSIGNED: &str = "This project is not assigned to your account. RDU assigns projects to depositors; ask \
                            them if you should have access to it.";

/// `GET /projects` — what this account may edit.
pub(crate) async fn list(State(state): State<AppState>, Authenticated(user): Authenticated) -> Response {
    // Not one page with a branch inside it: an RDU account's `shortcodes` is
    // empty by design (REQ-4.2), so rendering it through the depositor's list
    // would tell an administrator they have no projects.
    let content = if user.is_rdu() {
        editor_web::pages::projects::rdu_overview()
    } else {
        editor_web::pages::projects::assigned(&user.shortcodes)
    };
    crate::render(
        &state,
        "Projects — DaSCH Metadata Editor",
        axum::http::StatusCode::OK,
        Some(&user),
        content,
    )
}

/// `GET /projects/{shortcode}` — one project, if this account may reach it.
pub(crate) async fn detail(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    Path(shortcode): Path<String>,
) -> Response {
    // Shape first, so a path segment that could never name a project is a 404
    // rather than a 403. A 403 for `/projects/../etc/passwd` would assert that
    // such a project exists and is merely closed to this account.
    if !is_valid_shortcode(&shortcode) {
        return crate::not_found(State(state)).await;
    }
    // Nothing checks that the project *exists* — the published set is not
    // readable here yet, so a well-formed shortcode naming no project renders
    // the placeholder. The record lookup, and the 404 that goes with it, arrive
    // with the form. Ordering is what matters and it is already right: the
    // lookup goes after this check, or the gap between 403 and 404 becomes an
    // oracle for which projects exist.
    if !user.may_reach(&shortcode) {
        // REQ-1.3. Logged because a depositor repeatedly reaching for projects
        // that are not theirs is worth seeing, and the two identifiers here are
        // both non-personal: an opaque account id and a shortcode.
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused a project that is not assigned to this account"
        );
        return crate::forbidden(&state, &user, NOT_ASSIGNED);
    }
    crate::render(
        &state,
        &format!("Project {shortcode} — DaSCH Metadata Editor"),
        axum::http::StatusCode::OK,
        Some(&user),
        editor_web::pages::projects::project(&shortcode),
    )
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use editor_core::records::Role;
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
    async fn test_a_depositor_opens_a_project_assigned_to_them() {
        let (state, _) = test_state("project-allowed").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801", "080C"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = as_session(&app, "/projects/0801", &session).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("Project 0801"));
    }

    #[tokio::test]
    async fn test_a_depositor_reaching_an_unassigned_project_gets_a_403_page_with_a_way_back() {
        // REQ-1.3 asks for the status. The page is because a bare 403 is a dead
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

        assert_eq!(as_session(&app, "/projects/080C", &session).await.status(), StatusCode::OK);
        assert_eq!(as_session(&app, "/projects/080c", &session).await.status(), StatusCode::OK);
        assert_eq!(
            as_session(&app, "/projects/080E", &session).await.status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn test_rdu_opens_any_project_without_an_assignment() {
        // REQ-4.2: RDU access is role-based, not per-project, which is why an
        // RDU account's assignment set is empty.
        let (state, _) = test_state("project-rdu").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        assert_eq!(as_session(&app, "/projects/0803", &session).await.status(), StatusCode::OK);
        assert_eq!(as_session(&app, "/projects/0801a", &session).await.status(), StatusCode::OK);
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
        let (state, _) = test_state("list-depositor").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["0801", "080C"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains(r#"href="/projects/0801""#), "{body}");
        assert!(body.contains(r#"href="/projects/080C""#), "{body}");
        assert!(!body.contains("/projects/0803"), "{body}");
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
        // REQ-6.10. The two identifiers are an opaque account id and a
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
}
