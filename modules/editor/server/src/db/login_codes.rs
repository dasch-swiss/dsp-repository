//! [`LoginCodeRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::records::LoginCode;
use editor_core::repository::{Attempt, Issued, LoginCodeRepository, Result};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::mapping::{counter, uuid_column, OptionalRow};
use super::Database;

const ENTITY: &str = "login code";

const SELECT: &str =
    "SELECT id, user_id, code, attempts, created_at, expires_at, consumed_at, browser_token FROM login_codes";

fn map_row(row: &Row<'_>) -> rusqlite::Result<LoginCode> {
    Ok(LoginCode {
        id: uuid_column(row, 0)?,
        user_id: uuid_column(row, 1)?,
        code: row.get(2)?,
        attempts: counter(row.get::<_, i64>(3)?),
        created_at: row.get(4)?,
        expires_at: row.get(5)?,
        consumed_at: row.get(6)?,
        browser_token: row.get(7)?,
    })
}

/// The insert, shared by [`LoginCodeRepository::create`] and the
/// compare-and-set in [`LoginCodeRepository::create_unless_issued_since`], so
/// the two cannot drift into writing different column sets.
fn insert(tx: &rusqlite::Transaction<'_>, code: &LoginCode) -> rusqlite::Result<usize> {
    tx.execute(
        "INSERT INTO login_codes (id, user_id, code, attempts, created_at, expires_at, consumed_at, browser_token) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            code.id.to_string(),
            code.user_id.to_string(),
            code.code,
            i64::from(code.attempts),
            code.created_at,
            code.expires_at,
            code.consumed_at,
            code.browser_token,
        ],
    )
}

#[async_trait]
impl LoginCodeRepository for Database {
    async fn create(&self, code: &LoginCode) -> Result<()> {
        let code = code.clone();
        self.write(move |tx| insert(tx, &code))
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(())
    }

    async fn create_unless_issued_since(&self, code: &LoginCode, not_before: DateTime<Utc>) -> Result<Issued> {
        // The whole decision happens inside one `BEGIN IMMEDIATE`, which is what
        // makes it a compare-and-set: checking through `read` and then inserting
        // would let two simultaneous requests for one address both see no recent
        // code and both send.
        let code = code.clone();
        let outcome = self
            .write(move |tx| {
                let recent: Option<i64> = tx
                    .query_row(
                        "SELECT 1 FROM login_codes WHERE user_id = ?1 AND created_at >= ?2 LIMIT 1",
                        params![code.user_id.to_string(), not_before],
                        |row| row.get(0),
                    )
                    .optional_row()?;
                if recent.is_some() {
                    return Ok(Issued::Cooled);
                }
                insert(tx, &code)?;
                Ok(Issued::New)
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;
        Ok(outcome)
    }

    async fn find_by_browser_token(&self, token: &str) -> Result<Option<LoginCode>> {
        let token = token.to_string();
        Ok(self
            .read(move |conn| {
                conn.query_row(&format!("{SELECT} WHERE browser_token = ?1"), params![token], map_row)
                    .optional_row()
            })
            .await?)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM login_codes WHERE id = ?1", params![id.to_string()]))
            .await?;
        Ok(deleted > 0)
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

    async fn claim_attempt(&self, id: Uuid, max_attempts: u32) -> Result<Attempt> {
        // One statement, on the writer connection, so the check and the
        // increment cannot be separated. `attempts < ?2` in the WHERE clause is
        // REQ-6.4's three strikes: the fourth simultaneous guess updates zero
        // rows and never reaches a comparison. A read followed by an update
        // would let every request in flight pass the check at once, which is the
        // whole limit gone.
        //
        // `consumed_at IS NULL` is here for the same reason it is in `consume`:
        // a code being spent in a parallel request must not also be guessable.
        //
        // When it refuses, a second read inside the same transaction says which
        // predicate did it — the UPDATE itself cannot. That read is on the
        // failure path only, and it is what keeps "you have used your three
        // guesses" apart from "another tab just spent this code".
        let outcome = self
            .write(move |tx| {
                let claimed = tx.execute(
                    "UPDATE login_codes SET attempts = attempts + 1 \
                     WHERE id = ?1 AND consumed_at IS NULL AND attempts < ?2",
                    params![id.to_string(), i64::from(max_attempts)],
                )?;
                if claimed > 0 {
                    return Ok(Attempt::Claimed);
                }
                let refused: Option<(Option<String>, i64)> = tx
                    .query_row(
                        "SELECT consumed_at, attempts FROM login_codes WHERE id = ?1",
                        params![id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional_row()?;
                Ok(match refused {
                    None => Attempt::Unknown,
                    Some((Some(_), _)) => Attempt::AlreadySpent,
                    Some((None, _)) => Attempt::Exhausted,
                })
            })
            .await?;
        Ok(outcome)
    }

    async fn rebind_browser_token(&self, presented: &str, replacement: &str) -> Result<bool> {
        // Holding `presented` is the authorisation — it is a 256-bit token that
        // was handed to exactly one browser — so this cannot move a binding the
        // caller does not already have.
        let presented = presented.to_string();
        let replacement = replacement.to_string();
        let moved = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE login_codes SET browser_token = ?2 WHERE browser_token = ?1 AND consumed_at IS NULL",
                    params![presented, replacement],
                )
            })
            .await?;
        Ok(moved > 0)
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

    async fn delete_unconsumed_for_user(&self, user_id: Uuid) -> Result<u64> {
        // `consumed_at IS NULL` is the whole point: the spent code stays, because
        // it is the resend cooldown's only anchor — REQ-6.5 measures from the
        // last code issued, and that row is the only thing recording it. The
        // send caps do not read this table at all; they count `mail_sends`,
        // which is why deleting the mailed siblings below no longer hides them.
        let deleted = self
            .write(move |tx| {
                tx.execute(
                    "DELETE FROM login_codes WHERE user_id = ?1 AND consumed_at IS NULL",
                    params![user_id.to_string()],
                )
            })
            .await?;
        Ok(deleted as u64)
    }

    async fn unconsume(&self, id: Uuid) -> Result<bool> {
        let reopened = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE login_codes SET consumed_at = NULL WHERE id = ?1 AND consumed_at IS NOT NULL",
                    params![id.to_string()],
                )
            })
            .await?;
        Ok(reopened > 0)
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
    use editor_core::repository::{RepositoryError, UserRepository};

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

    fn code(user_id: Uuid, digits: &str, created: DateTime<Utc>, expires: DateTime<Utc>) -> LoginCode {
        LoginCode {
            id: Uuid::new_v4(),
            user_id,
            code: digits.to_string(),
            attempts: 0,
            created_at: created,
            expires_at: expires,
            consumed_at: None,
            // Unique per code, because the column is: two fixtures sharing a
            // token would fail the insert rather than the assertion.
            browser_token: Some(format!("token-{}", Uuid::new_v4())),
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
    async fn test_claim_attempt_hands_out_exactly_the_allowed_number() {
        // REQ-6.4's three strikes per code.
        let db = test_db("codes-attempts").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        for expected in 1..=3 {
            assert_eq!(
                db.claim_attempt(code.id, 3).await.unwrap(),
                Attempt::Claimed,
                "attempt {expected} must be allowed"
            );
        }
        assert_eq!(
            db.claim_attempt(code.id, 3).await.unwrap(),
            Attempt::Exhausted,
            "the fourth must not be, and must say why"
        );
        assert_eq!(
            db.find_active_for_user(user_id, at(10)).await.unwrap().unwrap().attempts,
            3,
            "a refused claim must not count either, or the code would keep aging"
        );
    }

    #[tokio::test]
    async fn test_concurrent_claims_cannot_exceed_the_limit() {
        // The property the single statement exists for. A read of `attempts`
        // followed by an update would let all twenty pass the check at once.
        let db = test_db("codes-attempts-race").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        let mut handles = Vec::new();
        for _ in 0..20 {
            let db = db.clone();
            let id = code.id;
            handles.push(tokio::spawn(async move { db.claim_attempt(id, 3).await }));
        }
        let mut granted = 0;
        for handle in handles {
            if handle.await.unwrap().unwrap() == Attempt::Claimed {
                granted += 1;
            }
        }
        assert_eq!(granted, 3, "exactly three of twenty simultaneous claims may be granted");
        assert_eq!(db.find_active_for_user(user_id, at(10)).await.unwrap().unwrap().attempts, 3);
    }

    #[tokio::test]
    async fn test_a_consumed_code_grants_no_further_attempts() {
        let db = test_db("codes-attempts-consumed").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();
        db.consume(code.id, at(10)).await.unwrap();

        // Distinguished from `Exhausted`: this is usually one person with two
        // tabs open, not somebody who mistyped three times.
        assert_eq!(db.claim_attempt(code.id, 3).await.unwrap(), Attempt::AlreadySpent);
    }

    #[tokio::test]
    async fn test_claim_attempt_on_an_unknown_code_grants_nothing() {
        let db = test_db("codes-attempts-missing").await;
        assert_eq!(db.claim_attempt(Uuid::new_v4(), 3).await.unwrap(), Attempt::Unknown);
    }

    #[tokio::test]
    async fn test_rebind_moves_a_binding_only_for_the_browser_that_holds_it() {
        // The `WHERE browser_token = presented` is the authorisation. Without it
        // anyone able to post an address could take over the binding of a code
        // already on its way to that address's owner.
        let db = test_db("codes-rebind").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();
        let held = code.browser_token.clone().unwrap();

        assert!(!db.rebind_browser_token("a-token-nobody-holds", "replacement").await.unwrap());
        assert_eq!(
            db.find_by_browser_token(&held).await.unwrap().map(|c| c.id),
            Some(code.id),
            "a failed rebind must leave the original binding alone"
        );

        assert!(db.rebind_browser_token(&held, "replacement").await.unwrap());
        assert_eq!(
            db.find_by_browser_token("replacement").await.unwrap().map(|c| c.id),
            Some(code.id)
        );
        assert_eq!(
            db.find_by_browser_token(&held).await.unwrap(),
            None,
            "the old token stops working"
        );
    }

    #[tokio::test]
    async fn test_a_consumed_code_cannot_be_rebound() {
        // Nothing useful, but a spent code should not follow a browser around.
        let db = test_db("codes-rebind-consumed").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();
        let held = code.browser_token.clone().unwrap();
        db.consume(code.id, at(10)).await.unwrap();

        assert!(!db.rebind_browser_token(&held, "replacement").await.unwrap());
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
    async fn test_create_unless_issued_since_inserts_when_the_cooldown_has_passed() {
        let db = test_db("codes-cas-free").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(12), at(13));

        // Nothing issued at or after 11:00, so the cooldown does not apply.
        assert_eq!(db.create_unless_issued_since(&code, at(11)).await.unwrap(), Issued::New);
        assert_eq!(db.find_active_for_user(user_id, at(12)).await.unwrap(), Some(code));
    }

    #[tokio::test]
    async fn test_create_unless_issued_since_refuses_and_names_the_live_code() {
        // REQ-6.5. Nothing comes back but the refusal: handing the caller the
        // outstanding code, or its binding, would let anyone able to post an
        // address obtain the binding of a code already on its way to that
        // address's owner.
        let db = test_db("codes-cas-cooled").await;
        let user_id = a_user(&db, "a@x.test").await;
        let first = code(user_id, "111111", at(11), at(12));
        LoginCodeRepository::create(&db, &first).await.unwrap();

        let second = code(user_id, "222222", at(11), at(12));
        assert_eq!(db.create_unless_issued_since(&second, at(10)).await.unwrap(), Issued::Cooled);
        assert_eq!(count(&db, "login_codes").await, 1, "the refused code must not be stored");
        assert_eq!(
            db.find_active_for_user(user_id, at(11)).await.unwrap().unwrap().code,
            "111111",
            "the code already on its way is the one that stays outstanding"
        );
    }

    #[tokio::test]
    async fn test_create_unless_issued_since_treats_the_window_edge_as_inside_it() {
        // `>=`, so a code issued exactly at the boundary still blocks. Off by one
        // in the other direction would let one extra mail out per cooldown.
        let db = test_db("codes-cas-edge").await;
        let user_id = a_user(&db, "a@x.test").await;
        let first = code(user_id, "111111", at(11), at(12));
        LoginCodeRepository::create(&db, &first).await.unwrap();

        let second = code(user_id, "222222", at(11), at(12));
        assert_eq!(db.create_unless_issued_since(&second, at(11)).await.unwrap(), Issued::Cooled);
        assert_eq!(db.create_unless_issued_since(&second, at(12)).await.unwrap(), Issued::New);
    }

    #[tokio::test]
    async fn test_only_one_of_many_concurrent_issues_wins_the_cooldown() {
        // The reason this is one transaction rather than a read then a create:
        // reads go to the reader pool, so a check-then-insert would let several
        // simultaneous requests for one address all see no recent code, all
        // insert, and all send.
        let db = test_db("codes-cas-race").await;
        let user_id = a_user(&db, "a@x.test").await;

        let mut handles = Vec::new();
        for n in 0..12 {
            let db = db.clone();
            let candidate = code(user_id, &format!("{n:06}"), at(11), at(12));
            handles.push(tokio::spawn(
                async move { db.create_unless_issued_since(&candidate, at(10)).await },
            ));
        }
        let mut issued = 0;
        for handle in handles {
            if handle.await.unwrap().unwrap() == Issued::New {
                issued += 1;
            }
        }
        assert_eq!(issued, 1, "exactly one code may be issued inside one cooldown");
        assert_eq!(count(&db, "login_codes").await, 1);
    }

    #[tokio::test]
    async fn test_find_by_browser_token_binds_a_code_to_one_browser() {
        let db = test_db("codes-token").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();
        let token = code.browser_token.clone().expect("the fixture binds a token");

        assert_eq!(db.find_by_browser_token(&token).await.unwrap(), Some(code));
        assert_eq!(
            db.find_by_browser_token("some-other-token").await.unwrap(),
            None,
            "a token that matches nothing is the ordinary case for an unknown address"
        );
    }

    #[tokio::test]
    async fn test_an_unbound_code_is_reachable_by_no_token_at_all() {
        // `browser_token` is nullable so that a row predating the column binds to
        // no browser rather than to the empty one. SQL equality never matches
        // NULL, which is what makes that fail closed.
        let db = test_db("codes-token-null").await;
        let user_id = a_user(&db, "a@x.test").await;
        let unbound = LoginCode {
            browser_token: None,
            ..code(user_id, "123456", at(10), at(11))
        };
        LoginCodeRepository::create(&db, &unbound).await.unwrap();

        assert_eq!(db.find_by_browser_token("").await.unwrap(), None);
        assert_eq!(db.find_by_browser_token("NULL").await.unwrap(), None);
        assert!(
            db.find_active_for_user(user_id, at(10)).await.unwrap().is_some(),
            "the row is there — it is simply unreachable by token"
        );
    }

    #[tokio::test]
    async fn test_two_codes_cannot_share_a_browser_token() {
        // The unique index is what makes one token address one code.
        let db = test_db("codes-token-unique").await;
        let user_id = a_user(&db, "a@x.test").await;
        let first = LoginCode {
            browser_token: Some("shared".to_string()),
            ..code(user_id, "111111", at(10), at(11))
        };
        let second = LoginCode {
            browser_token: Some("shared".to_string()),
            ..code(user_id, "222222", at(10), at(11))
        };
        LoginCodeRepository::create(&db, &first).await.unwrap();

        let error = LoginCodeRepository::create(&db, &second)
            .await
            .expect_err("a duplicate browser token must be refused");
        assert!(matches!(error, RepositoryError::Conflict { entity: "login code" }), "{error}");
    }

    #[tokio::test]
    async fn test_delete_rolls_back_a_single_code_and_its_cooldown() {
        // The rollback for a send that failed after the code was reserved:
        // without it the user waits out a cooldown for a code they never got.
        let db = test_db("codes-delete-one").await;
        let user_id = a_user(&db, "a@x.test").await;
        let first = code(user_id, "111111", at(10), at(11));
        let second = code(user_id, "222222", at(10), at(11));
        LoginCodeRepository::create(&db, &first).await.unwrap();
        LoginCodeRepository::create(&db, &second).await.unwrap();

        assert!(LoginCodeRepository::delete(&db, first.id).await.unwrap());
        assert!(
            !LoginCodeRepository::delete(&db, first.id).await.unwrap(),
            "deleting it again is not an error, it is already gone"
        );
        assert_eq!(count(&db, "login_codes").await, 1);
        assert_eq!(db.find_active_for_user(user_id, at(10)).await.unwrap().unwrap().code, "222222");

        // And the cooldown goes with it: the deleted code no longer blocks.
        let replacement = code(user_id, "333333", at(10), at(11));
        assert_eq!(
            db.create_unless_issued_since(&replacement, at(10)).await.unwrap(),
            Issued::Cooled
        );
        LoginCodeRepository::delete(&db, second.id).await.unwrap();
        assert_eq!(db.create_unless_issued_since(&replacement, at(10)).await.unwrap(), Issued::New);
    }

    #[tokio::test]
    async fn test_delete_unconsumed_spares_the_spent_code_and_other_users() {
        // The spent code has to survive: it is the resend cooldown's only anchor
        // (REQ-6.5 measures from the last code issued). What the send caps count
        // is `mail_sends`, which this does not touch at all — which is the point
        // of the split, since the sibling deleted here was mailed.
        let db = test_db("codes-delete-unconsumed").await;
        let first = a_user(&db, "a@x.test").await;
        let second = a_user(&db, "b@x.test").await;
        let spent = code(first, "111111", at(10), at(11));
        LoginCodeRepository::create(&db, &spent).await.unwrap();
        LoginCodeRepository::create(&db, &code(first, "222222", at(10), at(11)))
            .await
            .unwrap();
        LoginCodeRepository::create(&db, &code(second, "333333", at(10), at(11)))
            .await
            .unwrap();
        db.consume(spent.id, at(10)).await.unwrap();

        assert_eq!(db.delete_unconsumed_for_user(first).await.unwrap(), 1);
        assert_eq!(count(&db, "login_codes").await, 2, "the spent code and the other user's remain");
        assert!(db.find_active_for_user(second, at(10)).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_unconsume_reopens_a_code_that_bought_nothing() {
        // The window between spending a correct code and having a session to
        // show for it. Nobody was authenticated, so reopening costs nothing.
        let db = test_db("codes-unconsume").await;
        let user_id = a_user(&db, "a@x.test").await;
        let code = code(user_id, "123456", at(10), at(11));
        LoginCodeRepository::create(&db, &code).await.unwrap();

        assert!(db.consume(code.id, at(10)).await.unwrap());
        assert!(db.find_active_for_user(user_id, at(10)).await.unwrap().is_none());

        assert!(db.unconsume(code.id).await.unwrap());
        assert_eq!(
            db.find_active_for_user(user_id, at(10)).await.unwrap().map(|c| c.id),
            Some(code.id)
        );
        assert!(!db.unconsume(code.id).await.unwrap(), "reopening a live code changes nothing");
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
