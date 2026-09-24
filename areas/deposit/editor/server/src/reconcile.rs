//! The startup comparison: published data against local records, and the Online
//! transition it decides.
//!
//! Runs once per process, before the listener binds. Once is enough: the
//! published set is baked into the image, so the moment a deployment carrying
//! an approved change starts is the moment that change is Online.
//!
//! **Its writes are safe by construction.** Deleting a record whose data the
//! published set now carries: the comparison authorising the delete is the
//! proof the content is already published. Retiring an accepted entity
//! proposal once its entity appears in the published set: the id is now
//! resolvable from the published side, so the proposal's own row is no longer
//! what makes it visible. Everything else is reported and left alone — a
//! *stranded* record (its pull request merged, and the published data still
//! differs from it: reviewer edits landed instead of the depositor's own)
//! needs an RDU decision, and a record for a project dropped upstream may be
//! the only surviving copy of that work.
//!
//! **A failure here is not fatal**, unlike [`crate::accounts::ensure_rdu`]. The
//! cost is a stale status label; refusing to start costs the whole service.
use chrono::Utc;
use editor_core::agents::Agents;
use editor_core::proposals::ProposalOperation;
use editor_core::published::PublishedProjects;
use editor_core::repository::{ApprovedRecordRepository, EntityProposalRepository, RepositoryError};
use editor_core::status::{classify_record, RecordClassification};

/// What one startup pass did, for the log line and for tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reconciliation {
    /// Records whose data the published set now carries. Deleted: the project
    /// is Online.
    pub online: usize,
    /// Records still waiting for the release that carries them.
    pub waiting: usize,
    /// Records whose pull request merged but the published data still
    /// differs. Needs an RDU decision.
    pub stranded: usize,
    /// Records for projects the published set no longer holds.
    pub removed_upstream: usize,
    /// Records whose payload could not be parsed as a draft. Left alone.
    pub unreadable: usize,
    /// Records that matched but could not be discarded, or whose row was
    /// already gone. Distinct from [`Self::unreadable`]: nothing is wrong with
    /// the record, and the next start retries it.
    pub retry_failed: usize,
    /// Accepted entity proposals retired because their entity is now published.
    pub proposals_retired: usize,
    /// Accepted entity proposals whose retirement could not be written, or
    /// whose set could not be enumerated at all. The next start retries them.
    pub proposals_retire_failed: usize,
}

impl Reconciliation {
    /// Whether anything needs a human.
    pub(crate) const fn needs_attention(self) -> bool {
        self.stranded > 0
            || self.removed_upstream > 0
            || self.unreadable > 0
            || self.retry_failed > 0
            || self.proposals_retire_failed > 0
    }
}

/// Compare every approved record against the published set and discard the ones
/// whose change has shipped, then retire every accepted entity proposal whose
/// entity has appeared in the published set.
///
/// Errors only when the approved records cannot be enumerated at all: a single
/// record that cannot be read or deleted is counted and stepped over, so one
/// bad row does not strand the rest. A failure to enumerate entity proposals is
/// never one of these errors — see [`retire_accepted_proposals`].
pub(crate) async fn reconcile_published(
    records_repo: &dyn ApprovedRecordRepository,
    proposals: &dyn EntityProposalRepository,
    published: &PublishedProjects,
    agents: &Agents,
) -> Result<Reconciliation, RepositoryError> {
    let records = records_repo.list_all().await?;
    let mut summary = Reconciliation::default();

    for record in records {
        match classify_record(&record, published.get(&record.shortcode)) {
            RecordClassification::Published => {
                // The delete is the whole transition: with no approved record
                // and no submission, the project derives as Online. The returned
                // `bool` is load-bearing — `false` means there was no row, which
                // two instances sharing one database file reach on a rolling
                // restart, and counting it as a discard would hide a real
                // deletion bug behind an identical success line.
                match records_repo.delete(record.id).await {
                    Ok(true) => {
                        summary.online += 1;
                        tracing::info!(
                            project.shortcode = %record.shortcode,
                            "the published set carries this record; the project is Online and the local record is discarded"
                        );
                    }
                    Ok(false) => {
                        summary.retry_failed += 1;
                        tracing::warn!(
                            project.shortcode = %record.shortcode,
                            "a record that matched the published set was already gone when this pass tried to \
                             discard it; the project is Online either way"
                        );
                    }
                    Err(error) => {
                        // The record is still there, so the next start retries.
                        summary.retry_failed += 1;
                        tracing::error!(
                            project.shortcode = %record.shortcode,
                            error = %error,
                            "could not discard a record whose change is published; it will be retried on the next start"
                        );
                    }
                }
            }
            RecordClassification::Stranded { changed } => {
                summary.stranded += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    fields = %changed.join(", "),
                    "an approved record's pull request merged but the published data still differs; RDU must \
                     resolve which version stands"
                );
            }
            RecordClassification::AwaitingCollection { changed }
            | RecordClassification::PullRequestOpen { changed }
            | RecordClassification::PullRequestClosed { changed }
            | RecordClassification::CollectionFailed { changed, .. } => {
                summary.waiting += 1;
                tracing::debug!(
                    project.shortcode = %record.shortcode,
                    fields = %changed.join(", "),
                    "an approved record is waiting for the release that carries it"
                );
            }
            RecordClassification::RemovedUpstream => {
                summary.removed_upstream += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    "an approved record names a project the published set no longer holds; it is kept, since the \
                     record may be the only copy of this work"
                );
            }
            RecordClassification::Unreadable { problem } => {
                summary.unreadable += 1;
                tracing::error!(
                    project.shortcode = %record.shortcode,
                    error = %problem,
                    "an approved record's payload could not be read as a project; it is left untouched"
                );
            }
            // None is reachable from an approved record: the local side is
            // always `Some` here, and a record carries the project's own
            // members. Counted rather than ignored — reaching one means the
            // payload is not what this pass assumes.
            RecordClassification::Anomalous => {
                summary.unreadable += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    "an approved record compared as neither published nor local; its payload is not a project record"
                );
            }
        }
    }

    retire_accepted_proposals(proposals, agents, &mut summary).await;

    Ok(summary)
}

/// Retire every accepted entity proposal whose entity the published set now
/// carries.
///
/// A failure to enumerate the proposals does not propagate: the record pass
/// above has already built `summary`, and returning `Err` here would discard
/// it. Logged and counted instead — do not replace this with `?`.
async fn retire_accepted_proposals(
    proposals: &dyn EntityProposalRepository,
    agents: &Agents,
    summary: &mut Reconciliation,
) {
    let accepted = match proposals.list_accepted_unretired().await {
        Ok(accepted) => accepted,
        Err(error) => {
            summary.proposals_retire_failed += 1;
            tracing::error!(
                error = %error,
                "could not enumerate accepted entity proposals; retirement is retried on the next start"
            );
            return;
        }
    };

    for proposal in accepted {
        // `retire` enforces the `New`-only rule itself; skipping here just
        // saves the write.
        if proposal.operation != ProposalOperation::New || !agents.has(&proposal.entity_id) {
            continue;
        }
        match proposals.retire(proposal.id, Utc::now()).await {
            // `false` means another instance retired it between the enumeration
            // and this write. Counting it would report a retirement this pass
            // did not make, the same reason the record loop above splits its
            // own `delete` result.
            Ok(true) => {
                summary.proposals_retired += 1;
                tracing::debug!(
                    project.shortcode = %proposal.shortcode,
                    entity_id = %proposal.entity_id,
                    "an accepted entity proposal's entity is now published; retired"
                );
            }
            Ok(false) => tracing::debug!(
                project.shortcode = %proposal.shortcode,
                entity_id = %proposal.entity_id,
                "an accepted entity proposal was already retired when this pass reached it"
            ),
            Err(error) => {
                summary.proposals_retire_failed += 1;
                tracing::error!(
                    project.shortcode = %proposal.shortcode,
                    entity_id = %proposal.entity_id,
                    error = %error,
                    "could not retire an accepted entity proposal whose entity is now published; it will be retried \
                     on the next start"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use editor_core::draft::ProjectDraft;
    use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
    use editor_core::records::{ApprovedRecord, PullRequestState};
    use uuid::Uuid;

    use super::*;
    use crate::test_support::{agents_corpus, open_test_db, published_corpus};

    /// A project the committed published set really holds.
    const PUBLISHED_SHORTCODE: &str = "0801d";

    fn record(shortcode: &str, payload: &str, pull_request_state: Option<PullRequestState>) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: payload.to_string(),
            approved_by: None,
            approved_at: Utc::now(),
            collected_at: Some(Utc::now()),
            reported_at: None,
            pull_request_url: None,
            pull_request_state,
            last_failure: None,
        }
    }

    fn published_payload(published: &PublishedProjects, shortcode: &str) -> String {
        let raw = published.get(shortcode).expect("the fixture project is published");
        serde_json::to_string(&ProjectDraft::from_raw(raw)).expect("a draft serializes")
    }

    #[tokio::test]
    async fn a_record_matching_published_data_goes_online_and_is_discarded() {
        // A deployment carrying an approved change reports the project Online
        // and has discarded the local record.
        let db = open_test_db("reconcile-online").await;
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        let record = record(PUBLISHED_SHORTCODE, &payload, None);
        ApprovedRecordRepository::create(&db, &record).await.expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.online, 1);
        assert_eq!(summary.stranded, 0);
        assert!(
            ApprovedRecordRepository::find_by_shortcode(&db, PUBLISHED_SHORTCODE)
                .await
                .expect("find")
                .is_empty(),
            "the local record must be discarded once its change is published"
        );
    }

    #[tokio::test]
    async fn a_record_that_differs_is_left_waiting_for_its_release() {
        // The record stays: it has not shipped.
        let db = open_test_db("reconcile-waiting").await;
        let published = published_corpus();
        let mut draft: ProjectDraft =
            serde_json::from_str(&published_payload(&published, PUBLISHED_SHORTCODE)).expect("parses");
        draft.set("name", serde_json::json!("An Edited Name"));
        let payload = serde_json::to_string(&draft).expect("serializes");
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.waiting, 1);
        assert_eq!(summary.online, 0);
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&db, PUBLISHED_SHORTCODE)
                .await
                .expect("find")
                .len(),
            1,
            "a record that has not shipped must survive the pass"
        );
    }

    #[tokio::test]
    async fn differing_published_data_is_stranded_only_when_the_pull_request_merged() {
        // Merged is the one `Differs` case a human must resolve: reviewer edits
        // landed instead of the depositor's own, so published data can never
        // byte-equal the record again on its own. Open, closed, and
        // never-reported are all a normal wait for the release that carries
        // the record.
        let cases = [
            (Some(PullRequestState::Merged), true),
            (Some(PullRequestState::Open), false),
            (Some(PullRequestState::Closed), false),
            (None, false),
        ];

        for (state, expect_stranded) in cases {
            let db = open_test_db(&format!("reconcile-differs-{state:?}")).await;
            let published = published_corpus();
            let mut draft: ProjectDraft =
                serde_json::from_str(&published_payload(&published, PUBLISHED_SHORTCODE)).expect("parses");
            draft.set("name", serde_json::json!("What The Reviewer Edited Away"));
            let payload = serde_json::to_string(&draft).expect("serializes");
            ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, state))
                .await
                .expect("create");

            let summary = reconcile_published(&db, &db, &published, &agents_corpus())
                .await
                .expect("reconciles");

            assert_eq!(summary.stranded, usize::from(expect_stranded), "state {state:?}");
            assert_eq!(summary.waiting, usize::from(!expect_stranded), "state {state:?}");
            assert_eq!(summary.needs_attention(), expect_stranded, "state {state:?}");
            assert_eq!(
                ApprovedRecordRepository::find_by_shortcode(&db, PUBLISHED_SHORTCODE)
                    .await
                    .expect("find")
                    .len(),
                1,
                "a differing record is reported, never resolved automatically, for state {state:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_record_for_a_project_dropped_upstream_is_kept_and_reported() {
        // The fourth branch. The record may be the only copy of this work.
        let db = open_test_db("reconcile-removed").await;
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&db, &record("9999", &payload, None))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.removed_upstream, 1);
        assert_eq!(summary.online, 0);
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&db, "9999")
                .await
                .expect("find")
                .len(),
            1,
            "an upstream deletion must never destroy the local record"
        );
    }

    #[tokio::test]
    async fn an_unreadable_payload_is_counted_and_stepped_over() {
        // One bad row must not strand the rest.
        let db = open_test_db("reconcile-unreadable").await;
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&db, &record("9998", "{ not json", None))
            .await
            .expect("create");
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.unreadable, 1);
        assert_eq!(summary.online, 1, "the readable record is still reconciled");
    }

    #[tokio::test]
    async fn an_empty_published_set_discards_nothing() {
        // A deployment with no data directory loads no projects. Treating that
        // as "nothing is published" would be right; treating it as "everything
        // was removed upstream" must not delete anything, and this pins that
        // the pass has no path that deletes on absence.
        let db = open_test_db("reconcile-empty-set").await;
        let published = PublishedProjects::default();
        let payload = published_payload(&published_corpus(), PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(
            summary.online, 0,
            "an absent published set must never be read as 'already published'"
        );
        assert_eq!(summary.removed_upstream, 1);
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&db, PUBLISHED_SHORTCODE)
                .await
                .expect("find")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn a_database_that_cannot_be_enumerated_reconciles_nothing_and_reports_it() {
        // The module's central claim: a failure here is not fatal. Nothing is
        // deleted, the error reaches the caller, and `serve()` logs it and carries
        // on rather than refusing to start — so the cost of an unreadable
        // database is a stale label, never an editor nobody can reach.
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-list-fails").await);
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&*db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");

        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults { approved_records_list_all: true, ..Faults::default() },
        );

        let result = reconcile_published(&faulty, &faulty, &published, &agents_corpus()).await;

        assert!(
            result.is_err(),
            "an unreadable record set must surface as an error, not an empty pass"
        );
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&*db, PUBLISHED_SHORTCODE)
                .await
                .expect("find")
                .len(),
            1,
            "a pass that could not read the records must not have deleted any"
        );
    }

    #[tokio::test]
    async fn a_record_already_gone_is_not_counted_as_a_discard_this_pass_made() {
        // `delete` answers `Ok(false)` when the row is already gone, which two
        // instances sharing one database file reach on a rolling restart.
        // Counting it as a discard would make a genuine deletion bug log exactly
        // like a success.
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-already-gone").await);
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&*db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");

        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults { approved_records_delete_missing: true, ..Faults::default() },
        );

        let summary = reconcile_published(&faulty, &faulty, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.online, 0, "a row that was already gone is not a discard this pass made");
        assert_eq!(summary.retry_failed, 1, "it is reported instead");
        assert!(summary.needs_attention());
    }

    /// A `change` proposal so the fixture never collides with
    /// `entity_proposals_allocated_id`, which is unique on `entity_id` for
    /// `operation = 'new'` regardless of status.
    fn entity_proposal(shortcode: &str, entity_id: &str, status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            entity_id: entity_id.to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::Change,
            payload: r#"{"name":"placeholder"}"#.to_string(),
            status,
            decision: None,
            proposed_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            decided_by: None,
            decided_at: None,
            retired_at: None,
        }
    }

    /// The same fixture as a `new` proposal. `create_new` allocates the
    /// `entity_id` itself, so the placeholder here is never stored.
    fn new_proposal(shortcode: &str, status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            operation: ProposalOperation::New,
            ..entity_proposal(shortcode, "unallocated", status)
        }
    }

    #[tokio::test]
    async fn an_accepted_proposal_is_retired_only_once_its_entity_is_published() {
        // Allocation decides the two cases: below the published floor the
        // allocated id is one the corpus already carries, above it the entity
        // does not exist yet. That is the real difference between a proposal
        // whose entity has shipped and one whose has not.
        let db = open_test_db("reconcile-retire-accepted").await;
        let published = published_corpus();
        let agents = agents_corpus();
        let floor = agents.highest_id_number(ProposalKind::Person);
        let shipped = EntityProposalRepository::create_new(&db, &new_proposal("0801", ProposalStatus::Accepted), 0)
            .await
            .expect("create");
        let unshipped =
            EntityProposalRepository::create_new(&db, &new_proposal("0803", ProposalStatus::Accepted), floor)
                .await
                .expect("create");
        assert!(agents.has(&shipped.entity_id), "the low allocation must be published");
        assert!(!agents.has(&unshipped.entity_id), "the allocation above the floor must not be");

        let summary = reconcile_published(&db, &db, &published, &agents).await.expect("reconciles");

        assert_eq!(summary.proposals_retired, 1);
        let retired = EntityProposalRepository::find(&db, shipped.id)
            .await
            .expect("find")
            .expect("row");
        assert!(retired.retired_at.is_some(), "the published entity's proposal must be retired");
        let survivor = EntityProposalRepository::find(&db, unshipped.id)
            .await
            .expect("find")
            .expect("row");
        assert!(
            survivor.retired_at.is_none(),
            "a proposal for an entity that is not yet published must survive the pass"
        );
    }

    #[tokio::test]
    async fn a_retired_proposal_is_not_retired_again_on_the_next_pass() {
        let db = open_test_db("reconcile-retire-idempotent").await;
        let published = published_corpus();
        let agents = agents_corpus();
        let proposal = EntityProposalRepository::create_new(&db, &new_proposal("0801", ProposalStatus::Accepted), 0)
            .await
            .expect("create");

        let first = reconcile_published(&db, &db, &published, &agents).await.expect("reconciles");
        assert_eq!(first.proposals_retired, 1);

        let second = reconcile_published(&db, &db, &published, &agents).await.expect("reconciles");
        assert_eq!(
            second.proposals_retired, 0,
            "the guard makes retirement idempotent, not decorative"
        );
    }

    #[tokio::test]
    async fn an_accepted_change_proposal_is_never_retired_by_this_pass() {
        // Regression: retiring here loses the edit permanently, and every
        // other test in this module still passes when it does.
        let db = open_test_db("reconcile-retire-change").await;
        let published = published_corpus();
        let agents = agents_corpus();
        let proposal = entity_proposal("0801", "person-001", ProposalStatus::Accepted);
        assert!(agents.has(&proposal.entity_id), "the fixture entity must be published");
        EntityProposalRepository::create_change(&db, &proposal).await.expect("create");

        let summary = reconcile_published(&db, &db, &published, &agents).await.expect("reconciles");

        assert_eq!(summary.proposals_retired, 0);
        let survivor = EntityProposalRepository::find(&db, proposal.id)
            .await
            .expect("find")
            .expect("row");
        assert!(
            survivor.retired_at.is_none(),
            "an accepted change proposal must survive until its edit has been collected"
        );
    }

    #[tokio::test]
    async fn a_retirement_pass_that_cannot_enumerate_still_returns_the_record_summary() {
        // The record half has already run. Propagating here would trade a stale
        // label for a summary the caller never sees, which is the opposite of
        // this module's stance on a non-fatal failure.
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-retire-enumerate-fails").await);
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&*db, &record(PUBLISHED_SHORTCODE, &payload, None))
            .await
            .expect("create");
        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults {
                entity_proposals_list_accepted_unretired: true,
                ..Faults::default()
            },
        );

        let summary = reconcile_published(&faulty, &faulty, &published, &agents_corpus())
            .await
            .expect("the record pass still reports");

        assert_eq!(summary.online, 1, "the record pass's own result must survive");
        assert_eq!(summary.proposals_retire_failed, 1);
        assert!(summary.needs_attention());
    }

    #[tokio::test]
    async fn a_proposal_that_cannot_be_retired_is_counted_for_the_next_start() {
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-retire-write-fails").await);
        let published = published_corpus();
        EntityProposalRepository::create_new(&*db, &new_proposal("0801", ProposalStatus::Accepted), 0)
            .await
            .expect("create");
        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults { entity_proposals_retire: true, ..Faults::default() },
        );

        let summary = reconcile_published(&faulty, &faulty, &published, &agents_corpus())
            .await
            .expect("reconciles");

        assert_eq!(summary.proposals_retired, 0);
        assert_eq!(summary.proposals_retire_failed, 1);
        assert!(summary.needs_attention(), "an unretired proposal must reach RDU");
    }

    #[tokio::test]
    async fn only_an_accepted_proposal_is_retired() {
        let db = open_test_db("reconcile-retire-status-scope").await;
        let published = published_corpus();
        let agents = agents_corpus();
        for (shortcode, status) in [
            ("0801", ProposalStatus::Draft),
            ("0803", ProposalStatus::Submitted),
            ("0805", ProposalStatus::Rejected),
            ("0807", ProposalStatus::Withdrawn),
        ] {
            // `new`, so only the status stops them: a `change` would be
            // skipped whatever its status and the assertion would hold vacuously.
            EntityProposalRepository::create_new(&db, &new_proposal(shortcode, status), 0)
                .await
                .expect("create");
        }

        let summary = reconcile_published(&db, &db, &published, &agents).await.expect("reconciles");

        assert_eq!(
            summary.proposals_retired, 0,
            "a proposal that is not accepted must never acquire a retirement timestamp"
        );
    }
}
