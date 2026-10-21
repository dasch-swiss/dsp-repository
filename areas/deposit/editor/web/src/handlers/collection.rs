//! Two audiences share this module, per `areas/deposit/editor/CLAUDE.md`'s "machine-facing routes
//! live under /api/v1/" rule: [`list`] is `/api/v1`, JSON never HTML, and [`overview`],
//! [`discard_form`] and [`discard`] are the RDU browser surface.
//!
//! `list` is unauthenticated: a CI poller reads it, so the handler takes no `Authenticated`/`Rdu`
//! extractor. It applies no filter: see "Collection holds two invariants" in that file.
//!
//! [`overview`] classifies every record live, per request, with
//! [`editor_core::status::classify_record`], the same call `reconcile`'s startup pass makes,
//! because `reconcile::Reconciliation` holds aggregate counters with no per-record identity.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use editor_core::collection::{ApprovedRecordView, ApprovedRecordsResponse, BRANCH_PREFIX};
use editor_core::records::{normalize_shortcode, ApprovedRecord};
use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository, RepositoryError};
use editor_core::status::{classify_record, RecordClassification};
use maud::html;
use uuid::Uuid;

use crate::handlers::auth::{Account, Rdu};
use crate::pages::collection as page;
use crate::shell::EditorState;

/// Coupled to the collect workflow's `COLLECTOR_REPOSITORY`, which nothing here checks: change
/// either and the link built from this finds nothing.
const COLLECTOR_REPOSITORY: &str = "dasch-swiss/dsp-repository";

/// `GET /api/v1/approved-records`.
#[tracing::instrument(skip_all, fields(otel.kind = "internal", otel.name = "approved records list"))]
pub(crate) async fn list(State(state): State<EditorState>) -> Response {
    let records = match ApprovedRecordRepository::list_all(&*state.editor).await {
        Ok(records) => records,
        Err(error) => {
            tracing::error!(error = %error, "could not list approved records");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut views = Vec::with_capacity(records.len());
    for record in &records {
        // The two tables key differently: `entity_proposals.shortcode` is stored normalized,
        // `approved_records.shortcode` is not, so the join runs on the normalized form. Done here
        // so the call is correct against the trait rather than against the one implementation that
        // happens to normalize its own input.
        let shortcode = normalize_shortcode(&record.shortcode);
        let proposals = match EntityProposalRepository::list_for_shortcode(&*state.editor, &shortcode).await {
            Ok(proposals) => proposals,
            Err(error) => {
                tracing::error!(error = %error, project.shortcode = %shortcode, "could not list entity proposals");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        views.push(ApprovedRecordView::from_record(record, &proposals));
    }

    // Built up before any response is sent: a mid-loop repository error above
    // returns 500 with no body, never a 200 carrying fewer records than exist.
    Json(ApprovedRecordsResponse { records: views }).into_response()
}

const NO_SUCH_RECORD: &str =
    "There is no approved record with that id. It may already have been discarded by another RDU member.";

/// One row's owned strings, so the view can borrow them.
struct RowStrings {
    id: String,
    shortcode: String,
    project_name: Option<String>,
    approved_at: String,
    pull_requests_url: String,
    classification: RecordClassification,
}

/// `GET /collection`: every approved record and how it compares with the published set, for RDU.
pub(crate) async fn overview(State(state): State<EditorState>, Rdu(user): Rdu) -> Response {
    let records = match ApprovedRecordRepository::list_all(&*state.editor).await {
        Ok(records) => records,
        Err(error) => return storage_error(&state, &user, "read the approved records", &error),
    };

    let rows: Vec<RowStrings> = records
        .iter()
        .map(|record| RowStrings {
            id: record.id.to_string(),
            shortcode: crate::shell::shortcode_as_published(&state, &record.shortcode),
            project_name: crate::shell::project_name(&state, &record.shortcode).map(str::to_string),
            approved_at: crate::shell::format_instant(record.approved_at),
            pull_requests_url: pull_requests_url(&record.shortcode),
            classification: classify_record(record, state.published.get(&record.shortcode)),
        })
        .collect();
    let view_rows: Vec<page::CollectionRow<'_>> = rows
        .iter()
        .map(|row| page::CollectionRow {
            id: &row.id,
            shortcode: &row.shortcode,
            project_name: row.project_name.as_deref(),
            approved_at: &row.approved_at,
            pull_requests_url: &row.pull_requests_url,
            state: &row.classification,
        })
        .collect();

    crate::shell::render(
        &state,
        "Collection — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user.actor.display_name),
        page::list(&view_rows),
    )
}

/// GitHub's pull-request search for the branch the collector pushes `shortcode`'s record to.
///
/// Takes the stored shortcode, never the published spelling: the collector's branch name uses the
/// stored one. It needs no percent-encoding, since a stored shortcode passes `is_valid_shortcode`.
pub(crate) fn pull_requests_url(shortcode: &str) -> String {
    let head = BRANCH_PREFIX.replace('/', "%2F");
    format!("https://github.com/{COLLECTOR_REPOSITORY}/pulls?q=is%3Apr+head%3A{head}{shortcode}")
}

/// The record `id` names, read from the same full enumeration [`overview`] reads.
///
/// There is no narrower read on the port for this: [`ApprovedRecordRepository`] carries
/// `find_by_shortcode`, not `find_by_id`. `Ok(None)` covers both an id that is not a UUID at all
/// and one that names no row, so a hand-typed path segment gets the same 404 as a discarded one.
async fn find_record(state: &EditorState, id: &str) -> Result<Option<ApprovedRecord>, RepositoryError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Ok(None);
    };
    let records = ApprovedRecordRepository::list_all(&*state.editor).await?;
    Ok(records.into_iter().find(|record| record.id == id))
}

/// `GET /collection/{id}/discard`: the confirmation, naming what would be destroyed.
pub(crate) async fn discard_form(State(state): State<EditorState>, Rdu(user): Rdu, Path(id): Path<String>) -> Response {
    let record = match find_record(&state, &id).await {
        Ok(Some(record)) => record,
        Ok(None) => return no_such_record(&state, &user),
        Err(error) => return storage_error(&state, &user, "read the approved records", &error),
    };
    let shortcode = crate::shell::shortcode_as_published(&state, &record.shortcode);
    let approved_at = crate::shell::format_instant(record.approved_at);
    let pull_requests_url = pull_requests_url(&record.shortcode);
    // Classified here too, so a reader who reached this URL directly sees the
    // same state the list would have shown rather than a page that reads alike
    // whatever it is about to destroy.
    let classification = classify_record(&record, state.published.get(&record.shortcode));
    let impact = page::DiscardImpact {
        shortcode: &shortcode,
        project_name: crate::shell::project_name(&state, &record.shortcode),
        approved_at: &approved_at,
        pull_requests_url: &pull_requests_url,
        state: &classification,
    };
    crate::shell::render(
        &state,
        "Discard record — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user.actor.display_name),
        page::confirm_discard(&id, &impact),
    )
}

/// `POST /collection/{id}/discard` — delete the record.
///
/// Not gated on the record's classification: every row offers Discard. The record's pull request is
/// not touched. `delete` answering `false` (already gone) is not an error: a second submission of
/// the same confirmation must not 500.
pub(crate) async fn discard(State(state): State<EditorState>, Rdu(user): Rdu, Path(id): Path<String>) -> Response {
    let Ok(record_id) = Uuid::parse_str(&id) else {
        return no_such_record(&state, &user);
    };
    match ApprovedRecordRepository::delete(&*state.editor, record_id).await {
        Ok(_) => Redirect::to("/collection").into_response(),
        Err(error) => {
            tracing::error!(error = %error, "could not discard an approved record");
            storage_error(&state, &user, "discard the record", &error)
        }
    }
}

/// No record with this id.
fn no_such_record(state: &EditorState, user: &Account) -> Response {
    let content = html! {
        h1 class="font-display text-2xl mb-2" { "Nothing to discard" }
        p class="mb-4" { (NO_SUCH_RECORD) }
        p {
            a href="/collection" class="underline" { "Back to collection" }
        }
    };
    crate::shell::render(
        state,
        "Nothing to discard — DaSCH Metadata Editor",
        StatusCode::NOT_FOUND,
        Some(&user.actor.display_name),
        content,
    )
}

/// Storage would not answer, so the page cannot show what it should.
fn storage_error(state: &EditorState, user: &Account, what: &str, error: &impl std::fmt::Display) -> Response {
    tracing::error!(error = %error, operation = what, "the collection surface could not reach storage");
    crate::shell::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(&user.actor.display_name),
        deposit_shell::pages::problem::unavailable(
            "The editor could not reach its database, so this page is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
        ),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use chrono::Utc;
    use editor_core::draft::ProjectDraft;
    use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
    use editor_core::records::ApprovedRecord;
    use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository};
    use editor_ports::Role;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::fixtures::{
        a_session, a_user, body_string, get, location, post, test_app, test_state, with_cookie, TestState, User,
    };
    use crate::test_support::published_corpus;

    /// A project the committed published set really holds, so the payload
    /// converts to a project file without a fixture project of our own.
    const PUBLISHED_SHORTCODE: &str = "0801d";

    fn valid_payload() -> String {
        let published = published_corpus();
        let raw = published.get(PUBLISHED_SHORTCODE).expect("the fixture project is published");
        serde_json::to_string(&ProjectDraft::from_raw(raw)).expect("a draft serializes")
    }

    /// A payload that differs from the published set by one field, so a record built from it
    /// classifies as something other than [`editor_core::status::RecordClassification::Published`].
    fn differing_payload() -> String {
        let published = published_corpus();
        let raw = published.get(PUBLISHED_SHORTCODE).expect("the fixture project is published");
        let mut draft = ProjectDraft::from_raw(raw);
        draft.set("name", json!("What RDU Approved Instead"));
        serde_json::to_string(&draft).expect("a draft serializes")
    }

    fn as_session(request: Request<Body>, session: &str) -> Request<Body> {
        with_cookie(request, crate::fixtures::SESSION, session)
    }

    /// An RDU account with a live session.
    async fn an_rdu_session(state: &TestState) -> (User, String) {
        let user = a_user(state, "rdu@example.test", "An RDU Member", Role::Rdu, &[]).await;
        let session = a_session(state, user.id).await;
        (user, session)
    }

    fn approved_record(shortcode: &str, payload: &str) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: payload.to_string(),
            approved_by: None,
            approved_at: Utc::now(),
        }
    }

    fn accepted_proposal(shortcode: &str, entity_id: &str, payload: Value) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            entity_id: entity_id.to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::New,
            payload: payload.to_string(),
            status: ProposalStatus::Accepted,
            proposed_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            decision: None,
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    fn get_request() -> Request<Body> {
        // The rate limiter's key extractor needs an `X-Forwarded-For` entry or a
        // `ConnectInfo` extension to produce a key at all; `oneshot` supplies
        // neither on its own, so every request here carries one, as the
        // telemetry beacon's tests do.
        Request::builder()
            .method("GET")
            .uri("/api/v1/approved-records")
            .header("x-forwarded-for", "203.0.113.9")
            .body(Body::empty())
            .unwrap()
    }

    async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
        to_bytes(response.into_body(), usize::MAX).await.expect("a body reads").to_vec()
    }

    #[tokio::test]
    async fn every_record_is_served_to_an_unauthenticated_caller_whatever_the_editor_holds_about_it() {
        // No session cookie anywhere in this request: the route is public by contract.
        let (state, _) = test_state("collection-serves-every-record").await;
        let waiting = approved_record(PUBLISHED_SHORTCODE, &differing_payload());
        let unreadable = approved_record("0999", "{ not json");
        for record in [&waiting, &unreadable] {
            ApprovedRecordRepository::create(&*state.editor, record)
                .await
                .expect("seed a record");
        }

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("a JSON body");
        let records = value.get("records").and_then(Value::as_array).expect("a records array");
        let mut ids: Vec<&str> = records.iter().map(|r| r["id"].as_str().unwrap()).collect();
        ids.sort_unstable();
        let mut expected = vec![waiting.id.to_string(), unreadable.id.to_string()];
        expected.sort_unstable();
        assert_eq!(ids, expected, "every record is served: {value}");
    }

    #[tokio::test]
    async fn two_reads_with_no_intervening_write_are_byte_identical() {
        // The cheapest available proof that this endpoint is a pure read: the
        // payload carries no generation timestamp precisely so this holds.
        let (state, _) = test_state("collection-idempotent").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");

        let first = body_bytes(test_app(&state).oneshot(get_request()).await.unwrap()).await;
        let second = body_bytes(test_app(&state).oneshot(get_request()).await.unwrap()).await;
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn the_handler_writes_nothing_to_storage() {
        // Pins the endpoint's read-only contract behaviourally: a snapshot of
        // everything the handler can reach, taken before and after the request,
        // must agree. This proves the handler leaves stored state untouched; it
        // does not prove anything about ordering or isolation.
        let (state, _) = test_state("collection-no-write").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");
        let shortcode = editor_core::records::normalize_shortcode(&record.shortcode);
        let proposal = accepted_proposal(&shortcode, "person-1", json!({}));
        EntityProposalRepository::create_change(&*state.editor, &proposal)
            .await
            .expect("seed a proposal");

        let before_records = ApprovedRecordRepository::list_all(&*state.editor).await.expect("list before");
        let before_proposals = EntityProposalRepository::list_for_shortcode(&*state.editor, &shortcode)
            .await
            .expect("list proposals before");

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let after_records = ApprovedRecordRepository::list_all(&*state.editor).await.expect("list after");
        let after_proposals = EntityProposalRepository::list_for_shortcode(&*state.editor, &shortcode)
            .await
            .expect("list proposals after");

        assert_eq!(before_records, after_records);
        assert_eq!(before_proposals, after_proposals);
    }

    #[tokio::test]
    async fn a_record_with_an_unconvertible_payload_is_still_served_alongside_a_good_one() {
        // Dropping the broken record would hide a row; the endpoint exists so
        // that never happens.
        let (state, _) = test_state("collection-broken-payload").await;
        let good = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        let broken = approved_record("0999", "{}");
        ApprovedRecordRepository::create(&*state.editor, &good)
            .await
            .expect("seed the good record");
        ApprovedRecordRepository::create(&*state.editor, &broken)
            .await
            .expect("seed the broken record");

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body: Value = serde_json::from_slice(&body_bytes(response).await).expect("valid JSON");
        let records = body["records"].as_array().expect("a records array");
        let ids: Vec<&str> = records.iter().map(|r| r["id"].as_str().unwrap()).collect();

        assert!(ids.contains(&good.id.to_string().as_str()));
        assert!(ids.contains(&broken.id.to_string().as_str()));
        let broken_view = records.iter().find(|r| r["id"] == broken.id.to_string()).unwrap();
        assert!(broken_view["problem"].is_string());
        let good_view = records.iter().find(|r| r["id"] == good.id.to_string()).unwrap();
        assert!(good_view["problem"].is_null());
    }

    #[tokio::test]
    async fn an_accepted_proposal_reaches_entities_through_the_route() {
        // Seeds the record's shortcode in a form that differs from the
        // normalized one `entity_proposals` stores, so this fails if the
        // handler stops normalizing before the join.
        let (state, _) = test_state("collection-entity-join").await;
        let record = approved_record("0801D", &valid_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");
        let proposal = accepted_proposal(
            "0801d",
            "person-417",
            json!({"givenNames": ["Ada"], "familyNames": ["Lovelace"], "jobTitles": [], "email": null}),
        );
        EntityProposalRepository::create_change(&*state.editor, &proposal)
            .await
            .expect("seed a proposal");

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        let body: Value = serde_json::from_slice(&body_bytes(response).await).expect("valid JSON");
        let records = body["records"].as_array().expect("a records array");
        let view = records
            .iter()
            .find(|r| r["id"] == record.id.to_string())
            .expect("the seeded record");
        let entities = view["entities"].as_array().expect("an entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["id"], "person-417");
    }

    // ---- The RDU browser surface --------------------------------------------

    #[tokio::test]
    async fn a_depositor_session_is_refused_every_collection_route() {
        let (state, _) = test_state("collection-depositor-refused").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");
        let depositor = a_user(&state, "depositor@example.test", "A Depositor", Role::Depositor, &[]).await;
        let session = a_session(&state, depositor.id).await;
        let app = test_app(&state);

        for request in [
            get("/collection"),
            get(&format!("/collection/{}/discard", record.id)),
            post(&format!("/collection/{}/discard", record.id), ""),
        ] {
            let response = app.clone().oneshot(as_session(request, &session)).await.unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn a_differing_record_renders_waiting_for_release_with_a_link_built_from_the_stored_shortcode() {
        // The link uses the stored, lowercase spelling, which the collector's branch name carries;
        // the published spelling (`0801D`) would search for a branch that does not exist.
        let (state, _) = test_state("collection-waiting-for-release").await;
        let (_, session) = an_rdu_session(&state).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &differing_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");

        let body = body_string(
            test_app(&state)
                .oneshot(as_session(get("/collection"), &session))
                .await
                .unwrap(),
        )
        .await;
        assert!(body.contains("Waiting for release"), "{body}");
        assert!(body.contains("pulls?q=is%3Apr+head%3Aeditor-collect%2F0801d\""), "{body}");
    }

    #[tokio::test]
    async fn discarding_deletes_the_record_and_a_repeated_discard_does_not_500() {
        let (state, _) = test_state("collection-discard").await;
        let (_, session) = an_rdu_session(&state).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.editor, &record)
            .await
            .expect("seed a record");
        let app = test_app(&state);
        let uri = format!("/collection/{}/discard", record.id);

        let first = app.clone().oneshot(as_session(post(&uri, ""), &session)).await.unwrap();
        assert_eq!(first.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(location(&first).as_deref(), Some("/collection"));
        assert!(ApprovedRecordRepository::find_by_shortcode(&*state.editor, PUBLISHED_SHORTCODE)
            .await
            .unwrap()
            .is_empty());

        let second = app.oneshot(as_session(post(&uri, ""), &session)).await.unwrap();
        assert_ne!(second.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
