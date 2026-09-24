//! The records the editor persists.
//!
//! Framework-free: the SQLite column mapping lives in `editor-server`, behind
//! the ports in [`crate::repository`]. A `payload: String` is a serialized
//! [`ProjectDraft`](crate::draft::ProjectDraft), which is `#[serde(transparent)]`
//! over the project's members; it stays opaque here because this layer never
//! interprets it.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The two roles the editor recognises.
///
/// `rdu` members come from configuration and always exist without
/// provisioning; depositors are rows created by RDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Depositor,
    Rdu,
}

impl Role {
    /// The stored form. Pinned by a `CHECK` constraint in the schema, so this
    /// and the migration have to agree.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Depositor => "depositor",
            Self::Rdu => "rdu",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A stored value outside the known set. Reachable only if the database is
/// edited by hand or a migration adds a variant the code does not know.
#[derive(Debug, thiserror::Error)]
#[error("unknown {kind} {value:?}")]
pub struct UnknownVariant {
    pub kind: &'static str,
    pub value: String,
}

impl FromStr for Role {
    type Err = UnknownVariant;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "depositor" => Ok(Self::Depositor),
            "rdu" => Ok(Self::Rdu),
            other => Err(UnknownVariant { kind: "role", value: other.to_string() }),
        }
    }
}

/// The canonical storage key for a project shortcode: trimmed, ASCII-folded.
///
/// One definition, because `drafts`, `submissions` and `approved_records` key on
/// it as an exact-match column while
/// [`PublishedProjects::get`](crate::published::PublishedProjects::get) and
/// [`User::may_reach`] fold. Keying a write on the shortcode as typed would give
/// `/projects/080c` and `/projects/080C` a row each, and the unique
/// `submissions.shortcode` check would miss a pending submission. ASCII rather
/// than Unicode folding because `is_valid_shortcode` admits only ASCII.
#[must_use]
pub fn normalize_shortcode(shortcode: &str) -> String {
    shortcode.trim().to_ascii_lowercase()
}

/// A person who can log in.
///
/// `email` is stored as entered and in plaintext: the application must read it
/// to send mail, so a key would sit beside the data. `email_normalized`,
/// lowercased, carries the uniqueness constraint and every lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    /// Project shortcodes this user may reach. Empty for an RDU member, whose
    /// access is role-based rather than per-project.
    pub shortcodes: Vec<String>,
    /// Consecutive failed authentications **for the account**, not for a code.
    /// NIST SP 800-63B-4: "Generating a new authentication secret SHALL NOT
    /// reset the failed authentication count" — so this survives code
    /// invalidation and resend, and only a successful login clears it.
    pub failed_logins: u32,
    /// When [`Self::failed_logins`] last went up, and so when a lockout started.
    /// The counter alone cannot express a lockout that ends, since it resets only
    /// on success; throttling measures from this. Cleared with the counter.
    pub failed_login_at: Option<DateTime<Utc>>,
    /// When a login code was last issued, so RDU can answer "I never got a
    /// code" without an address reaching a log.
    pub last_code_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Whether this user may reach the project identified by `shortcode`.
    ///
    /// RDU is unconditional: access is role-based, which is why an RDU account's
    /// `shortcodes` is empty. A depositor is scoped to their assignments.
    ///
    /// The comparison ignores ASCII case: the published set mixes `080C` with
    /// `0801a`, and no two published shortcodes differ only in case. This is
    /// [`normalize_shortcode`]'s rule, compared rather than keyed. It does not
    /// trim: the argument has passed `is_valid_shortcode`, which admits no
    /// whitespace, and an authorization check must not be more permissive than
    /// the validator. `tests::the_assignment_comparison_agrees_with_the_storage_key`
    /// pins the two against each other.
    #[must_use]
    pub fn may_reach(&self, shortcode: &str) -> bool {
        match self.role {
            Role::Rdu => true,
            Role::Depositor => self.shortcodes.iter().any(|assigned| assigned.eq_ignore_ascii_case(shortcode)),
        }
    }

    /// Whether this account administers the service.
    #[must_use]
    pub fn is_rdu(&self) -> bool {
        self.role == Role::Rdu
    }

    /// The lookup and uniqueness key: `email` lowercased.
    ///
    /// `to_lowercase` rather than `to_ascii_lowercase`, so a non-ASCII address
    /// folds too. The whole address is folded: RFC 5321 makes the local part
    /// case-sensitive, but no provider does, and two spellings of one address
    /// would make "already taken" depend on how it was typed.
    #[must_use]
    pub fn normalize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }

    /// This user's normalized address.
    #[must_use]
    pub fn email_normalized(&self) -> String {
        Self::normalize_email(&self.email)
    }
}

/// An authenticated session.
///
/// `id` is the opaque token carried by the cookie, not a UUID: how it is minted
/// is the auth layer's decision, and this layer only stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    /// Advanced on use, for the idle timeout.
    pub last_seen_at: DateTime<Utc>,
    /// Absolute expiry, set at creation and never extended.
    pub expires_at: DateTime<Utc>,
}

/// A one-time login code.
///
/// Stored unhashed, deliberately: it lives ten minutes, and anyone who can read
/// this table already holds `sessions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    /// Wrong entries against *this* code. Three invalidates it; the
    /// account-level counter that survives a resend is [`User::failed_logins`].
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Set when the code is accepted. A code is usable once, for replay
    /// resistance (NIST SP 800-63B-4 §3.1.3.2).
    pub consumed_at: Option<DateTime<Utc>>,
    /// The opaque token held by the browser that asked for this code, and the
    /// only browser that may spend it.
    ///
    /// Blocks the attack email codes are most exposed to: triggering a login for
    /// the victim and spending the read-out code from another machine. `None`
    /// binds to no browser and can never be verified, which rows predating the
    /// column rely on.
    pub browser_token: Option<String>,
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
    /// The last editor, `None` once that user is removed — the row survives so
    /// the depositor's work is not destroyed by an account deletion, and the
    /// review queue's "last editor" column reads as unknown rather than
    /// dangling.
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
    /// `None` once the submitter is removed; see [`DraftRecord::updated_by`].
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
    /// `None` once the approver is removed; see [`DraftRecord::updated_by`].
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
    /// Who ended the round, `None` once that account is removed; see
    /// [`DraftRecord::updated_by`]. For a withdrawal this is the depositor.
    pub actor: Option<Uuid>,
    pub at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_round_trips_through_its_stored_form() {
        for role in [Role::Depositor, Role::Rdu] {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), role);
        }
    }

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
        // A row the code does not understand must surface, not silently become
        // `Depositor` — that would hand an unknown role a depositor's access.
        assert!("admin".parse::<Role>().is_err());
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

    #[test]
    fn test_normalize_email_folds_case_and_trims() {
        assert_eq!(User::normalize_email("  A.User@Example.TEST "), "a.user@example.test");
    }

    fn user(role: Role, shortcodes: &[&str]) -> User {
        User {
            id: Uuid::nil(),
            email: "a@x.test".to_string(),
            name: "A".to_string(),
            role,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: DateTime::<Utc>::MIN_UTC,
        }
    }

    #[test]
    fn test_a_depositor_reaches_only_the_projects_assigned_to_them() {
        let depositor = user(Role::Depositor, &["0801", "0812"]);
        assert!(depositor.may_reach("0801"));
        assert!(depositor.may_reach("0812"));
        assert!(!depositor.may_reach("0803"));
    }

    #[test]
    fn test_a_depositor_with_no_assignments_reaches_nothing() {
        assert!(!user(Role::Depositor, &[]).may_reach("0801"));
    }

    #[test]
    fn test_an_assignment_matches_however_it_is_capitalised() {
        let depositor = user(Role::Depositor, &["080c"]);
        assert!(depositor.may_reach("080C"));
        assert!(depositor.may_reach("080c"));
        assert!(!depositor.may_reach("080E"));
    }

    #[test]
    fn the_assignment_comparison_agrees_with_the_storage_key() {
        // Two expressions of one rule: `may_reach` compares, `normalize_shortcode`
        // keys. If they ever disagreed, a depositor could reach a project whose
        // draft they cannot load — or load someone else's. Checked over the
        // shapes the published set actually contains, mixed case included.
        for (assigned, requested) in [
            ("0801", "0801"),
            ("080C", "080c"),
            ("080c", "080C"),
            ("0801a", "0801A"),
            ("085F", "085f"),
        ] {
            assert!(
                user(Role::Depositor, &[assigned]).may_reach(requested),
                "{assigned} should reach {requested}"
            );
            assert_eq!(
                normalize_shortcode(assigned),
                normalize_shortcode(requested),
                "{assigned} and {requested} must key the same"
            );
        }
        for (assigned, requested) in [("0801", "0803"), ("080C", "080E")] {
            assert!(!user(Role::Depositor, &[assigned]).may_reach(requested));
            assert_ne!(normalize_shortcode(assigned), normalize_shortcode(requested));
        }
    }

    #[test]
    fn test_rdu_reaches_every_project_without_an_assignment() {
        let rdu = user(Role::Rdu, &[]);
        assert!(rdu.may_reach("0801"));
        assert!(rdu.may_reach("anything"));
        assert!(rdu.is_rdu());
        assert!(!user(Role::Depositor, &[]).is_rdu());
    }
}
