//! The records the editor persists.
//!
//! Framework-free on purpose: no `rusqlite`, no Axum, no Maud. The SQLite
//! column mapping lives in `editor-server`, behind the ports in
//! [`crate::repository`].
//!
//! Three of these carry their body as an opaque `payload: String` of JSON,
//! which is a [`ProjectDraft`](crate::draft::ProjectDraft) serialized: that type
//! is `#[serde(transparent)]` over the project's members, so the column holds
//! the project object itself and needs no migration to become typed. It stays a
//! `String` here because this layer never interprets it, and the caller that
//! does (the form's save and submit path) is the one that should decide when to
//! parse.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The two roles the editor recognises (REQ-7.1).
///
/// `rdu` members come from configuration and always exist without provisioning
/// (REQ-7.2); depositors are rows created by RDU (REQ-7.3).
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
/// One definition, because four things key on a shortcode and three of them are
/// writes. `drafts`, `submissions` and `approved_records` all carry it as an
/// exact-match column, while [`PublishedProjects::get`](crate::published::PublishedProjects::get)
/// and [`User::may_reach`] both fold — the published set mixes `080C` with
/// `0801a`, so a link typed either way has to reach the same project. Keying a
/// write on the shortcode as typed would therefore give `/projects/080c` and
/// `/projects/080C` a **row each** for one project, and two people editing it
/// would each keep half the edits with nothing to say so. `submissions.shortcode`
/// is unique, so the same mismatch would also make a pending-submission check
/// silently miss.
///
/// ASCII rather than Unicode folding: `is_valid_shortcode` admits only ASCII
/// alphanumerics, so the two cannot disagree about what a shortcode is.
#[must_use]
pub fn normalize_shortcode(shortcode: &str) -> String {
    shortcode.trim().to_ascii_lowercase()
}

/// A person who can log in.
///
/// `email` is stored as entered and in plaintext (PRD Constraints: the
/// application must decrypt it to send mail, so a key would sit beside the
/// data). `email_normalized` — lowercased — carries the uniqueness constraint
/// and every lookup, so REQ-7.4 rejects `A@x.test` against a stored `a@x.test`
/// and REQ-6.2's anti-enumeration lookup is case-insensitive too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    /// Project shortcodes this user may reach (REQ-1.2, REQ-7.3). Empty for an
    /// RDU member, whose access is role-based rather than per-project
    /// (REQ-4.2).
    pub shortcodes: Vec<String>,
    /// Consecutive failed authentications **for the account**, not for a code.
    /// NIST SP 800-63B-4: "Generating a new authentication secret SHALL NOT
    /// reset the failed authentication count" — so this survives code
    /// invalidation and resend, and only a successful login clears it.
    pub failed_logins: u32,
    /// When [`Self::failed_logins`] last went up, and therefore when a lockout
    /// started. The counter alone cannot express a lockout that ends: it resets
    /// only on success, and a locked-out account cannot succeed. Throttling is
    /// time-based for that reason, and this is what it measures from. Cleared
    /// with the counter.
    pub failed_login_at: Option<DateTime<Utc>>,
    /// When a login code was last issued, so RDU can answer "I never got a
    /// code" without an address reaching a log (REQ-6.10).
    pub last_code_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Whether this user may reach the project identified by `shortcode`.
    ///
    /// RDU is unconditional: REQ-4.2 makes RDU access role-based rather than
    /// per-project, which is also why an RDU account's `shortcodes` is empty. A
    /// depositor is scoped to their assignments (REQ-1.2), and anything else is
    /// REQ-1.3's 403.
    ///
    /// The comparison ignores ASCII case. The published set mixes `080C` with
    /// `0801a`, so which half of a shortcode is capitalised is not something an
    /// RDU member typing an assignment can be expected to get right — and
    /// getting it wrong would deny a depositor their own project with no visible
    /// cause. Two projects differing only in case would make this too generous;
    /// no such pair exists in the published set.
    ///
    /// This is [`normalize_shortcode`]'s rule, compared rather than keyed — the
    /// allocation-free form, since nothing here needs the string. It deliberately
    /// does *not* trim: an argument reaches this only after `is_valid_shortcode`,
    /// which admits no whitespace, and an authorization check is the last place
    /// to be more permissive than the thing that validated its input.
    /// [`tests::the_assignment_comparison_agrees_with_the_storage_key`] pins the
    /// two against each other.
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
    /// `to_lowercase` rather than `to_ascii_lowercase` so a non-ASCII address
    /// folds too. Only the whole address is folded — the local part is
    /// case-sensitive per RFC 5321, but no mail provider anyone here uses
    /// treats it that way, and letting `A@x.test` and `a@x.test` both exist
    /// would make REQ-7.4 depend on how the address was typed.
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

/// An authenticated session (REQ-6.3).
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

/// A one-time login code (REQ-6.1).
///
/// Stored unhashed, deliberately: it lives ten minutes, and anyone who can read
/// this table already holds `sessions` (PRD Constraints).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    /// Wrong entries against *this* code. Three invalidates it (REQ-6.4); the
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
    /// This is what blocks the attack email one-time codes are most exposed to:
    /// an attacker triggers a login for the victim, talks them into reading the
    /// code out, and spends it from their own machine. `None` binds to no
    /// browser and so can never be verified — the fail-closed reading, which
    /// matters because a row predating the column has it.
    pub browser_token: Option<String>,
}

/// Work in progress on one project (REQ-1.10).
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
    /// The note RDU left when it returned this project to the depositor
    /// (REQ-4.5). `None` for a draft nobody has reviewed.
    ///
    /// It lives on the draft rather than on the submission because
    /// request-changes turns the submission *into* a draft: the row carrying
    /// the note is deleted at the moment the depositor needs to read it, so a
    /// note stored there could never be shown to the person it is addressed to.
    /// Cleared when the project is submitted again, since it describes the
    /// round that has now been answered.
    pub reviewer_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Where a submission sits in review.
///
/// `Draft` and `Online` from REQ-2.1 are deliberately absent: a draft is a
/// `drafts` row, and Online is derived at startup by comparing against the
/// published set, at which point the local record is discarded (REQ-2.4).
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

/// A submission awaiting or under review (REQ-1.12, REQ-4.x).
///
/// One per project at a time — the schema makes `shortcode` unique, which is
/// PRD Constraints' "one pending submission per project" rather than a
/// convention handlers have to remember.
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
    /// Carried back to the depositor when RDU requests changes (REQ-4.5).
    pub reviewer_note: Option<String>,
    /// A serialized [`ReviewState`](crate::review::ReviewState) — the per-field
    /// decisions and substitutions RDU has recorded so far (REQ-4.3). `None`
    /// while nothing has been decided.
    ///
    /// Opaque here for the same reason as `payload`: this layer stores it and
    /// the reviewing handler is the one that should decide when to parse it,
    /// and what to do about a payload it cannot read.
    pub review_state: Option<String>,
}

/// An approved record waiting to be collected into a pull request (REQ-5.1).
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
    /// next run retries it (REQ-5.7).
    pub collected_at: Option<DateTime<Utc>>,
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
        // REQ-1.2, and REQ-1.3's 403 is the other half.
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
        // The published set mixes `080C` with `0801a`, so an RDU member typing
        // an assignment cannot be expected to get the case right — and getting
        // it wrong would deny a depositor their own project with no visible
        // cause.
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
        // REQ-4.2: RDU access is role-based, not per-project, which is why an
        // RDU account's `shortcodes` is empty.
        let rdu = user(Role::Rdu, &[]);
        assert!(rdu.may_reach("0801"));
        assert!(rdu.may_reach("anything"));
        assert!(rdu.is_rdu());
        assert!(!user(Role::Depositor, &[]).is_rdu());
    }
}
