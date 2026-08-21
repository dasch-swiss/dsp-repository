//! [`SessionRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::records::Session;
use editor_core::repository::{RepositoryError, Result, SessionRepository};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "session";

const SELECT: &str = "SELECT id, user_id, created_at, last_seen_at, expires_at FROM sessions";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        user_id: uuid_column(row, 1)?,
        created_at: row.get(2)?,
        last_seen_at: row.get(3)?,
        expires_at: row.get(4)?,
    })
}

#[async_trait]
impl SessionRepository for Database {
    async fn create(&self, session: &Session) -> Result<()> {
        let session = session.clone();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO sessions (id, user_id, created_at, last_seen_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session.id,
                    session.user_id.to_string(),
                    session.created_at,
                    session.last_seen_at,
                    session.expires_at,
                ],
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn find(&self, id: &str) -> Result<Option<Session>> {
        let id = id.to_string();
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE id = ?1"), params![id], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn touch(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
        let id = id.to_string();
        let updated = self
            .write(move |tx| tx.execute("UPDATE sessions SET last_seen_at = ?2 WHERE id = ?1", params![id, at]))
            .await?;
        if updated == 0 {
            return Err(RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let id = id.to_string();
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM sessions WHERE id = ?1", params![id]))
            .await?;
        Ok(deleted > 0)
    }

    async fn delete_for_user(&self, user_id: Uuid) -> Result<u64> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id.to_string()]))
            .await?;
        Ok(deleted as u64)
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now]))
            .await?;
        Ok(deleted as u64)
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

    async fn a_user(db: &Database) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: "a@x.test".to_string(),
            name: "A".to_string(),
            role: Role::Depositor,
            shortcodes: vec![],
            failed_logins: 0,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    fn session(id: &str, user_id: Uuid, expires: DateTime<Utc>) -> Session {
        Session {
            id: id.to_string(),
            user_id,
            created_at: at(10),
            last_seen_at: at(10),
            expires_at: expires,
        }
    }

    #[tokio::test]
    async fn test_create_then_find_round_trips_every_field() {
        let db = test_db("sessions-round-trip").await;
        let user_id = a_user(&db).await;
        let session = session("token-1", user_id, at(18));
        SessionRepository::create(&db, &session).await.unwrap();

        assert_eq!(db.find("token-1").await.unwrap(), Some(session));
    }

    #[tokio::test]
    async fn test_find_of_an_unknown_token_is_none_not_an_error() {
        // The common case on any request carrying a stale cookie, so it must not
        // be an error path.
        let db = test_db("sessions-find-missing").await;
        assert_eq!(db.find("no-such-token").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_touch_advances_last_seen_without_moving_the_absolute_expiry() {
        // The idle timeout advances; the absolute expiry does not. If `touch`
        // moved `expires_at` too, a session in continuous use would never expire.
        let db = test_db("sessions-touch").await;
        let user_id = a_user(&db).await;
        SessionRepository::create(&db, &session("token-1", user_id, at(18)))
            .await
            .unwrap();

        db.touch("token-1", at(14)).await.unwrap();

        let found = db.find("token-1").await.unwrap().unwrap();
        assert_eq!(found.last_seen_at, at(14));
        assert_eq!(found.expires_at, at(18));
    }

    #[tokio::test]
    async fn test_touch_of_an_unknown_token_is_not_found() {
        let db = test_db("sessions-touch-missing").await;
        let error = db
            .touch("no-such-token", at(14))
            .await
            .expect_err("touching a gone session must fail");
        assert!(matches!(error, RepositoryError::NotFound { entity: "session" }), "{error}");
    }

    #[tokio::test]
    async fn test_delete_reports_whether_there_was_anything_to_delete() {
        let db = test_db("sessions-delete").await;
        let user_id = a_user(&db).await;
        SessionRepository::create(&db, &session("token-1", user_id, at(18)))
            .await
            .unwrap();

        assert!(
            SessionRepository::delete(&db, "token-1").await.unwrap(),
            "the first delete removes it"
        );
        assert!(
            !SessionRepository::delete(&db, "token-1").await.unwrap(),
            "the second finds nothing"
        );
    }

    #[tokio::test]
    async fn test_delete_for_user_clears_every_session_of_that_user_only() {
        // Session rotation on login depends on this, and so does logging out
        // everywhere. Taking another user's sessions with it would log out
        // bystanders.
        let db = test_db("sessions-delete-for-user").await;
        let first = a_user(&db).await;
        let second = {
            let user = User {
                id: Uuid::new_v4(),
                email: "b@x.test".to_string(),
                name: "B".to_string(),
                role: Role::Depositor,
                shortcodes: vec![],
                failed_logins: 0,
                last_code_at: None,
                created_at: at(9),
            };
            UserRepository::create(&db, &user).await.unwrap();
            user.id
        };
        SessionRepository::create(&db, &session("a-1", first, at(18))).await.unwrap();
        SessionRepository::create(&db, &session("a-2", first, at(18))).await.unwrap();
        SessionRepository::create(&db, &session("b-1", second, at(18))).await.unwrap();

        assert_eq!(db.delete_for_user(first).await.unwrap(), 2);
        assert_eq!(count(&db, "sessions").await, 1);
        assert!(db.find("b-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_takes_only_sessions_past_their_absolute_expiry() {
        // `expires_at` is TEXT, so this is a string comparison; it is
        // chronological only because the stored format is fixed-width UTC.
        let db = test_db("sessions-delete-expired").await;
        let user_id = a_user(&db).await;
        SessionRepository::create(&db, &session("expired", user_id, at(11)))
            .await
            .unwrap();
        SessionRepository::create(&db, &session("boundary", user_id, at(12)))
            .await
            .unwrap();
        SessionRepository::create(&db, &session("live", user_id, at(13))).await.unwrap();

        assert_eq!(db.delete_expired(at(12)).await.unwrap(), 2, "the boundary counts as expired");
        assert!(db.find("expired").await.unwrap().is_none());
        assert!(db.find("boundary").await.unwrap().is_none());
        assert!(db.find("live").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_a_session_for_an_unknown_user_is_refused() {
        // The foreign key, observable. Without it an orphaned session would
        // authenticate a request against a user row that does not exist.
        let db = test_db("sessions-orphan").await;
        let error = SessionRepository::create(&db, &session("token-1", Uuid::new_v4(), at(18)))
            .await
            .expect_err("a session for an unknown user must be refused");
        // A foreign-key failure is not a duplicate. Reported as `Conflict` it
        // would read as "session already exists" and send the reader looking for
        // a second row that is not there, so only unique and primary-key
        // violations map to `Conflict`.
        assert!(
            matches!(error, RepositoryError::Backend(_)),
            "a foreign-key failure must not be reported as a conflict: {error}"
        );
        assert_eq!(count(&db, "sessions").await, 0);
    }
}
