//! [`ApprovedRecordRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::records::ApprovedRecord;
use editor_core::repository::{ApprovedRecordRepository, RepositoryError, Result};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{optional_uuid_column, uuid_column};
use super::Database;

const ENTITY: &str = "approved record";

const SELECT: &str = "SELECT id, shortcode, payload, approved_by, approved_at, collected_at FROM approved_records";

fn map_row(row: &Row<'_>) -> rusqlite::Result<ApprovedRecord> {
    Ok(ApprovedRecord {
        id: uuid_column(row, 0)?,
        shortcode: row.get(1)?,
        payload: row.get(2)?,
        approved_by: optional_uuid_column(row, 3)?,
        approved_at: row.get(4)?,
        collected_at: row.get(5)?,
    })
}

#[async_trait]
impl ApprovedRecordRepository for Database {
    async fn create(&self, record: &ApprovedRecord) -> Result<()> {
        let record = record.clone();
        self.write(move |tx| {
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
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn list_uncollected(&self) -> Result<Vec<ApprovedRecord>> {
        Ok(self
            .read(|conn| {
                // Oldest first, so a run that fails part-way still makes progress
                // from the front of the queue. `collected_at IS NULL` matches the
                // partial index, so collected rows are never scanned.
                let mut stmt =
                    conn.prepare(&format!("{SELECT} WHERE collected_at IS NULL ORDER BY approved_at, shortcode"))?;
                let rows = stmt.query_map([], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn find_by_shortcode(&self, shortcode: &str) -> Result<Vec<ApprovedRecord>> {
        let shortcode = shortcode.to_string();
        Ok(self
            .read(move |conn| {
                let mut stmt = conn.prepare(&format!("{SELECT} WHERE shortcode = ?1 ORDER BY approved_at"))?;
                let rows = stmt.query_map(params![shortcode], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn mark_collected(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        // `collected_at IS NULL` in the WHERE clause, so a re-run of a partly
        // successful collection cannot overwrite the time a record was first
        // collected.
        let marked = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE approved_records SET collected_at = ?2 WHERE id = ?1 AND collected_at IS NULL",
                    params![id.to_string(), at],
                )
            })
            .await?;
        if marked == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM approved_records WHERE id = ?1", params![id.to_string()]))
            .await?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use editor_core::records::{Role, User};
    use editor_core::repository::UserRepository;

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    async fn an_rdu_member(db: &Database) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: "rdu@x.test".to_string(),
            name: "R".to_string(),
            role: Role::Rdu,
            shortcodes: vec![],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    fn record(shortcode: &str, approver: Option<Uuid>, approved: DateTime<Utc>) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: r#"{"name":"approved"}"#.to_string(),
            approved_by: approver,
            approved_at: approved,
            collected_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_then_list_round_trips_every_field() {
        let db = test_db("approved-round-trip").await;
        let approver = an_rdu_member(&db).await;
        let record = record("0801", Some(approver), at(14));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        assert_eq!(db.list_uncollected().await.unwrap(), vec![record]);
    }

    #[tokio::test]
    async fn test_list_uncollected_excludes_collected_records() {
        // REQ-5.1: the public endpoint serves approved-and-uncollected only.
        // Serving collected ones would reopen a pull request on every run.
        let db = test_db("approved-uncollected").await;
        let first = record("0801", None, at(12));
        let second = record("0803", None, at(13));
        ApprovedRecordRepository::create(&db, &first).await.unwrap();
        ApprovedRecordRepository::create(&db, &second).await.unwrap();

        db.mark_collected(first.id, at(15)).await.unwrap();

        let uncollected: Vec<_> = db.list_uncollected().await.unwrap().into_iter().map(|r| r.shortcode).collect();
        assert_eq!(uncollected, vec!["0803".to_string()]);
    }

    #[tokio::test]
    async fn test_list_uncollected_is_oldest_first() {
        let db = test_db("approved-order").await;
        ApprovedRecordRepository::create(&db, &record("0805", None, at(14)))
            .await
            .unwrap();
        ApprovedRecordRepository::create(&db, &record("0801", None, at(12)))
            .await
            .unwrap();
        ApprovedRecordRepository::create(&db, &record("0803", None, at(13)))
            .await
            .unwrap();

        let order: Vec<_> = db.list_uncollected().await.unwrap().into_iter().map(|r| r.shortcode).collect();
        assert_eq!(order, vec!["0801".to_string(), "0803".to_string(), "0805".to_string()]);
    }

    #[tokio::test]
    async fn test_a_failed_collection_leaves_the_record_for_the_next_run() {
        // REQ-5.7. The record stays uncollected because nothing stamped it, which
        // is why `mark_collected` is a separate call after the pull request is
        // open rather than part of serving the endpoint.
        let db = test_db("approved-retry").await;
        let record = record("0801", None, at(12));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        assert_eq!(db.list_uncollected().await.unwrap().len(), 1);
        assert_eq!(db.list_uncollected().await.unwrap().len(), 1, "reading it must not consume it");
    }

    #[tokio::test]
    async fn test_mark_collected_is_not_repeatable() {
        // A second run must not overwrite when the record was first collected —
        // that time is the only record of when the pull request went out.
        let db = test_db("approved-mark-once").await;
        let record = record("0801", None, at(12));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        db.mark_collected(record.id, at(15)).await.unwrap();
        let error = db
            .mark_collected(record.id, at(16))
            .await
            .expect_err("a second mark must be refused");
        assert!(
            matches!(error, RepositoryError::NotFound { entity: "approved record" }),
            "{error}"
        );

        let collected = db.find_by_shortcode("0801").await.unwrap()[0].collected_at;
        assert_eq!(collected, Some(at(15)));
    }

    #[tokio::test]
    async fn test_find_by_shortcode_returns_every_record_for_that_project() {
        // The startup comparison (REQ-2.3) needs all of a project's records, not
        // just the uncollected ones, to find the one that matches the published
        // data.
        let db = test_db("approved-by-shortcode").await;
        let collected = record("0801", None, at(12));
        ApprovedRecordRepository::create(&db, &collected).await.unwrap();
        ApprovedRecordRepository::create(&db, &record("0801", None, at(13)))
            .await
            .unwrap();
        ApprovedRecordRepository::create(&db, &record("0803", None, at(14)))
            .await
            .unwrap();
        db.mark_collected(collected.id, at(15)).await.unwrap();

        assert_eq!(db.find_by_shortcode("0801").await.unwrap().len(), 2);
        assert_eq!(db.find_by_shortcode("0803").await.unwrap().len(), 1);
        assert!(db.find_by_shortcode("0899").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_delete_discards_the_local_record_once_the_change_is_online() {
        // REQ-2.4: a record identical to the published data is dropped.
        let db = test_db("approved-delete").await;
        let record = record("0801", None, at(12));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        assert!(ApprovedRecordRepository::delete(&db, record.id).await.unwrap());
        assert!(!ApprovedRecordRepository::delete(&db, record.id).await.unwrap());
        assert_eq!(count(&db, "approved_records").await, 0);
    }

    #[tokio::test]
    async fn test_removing_the_approver_leaves_the_record_collectable() {
        // ON DELETE SET NULL. An approved record must not disappear because the
        // RDU member who approved it left — the change is already approved for
        // publication.
        let db = test_db("approved-approver-removed").await;
        let approver = an_rdu_member(&db).await;
        let record = record("0801", Some(approver), at(12));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        UserRepository::delete(&db, approver).await.unwrap();

        let uncollected = db.list_uncollected().await.unwrap();
        assert_eq!(uncollected.len(), 1);
        assert_eq!(uncollected[0].approved_by, None);
        assert_eq!(uncollected[0].payload, record.payload);
    }
}
