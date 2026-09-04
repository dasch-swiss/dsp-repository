//! The field-by-field comparison RDU reviews a submission through (REQ-4.3),
//! and the per-field decisions it records.
//!
//! Two halves that stay apart on purpose. [`diff`] is a pure comparison of two
//! drafts and knows nothing about a form or a reviewer; [`ReviewState`] is what
//! a reviewer decided, stored beside the submission and read back by the
//! surface that renders it. Neither knows a field's label — that is the
//! registry's, in `editor-web`, and the dependency runs the other way.
//!
//! ## The comparison is over top-level members, not registry fields
//!
//! A draft is the project's JSON members ([`ProjectDraft`]), including members
//! no applier touches (REQ-1.7) and members added to the contract since this
//! build (REQ-1.8). Enumerating the registry instead would show a reviewer only
//! the fields the *form* knows, so a change arriving through any other path
//! would be approved without ever being displayed. The union of both sides is
//! therefore the row set, and the registry supplies wording for the ids it
//! recognises.
//!
//! No registry id in the form is nested (`accessRights.embargoDate` is the only
//! dotted one and has no shape), so top-level granularity loses no editable
//! field today. A nested change shows as its parent member changing.
//!
//! ## What "changed" means
//!
//! Equality of the stored `serde_json::Value`s. That is stricter than the
//! comparison [`crate::form`] applies to a submitted value — which forgives
//! surrounding whitespace and a CRLF — and it has to be: those rules exist so
//! that *saving* an untouched form writes no bytes, and by the time a
//! submission exists they have already been applied. A difference that survives
//! them is a real difference in what would be committed, and hiding it from a
//! reviewer would approve bytes nobody saw.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::draft::ProjectDraft;

/// What a reviewer decided about one field (REQ-4.3).
///
/// Absent — no entry in [`ReviewState`] — is a third state and the initial one:
/// the field is changed and nobody has looked at it yet. It is not a variant
/// here because "undecided" is the absence of a decision, and giving it a
/// stored form would let a field be explicitly undecided, which is the same
/// thing written two ways.
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
    /// value an "undecided" control posts — a hand-built body naming a decision
    /// this build does not know must leave the field undecided rather than pick
    /// one.
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
/// The substitute is kept *here* rather than written into the submission's
/// payload, which would be the shorter path and would destroy evidence: the
/// depositor's own value is what a later screen has to show them beside what
/// RDU substituted (REQ-4.4 waives the second approver, so nobody else sees the
/// change), and an overwritten payload cannot answer what was submitted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldReview {
    /// `None` while the field is still undecided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    /// The reviewer's replacement for the submitted value, or `None` where they
    /// left it alone.
    ///
    /// `Some(Value::Null)` is a reviewer *clearing* a field the contract types
    /// as an `Option` — the applier removed the member, and that is a real
    /// substitution. It has to be representable: with absence meaning
    /// "unchanged", `substitute.or(submitted)` could not express a cleared
    /// field at all, and the surface would keep offering the submitted value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl FieldReview {
    /// Whether this entry says anything at all. An empty one is dropped rather
    /// than stored, so a reload cannot tell "decided nothing" from "never
    /// touched" — because they are the same state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.decision.is_none() && self.value.is_none()
    }
}

/// Every field decision on one submission.
///
/// Serializes as a plain object keyed by field id, which is what the
/// `submissions.review_state` column holds. A `BTreeMap` so the stored JSON is
/// stable across writes — the column is compared in tests and read by a human
/// debugging a review.
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
    /// A payload this build cannot read is an empty state and an error in the
    /// log, never a refusal: the reviewer can still see the diff and decide
    /// again, where a 500 would strand the submission until someone edited the
    /// database.
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
/// `published` is `None` for a project that exists only locally (REQ-2.3),
/// which is not the same as a published project holding none of these members:
/// there is no published side to revert *to*, and the surface says so rather
/// than offering a revert that would silently unset a required field. The
/// comparison itself is identical either way — an absent project answers `None`
/// for every member — so only the framing differs.
///
/// Order is the submission's own member order (which is `ProjectRaw`'s
/// declaration order, the same order the canonical writer emits), with any
/// member only the published project has appended. A member the submission
/// dropped is therefore still a row, at the end: a removal is a change a
/// reviewer has to see, and one sorted out of the list is one nobody rejects.
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
        // REQ-2.3's local-only project: there is no published side at all, so
        // every member the submission holds is new. The rows are the same shape
        // as any other diff — only the framing around them differs.
        let submitted = draft(json!({ "name": "N", "description": { "en": "D" } }));
        let rows = diff(None, &submitted);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.published.is_none() && row.changed()));
    }

    #[test]
    fn test_a_difference_only_in_whitespace_is_still_a_difference() {
        // The form's own comparison forgives surrounding whitespace so an
        // untouched save writes no bytes. By the time a submission exists those
        // rules have already run, so anything left is a real difference in what
        // would be committed — and a reviewer must not approve bytes nobody saw.
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
        // The reviewer can still see the diff and decide again; a refusal would
        // strand the submission until someone edited the database.
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
