//! [`MailSendRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::repository::{MailSendRepository, Result};
use rusqlite::params;
use uuid::Uuid;

use super::mapping::row_count;
use super::Database;

#[async_trait]
impl MailSendRepository for Database {
    async fn record(&self, user_id: Uuid, sent_at: DateTime<Utc>) -> Result<()> {
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO mail_sends (user_id, sent_at) VALUES (?1, ?2)",
                params![user_id.to_string(), sent_at],
            )
        })
        .await?;
        Ok(())
    }

    async fn count_since(&self, since: DateTime<Utc>) -> Result<u64> {
        // Across all users, including sends whose account has since been
        // deleted: the relay quota they spent is spent either way.
        let counted: i64 = self
            .read(move |conn| {
                conn.query_row("SELECT count(*) FROM mail_sends WHERE sent_at >= ?1", params![since], |row| {
                    row.get(0)
                })
            })
            .await?;
        Ok(row_count(counted))
    }

    async fn count_for_user_since(&self, user_id: Uuid, since: DateTime<Utc>) -> Result<u64> {
        let counted: i64 = self
            .read(move |conn| {
                conn.query_row(
                    "SELECT count(*) FROM mail_sends WHERE user_id = ?1 AND sent_at >= ?2",
                    params![user_id.to_string(), since],
                    |row| row.get(0),
                )
            })
            .await?;
        Ok(row_count(counted))
    }

    async fn delete_before(&self, cutoff: DateTime<Utc>) -> Result<u64> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM mail_sends WHERE sent_at < ?1", params![cutoff]))
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
            failed_login_at: None,
            last_code_at: None,
            created_at: at(9),
        };
        UserRepository::create(db, &user).await.unwrap();
        user.id
    }

    #[tokio::test]
    async fn test_the_global_count_sees_every_users_sends_inside_the_window() {
        let db = test_db("sends-global-window").await;
        let one = a_user(&db, "one@x.test").await;
        let two = a_user(&db, "two@x.test").await;

        db.record(one, at(9)).await.unwrap();
        db.record(one, at(11)).await.unwrap();
        db.record(two, at(12)).await.unwrap();

        assert_eq!(db.count_since(at(9)).await.unwrap(), 3, "the window edge is inside it");
        assert_eq!(db.count_since(at(10)).await.unwrap(), 2, "and what predates it is not");
        assert_eq!(db.count_since(at(13)).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_the_per_account_count_sees_only_that_account() {
        // The whole point of the per-account cap: one address hammering the
        // endpoint must not be answerable only by the shared budget.
        let db = test_db("sends-per-account").await;
        let one = a_user(&db, "one@x.test").await;
        let two = a_user(&db, "two@x.test").await;

        db.record(one, at(10)).await.unwrap();
        db.record(one, at(11)).await.unwrap();
        db.record(two, at(11)).await.unwrap();

        assert_eq!(db.count_for_user_since(one, at(9)).await.unwrap(), 2);
        assert_eq!(db.count_for_user_since(two, at(9)).await.unwrap(), 1);
        assert_eq!(db.count_for_user_since(one, at(11)).await.unwrap(), 1, "and honours the window");
    }

    #[tokio::test]
    async fn test_a_send_is_recorded_once_per_call_and_never_replaces_another() {
        // Append-only, so two sends to one address at the same instant are two
        // rows. A key or an upsert here would collapse them and under-count.
        let db = test_db("sends-append-only").await;
        let user_id = a_user(&db, "a@x.test").await;

        db.record(user_id, at(10)).await.unwrap();
        db.record(user_id, at(10)).await.unwrap();

        assert_eq!(count(&db, "mail_sends").await, 2);
        assert_eq!(db.count_for_user_since(user_id, at(10)).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_the_prune_drops_what_aged_out_and_keeps_the_window() {
        let db = test_db("sends-prune").await;
        let user_id = a_user(&db, "a@x.test").await;
        db.record(user_id, at(9)).await.unwrap();
        db.record(user_id, at(10)).await.unwrap();
        db.record(user_id, at(11)).await.unwrap();

        assert_eq!(db.delete_before(at(10)).await.unwrap(), 1);

        // The cutoff itself survives, because the count that reads the same
        // instant includes it — a prune one row greedier than the count would
        // hand back budget that was spent.
        assert_eq!(db.count_since(at(10)).await.unwrap(), 2);
        assert_eq!(count(&db, "mail_sends").await, 2);
    }

    #[tokio::test]
    async fn test_deleting_an_account_leaves_its_sends_in_the_global_count() {
        // `ON DELETE SET NULL`, not CASCADE. The relay budget an account spent
        // is spent whether or not the account still exists, and cascading would
        // let deleting a depositor hand the attacker their budget back.
        let db = test_db("sends-survive-account-deletion").await;
        let user_id = a_user(&db, "a@x.test").await;
        db.record(user_id, at(10)).await.unwrap();

        UserRepository::delete(&db, user_id).await.unwrap();

        assert_eq!(db.count_since(at(9)).await.unwrap(), 1, "the send is still counted globally");
        assert_eq!(
            db.count_for_user_since(user_id, at(9)).await.unwrap(),
            0,
            "but no longer attributed to an account that is gone"
        );
    }
}
