//! The records the editor persists.
//!
//! Framework-free: the SQLite column mapping lives in `editor-store`, behind
//! the ports in [`crate::repository`]. A `payload: String` is a serialized
//! [`ProjectDraft`](crate::draft::ProjectDraft), which is `#[serde(transparent)]`
//! over the project's members; it stays opaque here because this layer never
//! interprets it.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stored value outside the known set. Reachable only if the database is
/// edited by hand or a migration adds a variant the code does not know.
#[derive(Debug, thiserror::Error)]
#[error("unknown {kind} {value:?}")]
pub struct UnknownVariant {
    pub kind: &'static str,
    pub value: String,
}

/// The canonical storage key for a project shortcode: trimmed, ASCII-folded.
///
/// One definition, because `drafts`, `submissions` and `approved_records` key on
/// it as an exact-match column while
/// [`PublishedProjects::get`](crate::published::PublishedProjects::get) and
/// `editor_ports::Principal::may_reach` fold. Keying a write on the
/// shortcode as typed would give `/projects/080c` and `/projects/080C` a row
/// each, and the unique `submissions.shortcode` check would miss a pending
/// submission. ASCII rather than Unicode folding because `is_valid_shortcode`
/// admits only ASCII.
///
/// `the_assignment_comparison_agrees_with_the_storage_key`, in `editor-server`'s
/// `identity` tests, pins `Principal::may_reach`'s comparison against this key.
#[must_use]
pub fn normalize_shortcode(shortcode: &str) -> String {
    shortcode.trim().to_ascii_lowercase()
}

/// Work in progress on one project.
///
/// Keyed by shortcode: one draft per project, not per user — per-user multiple
/// drafts are out of scope, and concurrency is last-write-wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRecord {
    pub shortcode: String,
    /// A serialized [`ProjectDraft`](crate::draft::ProjectDraft); see the module docs.
    pub payload: String,
    /// The last editor's account id. Accounts live in identity's store, so the
    /// id is kept when its account is removed: the row survives, the depositor's
    /// work with it, and the review queue's "last editor" column reads an id
    /// that names no account as unknown.
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Where a submission sits in review.
///
/// `Draft` and `Online`, the two remaining lifecycle states, are deliberately absent: a draft is a
/// `drafts` row, and Online is derived at startup by comparing against the
/// published set, at which point the local record is discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubmissionState {
    Submitted,
    InReview,
    Approved,
}

impl SubmissionState {
    /// The stored form, pinned by a `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::InReview => "in_review",
            Self::Approved => "approved",
        }
    }
}

impl fmt::Display for SubmissionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SubmissionState {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "submitted" => Ok(Self::Submitted),
            "in_review" => Ok(Self::InReview),
            "approved" => Ok(Self::Approved),
            other => Err(UnknownVariant { kind: "submission state", value: other.to_string() }),
        }
    }
}

/// A submission awaiting or under review. One per project at a time: the
/// schema makes `shortcode` unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    pub id: Uuid,
    pub shortcode: String,
    /// A serialized [`ProjectDraft`](crate::draft::ProjectDraft); see the module docs.
    pub payload: String,
    pub state: SubmissionState,
    /// Kept when the submitter's account is removed; see [`DraftRecord::updated_by`].
    pub submitted_by: Option<Uuid>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// The reviewer's working note, kept across saves while the review runs.
    /// What the depositor reads is [`ReviewRound::note`]: this row is deleted by
    /// the transition that ends the round.
    pub reviewer_note: Option<String>,
    /// A serialized [`ReviewState`](crate::review::ReviewState), `None` while
    /// nothing has been decided. Opaque here for the same reason as `payload`.
    pub review_state: Option<String>,
}

/// Where a collected record's pull request sits, as last reported.
///
/// The wire form is the same lowercase set [`Self::as_str`] produces and the schema's `CHECK`
/// constraint pins. `test_pull_request_state_deserializes_from_its_stored_form` holds the two
/// together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    Open,
    Merged,
    Closed,
}

impl PullRequestState {
    /// The stored form, pinned by a `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }

    /// Whether a pull request in this state still stands between the record and
    /// the published corpus.
    ///
    /// The single definition of "live". A second approved record may not be
    /// created for a project while one of its records is live, and the supersede
    /// `DELETE` in `ReviewRoundRepository::approve` must match exactly the
    /// states this rejects. A variant added here without revisiting both is the
    /// way those two silently disagree.
    #[must_use]
    pub const fn is_live(self) -> bool {
        match self {
            Self::Open | Self::Merged => true,
            Self::Closed => false,
        }
    }
}

impl fmt::Display for PullRequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PullRequestState {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            other => Err(UnknownVariant { kind: "pull request state", value: other.to_string() }),
        }
    }
}

/// An approved record waiting to be collected into a pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedRecord {
    pub id: Uuid,
    pub shortcode: String,
    /// A serialized [`ProjectDraft`](crate::draft::ProjectDraft); see the module docs.
    pub payload: String,
    /// Kept when the approver's account is removed; see [`DraftRecord::updated_by`].
    pub approved_by: Option<Uuid>,
    pub approved_at: DateTime<Utc>,
    /// `None` while uncollected. A failed collection leaves it `None` so the
    /// next run retries it.
    pub collected_at: Option<DateTime<Utc>>,
    /// `None` until a collection run has reported on this record. Moved by
    /// every report, whatever it said, so this is the age of the state beside
    /// it, not the moment of first dispatch.
    pub reported_at: Option<DateTime<Utc>>,
    /// The pull request a collection run opened for this record, `None` until
    /// one has.
    pub pull_request_url: Option<String>,
    /// The pull request's state as last reported, `None` alongside
    /// `pull_request_url`.
    pub pull_request_state: Option<PullRequestState>,
    /// Why the last collection attempt failed, `None` when it last succeeded
    /// or none has run yet.
    pub last_failure: Option<String>,
}

/// How a review round ended.
///
/// Not a [`SubmissionState`]: every variant deletes the submission, and the
/// depositor-facing state list has no Rejected. [`Self::Withdrawn`] is the
/// depositor's own discard, here so their action leaves the same trail as a
/// reviewer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewOutcome {
    /// Accepted. An `approved_records` row now holds what was
    /// approved, including anything RDU substituted.
    Approved,
    /// Returned to the depositor as a draft, with a note.
    ChangesRequested,
    /// Discarded by RDU. Published metadata is unchanged.
    Rejected,
    /// Discarded by the depositor who made it.
    Withdrawn,
}

impl ReviewOutcome {
    /// The stored form, pinned by a `CHECK` constraint in the schema.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes_requested",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
        }
    }

    /// Whether this outcome hands the project back for more editing: everything
    /// but [`Self::Approved`], the only outcome that moves the record on, so
    /// editing after it starts the next cycle.
    #[must_use]
    pub const fn returns_the_project(self) -> bool {
        !matches!(self, Self::Approved)
    }
}

impl fmt::Display for ReviewOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ReviewOutcome {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "approved" => Ok(Self::Approved),
            "changes_requested" => Ok(Self::ChangesRequested),
            "rejected" => Ok(Self::Rejected),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(UnknownVariant { kind: "review outcome", value: other.to_string() }),
        }
    }
}

/// One finished review round: what was decided about a submission, by whom, and
/// what the depositor has to be told.
///
/// Append-only: written by the transition that ends the round and never
/// updated, so the rows for one project are its review history. The editor
/// architecture documentation says what reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRound {
    pub id: Uuid,
    pub shortcode: String,
    /// The submission this round ended. Kept although that row is gone: it is
    /// how two rounds recorded in the same second are told apart, and how a
    /// round is tied to the `approved_records` row it produced.
    pub submission_id: Uuid,
    pub outcome: ReviewOutcome,
    /// What the depositor is told. `None` is allowed for every outcome —
    /// requiring one is the handler's rule, not the record's, since a
    /// withdrawal has nobody to address.
    pub note: Option<String>,
    /// The serialized [`ReviewState`](crate::review::ReviewState) as it stood
    /// when the round ended, or `None` where nothing was decided. A snapshot:
    /// the submission is deleted by the same transaction.
    pub review_state: Option<String>,
    /// Who ended the round, kept when that account is removed; see
    /// [`DraftRecord::updated_by`]. For a withdrawal this is the depositor.
    pub actor: Option<Uuid>,
    pub at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submission_state_round_trips_through_its_stored_form() {
        for state in [
            SubmissionState::Submitted,
            SubmissionState::InReview,
            SubmissionState::Approved,
        ] {
            assert_eq!(state.as_str().parse::<SubmissionState>().unwrap(), state);
        }
    }

    #[test]
    fn test_unknown_stored_variant_is_an_error_not_a_default() {
        assert!("rejected".parse::<SubmissionState>().is_err());
    }

    #[test]
    fn test_review_outcome_round_trips_through_its_stored_form() {
        for outcome in [
            ReviewOutcome::Approved,
            ReviewOutcome::ChangesRequested,
            ReviewOutcome::Rejected,
            ReviewOutcome::Withdrawn,
        ] {
            assert_eq!(outcome.as_str().parse::<ReviewOutcome>().unwrap(), outcome);
        }
    }

    #[test]
    fn test_an_unknown_stored_review_outcome_is_an_error() {
        // The two vocabularies share `approved`, so reading either as the other
        // has to fail rather than land on whichever variant sorts first.
        assert!("submitted".parse::<ReviewOutcome>().is_err());
        assert!("in_review".parse::<ReviewOutcome>().is_err());
        assert_eq!("approved".parse::<ReviewOutcome>().unwrap(), ReviewOutcome::Approved);
    }

    #[test]
    fn test_only_approval_does_not_return_the_project() {
        assert!(!ReviewOutcome::Approved.returns_the_project());
        for outcome in [
            ReviewOutcome::ChangesRequested,
            ReviewOutcome::Rejected,
            ReviewOutcome::Withdrawn,
        ] {
            assert!(outcome.returns_the_project(), "{outcome}");
        }
    }

    #[test]
    fn test_pull_request_state_deserializes_from_its_stored_form() {
        // A `#[serde(rename_all)]` change here would otherwise silently disagree with both
        // `as_str` and the schema's `CHECK` constraint.
        for state in [
            PullRequestState::Open,
            PullRequestState::Merged,
            PullRequestState::Closed,
        ] {
            let wire = format!("\"{}\"", state.as_str());
            assert_eq!(serde_json::from_str::<PullRequestState>(&wire).unwrap(), state);
        }
    }
}
