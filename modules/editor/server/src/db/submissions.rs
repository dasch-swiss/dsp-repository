//! [`SubmissionRepository`] against SQLite.

use async_trait::async_trait;
use editor_core::records::{Submission, SubmissionState};
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
            )
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
    use editor_core::records::{Role, User};
    use editor_core::repository::UserRepository;

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
