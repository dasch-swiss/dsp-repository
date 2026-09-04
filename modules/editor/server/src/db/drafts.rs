//! [`DraftRepository`] against SQLite.

use async_trait::async_trait;
use editor_core::records::DraftRecord;
use editor_core::repository::{DraftRepository, Result};
use rusqlite::{params, Row};

use super::mapping::{optional_uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "draft";

const SELECT: &str = "SELECT shortcode, payload, updated_by, reviewer_note, created_at, updated_at FROM drafts";

fn map_row(row: &Row<'_>) -> rusqlite::Result<DraftRecord> {
    Ok(DraftRecord {
        shortcode: row.get(0)?,
        payload: row.get(1)?,
        updated_by: optional_uuid_column(row, 2)?,
        reviewer_note: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[async_trait]
impl DraftRepository for Database {
    async fn upsert(&self, draft: &DraftRecord) -> Result<()> {
        let draft = draft.clone();
        self.write(move |tx| {
            // `created_at` is kept from the existing row on conflict: it records
            // when the depositor started, and overwriting it on every save would
            // make a draft look newly created each time it was touched.
            tx.execute(
                "INSERT INTO drafts (shortcode, payload, updated_by, reviewer_note, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT (shortcode) DO UPDATE SET payload = ?2, updated_by = ?3, reviewer_note = ?4, \
                 updated_at = ?6",
                params![
                    draft.shortcode,
                    draft.payload,
                    draft.updated_by.map(|id| id.to_string()),
                    draft.reviewer_note,
                    draft.created_at,
                    draft.updated_at,
                ],
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn find(&self, shortcode: &str) -> Result<Option<DraftRecord>> {
        let shortcode = shortcode.to_string();
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE shortcode = ?1"), params![shortcode], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn list(&self) -> Result<Vec<DraftRecord>> {
        Ok(self
            .read(|conn| {
                let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY updated_at DESC, shortcode"))?;
                let rows = stmt.query_map([], map_row)?;
                rows.collect()
            })
            .await?)
    }

    async fn delete(&self, shortcode: &str) -> Result<bool> {
        let shortcode = shortcode.to_string();
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM drafts WHERE shortcode = ?1", params![shortcode]))
            .await?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use editor_core::records::{Role, User};
    use editor_core::repository::UserRepository;
    use uuid::Uuid;

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    async fn a_user(db: &Database) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: "a@x.test".to_string(),
            name: "A".to_string(),
            role: Role::Depositor,
            shortcodes: vec!["0801".to_string()],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    fn draft(shortcode: &str, payload: &str, author: Option<Uuid>, updated: DateTime<Utc>) -> DraftRecord {
        DraftRecord {
            shortcode: shortcode.to_string(),
            payload: payload.to_string(),
            updated_by: author,
            reviewer_note: None,
            created_at: at(10),
            updated_at: updated,
        }
    }

    #[tokio::test]
    async fn test_upsert_then_find_round_trips_every_field() {
        let db = test_db("drafts-round-trip").await;
        let author = a_user(&db).await;
        let mut draft = draft("0801", r#"{"name":"work in progress"}"#, Some(author), at(11));
        // Set rather than left `None`, or the column added by 0004 round-trips
        // vacuously: every field of this record is compared below, and a `None`
        // compares equal whether the column is read, written, or missing.
        draft.reviewer_note = Some("Please add a German description.".to_string());
        db.upsert(&draft).await.unwrap();

        assert_eq!(DraftRepository::find(&db, "0801").await.unwrap(), Some(draft));
    }

    #[tokio::test]
    async fn test_upsert_replaces_the_reviewer_note() {
        // The note describes one review round (REQ-4.5). Left out of the
        // conflict clause it would survive the round it belongs to and read as
        // current advice on work that has already answered it.
        let db = test_db("drafts-note").await;
        let author = a_user(&db).await;
        let mut first = draft("0801", "first", Some(author), at(11));
        first.reviewer_note = Some("Add a German description.".to_string());
        db.upsert(&first).await.unwrap();

        let second = draft("0801", "second", Some(author), at(12));
        assert_eq!(second.reviewer_note, None, "the premise of this test");
        db.upsert(&second).await.unwrap();

        let found = DraftRepository::find(&db, "0801").await.unwrap().unwrap();
        assert_eq!(found.reviewer_note, None);
    }

    #[tokio::test]
    async fn test_upsert_replaces_the_payload_and_keeps_the_original_created_at() {
        // One draft per project, last write wins (PRD Constraints). `created_at`
        // records when the depositor started; refreshing it on every save would
        // lose that.
        let db = test_db("drafts-upsert").await;
        let author = a_user(&db).await;
        db.upsert(&draft("0801", "first", Some(author), at(11))).await.unwrap();

        let mut later = draft("0801", "second", Some(author), at(13));
        later.created_at = at(12); // A caller that does not know the original.
        db.upsert(&later).await.unwrap();

        let found = DraftRepository::find(&db, "0801").await.unwrap().unwrap();
        assert_eq!(found.payload, "second");
        assert_eq!(found.updated_at, at(13));
        assert_eq!(found.created_at, at(10), "the original created_at must survive the update");
        assert_eq!(
            count(&db, "drafts").await,
            1,
            "an upsert must not add a second row for one project"
        );
    }

    #[tokio::test]
    async fn test_payload_is_stored_verbatim() {
        // This layer never interprets the payload, which is a serialized
        // `ProjectDraft`. A byte-exact round trip is what lets the caller parse
        // it without this layer or a migration having to change.
        let db = test_db("drafts-payload").await;
        let payload = "{\"ar\":\"مرحبا\",\"nested\":{\"a\":[1,2,null]},\"quote\":\"he said \\\"hi\\\"\"}";
        db.upsert(&draft("0820", payload, None, at(11))).await.unwrap();

        assert_eq!(DraftRepository::find(&db, "0820").await.unwrap().unwrap().payload, payload);
    }

    #[tokio::test]
    async fn test_list_returns_every_draft_newest_first() {
        // REQ-1.11: RDU sees all drafts, not only their own projects'.
        let db = test_db("drafts-list").await;
        db.upsert(&draft("0801", "a", None, at(11))).await.unwrap();
        db.upsert(&draft("0803", "b", None, at(13))).await.unwrap();
        db.upsert(&draft("0805", "c", None, at(12))).await.unwrap();

        let shortcodes: Vec<_> = DraftRepository::list(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|d| d.shortcode)
            .collect();
        assert_eq!(shortcodes, vec!["0803".to_string(), "0805".to_string(), "0801".to_string()]);
    }

    #[tokio::test]
    async fn test_delete_reports_whether_there_was_anything_to_delete() {
        let db = test_db("drafts-delete").await;
        db.upsert(&draft("0801", "a", None, at(11))).await.unwrap();

        assert!(DraftRepository::delete(&db, "0801").await.unwrap());
        assert!(!DraftRepository::delete(&db, "0801").await.unwrap());
        assert_eq!(count(&db, "drafts").await, 0);
    }

    #[tokio::test]
    async fn test_a_draft_by_an_unknown_user_is_refused() {
        // The foreign key. A draft attributed to a user id that never existed
        // would show an author the depositor list cannot explain.
        let db = test_db("drafts-orphan").await;
        let error = db
            .upsert(&draft("0801", "a", Some(Uuid::new_v4()), at(11)))
            .await
            .expect_err("a draft for an unknown user must be refused");
        assert!(
            matches!(error, editor_core::repository::RepositoryError::Backend(_)),
            "a foreign-key failure must not be reported as a conflict: {error}"
        );
        assert_eq!(count(&db, "drafts").await, 0);
    }
}
