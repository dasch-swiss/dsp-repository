//! [`LoginCodeRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::records::LoginCode;
use editor_core::repository::{LoginCodeRepository, RepositoryError, Result};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{counter, row_count, uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "login code";

const SELECT: &str = "SELECT id, user_id, code, attempts, created_at, expires_at, consumed_at FROM login_codes";

fn map_row(row: &Row<'_>) -> rusqlite::Result<LoginCode> {
    Ok(LoginCode {
        id: uuid_column(row, 0)?,
        user_id: uuid_column(row, 1)?,
        code: row.get(2)?,
        attempts: counter(row.get::<_, i64>(3)?),
        created_at: row.get(4)?,
        expires_at: row.get(5)?,
        consumed_at: row.get(6)?,
    })
}

#[async_trait]
impl LoginCodeRepository for Database {
    async fn create(&self, code: &LoginCode) -> Result<()> {
        let code = code.clone();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO login_codes (id, user_id, code, attempts, created_at, expires_at, consumed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    code.id.to_string(),
                    code.user_id.to_string(),
                    code.code,
                    i64::from(code.attempts),
                    code.created_at,
                    code.expires_at,
                    code.consumed_at,
                ],
            )
        })
        .await
        .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn find_active_for_user(&self, user_id: Uuid, now: DateTime<Utc>) -> Result<Option<LoginCode>> {
        Ok(self
            .read(move |conn| {
                conn.query_row(
                    &format!(
                        "{SELECT} WHERE user_id = ?1 AND consumed_at IS NULL AND expires_at > ?2 \
                         ORDER BY created_at DESC LIMIT 1"
                    ),
                    params![user_id.to_string(), now],
                    map_row,
                )
                .optional_row()
            })
            .await?)
    }

    async fn find_latest_for_user(&self, user_id: Uuid) -> Result<Option<LoginCode>> {
        // Whatever its state: the resend cooldown (REQ-6.5) is measured from the
        // last code *issued*, not the last one still usable, or a user could
        // defeat it by burning the code first.
        Ok(self
            .read(move |conn| {
                conn.query_row(
                    &format!("{SELECT} WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 1"),
                    params![user_id.to_string()],
                    map_row,
                )
                .optional_row()
            })
            .await?)
    }

    async fn record_attempt(&self, id: Uuid) -> Result<u32> {
        // Incremented and read back in one transaction, so parallel guesses
        // cannot both read the old count and spend one strike between them.
        let attempts = self
            .write(move |tx| {
                let updated = tx.execute(
                    "UPDATE login_codes SET attempts = attempts + 1 WHERE id = ?1",
                    params![id.to_string()],
                )?;
                if updated == 0 {
                    return Ok(None);
                }
                let attempts: i64 = tx.query_row(
                    "SELECT attempts FROM login_codes WHERE id = ?1",
                    params![id.to_string()],
                    |row| row.get(0),
                )?;
                Ok(Some(counter(attempts)))
            })
            .await?;
        attempts.ok_or(RepositoryError::NotFound { entity: ENTITY })
    }

    async fn consume(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool> {
        // `consumed_at IS NULL` in the WHERE clause is what makes this
        // single-use: a replay updates zero rows and gets `false`. Checking
        // first and then updating would leave a window in which two requests
        // both saw it unconsumed and both authenticated.
        let consumed = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE login_codes SET consumed_at = ?2 WHERE id = ?1 AND consumed_at IS NULL",
                    params![id.to_string(), at],
                )
            })
            .await?;
        Ok(consumed > 0)
    }

    async fn delete_for_user(&self, user_id: Uuid) -> Result<u64> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM login_codes WHERE user_id = ?1", params![user_id.to_string()]))
            .await?;
        Ok(deleted as u64)
    }

    async fn count_issued_since(&self, since: DateTime<Utc>) -> Result<u64> {
        // Across all users: the cap exists because the relay quota is shared, so
        // a per-user count would not see the loop that exhausts it.
        let counted: i64 = self
            .read(move |conn| {
                conn.query_row(
                    "SELECT count(*) FROM login_codes WHERE created_at >= ?1",
                    params![since],
                    |row| row.get(0),
                )
            })
            .await?;
        Ok(row_count(counted))
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM login_codes WHERE expires_at <= ?1", params![now]))
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

    async fn a_user(db: &Database, email: &str) -> Uuid {
        let user = User {
            id: Uuid::new_v4(),
            email: email.to_string(),
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

    fn code(user_id: Uuid, digits: &str, created: DateTime<Utc>, expires: DateTime<Utc>) -> LoginCode {
        LoginCode {
            id: Uuid::new_v4(),
            user_id,
            code: digits.to_string(),
            attempts: 0,
            created_at: created,
            expires_at: expires,
            consumed_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_then_find_round_trips_every_field() {
        let db = test_db("codes-round-trip").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        assert_eq!(db.find_active_for_user(user_id, at(10)).await.unwrap(), Some(code));
    }

    #[tokio::test]
    async fn test_an_expired_code_is_not_active() {
        // Ten minutes is the whole point of the expiry (REQ-6.1); a code that
        // stayed active past it would be a standing credential in a mailbox.
        let db = test_db("codes-expired").await;
        let user_id = a_user(&db, "a@x.test").await;
        LoginCodeRepository::create(&db, &code(user_id, "123456", at(10), at(11)))
            .await
            .unwrap();

        assert!(
            db.find_active_for_user(user_id, at(11)).await.unwrap().is_none(),
            "the expiry instant is not active"
        );
        assert!(db.find_active_for_user(user_id, at(12)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_a_consumed_code_is_not_active_and_cannot_be_consumed_twice() {
        // Replay resistance (NIST SP 800-63B-4 §3.1.3.2). The single-use check
        // is in the UPDATE's WHERE clause, so two simultaneous submissions of
        // one code cannot both win.
        let db = test_db("codes-consume").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        assert!(db.consume(code.id, at(10)).await.unwrap(), "the first use succeeds");
        assert!(!db.consume(code.id, at(10)).await.unwrap(), "the replay must be refused");
        assert!(db.find_active_for_user(user_id, at(10)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_only_one_of_many_concurrent_consumptions_wins() {
        let db = test_db("codes-consume-race").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        let mut handles = Vec::new();
        for _ in 0..12 {
            let db = db.clone();
            let id = code.id;
            handles.push(tokio::spawn(async move { db.consume(id, at(10)).await }));
        }
        let mut wins = 0;
        for handle in handles {
            if handle.await.unwrap().unwrap() {
                wins += 1;
            }
        }
        assert_eq!(wins, 1, "exactly one consumption may succeed");
    }

    #[tokio::test]
    async fn test_record_attempt_counts_up_from_the_stored_value() {
        // REQ-6.4's three strikes per code.
        let db = test_db("codes-attempts").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        assert_eq!(db.record_attempt(code.id).await.unwrap(), 1);
        assert_eq!(db.record_attempt(code.id).await.unwrap(), 2);
        assert_eq!(db.record_attempt(code.id).await.unwrap(), 3);
        assert_eq!(db.find_active_for_user(user_id, at(10)).await.unwrap().unwrap().attempts, 3);
    }

    #[tokio::test]
    async fn test_record_attempt_on_an_unknown_code_is_not_found() {
        let db = test_db("codes-attempts-missing").await;
        let error = db
            .record_attempt(Uuid::new_v4())
            .await
            .expect_err("an unknown code must not count");
        assert!(matches!(error, RepositoryError::NotFound { entity: "login code" }), "{error}");
    }

    #[tokio::test]
    async fn test_find_active_returns_the_newest_when_several_are_outstanding() {
        let db = test_db("codes-newest").await;
        let user_id = a_user(&db, "a@x.test").await;
        LoginCodeRepository::create(&db, &code(user_id, "111111", at(10), at(14)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(user_id, "222222", at(11), at(14)))
            .await
            .unwrap();

        let active = db.find_active_for_user(user_id, at(12)).await.unwrap().unwrap();
        assert_eq!(active.code, "222222");
    }

    #[tokio::test]
    async fn test_find_latest_sees_a_consumed_code_so_the_cooldown_cannot_be_reset() {
        // REQ-6.5. Measured from the last code issued: if the cooldown looked at
        // active codes only, using a code would clear it and allow an immediate
        // resend.
        let db = test_db("codes-latest").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();
        db.consume(code.id, at(10)).await.unwrap();

        let latest = db
            .find_latest_for_user(user_id)
            .await
            .unwrap()
            .expect("the consumed code is still the latest");
        assert_eq!(latest.created_at, at(10));
        assert_eq!(latest.consumed_at, Some(at(10)));
    }

    #[tokio::test]
    async fn test_count_issued_since_spans_every_user() {
        // The global daily cap. A per-user count would miss the loop across
        // addresses that exhausts the relay quota and locks everyone out.
        let db = test_db("codes-daily-cap").await;
        let first = a_user(&db, "a@x.test").await;
        let second = a_user(&db, "b@x.test").await;
        LoginCodeRepository::create(&db, &code(first, "111111", at(9), at(10)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(second, "222222", at(10), at(11)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(second, "333333", at(11), at(12)))
            .await
            .unwrap();

        assert_eq!(db.count_issued_since(at(9)).await.unwrap(), 3);
        assert_eq!(
            db.count_issued_since(at(10)).await.unwrap(),
            2,
            "the window boundary is inclusive"
        );
        assert_eq!(db.count_issued_since(at(12)).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_delete_for_user_clears_that_users_codes_only() {
        let db = test_db("codes-delete-for-user").await;
        let first = a_user(&db, "a@x.test").await;
        let second = a_user(&db, "b@x.test").await;
        LoginCodeRepository::create(&db, &code(first, "111111", at(10), at(11)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(first, "222222", at(10), at(11)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(second, "333333", at(10), at(11)))
            .await
            .unwrap();

        assert_eq!(LoginCodeRepository::delete_for_user(&db, first).await.unwrap(), 2);
        assert_eq!(count(&db, "login_codes").await, 1);
        assert!(db.find_active_for_user(second, at(10)).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_leaves_live_codes_alone() {
        let db = test_db("codes-delete-expired").await;
        let user_id = a_user(&db, "a@x.test").await;
        LoginCodeRepository::create(&db, &code(user_id, "111111", at(9), at(10)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(user_id, "222222", at(11), at(14)))
            .await
            .unwrap();

        assert_eq!(LoginCodeRepository::delete_expired(&db, at(12)).await.unwrap(), 1);
        assert_eq!(count(&db, "login_codes").await, 1);
        assert_eq!(db.find_active_for_user(user_id, at(12)).await.unwrap().unwrap().code, "222222");
    }
}
