//! `GET /api/v1/approved-records` — the public read of every approved record
//! and its accepted entity proposals.
//!
//! Unauthenticated: a CI poller reads this, not a signed-in user, so the
//! handler takes no `Authenticated`/`Rdu` extractor. Enumerates with
//! [`ApprovedRecordRepository::list_all`], not `list_uncollected` — there is no
//! state-dependent selection here for a stale advisory flag to hide, and the
//! served set is already bounded by startup reconciliation, which deletes a
//! record once the published set matches it.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use editor_core::collection::{ApprovedRecordView, ApprovedRecordsResponse};
use editor_core::records::normalize_shortcode;
use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository};

use crate::AppState;

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

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use chrono::Utc;
    use editor_core::draft::ProjectDraft;
    use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
    use editor_core::records::{ApprovedRecord, PullRequestState};
    use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository};
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::test_support::{published_corpus, test_app, test_state};

    /// A project the committed published set really holds, so the payload
    /// converts to a project file without a fixture project of our own.
    const PUBLISHED_SHORTCODE: &str = "0801d";

    fn valid_payload() -> String {
        let published = published_corpus();
        let raw = published.get(PUBLISHED_SHORTCODE).expect("the fixture project is published");
        serde_json::to_string(&ProjectDraft::from_raw(raw)).expect("a draft serializes")
    }

    fn approved_record(shortcode: &str, payload: &str) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: payload.to_string(),
            approved_by: None,
            approved_at: Utc::now(),
            collected_at: None,
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
}
