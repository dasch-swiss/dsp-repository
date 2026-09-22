//! The persistence ports, one trait per aggregate.
//!
//! Framework-free: the traits name domain records and [`RepositoryError`], never
//! a `rusqlite` type. Every method is `async` and boxed via `async_trait`
//! rather than a bare `async fn` in a trait: the futures have to be `Send` to be
//! awaited inside an Axum handler, and the traits have to stay dyn-compatible
//! for the `Arc<dyn Repositories>` in `AppState`, which is how a test puts a
//! fake behind it and makes a chosen call fail.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::proposals::{EntityProposal, ProposalDecision};
use crate::records::{
    ApprovedRecord, DraftRecord, LoginCode, PullRequestState, ReviewRound, Session, Submission, User,
};

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
    /// Insert a user, together with its shortcode assignments.
    ///
    /// Returns [`RepositoryError::Conflict`] if the normalized address is
    /// already taken.
    async fn create(&self, user: &User) -> Result<()>;

    /// Replace name, address, role and shortcode assignments. Exists because
    /// removing a shortcode from someone holding a draft on it is otherwise
    /// undefined.
    async fn update(&self, user: &User) -> Result<()>;

    /// Delete a user. `ON DELETE CASCADE` takes its sessions, codes and
    /// shortcode assignments with it; its drafts and submissions
    /// survive with a null author.
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

/// Drafts.
#[async_trait]
pub trait DraftRepository: Send + Sync {
    /// Insert or replace the project's draft. Last write wins.
    async fn upsert(&self, draft: &DraftRecord) -> Result<()>;

    async fn find(&self, shortcode: &str) -> Result<Option<DraftRecord>>;

    /// Every draft, newest first — RDU sees all of them, so it can help a
    /// depositor who is stuck before submission.
    async fn list(&self) -> Result<Vec<DraftRecord>>;

    /// `false` if there was no draft to delete.
    async fn delete(&self, shortcode: &str) -> Result<bool>;
}

/// Submissions.
#[async_trait]
pub trait SubmissionRepository: Send + Sync {
    /// Record a project's pending submission. [`RepositoryError::Conflict`] if
    /// one is already pending for that shortcode.
    async fn create(&self, submission: &Submission) -> Result<()>;

    /// Replace state, reviewer, review time and note.
    async fn update(&self, submission: &Submission) -> Result<()>;

    async fn find(&self, id: Uuid) -> Result<Option<Submission>>;

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Option<Submission>>;

    /// The review queue: every pending submission, oldest first.
    async fn list(&self) -> Result<Vec<Submission>>;

    /// `false` if there was no submission to delete. Covers reject,
    /// depositor discard and the move to approved.
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

/// Approved records awaiting collection.
#[async_trait]
pub trait ApprovedRecordRepository: Send + Sync {
    async fn create(&self, record: &ApprovedRecord) -> Result<()>;

    /// Approved records carrying no collection timestamp, oldest first.
    ///
    /// Not what the public endpoint serves: that one applies no filter at all,
    /// so advisory collection state can never decide what is published.
    async fn list_uncollected(&self) -> Result<Vec<ApprovedRecord>>;

    /// Every approved record for a project, so the startup comparison can find
    /// the one that matches the published data.
    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Vec<ApprovedRecord>>;

    /// Every approved record, collected or not.
    ///
    /// The startup comparison's enumeration. It cannot walk the published set
    /// instead: a record whose project the published set no longer holds is
    /// invisible to such a walk. `list_uncollected` is no substitute either: a
    /// record is collected by the time its change ships, so filtering those out
    /// would hide every record about to go Online.
    async fn list_all(&self) -> Result<Vec<ApprovedRecord>>;

    /// Stamp a record as collected. Leaving it unstamped is what makes a failed
    /// collection retry on the next run.
    async fn mark_collected(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;

    /// Record the advisory outcome of one collection attempt. May run any number of times.
    ///
    /// A report carries **either** a pull request and its state **or** a failure, never both;
    /// the two write different columns. A pull request replaces the stored reference and clears
    /// the last failure. A failure records itself and **leaves the pull request reference
    /// alone**: it says nothing about that pull request, so erasing a reference an earlier
    /// report established would lose a live pull request the editor still knows about.
    ///
    /// Never touches `collected_at` — that column belongs to [`Self::mark_collected`] alone.
    /// `at` always stamps `reported_at`, in both branches: a failure is still a report.
    /// `false` when `id` names no row, which covers both an unknown id and one whose record was
    /// already discarded (see [`Self::delete`]); the caller must apply nothing else in that case.
    async fn report_collection(
        &self,
        id: Uuid,
        pull_request_url: Option<&str>,
        state: Option<PullRequestState>,
        failure: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<bool>;

    /// `false` if there was no record to delete. Used when the change goes
    /// Online and the local record is discarded.
    async fn delete(&self, id: Uuid) -> Result<bool>;
}

/// Whether a terminating review action found the submission it named.
///
/// The terminal-state guard as a return value rather than a prior check: a
/// `find` followed by a write has a window, and two reviewers deciding at once
/// is the case this exists for. Every method returning it deletes the
/// submission inside the transaction that records the round, so the delete's
/// row count decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// The submission was there. It is gone, and the round is recorded.
    Applied,
    /// Somebody finished it first. **Nothing was written**, including the round.
    AlreadyReviewed,
}

/// The transitions that end a review round and the history they leave.
///
/// Each writing method spans three tables in one transaction: a reject that
/// deleted the submission and then failed to record the round would destroy
/// the depositor's work with nothing saying it existed. The `submissions`
/// delete is also the terminal-state guard, see [`Transition`].
#[async_trait]
pub trait ReviewRoundRepository: Send + Sync {
    /// Approve: delete the submission, insert the approved record it becomes,
    /// record the round. The record's payload is what RDU approved, computed
    /// by the caller, the only layer that knows the form's appliers.
    async fn approve(&self, submission_id: Uuid, record: &ApprovedRecord, round: &ReviewRound) -> Result<Transition>;

    /// Request changes: delete the submission, write the draft it becomes,
    /// record the round. The draft carries the submitted payload, so the
    /// depositor resumes from what they sent; what RDU decided per field rides
    /// on the round.
    async fn request_changes(
        &self,
        submission_id: Uuid,
        draft: &DraftRecord,
        round: &ReviewRound,
    ) -> Result<Transition>;

    /// Reject or withdraw: delete the submission and record the round, leaving
    /// the draft and the published metadata alone. One method because the write
    /// is identical; the outcome and who may ask are the handler's.
    async fn discard(&self, submission_id: Uuid, round: &ReviewRound) -> Result<Transition>;

    /// Every round on one project, **newest first**, so the head of the list is
    /// the one the depositor's form has to show.
    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<ReviewRound>>;
}

/// Entity proposals: a person or organisation a depositor proposes to
/// create, or a change to one their project references.
#[async_trait]
pub trait EntityProposalRepository: Send + Sync {
    /// Insert a proposal for a **new** entity, allocating its `entity_id`
    /// inside the write transaction.
    ///
    /// `published_floor` is the highest id number the published store holds for
    /// that kind, which this layer cannot see. Returns the proposal as stored,
    /// with `entity_id` filled in.
    async fn create_new(&self, proposal: &EntityProposal, published_floor: u32) -> Result<EntityProposal>;

    /// Insert a proposal to change an entity that already exists. `entity_id`
    /// is the caller's; nothing is allocated.
    async fn create_change(&self, proposal: &EntityProposal) -> Result<()>;

    /// Replace a proposal's payload, stamping `updated_at`.
    async fn update_payload(&self, id: Uuid, payload: &str, at: DateTime<Utc>) -> Result<()>;

    /// Record what RDU decided about this proposal, or clear it. Leaves
    /// `status` alone — the status changes only when the round ends.
    async fn set_decision(
        &self,
        id: Uuid,
        decision: Option<ProposalDecision>,
        by: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<()>;

    async fn find(&self, id: Uuid) -> Result<Option<EntityProposal>>;

    /// One project's proposals, oldest first. Every status, so a caller can
    /// filter with `EntityProposal::is_live`.
    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<EntityProposal>>;

    /// Every **live** proposal naming this entity, whatever project it belongs
    /// to.
    async fn list_live_for_entity(&self, entity_id: &str) -> Result<Vec<EntityProposal>>;

    /// Discard a proposal the depositor abandoned. The row survives as a
    /// tombstone so its allocated id is never handed out again.
    async fn withdraw(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;
}

/// Every port at once, so one handle can serve all of them.
///
/// The nine traits are the units of dependency: a function that only looks up
/// accounts says `&dyn UserRepository`. This is for `AppState`, which every
/// handler shares and which needs all nine, since Rust has no trait object over
/// several traits. The blanket impl means neither the SQLite implementation nor
/// a test fake writes `impl Repositories`, and a method is still called through
/// the port that declares it, so nothing here widens what a call site reaches.
pub trait Repositories:
    UserRepository
    + SessionRepository
    + LoginCodeRepository
    + MailSendRepository
    + DraftRepository
    + SubmissionRepository
    + ApprovedRecordRepository
    + ReviewRoundRepository
    + EntityProposalRepository
{
}

impl<T> Repositories for T where
    T: UserRepository
        + SessionRepository
        + LoginCodeRepository
        + MailSendRepository
        + DraftRepository
        + SubmissionRepository
        + ApprovedRecordRepository
        + ReviewRoundRepository
        + EntityProposalRepository
{
}
