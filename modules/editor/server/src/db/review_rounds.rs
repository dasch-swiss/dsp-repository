//! [`ReviewRoundRepository`] against SQLite: the three transitions that end a
//! review round, and the history they leave.
//!
//! Each transition spans three tables and is therefore one `write` closure
//! rather than a caller-side sequence. Two properties come out of that, and
//! both are the point of the module:
//!
//! - **The delete is the terminal-state guard.** `DELETE FROM submissions WHERE id = ?` returns how
//!   many rows it removed, and exactly one of two concurrent calls can see the row — so zero means
//!   somebody finished it first and the answer is [`Transition::AlreadyReviewed`]. Checking with a
//!   `find` first would leave a window between the read and the write, which is the case this
//!   exists for: a reject landing after an approve destroys a record the collection endpoint has
//!   already served.
//! - **Nothing is written on a refusal.** The guard returns before the round is inserted, so an
//!   already-reviewed submission leaves no second round and no second `approved_records` row.
//!   Anything failing later rolls the delete back with it, so a round that cannot be written cannot
//!   destroy the submission either.

use async_trait::async_trait;
use editor_core::records::{ApprovedRecord, DraftRecord, ReviewOutcome, ReviewRound};
use editor_core::repository::{Result, ReviewRoundRepository, Transition};
use rusqlite::{params, Row, Transaction};
use uuid::Uuid;

use super::mapping::{optional_uuid_column, parsed_column, uuid_column};
use super::Database;

const ENTITY: &str = "review round";

const SELECT: &str = "SELECT id, shortcode, submission_id, outcome, note, review_state, actor, at FROM review_rounds";

fn map_row(row: &Row<'_>) -> rusqlite::Result<ReviewRound> {
    Ok(ReviewRound {
        id: uuid_column(row, 0)?,
        shortcode: row.get(1)?,
        submission_id: uuid_column(row, 2)?,
        outcome: parsed_column::<ReviewOutcome>(row, 3)?,
        note: row.get(4)?,
        review_state: row.get(5)?,
        actor: optional_uuid_column(row, 6)?,
        at: row.get(7)?,
    })
}

/// Delete the submission the round ends, answering whether it was there.
///
/// The whole terminal-state guard, in one place so the three transitions cannot
/// implement it differently.
fn claim(tx: &Transaction<'_>, submission_id: Uuid) -> rusqlite::Result<Transition> {
    let deleted = tx.execute("DELETE FROM submissions WHERE id = ?1", params![submission_id.to_string()])?;
    if deleted == 0 {
        return Ok(Transition::AlreadyReviewed);
    }
    Ok(Transition::Applied)
}

fn insert_round(tx: &Transaction<'_>, round: &ReviewRound) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO review_rounds (id, shortcode, submission_id, outcome, note, review_state, actor, at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            round.id.to_string(),
            round.shortcode,
            round.submission_id.to_string(),
            round.outcome.as_str(),
            round.note,
            round.review_state,
            round.actor.map(|id| id.to_string()),
            round.at,
        ],
    )?;
    Ok(())
}

#[async_trait]
impl ReviewRoundRepository for Database {
    async fn approve(&self, submission_id: Uuid, record: &ApprovedRecord, round: &ReviewRound) -> Result<Transition> {
        let record = record.clone();
        let round = round.clone();
        self.write(move |tx| {
            if claim(tx, submission_id)? == Transition::AlreadyReviewed {
                return Ok(Transition::AlreadyReviewed);
            }
            tx.execute(
                "INSERT INTO approved_records (id, shortcode, payload, approved_by, approved_at, collected_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.id.to_string(),
                    record.shortcode,
                    record.payload,
                    record.approved_by.map(|id| id.to_string()),
                    record.approved_at,
                    record.collected_at,
                ],
            )?;
            insert_round(tx, &round)?;
            Ok(Transition::Applied)
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn request_changes(
        &self,
        submission_id: Uuid,
        draft: &DraftRecord,
        round: &ReviewRound,
    ) -> Result<Transition> {
        let draft = draft.clone();
        let round = round.clone();
        self.write(move |tx| {
            if claim(tx, submission_id)? == Transition::AlreadyReviewed {
                return Ok(Transition::AlreadyReviewed);
            }
            // The same upsert `DraftRepository::upsert` performs. Inline rather
            // than shared because it has to run on *this* transaction: a second
            // `write` would be a second transaction, which is the atomicity the
            // guard above depends on.
            tx.execute(
                "INSERT INTO drafts (shortcode, payload, updated_by, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT (shortcode) DO UPDATE SET payload = ?2, updated_by = ?3, updated_at = ?5",
                params![
                    draft.shortcode,
                    draft.payload,
                    draft.updated_by.map(|id| id.to_string()),
                    draft.created_at,
                    draft.updated_at,
                ],
            )?;
            insert_round(tx, &round)?;
            Ok(Transition::Applied)
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn discard(&self, submission_id: Uuid, round: &ReviewRound) -> Result<Transition> {
        let round = round.clone();
        self.write(move |tx| {
            if claim(tx, submission_id)? == Transition::AlreadyReviewed {
                return Ok(Transition::AlreadyReviewed);
            }
            insert_round(tx, &round)?;
            Ok(Transition::Applied)
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn list_for_shortcode(&self, shortcode: &str) -> Result<Vec<ReviewRound>> {
        let shortcode = shortcode.to_string();
        Ok(self
            .read(move |conn| {
                // Newest first: the head is the round the depositor's form shows.
                // `id` breaks a tie so two rounds recorded in the same instant
                // have a stable order — reachable, because the timestamp has
                // millisecond resolution and a resubmit-and-review cycle is not
                // bounded below.
                let mut stmt = conn.prepare(&format!("{SELECT} WHERE shortcode = ?1 ORDER BY at DESC, id DESC"))?;
                let rows = stmt.query_map(params![shortcode], map_row)?;
                rows.collect()
            })
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use editor_core::records::{Role, Submission, SubmissionState, User};
    use editor_core::repository::{ApprovedRecordRepository, DraftRepository, SubmissionRepository, UserRepository};

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 8, hour, 0, 0).unwrap()
    }

    async fn a_user(db: &Database, email: &str, role: Role) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: email.to_string(),
            name: "A".to_string(),
            role,
            shortcodes: vec![],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    /// A pending submission on `0801`, carrying a review state so the snapshot
    /// the round takes is not vacuously `None`.
    async fn a_submission(db: &Database, author: Option<Uuid>) -> Submission {
        let submission = Submission {
            id: Uuid::new_v4(),
            shortcode: "0801".to_string(),
            payload: r#"{"name":"submitted"}"#.to_string(),
            state: SubmissionState::InReview,
            submitted_by: author,
            submitted_at: at(11),
            reviewed_by: None,
            reviewed_at: Some(at(12)),
            reviewer_note: None,
            review_state: Some(r#"{"name":{"decision":"accept"}}"#.to_string()),
        };
        SubmissionRepository::create(db, &submission).await.unwrap();
        submission
    }

    fn a_round(submission: &Submission, outcome: ReviewOutcome, actor: Option<Uuid>) -> ReviewRound {
        ReviewRound {
            id: Uuid::new_v4(),
            shortcode: submission.shortcode.clone(),
            submission_id: submission.id,
            outcome,
            note: Some("Please add a German description.".to_string()),
            review_state: submission.review_state.clone(),
            actor,
            at: at(13),
        }
    }

    fn a_record(submission: &Submission, approver: Option<Uuid>) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: submission.shortcode.clone(),
            payload: r#"{"name":"as approved"}"#.to_string(),
            approved_by: approver,
            approved_at: at(13),
            collected_at: None,
        }
    }

    fn a_draft(submission: &Submission) -> DraftRecord {
        DraftRecord {
            shortcode: submission.shortcode.clone(),
            payload: submission.payload.clone(),
            updated_by: submission.submitted_by,
            created_at: at(10),
            updated_at: at(13),
        }
    }

    #[tokio::test]
    async fn test_approve_moves_the_submission_into_an_approved_record_and_records_the_round() {
        // REQ-4.4. Three writes in one transaction: the submission goes, the
        // record it becomes appears, and the round says who approved what.
        let db = test_db("rounds-approve").await;
        let reviewer = a_user(&db, "rdu@x.test", Role::Rdu).await;
        let submission = a_submission(&db, None).await;
        let record = a_record(&submission, Some(reviewer));
        let round = a_round(&submission, ReviewOutcome::Approved, Some(reviewer));

        assert_eq!(
            ReviewRoundRepository::approve(&db, submission.id, &record, &round)
                .await
                .unwrap(),
            Transition::Applied
        );

        assert_eq!(count(&db, "submissions").await, 0);
        assert_eq!(
            ApprovedRecordRepository::find_by_shortcode(&db, "0801").await.unwrap(),
            vec![record]
        );
        assert_eq!(db.list_for_shortcode("0801").await.unwrap(), vec![round]);
    }

    #[tokio::test]
    async fn test_approving_twice_is_already_reviewed_and_writes_nothing() {
        // The terminal-state guard. Two reviewers hold the page open, both
        // approve; the second must not produce a second record on its way to a
        // second pull request, nor a second round claiming the same submission
        // was approved twice.
        let db = test_db("rounds-approve-twice").await;
        let submission = a_submission(&db, None).await;
        ReviewRoundRepository::approve(
            &db,
            submission.id,
            &a_record(&submission, None),
            &a_round(&submission, ReviewOutcome::Approved, None),
        )
        .await
        .unwrap();

        assert_eq!(
            ReviewRoundRepository::approve(
                &db,
                submission.id,
                &a_record(&submission, None),
                &a_round(&submission, ReviewOutcome::Approved, None),
            )
            .await
            .unwrap(),
            Transition::AlreadyReviewed
        );

        assert_eq!(count(&db, "approved_records").await, 1);
        assert_eq!(count(&db, "review_rounds").await, 1);
    }

    #[tokio::test]
    async fn test_request_changes_writes_the_draft_and_records_the_round() {
        // REQ-4.5. The draft carries the *submitted* payload, so the depositor
        // resumes from what they sent rather than from whatever the draft held
        // when they sent it; what RDU decided per field rides on the round.
        let db = test_db("rounds-request-changes").await;
        let author = a_user(&db, "a@x.test", Role::Depositor).await;
        let reviewer = a_user(&db, "rdu@x.test", Role::Rdu).await;
        let submission = a_submission(&db, Some(author)).await;
        let draft = a_draft(&submission);
        let round = a_round(&submission, ReviewOutcome::ChangesRequested, Some(reviewer));

        assert_eq!(
            ReviewRoundRepository::request_changes(&db, submission.id, &draft, &round)
                .await
                .unwrap(),
            Transition::Applied
        );

        assert_eq!(count(&db, "submissions").await, 0);
        // Struct equality, not just the payload: this upsert is a second copy
        // of `DraftRepository::upsert`'s — it has to run on *this* transaction,
        // so it cannot be shared — and a column added to one and not the other
        // would leave a returned draft silently missing it. A new
        // `DraftRecord` field cannot pass here without being accounted for.
        assert_eq!(
            DraftRepository::find(&db, "0801").await.unwrap(),
            Some(draft.clone()),
            "the returned draft is exactly what the shared upsert would have written"
        );
        assert_eq!(draft.payload, submission.payload, "and it carries what was submitted");
        let rounds = db.list_for_shortcode("0801").await.unwrap();
        assert_eq!(rounds, vec![round]);
        assert_eq!(
            rounds[0].review_state, submission.review_state,
            "the per-field state survives the return"
        );
    }

    #[tokio::test]
    async fn test_request_changes_after_an_approve_writes_neither_draft_nor_round() {
        // The second shape of the race the guard exists for. Unguarded this
        // resurrects an approved project as an editable draft: the record is
        // already on its way to a pull request, and the depositor is handed a
        // form for work that has left their hands.
        let db = test_db("rounds-request-changes-race").await;
        let submission = a_submission(&db, None).await;
        ReviewRoundRepository::approve(
            &db,
            submission.id,
            &a_record(&submission, None),
            &a_round(&submission, ReviewOutcome::Approved, None),
        )
        .await
        .unwrap();

        assert_eq!(
            ReviewRoundRepository::request_changes(
                &db,
                submission.id,
                &a_draft(&submission),
                &a_round(&submission, ReviewOutcome::ChangesRequested, None),
            )
            .await
            .unwrap(),
            Transition::AlreadyReviewed
        );

        assert_eq!(
            DraftRepository::find(&db, "0801").await.unwrap(),
            None,
            "no draft was resurrected"
        );
        assert_eq!(count(&db, "review_rounds").await, 1);
    }

    #[tokio::test]
    async fn test_discard_leaves_the_draft_alone() {
        // REQ-4.6 and REQ-4.7 delete the submission; REQ-1.13 preserves drafts.
        // Reject must not destroy the depositor's work, and a withdrawal is the
        // depositor taking it back to keep editing.
        for outcome in [ReviewOutcome::Rejected, ReviewOutcome::Withdrawn] {
            let db = test_db(&format!("rounds-discard-{outcome}")).await;
            let submission = a_submission(&db, None).await;
            let draft = a_draft(&submission);
            DraftRepository::upsert(&db, &draft).await.unwrap();

            assert_eq!(
                ReviewRoundRepository::discard(&db, submission.id, &a_round(&submission, outcome, None))
                    .await
                    .unwrap(),
                Transition::Applied,
                "{outcome}"
            );

            assert_eq!(count(&db, "submissions").await, 0, "{outcome}");
            assert_eq!(DraftRepository::find(&db, "0801").await.unwrap(), Some(draft), "{outcome}");
            assert_eq!(count(&db, "approved_records").await, 0, "{outcome}");
        }
    }

    #[tokio::test]
    async fn test_discarding_an_already_reviewed_submission_records_nothing() {
        let db = test_db("rounds-discard-twice").await;
        let submission = a_submission(&db, None).await;
        ReviewRoundRepository::discard(&db, submission.id, &a_round(&submission, ReviewOutcome::Rejected, None))
            .await
            .unwrap();

        assert_eq!(
            ReviewRoundRepository::discard(&db, submission.id, &a_round(&submission, ReviewOutcome::Rejected, None))
                .await
                .unwrap(),
            Transition::AlreadyReviewed
        );
        assert_eq!(count(&db, "review_rounds").await, 1);
    }

    #[tokio::test]
    async fn test_a_round_that_cannot_be_written_leaves_the_submission_in_place() {
        // Atomicity, in the direction that matters: the delete runs first, so
        // without one transaction a failure while recording the round would
        // destroy the submission and leave nothing saying it existed. A primary
        // key collision on the round is the cheapest real failure to provoke.
        let db = test_db("rounds-rollback").await;
        let submission = a_submission(&db, None).await;
        let round = a_round(&submission, ReviewOutcome::Rejected, None);
        ReviewRoundRepository::discard(&db, submission.id, &round).await.unwrap();

        let second = a_submission(&db, None).await;
        let colliding = ReviewRound { submission_id: second.id, ..round.clone() };
        assert!(
            ReviewRoundRepository::discard(&db, second.id, &colliding).await.is_err(),
            "a duplicate round id must not be accepted"
        );

        assert_eq!(
            SubmissionRepository::find(&db, second.id).await.unwrap(),
            Some(second),
            "the submission survives a round that could not be written"
        );
        assert_eq!(count(&db, "review_rounds").await, 1);
    }

    #[tokio::test]
    async fn test_rounds_are_newest_first_and_repeated_rounds_all_survive() {
        // "Repeated reject cycles leave no trace anywhere" is the gap this
        // answers: three rounds on one project are three rows, and the head of
        // the list is the one the depositor's form shows.
        let db = test_db("rounds-history").await;
        let mut outcomes = vec![];
        for (hour, outcome) in [
            (13, ReviewOutcome::Rejected),
            (14, ReviewOutcome::ChangesRequested),
            (15, ReviewOutcome::Withdrawn),
        ] {
            let submission = a_submission(&db, None).await;
            let round = ReviewRound { at: at(hour), ..a_round(&submission, outcome, None) };
            ReviewRoundRepository::discard(&db, submission.id, &round).await.unwrap();
            outcomes.push(outcome);
        }

        let rounds = db.list_for_shortcode("0801").await.unwrap();
        assert_eq!(rounds.len(), 3, "every round survives the next one");
        outcomes.reverse();
        assert_eq!(rounds.iter().map(|round| round.outcome).collect::<Vec<_>>(), outcomes);
    }

    #[tokio::test]
    async fn test_rounds_for_another_project_are_not_listed() {
        let db = test_db("rounds-per-project").await;
        let submission = a_submission(&db, None).await;
        ReviewRoundRepository::discard(&db, submission.id, &a_round(&submission, ReviewOutcome::Rejected, None))
            .await
            .unwrap();

        assert!(db.list_for_shortcode("0803").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_removing_the_actor_leaves_the_round_with_no_actor() {
        // ON DELETE SET NULL, as everywhere else: removing an account must not
        // destroy the record of what was decided.
        let db = test_db("rounds-actor-removed").await;
        let reviewer = a_user(&db, "rdu@x.test", Role::Rdu).await;
        let submission = a_submission(&db, None).await;
        let round = a_round(&submission, ReviewOutcome::Rejected, Some(reviewer));
        ReviewRoundRepository::discard(&db, submission.id, &round).await.unwrap();

        UserRepository::delete(&db, reviewer).await.unwrap();

        let stored = db.list_for_shortcode("0801").await.unwrap();
        assert_eq!(stored, vec![ReviewRound { actor: None, ..round }]);
    }

    #[tokio::test]
    async fn test_an_unknown_outcome_is_refused_by_the_check_constraint() {
        // The CHECK constraint and `FromStr` have to agree. If the constraint
        // let an unknown outcome in, reading it back would fail at a handler
        // instead of at the write. `submitted` is the trap: a submission state,
        // one table over, sharing the word `approved` with this vocabulary.
        let db = test_db("rounds-outcome-check").await;
        let result = db
            .write(|tx| {
                tx.execute(
                    "INSERT INTO review_rounds (id, shortcode, submission_id, outcome, at) \
                     VALUES ('r1', '0801', 's1', 'submitted', '2026-09-08 13:00:00+00:00')",
                    [],
                )
            })
            .await;
        assert!(result.is_err(), "an unknown outcome must be rejected at the write");
    }
}
