//! [`ApprovedRecordRepository`] against SQLite.

use async_trait::async_trait;
use editor_core::records::ApprovedRecord;
use editor_core::repository::{ApprovedRecordRepository, Result};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{optional_uuid_column, uuid_column};
use super::Database;

const ENTITY: &str = "approved record";

const SELECT: &str = "SELECT id, shortcode, payload, approved_by, approved_at FROM approved_records";

fn map_row(row: &Row<'_>) -> rusqlite::Result<ApprovedRecord> {
    Ok(ApprovedRecord {
        id: uuid_column(row, 0)?,
        shortcode: row.get(1)?,
        payload: row.get(2)?,
        approved_by: optional_uuid_column(row, 3)?,
        approved_at: row.get(4)?,
    })
}

#[async_trait]
impl ApprovedRecordRepository for Database {
    async fn create(&self, record: &ApprovedRecord) -> Result<()> {
        let record = record.clone();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO approved_records (id, shortcode, payload, approved_by, approved_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    record.id.to_string(),
                    record.shortcode,
                    record.payload,
                    record.approved_by.map(|id| id.to_string()),
                    record.approved_at,
                ],
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
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

    async fn list_all(&self) -> Result<Vec<ApprovedRecord>> {
        Ok(self
            .read(|conn| {
                // So the startup log reads in approval order, not rowid order.
                let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY approved_at, shortcode"))?;
                let rows = stmt.query_map([], map_row)?;
                rows.collect()
            })
            .await?)
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
    use chrono::{DateTime, TimeZone, Utc};

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    fn record(shortcode: &str, approver: Option<Uuid>, approved: DateTime<Utc>) -> ApprovedRecord {
        ApprovedRecord {
            id: Uuid::new_v4(),
            shortcode: shortcode.to_string(),
            payload: r#"{"name":"approved"}"#.to_string(),
            approved_by: approver,
            approved_at: approved,
        }
    }

    #[tokio::test]
    async fn test_create_then_list_round_trips_every_field() {
        let db = test_db("approved-round-trip").await;
        let approver = Uuid::new_v4();
        let record = record("0801", Some(approver), at(14));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        assert_eq!(db.list_all().await.unwrap(), vec![record]);
    }

    #[tokio::test]
    async fn test_list_all_is_oldest_first() {
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

        let order: Vec<_> = db.list_all().await.unwrap().into_iter().map(|r| r.shortcode).collect();
        assert_eq!(order, vec!["0801".to_string(), "0803".to_string(), "0805".to_string()]);
    }

    #[tokio::test]
    async fn test_find_by_shortcode_returns_every_record_for_that_project() {
        // The startup comparison needs all of a project's records to find the
        // one that matches the published data.
        let db = test_db("approved-by-shortcode").await;
        ApprovedRecordRepository::create(&db, &record("0801", None, at(12)))
            .await
            .unwrap();
        ApprovedRecordRepository::create(&db, &record("0801", None, at(13)))
            .await
            .unwrap();
        ApprovedRecordRepository::create(&db, &record("0803", None, at(14)))
            .await
            .unwrap();

        assert_eq!(db.find_by_shortcode("0801").await.unwrap().len(), 2);
        assert_eq!(db.find_by_shortcode("0803").await.unwrap().len(), 1);
        assert!(db.find_by_shortcode("0899").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_delete_discards_the_local_record_once_the_change_is_online() {
        let db = test_db("approved-delete").await;
        let record = record("0801", None, at(12));
        ApprovedRecordRepository::create(&db, &record).await.unwrap();

        assert!(ApprovedRecordRepository::delete(&db, record.id).await.unwrap());
        assert!(!ApprovedRecordRepository::delete(&db, record.id).await.unwrap());
        assert_eq!(count(&db, "approved_records").await, 0);
    }
}
