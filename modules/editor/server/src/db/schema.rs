//! The schema, as a forward-only ordered statement list guarded by
//! `PRAGMA user_version`.
//!
//! No migration framework and no added dependency. What one would buy here is a
//! version table, checksums and down-migrations; what this needs is "run the
//! statements this database has not run yet", and SQLite already carries the
//! counter for it in its own header.
//!
//! The rules that keep it honest:
//!
//! - [`MIGRATIONS`] is **append-only**. A released entry is never edited — every database that
//!   already ran it would skip the edit, so the schema would differ by deployment age.
//! - Everything runs inside one `BEGIN IMMEDIATE` transaction, the `user_version` bump included, so
//!   a crash part-way leaves the database at the version it started from rather than half-migrated.
//!   `BEGIN IMMEDIATE` also makes two processes starting at once safe: the second waits and then
//!   finds nothing to do.
//! - `PRAGMA foreign_keys` is **not** touched here. It is a documented silent no-op inside a
//!   transaction, so a migration that set it would appear to work and leave every `ON DELETE
//!   CASCADE` unenforced. It belongs to the per-connection init hook in [`super::init_connection`],
//!   and this is the helper it would be tempting to share.

use super::{Database, DbError};

/// Ordered and append-only: index `i` is migration `i + 1`, and how many have
/// been applied is `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_initial.sql"),
    include_str!("migrations/0002_auth.sql"),
    include_str!("migrations/0003_mail_sends.sql"),
    include_str!("migrations/0004_review.sql"),
];

/// The version a fully migrated database reports.
pub(crate) const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

impl Database {
    /// Apply every migration this database has not run, and return how many ran.
    ///
    /// Zero means the schema was already current, which is the normal case on
    /// every restart after the first.
    pub(super) async fn migrate(&self) -> Result<u32, DbError> {
        self.write(|tx| {
            let current: u32 = tx.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?.max(0) as u32;

            // A database from a newer release. Continuing would run application
            // code against a schema it does not know, so stop — with the two
            // numbers in the message, because the cause is a rollback to an
            // older image and nothing about SQLite's own errors would say so.
            if current > SCHEMA_VERSION {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
                    Some(format!(
                        "the database is at schema version {current}, but this build only knows {SCHEMA_VERSION} — it \
                         was written by a newer release of editor-server"
                    )),
                ));
            }

            let mut applied = 0;
            for (index, statements) in MIGRATIONS.iter().enumerate().skip(current as usize) {
                let version = index as u32 + 1;
                tx.execute_batch(statements)?;
                // Not a bind parameter: PRAGMA values cannot be parameterised.
                // `version` is derived from a slice index, so there is nothing to
                // inject.
                tx.pragma_update(None, "user_version", version)?;
                applied += 1;
            }
            Ok(applied)
        })
        .await
    }

    /// The schema version this database reports.
    pub(crate) async fn schema_version(&self) -> Result<u32, DbError> {
        let version: i64 = self
            .read(|conn| conn.pragma_query_value(None, "user_version", |row| row.get(0)))
            .await?;
        Ok(version.max(0) as u32)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::tests::test_db;
    use super::*;
    use crate::db::Source;

    /// Every table the persistence layer is responsible for.
    const EXPECTED_TABLES: &[&str] = &[
        "approved_records",
        "drafts",
        "login_codes",
        "mail_sends",
        "sessions",
        "submissions",
        "user_shortcodes",
        "users",
    ];

    #[tokio::test]
    async fn test_migrations_apply_to_an_empty_database() {
        let db = test_db("migrate-empty").await;
        assert_eq!(db.schema_version().await.unwrap(), SCHEMA_VERSION);

        let mut tables: Vec<String> = db
            .read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )?;
                let rows = stmt.query_map([], |row| row.get(0))?;
                rows.collect()
            })
            .await
            .expect("reading the schema should succeed");
        tables.sort();
        assert_eq!(tables, EXPECTED_TABLES);
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent_on_re_run() {
        // The property `user_version` exists to give: a second run finds nothing
        // to do. Without the guard the CREATE TABLEs would fail on every restart
        // after the first.
        let db = test_db("migrate-idempotent").await;
        assert_eq!(db.migrate().await.unwrap(), 0, "a re-run must apply nothing");
        assert_eq!(db.migrate().await.unwrap(), 0);
        assert_eq!(db.schema_version().await.unwrap(), SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn test_open_applies_exactly_the_outstanding_migrations() {
        let db = test_db("migrate-count").await;
        // `open` already migrated, so the fresh count is observable only through
        // the version it left behind.
        assert_eq!(db.schema_version().await.unwrap(), SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn test_reopening_a_file_database_keeps_its_data_and_does_not_re_migrate() {
        // The restart path. Reopening must find the schema current and leave the
        // rows alone — a migration that re-ran would drop or duplicate them.
        let dir = std::env::temp_dir().join(format!("editor-db-reopen-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let first = Database::open(Source::Directory(dir.clone()), 2, Duration::from_secs(5))
            .await
            .expect("first open should succeed");
        first
            .write(|tx| {
                tx.execute(
                    "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                     VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', '2026-08-21 10:00:00+00:00')",
                    [],
                )
            })
            .await
            .expect("write should succeed");
        drop(first);

        let second = Database::open(Source::Directory(dir.clone()), 2, Duration::from_secs(5))
            .await
            .expect("reopen should succeed");
        assert_eq!(second.migrate().await.unwrap(), 0, "a reopened database must not re-migrate");
        let count: i64 = second
            .read(|conn| conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)))
            .await
            .expect("read should succeed");
        assert_eq!(count, 1);

        drop(second);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_a_database_at_the_previous_version_is_upgraded_in_place() {
        // The upgrade path, which the empty-database test cannot cover: a
        // database that already ran 0001 must gain 0002's columns without
        // losing its rows. Building it by hand rather than by rolling back,
        // because a released migration is never edited and there is nothing to
        // roll back with.
        let dir = std::env::temp_dir().join(format!("editor-db-upgrade-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("editor.sqlite3");
        {
            let conn = rusqlite::Connection::open(&file).expect("opening a raw connection should succeed");
            conn.execute_batch(MIGRATIONS[0]).expect("0001 should apply");
            conn.pragma_update(None, "user_version", 1u32).unwrap();
            conn.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                 VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', '2026-08-21 10:00:00+00:00')",
                [],
            )
            .expect("the pre-upgrade row should insert");
        }

        let db = Database::open(Source::Directory(dir.clone()), 2, Duration::from_secs(5))
            .await
            .expect("opening a version-1 database should upgrade it");
        assert_eq!(db.schema_version().await.unwrap(), SCHEMA_VERSION);
        assert_eq!(db.migrate().await.unwrap(), 0, "the upgrade must not run twice");

        // The row survived, and the columns 0002 added read as unset on it —
        // which is the fail-closed value for both: no lockout in progress, and
        // no browser bound.
        let (users, unset_failed_at): (i64, i64) = db
            .read(|conn| {
                conn.query_row("SELECT count(*), sum(failed_login_at IS NULL) FROM users", [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
            })
            .await
            .expect("reading the upgraded table should succeed");
        assert_eq!((users, unset_failed_at), (1, 1));

        drop(db);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_a_database_from_a_newer_release_is_refused() {
        // The rollback case: an older image against a newer schema. Running the
        // application anyway would query columns that do not exist yet, one
        // handler at a time; failing at startup makes it one clear error.
        let db = test_db("migrate-future").await;
        db.write(|tx| tx.pragma_update(None, "user_version", SCHEMA_VERSION + 5))
            .await
            .expect("bumping the version should succeed");

        let error = db
            .migrate()
            .await
            .expect_err("a future schema version must be refused")
            .to_string();
        assert!(error.contains("newer release"), "{error}");
    }

    #[tokio::test]
    async fn test_timestamp_columns_order_chronologically_as_text() {
        // Timestamps are TEXT, so `expires_at > ?` is a string comparison.
        // rusqlite's chrono format is fixed-width and always UTC, which is what
        // makes that comparison chronological — a mixed-offset or variable-width
        // format would sort wrongly and silently accept expired sessions.
        use chrono::{TimeZone, Utc};

        let db = test_db("timestamp-order").await;
        let base = Utc.with_ymd_and_hms(2026, 8, 21, 10, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 8, 21, 11, 0, 0).unwrap();
        let much_later = Utc.with_ymd_and_hms(2026, 12, 1, 9, 0, 0).unwrap();

        db.write(move |tx| {
            tx.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                 VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', ?1)",
                rusqlite::params![base],
            )?;
            for (id, expires) in [("s1", base), ("s2", later), ("s3", much_later)] {
                tx.execute(
                    "INSERT INTO sessions (id, user_id, created_at, last_seen_at, expires_at) \
                     VALUES (?1, 'u1', ?2, ?2, ?3)",
                    rusqlite::params![id, base, expires],
                )?;
            }
            Ok(())
        })
        .await
        .expect("write should succeed");

        let live: Vec<String> = db
            .read(move |conn| {
                let mut stmt = conn.prepare("SELECT id FROM sessions WHERE expires_at > ?1 ORDER BY expires_at")?;
                let rows = stmt.query_map(rusqlite::params![later], |row| row.get(0))?;
                rows.collect()
            })
            .await
            .expect("read should succeed");
        assert_eq!(live, vec!["s3".to_string()], "only the session past `later` is live");
    }
}
