//! [`SubmissionRepository`] against SQLite.
//!
//! [`create`](Database::create) also flips the project's `draft` proposals to `submitted`, in the
//! same transaction as the `submissions` INSERT. A submission that exists while its proposals still
//! read `draft` is a submission RDU cannot review — the review surface selects `submitted` rows —
//! and nothing in the schema would say the two disagreed, so the two writes cannot be allowed to
//! land separately.

use async_trait::async_trait;
use editor_core::proposals::ProposalStatus;
use editor_core::records::{normalize_shortcode, Submission, SubmissionState};
use editor_core::repository::{RepositoryError, Result, SubmissionRepository};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{optional_uuid_column, parsed_column, uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "submission";

const SELECT: &str = "SELECT id, shortcode, payload, state, submitted_by, submitted_at, reviewed_by, reviewed_at, \
                      reviewer_note, review_state FROM submissions";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Submission> {
    Ok(Submission {
        id: uuid_column(row, 0)?,
        shortcode: row.get(1)?,
        payload: row.get(2)?,
        state: parsed_column::<SubmissionState>(row, 3)?,
        submitted_by: optional_uuid_column(row, 4)?,
        submitted_at: row.get(5)?,
        reviewed_by: optional_uuid_column(row, 6)?,
        reviewed_at: row.get(7)?,
        reviewer_note: row.get(8)?,
        review_state: row.get(9)?,
    })
}

#[async_trait]
impl SubmissionRepository for Database {
    async fn create(&self, submission: &Submission) -> Result<()> {
        let submission = submission.clone();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO submissions (id, shortcode, payload, state, submitted_by, submitted_at, reviewed_by, \
                 reviewed_at, reviewer_note, review_state) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    submission.id.to_string(),
                    submission.shortcode,
                    submission.payload,
                    submission.state.as_str(),
                    submission.submitted_by.map(|id| id.to_string()),
                    submission.submitted_at,
                    submission.reviewed_by.map(|id| id.to_string()),
                    submission.reviewed_at,
                    submission.reviewer_note,
                    submission.review_state,
                ],
            )?;
            // REQ-3.3: a proposal rides with the project's pending submission through the same
            // review path, so it must carry the same status forward — a `draft` proposal on a
            // project that is now `submitted` is invisible to the review surface, which selects
            // `submitted` rows, and nothing in the schema would say the two disagreed.
            tx.execute(
                "UPDATE entity_proposals SET status = ?3, updated_at = ?4 \
                 WHERE shortcode = ?1 AND status = ?2",
                params![
                    // Normalized here rather than trusted from the caller. `entity_proposals`
                    // is keyed on the normalized shortcode — `create_new`/`create_change` always
                    // fold before insert — while this method stores `submissions.shortcode`
                    // exactly as given. Keyed on an unfolded `080C` this `UPDATE` matches zero
                    // rows and reports nothing: the submission exists, its proposals stay
                    // `draft`, and per this closure's own comment that makes them invisible to
                    // the review surface with nothing in the schema showing the disagreement.
                    // 24 of the 85 committed shortcodes are mixed case, so the failing shape is
                    // ordinary.
                    normalize_shortcode(&submission.shortcode),
                    ProposalStatus::Draft.as_str(),
                    ProposalStatus::Submitted.as_str(),
                    submission.submitted_at,
                ],
            )?;
            Ok(())
        })
        .await
        // `shortcode` is the only unique index here, so a constraint violation
        // is a second pending submission for one project — PRD Constraints'
        // "one pending submission per project", reported rather than silently
        // replacing the first.
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn update(&self, submission: &Submission) -> Result<()> {
        let submission = submission.clone();
        let updated = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE submissions SET payload = ?2, state = ?3, reviewed_by = ?4, reviewed_at = ?5, \
                     reviewer_note = ?6, review_state = ?7 WHERE id = ?1",
                    params![
                        submission.id.to_string(),
                        submission.payload,
                        submission.state.as_str(),
                        submission.reviewed_by.map(|id| id.to_string()),
                        submission.reviewed_at,
                        submission.reviewer_note,
                        submission.review_state,
                    ],
                )
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        if updated == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn find(&self, id: Uuid) -> Result<Option<Submission>> {
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE id = ?1"), params![id.to_string()], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Option<Submission>> {
        let shortcode = shortcode.to_string();
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE shortcode = ?1"), params![shortcode], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn list(&self) -> Result<Vec<Submission>> {
        Ok(self
            .read(|conn| {
                // Oldest first (REQ-4.1), with the shortcode breaking ties so two
                // submissions made in the same instant have a stable order.
                let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY submitted_at, shortcode"))?;
                let rows = stmt.query_map([], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM submissions WHERE id = ?1", params![id.to_string()]))
            .await?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation};
    use editor_core::records::{Role, User};
    use editor_core::repository::{EntityProposalRepository, UserRepository};

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
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

    fn submission(shortcode: &str, author: Option<Uuid>, submitted: DateTime<Utc>) -> Submission {
        Submission {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: r#"{"name":"submitted"}"#.to_string(),
            state: SubmissionState::Submitted,
            submitted_by: author,
            submitted_at: submitted,
            reviewed_by: None,
            reviewed_at: None,
            reviewer_note: None,
            review_state: None,
        }
    }

    fn a_proposal(shortcode: &str, entity_id: &str, status: ProposalStatus) -> EntityProposal {
        EntityProposal {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            entity_id: entity_id.to_string(),
            kind: ProposalKind::Person,
            operation: ProposalOperation::New,
            payload: r#"{"name":"placeholder"}"#.to_string(),
            status,
            proposed_by: None,
            created_at: at(10),
            updated_at: at(10),
            decision: None,
            decided_by: None,
            decided_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_flips_its_own_draft_proposals_to_submitted_and_leaves_another_project_alone() {
        let db = test_db("submissions-create-flips-draft-proposals").await;
        let own_draft = a_proposal("0801", "person-417", ProposalStatus::Draft);
        let other_draft = a_proposal("0803", "person-418", ProposalStatus::Draft);
        EntityProposalRepository::create_change(&db, &own_draft).await.unwrap();
        EntityProposalRepository::create_change(&db, &other_draft).await.unwrap();

        SubmissionRepository::create(&db, &submission("0801", None, at(11)))
            .await
            .unwrap();

        let own = EntityProposalRepository::find(&db, own_draft.id).await.unwrap().unwrap();
        assert_eq!(own.status, ProposalStatus::Submitted);
        assert_eq!(own.updated_at, at(11), "stamped from the submission's submitted_at");
        let other = EntityProposalRepository::find(&db, other_draft.id).await.unwrap().unwrap();
        assert_eq!(other.status, ProposalStatus::Draft, "another project's proposal is untouched");
    }

    /// The mixed-case case `review_rounds.rs` has its own test for, on this side of the pair.
    ///
    /// `entity_proposals` is keyed on the folded shortcode; this method stores
    /// `submissions.shortcode` as given. Keyed unfolded, the `UPDATE` matches nothing and says
    /// nothing — the submission exists while its proposals stay `draft`, invisible to the
    /// review surface. 24 of the 85 committed shortcodes are mixed case, so this is the
    /// ordinary shape, not an edge case.
    #[tokio::test]
    async fn test_create_flips_proposals_for_a_mixed_case_shortcode() {
        let db = test_db("submissions-create-mixed-case-shortcode").await;
        let proposal = a_proposal("080C", "person-501", ProposalStatus::Draft);
        EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

        let mut submission = submission("0801", None, at(11));
        submission.shortcode = "080C".to_string();
        SubmissionRepository::create(&db, &submission).await.unwrap();

        let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
        assert_eq!(found.status, ProposalStatus::Submitted, "the fold must reach the proposal row");
    }

    #[tokio::test]
    async fn test_create_does_not_touch_a_proposal_that_is_not_draft() {
        let db = test_db("submissions-create-ignores-non-draft-proposals").await;
        for (index, status) in [
            ProposalStatus::Submitted,
            ProposalStatus::Accepted,
            ProposalStatus::Rejected,
            ProposalStatus::Withdrawn,
        ]
        .into_iter()
        .enumerate()
        {
            // A distinct `entity_id` per iteration: `create_change` writes the row as given, and
            // `entity_proposals_allocated_id` refuses two `new`-operation rows sharing one id.
            let proposal = a_proposal("0801", &format!("person-{}", 500 + index), status);
            EntityProposalRepository::create_change(&db, &proposal).await.unwrap();

            SubmissionRepository::create(&db, &submission("0801", None, at(11)))
                .await
                .unwrap();

            let found = EntityProposalRepository::find(&db, proposal.id).await.unwrap().unwrap();
            assert_eq!(found.status, status, "a proposal not in draft must not be touched");

            let pending = SubmissionRepository::find_by_shortcode(&db, "0801").await.unwrap().unwrap();
            SubmissionRepository::delete(&db, pending.id).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_create_then_find_round_trips_every_field() {
        let db = test_db("submissions-round-trip").await;
        let author = a_user(&db, "a@x.test", Role::Depositor).await;
        let mut submission = submission("0801", Some(author), at(11));
        // Set rather than left `None`, or the column added by 0004 round-trips
        // vacuously: a `None` compares equal whether the column is read,
        // written, or missing.
        submission.review_state = Some(r#"{"name":{"decision":"accept"}}"#.to_string());
        SubmissionRepository::create(&db, &submission).await.unwrap();

        assert_eq!(
            SubmissionRepository::find(&db, submission.id).await.unwrap(),
            Some(submission.clone())
        );
        assert_eq!(db.find_by_shortcode("0801").await.unwrap(), Some(submission));
    }

    #[tokio::test]
    async fn test_a_second_pending_submission_for_one_project_is_a_conflict() {
        // PRD Constraints: one pending submission per project. Enforced by the
        // unique index rather than by handlers remembering to check, so a race
        // between two submits cannot produce two rows.
        let db = test_db("submissions-conflict").await;
        let author = a_user(&db, "a@x.test", Role::Depositor).await;
        SubmissionRepository::create(&db, &submission("0801", Some(author), at(11)))
            .await
            .unwrap();

        let error = SubmissionRepository::create(&db, &submission("0801", Some(author), at(12)))
            .await
            .expect_err("a second submission for one project must be refused");
        assert!(matches!(error, RepositoryError::Conflict { entity: "submission" }), "{error}");
        assert_eq!(count(&db, "submissions").await, 1);
    }

    #[tokio::test]
    async fn test_update_records_the_review_without_moving_the_submission_time() {
        // `submitted_at` orders the review queue; a review that reset it would
        // send the submission to the back.
        let db = test_db("submissions-update").await;
        let author = a_user(&db, "a@x.test", Role::Depositor).await;
        let reviewer = a_user(&db, "rdu@x.test", Role::Rdu).await;
        let mut submission = submission("0801", Some(author), at(11));
        SubmissionRepository::create(&db, &submission).await.unwrap();

        submission.state = SubmissionState::Approved;
        submission.reviewed_by = Some(reviewer);
        submission.reviewed_at = Some(at(14));
        submission.reviewer_note = Some("Looks right.".to_string());
        submission.review_state = Some(r#"{"name":{"decision":"revert"}}"#.to_string());
        SubmissionRepository::update(&db, &submission).await.unwrap();

        let found = SubmissionRepository::find(&db, submission.id).await.unwrap().unwrap();
        assert_eq!(found.state, SubmissionState::Approved);
        assert_eq!(found.reviewed_by, Some(reviewer));
        assert_eq!(found.reviewer_note.as_deref(), Some("Looks right."));
        assert_eq!(found.review_state.as_deref(), Some(r#"{"name":{"decision":"revert"}}"#));
        assert_eq!(found.submitted_at, at(11));
    }

    #[tokio::test]
    async fn test_update_of_an_unknown_submission_is_not_found() {
        let db = test_db("submissions-update-missing").await;
        let error = SubmissionRepository::update(&db, &submission("0801", None, at(11)))
            .await
            .expect_err("an unknown submission must not update");
        assert!(matches!(error, RepositoryError::NotFound { entity: "submission" }), "{error}");
    }

    #[tokio::test]
    async fn test_list_is_oldest_first() {
        // REQ-4.1's review queue order.
        let db = test_db("submissions-list").await;
        SubmissionRepository::create(&db, &submission("0803", None, at(13)))
            .await
            .unwrap();
        SubmissionRepository::create(&db, &submission("0801", None, at(11)))
            .await
            .unwrap();
        SubmissionRepository::create(&db, &submission("0805", None, at(12)))
            .await
            .unwrap();

        let shortcodes: Vec<_> = SubmissionRepository::list(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.shortcode)
            .collect();
        assert_eq!(shortcodes, vec!["0801".to_string(), "0805".to_string(), "0803".to_string()]);
    }

    #[tokio::test]
    async fn test_delete_frees_the_project_for_a_new_submission() {
        // Reject (REQ-4.6) and depositor discard (REQ-4.7) both delete, and the
        // depositor must then be able to submit again.
        let db = test_db("submissions-delete").await;
        let first = submission("0801", None, at(11));
        SubmissionRepository::create(&db, &first).await.unwrap();

        assert!(SubmissionRepository::delete(&db, first.id).await.unwrap());
        assert!(!SubmissionRepository::delete(&db, first.id).await.unwrap());
        SubmissionRepository::create(&db, &submission("0801", None, at(12)))
            .await
            .unwrap();
        assert_eq!(count(&db, "submissions").await, 1);
    }

    #[tokio::test]
    async fn test_removing_the_submitter_leaves_the_submission_with_no_author() {
        // ON DELETE SET NULL. The review queue's "last editor" reads as unknown;
        // the submission itself is not destroyed by an account removal.
        let db = test_db("submissions-author-removed").await;
        let author = a_user(&db, "a@x.test", Role::Depositor).await;
        let submission = submission("0801", Some(author), at(11));
        SubmissionRepository::create(&db, &submission).await.unwrap();

        UserRepository::delete(&db, author).await.unwrap();

        let found = SubmissionRepository::find(&db, submission.id).await.unwrap().unwrap();
        assert_eq!(found.submitted_by, None);
        assert_eq!(found.payload, submission.payload);
    }

    #[tokio::test]
    async fn test_an_unknown_state_in_the_database_is_refused_by_the_check_constraint() {
        // The CHECK constraint and the `FromStr` mapping have to agree. If the
        // constraint let an unknown state in, reading it back would fail at a
        // handler instead of at the write.
        let db = test_db("submissions-state-check").await;
        let result = db
            .write(|tx| {
                tx.execute(
                    "INSERT INTO submissions (id, shortcode, payload, state, submitted_at) \
                     VALUES ('s1', '0801', '{}', 'rejected', '2026-08-21 11:00:00+00:00')",
                    [],
                )
            })
            .await;
        assert!(result.is_err(), "an unknown state must be rejected at the write");
    }
}
