//! Tests that need both capabilities' stores: deleting an account keeps its id on the editor
//! rows, and the pages that show those rows render a kept id that names no account as a
//! missing author.

use axum::http::StatusCode;
use chrono::{DateTime, TimeZone, Utc};
use editor_store::test_support::{seed_draft, seed_entity_proposal, seed_review_round, seed_submission};
use identity_store::test_support::{delete_user, seed_session, seed_user};
use tower::ServiceExt;

use crate::test_support::{
    body_string, get, open_test_db, state_over, test_app, with_cookie, RecordingMailer, TestState,
};

const PROJECT_A: &str = "0801d";
const PROJECT_B: &str = "0803";

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 8, hour, 0, 0).unwrap()
}

fn main_of(body: &str) -> &str {
    let start = body.find("<main").expect("the page should have a <main>");
    let end = body.find("</main>").expect("the page should close its <main>") + "</main>".len();
    &body[start..end]
}

/// Two projects whose editor rows were authored by accounts that are now deleted, the ids of
/// those accounts, and the sessions of the depositor D2 and the reviewer R1 who remain.
struct Fixture {
    state: TestState,
    deleted: [String; 2],
    depositor_session: String,
    reviewer_session: String,
}

async fn deleted_authors(label: &str) -> Fixture {
    let stores = open_test_db(label).await;
    let (identity, editor) = (&*stores.identity, &*stores.editor);
    let both = [PROJECT_A, PROJECT_B];
    let d = seed_user(identity, "d@example.test", "Dora Depositor", "depositor", &both).await;
    let r2 = seed_user(identity, "r2@example.test", "Rudi Reviewer", "rdu", &[]).await;
    let d2 = seed_user(identity, "d2@example.test", "Dana Depositor", "depositor", &both).await;
    let r1 = seed_user(identity, "r1@example.test", "Rita Reviewer", "rdu", &[]).await;

    // Project A: a draft last edited by D, returned to it by R2's changes-requested round.
    seed_draft(editor, PROJECT_A, "{}", Some(&d), at(8), at(10)).await;
    seed_review_round(
        editor,
        PROJECT_A,
        "changes_requested",
        Some("Please add a German description."),
        Some(&r2),
        at(10),
    )
    .await;

    // Project B: D's submission, claimed by R2, with a proposal riding along.
    seed_submission(editor, PROJECT_B, "{}", "in_review", Some(&d), at(11), Some(&r2), Some(at(12))).await;
    let person = r#"{"givenNames":["Ada"],"familyNames":["Lovelace"],"jobTitles":[]}"#;
    seed_entity_proposal(
        editor,
        PROJECT_B,
        "person-417",
        "person",
        "new",
        person,
        "submitted",
        Some(&d),
        at(11),
    )
    .await;

    delete_user(identity, &d).await;
    delete_user(identity, &r2).await;

    let now = Utc::now();
    let depositor_session = seed_session(identity, &d2, now, now + chrono::TimeDelta::hours(12)).await;
    let reviewer_session = seed_session(identity, &r1, now, now + chrono::TimeDelta::hours(12)).await;
    let state = state_over(stores.identity.clone(), stores.editor.clone(), RecordingMailer::new(), |_| {});
    Fixture { state, deleted: [d, r2], depositor_session, reviewer_session }
}

#[tokio::test]
async fn deleting_an_account_leaves_its_id_on_the_editor_rows() {
    let stores = open_test_db("removed_account_rows").await;
    let (identity, editor) = (&*stores.identity, &*stores.editor);
    let author = seed_user(identity, "a@example.test", "Ann Author", "depositor", &["0801"]).await;
    seed_draft(editor, "0801", "{}", Some(&author), at(8), at(8)).await;
    seed_submission(editor, "0803", "{}", "submitted", Some(&author), at(9), None, None).await;

    delete_user(identity, &author).await;

    assert_eq!(identity.count_rows("users").await, 0, "the account is gone");
    assert_eq!(editor.account_column("drafts", "updated_by").await, vec![Some(author.clone())]);
    assert_eq!(editor.account_column("submissions", "submitted_by").await, vec![Some(author)]);
}

async fn page(fixture: &Fixture, uri: &str, session: &str) -> String {
    let response = test_app(&fixture.state)
        .oneshot(with_cookie(get(uri), identity_core::cookie::SESSION, session))
        .await
        .expect("the app should answer");
    assert_eq!(response.status(), StatusCode::OK);
    body_string(response).await
}

#[tokio::test]
async fn a_deleted_authors_id_renders_as_a_missing_author() {
    let fixture = deleted_authors("removed_account_pages").await;
    for (uri, session) in [
        ("/projects".to_owned(), &fixture.depositor_session),
        (format!("/projects/{PROJECT_A}/sections/overview"), &fixture.depositor_session),
        ("/review".to_owned(), &fixture.reviewer_session),
        (format!("/review/{PROJECT_B}"), &fixture.reviewer_session),
    ] {
        let body = page(&fixture, &uri, session).await;
        let main = main_of(&body);
        for id in &fixture.deleted {
            assert!(!main.contains(id.as_str()), "{uri} shows the deleted account's id {id}");
        }
        // The one page that names a draft's author says why there is no name.
        if uri == "/review" {
            assert!(main.contains("Account removed"), "{main}");
        }
    }
}
