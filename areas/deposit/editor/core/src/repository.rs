//! The persistence ports, one trait per aggregate.
//!
//! Framework-free: the traits name domain records and [`RepositoryError`], never
//! a `rusqlite` type. Every method is `async` and boxed via `async_trait`
//! rather than a bare `async fn` in a trait: the futures have to be `Send` to be
//! awaited inside an Axum handler, and the traits have to stay dyn-compatible
//! for the `Arc<dyn Repositories>` that `editor-server`'s `AppState` holds
//! (this crate's [`EditorRepositories`] plus identity's), which is how a test
//! puts a fake behind it and makes a chosen call fail.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::proposals::{EntityProposal, ProposalDecision};
use crate::records::{ApprovedRecord, DraftRecord, PullRequestState, ReviewRound, Submission};

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

    /// Every approved record for a project, so the startup comparison can find
    /// the one that matches the published data.
    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Vec<ApprovedRecord>>;

    /// Every approved record, collected or not, oldest approval first.
    ///
    /// What the public endpoint serves, unfiltered, so advisory collection state
    /// can never decide what is published. Also the startup comparison's
    /// enumeration. It cannot walk the published set instead: a record whose
    /// project the published set no longer holds is invisible to such a walk.
    /// Filtering out collected records is no substitute either: a record is
    /// collected by the time its change ships, so that would hide every record
    /// about to go Online.
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

    /// Every accepted proposal not yet retired, across all projects.
    ///
    /// Startup-only: the reconciliation pass calls this once per process start
    /// to retire the ones whose entity has since appeared in the published set.
    /// Not indexed for that reason — an index here would cost a write on every
    /// proposal write for a read that runs once per start.
    async fn list_accepted_unretired(&self) -> Result<Vec<EntityProposal>>;

    /// Discard a proposal the depositor abandoned. The row survives as a
    /// tombstone so its allocated id is never handed out again.
    async fn withdraw(&self, id: Uuid, at: DateTime<Utc>) -> Result<()>;

    /// Stamp `retired_at` on an accepted `new`-entity proposal whose entity the
    /// startup reconciliation pass has observed in the published set.
    ///
    /// **Refuses a `change` proposal.** Such a proposal names an entity the
    /// published set carries by definition, so membership cannot tell a shipped
    /// edit from an unshipped one, and retiring it drops the edit from the next
    /// approved record's payload for good. The operation is part of this
    /// method's own predicate rather than a check at its one call site, so a
    /// second caller cannot omit it.
    ///
    /// `false` when the row was already retired, when it is a `change`, or when
    /// `id` names no row. A caller counting retirements distinguishes it from
    /// `true`; one merely ensuring the state does not have to.
    async fn retire(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool>;
}

/// The five editor ports at once, so one handle can serve all of them.
///
/// The five traits are the units of dependency: a function that only reads
/// drafts says `&dyn DraftRepository`. This is for a composition root that holds
/// one handle for all of them, since Rust has no trait object over several
/// traits. The blanket impl means neither the SQLite implementation nor a test
/// fake writes `impl EditorRepositories`, and a method is still called through
/// the port that declares it, so nothing here widens what a call site reaches.
pub trait EditorRepositories:
    DraftRepository + SubmissionRepository + ApprovedRecordRepository + ReviewRoundRepository + EntityProposalRepository
{
}

impl<T> EditorRepositories for T where
    T: DraftRepository
        + SubmissionRepository
        + ApprovedRecordRepository
        + ReviewRoundRepository
        + EntityProposalRepository
{
}
