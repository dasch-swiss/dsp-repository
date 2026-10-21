//! The public payload for approved records served at `GET /api/v1/approved-records`.
//!
//! This is a public, allow-listed view: every member is chosen deliberately, and
//! a field added to [`ApprovedRecord`] must never reach it implicitly. The two
//! fields that make [`ApprovedRecord`] unfit to serialize directly are
//! `approved_by` (an internal user id) and `payload` (opaque serialized JSON,
//! only ever valid to interpret through [`ProjectDraft`]) — neither appears
//! here, and the key-set test below pins the allow-list so a new member on the
//! stored record cannot cross the boundary unnoticed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared_metadata::project::ProjectRaw;
use uuid::Uuid;

use crate::draft::ProjectDraft;
use crate::proposals::{EntityProposal, ProposalStatus};
use crate::records::ApprovedRecord;

/// The branch prefix the pull-request run pushes a project's record to (`<prefix><shortcode>`), and
/// the head the editor's GitHub link searches for.
pub const BRANCH_PREFIX: &str = "editor/";

/// The response body for the approved-records endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedRecordsResponse {
    pub records: Vec<ApprovedRecordView>,
}

/// The public view of one [`ApprovedRecord`].
///
/// Every member is always present, `null` when there is nothing to report:
/// there is no `skip_serializing_if` here, so a consumer reads `null` rather
/// than having to distinguish an absent key from an empty one, and the
/// key-set test stays a stable pin on the wire shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedRecordView {
    pub id: Uuid,
    pub shortcode: String,
    pub approved_at: DateTime<Utc>,
    pub project: Option<ProjectRaw>,
    pub entities: Vec<ProposedEntityView>,
    /// Why [`Self::project`] is `None`, or `None` when it converted cleanly.
    /// About the project conversion only — an entity whose own payload will
    /// not parse is represented in [`Self::entities`], not reported here.
    pub problem: Option<String>,
}

/// One accepted proposal, joined onto the record it rides with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedEntityView {
    pub kind: String,
    pub operation: String,
    pub id: String,
    pub body: Value,
}

impl ApprovedRecordView {
    /// Builds the public view of one approved record and its accepted
    /// proposals.
    ///
    /// `proposals` is the caller's job to fetch and scope: this never queries
    /// on its own. The two sides key differently — [`EntityProposal::shortcode`]
    /// is stored normalized, [`ApprovedRecord::shortcode`] is not — so a caller
    /// joins them on [`crate::records::normalize_shortcode`]'s form.
    ///
    /// Never fails and never drops the record: if `record.payload` does not
    /// parse as a [`ProjectDraft`], or the draft is not a publishable project,
    /// [`Self::project`] is `None` and [`Self::problem`] carries the error, but
    /// the record still appears in the response. Dropping it would hide a row
    /// silently; failing the whole response would let one bad record block
    /// every other project's pull request.
    #[must_use]
    pub fn from_record(record: &ApprovedRecord, proposals: &[EntityProposal]) -> Self {
        let (project, problem) = match parse_project(&record.payload) {
            Ok(raw) => (Some(raw), None),
            Err(message) => (None, Some(message)),
        };

        // Excludes a retired proposal: its entity is already published, so a new approved record
        // for this project must not re-serve it with `operation: "new"` and create a duplicate.
        let entities = proposals
            .iter()
            .filter(|proposal| proposal.status == ProposalStatus::Accepted && proposal.retired_at.is_none())
            .map(ProposedEntityView::from_proposal)
            .collect();

        Self {
            id: record.id,
            shortcode: record.shortcode.clone(),
            approved_at: record.approved_at,
            project,
            entities,
            problem,
        }
    }
}

/// Parses a stored payload as a [`ProjectDraft`] and converts it, folding both
/// failure points into one message for [`ApprovedRecordView::problem`].
fn parse_project(payload: &str) -> Result<ProjectRaw, String> {
    let draft: ProjectDraft = serde_json::from_str(payload).map_err(|err| err.to_string())?;
    draft.to_raw().map_err(|err| err.to_string())
}

impl ProposedEntityView {
    /// Builds one entity's view from an accepted proposal.
    ///
    /// Fills `id` into the body from `entity_id`, overwriting whatever the
    /// payload holds — per [`EntityProposal::payload`]'s contract, a producer
    /// leaves `id` out, so a reader must supply it to serve a complete entity.
    /// A payload that does not parse as JSON still yields an entity, with
    /// `body: Value::Null`, rather than panicking or dropping it.
    fn from_proposal(proposal: &EntityProposal) -> Self {
        let mut body: Value = serde_json::from_str(&proposal.payload).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut body {
            map.insert("id".to_string(), Value::String(proposal.entity_id.clone()));
        }
        Self {
            kind: proposal.kind.as_str().to_string(),
            operation: proposal.operation.as_str().to_string(),
            id: proposal.entity_id.clone(),
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use shared_metadata::person::Person;
    use uuid::Uuid;

    use super::*;
    use crate::proposals::{ProposalDecision, ProposalKind, ProposalOperation};
    use crate::test_support::sample_raw;

    fn fully_populated_record() -> ApprovedRecord {
        let draft = ProjectDraft::from_raw(&sample_raw());
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: "0803".to_string(),
            payload: serde_json::to_string(&draft).expect("a draft serializes"),
            approved_by: Some(Uuid::new_v4()),
            approved_at: Utc::now(),
        }
    }

    fn accepted_proposal(kind: ProposalKind, entity_id: &str, payload: Value) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: "0803".to_string(),
            entity_id: entity_id.to_string(),
            kind,
            operation: ProposalOperation::New,
            payload: payload.to_string(),
            status: ProposalStatus::Accepted,
            proposed_by: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            decision: Some(ProposalDecision::Accept),
            decided_by: Some(Uuid::new_v4()),
            decided_at: Some(Utc::now()),
            retired_at: None,
        }
    }

    fn object_keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Pins the allow-list. A member added to or removed from `ApprovedRecordView` must show up
    /// here as a deliberate change to this list, not as silent drift from a field added to the
    /// stored record.
    #[test]
    fn serializes_only_the_allow_listed_keys() {
        let record = fully_populated_record();
        let proposal = accepted_proposal(
            ProposalKind::Person,
            "person-417",
            json!({"givenNames": ["Ada"], "familyNames": ["Lovelace"], "jobTitles": [], "email": "ada@example.com"}),
        );
        let view = ApprovedRecordView::from_record(&record, &[proposal]);
        let value = serde_json::to_value(&view).expect("serializes");

        assert_eq!(
            object_keys(&value),
            vec![
                "approvedAt".to_string(),
                "entities".to_string(),
                "id".to_string(),
                "problem".to_string(),
                "project".to_string(),
                "shortcode".to_string(),
            ],
            "the public ApprovedRecordView key set changed; if that is intended, update this allow-list \
             deliberately rather than letting a new member ride along silently"
        );
        assert!(
            value.get("approvedBy").is_none(),
            "approved_by must never reach the public payload"
        );
        assert!(
            value.get("payload").is_none(),
            "the opaque draft payload must never reach the public payload"
        );
    }

    /// Contributor addresses are already committed to this public repository
    /// and rendered as `mailto:` links on the public project page. An entity
    /// served without its address would make a later write-back delete a
    /// committed address, so the round-trip through the view must not lose it.
    #[test]
    fn an_accepted_persons_email_survives_into_its_entity_body() {
        let record = fully_populated_record();
        let person = Person {
            id: "person-417".to_string(),
            given_names: vec!["Ada".to_string()],
            family_names: vec!["Lovelace".to_string()],
            job_titles: vec![],
            affiliations: vec![],
            same_as: vec![],
            email: Some("ada@example.com".to_string()),
        };
        let payload = serde_json::to_value(&person).expect("a person serializes");
        let proposal = accepted_proposal(ProposalKind::Person, "person-417", payload);

        let view = ApprovedRecordView::from_record(&record, &[proposal]);

        assert_eq!(
            view.entities[0].body.get("email").and_then(Value::as_str),
            Some("ada@example.com"),
        );
    }

    #[test]
    fn a_proposal_not_accepted_produces_no_entity() {
        let record = fully_populated_record();
        let mut submitted = accepted_proposal(ProposalKind::Person, "person-1", json!({}));
        submitted.status = ProposalStatus::Submitted;

        let view = ApprovedRecordView::from_record(&record, &[submitted]);

        assert!(view.entities.is_empty());
    }

    #[test]
    fn a_retired_proposal_produces_no_entity_but_a_live_accepted_one_still_does() {
        let record = fully_populated_record();
        let live = accepted_proposal(ProposalKind::Person, "person-1", json!({}));
        let mut retired = accepted_proposal(ProposalKind::Organization, "organization-1", json!({}));
        retired.retired_at = Some(Utc::now());

        let view = ApprovedRecordView::from_record(&record, &[live, retired]);

        let ids: Vec<&str> = view.entities.iter().map(|entity| entity.id.as_str()).collect();
        assert_eq!(ids, vec!["person-1"]);
    }

    #[test]
    fn an_entity_with_unparseable_payload_gets_a_null_body_and_no_record_problem() {
        let record = fully_populated_record();
        let mut proposal = accepted_proposal(ProposalKind::Person, "person-1", json!({}));
        proposal.payload = "not json".to_string();

        let view = ApprovedRecordView::from_record(&record, &[proposal]);

        assert_eq!(view.entities.len(), 1);
        assert_eq!(view.entities[0].body, Value::Null);
        assert!(view.problem.is_none());
    }

    #[test]
    fn a_record_whose_payload_converts_carries_no_problem_and_the_project() {
        let record = fully_populated_record();

        let view = ApprovedRecordView::from_record(&record, &[]);

        assert!(view.project.is_some());
        assert!(view.problem.is_none());
    }

    /// A record whose payload cannot become a project file is still served —
    /// dropping it would hide the row, so `project` is `None` and `problem`
    /// carries the error instead.
    #[test]
    fn a_record_whose_payload_will_not_convert_is_still_served_with_a_problem() {
        let mut broken = fully_populated_record();
        broken.payload = "{}".to_string();

        let view = ApprovedRecordView::from_record(&broken, &[]);

        assert!(view.project.is_none());
        assert!(view.problem.is_some());
    }
}
