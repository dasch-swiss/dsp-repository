//! The persistence ports for accounts, sessions, login codes and the mail
//! ledger.
//!
//! Framework-free: the traits name identity's records and [`RepositoryError`],
//! never a `rusqlite` type. Every method is `async` and boxed via `async_trait`
//! rather than a bare `async fn` in a trait: the futures have to be `Send` to be
//! awaited inside an Axum handler, and the traits have to stay dyn-compatible
//! for the `Arc<dyn IdentityRepositories>` that `identity-web`'s `IdentityState`
//! holds, which is how a test puts a fake behind it and makes a chosen call
//! fail.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::records::{LoginCode, Session, User};

/// What can go wrong in a repository call.
///
/// [`Self::Backend`] keeps the driver error as a `source` without naming its
/// type, so this crate stays free of a database dependency. Its `Display`
/// includes that source, because every call site logs with `%error`. The
/// driver's message names tables, columns and parameter names, never bound
/// values, so it does not reopen the address-disclosure channel.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// The row addressed by an update or delete does not exist.
    #[error("{entity} not found")]
    NotFound { entity: &'static str },

    /// A uniqueness constraint rejected the write — a duplicate email, or a
    /// second pending submission for one project.
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

/// Accounts and the account-level failure counter the login flow needs.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Insert a user, together with its assigned projects.
    ///
    /// Returns [`RepositoryError::Conflict`] if the normalized address is
    /// already taken.
    async fn create(&self, user: &User) -> Result<()>;

    /// Replace name, address, role and assigned projects. Exists because
    /// removing a shortcode from someone holding a draft on it is otherwise
    /// undefined.
    async fn update(&self, user: &User) -> Result<()>;

    /// Delete a user. `ON DELETE CASCADE` takes its sessions, codes and
    /// assigned projects with it. Editor rows it authored are not
    /// identity's, and keep its id.
    async fn delete(&self, id: Uuid) -> Result<()>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;

    /// Look up by address, case-insensitively — the argument is normalized
    /// before the query, so callers pass whatever the user typed.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Every user, for the RDU depositor list.
    async fn list(&self) -> Result<Vec<User>>;

    /// Record a failed authentication and return the account's new consecutive-
    /// failure count. The counter lives on the user, not on the code, so it
    /// survives code invalidation and resend.
    ///
    /// `decay_before` makes the counter a rolling window: a failure whose
    /// predecessor is older than that instant starts the count at one. Without
    /// it a single wrong entry after each lockout expires re-locks the account
    /// forever, a cheap denial of service against any known address. NIST SP
    /// 800-63B-4 forbids resetting the count on a new secret and says nothing
    /// about an elapsed window.
    async fn record_failed_login(&self, id: Uuid, at: DateTime<Utc>, decay_before: DateTime<Utc>) -> Result<u32>;

    /// Clear the counter and the instant. Only a successful authentication may
    /// call this.
    async fn clear_failed_logins(&self, id: Uuid) -> Result<()>;

    /// Stamp when a code was last issued to this user, so a send can be diagnosed without an
    /// address in a log.
    async fn record_code_issued(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
}

/// Sessions.
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, session: &Session) -> Result<()>;

    async fn find(&self, id: &str) -> Result<Option<Session>>;

    /// Advance `last_seen_at` for the idle timeout.
    async fn touch(&self, id: &str, at: DateTime<Utc>) -> Result<()>;

    /// Delete one session. `false` if it was already gone.
    async fn delete(&self, id: &str) -> Result<bool>;

    /// Drop sessions past their absolute expiry. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// What [`LoginCodeRepository::claim_attempt`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attempt {
    /// One of the code's attempts is now spent, and the caller may compare.
    Claimed,
    /// The code has no attempts left.
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
    /// Carries nothing: returning the outstanding code or its binding would
    /// hand anyone who can post an address the binding of a code on its way to
    /// that address's owner.
    Cooled,
}

/// One-time login codes.
#[async_trait]
pub trait LoginCodeRepository: Send + Sync {
    async fn create(&self, code: &LoginCode) -> Result<()>;

    /// Insert `code` unless this user was already issued one at or after
    /// `not_before`: the resend cooldown, as a compare-and-set.
    ///
    /// Atomic rather than a read followed by [`Self::create`]: reads go to the
    /// reader pool, so two simultaneous requests for one address would both see
    /// no recent code, both insert and both send.
    async fn create_unless_issued_since(&self, code: &LoginCode, not_before: DateTime<Utc>) -> Result<Issued>;

    /// The code a browser is bound to, by the token it holds. `None` for a token
    /// that matches nothing — which is the ordinary case for an address that was
    /// never known, since anti-enumeration requires the browser be handed a token anyway.
    async fn find_by_browser_token(&self, token: &str) -> Result<Option<LoginCode>>;

    /// Delete one code. `false` if it was already gone. This is the rollback for
    /// a send that failed after the code was reserved.
    async fn delete(&self, id: Uuid) -> Result<bool>;

    /// The user's newest code that has not expired and has not been consumed.
    async fn find_active_for_user(&self, user_id: Uuid, now: DateTime<Utc>) -> Result<Option<LoginCode>>;

    /// Claim one of this code's attempts, and report what happened.
    ///
    /// The check and the increment are one statement: read-then-increment lets
    /// every simultaneous submission pass the check, and the attempt limit is
    /// one of two controls between a ~20-bit secret and a guesser. Anything but
    /// [`Attempt::Claimed`] means the caller must not compare.
    /// [`Attempt::Exhausted`] is the limit doing its job; [`Attempt::AlreadySpent`]
    /// is usually one person with two tabs, and telling them they used up their
    /// guesses sends support down the wrong path.
    async fn claim_attempt(&self, id: Uuid, max_attempts: u32) -> Result<Attempt>;

    /// Move a live code's binding from `presented` to `replacement`, and report
    /// whether one moved.
    ///
    /// The `WHERE browser_token = presented` is the authorisation: only a browser
    /// that already holds the binding can move it. Exists so every `POST /login`
    /// can hand back a fresh token, which keeps the response identical for a
    /// known and an unknown address, without stranding the code the browser owns.
    async fn rebind_browser_token(&self, presented: &str, replacement: &str) -> Result<bool>;

    /// Mark a code used, once. `false` means it was already consumed — a replay,
    /// which must not authenticate (NIST SP 800-63B-4 §3.1.3.2).
    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool>;

    /// Delete a user's codes that were never spent, leaving any consumed one in
    /// place.
    ///
    /// Called after a successful sign-in. The consumed code stays because the
    /// resend cooldown is measured from the last code issued; deleting it would
    /// let a user sign in and immediately be sent another code.
    async fn delete_unconsumed_for_user(&self, user_id: Uuid) -> Result<u64>;

    /// Undo [`Self::consume`], returning the code to a spendable state.
    ///
    /// For session creation failing after a correct code was consumed: otherwise
    /// the user is told the code was invalid and the cooldown refuses another,
    /// locked out by an error that was never theirs. Reopening costs nothing,
    /// because nobody was authenticated.
    async fn unconsume(&self, id: Uuid) -> Result<bool>;

    /// Drop expired codes. Returns how many went.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64>;
}

/// An append-only record of the mail that went out, and the only thing the
/// daily send caps count.
///
/// Separate from [`LoginCodeRepository`] because a send is an event that
/// already happened, while a code is state that is rolled back, spent and
/// swept; counting codes made the cap read low. Nothing here identifies a
/// message beyond who it went to and when: the recipient is the account id,
/// never the address.
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
    /// The global cap alone does not bound this: the cooldown is per address, so
    /// at its sixty-second default one address can be sent 1,440 codes a day
    /// against a global default of 500, and one known address could spend the
    /// whole shared budget.
    async fn count_for_user_since(&self, user_id: Uuid, since: DateTime<Utc>) -> Result<u64>;

    /// Drop sends older than `cutoff`. Returns how many went.
    ///
    /// The caller passes the caps' own window, so retention and the count are
    /// one span; pruning anything the count still reads would free budget.
    async fn delete_before(&self, cutoff: DateTime<Utc>) -> Result<u64>;
}

/// The four identity ports at once, so one handle can serve all of them.
///
/// The four traits are the units of dependency: a function that only looks up
/// accounts says `&dyn UserRepository`. This is for a composition root that
/// holds one handle for all of them, since Rust has no trait object over
/// several traits. The blanket impl means neither the SQLite implementation nor
/// a test fake writes `impl IdentityRepositories`, and a method is still called
/// through the port that declares it, so nothing here widens what a call site
/// reaches.
pub trait IdentityRepositories: UserRepository + SessionRepository + LoginCodeRepository + MailSendRepository {}

impl<T> IdentityRepositories for T where T: UserRepository + SessionRepository + LoginCodeRepository + MailSendRepository
{}
