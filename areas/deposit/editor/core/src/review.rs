//! The field-by-field comparison RDU reviews a submission through, and the
//! per-field decisions it records.
//!
//! [`diff`] is a pure comparison of two drafts; [`ReviewState`] is what a
//! reviewer decided, stored beside the submission. Neither knows a field's
//! label. [`decisions`] reads a posted review body into a [`ReviewState`],
//! [`decided_draft`] applies it, and [`approval`] runs the approve gates.
//!
//! The comparison is over top-level members, not registry fields: enumerating
//! the registry would show a reviewer only the fields the form knows, so a
//! change arriving through any other path would be approved without being
//! displayed. The union of both sides is the row set. No registry id is nested
//! (`accessRights.embargoDate` is the only dotted one), so top-level
//! granularity loses no editable field; a nested change shows as its parent
//! member changing.
//!
//! "Changed" is equality of the stored `serde_json::Value`s. That is stricter
//! than the whitespace and CRLF forgiveness [`crate::form`] applies, and has to
//! be: those rules have already run by the time a submission exists, so a
//! difference that survives them is a real difference in what would be
//! committed, and hiding it would approve bytes nobody saw.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agents::{AgentScope, Agents};
use crate::draft::{DraftError, ProjectDraft};
use crate::form::registry::{self, Audience, Field};
use crate::form::submit::unresolved_agents;
use crate::form::{apply, FormBody};
use crate::proposals::{EntityProposal, ProposalDecision};

/// The name every decision control on the diff form posts under, before the
/// field id. Stated once, because the renderer and [`decisions`] both spell it.
pub const DECISION_PREFIX: &str = "decision";

/// What a reviewer decided about one field.
///
/// Absent from [`ReviewState`] is the initial, undecided state. Not a variant:
/// "undecided" is the absence of a decision, and a stored form would be the same
/// thing written twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Take the submitted value, or the reviewer's substitute for it.
    Accept,
    /// Keep the published value. The submitted change is not applied.
    Revert,
}

impl Decision {
    /// The stored form, and the value a decision control posts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Revert => "revert",
        }
    }

    /// Parse a posted decision. `None` for anything else, including the empty
    /// value an undecided control posts, so an unknown posted decision leaves the
    /// field undecided rather than picking one.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accept" => Some(Self::Accept),
            "revert" => Some(Self::Revert),
            _ => None,
        }
    }
}

/// One field's review: the decision, and the value a reviewer put in place of
/// the submitted one.
///
/// The substitute is kept here rather than written into the payload: the
/// depositor's own value has to be shown beside RDU's later, and with no second
/// approver nobody else would see the change.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldReview {
    /// `None` while the field is still undecided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    /// The reviewer's replacement for the submitted value, or `None` where they
    /// left it alone.
    ///
    /// `Some(Value::Null)` is a reviewer clearing an `Option` field, a real
    /// substitution. With absence meaning "unchanged", `substitute.or(submitted)`
    /// could not express it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl FieldReview {
    /// Whether this entry says anything. An empty one is dropped rather than
    /// stored, since "decided nothing" and "never touched" are the same state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.decision.is_none() && self.value.is_none()
    }
}

/// Every field decision on one submission.
///
/// Serializes as an object keyed by field id, which is what the
/// `submissions.review_state` column holds. A `BTreeMap` so the stored JSON is
/// stable across writes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReviewState {
    fields: BTreeMap<String, FieldReview>,
}

impl ReviewState {
    /// An empty state — no field decided, nothing substituted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a stored `review_state` column.
    ///
    /// A payload this build cannot read is an empty state and an error for the
    /// log, never a refusal: the reviewer can decide again, where a 500 would
    /// strand the submission.
    #[must_use]
    pub fn parse(stored: Option<&str>) -> (Self, Option<serde_json::Error>) {
        match stored {
            None => (Self::new(), None),
            Some(raw) => match serde_json::from_str(raw) {
                Ok(state) => (state, None),
                Err(error) => (Self::new(), Some(error)),
            },
        }
    }

    /// One field's review, or `None` where it has none.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&FieldReview> {
        self.fields.get(field)
    }

    /// One field's decision, `None` while it is undecided.
    #[must_use]
    pub fn decision(&self, field: &str) -> Option<Decision> {
        self.fields.get(field).and_then(|review| review.decision)
    }

    /// The reviewer's substitute for a field, `None` where they left the
    /// submitted value alone.
    #[must_use]
    pub fn substitute(&self, field: &str) -> Option<&Value> {
        self.fields.get(field).and_then(|review| review.value.as_ref())
    }

    /// Record one field's review. An entry that says nothing is removed rather
    /// than stored empty.
    pub fn set(&mut self, field: &str, review: FieldReview) {
        if review.is_empty() {
            self.fields.remove(field);
        } else {
            self.fields.insert(field.to_string(), review);
        }
    }

    /// How many of `fields` carry `decision`.
    #[must_use]
    pub fn count(&self, fields: &[String], decision: Decision) -> usize {
        fields.iter().filter(|field| self.decision(field) == Some(decision)).count()
    }

    /// Whether anything has been decided or substituted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Every field marked [`Decision::Accept`], in stored order.
    ///
    /// What locks a field on a returned draft: request-changes keeps the
    /// per-field state, and an accepted field the depositor then altered must
    /// not re-enter review flagged accepted. Reverted fields are excluded: the
    /// depositor has to redo them.
    #[must_use]
    pub fn accepted_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(_, review)| review.decision == Some(Decision::Accept))
            .map(|(field, _)| field.as_str())
            .collect()
    }

    /// Every field RDU put its own value in place of, with that value.
    ///
    /// What the depositor is shown before an approval; with no second approver
    /// a substituted value is otherwise seen by nobody. Reverted fields are
    /// excluded, and a revert stores no substitute anyway.
    #[must_use]
    pub fn substitutions(&self) -> Vec<(&str, &Value)> {
        self.fields
            .iter()
            .filter(|(_, review)| review.decision != Some(Decision::Revert))
            .filter_map(|(field, review)| review.value.as_ref().map(|value| (field.as_str(), value)))
            .collect()
    }
}

/// One field, as the review surface shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDiff {
    /// The `ProjectRaw` member name. Also the key a decision and a substitute
    /// are stored under, and the name the in-place control posts under.
    pub field: String,
    /// What the published project holds, `None` for a member it does not have —
    /// and for *every* member when the project is not published at all, which
    /// the caller distinguishes by passing `None` for the whole published side.
    pub published: Option<Value>,
    /// What the submission holds, `None` for a member it dropped.
    pub submitted: Option<Value>,
}

impl FieldDiff {
    /// Whether the submission changes this field.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.published != self.submitted
    }
}

/// Compare a submission against the published project, member by member.
///
/// `published` is `None` for a project that exists only locally, which is not
/// a published project holding none of these members: there is no published
/// side to revert to, and the surface says so rather than offering a revert.
///
/// Order is the submission's own member order (`ProjectRaw`'s declaration
/// order, which the canonical writer emits), with members only the published
/// project has appended, so a member the submission dropped is still a row.
#[must_use]
pub fn diff(published: Option<&ProjectDraft>, submitted: &ProjectDraft) -> Vec<FieldDiff> {
    let mut rows: Vec<FieldDiff> = submitted
        .fields()
        .map(|field| FieldDiff {
            field: field.to_string(),
            published: published.and_then(|draft| draft.get(field)).cloned(),
            submitted: submitted.get(field).cloned(),
        })
        .collect();
    if let Some(published) = published {
        rows.extend(
            published
                .fields()
                .filter(|field| submitted.get(field).is_none())
                .map(|field| FieldDiff {
                    field: field.to_string(),
                    published: published.get(field).cloned(),
                    submitted: None,
                }),
        );
    }
    rows
}

/// The submitted draft with every field decision applied: what an approval commits, and what
/// [`approval`]'s gates check. Checking the submitted draft instead would check a document that is
/// not what gets committed: a revert can remove a reference and a substitution can add one.
///
/// An accepted field takes the reviewer's substitute where there is one, and a reverted field goes
/// back to the published value; `submissions.payload` stays the depositor's own.
#[must_use]
pub fn decided_draft(submitted: &ProjectDraft, rows: &[FieldDiff], state: &ReviewState) -> ProjectDraft {
    let mut decided = submitted.clone();
    for row in rows.iter().filter(|row| row.changed()) {
        match state.decision(&row.field) {
            Some(Decision::Accept) => {
                if let Some(substitute) = state.substitute(&row.field) {
                    // `Null` is a reviewer clearing an optional field, which is
                    // a removal and not a stored null: the canonical writer
                    // strips nulls, so writing one would differ from what the
                    // depositor's own clear produces.
                    if substitute.is_null() {
                        decided.remove(&row.field);
                    } else {
                        decided.set(&row.field, substitute.clone());
                    }
                }
            }
            // Back to what is published. Absent there means the submission
            // added the member, so reverting removes it again.
            Some(Decision::Revert) => match row.published.as_ref() {
                Some(published) => decided.set(&row.field, published.clone()),
                None => {
                    decided.remove(&row.field);
                }
            },
            // Unreachable: an approval with an undecided change is refused by
            // [`approval`]. Left as the submitted value rather than guessed at, so if
            // the guard ever regresses the record is at least what was sent.
            None => {}
        }
    }
    decided
}

/// Read every decision and substitution the body carries.
///
/// Only rows the submission actually changes are considered: an unchanged field
/// has no decision control rendered, so a decision naming one came from a
/// hand-built body, and honouring it would store a decision the surface can
/// never show.
#[must_use]
pub fn decisions(
    body: &FormBody,
    rows: &[FieldDiff],
    submitted: &ProjectDraft,
    accept_all: bool,
    published: bool,
) -> ReviewState {
    let mut state = ReviewState::new();
    for row in rows.iter().filter(|row| row.changed()) {
        let posted = body.get(&format!("{DECISION_PREFIX}.{}", row.field));
        let decision = match posted.and_then(Decision::parse) {
            // A revert on a project with no published counterpart is a decision
            // the surface never offers, because there is nothing to revert *to*.
            // Stored anyway it renders "Reverted — keeps published" beside a
            // "Not published yet" column, in a radio group with no matching
            // option — so it shows as undecided and cannot be cleared.
            Some(Decision::Revert) if !published => None,
            Some(decision) => Some(decision),
            None if accept_all => Some(Decision::Accept),
            None => None,
        };
        // A reverted row renders read-only, so it posts no value control and
        // there is nothing to read — and reading one anyway would let a
        // hand-built body attach a substitute to a decision that discards it.
        let value = if decision == Some(Decision::Revert) {
            None
        } else {
            substitute(body, submitted, &row.field)
        };
        state.set(&row.field, FieldReview { decision, value });
    }
    state
}

/// What the reviewer put in place of the submitted value, or `None` where they
/// left it alone.
///
/// Runs the field's own applier over a clone of the submitted draft, so the reviewer's edit obeys
/// the rules a depositor's does (whitespace, newline encoding, a stored `MISSING` surviving an
/// empty submit). `Some(Value::Null)` is a reviewer clearing an optional field.
fn substitute(body: &FormBody, submitted: &ProjectDraft, field: &str) -> Option<Value> {
    let shape = registry::field(field).and_then(|field| field.shape)?;
    let mut edited = submitted.clone();
    apply(shape, body, &mut edited, field);
    let before = submitted.get(field);
    let after = edited.get(field);
    if before == after {
        return None;
    }
    Some(after.cloned().unwrap_or(Value::Null))
}

/// The first approval gate a decided review failed, and what it found there.
#[derive(Debug)]
pub enum ApproveRefusal {
    /// Changed fields nobody decided.
    Undecided { count: usize },
    /// Proposed entities nobody decided.
    UndecidedProposal { count: usize },
    /// Fields referring to an entity this approval rejects or that does not exist, with the
    /// reference each holds.
    RejectedEntity(Vec<(&'static Field, String)>),
    /// The decided draft is not a [`ProjectRaw`](shared_metadata::project::ProjectRaw).
    Unpublishable(DraftError),
}

/// Run the approval gates in order and return the first refusal. Call only for an approving
/// outcome: request-changes and reject commit nothing, and an undecided row is an ordinary state
/// to return or discard a submission in.
///
/// 1. every changed field is decided ([`ApproveRefusal::Undecided`]): approving an undecided change
///    commits bytes nobody looked at;
/// 2. every proposed entity is decided ([`ApproveRefusal::UndecidedProposal`]):
///    `editor_store::review_rounds` relies on this to leave an undecided proposal's own `approve()`
///    mapping `submitted` rather than guessed at;
/// 3. the decided draft refers to no rejected or unknown entity
///    ([`ApproveRefusal::RejectedEntity`]): nothing is stripped silently, RDU substitutes the
///    referencing fields or requests changes instead;
/// 4. the decided draft converts to a project file ([`ApproveRefusal::Unpublishable`]): a
///    permissive draft can pass every check above and still fail to become one.
///
/// `proposals` are the project's [`Submitted`](crate::proposals::ProposalStatus::Submitted) ones
/// with the decisions applied in memory, and `agents` the published store. The reference scope is
/// built from the proposals not being rejected, never by status: every one is still `Submitted`
/// until `ReviewRoundRepository::approve` runs (`docs/src/editor/entity-proposals.md`, "The
/// second gate has a trap").
///
/// # Errors
///
/// The first failing gate's [`ApproveRefusal`].
pub fn approval(
    submitted: &ProjectDraft,
    rows: &[FieldDiff],
    state: &ReviewState,
    proposals: &[EntityProposal],
    agents: &Agents,
) -> Result<(), ApproveRefusal> {
    let changed: Vec<String> = rows.iter().filter(|row| row.changed()).map(|row| row.field.clone()).collect();
    let undecided = changed.len() - state.count(&changed, Decision::Accept) - state.count(&changed, Decision::Revert);
    if undecided > 0 {
        return Err(ApproveRefusal::Undecided { count: undecided });
    }

    let undecided_proposals = proposals.iter().filter(|proposal| proposal.decision.is_none()).count();
    if undecided_proposals > 0 {
        return Err(ApproveRefusal::UndecidedProposal { count: undecided_proposals });
    }

    let surviving: Vec<EntityProposal> = proposals
        .iter()
        .filter(|proposal| proposal.decision != Some(ProposalDecision::Reject))
        .cloned()
        .collect();
    let scope = AgentScope::with_proposals(agents, &surviving);
    let decided = decided_draft(submitted, rows, state);
    let dangling = unresolved_agents(Audience::RduOnly, &decided, &scope);
    if !dangling.is_empty() {
        return Err(ApproveRefusal::RejectedEntity(dangling));
    }

    decided.to_raw().map_err(ApproveRefusal::Unpublishable)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn draft(value: serde_json::Value) -> ProjectDraft {
        serde_json::from_value(value).expect("an object deserializes as a draft")
    }

    #[test]
    fn test_diff_marks_only_the_members_that_differ() {
        let published = draft(json!({ "name": "Old", "shortcode": "0801" }));
        let submitted = draft(json!({ "name": "New", "shortcode": "0801" }));

        let rows = diff(Some(&published), &submitted);
        let changed: Vec<&str> = rows.iter().filter(|row| row.changed()).map(|row| row.field.as_str()).collect();
        assert_eq!(changed, ["name"]);
    }

    #[test]
    fn test_diff_keeps_the_submissions_member_order() {
        // The order a reviewer reads is the order the file is written in, so
        // the diff reads like the record rather than like a hash map.
        let submitted = draft(json!({ "name": "N", "shortDescription": "S", "startDate": "2020-01-01" }));
        let rows = diff(None, &submitted);
        let fields: Vec<&str> = rows.iter().map(|row| row.field.as_str()).collect();
        assert_eq!(fields, ["name", "shortDescription", "startDate"]);
    }

    #[test]
    fn test_a_member_the_submission_dropped_is_still_a_row() {
        // A removal is a change a reviewer has to see. Left out of the row set
        // it would be applied without ever being displayed.
        let published = draft(json!({ "name": "N", "provenance": "P" }));
        let submitted = draft(json!({ "name": "N" }));

        let rows = diff(Some(&published), &submitted);
        let dropped = rows
            .iter()
            .find(|row| row.field == "provenance")
            .expect("a row for the dropped member");
        assert!(dropped.changed());
        assert_eq!(dropped.published, Some(json!("P")));
        assert_eq!(dropped.submitted, None);
    }

    #[test]
    fn test_a_member_the_submission_added_is_a_change_against_nothing() {
        let published = draft(json!({ "name": "N" }));
        let submitted = draft(json!({ "name": "N", "provenance": "P" }));

        let rows = diff(Some(&published), &submitted);
        let added = rows
            .iter()
            .find(|row| row.field == "provenance")
            .expect("a row for the added member");
        assert!(added.changed());
        assert_eq!(added.published, None);
    }

    #[test]
    fn test_an_unpublished_project_makes_every_member_a_change() {
        // A local-only project: there is no published side at all, so
        // every member the submission holds is new. The rows are the same shape
        // as any other diff — only the framing around them differs.
        let submitted = draft(json!({ "name": "N", "description": { "en": "D" } }));
        let rows = diff(None, &submitted);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.published.is_none() && row.changed()));
    }

    #[test]
    fn test_a_difference_only_in_whitespace_is_still_a_difference() {
        let published = draft(json!({ "name": "Bernoulli" }));
        let submitted = draft(json!({ "name": "Bernoulli " }));
        assert!(diff(Some(&published), &submitted)[0].changed());
    }

    #[test]
    fn test_review_state_round_trips_through_its_stored_form() {
        let mut state = ReviewState::new();
        state.set(
            "name",
            FieldReview {
                decision: Some(Decision::Accept),
                value: Some(json!("Edited")),
            },
        );
        state.set("provenance", FieldReview { decision: Some(Decision::Revert), value: None });

        let stored = serde_json::to_string(&state).expect("a review state serializes");
        let (read, error) = ReviewState::parse(Some(&stored));
        assert!(error.is_none());
        assert_eq!(read, state);
        assert_eq!(read.decision("name"), Some(Decision::Accept));
        assert_eq!(read.substitute("name"), Some(&json!("Edited")));
        assert_eq!(read.substitute("provenance"), None);
    }

    #[test]
    fn test_an_entry_that_says_nothing_is_not_stored() {
        // "Decided nothing" and "never touched" are the same state, so storing
        // an empty entry would make a reload able to tell them apart when
        // nothing else can.
        let mut state = ReviewState::new();
        state.set("name", FieldReview { decision: Some(Decision::Accept), value: None });
        state.set("name", FieldReview::default());
        assert!(state.is_empty());
        assert_eq!(serde_json::to_string(&state).expect("serializes"), "{}");
    }

    #[test]
    fn test_an_unreadable_review_state_is_empty_rather_than_an_error() {
        let (state, error) = ReviewState::parse(Some("not json"));
        assert!(state.is_empty());
        assert!(error.is_some());
    }

    #[test]
    fn test_an_unknown_stored_decision_does_not_become_a_known_one() {
        // Silently reading an unknown decision as `Accept` would approve a
        // change on the strength of a value this build does not understand.
        assert_eq!(Decision::parse("approve"), None);
        assert_eq!(Decision::parse(""), None);
        assert_eq!(Decision::parse("accept"), Some(Decision::Accept));
        assert_eq!(Decision::parse("revert"), Some(Decision::Revert));
        for decision in [Decision::Accept, Decision::Revert] {
            assert_eq!(Decision::parse(decision.as_str()), Some(decision));
        }
    }

    #[test]
    fn test_accepted_fields_are_the_ones_a_returned_draft_locks() {
        let mut state = ReviewState::new();
        state.set("name", FieldReview { decision: Some(Decision::Accept), value: None });
        state.set("abstract", FieldReview { decision: Some(Decision::Revert), value: None });
        state.set("provenance", FieldReview { decision: None, value: Some(json!("a substitute")) });
        state.set(
            "shortDescription",
            FieldReview {
                decision: Some(Decision::Accept),
                value: Some(json!("Edited by RDU")),
            },
        );

        assert_eq!(state.accepted_fields(), ["name", "shortDescription"]);
    }

    #[test]
    fn test_substitutions_are_what_the_depositor_is_shown() {
        let mut state = ReviewState::new();
        state.set(
            "name",
            FieldReview {
                decision: Some(Decision::Accept),
                value: Some(json!("Edited by RDU")),
            },
        );
        // Accepted as submitted: nothing was put in its place, so there is
        // nothing to show.
        state.set("abstract", FieldReview { decision: Some(Decision::Accept), value: None });

        assert_eq!(state.substitutions(), [("name", &json!("Edited by RDU"))]);
    }

    #[test]
    fn test_a_cleared_field_is_a_substitution_the_depositor_sees() {
        // `Some(Value::Null)` is a reviewer clearing a field, a real substitution;
        // filtering it out would hide exactly the case where a value disappeared.
        let mut state = ReviewState::new();
        state.set(
            "endDate",
            FieldReview { decision: Some(Decision::Accept), value: Some(Value::Null) },
        );
        assert_eq!(state.substitutions(), [("endDate", &Value::Null)]);
    }

    #[test]
    fn test_a_reverted_field_reports_neither_a_lock_nor_a_substitution() {
        // A revert discards the submitted value and stores no substitute, so a
        // stored one came from a hand-built body. Reporting it would show the
        // depositor a value RDU explicitly chose not to take.
        let mut state = ReviewState::new();
        state.set(
            "name",
            FieldReview {
                decision: Some(Decision::Revert),
                value: Some(json!("never taken")),
            },
        );
        assert!(state.accepted_fields().is_empty());
        assert!(state.substitutions().is_empty());
    }

    #[test]
    fn test_count_reports_decisions_over_the_fields_asked_about() {
        let mut state = ReviewState::new();
        state.set("name", FieldReview { decision: Some(Decision::Accept), value: None });
        state.set("abstract", FieldReview { decision: Some(Decision::Revert), value: None });
        // A decision on a field outside the set asked about is not counted:
        // the surface counts the rows it shows.
        state.set("provenance", FieldReview { decision: Some(Decision::Accept), value: None });

        let fields = vec!["name".to_string(), "abstract".to_string()];
        assert_eq!(state.count(&fields, Decision::Accept), 1);
        assert_eq!(state.count(&fields, Decision::Revert), 1);
    }
}
