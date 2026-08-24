//! [`UserRepository`] against SQLite.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use editor_core::records::{Role, User};
use editor_core::repository::{Result, UserRepository};
use rusqlite::{params, Row, Transaction};
use uuid::Uuid;

use super::mapping::{counter, parsed_column, uuid_column};
use super::Database;

const ENTITY: &str = "user";

const SELECT: &str =
    "SELECT id, email, name, role, failed_logins, failed_login_at, last_code_at, created_at FROM users";

fn map_row(row: &Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: uuid_column(row, 0)?,
        email: row.get(1)?,
        name: row.get(2)?,
        role: parsed_column::<Role>(row, 3)?,
        // Filled in by the caller: the shortcodes live in a child table, and
        // joining them into this row would repeat every user column per
        // assignment.
        shortcodes: Vec::new(),
        failed_logins: counter(row.get::<_, i64>(4)?),
        failed_login_at: row.get(5)?,
        last_code_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

/// Read one user's shortcode assignments.
fn shortcodes_of(tx: &Transaction<'_>, user_id: Uuid) -> rusqlite::Result<Vec<String>> {
    let mut stmt = tx.prepare("SELECT shortcode FROM user_shortcodes WHERE user_id = ?1 ORDER BY shortcode")?;
    let rows = stmt.query_map(params![user_id.to_string()], |row| row.get(0))?;
    rows.collect()
}

/// Replace a user's shortcode assignments with `shortcodes`.
///
/// Delete-then-insert rather than a diff: the set is a handful of entries, and a
/// diff would have to be right about both directions to avoid leaving an
/// assignment behind.
fn replace_shortcodes(tx: &Transaction<'_>, user_id: Uuid, shortcodes: &[String]) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM user_shortcodes WHERE user_id = ?1", params![user_id.to_string()])?;
    let mut stmt = tx.prepare("INSERT INTO user_shortcodes (user_id, shortcode) VALUES (?1, ?2)")?;
    for shortcode in shortcodes {
        stmt.execute(params![user_id.to_string(), shortcode])?;
    }
    Ok(())
}

#[async_trait]
impl UserRepository for Database {
    async fn create(&self, user: &User) -> Result<()> {
        let user = user.clone();
        let email_normalized = user.email_normalized();
        self.write(move |tx| {
            tx.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, failed_logins, failed_login_at, \
                 last_code_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    user.id.to_string(),
                    user.email,
                    email_normalized,
                    user.name,
                    user.role.as_str(),
                    i64::from(user.failed_logins),
                    user.failed_login_at,
                    user.last_code_at,
                    user.created_at,
                ],
            )?;
            replace_shortcodes(tx, user.id, &user.shortcodes)
        })
        .await
        // `email_normalized` is the only unique index on this table, so a
        // constraint violation here is REQ-7.4's duplicate address.
        .map_err(|e| e.into_repository_error(ENTITY))
    }

    async fn update(&self, user: &User) -> Result<()> {
        let user = user.clone();
        let email_normalized = user.email_normalized();
        let updated = self
            .write(move |tx| {
                let updated = tx.execute(
                    "UPDATE users SET email = ?2, email_normalized = ?3, name = ?4, role = ?5 WHERE id = ?1",
                    params![
                        user.id.to_string(),
                        user.email,
                        email_normalized,
                        user.name,
                        user.role.as_str()
                    ],
                )?;
                if updated > 0 {
                    replace_shortcodes(tx, user.id, &user.shortcodes)?;
                }
                Ok(updated)
            })
            .await
            .map_err(|e| e.into_repository_error(ENTITY))?;

        // Deliberately not silent. An update that matched nothing means the
        // account was removed underneath the form, and reporting success would
        // tell RDU the change landed.
        if updated == 0 {
            return Err(editor_core::repository::RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        // Sessions, codes and shortcode assignments go with it via
        // `ON DELETE CASCADE` (REQ-7.5) — which only fires because
        // `PRAGMA foreign_keys` is set per connection outside any transaction.
        let deleted = self
            .write(move |tx| tx.execute("DELETE FROM users WHERE id = ?1", params![id.to_string()]))
            .await?;
        if deleted == 0 {
            return Err(editor_core::repository::RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        // `read_tx`, not `read`: the shortcodes come from a second query and both
        // have to see one snapshot, or an assignment change landing between them
        // returns a user with shortcodes it no longer has. Not `write` either —
        // every authenticated request looks a user up, and routing those through
        // the single writer connection would serialise all of them.
        Ok(self
            .read_tx(move |tx| {
                let mut stmt = tx.prepare(&format!("{SELECT} WHERE id = ?1"))?;
                let mut rows = stmt.query_map(params![id.to_string()], map_row)?;
                let Some(user) = rows.next().transpose()? else {
                    return Ok(None);
                };
                drop(rows);
                drop(stmt);
                Ok(Some(User { shortcodes: shortcodes_of(tx, user.id)?, ..user }))
            })
            .await?)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        // Normalized here, so callers pass whatever the user typed and REQ-6.2's
        // anti-enumeration lookup cannot be defeated by capitalisation.
        let normalized = User::normalize_email(email);
        Ok(self
            .read_tx(move |tx| {
                let mut stmt = tx.prepare(&format!("{SELECT} WHERE email_normalized = ?1"))?;
                let mut rows = stmt.query_map(params![normalized], map_row)?;
                let Some(user) = rows.next().transpose()? else {
                    return Ok(None);
                };
                drop(rows);
                drop(stmt);
                Ok(Some(User { shortcodes: shortcodes_of(tx, user.id)?, ..user }))
            })
            .await?)
    }

    async fn list(&self) -> Result<Vec<User>> {
        Ok(self
            .read_tx(|tx| {
                let mut users = {
                    let mut stmt = tx.prepare(&format!("{SELECT} ORDER BY email_normalized"))?;
                    let rows = stmt.query_map([], map_row)?;
                    rows.collect::<rusqlite::Result<Vec<User>>>()?
                };
                // One query for every assignment rather than one per user, so
                // the depositor list does not scale in queries with its length.
                let mut stmt = tx.prepare("SELECT user_id, shortcode FROM user_shortcodes ORDER BY shortcode")?;
                let assignments = stmt
                    .query_map([], |row| Ok((uuid_column(row, 0)?, row.get::<_, String>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                for (user_id, shortcode) in assignments {
                    if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                        user.shortcodes.push(shortcode);
                    }
                }
                Ok(users)
            })
            .await?)
    }

    async fn record_failed_login(&self, id: Uuid, at: DateTime<Utc>, decay_before: DateTime<Utc>) -> Result<u32> {
        // Incremented and read back in one transaction, so two simultaneous wrong
        // codes cannot both read the old value and count as one.
        //
        // The CASE is the rolling window. Without it the counter only ever rises,
        // so an account that has once reached its cap is re-locked by a *single*
        // wrong entry after every lockout expires — a permanent denial of service
        // against any address an attacker knows is registered, for one request
        // per window. A failure whose predecessor has aged out starts over at one.
        let failed = self
            .write(move |tx| {
                let updated = tx.execute(
                    "UPDATE users SET failed_logins = CASE \
                       WHEN failed_login_at IS NULL OR failed_login_at <= ?3 THEN 1 \
                       ELSE failed_logins + 1 END, \
                     failed_login_at = ?2 WHERE id = ?1",
                    params![id.to_string(), at, decay_before],
                )?;
                if updated == 0 {
                    return Ok(None);
                }
                let count: i64 = tx.query_row(
                    "SELECT failed_logins FROM users WHERE id = ?1",
                    params![id.to_string()],
                    |row| row.get(0),
                )?;
                Ok(Some(counter(count)))
            })
            .await?;
        failed.ok_or(editor_core::repository::RepositoryError::NotFound { entity: ENTITY })
    }

    async fn clear_failed_logins(&self, id: Uuid) -> Result<()> {
        let cleared = self
            .write(move |tx| {
                tx.execute(
                    "UPDATE users SET failed_logins = 0, failed_login_at = NULL WHERE id = ?1",
                    params![id.to_string()],
                )
            })
            .await?;
        if cleared == 0 {
            return Err(editor_core::repository::RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }

    async fn record_code_issued(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        let updated = self
            .write(move |tx| {
                tx.execute("UPDATE users SET last_code_at = ?2 WHERE id = ?1", params![id.to_string(), at])
            })
            .await?;
        if updated == 0 {
            return Err(editor_core::repository::RepositoryError::NotFound { entity: ENTITY });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use editor_core::repository::RepositoryError;

    use super::super::tests::{count, test_db};
    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 21, hour, 0, 0).unwrap()
    }

    fn depositor(email: &str, shortcodes: &[&str]) -> User {
        User {
            id: Uuid::new_v4(),
            email: email.to_string(),
            name: "A Depositor".to_string(),
            role: Role::Depositor,
            shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: at(10),
        }
    }

    #[tokio::test]
    async fn test_create_then_find_round_trips_every_field() {
        let db = test_db("users-round-trip").await;
        let user = depositor("A.User@Example.TEST", &["0801", "0803"]);
        db.create(&user).await.unwrap();

        let found = db.find_by_id(user.id).await.unwrap().expect("the user should be found");
        assert_eq!(found, user, "every field, shortcodes included, must survive the round trip");
    }

    #[tokio::test]
    async fn test_create_rejects_a_duplicate_address_case_insensitively() {
        // REQ-7.4. Folding case matters: without it `A@x.test` would be accepted
        // alongside `a@x.test` and both would receive login codes.
        let db = test_db("users-duplicate").await;
        db.create(&depositor("a@x.test", &[])).await.unwrap();

        let error = db
            .create(&depositor("A@X.test", &[]))
            .await
            .expect_err("a duplicate must be rejected");
        assert!(matches!(error, RepositoryError::Conflict { entity: "user" }), "{error}");
        assert_eq!(count(&db, "users").await, 1);
    }

    #[tokio::test]
    async fn test_find_by_email_ignores_case_and_surrounding_space() {
        let db = test_db("users-find-email").await;
        let user = depositor("a.user@x.test", &["0801"]);
        db.create(&user).await.unwrap();

        for typed in ["a.user@x.test", "A.User@X.TEST", "  a.user@x.test  "] {
            let found = db.find_by_email(typed).await.unwrap();
            assert_eq!(found.map(|u| u.id), Some(user.id), "lookup of {typed:?} should match");
        }
        assert!(db.find_by_email("nobody@x.test").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_replaces_shortcodes_rather_than_adding_to_them() {
        // Removing an assignment has to actually remove it — a depositor who
        // keeps a shortcode after RDU takes it away keeps access to that project.
        let db = test_db("users-update").await;
        let mut user = depositor("a@x.test", &["0801", "0803"]);
        db.create(&user).await.unwrap();

        user.name = "Renamed".to_string();
        user.shortcodes = vec!["0803".to_string(), "0805".to_string()];
        db.update(&user).await.unwrap();

        let found = db.find_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Renamed");
        assert_eq!(found.shortcodes, vec!["0803".to_string(), "0805".to_string()]);
    }

    #[tokio::test]
    async fn test_update_of_an_unknown_user_is_not_found_not_success() {
        let db = test_db("users-update-missing").await;
        let error = db
            .update(&depositor("a@x.test", &[]))
            .await
            .expect_err("an unknown user must not update");
        assert!(matches!(error, RepositoryError::NotFound { entity: "user" }), "{error}");
    }

    #[tokio::test]
    async fn test_delete_takes_sessions_codes_and_assignments_with_it() {
        // REQ-7.5, and the observable proof that `PRAGMA foreign_keys` is on:
        // without it the cascade silently does nothing and orphaned sessions
        // accumulate against a deleted account.
        let db = test_db("users-delete-cascade").await;
        let user = depositor("a@x.test", &["0801"]);
        db.create(&user).await.unwrap();
        let user_id = user.id.to_string();
        db.write(move |tx| {
            tx.execute(
                "INSERT INTO sessions (id, user_id, created_at, last_seen_at, expires_at) \
                 VALUES ('s1', ?1, ?2, ?2, ?3)",
                params![user_id, at(10), at(12)],
            )?;
            tx.execute(
                "INSERT INTO login_codes (id, user_id, code, created_at, expires_at) VALUES (?1, ?2, '123456', ?3, ?4)",
                params![Uuid::new_v4().to_string(), user_id, at(10), at(11)],
            )
        })
        .await
        .unwrap();

        db.delete(user.id).await.unwrap();

        assert_eq!(count(&db, "users").await, 0);
        assert_eq!(count(&db, "sessions").await, 0, "sessions must cascade");
        assert_eq!(count(&db, "login_codes").await, 0, "codes must cascade");
        assert_eq!(count(&db, "user_shortcodes").await, 0, "assignments must cascade");
    }

    #[tokio::test]
    async fn test_delete_leaves_drafts_and_submissions_with_a_null_author() {
        // ON DELETE SET NULL, not CASCADE: removing an account must not destroy
        // the project's work, and "last editor" then reads as unknown rather
        // than pointing at a row that is gone.
        let db = test_db("users-delete-set-null").await;
        let user = depositor("a@x.test", &[]);
        db.create(&user).await.unwrap();
        let user_id = user.id.to_string();
        db.write(move |tx| {
            tx.execute(
                "INSERT INTO drafts (shortcode, payload, updated_by, created_at, updated_at) \
                 VALUES ('0801', '{}', ?1, ?2, ?2)",
                params![user_id, at(10)],
            )
        })
        .await
        .unwrap();

        db.delete(user.id).await.unwrap();

        let author: Option<String> = db
            .read(|conn| conn.query_row("SELECT updated_by FROM drafts WHERE shortcode = '0801'", [], |row| row.get(0)))
            .await
            .unwrap();
        assert_eq!(author, None);
        assert_eq!(count(&db, "drafts").await, 1, "the draft itself must survive");
    }

    #[tokio::test]
    async fn test_failed_login_counter_survives_and_only_a_success_clears_it() {
        // NIST SP 800-63B-4: "Generating a new authentication secret SHALL NOT
        // reset the failed authentication count." The counter lives on the user
        // for exactly this reason — a per-code counter hands out a fresh budget
        // on every resend.
        let db = test_db("users-failed-logins").await;
        let user = depositor("a@x.test", &[]);
        db.create(&user).await.unwrap();

        assert_eq!(db.record_failed_login(user.id, at(10), at(9)).await.unwrap(), 1);
        assert_eq!(db.record_failed_login(user.id, at(11), at(9)).await.unwrap(), 2);

        // Issuing a new code must not touch it.
        db.record_code_issued(user.id, at(11)).await.unwrap();
        let after_resend = db.find_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(after_resend.failed_logins, 2);
        // The instant tracks the newest failure, so a lockout window is measured
        // from the last attempt rather than the first.
        assert_eq!(after_resend.failed_login_at, Some(at(11)));

        db.clear_failed_logins(user.id).await.unwrap();
        let after_success = db.find_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(after_success.failed_logins, 0);
        // Cleared with the counter: a stale instant left behind would keep a
        // freshly successful account inside a lockout window.
        assert_eq!(after_success.failed_login_at, None);
    }

    #[tokio::test]
    async fn test_a_failure_after_the_window_starts_the_count_over() {
        // Otherwise the counter is a ratchet: an account that once reached its
        // cap is re-locked by one wrong entry after every lockout expires, so an
        // attacker keeps a known address locked out forever for one request per
        // window.
        let db = test_db("users-failed-decay").await;
        let user = depositor("a@x.test", &[]);
        db.create(&user).await.unwrap();

        // Three failures inside the window accumulate. `decay_before` stays well
        // behind the stored instant, so none of them is on the boundary.
        assert_eq!(db.record_failed_login(user.id, at(10), at(9)).await.unwrap(), 1);
        assert_eq!(db.record_failed_login(user.id, at(10), at(9)).await.unwrap(), 2);
        assert_eq!(db.record_failed_login(user.id, at(11), at(9)).await.unwrap(), 3);

        // A failure whose predecessor has aged out starts over, so re-locking
        // costs a full budget again rather than one attempt.
        assert_eq!(db.record_failed_login(user.id, at(14), at(13)).await.unwrap(), 1);

        // Exactly on the boundary counts as aged out, matching `locked_out`,
        // which treats the window as over at `failed_login_at + lockout`. The
        // previous call stamped `at(14)`, so this one sits on it precisely.
        assert_eq!(db.record_failed_login(user.id, at(15), at(14)).await.unwrap(), 1);
        assert_eq!(
            db.find_by_id(user.id).await.unwrap().unwrap().failed_login_at,
            Some(at(15)),
            "and the instant always advances to the newest failure"
        );
    }

    #[tokio::test]
    async fn test_concurrent_failed_logins_are_all_counted() {
        // Read-modify-write in one transaction. Two simultaneous wrong codes
        // must count as two, or the lockout can be outrun by parallel guessing.
        let db = test_db("users-failed-concurrent").await;
        let user = depositor("a@x.test", &[]);
        db.create(&user).await.unwrap();

        let mut handles = Vec::new();
        for _ in 0..16 {
            let db = db.clone();
            let id = user.id;
            handles.push(tokio::spawn(async move { db.record_failed_login(id, at(10), at(9)).await }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert_eq!(db.find_by_id(user.id).await.unwrap().unwrap().failed_logins, 16);
    }

    #[tokio::test]
    async fn test_record_code_issued_stamps_the_time_for_rdu_diagnosis() {
        let db = test_db("users-last-code").await;
        let user = depositor("a@x.test", &[]);
        db.create(&user).await.unwrap();
        assert_eq!(db.find_by_id(user.id).await.unwrap().unwrap().last_code_at, None);

        db.record_code_issued(user.id, at(11)).await.unwrap();
        assert_eq!(db.find_by_id(user.id).await.unwrap().unwrap().last_code_at, Some(at(11)));
    }

    #[tokio::test]
    async fn test_list_returns_every_user_with_its_own_assignments() {
        let db = test_db("users-list").await;
        db.create(&depositor("b@x.test", &["0803"])).await.unwrap();
        db.create(&depositor("a@x.test", &["0801", "0802"])).await.unwrap();
        db.create(&User { role: Role::Rdu, ..depositor("c@x.test", &[]) })
            .await
            .unwrap();

        let users = db.list().await.unwrap();
        let listed: Vec<_> = users.iter().map(|u| (u.email.as_str(), u.shortcodes.clone())).collect();
        assert_eq!(
            listed,
            vec![
                ("a@x.test", vec!["0801".to_string(), "0802".to_string()]),
                ("b@x.test", vec!["0803".to_string()]),
                ("c@x.test", vec![]),
            ],
            "assignments must not leak between users"
        );
    }
}
