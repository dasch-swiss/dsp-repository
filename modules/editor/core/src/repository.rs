//! The persistence ports, one trait per aggregate.
//!
//! Framework-free: the traits name domain records and [`RepositoryError`], never
//! a `rusqlite` type. `editor-server` implements all seven against SQLite, so the
//! handlers Phase 3 onwards writes depend on these and not on the driver.
//!
//! Every method is `async` and boxed via `async_trait` rather than left as a
//! bare `async fn` in a trait: the futures have to be `Send` to be awaited
//! inside an Axum handler. Native `async fn` in traits gives that only by
//! spelling out `-> impl Future + Send` on every signature.
//!
//! It also keeps the traits dyn-compatible. Nothing needs that yet — `AppState`
//! holds a concrete `Database` and every call site is UFCS on it — but a boxed
//! `Arc<dyn UserRepository>` is what a fault-injecting fake would need, and the
//! paths that swallow a storage error are currently untestable for want of one.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::records::{ApprovedRecord, DraftRecord, LoginCode, Session, Submission, User};

/// What can go wrong in a repository call.
///
/// [`Self::Backend`] keeps the driver error as a `source` without naming its
/// type, so this crate stays free of a database dependency.
///
/// Its `Display` includes that source. It has to: every call site logs with
/// `%error`, and `Display` on a thiserror type is exactly the format string — so
/// without the `{0}` every storage failure in the service logged the words
/// "storage backend failed" and nothing whatever about which one. The driver's
/// message names tables, columns and SQL parameter *names*, never bound values,
/// so this does not reopen the channel REQ-6.10 closes.
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
    #[error("storage backend failed: {0}")]
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

    /// Record a failed authentication and return the account's new consecutive-
    /// failure count. Survives code invalidation and resend by construction: the
    /// counter lives on the user, not on the code.
    ///
    /// `decay_before` makes the counter a rolling window rather than a ratchet.
    /// A failure whose predecessor is older than that instant starts the count at
    /// one; otherwise it adds to it. Without the decay the counter only ever
    /// rises, so once an account has reached its cap a *single* wrong entry after
    /// each lockout expires re-locks it — a permanent, cheap denial of service
    /// against any address an attacker knows is registered.
    ///
    /// NIST SP 800-63B-4 is not in the way: it requires that generating a new
    /// authentication secret not reset the count, and says nothing about an
    /// elapsed throttling window.
    async fn record_failed_login(&self, id: Uuid, at: DateTime<Utc>, decay_before: DateTime<Utc>) -> Result<u32>;

    /// Clear the counter and the instant. Only a successful authentication may
    /// call this.
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

    /// Drop sessions past their absolute expiry. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// What [`LoginCodeRepository::claim_attempt`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// One of the code's attempts is now spent, and the caller may compare.
    Claimed,
    /// The code has no attempts left (REQ-6.4).
    Exhausted,
    /// The code was already consumed — by an earlier request, or by one racing
    /// this one.
    AlreadySpent,
    /// There is no such code.
    Unknown,
}

/// What [`LoginCodeRepository::create_unless_issued_since`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issued {
    /// The code was inserted. The caller must now deliver it — and delete it if
    /// delivery fails, or a code nobody received sits behind an active cooldown.
    New,
    /// A code was issued to this user too recently, so nothing was stored.
    ///
    /// Carries nothing on purpose. Returning the outstanding code — or its
    /// browser binding — would be the obvious convenience and a hole: anyone who
    /// can post an address could then ask for the binding of a code already on
    /// its way to that address's owner, which is precisely what the binding
    /// exists to prevent. A browser that already holds the right token keeps it
    /// by being left alone, not by being handed it again.
    Cooled,
}

/// One-time login codes (REQ-6.1, REQ-6.4, REQ-6.5).
#[async_trait]
pub trait LoginCodeRepository: Send + Sync {
    async fn create(&self, code: &LoginCode) -> Result<()>;

    /// Insert `code` unless this user was already issued one at or after
    /// `not_before` — the resend cooldown (REQ-6.5), applied as a
    /// compare-and-set.
    ///
    /// Atomic rather than a read followed by [`Self::create`]: reads go to the
    /// reader pool, so two simultaneous requests for one address would both see
    /// no recent code, both insert, and both send. That is two live codes and
    /// two mails against a relay quota the global daily cap exists to protect.
    async fn create_unless_issued_since(&self, code: &LoginCode, not_before: DateTime<Utc>) -> Result<Issued>;

    /// The code a browser is bound to, by the token it holds. `None` for a token
    /// that matches nothing — which is the ordinary case for an address that was
    /// never known, since REQ-6.2 requires the browser be handed a token anyway.
    async fn find_by_browser_token(&self, token: &str) -> Result<Option<LoginCode>>;

    /// Delete one code. `false` if it was already gone. This is the rollback for
    /// a send that failed after the code was reserved.
    async fn delete(&self, id: Uuid) -> Result<bool>;

    /// The user's newest code that has not expired and has not been consumed.
    async fn find_active_for_user(&self, user_id: Uuid, now: DateTime<Utc>) -> Result<Option<LoginCode>>;

    /// Claim one of this code's three attempts (REQ-6.4), and report what
    /// happened.
    ///
    /// The check and the increment are one statement on purpose. Reading
    /// `attempts` and then incrementing it leaves a window in which every
    /// simultaneous submission passes the check, and REQ-6.4 is one of exactly
    /// two controls standing between a ~19.93-bit secret and a guesser — so the
    /// limit has to *be* the increment. Anything but [`Attempt::Claimed`] means
    /// the caller must not compare.
    ///
    /// The outcomes are distinguished because they are different events for an
    /// operator: [`Attempt::Exhausted`] is REQ-6.4 doing its job, while
    /// [`Attempt::AlreadySpent`] is usually one person with two tabs open, and
    /// telling the second one it used up its guesses sends support down the
    /// wrong path.
    async fn claim_attempt(&self, id: Uuid, max_attempts: u32) -> Result<Attempt>;

    /// Move a live code's binding from `presented` to `replacement`, and report
    /// whether one moved.
    ///
    /// The `WHERE browser_token = presented` is the authorisation: only a browser
    /// that already holds the binding can move it, so this cannot be used to
    /// acquire the binding of a code on its way to somebody else. It exists so
    /// that every `POST /login` can hand back a fresh token — which is what makes
    /// the response identical for an address with an account and one without —
    /// without stranding the code the requesting browser already owns.
    async fn rebind_browser_token(&self, presented: &str, replacement: &str) -> Result<bool>;

    /// Mark a code used, once. `false` means it was already consumed — a replay,
    /// which must not authenticate (NIST SP 800-63B-4 §3.1.3.2).
    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool>;

    /// Delete a user's codes that were never spent, leaving any consumed one in
    /// place.
    ///
    /// Called after a successful sign-in, to invalidate codes still live in
    /// browsers nobody is using. It deliberately does **not** take the consumed
    /// code with it, which an earlier version did: that row is the only anchor
    /// the resend cooldown has (REQ-6.5 measures from the last code *issued*),
    /// so deleting it let a user sign in and immediately be sent another code.
    ///
    /// The send caps no longer depend on any of this. They count
    /// [`MailSendRepository`] rows, which is why the siblings this deletes —
    /// codes that were mailed and then abandoned — no longer vanish from the
    /// count along with the rows.
    async fn delete_unconsumed_for_user(&self, user_id: Uuid) -> Result<u64>;

    /// Undo [`Self::consume`], returning the code to a spendable state.
    ///
    /// For the window between consuming a correct code and having a session to
    /// show for it: if session creation fails, the code has been spent, the user
    /// is told it was invalid, and the cooldown refuses them another — locked out
    /// by an error that was never theirs. Reopening it costs nothing, because
    /// nobody was authenticated.
    async fn unconsume(&self, id: Uuid) -> Result<bool>;

    /// Drop expired codes. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// An append-only record of the mail that went out, and the only thing the
/// daily send caps count.
///
/// Separate from [`LoginCodeRepository`] because it records a different fact. A
/// login code is state with a ten-minute life that is rolled back, spent and
/// swept; a send is an event that already happened and cannot be taken back.
/// Counting the former to bound the latter is what made the cap read low:
/// delivery failures remove rows for sends that never happened (correct), and a
/// successful sign-in removes the user's other unspent codes, which *were*
/// mailed (not correct).
///
/// Nothing here identifies a message beyond who it went to and when. The
/// recipient is the account id, never the address (REQ-6.10).
#[async_trait]
pub trait MailSendRepository: Send + Sync {
    /// Record one message as sent. Append-only: there is no update and no
    /// delete-by-id, only [`Self::delete_before`].
    async fn record(&self, user_id: Uuid, sent_at: DateTime<Utc>) -> Result<()>;

    /// Sends across all users since `since`, for the global daily cap — without
    /// it, looping resend exhausts the shared relay quota and locks out every
    /// user including RDU.
    async fn count_since(&self, since: DateTime<Utc>) -> Result<u64>;

    /// Sends to one account since `since`, for the per-account daily cap.
    ///
    /// The global cap alone does not bound this: the resend cooldown is per
    /// address, so at its sixty-second default one address can be sent 1,440
    /// codes a day against a global default of 500. One attacker with one known
    /// address could therefore spend the whole shared budget and stop everyone
    /// signing in, which is the outage the global cap exists to prevent,
    /// reached about twenty times more cheaply than exhausting the relay.
    async fn count_for_user_since(&self, user_id: Uuid, since: DateTime<Utc>) -> Result<u64>;

    /// Drop sends older than `cutoff`. Returns how many went.
    ///
    /// The caller passes the caps' own window, so retention and the count are
    /// the same span by construction: pruning anything the count still reads
    /// would silently free budget.
    async fn delete_before(&self, cutoff: DateTime<Utc>) -> Result<u64>;
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
