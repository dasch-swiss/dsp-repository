//! Test support for the crates that hold the editor's store: plain-value seed helpers, row
//! readers, and [`FaultyStore`], which fails chosen calls of a real [`Database`].
//!
//! The helpers take strings and instants rather than `editor-core` records, so a crate that may
//! not name `editor-core` (the area's composition root, `areas/deposit/ADR-0004`) can seed with
//! them. Account ids are strings: they belong to identity, and this store keeps them without
//! knowing whether the account still exists.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::proposals::{EntityProposal, ProposalDecision};
use editor_core::records::{ApprovedRecord, DraftRecord, PullRequestState, ReviewRound, Submission};
use editor_core::repository::{
    ApprovedRecordRepository, DraftRepository, EntityProposalRepository, RepositoryError, Result,
    ReviewRoundRepository, SubmissionRepository, Transition,
};
use rusqlite::params;
use uuid::Uuid;

use crate::{Database, Source};

/// A fresh in-memory store for one test.
pub async fn open_for_test(label: &str) -> Database {
    Database::open(Source::memory_for_test(label), 4, std::time::Duration::from_secs(5))
        .await
        .expect("the test editor store should open")
}

impl Database {
    /// Rows in one table, so a test can assert on state no repository method returns: what a
    /// rollback left behind.
    pub async fn count_rows(&self, table: &'static str) -> i64 {
        self.read(move |conn| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)))
            .await
            .expect("counting rows should succeed")
    }

    /// Every value of one account column (`updated_by`, `submitted_by`, …) in `table`, in row
    /// order, so a test can see the stored id whether or not its account still exists.
    pub async fn account_column(&self, table: &'static str, column: &'static str) -> Vec<Option<String>> {
        self.read(move |conn| {
            let mut stmt = conn.prepare(&format!("SELECT {column} FROM {table} ORDER BY rowid"))?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            rows.collect()
        })
        .await
        .expect("reading the account column should succeed")
    }
}

fn account(id: Option<&str>) -> Option<Uuid> {
    id.map(|id| Uuid::parse_str(id).expect("an account id should be a uuid"))
}

/// Save a draft for `shortcode`.
pub async fn seed_draft(
    db: &Database,
    shortcode: &str,
    payload: &str,
    updated_by: Option<&str>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) {
    let draft = DraftRecord {
        shortcode: shortcode.to_string(),
        payload: payload.to_string(),
        updated_by: account(updated_by),
        created_at,
        updated_at,
    };
    DraftRepository::upsert(db, &draft)
        .await
        .expect("the seeded draft should store");
}

/// Insert a submission and return its id. `state` is `"submitted"`, `"in_review"` or `"approved"`.
#[allow(clippy::too_many_arguments)]
pub async fn seed_submission(
    db: &Database,
    shortcode: &str,
    payload: &str,
    state: &str,
    submitted_by: Option<&str>,
    submitted_at: DateTime<Utc>,
    reviewed_by: Option<&str>,
    reviewed_at: Option<DateTime<Utc>>,
) -> String {
    let submission = Submission {
        id: Uuid::new_v4(),
        shortcode: shortcode.to_string(),
        payload: payload.to_string(),
        state: state.parse().expect("the state should be a submission state"),
        submitted_by: account(submitted_by),
        submitted_at,
        reviewed_by: account(reviewed_by),
        reviewed_at,
        reviewer_note: None,
        review_state: None,
    };
    SubmissionRepository::create(db, &submission)
        .await
        .expect("the seeded submission should store");
    submission.id.to_string()
}

/// Record a finished review round. `outcome` is `"approved"`, `"changes_requested"`, `"rejected"`
/// or `"withdrawn"`. Written directly: every repository path to a round is a transition that also
/// moves a submission.
pub async fn seed_review_round(
    db: &Database,
    shortcode: &str,
    outcome: &str,
    note: Option<&str>,
    actor: Option<&str>,
    at: DateTime<Utc>,
) {
    let (shortcode, outcome, note, actor) = (
        shortcode.to_string(),
        outcome.to_string(),
        note.map(str::to_string),
        actor.map(str::to_string),
    );
    db.write(move |tx| {
        tx.execute(
            "INSERT INTO review_rounds (id, shortcode, submission_id, outcome, note, actor, at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                Uuid::new_v4().to_string(),
                shortcode,
                Uuid::new_v4().to_string(),
                outcome,
                note,
                actor,
                at
            ],
        )
    })
    .await
    .expect("the seeded review round should store");
}

/// Insert an entity proposal under the id `entity_id` and return the proposal's id. `kind` is
/// `"person"` or `"organization"`, `operation` `"new"` or `"change"`, `status` one of the proposal
/// statuses. Written directly, so the test chooses the entity id the allocator would otherwise
/// pick.
#[allow(clippy::too_many_arguments)]
pub async fn seed_entity_proposal(
    db: &Database,
    shortcode: &str,
    entity_id: &str,
    kind: &str,
    operation: &str,
    payload: &str,
    status: &str,
    proposed_by: Option<&str>,
    at: DateTime<Utc>,
) -> String {
    let id = Uuid::new_v4().to_string();
    let row = (
        id.clone(),
        shortcode.to_string(),
        entity_id.to_string(),
        kind.to_string(),
        operation.to_string(),
        payload.to_string(),
        status.to_string(),
        proposed_by.map(str::to_string),
    );
    db.write(move |tx| {
        let (id, shortcode, entity_id, kind, operation, payload, status, proposed_by) = row;
        tx.execute(
            "INSERT INTO entity_proposals (id, shortcode, entity_id, kind, operation, payload, status, \
             proposed_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                shortcode,
                entity_id,
                kind,
                operation,
                payload,
                status,
                proposed_by,
                at
            ],
        )
    })
    .await
    .expect("the seeded entity proposal should store");
    id
}

/// Which calls a [`FaultyStore`] must fail.
///
/// One field per injectable call, named after the port method, so the compiler checks the name and
/// a new branch adds a field rather than a string. All false by default: a test sets exactly the
/// one branch it is driving.
#[derive(Default)]
pub struct Faults {
    /// `SubmissionRepository::update`: the review surface's only write, and the branch where a
    /// page must not report a claim that did not happen.
    pub submission_update: bool,
    /// `SubmissionRepository::create`: the submit path's write, and the branch where a refused
    /// submit must leave the draft editable rather than report a submission nobody holds.
    pub submission_create: bool,
    /// Every writing transition on [`ReviewRoundRepository`] at once.
    ///
    /// One flag for three methods, because a handler reaches exactly one of them per request and
    /// they share the branch a test is driving: a terminating action that could not be written
    /// must leave the submission where it was and say so.
    pub review_transition: bool,
    /// `ReviewRoundRepository::list_for_shortcode`: the read the depositor's form makes to find
    /// out what RDU said.
    pub review_rounds_list: bool,
    /// `ApprovedRecordRepository::list_all`: the startup comparison's enumeration, and the branch
    /// where a database that cannot be read must leave every record alone rather than reconcile a
    /// set it never saw.
    pub approved_records_list_all: bool,
    /// `ApprovedRecordRepository::delete` answers `Ok(false)`: the row was already gone.
    /// Reachable for real only as a race between two instances sharing one database file, which
    /// is why it needs a seam: the startup pass must not report a discard it did not make.
    pub approved_records_delete_missing: bool,
    /// `EntityProposalRepository::list_accepted_unretired`: the retirement pass's enumeration. Its
    /// failure must not discard the record pass's summary, which is the one thing that branch
    /// exists to hold.
    pub entity_proposals_list_accepted_unretired: bool,
    /// `EntityProposalRepository::retire`: the retirement write, and the branch where a proposal
    /// that could not be retired is counted so the next start retries it rather than passing
    /// silently.
    pub entity_proposals_retire: bool,
}

/// A store that delegates every call to a real [`Database`] and fails the ones [`Faults`] names.
///
/// Delegation rather than a store built from scratch, because the branches this exists for are
/// deep in a flow. A fake answering everything from nothing would have to reimplement the store to
/// get there, and would then be proving things about the reimplementation.
pub struct FaultyStore {
    inner: Arc<Database>,
    faults: Faults,
}

impl FaultyStore {
    pub fn new(inner: Arc<Database>, faults: Faults) -> Self {
        Self { inner, faults }
    }
}

/// The error an injected failure reports.
///
/// [`RepositoryError::Backend`] specifically: it is the variant a driver failure arrives as, so the
/// handler takes the same branch it would in production. The message says it was injected, so a
/// capture of a test that fails for another reason does not read like a real storage fault.
fn injected(call: &'static str) -> RepositoryError {
    RepositoryError::Backend(format!("injected failure in {call} (test)").into())
}

#[async_trait]
impl DraftRepository for FaultyStore {
    async fn upsert(&self, draft: &DraftRecord) -> Result<()> {
        DraftRepository::upsert(&*self.inner, draft).await
    }

    async fn find(&self, shortcode: &str) -> Result<Option<DraftRecord>> {
        DraftRepository::find(&*self.inner, shortcode).await
    }

    async fn list(&self) -> Result<Vec<DraftRecord>> {
        DraftRepository::list(&*self.inner).await
    }

    async fn delete(&self, shortcode: &str) -> Result<bool> {
        DraftRepository::delete(&*self.inner, shortcode).await
    }
}

#[async_trait]
impl SubmissionRepository for FaultyStore {
    async fn create(&self, submission: &Submission) -> Result<()> {
        if self.faults.submission_create {
            return Err(injected("SubmissionRepository::create"));
        }
        SubmissionRepository::create(&*self.inner, submission).await
    }

    async fn update(&self, submission: &Submission) -> Result<()> {
        if self.faults.submission_update {
            return Err(injected("SubmissionRepository::update"));
        }
        SubmissionRepository::update(&*self.inner, submission).await
    }

    async fn find(&self, id: Uuid) -> Result<Option<Submission>> {
        SubmissionRepository::find(&*self.inner, id).await
    }

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Option<Submission>> {
        SubmissionRepository::find_by_shortcode(&*self.inner, shortcode).await
    }

    async fn list(&self) -> Result<Vec<Submission>> {
        SubmissionRepository::list(&*self.inner).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        SubmissionRepository::delete(&*self.inner, id).await
    }
}

#[async_trait]
impl ReviewRoundRepository for FaultyStore {
    async fn approve(&self, submission_id: Uuid, record: &ApprovedRecord, round: &ReviewRound) -> Result<Transition> {
        if self.faults.review_transition {
            return Err(injected("ReviewRoundRepository::approve"));
        }
        ReviewRoundRepository::approve(&*self.inner, submission_id, record, round).await
    }

    async fn request_changes(
        &self,
        submission_id: Uuid,
        draft: &DraftRecord,
        round: &ReviewRound,
    ) -> Result<Transition> {
        if self.faults.review_transition {
            return Err(injected("ReviewRoundRepository::request_changes"));
        }
        ReviewRoundRepository::request_changes(&*self.inner, submission_id, draft, round).await
    }

    async fn discard(&self, submission_id: Uuid, round: &ReviewRound) -> Result<Transition> {
        if self.faults.review_transition {
            return Err(injected("ReviewRoundRepository::discard"));
        }
        ReviewRoundRepository::discard(&*self.inner, submission_id, round).await
    }

    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<ReviewRound>> {
        if self.faults.review_rounds_list {
            return Err(injected("ReviewRoundRepository::list_for_shortcode"));
        }
        ReviewRoundRepository::list_for_shortcode(&*self.inner, shortcode).await
    }
}

#[async_trait]
impl ApprovedRecordRepository for FaultyStore {
    async fn create(&self, record: &ApprovedRecord) -> Result<()> {
        ApprovedRecordRepository::create(&*self.inner, record).await
    }

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Vec<ApprovedRecord>> {
        ApprovedRecordRepository::find_by_shortcode(&*self.inner, shortcode).await
    }

    async fn list_all(&self) -> Result<Vec<ApprovedRecord>> {
        if self.faults.approved_records_list_all {
            return Err(injected("ApprovedRecordRepository::list_all"));
        }
        ApprovedRecordRepository::list_all(&*self.inner).await
    }

    async fn mark_collected(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        ApprovedRecordRepository::mark_collected(&*self.inner, id, at).await
    }

    async fn report_collection(
        &self,
        id: Uuid,
        pull_request_url: Option<&str>,
        state: Option<PullRequestState>,
        failure: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<bool> {
        ApprovedRecordRepository::report_collection(&*self.inner, id, pull_request_url, state, failure, at).await
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        if self.faults.approved_records_delete_missing {
            return Ok(false);
        }
        ApprovedRecordRepository::delete(&*self.inner, id).await
    }
}

#[async_trait]
impl EntityProposalRepository for FaultyStore {
    async fn create_new(&self, proposal: &EntityProposal, published_floor: u32) -> Result<EntityProposal> {
        EntityProposalRepository::create_new(&*self.inner, proposal, published_floor).await
    }

    async fn create_change(&self, proposal: &EntityProposal) -> Result<()> {
        EntityProposalRepository::create_change(&*self.inner, proposal).await
    }

    async fn update_payload(&self, id: Uuid, payload: &str, at: DateTime<Utc>) -> Result<()> {
        EntityProposalRepository::update_payload(&*self.inner, id, payload, at).await
    }

    async fn set_decision(
        &self,
        id: Uuid,
        decision: Option<ProposalDecision>,
        by: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<()> {
        EntityProposalRepository::set_decision(&*self.inner, id, decision, by, at).await
    }

    async fn find(&self, id: Uuid) -> Result<Option<EntityProposal>> {
        EntityProposalRepository::find(&*self.inner, id).await
    }

    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<EntityProposal>> {
        EntityProposalRepository::list_for_shortcode(&*self.inner, shortcode).await
    }

    async fn list_live_for_entity(&self, entity_id: &str) -> Result<Vec<EntityProposal>> {
        EntityProposalRepository::list_live_for_entity(&*self.inner, entity_id).await
    }

    async fn list_accepted_unretired(&self) -> Result<Vec<EntityProposal>> {
        if self.faults.entity_proposals_list_accepted_unretired {
            return Err(injected("EntityProposalRepository::list_accepted_unretired"));
        }
        EntityProposalRepository::list_accepted_unretired(&*self.inner).await
    }

    async fn withdraw(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        EntityProposalRepository::withdraw(&*self.inner, id, at).await
    }

    async fn retire(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool> {
        if self.faults.entity_proposals_retire {
            return Err(injected("EntityProposalRepository::retire"));
        }
        EntityProposalRepository::retire(&*self.inner, id, at).await
    }
}
