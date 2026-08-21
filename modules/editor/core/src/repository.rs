//! The persistence ports, one trait per aggregate.
//!
//! Framework-free: the traits name domain records and [`RepositoryError`], never
//! a `rusqlite` type. `editor-server` implements all six against SQLite, so the
//! handlers Phase 3 onwards writes depend on these and not on the driver.
//!
//! Every method is `async` and boxed via `async_trait` rather than left as a
//! bare `async fn` in a trait: the futures have to be `Send` to be awaited
//! inside an Axum handler, and the traits have to stay dyn-compatible so state
//! can hold `Arc<dyn UserRepository>`. Native `async fn` in traits gives neither
//! without spelling out `-> impl Future + Send` on every signature.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::records::{ApprovedRecord, DraftRecord, LoginCode, Session, Submission, User};

/// What can go wrong in a repository call.
///
/// [`Self::Backend`] keeps the driver error as a `source` without naming its
/// type, so the error chain survives into the logs while this crate stays free
/// of a database dependency.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// The row addressed by an update or delete does not exist.
    #[error("{entity} not found")]
    NotFound { entity: &'static str },

    /// A uniqueness constraint rejected the write — a duplicate email
    /// (REQ-7.4), or a second pending submission for one project.
    #[error("{entity} already exists")]
    Conflict { entity: &'static str },

    /// A stored value the code cannot interpret; see
    /// [`crate::records::UnknownVariant`].
    #[error("stored data could not be read: {0}")]
    Corrupt(String),

    /// Anything the storage backend itself reported.
    #[error("storage backend failed")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl RepositoryError {
    /// Wrap a backend error.
    pub fn backend<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Backend(Box::new(error))
    }
}

/// Shorthand for repository results.
pub type Result<T> = std::result::Result<T, RepositoryError>;

/// Accounts (US-7) and the account-level failure counter the login flow needs.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Insert a user, together with its shortcode assignments.
    ///
    /// Returns [`RepositoryError::Conflict`] if the normalized address is
    /// already taken (REQ-7.4).
    async fn create(&self, user: &User) -> Result<()>;

    /// Replace name, address, role and shortcode assignments.
    ///
    /// US-7 has create and remove only; update exists because removing a
    /// shortcode from someone holding a draft on it is otherwise undefined.
    async fn update(&self, user: &User) -> Result<()>;

    /// Delete a user. `ON DELETE CASCADE` takes its sessions, codes and
    /// shortcode assignments with it (REQ-7.5); its drafts and submissions
    /// survive with a null author.
    async fn delete(&self, id: Uuid) -> Result<()>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;

    /// Look up by address, case-insensitively — the argument is normalized
    /// before the query, so callers pass whatever the user typed.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Every user, for the RDU depositor list.
    async fn list(&self) -> Result<Vec<User>>;

    /// Increment the account-level consecutive-failure counter and return its
    /// new value. Survives code invalidation and resend by construction: it
    /// lives on the user, not on the code.
    async fn record_failed_login(&self, id: Uuid) -> Result<u32>;

    /// Clear the counter. Only a successful authentication may call this.
    async fn clear_failed_logins(&self, id: Uuid) -> Result<()>;

    /// Stamp when a code was last issued to this user (REQ-6.10 diagnosis
    /// without an address in a log).
    async fn record_code_issued(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
}

/// Sessions (REQ-6.3, REQ-6.6, REQ-7.5).
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &Session) -> Result<()>;

    async fn find(&self, id: &str) -> Result<Option<Session>>;

    /// Advance `last_seen_at` for the idle timeout.
    async fn touch(&self, id: &str, at: DateTime<Utc>) -> Result<()>;

    /// Delete one session (REQ-6.6). `false` if it was already gone.
    async fn delete(&self, id: &str) -> Result<bool>;

    /// Delete every session belonging to a user, for logout-everywhere and for
    /// session rotation on login. Account removal does not need this — the
    /// foreign key cascades — but rotation does.
    async fn delete_for_user(&self, user_id: Uuid) -> Result<u64>;

    /// Drop sessions past their absolute expiry. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// One-time login codes (REQ-6.1, REQ-6.4, REQ-6.5).
#[async_trait]
pub trait LoginCodeRepository: Send + Sync {
    async fn create(&self, code: &LoginCode) -> Result<()>;

    /// The user's newest code that has not expired and has not been consumed.
    async fn find_active_for_user(&self, user_id: Uuid, now: DateTime<Utc>) -> Result<Option<LoginCode>>;

    /// The user's newest code whatever its state, so the resend cooldown can be
    /// measured against it (REQ-6.5).
    async fn find_latest_for_user(&self, user_id: Uuid) -> Result<Option<LoginCode>>;

    /// Increment this code's wrong-entry count and return the new value. Three
    /// invalidates it (REQ-6.4).
    async fn record_attempt(&self, id: Uuid) -> Result<u32>;

    /// Mark a code used, once. `false` means it was already consumed — a replay,
    /// which must not authenticate (NIST SP 800-63B-4 §3.1.3.2).
    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool>;

    /// Delete every outstanding code for a user, for the three-strike
    /// invalidation and before issuing a replacement.
    async fn delete_for_user(&self, user_id: Uuid) -> Result<u64>;

    /// How many codes were issued across all users since `since`, for the global
    /// daily send cap — without it, looping resend exhausts the relay quota and
    /// locks out every user including RDU.
    async fn count_issued_since(&self, since: DateTime<Utc>) -> Result<u64>;

    /// Drop expired codes. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// Drafts (REQ-1.10, REQ-1.11).
#[async_trait]
pub trait DraftRepository: Send + Sync {
    /// Insert or replace the project's draft. Last write wins, per PRD
    /// Constraints.
    async fn upsert(&self, draft: &DraftRecord) -> Result<()>;

    async fn find(&self, shortcode: &str) -> Result<Option<DraftRecord>>;

    /// Every draft, newest first — RDU sees all of them, so it can help a
    /// depositor who is stuck before submission (REQ-1.11).
    async fn list(&self) -> Result<Vec<DraftRecord>>;

    /// `false` if there was no draft to delete.
    async fn delete(&self, shortcode: &str) -> Result<bool>;
}

/// Submissions (REQ-1.12, US-4).
#[async_trait]
pub trait SubmissionRepository: Send + Sync {
    /// Record a project's pending submission. [`RepositoryError::Conflict`] if
    /// one is already pending for that shortcode.
    async fn create(&self, submission: &Submission) -> Result<()>;

    /// Replace state, reviewer, review time and note.
    async fn update(&self, submission: &Submission) -> Result<()>;

    async fn find(&self, id: Uuid) -> Result<Option<Submission>>;

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Option<Submission>>;

    /// The review queue: every pending submission, oldest first (REQ-4.1).
    async fn list(&self) -> Result<Vec<Submission>>;

    /// `false` if there was no submission to delete. Covers reject (REQ-4.6),
    /// depositor discard (REQ-4.7) and the move to approved.
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

/// Approved records awaiting collection (US-5).
#[async_trait]
pub trait ApprovedRecordRepository: Send + Sync {
    async fn create(&self, record: &ApprovedRecord) -> Result<()>;

    /// What the public collection endpoint serves: approved and not yet
    /// collected, oldest first (REQ-5.1).
    async fn list_uncollected(&self) -> Result<Vec<ApprovedRecord>>;

    /// Every approved record for a project, so the startup comparison can find
    /// the one that matches the published data (REQ-2.3).
    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Vec<ApprovedRecord>>;

    /// Stamp a record as collected. Leaving it unstamped is what makes a failed
    /// collection retry on the next run (REQ-5.7).
    async fn mark_collected(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;

    /// `false` if there was no record to delete. Used when the change goes
    /// Online and the local record is discarded (REQ-2.4).
    async fn delete(&self, id: Uuid) -> Result<bool>;
}
