//! Depositor account management, and the RDU-only guard the whole
//! surface sits behind.
//!
//! An account is created from a name, an address and a set of shortcodes; a duplicate address is
//! refused; removal takes the account and every session belonging to it. Update is here because
//! without it correcting a misspelled name means deleting the account, which destroys its
//! sessions, its assignments and the authorship of everything it touched, to fix a typo.
//!
//! **RDU accounts are read-only here.** The list shows every account, because RDU needs to see who
//! administers the service, but the controls act on depositors only. Configuration is the source
//! of truth for RDU membership: an edit made here would be undone by the next restart, and a
//! removal would either be undone the same way or — for an address no longer listed — leave the
//! product and the configuration disagreeing with nothing to say so. Startup already warns about
//! an `rdu` account the configuration does not list; that stays the channel for it.
//!
//! **What removal leaves behind.** The editor's rows live in a database of their own and keep the
//! author's id, so a depositor's work survives them and the review queue's "last editor", finding
//! no account behind the id, reads as unknown. Two consequences are not covered
//! by that and are not reversible, so the confirmation page names both before the fact: the
//! address is deleted with the row, and it is RDU's only channel to its owner; and a submission
//! they made stays pending with no author, so returning it to the depositor has no recipient — it
//! can be approved or rejected and nothing else.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Form;
use chrono::Utc;
use identity_core::records::{Role, User};
use identity_core::repository::{RepositoryError, UserRepository};
use serde::Deserialize;
use shared_metadata::is_valid_shortcode;
use uuid::Uuid;

use crate::handlers::guard::Rdu;
use crate::pages::depositors as page;
use crate::shell::{format_instant, IdentityState};

/// The longest name the form accepts.
///
/// Not a rule about names — it is a bound on a free-text field that becomes a
/// column and is rendered on a page.
const MAX_NAME_LEN: usize = 200;

const LIST_TITLE: &str = "Accounts — DaSCH Metadata Editor";
const CREATE_TITLE: &str = "Add a depositor — DaSCH Metadata Editor";
const EDIT_TITLE: &str = "Edit depositor — DaSCH Metadata Editor";
const REMOVE_TITLE: &str = "Remove depositor — DaSCH Metadata Editor";
const UNAVAILABLE_TITLE: &str = "Page unavailable — DaSCH Metadata Editor";

const DUPLICATE_EMAIL: &str = "An account already uses that email address.";
const GONE: &str = "That account no longer exists. It may have been removed in another window.";
const STORAGE_FAILED: &str = "The change could not be saved. Try again.";
const RDU_IMMUTABLE: &str = "RDU accounts come from the EDITOR_RDU_EMAILS setting and cannot be changed here.";

/// The form behind create and edit alike.
#[derive(Deserialize)]
pub(crate) struct DepositorForm {
    name: String,
    email: String,
    /// As typed. Parsed by [`parse_shortcodes`], and echoed back verbatim when
    /// the form is rejected so the entry that was wrong is visible.
    #[serde(default)]
    shortcodes: String,
}

impl DepositorForm {
    /// The form as the page re-renders it.
    fn as_fields(&self) -> page::DepositorFields<'_> {
        page::DepositorFields {
            name: self.name.trim(),
            email: self.email.trim(),
            shortcodes: self.shortcodes.trim(),
        }
    }

    /// The form's values, or the message the reader should see.
    fn validate(&self) -> Result<(&str, &str, Vec<String>), String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Enter a name.".to_string());
        }
        if name.chars().count() > MAX_NAME_LEN {
            return Err(format!("The name is too long — keep it to {MAX_NAME_LEN} characters or fewer."));
        }
        let email = self.email.trim();
        if !identity_core::login::is_plausible_address(email) {
            return Err("Enter a valid email address.".to_string());
        }
        let shortcodes = parse_shortcodes(&self.shortcodes)?;
        Ok((name, email, shortcodes))
    }
}

/// Split the shortcode field into assignments, or say which entry is wrong.
///
/// Commas and whitespace both separate, because a list typed by hand arrives
/// either way and refusing one of them teaches nothing.
///
/// Duplicates are dropped rather than refused: `user_shortcodes` has a composite
/// primary key, so a repeated entry would otherwise be a constraint violation
/// surfacing as "the change could not be saved" for something that is not a
/// mistake. Case-insensitively, to match how access is checked.
///
/// The offending entry is quoted in the message. A shortcode is not personal
/// data, and naming it is the difference between a fixable error and a puzzle —
/// this is a page, not a log, so the address-disclosure rule does not bear on it.
fn parse_shortcodes(field: &str) -> Result<Vec<String>, String> {
    let mut assignments: Vec<String> = Vec::new();
    for entry in field.split([',', ' ', '\t', '\n', '\r']).filter(|e| !e.is_empty()) {
        if !is_valid_shortcode(entry) {
            return Err(format!(
                "{entry:?} is not a project shortcode. Shortcodes are letters and digits only, for example 0801."
            ));
        }
        if !assignments.iter().any(|held| held.eq_ignore_ascii_case(entry)) {
            assignments.push(entry.to_string());
        }
    }
    Ok(assignments)
}

/// `GET /depositors` — every account.
pub(crate) async fn list(State(state): State<IdentityState>, Rdu(viewer): Rdu) -> Response {
    let users = match UserRepository::list(&*state.store).await {
        Ok(users) => users,
        Err(error) => return storage_error(&state, &viewer, "list the accounts", &error),
    };

    // Rendered in two passes because the view rows borrow the formatted
    // timestamps and the ids, and both have to outlive the borrow.
    let formatted: Vec<(String, Option<String>)> = users
        .iter()
        .map(|user| (user.id.to_string(), user.last_code_at.map(format_instant)))
        .collect();
    let rows: Vec<page::DepositorRow<'_>> = users
        .iter()
        .zip(&formatted)
        .map(|(user, (id, last_code_at))| page::DepositorRow {
            id,
            name: &user.name,
            email: &user.email,
            role: user.role.as_str(),
            shortcodes: &user.shortcodes,
            last_code_at: last_code_at.as_deref(),
            // RDU membership comes from configuration, so the controls
            // here would be lying about what they can change.
            manageable: !user.is_rdu(),
        })
        .collect();

    crate::shell::render(&state, LIST_TITLE, StatusCode::OK, Some(&viewer.name), page::list(&rows))
}

/// `GET /depositors/new` — the empty create form.
pub(crate) async fn create_form(State(state): State<IdentityState>, Rdu(viewer): Rdu) -> Response {
    let fields = page::DepositorFields { name: "", email: "", shortcodes: "" };
    crate::shell::render(
        &state,
        CREATE_TITLE,
        StatusCode::OK,
        Some(&viewer.name),
        page::create(&fields, None),
    )
}

/// `POST /depositors` — create a depositor.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "depositors create",
        auth.actor = tracing::field::Empty,
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn create(
    State(state): State<IdentityState>,
    Rdu(viewer): Rdu,
    Form(form): Form<DepositorForm>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(viewer.id));

    let (name, email, shortcodes) = match form.validate() {
        Ok(values) => values,
        Err(message) => {
            span.record("auth.outcome", "rejected");
            return rejected_create(&state, &viewer, &form, &message);
        }
    };

    let user = User {
        id: Uuid::new_v4(),
        email: email.to_string(),
        name: name.to_string(),
        // Never from the form. RDU membership is configuration's to decide, so
        // this surface can only ever make a depositor.
        role: Role::Depositor,
        shortcodes,
        failed_logins: 0,
        failed_login_at: None,
        last_code_at: None,
        created_at: Utc::now(),
    };

    match UserRepository::create(&*state.store, &user).await {
        Ok(()) => {
            span.record("auth.subject", tracing::field::display(user.id));
            span.record("auth.outcome", "created");
            tracing::info!(shortcodes = user.shortcodes.len(), "created a depositor account");
            Redirect::to("/depositors").into_response()
        }
        // `email_normalized` is the only unique index on the table, so a conflict
        // here is a duplicate address and nothing else.
        Err(RepositoryError::Conflict { .. }) => {
            span.record("auth.outcome", "duplicate_email");
            tracing::info!("refused a depositor whose address is already registered");
            rejected_create(&state, &viewer, &form, DUPLICATE_EMAIL)
        }
        Err(error) => {
            span.record("auth.outcome", "store_failed");
            tracing::error!(error = %error, "could not create a depositor account");
            rejected_create(&state, &viewer, &form, STORAGE_FAILED)
        }
    }
}

/// `GET /depositors/{id}/edit` — the form, filled in.
pub(crate) async fn edit_form(
    State(state): State<IdentityState>,
    Rdu(viewer): Rdu,
    Path(id): Path<String>,
) -> Response {
    let target = match manageable(&state, &viewer, &id).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    let shortcodes = target.shortcodes.join(", ");
    let fields = page::DepositorFields {
        name: &target.name,
        email: &target.email,
        shortcodes: &shortcodes,
    };
    crate::shell::render(
        &state,
        EDIT_TITLE,
        StatusCode::OK,
        Some(&viewer.name),
        page::edit(&target.id.to_string(), &fields, None),
    )
}

/// `POST /depositors/{id}` — change name, address and assignments.
///
/// See the module docs for why update exists.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "depositors update",
        auth.actor = tracing::field::Empty,
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn update(
    State(state): State<IdentityState>,
    Rdu(viewer): Rdu,
    Path(id): Path<String>,
    Form(form): Form<DepositorForm>,
) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(viewer.id));

    let target = match manageable(&state, &viewer, &id).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    span.record("auth.subject", tracing::field::display(target.id));

    let (name, email, shortcodes) = match form.validate() {
        Ok(values) => values,
        Err(message) => {
            span.record("auth.outcome", "rejected");
            return rejected_edit(&state, &viewer, target.id, &form, &message);
        }
    };

    // Built from the stored record rather than from the form, so the fields this
    // surface does not own survive an edit. The failure counter, its instant, the
    // last-code stamp and the creation time survive because
    // `UserRepository::update`'s statement does not write those columns; what the
    // spread protects is **`role`**, which that statement *does* write, so this is
    // what stops a form ever promoting an account — and what holds the invariant
    // if that `UPDATE` is ever widened.
    let updated = User {
        email: email.to_string(),
        name: name.to_string(),
        shortcodes,
        ..target
    };

    match UserRepository::update(&*state.store, &updated).await {
        Ok(()) => {
            span.record("auth.outcome", "updated");
            tracing::info!(shortcodes = updated.shortcodes.len(), "updated a depositor account");
            Redirect::to("/depositors").into_response()
        }
        Err(RepositoryError::Conflict { .. }) => {
            span.record("auth.outcome", "duplicate_email");
            tracing::info!("refused an address that is already registered to another account");
            rejected_edit(&state, &viewer, updated.id, &form, DUPLICATE_EMAIL)
        }
        Err(RepositoryError::NotFound { .. }) => {
            span.record("auth.outcome", "gone");
            tracing::warn!("the account being edited was removed underneath the form");
            rejected_edit(&state, &viewer, updated.id, &form, GONE)
        }
        Err(error) => {
            span.record("auth.outcome", "store_failed");
            tracing::error!(error = %error, "could not update a depositor account");
            rejected_edit(&state, &viewer, updated.id, &form, STORAGE_FAILED)
        }
    }
}

/// `GET /depositors/{id}/remove` — the confirmation.
///
/// A `GET` because it changes nothing: it reads what the account holds so the
/// consequences are visible before the `POST` that does the work.
pub(crate) async fn remove_form(
    State(state): State<IdentityState>,
    Rdu(viewer): Rdu,
    Path(id): Path<String>,
) -> Response {
    let target = match manageable(&state, &viewer, &id).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    let id = target.id;

    // A failed read here is not fatal: it would leave the page unable to name
    // what is in flight, and a confirmation that silently claims there is
    // nothing is worse than one that says it could not tell.
    let account_id = id.to_string();
    let drafts = match state.authored_work.open_drafts(&account_id).await {
        Ok(drafts) => drafts,
        Err(error) => return storage_error(&state, &viewer, "read this account's drafts", &error),
    };
    let submissions = match state.authored_work.open_submissions(&account_id).await {
        Ok(submissions) => submissions,
        Err(error) => return storage_error(&state, &viewer, "read this account's submissions", &error),
    };

    let impact = page::RemovalImpact {
        name: &target.name,
        email: &target.email,
        draft_shortcodes: &drafts,
        submission_shortcodes: &submissions,
    };
    crate::shell::render(
        &state,
        REMOVE_TITLE,
        StatusCode::OK,
        Some(&viewer.name),
        page::confirm_removal(&target.id.to_string(), &impact),
    )
}

/// `POST /depositors/{id}/remove` — delete the account and its sessions.
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "internal",
        otel.name = "depositors remove",
        auth.actor = tracing::field::Empty,
        auth.subject = tracing::field::Empty,
        auth.outcome = tracing::field::Empty,
    )
)]
pub(crate) async fn remove(State(state): State<IdentityState>, Rdu(viewer): Rdu, Path(id): Path<String>) -> Response {
    let span = tracing::Span::current();
    span.record("auth.actor", tracing::field::display(viewer.id));

    // Re-checked here and not only on the confirmation page: the confirmation is
    // a `GET`, so nothing stops a `POST` arriving without one.
    let target = match manageable(&state, &viewer, &id).await {
        Ok(user) => user,
        Err(response) => {
            span.record("auth.outcome", "refused");
            return response;
        }
    };
    span.record("auth.subject", tracing::field::display(target.id));

    match UserRepository::delete(&*state.store, target.id).await {
        Ok(()) => {
            // Sessions, login codes and shortcode assignments go with it through
            // `ON DELETE CASCADE`, which is the "and every session belonging to it" half.
            span.record("auth.outcome", "removed");
            tracing::info!("removed a depositor account and its sessions");
            Redirect::to("/depositors").into_response()
        }
        Err(RepositoryError::NotFound { .. }) => {
            span.record("auth.outcome", "gone");
            tracing::info!("a removal named an account that was already gone");
            Redirect::to("/depositors").into_response()
        }
        Err(error) => {
            span.record("auth.outcome", "delete_failed");
            tracing::error!(error = %error, "could not remove a depositor account");
            storage_error(&state, &viewer, "remove the account", &error)
        }
    }
}

/// The account `id` names, if this surface may act on it.
///
/// Three refusals in one place, because every route that changes an account
/// needs all three and one route forgetting one is how an RDU row gets edited
/// through a URL typed by hand.
async fn manageable(state: &IdentityState, viewer: &User, id: &str) -> Result<User, Response> {
    // The id is a path segment, so it is whatever was typed. Parsed here rather
    // than by `Path<Uuid>`: a value that is not an id at all should be the same
    // 404 as an id that names nothing, not a deserialization error explaining
    // what a UUID looks like.
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(crate::shell::not_found(state));
    };
    match UserRepository::find_by_id(&*state.store, id).await {
        Ok(Some(user)) if user.is_rdu() => Err(crate::shell::forbidden(state, &viewer.name, RDU_IMMUTABLE)),
        Ok(Some(user)) => Ok(user),
        // Not a 403: a reader who reached this page followed a link from the
        // list, and an account that is not there is gone rather than closed.
        Ok(None) => Err(crate::shell::not_found(state)),
        Err(error) => Err(storage_error(state, viewer, "read the account", &error)),
    }
}

/// The create form, redisplayed with what was typed and why it was refused.
fn rejected_create(state: &IdentityState, viewer: &User, form: &DepositorForm, message: &str) -> Response {
    // 200 rather than 422: this is a form being redisplayed, which is what a
    // browser needs to show it. The outcome is on the span, which is where
    // alerting reads it from.
    crate::shell::render(
        state,
        CREATE_TITLE,
        StatusCode::OK,
        Some(&viewer.name),
        page::create(&form.as_fields(), Some(message)),
    )
}

/// The edit form, redisplayed the same way.
fn rejected_edit(state: &IdentityState, viewer: &User, id: Uuid, form: &DepositorForm, message: &str) -> Response {
    crate::shell::render(
        state,
        EDIT_TITLE,
        StatusCode::OK,
        Some(&viewer.name),
        page::edit(&id.to_string(), &form.as_fields(), Some(message)),
    )
}

/// A storage failure, reported as a page rather than swallowed.
///
/// `what` names the operation in the log, never in the page: an operator needs
/// to know which read failed, and a reader needs to know the screen is not to be
/// trusted, and those are different messages.
///
/// Rendered through the problem page and **not** the 403 one. The two say
/// opposite things, and routing a 500 through the 403 tells an RDU member they
/// do not have access to a page they administer — which sends whoever reads it
/// hunting for a misconfigured account instead of a database that is down.
fn storage_error(state: &IdentityState, viewer: &User, what: &str, error: &impl std::fmt::Display) -> Response {
    tracing::error!(error = %error, operation = what, "an account screen could not reach storage");
    crate::shell::render(
        state,
        UNAVAILABLE_TITLE,
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(&viewer.name),
        deposit_shell::pages::problem::unavailable(
            "The editor could not reach its database, so this page is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_shortcode_list_splits_on_commas_and_whitespace() {
        // Both arrive from a field typed by hand, and refusing one of them
        // teaches the reader nothing.
        assert_eq!(parse_shortcodes("0801, 080C").unwrap(), vec!["0801", "080C"]);
        assert_eq!(parse_shortcodes("0801 080C").unwrap(), vec!["0801", "080C"]);
        assert_eq!(parse_shortcodes(" 0801 ,080C,  ").unwrap(), vec!["0801", "080C"]);
    }

    #[test]
    fn test_an_empty_field_assigns_nothing() {
        // A depositor with no projects is a real state — an account created a
        // moment before the assignments are known.
        assert_eq!(parse_shortcodes("").unwrap(), Vec::<String>::new());
        assert_eq!(parse_shortcodes("  ,, ").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_a_repeated_shortcode_is_dropped_rather_than_refused() {
        // `user_shortcodes` has a composite primary key, so a repeat would
        // otherwise surface as "the change could not be saved" for something
        // that is not a mistake.
        assert_eq!(parse_shortcodes("0801, 0801").unwrap(), vec!["0801"]);
        assert_eq!(parse_shortcodes("080C, 080c").unwrap(), vec!["080C"]);
    }

    #[test]
    fn test_an_entry_that_is_not_a_shortcode_is_named_in_the_message() {
        // A shortcode is not personal data, and this is a page rather than a
        // log, so naming it is the difference between a fixable error and a
        // puzzle.
        let error = parse_shortcodes("0801, not-a-code").expect_err("must be refused");
        assert!(error.contains("not-a-code"), "{error}");
    }

    #[test]
    fn test_a_path_separator_in_the_field_cannot_become_an_assignment() {
        // An assignment is compared against a path segment, so a value with a
        // slash in it could never match one and has no business being stored.
        for field in ["../etc", "0801/0802", "08%2f01"] {
            assert!(parse_shortcodes(field).is_err(), "{field}");
        }
    }

    fn form(name: &str, email: &str, shortcodes: &str) -> DepositorForm {
        DepositorForm {
            name: name.to_string(),
            email: email.to_string(),
            shortcodes: shortcodes.to_string(),
        }
    }

    #[test]
    fn test_a_complete_form_validates_to_its_three_values() {
        let form = form("  A Depositor ", " a@example.test ", "0801");
        let (name, email, shortcodes) = form.validate().expect("valid");
        assert_eq!(name, "A Depositor");
        assert_eq!(email, "a@example.test");
        assert_eq!(shortcodes, vec!["0801".to_string()]);
    }

    #[test]
    fn test_a_missing_name_or_an_impossible_address_is_refused() {
        assert!(form("", "a@example.test", "").validate().is_err());
        assert!(form("   ", "a@example.test", "").validate().is_err());
        assert!(form("A", "not-an-address", "").validate().is_err());
        assert!(form("A", "", "").validate().is_err());
    }

    #[test]
    fn test_an_absurdly_long_name_is_refused() {
        let long = "a".repeat(MAX_NAME_LEN + 1);
        assert!(form(&long, "a@example.test", "").validate().is_err());
        assert!(form(&"a".repeat(MAX_NAME_LEN), "a@example.test", "").validate().is_ok());
    }

    #[test]
    fn test_an_instant_is_shown_in_utc_and_says_so() {
        use chrono::TimeZone;

        let at = Utc.with_ymd_and_hms(2026, 8, 25, 9, 14, 30).unwrap();
        assert_eq!(format_instant(at), "2026-08-25 09:14 UTC");
    }
}

#[cfg(test)]
mod route_tests {
    use axum::body::Body;
    use axum::http::Request;
    use identity_core::cookie;
    use tower::ServiceExt;

    use super::*;
    use crate::fixtures::{
        a_session, a_user, body_string, capture_logs, get, location, post, test_app, test_state, urlencode,
    };

    const DEPOSITOR_EMAIL: &str = "a.depositor@example.test";

    fn as_session(request: Request<Body>, session: &str) -> Request<Body> {
        crate::fixtures::with_cookie(request, cookie::SESSION, session)
    }

    /// An RDU account with a live session.
    async fn rdu(state: &IdentityState) -> (User, String) {
        let user = a_user(state, "rdu@dasch.swiss", "An Admin", Role::Rdu, &[]).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    /// A depositor account with a live session.
    async fn depositor(state: &IdentityState, email: &str, shortcodes: &[&str]) -> (User, String) {
        let user = a_user(state, email, "A Depositor", Role::Depositor, shortcodes).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    fn form_body(name: &str, email: &str, shortcodes: &str) -> String {
        format!(
            "name={}&email={}&shortcodes={}",
            urlencode(name),
            urlencode(email),
            urlencode(shortcodes)
        )
    }

    // ---- Who may reach the surface at all -----------------------------------

    #[tokio::test]
    async fn test_rdu_reaches_the_account_screens() {
        let (state, _) = test_state("dep-rdu-access").await;
        let (_, session) = rdu(&state).await;
        let app = test_app(&state);

        for uri in ["/depositors", "/depositors/new"] {
            let response = app.clone().oneshot(as_session(get(uri), &session)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
        }
    }

    #[tokio::test]
    async fn test_a_depositor_is_refused_every_account_screen() {
        // The role check is on the extractor, so it covers each route by being
        // named in the signature rather than by anyone remembering to add it.
        let (state, _) = test_state("dep-depositor-refused").await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        let (intruder, session) = depositor(&state, "other@example.test", &[]).await;
        let app = test_app(&state);
        let id = target.id;

        let reads = [
            "/depositors".to_string(),
            "/depositors/new".to_string(),
            format!("/depositors/{id}/edit"),
            format!("/depositors/{id}/remove"),
        ];
        for uri in &reads {
            let response = app.clone().oneshot(as_session(get(uri), &session)).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
        }

        let writes = [
            ("/depositors".to_string(), form_body("X", "x@example.test", "")),
            (format!("/depositors/{id}/edit"), form_body("X", "x@example.test", "")),
            (format!("/depositors/{id}/remove"), String::new()),
        ];
        for (uri, body) in &writes {
            let response = app.clone().oneshot(as_session(post(uri, body), &session)).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{uri}");
        }

        assert!(
            UserRepository::find_by_id(&*state.store, target.id).await.unwrap().is_some(),
            "and nothing a refused request asked for may have happened"
        );
        assert_eq!(UserRepository::list(&*state.store).await.unwrap().len(), 2);
        let _ = intruder;
    }

    #[tokio::test]
    async fn test_a_signed_out_visitor_is_sent_to_login_and_back_again() {
        let (state, _) = test_state("dep-anonymous").await;
        let app = test_app(&state);

        let response = app.clone().oneshot(get("/depositors")).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&response).as_deref(), Some("/login?next=/depositors"));
    }

    // ---- Creation -----------------------------------------------------------

    #[tokio::test]
    async fn test_a_duplicate_address_is_refused_however_it_is_capitalised() {
        // The uniqueness lives on the normalized address, so this cannot be defeated by
        // typing it differently.
        let (state, _) = test_state("dep-duplicate").await;
        let (_, session) = rdu(&state).await;
        a_user(&state, DEPOSITOR_EMAIL, "A Depositor", Role::Depositor, &[]).await;
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(
                post("/depositors", &form_body("Someone Else", "A.Depositor@Example.TEST", "0801")),
                &session,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "the form comes back rather than redirecting");
        let body = body_string(response).await;
        assert!(body.contains("already uses that email address"), "{body}");
        // And what was typed is still in the form, so the correction is one edit.
        assert!(body.contains(r#"value="Someone Else""#), "{body}");

        assert_eq!(
            UserRepository::list(&*state.store).await.unwrap().len(),
            2,
            "the RDU account and the one depositor, and no second one"
        );
    }

    #[tokio::test]
    async fn test_a_form_that_cannot_be_stored_creates_nothing_and_says_why() {
        let (state, _) = test_state("dep-invalid").await;
        let (_, session) = rdu(&state).await;
        let app = test_app(&state);

        let cases = [
            (form_body("", DEPOSITOR_EMAIL, ""), "Enter a name"),
            (form_body("A Depositor", "not-an-address", ""), "valid email address"),
            (form_body("A Depositor", DEPOSITOR_EMAIL, "0801, not-a-code"), "not-a-code"),
        ];
        for (body, expected) in cases {
            let response = app
                .clone()
                .oneshot(as_session(post("/depositors", &body), &session))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let rendered = body_string(response).await;
            assert!(rendered.contains(expected), "expected {expected:?} in {rendered}");
        }
        assert_eq!(
            UserRepository::list(&*state.store).await.unwrap().len(),
            1,
            "only the RDU account"
        );
    }

    // ---- Update -------------------------------------------------------------

    #[tokio::test]
    async fn test_an_edit_onto_another_accounts_address_is_refused() {
        // The duplicate-address refusal on the edit path: `email_normalized` is
        // the only unique column on `users`, so pointing one account at
        // another's address hits it.
        let (state, _) = test_state("dep-update-duplicate").await;
        let (_, session) = rdu(&state).await;
        let first = a_user(&state, "first@example.test", "First", Role::Depositor, &[]).await;
        let second = a_user(&state, "second@example.test", "Second", Role::Depositor, &["0801"]).await;
        let app = test_app(&state);

        let response = app
            .clone()
            .oneshot(as_session(
                post(
                    &format!("/depositors/{}/edit", second.id),
                    &form_body("Second", "FIRST@Example.TEST", "0801"),
                ),
                &session,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "the form comes back rather than redirecting");
        let body = body_string(response).await;
        assert!(body.contains("already uses that email address"), "{body}");

        // And neither account moved.
        let unchanged = UserRepository::find_by_id(&*state.store, second.id).await.unwrap().unwrap();
        assert_eq!(unchanged.email, "second@example.test");
        assert_eq!(
            UserRepository::find_by_id(&*state.store, first.id)
                .await
                .unwrap()
                .unwrap()
                .email,
            "first@example.test"
        );
    }

    // ---- Removal ------------------------------------------------------------

    // ---- RDU accounts, and ids that name nothing ----------------------------

    #[tokio::test]
    async fn test_an_rdu_account_cannot_be_edited_or_removed_here() {
        // Configuration is the source of truth. A change here would
        // be undone by the next restart, or would diverge from configuration
        // with nothing to say so.
        let (state, _) = test_state("dep-rdu-immutable").await;
        let (admin, session) = rdu(&state).await;
        let other = a_user(&state, "rdu2@dasch.swiss", "Another Admin", Role::Rdu, &[]).await;
        let app = test_app(&state);

        for id in [admin.id, other.id] {
            for request in [
                as_session(get(&format!("/depositors/{id}/edit")), &session),
                as_session(get(&format!("/depositors/{id}/remove")), &session),
                as_session(
                    post(&format!("/depositors/{id}/edit"), &form_body("X", "x@example.test", "")),
                    &session,
                ),
                as_session(post(&format!("/depositors/{id}/remove"), ""), &session),
            ] {
                let response = app.clone().oneshot(request).await.unwrap();
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "{id}");
            }
        }

        assert_eq!(UserRepository::list(&*state.store).await.unwrap().len(), 2);
        let unchanged = UserRepository::find_by_id(&*state.store, admin.id).await.unwrap().unwrap();
        assert_eq!(unchanged.email, "rdu@dasch.swiss");
    }

    #[tokio::test]
    async fn test_the_list_shows_rdu_accounts_without_controls_and_names_the_setting() {
        // RDU has to see who administers the service; the controls just do not
        // act on them.
        let (state, _) = test_state("dep-list-render").await;
        let (admin, session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &["0801"]).await;
        let app = test_app(&state);

        let body = body_string(app.clone().oneshot(as_session(get("/depositors"), &session)).await.unwrap()).await;
        assert!(body.contains("rdu@dasch.swiss"), "{body}");
        assert!(body.contains(DEPOSITOR_EMAIL), "{body}");
        assert!(body.contains("EDITOR_RDU_EMAILS"), "{body}");
        assert!(body.contains(&format!("/depositors/{}/edit", target.id)), "{body}");
        assert!(!body.contains(&format!("/depositors/{}/edit", admin.id)), "{body}");
    }

    #[tokio::test]
    async fn test_the_list_answers_i_never_got_a_code_without_an_address_in_a_log() {
        // An unconfigured relay and a failed send are each reported; neither covers
        // accepted-then-undelivered, and the address must not go in a log. This column is what
        // is left.
        let (state, _) = test_state("dep-last-code").await;
        let (_, session) = rdu(&state).await;
        let (target, _) = depositor(&state, DEPOSITOR_EMAIL, &[]).await;
        let app = test_app(&state);

        let before = body_string(app.clone().oneshot(as_session(get("/depositors"), &session)).await.unwrap()).await;
        assert!(before.contains("never"), "{before}");

        UserRepository::record_code_issued(&*state.store, target.id, Utc::now())
            .await
            .unwrap();
        let after = body_string(app.clone().oneshot(as_session(get("/depositors"), &session)).await.unwrap()).await;
        assert!(after.contains("UTC"), "{after}");
    }

    #[tokio::test]
    async fn test_an_id_that_names_nothing_is_a_404_whatever_shape_it_is() {
        // A value that is not an id at all is the same "there is no such
        // account" as one that is, not a message explaining what a UUID is.
        let (state, _) = test_state("dep-unknown-id").await;
        let (_, session) = rdu(&state).await;
        let app = test_app(&state);

        for id in [Uuid::new_v4().to_string(), "not-a-uuid".to_string()] {
            for request in [
                as_session(get(&format!("/depositors/{id}/edit")), &session),
                as_session(get(&format!("/depositors/{id}/remove")), &session),
                as_session(post(&format!("/depositors/{id}/remove"), ""), &session),
            ] {
                let response = app.clone().oneshot(request).await.unwrap();
                assert_eq!(response.status(), StatusCode::NOT_FOUND, "{id}");
            }
        }
    }

    // ---- Method discipline, and no address in a log -------------------------

    #[tokio::test]
    async fn test_no_address_reaches_a_log_or_a_span_on_any_account_path() {
        // These handlers take an address in a form body and hold one
        // on every record they touch, so they are the most likely place for one
        // to reach a span field.
        let (state, _) = test_state("dep-no-address-in-logs").await;
        let (_, session) = rdu(&state).await;
        let app = test_app(&state);

        let (logs, guard) = capture_logs();
        // Create, then a duplicate, then an edit, then a removal.
        app.clone()
            .oneshot(as_session(
                post("/depositors", &form_body("A Depositor", DEPOSITOR_EMAIL, "0801")),
                &session,
            ))
            .await
            .unwrap();
        app.clone()
            .oneshot(as_session(
                post("/depositors", &form_body("Someone Else", DEPOSITOR_EMAIL, "")),
                &session,
            ))
            .await
            .unwrap();
        let created = UserRepository::find_by_email(&*state.store, DEPOSITOR_EMAIL)
            .await
            .unwrap()
            .unwrap();
        app.clone()
            .oneshot(as_session(
                post(
                    &format!("/depositors/{}/edit", created.id),
                    &form_body("A Depositor", "renamed@example.test", ""),
                ),
                &session,
            ))
            .await
            .unwrap();
        app.clone()
            .oneshot(as_session(post(&format!("/depositors/{}/remove", created.id), ""), &session))
            .await
            .unwrap();
        drop(guard);

        let lines = logs.lines();
        for address in [DEPOSITOR_EMAIL, "renamed@example.test", "rdu@dasch.swiss"] {
            assert!(
                !lines.iter().any(|line| line.contains(address)),
                "{address} reached a log or a span: {lines:?}"
            );
        }
        // And the account is still traceable by its opaque id.
        assert!(
            lines.iter().any(|line| line.contains(&created.id.to_string())),
            "the account must stay traceable: {lines:?}"
        );
    }
}
