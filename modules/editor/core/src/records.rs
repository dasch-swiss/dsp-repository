//! The records the editor persists.
//!
//! Framework-free on purpose: no `rusqlite`, no Axum, no Maud. The SQLite
//! column mapping lives in `editor-server`, behind the ports in
//! [`crate::repository`].
//!
//! Three of these carry their body as an opaque `payload: String` of JSON. The
//! permissive draft representation is Phase 4's work (`ProjectRaw` requires
//! fields a draft must be allowed to omit), and inventing a type for it here
//! would mean inventing it twice. The persistence layer never interprets the
//! payload, so typing it later changes these structs and nothing else.

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
    /// JSON. Typed in Phase 4; see the module docs.
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
    /// JSON. Typed in Phase 4; see the module docs.
    pub payload: String,
    pub state: SubmissionState,
    /// `None` once the submitter is removed; see [`DraftRecord::updated_by`].
    pub submitted_by: Option<Uuid>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    /// Carried back to the depositor when RDU requests changes (REQ-4.5).
    pub reviewer_note: Option<String>,
}

/// An approved record waiting to be collected into a pull request (REQ-5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedRecord {
    pub id: Uuid,
    pub shortcode: String,
    /// JSON. Typed in Phase 4; see the module docs.
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
}
