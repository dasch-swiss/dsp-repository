//! Two audiences share this module, per `modules/editor/CLAUDE.md`'s "machine-facing routes live
//! under /api/v1/" rule: [`list`] and [`report`] below are `/api/v1` — JSON or a bare status,
//! never HTML — and [`overview`], [`discard_form`] and [`discard`] are the RDU browser surface,
//! `GET /collection` and `GET`/`POST /collection/{id}/discard`.
//!
//! `GET /api/v1/approved-records` — the public read of every approved record
//! and its accepted entity proposals — and `POST /api/v1/collection-report`,
//! the token-authenticated write the collecting workflow uses to report back.
//!
//! `list` is unauthenticated: a CI poller reads this, not a signed-in user, so
//! the handler takes no `Authenticated`/`Rdu` extractor. Enumerates with
//! [`ApprovedRecordRepository::list_all`], not `list_uncollected` — there is no
//! state-dependent selection here for a stale advisory flag to hide, and the
//! served set is already bounded by startup reconciliation, which deletes a
//! record once the published set matches it.
//!
//! `report` is not unauthenticated, but it takes no `Authenticated`/`Rdu`
//! extractor either: its caller is a CI job with no session to hold, so it
//! authenticates by bearer token instead. It writes **advisory display state
//! only** — [`editor_core::collection::CollectionStateView`] — and must never
//! change what `list` serves. A record leaves that set only by being deleted:
//! by the startup reconcile once the published set carries it, or by approval
//! superseding an earlier record for the same project.
//!
//! [`overview`] classifies every record live, per request, with
//! [`editor_core::status::classify_record`] — the same call `reconcile`'s startup pass makes —
//! rather than reading `reconcile::Reconciliation`, which is aggregate counters computed once at
//! startup with no per-record identity.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use chrono::Utc;
use editor_core::collection::{ApprovedRecordView, ApprovedRecordsResponse, CollectionReport};
use editor_core::records::{normalize_shortcode, ApprovedRecord, User};
use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository, RepositoryError};
use editor_core::status::{classify_record, RecordClassification};
use editor_web::pages::collection as page;
use maud::html;
use uuid::Uuid;

use crate::auth::guard::Rdu;
use crate::auth::secret::code_matches;
use crate::AppState;

/// Every pull request this endpoint accepts must live under this prefix, so a forged report
/// cannot point RDU's advisory display at another site.
const PULL_REQUEST_PREFIX: &str = "https://github.com/dasch-swiss/dsp-repository/pull/";

/// `GET /api/v1/approved-records`.
#[tracing::instrument(skip_all, fields(otel.kind = "internal", otel.name = "approved records list"))]
pub(crate) async fn list(State(state): State<AppState>) -> Response {
    let records = match ApprovedRecordRepository::list_all(&*state.db).await {
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
        let proposals = match EntityProposalRepository::list_for_shortcode(&*state.db, &shortcode).await {
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

/// `POST /api/v1/collection-report`.
///
/// Bodies are plain text, not the HTML page shell — the caller is a CI job, matching how
/// `crate::csrf` answers the same beacon-shaped caller.
#[tracing::instrument(skip_all, fields(otel.kind = "internal", otel.name = "collection report"))]
pub(crate) async fn report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CollectionReport>,
) -> Response {
    // The comparison always runs, on a presented value that is the empty string when the header
    // is absent, so an absent token and a wrong one take the same branch and answer identically —
    // there is no early return for "absent" ahead of it. A `None` configured token still refuses
    // every call: a service with no verifier must not accept an empty presented one, which is why
    // this reads the option rather than defaulting it to an empty `Secret`.
    let presented = bearer_token(&headers).unwrap_or_default();
    let authorized = state
        .collection_token
        .as_ref()
        .is_some_and(|token| code_matches(presented, token.expose()));
    if !authorized {
        tracing::warn!("refused a collection report: missing or invalid bearer token");
        return (
            StatusCode::UNAUTHORIZED,
            "This request was refused: the bearer token is missing or invalid.\n",
        )
            .into_response();
    }

    if let Err(message) = validate(&body) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }

    // Only the first dispatch stamps `collected_at`. `mark_collected`'s `NotFound` on a second
    // call is its contract, not an error: it means either an unknown record or one already
    // collected, and `report_collection` below tells the two apart, so nothing is decided here.
    if body.pull_request.is_some() {
        match ApprovedRecordRepository::mark_collected(&*state.db, body.record, Utc::now()).await {
            Ok(()) | Err(RepositoryError::NotFound { .. }) => {}
            Err(error) => {
                tracing::error!(error = %error, "could not mark a record collected");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    let updated = match ApprovedRecordRepository::report_collection(
        &*state.db,
        body.record,
        body.pull_request.as_deref(),
        body.state,
        body.failure.as_deref(),
        Utc::now(),
    )
    .await
    {
        Ok(updated) => updated,
        Err(error) => {
            tracing::error!(error = %error, "could not record a collection report");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if !updated {
        return (
            StatusCode::NOT_FOUND,
            "No such approved record, or it has already been discarded.\n",
        )
            .into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

/// The token from `Authorization: Bearer <token>`, or `None` for any other shape — a missing
/// header, a different scheme, or a value that is not valid UTF-8.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

/// Rejects a report that is malformed or ambiguous, before any write. Every branch here must
/// leave the caller applying nothing: a report failing later would otherwise commit half of a
/// report the whole submission was supposed to stand or fall on together.
fn validate(report: &CollectionReport) -> Result<(), &'static str> {
    match (&report.pull_request, &report.failure) {
        (Some(_), Some(_)) => Err("a report may not carry both a pull request and a failure"),
        (None, None) => Err("a report must carry either a pull request or a failure"),
        (Some(url), None) => {
            if report.state.is_none() {
                return Err("a pull request report must also carry a state");
            }
            if !url.starts_with(PULL_REQUEST_PREFIX) {
                return Err("pull_request must be a dsp-repository pull request URL");
            }
            Ok(())
        }
        (None, Some(_)) => Ok(()),
    }
}

const NO_SUCH_RECORD: &str =
    "There is no approved record with that id. It may already have been discarded by another RDU member.";

/// One row's owned strings, so the view can borrow them — a row carries its formatted timestamps
/// and its live classification, and neither can be produced inside the `map` that builds it.
struct RowStrings {
    id: String,
    shortcode: String,
    project_name: Option<String>,
    approved_at: String,
    reported_at: Option<String>,
    pull_request: Option<String>,
    classification: RecordClassification,
}

/// `GET /collection` — every approved record and where its collection stands, for RDU.
pub(crate) async fn overview(State(state): State<AppState>, Rdu(user): Rdu) -> Response {
    let records = match ApprovedRecordRepository::list_all(&*state.db).await {
        Ok(records) => records,
        Err(error) => return storage_error(&state, &user, "read the approved records", &error),
    };

    let rows: Vec<RowStrings> = records
        .iter()
        .map(|record| RowStrings {
            id: record.id.to_string(),
            shortcode: crate::shortcode_as_published(&state, &record.shortcode),
            project_name: crate::project_name(&state, &record.shortcode).map(str::to_string),
            approved_at: crate::format_instant(record.approved_at),
            reported_at: record.reported_at.map(crate::format_instant),
            pull_request: record.pull_request_url.clone(),
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
            reported_at: row.reported_at.as_deref(),
            pull_request: row.pull_request.as_deref(),
            state: &row.classification,
        })
        .collect();

    crate::render(
        &state,
        "Collection — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user),
        page::list(&view_rows),
    )
}

/// The record `id` names, read from the same full enumeration [`overview`] reads.
///
/// There is no narrower read on the port for this: [`ApprovedRecordRepository`] carries
/// `find_by_shortcode`, not `find_by_id`. `Ok(None)` covers both an id that is not a UUID at all
/// and one that names no row, so a hand-typed path segment gets the same 404 as a discarded one.
async fn find_record(state: &AppState, id: &str) -> Result<Option<ApprovedRecord>, RepositoryError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Ok(None);
    };
    let records = ApprovedRecordRepository::list_all(&*state.db).await?;
    Ok(records.into_iter().find(|record| record.id == id))
}

/// `GET /collection/{id}/discard` — the confirmation, naming what would be destroyed.
pub(crate) async fn discard_form(State(state): State<AppState>, Rdu(user): Rdu, Path(id): Path<String>) -> Response {
    let record = match find_record(&state, &id).await {
        Ok(Some(record)) => record,
        Ok(None) => return no_such_record(&state, &user),
        Err(error) => return storage_error(&state, &user, "read the approved records", &error),
    };
    let shortcode = crate::shortcode_as_published(&state, &record.shortcode);
    let approved_at = crate::format_instant(record.approved_at);
    // Classified here too, so a reader who reached this URL directly sees the
    // same state the list would have shown rather than a page that reads alike
    // whatever it is about to destroy.
    let classification = classify_record(&record, state.published.get(&record.shortcode));
    let impact = page::DiscardImpact {
        shortcode: &shortcode,
        project_name: crate::project_name(&state, &record.shortcode),
        approved_at: &approved_at,
        pull_request: record.pull_request_url.as_deref(),
        state: &classification,
    };
    crate::render(
        &state,
        "Discard record — DaSCH Metadata Editor",
        StatusCode::OK,
        Some(&user),
        page::confirm_discard(&id, &impact),
    )
}

/// `POST /collection/{id}/discard` — delete the record.
///
/// Not gated on the record's classification: the confirmation link only appears on a
/// [`RecordClassification::Stranded`] row as guidance, but this acts on RDU's explicit
/// confirmation of whatever id was posted, the same way [`discard_form`] renders whatever id was
/// asked for. `delete` answering `false` (already gone) is not an error — a second submission of
/// the same confirmation must not 500.
pub(crate) async fn discard(State(state): State<AppState>, Rdu(user): Rdu, Path(id): Path<String>) -> Response {
    let Ok(record_id) = Uuid::parse_str(&id) else {
        return no_such_record(&state, &user);
    };
    match ApprovedRecordRepository::delete(&*state.db, record_id).await {
        Ok(_) => Redirect::to("/collection").into_response(),
        Err(error) => {
            tracing::error!(error = %error, "could not discard an approved record");
            storage_error(&state, &user, "discard the record", &error)
        }
    }
}

/// No record with this id.
fn no_such_record(state: &AppState, user: &User) -> Response {
    let content = html! {
        h1 class="font-display text-2xl mb-2" { "Nothing to discard" }
        p class="mb-4" { (NO_SUCH_RECORD) }
        p {
            a href="/collection" class="underline" { "Back to collection" }
        }
    };
    crate::render(
        state,
        "Nothing to discard — DaSCH Metadata Editor",
        StatusCode::NOT_FOUND,
        Some(user),
        content,
    )
}

/// Storage would not answer, so the page cannot show what it should.
fn storage_error(state: &AppState, user: &User, what: &str, error: &RepositoryError) -> Response {
    tracing::error!(error = %error, operation = what, "the collection surface could not reach storage");
    crate::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(user),
        editor_web::pages::problem::unavailable(
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
    use editor_core::records::{
        ApprovedRecord, PullRequestState, ReviewOutcome, ReviewRound, Role, Submission, SubmissionState, User,
    };
    use editor_core::repository::{
        ApprovedRecordRepository, EntityProposalRepository, ReviewRoundRepository, SubmissionRepository, Transition,
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::auth::cookie;
    use crate::test_support::{
        a_session, a_user, body_string, get, location, post, published_corpus, state_with_collection_token, test_app,
        test_state, with_cookie,
    };
    use crate::AppState;

    /// The bearer token these tests configure the endpoint with.
    const TOKEN: &str = "a-collection-token";

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
        with_cookie(request, cookie::SESSION, session)
    }

    /// An RDU account with a live session.
    async fn an_rdu_session(state: &AppState) -> (User, String) {
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
            collected_at: None,
            reported_at: None,
            pull_request_url: None,
            pull_request_state: None,
            last_failure: None,
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
    async fn a_collected_record_with_a_live_pull_request_is_still_served() {
        // The invariant the design rests on: reported collection state drives what RDU is
        // shown and never what is served. `list_all` is what holds it — an enumeration
        // filtering on `collected_at` or `pull_request_state` would hide a row instead.
        let (state, _) = test_state("collection-serves-collected").await;
        let mut record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        record.collected_at = Some(Utc::now());
        record.pull_request_url = Some("https://github.com/dasch-swiss/dsp-repository/pull/1".to_string());
        record.pull_request_state = Some(PullRequestState::Open);
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a collected record");

        let body = body_bytes(test_app(&state).oneshot(get_request()).await.unwrap()).await;
        let value: Value = serde_json::from_slice(&body).expect("a JSON body");
        let records = value.get("records").and_then(Value::as_array).expect("a records array");
        assert_eq!(records.len(), 1, "a collected record is still served: {value}");
        assert_eq!(
            records[0].get("id").and_then(Value::as_str),
            Some(record.id.to_string().as_str())
        );
        assert_eq!(
            records[0].pointer("/collection/state").and_then(Value::as_str),
            Some("open"),
            "the advisory state rides along without filtering anything"
        );
    }

    #[tokio::test]
    async fn unauthenticated_access_serves_the_expected_records() {
        // No session cookie anywhere in this request: the contract is that this
        // route is public, not an oversight to fix later.
        let (state, _) = test_state("collection-public").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body: Value = serde_json::from_slice(&body_bytes(response).await).expect("valid JSON");
        let records = body["records"].as_array().expect("a records array");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["id"], record.id.to_string());
    }

    #[tokio::test]
    async fn two_reads_with_no_intervening_write_are_byte_identical() {
        // The cheapest available proof that this endpoint is a pure read: the
        // payload carries no generation timestamp precisely so this holds.
        let (state, _) = test_state("collection-idempotent").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
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
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let shortcode = editor_core::records::normalize_shortcode(&record.shortcode);
        let proposal = accepted_proposal(&shortcode, "person-1", json!({}));
        EntityProposalRepository::create_change(&*state.db, &proposal)
            .await
            .expect("seed a proposal");

        let before_records = ApprovedRecordRepository::list_all(&*state.db).await.expect("list before");
        let before_proposals = EntityProposalRepository::list_for_shortcode(&*state.db, &shortcode)
            .await
            .expect("list proposals before");

        let response = test_app(&state).oneshot(get_request()).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let after_records = ApprovedRecordRepository::list_all(&*state.db).await.expect("list after");
        let after_proposals = EntityProposalRepository::list_for_shortcode(&*state.db, &shortcode)
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
        ApprovedRecordRepository::create(&*state.db, &good)
            .await
            .expect("seed the good record");
        ApprovedRecordRepository::create(&*state.db, &broken)
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
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let proposal = accepted_proposal(
            "0801d",
            "person-417",
            json!({"givenNames": ["Ada"], "familyNames": ["Lovelace"], "jobTitles": [], "email": null}),
        );
        EntityProposalRepository::create_change(&*state.db, &proposal)
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

    /// Deliberately carries no `sec-fetch-site` header, matching a GitHub Actions runner: every
    /// test below reaching a response other than 403 is itself evidence the CSRF exemption
    /// (`crate::csrf::COLLECTION_REPORT_PATH`) is in effect for this path.
    fn report_request(token: Option<&str>, body: &Value) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/v1/collection-report")
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder.body(Body::from(body.to_string())).unwrap()
    }

    async fn snapshot(state: &crate::AppState) -> Vec<ApprovedRecord> {
        ApprovedRecordRepository::list_all(&*state.db)
            .await
            .expect("list should succeed")
    }

    /// A pending submission on `shortcode`, so `ReviewRoundRepository::approve` has something to
    /// claim.
    async fn a_pending_submission(state: &crate::AppState, shortcode: &str) -> Submission {
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: valid_payload(),
            state: SubmissionState::InReview,
            submitted_by: None,
            submitted_at: Utc::now(),
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        };
        SubmissionRepository::create(&*state.db, &submission)
            .await
            .expect("seed a submission");
        submission
    }

    #[tokio::test]
    async fn a_valid_first_report_stamps_collected_at_and_stores_the_pull_request() {
        let (state, _) = state_with_collection_token("collection-report-first", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/42",
            "state": "open",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);

        let stored = &snapshot(&state).await[0];
        assert!(stored.collected_at.is_some(), "the first dispatch must stamp it");
        assert_eq!(
            stored.pull_request_url.as_deref(),
            Some("https://github.com/dasch-swiss/dsp-repository/pull/42")
        );
        assert_eq!(stored.pull_request_state, Some(PullRequestState::Open));
    }

    #[tokio::test]
    async fn a_repeated_report_refreshes_state_without_moving_collected_at() {
        let (state, _) = state_with_collection_token("collection-report-repeat", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");

        let opened = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/42",
            "state": "open",
            "failure": null,
        });
        test_app(&state).oneshot(report_request(Some(TOKEN), &opened)).await.unwrap();
        let after_first = snapshot(&state).await[0].collected_at;

        let merged = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/42",
            "state": "merged",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &merged)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);

        let stored = &snapshot(&state).await[0];
        assert_eq!(stored.pull_request_state, Some(PullRequestState::Merged));
        assert_eq!(stored.collected_at, after_first, "a repeated report must not move it");
    }

    #[tokio::test]
    async fn a_report_naming_an_unknown_record_is_rejected_whole() {
        let (state, _) = state_with_collection_token("collection-report-unknown", TOKEN).await;
        let untouched = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &untouched)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({
            "record": Uuid::new_v4(),
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/1",
            "state": "open",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        assert_eq!(snapshot(&state).await, before, "nothing must be written for an unknown record");
    }

    #[tokio::test]
    async fn a_report_carrying_both_a_pull_request_and_a_failure_is_rejected() {
        let (state, _) = state_with_collection_token("collection-report-both", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/1",
            "state": "open",
            "failure": "boom",
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&state).await, before);
    }

    #[tokio::test]
    async fn a_report_carrying_neither_a_pull_request_nor_a_failure_is_rejected() {
        let (state, _) = state_with_collection_token("collection-report-neither", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({ "record": record.id, "pullRequest": null, "state": null, "failure": null });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&state).await, before);
    }

    #[tokio::test]
    async fn a_pull_request_without_a_state_is_rejected() {
        let (state, _) = state_with_collection_token("collection-report-no-state", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/1",
            "state": null,
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&state).await, before);
    }

    #[tokio::test]
    async fn a_pull_request_on_another_host_is_rejected() {
        let (state, _) = state_with_collection_token("collection-report-other-host", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({
            "record": record.id,
            "pullRequest": "https://evil.test/dasch-swiss/dsp-repository/pull/1",
            "state": "open",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&state).await, before);
    }

    #[tokio::test]
    async fn a_pull_request_on_another_github_repository_is_rejected() {
        let (state, _) = state_with_collection_token("collection-report-other-repo", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let before = snapshot(&state).await;

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dasch-specs/pull/1",
            "state": "open",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(snapshot(&state).await, before);
    }

    #[tokio::test]
    async fn an_absent_token_and_a_wrong_token_produce_the_same_refusal() {
        let (state, _) = state_with_collection_token("collection-report-token", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/1",
            "state": "open",
            "failure": null,
        });

        let absent = test_app(&state).oneshot(report_request(None, &body)).await.unwrap();
        let absent_status = absent.status();
        let absent_body = body_bytes(absent).await;

        let wrong = test_app(&state)
            .oneshot(report_request(Some("not-the-token"), &body))
            .await
            .unwrap();
        let wrong_status = wrong.status();
        let wrong_body = body_bytes(wrong).await;

        assert_eq!(absent_status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(absent_status, wrong_status);
        assert_eq!(absent_body, wrong_body);
        assert_eq!(
            snapshot(&state).await,
            vec![record],
            "an unauthorized report must write nothing"
        );
    }

    #[tokio::test]
    async fn a_report_is_refused_when_no_token_is_configured() {
        // `test_state` configures no collection token, matching production's fail-closed
        // default: a service with no verifier must not accept an empty presented one.
        let (state, _) = test_state("collection-report-no-token-configured").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/1",
            "state": "open",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some("anything"), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_record_reported_closed_keeps_collected_at_and_stops_blocking_a_new_approval() {
        let (state, _) = state_with_collection_token("collection-report-closed", TOKEN).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");

        let body = json!({
            "record": record.id,
            "pullRequest": "https://github.com/dasch-swiss/dsp-repository/pull/9",
            "state": "closed",
            "failure": null,
        });
        let response = test_app(&state).oneshot(report_request(Some(TOKEN), &body)).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);

        let after_report = ApprovedRecordRepository::find_by_shortcode(&*state.db, PUBLISHED_SHORTCODE)
            .await
            .expect("read the record back");
        assert_eq!(after_report.len(), 1);
        assert!(after_report[0].collected_at.is_some(), "the first dispatch stamped it");

        // A closed pull request no longer stands between the record and the published corpus,
        // so a fresh approval must supersede it rather than be refused.
        let submission = a_pending_submission(&state, PUBLISHED_SHORTCODE).await;
        let new_record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        let round = ReviewRound {
            id: Uuid::new_v4(),
            shortcode: PUBLISHED_SHORTCODE.to_string(),
            submission_id: submission.id,
            outcome: ReviewOutcome::Approved,
            note: None,
            review_state: None,
            actor: None,
            at: Utc::now(),
        };
        let transition = ReviewRoundRepository::approve(&*state.db, submission.id, &new_record, &round)
            .await
            .expect("approval should succeed");
        assert_eq!(transition, Transition::Applied);

        let after_approve = ApprovedRecordRepository::find_by_shortcode(&*state.db, PUBLISHED_SHORTCODE)
            .await
            .expect("read the record back");
        assert_eq!(after_approve.len(), 1);
        assert_eq!(after_approve[0].id, new_record.id, "the closed record must be superseded");
    }

    // ---- The RDU browser surface --------------------------------------------

    #[tokio::test]
    async fn a_depositor_session_is_refused_every_collection_route() {
        let (state, _) = test_state("collection-depositor-refused").await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
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
    async fn a_stranded_record_renders_as_stranded_and_the_same_record_open_renders_as_waiting() {
        // The live-classification requirement: the page has to read the record fresh on every
        // request, not a startup summary with no per-record identity — so this fails if the
        // handler is ever changed to read `reconcile::Reconciliation` instead.
        let (state, _) = test_state("collection-live-classification").await;
        let (_, session) = an_rdu_session(&state).await;
        let mut record = approved_record(PUBLISHED_SHORTCODE, &differing_payload());
        record.pull_request_state = Some(PullRequestState::Merged);
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let app = test_app(&state);

        let stranded = body_string(app.clone().oneshot(as_session(get("/collection"), &session)).await.unwrap()).await;
        assert!(stranded.contains("Merged, still differs"), "{stranded}");

        ApprovedRecordRepository::report_collection(
            &*state.db,
            record.id,
            Some("https://github.com/dasch-swiss/dsp-repository/pull/1"),
            Some(PullRequestState::Open),
            None,
            Utc::now(),
        )
        .await
        .expect("the report should be recorded");

        let waiting = body_string(app.oneshot(as_session(get("/collection"), &session)).await.unwrap()).await;
        assert!(waiting.contains("Pull request open"), "{waiting}");
        assert!(!waiting.contains("Merged, still differs"), "{waiting}");
    }

    #[tokio::test]
    async fn discarding_deletes_the_record_and_a_repeated_discard_does_not_500() {
        let (state, _) = test_state("collection-discard").await;
        let (_, session) = an_rdu_session(&state).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &valid_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let app = test_app(&state);
        let uri = format!("/collection/{}/discard", record.id);

        let first = app.clone().oneshot(as_session(post(&uri, ""), &session)).await.unwrap();
        assert_eq!(first.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(location(&first).as_deref(), Some("/collection"));
        assert!(ApprovedRecordRepository::find_by_shortcode(&*state.db, PUBLISHED_SHORTCODE)
            .await
            .unwrap()
            .is_empty());

        let second = app.oneshot(as_session(post(&uri, ""), &session)).await.unwrap();
        assert_ne!(second.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn a_record_never_reported_on_says_so_rather_than_rendering_blank() {
        let (state, _) = test_state("collection-never-reported").await;
        let (_, session) = an_rdu_session(&state).await;
        let record = approved_record(PUBLISHED_SHORTCODE, &differing_payload());
        ApprovedRecordRepository::create(&*state.db, &record)
            .await
            .expect("seed a record");
        let app = test_app(&state);

        let body = body_string(app.oneshot(as_session(get("/collection"), &session)).await.unwrap()).await;
        assert!(body.contains("Never reported on"), "{body}");
    }
}
