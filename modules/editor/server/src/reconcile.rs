//! The startup comparison: published data against local records (REQ-2.3), and
//! the Online transition it decides (REQ-2.4).
//!
//! Runs once per process, before the listener binds. Once is enough: the
//! published set is baked into the image, so the moment a deployment carrying
//! an approved change starts is the moment that change is Online.
//!
//! **It makes exactly one write — deleting a record whose data the published
//! set now carries.** That is safe by construction: the comparison authorising
//! the delete is the proof the content is already published. Everything else is
//! reported and left alone, including a *stranded* record — one already
//! collected that still differs, because its pull request merged with reviewer
//! edits or was closed unmerged, so REQ-2.4 can never fire for it. No automatic
//! resolution is correct there; force-online and force-discard are Phase 9's
//! and are RDU decisions. A record for a project dropped upstream is kept too:
//! it may be the only surviving copy of that work.
//!
//! **A failure here is not fatal**, unlike [`crate::accounts::ensure_rdu`]. The
//! cost is a stale status label; refusing to start costs the whole service.
use editor_core::draft::ProjectDraft;
use editor_core::published::PublishedProjects;
use editor_core::records::ApprovedRecord;
use editor_core::repository::{ApprovedRecordRepository, RepositoryError};
use editor_core::status::Comparison;

/// What one startup pass did, for the log line and for tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reconciliation {
    /// Records whose data the published set now carries. Deleted: the project
    /// is Online.
    pub online: usize,
    /// Records still waiting for the release that carries them (REQ-2.5).
    pub waiting: usize,
    /// Collected records that still differ — a merged-with-edits or
    /// closed-unmerged pull request. Needs an RDU decision.
    pub stranded: usize,
    /// Records for projects the published set no longer holds.
    pub removed_upstream: usize,
    /// Records whose payload could not be parsed as a draft. Left alone.
    pub unreadable: usize,
    /// Records that matched but could not be discarded, or whose row was
    /// already gone. Distinct from [`Self::unreadable`]: nothing is wrong with
    /// the record, and the next start retries it.
    pub retry_failed: usize,
}

impl Reconciliation {
    /// Whether anything needs a human. Keeps the caller from restating the
    /// rule at the log site.
    pub(crate) const fn needs_attention(self) -> bool {
        self.stranded > 0 || self.removed_upstream > 0 || self.unreadable > 0 || self.retry_failed > 0
    }
}

/// Compare every approved record against the published set and discard the ones
/// whose change has shipped.
///
/// Errors only when the records cannot be enumerated at all: a single record
/// that cannot be read or deleted is counted and stepped over, so one bad row
/// does not strand the rest.
pub(crate) async fn reconcile_published(
    db: &dyn ApprovedRecordRepository,
    published: &PublishedProjects,
) -> Result<Reconciliation, RepositoryError> {
    let records = db.list_all().await?;
    let mut summary = Reconciliation::default();

    for record in records {
        let Some(local) = parse_payload(&record) else {
            summary.unreadable += 1;
            continue;
        };

        match Comparison::classify(published.get(&record.shortcode), Some(&local)) {
            Comparison::Matches => {
                // REQ-2.4. The delete is the whole transition: with no approved
                // record and no submission, the project derives as Online.
                // The returned `bool` is load-bearing: `false` means there was
                // no row to delete, which two instances sharing one database
                // file reach on a rolling restart. Counting that as a discard
                // would overstate the summary and hide a real deletion bug
                // behind an identical success line.
                match db.delete(record.id).await {
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
            Comparison::Differs { changed } if record.collected_at.is_some() => {
                summary.stranded += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    fields = %changed.join(", "),
                    "an approved record was collected but the published data still differs: its pull request merged \
                     with edits or was closed unmerged. The project will read as waiting for release until RDU \
                     resolves it"
                );
            }
            Comparison::Differs { changed } => {
                summary.waiting += 1;
                tracing::debug!(
                    project.shortcode = %record.shortcode,
                    fields = %changed.join(", "),
                    "an approved record is waiting for the release that carries it"
                );
            }
            Comparison::RemovedUpstream => {
                summary.removed_upstream += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    "an approved record names a project the published set no longer holds; it is kept, since the \
                     record may be the only copy of this work"
                );
            }
            // None is reachable from an approved record: the local side is
            // always `Some` here, and a record carries the project's own
            // members. Counted rather than ignored — reaching one means the
            // payload is not what this pass assumes.
            Comparison::NewAndUnpublished | Comparison::Unchanged | Comparison::Absent => {
                summary.unreadable += 1;
                tracing::warn!(
                    project.shortcode = %record.shortcode,
                    "an approved record compared as neither published nor local; its payload is not a project record"
                );
            }
        }
    }

    Ok(summary)
}

/// The record's payload as a draft, or `None` when it cannot be read. Such a
/// record is left strictly alone — it is somebody's approved work.
fn parse_payload(record: &ApprovedRecord) -> Option<ProjectDraft> {
    match serde_json::from_str::<ProjectDraft>(&record.payload) {
        Ok(draft) => Some(draft),
        Err(error) => {
            tracing::error!(
                project.shortcode = %record.shortcode,
                error = %error,
                "an approved record's payload could not be read as a project; it is left untouched"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use editor_core::records::ApprovedRecord;
    use uuid::Uuid;

    use super::*;
    use crate::test_support::{open_test_db, published_corpus};

    /// A project the committed published set really holds.
    const PUBLISHED_SHORTCODE: &str = "0801d";

    /// An approved record holding `payload` for `shortcode`.
    fn record(shortcode: &str, payload: &str, collected: bool) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: payload.to_string(),
            approved_by: None,
            approved_at: Utc::now(),
            collected_at: collected.then(Utc::now),
        }
    }

    /// The published project's own data, as a record payload would hold it.
    fn published_payload(published: &PublishedProjects, shortcode: &str) -> String {
        let raw = published.get(shortcode).expect("the fixture project is published");
        serde_json::to_string(&ProjectDraft::from_raw(raw)).expect("a draft serializes")
    }

    #[tokio::test]
    async fn a_record_matching_published_data_goes_online_and_is_discarded() {
        // Success Criterion 4, and the REQ-2.3/2.4 test the issue asks for: a
        // deployment carrying an approved change reports the project Online and
        // has discarded the local record.
        let db = open_test_db("reconcile-online").await;
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        let record = record(PUBLISHED_SHORTCODE, &payload, true);
        ApprovedRecordRepository::create(&db, &record).await.expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

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
        // REQ-2.5. The record stays: it has not shipped.
        let db = open_test_db("reconcile-waiting").await;
        let published = published_corpus();
        let mut draft: ProjectDraft =
            serde_json::from_str(&published_payload(&published, PUBLISHED_SHORTCODE)).expect("parses");
        draft.set("name", serde_json::json!("An Edited Name"));
        let payload = serde_json::to_string(&draft).expect("serializes");
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, false))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

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
    async fn a_collected_record_that_still_differs_is_stranded_not_merely_waiting() {
        // The sharp case: the pull request merged *with reviewer edits*, so
        // published data can never byte-equal the record and REQ-2.4 can never
        // fire. Distinguished from waiting only by `collected_at`.
        let db = open_test_db("reconcile-stranded").await;
        let published = published_corpus();
        let mut draft: ProjectDraft =
            serde_json::from_str(&published_payload(&published, PUBLISHED_SHORTCODE)).expect("parses");
        draft.set("name", serde_json::json!("What The Reviewer Edited Away"));
        let payload = serde_json::to_string(&draft).expect("serializes");
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, true))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

        assert_eq!(summary.stranded, 1, "a collected record that still differs is stranded");
        assert_eq!(summary.waiting, 0);
        assert!(summary.needs_attention());
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&db, PUBLISHED_SHORTCODE)
                .await
                .expect("find")
                .len(),
            1,
            "a stranded record is reported, never resolved automatically"
        );
    }

    #[tokio::test]
    async fn a_record_for_a_project_dropped_upstream_is_kept_and_reported() {
        // The fourth branch. The record may be the only copy of this work.
        let db = open_test_db("reconcile-removed").await;
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&db, &record("9999", &payload, true))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

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
        ApprovedRecordRepository::create(&db, &record("9998", "{ not json", true))
            .await
            .expect("create");
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, true))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

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
        ApprovedRecordRepository::create(&db, &record(PUBLISHED_SHORTCODE, &payload, true))
            .await
            .expect("create");

        let summary = reconcile_published(&db, &published).await.expect("reconciles");

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
        // deleted, the error reaches the caller, and `main` logs it and carries
        // on rather than refusing to start — so the cost of an unreadable
        // database is a stale label, never an editor nobody can reach.
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-list-fails").await);
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&*db, &record(PUBLISHED_SHORTCODE, &payload, true))
            .await
            .expect("create");

        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults { approved_records_list_all: true, ..Faults::default() },
        );

        let result = reconcile_published(&faulty, &published).await;

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
        // `delete` answers `Ok(false)` when the row is already gone — two
        // instances sharing one database file over a rolling restart is the
        // ordinary way to reach it, so the pass sees the record in `list_all`
        // and then finds nothing to delete. Counting that as a discard would
        // overstate the summary and, worse, make a genuine deletion bug (a
        // `WHERE` that stopped matching) log exactly like a success.
        use crate::test_support::{Faults, FaultyDatabase};

        let db = std::sync::Arc::new(open_test_db("reconcile-already-gone").await);
        let published = published_corpus();
        let payload = published_payload(&published, PUBLISHED_SHORTCODE);
        ApprovedRecordRepository::create(&*db, &record(PUBLISHED_SHORTCODE, &payload, true))
            .await
            .expect("create");

        let faulty = FaultyDatabase::new(
            std::sync::Arc::clone(&db),
            Faults { approved_records_delete_missing: true, ..Faults::default() },
        );

        let summary = reconcile_published(&faulty, &published).await.expect("reconciles");

        assert_eq!(summary.online, 0, "a row that was already gone is not a discard this pass made");
        assert_eq!(summary.retry_failed, 1, "it is reported instead");
        assert!(summary.needs_attention());
    }
}
