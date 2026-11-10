//! `GET /projects`, and the `/projects/{shortcode}` redirect into the form.
//!
//! What lands here is the **scope**: a depositor is confined to the shortcodes
//! assigned to them, and anything else is a 403. Both routes read the published
//! set for the projects' names, and both take [`Authenticated`], so an
//! unauthenticated request never reaches this module.
//!
//! The 403 is answered before the record is read: an authorization check that
//! runs after a lookup leaks the project's existence through the difference
//! between 403 and 404.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::Utc;
use editor_core::draft::ProjectDraft;
use editor_core::published::ProjectSummary;
use editor_core::records::{normalize_shortcode, CreatedProject, DraftRecord};
use editor_core::repository::{
    ApprovedRecordRepository, CreatedProjectRepository, DraftRepository, Removal, RepositoryError, SubmissionRepository,
};
use editor_core::status::{depositor_state, Comparison, ProjectState};
use shared_metadata::is_valid_shortcode;

use crate::handlers::auth::{Authenticated, Rdu};
use crate::handlers::{storage_error, StorageSurface};
use crate::pages::projects::{AssignedProject, CreatedRow, NewProjectErrors, NewProjectFields};
use crate::shell::EditorState;

/// What a depositor is told about a project that is not theirs.
///
/// Safe to say plainly: they named the shortcode to get here, so it tells them
/// nothing they did not supply, and a vaguer message would send them to RDU
/// without knowing what to ask for.
pub(crate) const NOT_ASSIGNED: &str =
    "This project is not assigned to your account. RDU assigns projects to depositors; ask \
                            them if you should have access to it.";

/// `GET /projects`'s query: `removed` names the created project a removal just redirected from.
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct ListParams {
    removed: Option<String>,
}

/// `GET /projects` — what this account may edit.
pub(crate) async fn list(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Query(params): Query<ListParams>,
) -> Response {
    // Not one page with a branch inside it: an RDU account's `assigned_shortcodes` is
    // empty by design, so rendering it through the depositor's list
    // would tell an administrator they have no projects.
    // A line filled at load is not reliably announced; the title is.
    let mut title = "Projects — DaSCH Metadata Editor".to_string();
    let content = if user.actor.is_rdu() {
        let rows: Vec<_> = state.published.summaries().collect();
        let created = match created_rows(&state).await {
            Ok(created) => created,
            Err(error) => return storage_error(&state, &user, SURFACE, "read the created projects", &error),
        };
        // Only for a shortcode no registration and no published project holds, so the line never
        // contradicts the list below it.
        let removed = match params.removed.as_deref().filter(|code| is_valid_shortcode(code)) {
            Some(code) if state.published.get(code).is_none() => {
                match CreatedProjectRepository::find(&*state.editor, &normalize_shortcode(code)).await {
                    Ok(None) => Some(code),
                    Ok(Some(_)) => None,
                    Err(error) => return storage_error(&state, &user, SURFACE, "read the created projects", &error),
                }
            }
            _ => None,
        };
        if let Some(shortcode) = removed {
            title = format!("Project {shortcode} removed — {title}");
        }
        crate::pages::projects::rdu_overview(&rows, &created, removed)
    } else {
        // A row per assigned shortcode the published set or a created project carries; the
        // rest are named, since an assignment naming no project is a mistake only RDU can fix.
        let registered = match CreatedProjectRepository::list(&*state.editor).await {
            Ok(registered) => registered,
            Err(error) => return storage_error(&state, &user, SURFACE, "read the created projects", &error),
        };
        let mut created = Vec::new();
        let mut unknown = Vec::new();
        for assigned in &user.actor.assigned_shortcodes {
            let key = normalize_shortcode(assigned);
            if state.published.get(&key).is_some() {
                continue;
            }
            match registered.iter().find(|project| project.shortcode == key) {
                Some(project) => match created_identity(&state, project).await {
                    Ok(identity) => created.push(identity),
                    Err(error) => return storage_error(&state, &user, SURFACE, "read the created projects", &error),
                },
                None => unknown.push(assigned.as_str()),
            }
        }
        let mut summaries: Vec<_> = state.published.summaries_for(&user.actor.assigned_shortcodes).collect();
        summaries.extend(created.iter().map(|(shortcode, name)| ProjectSummary {
            shortcode,
            name,
            status: crate::pages::projects::NOT_PUBLISHED_YET,
        }));
        let mut rows = Vec::with_capacity(summaries.len());
        for summary in summaries {
            // One read set per row rather than one for the page: a depositor
            // holds a handful of assigned projects, and the bulk query this would
            // otherwise need is three new ports for a list that is never long.
            match project_state(&state, summary.shortcode).await {
                Ok(project) => rows.push(AssignedProject { summary, state: project }),
                Err(error) => return storage_error(&state, &user, SURFACE, "read this project's state", &error),
            }
        }
        crate::pages::projects::assigned(&rows, &unknown)
    };
    crate::shell::render(&state, &title, axum::http::StatusCode::OK, Some(&user), content)
}

/// `GET /projects/{shortcode}` — a redirect to the first form section.
///
/// A redirect rather than a page, so exactly one place decides where a project
/// link lands, and the scheme in `docs/src/editor/routing.md` keeps the
/// form's own URLs section-scoped and bookmarkable.
///
/// The redirect target is deliberately the *same* section for both audiences — a
/// destination that depended on the role is one more thing to get wrong in a link
/// shared between a depositor and a reviewer.
pub(crate) async fn detail(
    State(state): State<EditorState>,
    Authenticated(user): Authenticated,
    Path(shortcode): Path<String>,
) -> Response {
    // Shape first, so a path segment that could never name a project is a 404
    // rather than a 403. A 403 for `/projects/../etc/passwd` would assert that
    // such a project exists and is merely closed to this account.
    if !is_valid_shortcode(&shortcode) {
        return crate::shell::not_found(State(state)).await;
    }
    // The authorization check runs before anything is read: 404 for a shortcode
    // that is not published and 403 for one that is would make the pair an oracle
    // for which projects exist, to a reader who is not allowed to know.
    //
    // Nothing answers 404 for an unknown shortcode either: a project may exist
    // only locally, so "absent from the published set" is not "does not exist",
    // and the section handler opens such a project blank rather than refusing
    // it.
    if !user.actor.may_reach(&shortcode) {
        // Logged because a depositor repeatedly reaching for projects
        // that are not theirs is worth seeing, and the two identifiers here are
        // both non-personal: an opaque account id and a shortcode.
        tracing::info!(
            auth.subject = %user.id,
            project.shortcode = %shortcode,
            "refused a project that is not assigned to this account"
        );
        return crate::shell::forbidden(&state, &user, NOT_ASSIGNED);
    }
    let audience = if user.actor.is_rdu() {
        editor_core::form::registry::Audience::RduOnly
    } else {
        editor_core::form::registry::Audience::Everyone
    };
    let section = editor_core::form::registry::first_section(audience);
    axum::response::Redirect::to(&format!("/projects/{shortcode}/sections/{}", section.id)).into_response()
}

/// `GET /states` — what each state means and how long Online takes.
///
/// Behind [`Authenticated`] like the rest: it explains a depositor's own
/// projects, and the editor has no public pages besides the login flow.
pub(crate) async fn states(State(state): State<EditorState>, Authenticated(user): Authenticated) -> Response {
    crate::shell::render(
        &state,
        "What the states mean — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user),
        crate::pages::states::explanation(),
    )
}

const NEW_PROJECT_TITLE: &str = "Create a project — DaSCH Metadata Editor";

/// `GET /projects/new` — the empty create form. RDU only: [`Rdu`] answers a depositor with the
/// 403 page.
pub(crate) async fn new_form(State(state): State<EditorState>, Rdu(user): Rdu) -> Response {
    crate::shell::render(
        &state,
        NEW_PROJECT_TITLE,
        StatusCode::OK,
        Some(&user),
        crate::pages::projects::new_project(NewProjectFields::default(), &NewProjectErrors::default()),
    )
}

/// The create form's body. Defaulted, so a body missing a field is refused with a field error
/// rather than a bare 422.
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct NewProjectForm {
    #[serde(default)]
    shortcode: String,
    #[serde(default)]
    name: String,
}

/// `POST /projects/new` — register a project and open its seeded draft, landing on its overview.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "create project",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn create(
    State(state): State<EditorState>,
    Rdu(user): Rdu,
    Form(form): Form<NewProjectForm>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    // DSP's spelling: every shortcode it assigns is upper-case, and every lookup folds case.
    let shortcode = form.shortcode.trim().to_ascii_uppercase();
    let shortcode = shortcode.as_str();
    let name = form.name.trim();
    let fields = NewProjectFields { shortcode, name };

    let errors = match creation_refusals(&state, shortcode, name).await {
        Ok(errors) => errors,
        Err(error) => return storage_error(&state, &user, SURFACE, "check a shortcode before creating it", &error),
    };
    if errors.shortcode.is_some() || errors.name.is_some() {
        span.record("form.outcome", "refused");
        return refused_creation(&state, &user, fields, &errors);
    }
    span.record("project.shortcode", shortcode);

    let key = normalize_shortcode(shortcode);
    let now = Utc::now();
    let project = CreatedProject {
        shortcode: key.clone(),
        name: name.to_string(),
        created_by: Some(user.id),
        created_at: now,
    };
    let seed = editor_core::creation::seed(shortcode, name);
    let draft = DraftRecord {
        shortcode: key,
        payload: serde_json::to_string(&seed).expect("a draft always serializes"),
        updated_by: Some(user.id),
        created_at: now,
        updated_at: now,
    };
    match CreatedProjectRepository::create(&*state.editor, &project, &draft).await {
        Ok(()) => {
            span.record("form.outcome", "created");
            tracing::info!("created a project");
            let section = editor_core::form::registry::first_section(editor_core::form::registry::Audience::RduOnly);
            Redirect::to(&format!("/projects/{shortcode}/sections/{}", section.id)).into_response()
        }
        // A registration or a draft written between the checks and this write.
        Err(RepositoryError::Conflict { .. }) => {
            span.record("form.outcome", "refused");
            let errors = NewProjectErrors { shortcode: Some(already_in_editor(shortcode)), name: None };
            refused_creation(&state, &user, fields, &errors)
        }
        Err(error) => storage_error(&state, &user, SURFACE, "create a project", &error),
    }
}

/// What stops `shortcode` and `name` from becoming a project, per field. The shortcode is checked
/// against the published set first, then against every table that keys on it, since an assigned
/// depositor may already have saved a draft under it.
async fn creation_refusals(
    state: &EditorState,
    shortcode: &str,
    name: &str,
) -> Result<NewProjectErrors, RepositoryError> {
    let name = name.is_empty().then(|| "Enter the project's name.".to_string());
    let shortcode = if shortcode.is_empty() {
        Some("Enter a shortcode.".to_string())
    } else if !is_valid_shortcode(shortcode) {
        Some(format!(
            "A shortcode has only letters and digits, at most {} of them.",
            shared_metadata::MAX_SHORTCODE_LEN
        ))
    } else if shortcode.eq_ignore_ascii_case(RESERVED_SHORTCODE) {
        // `/projects/new` is the create form, so a project under this shortcode has no overview
        // link.
        Some(format!(
            "\"{shortcode}\" cannot be a shortcode: the editor uses it for this form."
        ))
    } else if state.published.get(shortcode).is_some() {
        Some(format!("Project {shortcode} is already published."))
    } else if let Some(holder) = editor_core::creation::pid_holder(&state.published, shortcode) {
        Some(format!(
            "Shortcode {shortcode} would give the new project the same permanent address (ARK) as the published \
             project {holder}. Choose another shortcode."
        ))
    } else {
        let key = normalize_shortcode(shortcode);
        if CreatedProjectRepository::find(&*state.editor, &key).await?.is_some() {
            Some(format!("Project {shortcode} has already been created."))
        } else if SubmissionRepository::find_by_shortcode(&*state.editor, &key).await?.is_some()
            || !ApprovedRecordRepository::find_by_shortcode(&*state.editor, &key)
                .await?
                .is_empty()
        {
            Some(already_in_editor(shortcode))
        } else if DraftRepository::find(&*state.editor, &key).await?.is_some() {
            Some(format!(
                "Project {shortcode} already has a draft in the editor, saved by a depositor the shortcode is \
                 assigned to. Open it at /projects/{shortcode} and discard the draft there to create the project."
            ))
        } else {
            None
        }
    };
    Ok(NewProjectErrors { shortcode, name })
}

const RESERVED_SHORTCODE: &str = "new";

fn already_in_editor(shortcode: &str) -> String {
    format!("Project {shortcode} already has a draft, a submission or an approved record in the editor.")
}

/// The form again with what was typed and why it was refused. 200 rather than 422: a browser shows
/// the redisplayed form either way, and the outcome is on the span.
fn refused_creation(
    state: &EditorState,
    user: &crate::handlers::auth::Account,
    fields: NewProjectFields<'_>,
    errors: &NewProjectErrors,
) -> Response {
    crate::shell::render(
        state,
        NEW_PROJECT_TITLE,
        StatusCode::OK,
        Some(user),
        crate::pages::projects::new_project(fields, errors),
    )
}

/// A created project's shortcode spelling and name as its draft holds them, the registration's
/// where the draft holds none.
async fn created_identity(state: &EditorState, project: &CreatedProject) -> Result<(String, String), RepositoryError> {
    let draft = DraftRepository::find(&*state.editor, &project.shortcode)
        .await?
        .and_then(|record| serde_json::from_str::<ProjectDraft>(&record.payload).ok());
    let shown = |member, registered| editor_core::creation::shown(draft.as_ref(), member, registered).to_string();
    Ok((shown("shortcode", &project.shortcode), shown("name", &project.name)))
}

/// Every created project as the RDU overview lists it.
async fn created_rows(state: &EditorState) -> Result<Vec<CreatedRow>, RepositoryError> {
    let mut rows = Vec::new();
    for project in CreatedProjectRepository::list(&*state.editor).await? {
        let (shortcode, name) = created_identity(state, &project).await?;
        rows.push(CreatedRow {
            shortcode,
            name,
            created_at: crate::shell::format_instant(project.created_at),
            removable: CreatedProjectRepository::removable(&*state.editor, &project.shortcode).await?,
        });
    }
    Ok(rows)
}

const REMOVAL_REFUSED: &str = "This project has a submission or an approved record, so it cannot be removed. Reject \
                               the submission, or discard the approved record, first.";

/// `GET /projects/{shortcode}/remove` — the confirmation before a created project is removed.
/// Checks what makes it removable, so the confirmation never offers what the write would refuse.
pub(crate) async fn remove_form(
    State(state): State<EditorState>,
    Rdu(user): Rdu,
    Path(shortcode): Path<String>,
) -> Response {
    let project = match registration(&state, &user, &shortcode).await {
        Ok(project) => project,
        Err(response) => return response,
    };
    let refusal = match CreatedProjectRepository::removable(&*state.editor, &project.shortcode).await {
        Ok(true) => None,
        Ok(false) => Some(REMOVAL_REFUSED),
        Err(error) => return storage_error(&state, &user, SURFACE, "check a created project's work", &error),
    };
    confirm_removal(&state, &user, &shortcode, &project.name, refusal)
}

/// `POST /projects/{shortcode}/remove` — remove a created project
/// ([`CreatedProjectRepository::remove`]) while it has no submission and no approved record.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "remove created project",
        auth.actor = tracing::field::Empty,
        project.shortcode = tracing::field::Empty,
        form.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn remove(
    State(state): State<EditorState>,
    Rdu(user): Rdu,
    Path(shortcode): Path<String>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(user.id));
    let project = match registration(&state, &user, &shortcode).await {
        Ok(project) => project,
        Err(response) => return response,
    };
    span.record("project.shortcode", shortcode.as_str());
    match CreatedProjectRepository::remove(&*state.editor, &project.shortcode, Utc::now()).await {
        Ok(Removal::Removed) => {
            span.record("form.outcome", "removed");
            tracing::info!("removed a created project and its draft");
            Redirect::to(&format!("/projects?removed={shortcode}")).into_response()
        }
        Ok(Removal::Pending) => {
            span.record("form.outcome", "refused");
            confirm_removal(&state, &user, &shortcode, &project.name, Some(REMOVAL_REFUSED))
        }
        // Removed by someone else since `registration` read it.
        Ok(Removal::NotFound) => crate::shell::not_found(State(state)).await,
        Err(error) => storage_error(&state, &user, SURFACE, "remove a created project", &error),
    }
}

/// The registration `shortcode` names, or the response that ends the request: a 404 for a path
/// that is not a shortcode or names no created project.
async fn registration(
    state: &EditorState,
    user: &crate::handlers::auth::Account,
    shortcode: &str,
) -> Result<CreatedProject, Response> {
    if !is_valid_shortcode(shortcode) {
        return Err(crate::shell::not_found(State(state.clone())).await);
    }
    match CreatedProjectRepository::find(&*state.editor, &normalize_shortcode(shortcode)).await {
        Ok(Some(project)) => Ok(project),
        Ok(None) => Err(crate::shell::not_found(State(state.clone())).await),
        Err(error) => Err(storage_error(state, user, SURFACE, "read a created project", &error)),
    }
}

fn confirm_removal(
    state: &EditorState,
    user: &crate::handlers::auth::Account,
    shortcode: &str,
    name: &str,
    refusal: Option<&str>,
) -> Response {
    crate::shell::render(
        state,
        "Remove a project — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(user),
        crate::pages::projects::confirm_removal(shortcode, name, refusal),
    )
}

/// The state to show a depositor for one project.
///
/// Three reads and the published set. The comparison is made against the
/// **approved record** where there is one, because that is the only local row
/// whose publication is in question: a draft differing from published data is
/// just an edit in progress, and comparing it would make every unsaved change
/// look like a pending release.
pub(crate) async fn project_state(state: &EditorState, shortcode: &str) -> Result<ProjectState, RepositoryError> {
    // The three record tables key on the *normalized* shortcode, while the
    // summary carries the published set's own spelling. Querying `080C` against
    // rows stored as `080c` finds nothing, which would read as "no local record"
    // and report those projects Online no matter what their depositor had
    // pending.
    let key = normalize_shortcode(shortcode);
    let submission = SubmissionRepository::find_by_shortcode(&*state.editor, &key)
        .await?
        .map(|s| s.state);
    let has_draft = DraftRepository::find(&*state.editor, &key).await?.is_some();
    let (approved, comparison) = approved_comparison(state, &key).await?;

    Ok(depositor_state(submission, approved, has_draft, &comparison))
}

/// How a project's newest approved record compares against the published set,
/// and whether it holds one at all. `last()` is the newest: an approval
/// supersedes the earlier record.
///
/// `key` is a normalized shortcode. A record whose payload cannot be parsed is
/// treated as no record rather than as an error: the page's job is to render a
/// state, and refusing the whole list over one unreadable row would take every
/// other project down with it. The startup pass is where an unreadable payload
/// is reported.
async fn approved_comparison(state: &EditorState, key: &str) -> Result<(bool, Comparison), RepositoryError> {
    let approved = ApprovedRecordRepository::find_by_shortcode(&*state.editor, key).await?;
    let local = approved
        .last()
        .and_then(|record| serde_json::from_str::<ProjectDraft>(&record.payload).ok());
    Ok((
        !approved.is_empty(),
        Comparison::classify(state.published.get(key), local.as_ref()),
    ))
}

const SURFACE: StorageSurface = StorageSurface {
    log: "the project list could not reach storage",
    noun: "page",
};

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use editor_core::draft::ProjectDraft;
    use editor_core::records::{normalize_shortcode, DraftRecord};
    use editor_core::repository::{CreatedProjectRepository, DraftRepository};
    use editor_ports::Role;
    use tower::ServiceExt;

    use crate::fixtures::{
        a_session, a_user, body_string, capture_logs, get, location, test_app, test_state, with_cookie,
    };

    async fn as_session(app: &axum::Router, uri: &str, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(get(uri), crate::fixtures::SESSION, session))
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
        // Both audiences see `overview` first.
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
        // A bare 403 is a dead end in a browser: the reader is signed in and has
        // nothing to press.
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
    async fn test_the_assigned_project_check_ignores_case_so_a_typed_shortcode_still_works() {
        // The published set mixes `080C` with `0801a`; an RDU member typing an
        // assigned shortcode cannot be expected to get the case right, and getting it
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
    async fn test_rdu_opens_any_project_without_an_assigned_project() {
        // RDU access is role-based, not per-project, which is why an
        // RDU account's assigned-project set is empty.
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
    async fn test_the_list_shows_a_depositor_their_assigned_projects_and_nothing_else() {
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
    async fn test_an_assigned_shortcode_naming_no_project_is_named_and_not_a_blank_row() {
        let (state, _) = test_state("list-unpublished").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["9999"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains("DaSCH has no project with the shortcode"), "{body}");
        assert!(!body.contains("No projects are assigned"), "{body}");
        assert!(!body.contains("<table"), "{body}");
    }

    #[tokio::test]
    async fn a_depositor_sees_their_assigned_created_project_with_its_state() {
        let (state, _) = test_state("list-created").await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let rdu_session = a_session(&state, rdu.id).await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["08ab", "0801d"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);
        post_as(&app, "/projects/new", "shortcode=08AB&name=Basler+Fotoarchiv", &rdu_session).await;

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains(r#"href="/projects/08AB""#), "{body}");
        assert!(body.contains("Basler Fotoarchiv"), "{body}");
        assert!(body.contains("Not published yet"), "{body}");
        assert!(body.contains(r#"href="/projects/0801d""#), "{body}");
        assert!(!body.contains("DaSCH has no project"), "{body}");
        assert_eq!(
            as_session(&app, "/projects/08AB/sections/overview", &session).await.status(),
            StatusCode::OK
        );
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
        // An RDU account's assigned-project set is empty by design, so rendering it
        // through the depositor's list would read as "you have none".
        let (state, _) = test_state("list-rdu").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let body = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(body.contains("role-based"), "{body}");
        assert!(!body.contains("No projects are assigned"), "{body}");
        assert!(!body.contains("DaSCH has no project"), "{body}");
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
    async fn a_mixed_case_assigned_project_still_finds_its_local_records() {
        // `drafts` keys on the normalized shortcode while the published set
        // spells `080C` with a capital. Querying the stored spelling finds no
        // draft, so the project reports Online while its depositor has unsaved
        // work.
        let (state, _) = test_state("project-state-case").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["080C"]).await;
        let session = a_session(&state, user.id).await;
        DraftRepository::upsert(
            &*state.editor,
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
        // The only place a depositor sees the result of the startup discard.
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
        // A page nothing links to does not explain anything, so the link is part
        // of it.
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
        assert!(body.contains("same working day"), "REQ-2.6 requires the expected wait: {body}");
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

    async fn post_as(app: &axum::Router, uri: &str, form: &str, session: &str) -> axum::response::Response {
        app.clone()
            .oneshot(with_cookie(crate::fixtures::post(uri, form), crate::fixtures::SESSION, session))
            .await
            .expect("the request should complete")
    }

    #[test]
    fn every_static_segment_under_projects_is_a_refused_shortcode() {
        // A route `/projects/<x>` shadows the overview link of a project created as `<x>`.
        for path in crate::routes::route_paths() {
            let Some(segment) = path
                .strip_prefix("/projects/")
                .map(|rest| rest.split('/').next().unwrap_or(rest))
            else {
                continue;
            };
            if !segment.starts_with('{') {
                assert!(
                    segment.eq_ignore_ascii_case(super::RESERVED_SHORTCODE),
                    "{path}: refuse {segment} at creation"
                );
            }
        }
    }

    #[tokio::test]
    async fn a_depositor_cannot_reach_project_creation_and_nothing_is_written() {
        let (state, _) = test_state("create-depositor").await;
        let user = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["1111"]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        assert_eq!(
            as_session(&app, "/projects/new", &session).await.status(),
            StatusCode::FORBIDDEN
        );
        let response = post_as(&app, "/projects/new", "shortcode=1111&name=Mine", &session).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(CreatedProjectRepository::list(&*state.editor).await.unwrap().is_empty());
        assert!(DraftRepository::find(&*state.editor, "1111").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rdu_creates_a_project_and_lands_on_its_seeded_overview() {
        let (state, _) = test_state("create-rdu").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        let response = post_as(&app, "/projects/new", "shortcode=+08ab+&name=Basler+Fotoarchiv", &session).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/projects/08AB/sections/overview"));

        let created = CreatedProjectRepository::find(&*state.editor, "08ab").await.unwrap().unwrap();
        assert_eq!(created.name, "Basler Fotoarchiv");
        assert_eq!(created.created_by, Some(user.id));
        let draft = DraftRepository::find(&*state.editor, "08ab").await.unwrap().unwrap();
        let draft: ProjectDraft = serde_json::from_str(&draft.payload).unwrap();
        assert_eq!(draft, editor_core::creation::seed("08AB", "Basler Fotoarchiv"));
        assert_eq!(
            draft.get("pid"),
            Some(&serde_json::json!("https://ark.dasch.swiss/ark:/72163/1/08AB"))
        );

        let body = body_string(as_session(&app, "/projects/08AB/sections/overview", &session).await).await;
        assert!(body.contains("Basler Fotoarchiv"), "{body}");
    }

    #[tokio::test]
    async fn a_shortcode_that_is_taken_or_malformed_is_refused_with_a_field_error() {
        let (state, _) = test_state("create-refused").await;
        let user = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, user.id).await;
        let app = test_app(&state);

        DraftRepository::upsert(
            &*state.editor,
            &DraftRecord {
                shortcode: "2222".to_string(),
                payload: "{}".to_string(),
                updated_by: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();
        editor_core::repository::SubmissionRepository::create(
            &*state.editor,
            &editor_core::records::Submission {
                id: uuid::Uuid::new_v4(),
                shortcode: "3333".to_string(),
                payload: "{}".to_string(),
                state: editor_core::records::SubmissionState::Submitted,
                submitted_by: None,
                submitted_at: chrono::Utc::now(),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
                review_state: None,
            },
        )
        .await
        .unwrap();
        editor_core::repository::ApprovedRecordRepository::create(
            &*state.editor,
            &editor_core::records::ApprovedRecord {
                id: uuid::Uuid::new_v4(),
                shortcode: "4444".to_string(),
                payload: "{}".to_string(),
                approved_by: None,
                approved_at: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();
        let first = post_as(&app, "/projects/new", "shortcode=5555&name=First", &session).await;
        assert_eq!(first.status(), StatusCode::SEE_OTHER);

        for (shortcode, expected) in [
            ("0101", "already published"),
            ("0801", "same permanent address (ARK) as the published"),
            ("080c", "already published"),
            ("08-1", "only letters and digits"),
            ("", "Enter a shortcode"),
            ("2222", "discard the draft there"),
            ("3333", "already has a draft, a submission"),
            ("NEW", "cannot be a shortcode"),
            ("4444", "an approved record"),
            ("5555", "already been created"),
        ] {
            let response = post_as(&app, "/projects/new", &format!("shortcode={shortcode}&name=Again"), &session).await;
            assert_eq!(response.status(), StatusCode::OK, "{shortcode}");
            let body = body_string(response).await;
            assert!(body.contains(expected), "{shortcode}: {body}");
            assert!(body.contains(r#"aria-invalid="true""#), "{shortcode}: {body}");
            assert!(body.contains(r#"value="Again""#), "the typed name is kept: {body}");
        }
        let blank = body_string(post_as(&app, "/projects/new", "shortcode=6666&name=+", &session).await).await;
        assert!(
            blank.contains("Enter the project&#39;s name") || blank.contains("Enter the project's name"),
            "{blank}"
        );

        let registered: Vec<_> = CreatedProjectRepository::list(&*state.editor)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.shortcode)
            .collect();
        assert_eq!(registered, ["5555"]);
        assert_eq!(
            DraftRepository::find(&*state.editor, "2222").await.unwrap().unwrap().payload,
            "{}",
            "a refused creation leaves an existing draft alone"
        );
    }

    #[tokio::test]
    async fn rdu_lists_a_created_project_under_new_projects_and_removes_it_with_its_draft() {
        let (state, _) = test_state("created-remove").await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, rdu.id).await;
        let depositor = a_user(&state, "d@example.test", "A Depositor", Role::Depositor, &["08AB"]).await;
        let depositor_session = a_session(&state, depositor.id).await;
        let app = test_app(&state);
        post_as(&app, "/projects/new", "shortcode=08AB&name=Basler+Fotoarchiv", &session).await;

        let list = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(list.contains(r#"href="/projects/new""#), "{list}");
        let new_projects = list.find("New projects").expect("a New projects group");
        let published = list.find("Every published project").expect("the published list");
        assert!(new_projects < published, "the created projects come first: {list}");
        assert!(list.contains(r#"href="/projects/08AB""#), "{list}");
        assert!(list.contains(r#"href="/projects/08AB/remove""#), "{list}");

        for response in [
            as_session(&app, "/projects/08AB/remove", &depositor_session).await,
            post_as(&app, "/projects/08AB/remove", "", &depositor_session).await,
        ] {
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        let confirm = body_string(as_session(&app, "/projects/08AB/remove", &session).await).await;
        assert!(confirm.contains(r#"action="/projects/08AB/remove""#), "{confirm}");
        assert!(confirm.contains("Basler Fotoarchiv"), "{confirm}");
        assert!(CreatedProjectRepository::find(&*state.editor, "08ab").await.unwrap().is_some());

        let removed = post_as(&app, "/projects/08AB/remove", "", &session).await;
        assert_eq!(removed.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&removed).as_deref(), Some("/projects?removed=08AB"));
        let after = body_string(as_session(&app, "/projects?removed=08AB", &session).await).await;
        assert!(after.contains("Project 08AB removed."), "{after}");
        assert!(
            after.contains("<title>Project 08AB removed — Projects"),
            "announced by the title: {after}"
        );
        let spoofed = body_string(as_session(&app, "/projects?removed=0101", &session).await).await;
        assert!(
            !spoofed.contains("removed."),
            "a published project was never removed: {spoofed}"
        );
        assert!(CreatedProjectRepository::find(&*state.editor, "08ab").await.unwrap().is_none());
        assert!(DraftRepository::find(&*state.editor, "08ab").await.unwrap().is_none());
        let again = post_as(&app, "/projects/08AB/remove", "", &session).await;
        assert_eq!(again.status(), StatusCode::NOT_FOUND, "a repeated removal is not a 500");
    }

    #[tokio::test]
    async fn a_created_project_with_a_submission_is_not_removable() {
        let (state, _) = test_state("created-not-removable").await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, rdu.id).await;
        let app = test_app(&state);
        post_as(&app, "/projects/new", "shortcode=1111&name=Pending", &session).await;
        editor_core::repository::SubmissionRepository::create(
            &*state.editor,
            &editor_core::records::Submission {
                id: uuid::Uuid::new_v4(),
                shortcode: "1111".to_string(),
                payload: "{}".to_string(),
                state: editor_core::records::SubmissionState::Submitted,
                submitted_by: None,
                submitted_at: chrono::Utc::now(),
                reviewed_by: None,
                reviewed_at: None,
                reviewer_note: None,
                review_state: None,
            },
        )
        .await
        .unwrap();

        let list = body_string(as_session(&app, "/projects", &session).await).await;
        assert!(!list.contains(r#"href="/projects/1111/remove""#), "{list}");
        assert!(list.contains("Submitted or approved"), "{list}");

        let refused = body_string(post_as(&app, "/projects/1111/remove", "", &session).await).await;
        assert!(refused.contains("cannot be removed"), "{refused}");
        assert!(!refused.contains(r#"action="/projects/1111/remove""#), "{refused}");
        assert!(CreatedProjectRepository::find(&*state.editor, "1111").await.unwrap().is_some());
        assert!(DraftRepository::find(&*state.editor, "1111").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn removing_a_published_project_is_a_404() {
        let (state, _) = test_state("created-remove-published").await;
        let rdu = a_user(&state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(&state, rdu.id).await;
        let app = test_app(&state);
        let response = post_as(&app, "/projects/0101/remove", "", &session).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
