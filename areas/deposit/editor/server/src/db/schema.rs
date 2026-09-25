//! The schema, as a forward-only ordered statement list guarded by
//! `PRAGMA user_version`.
//!
//! The rules that keep it honest:
//!
//! - Until the first production deployment (DEV-6921), a schema change edits `0001` in place. After
//!   it, `0001` is frozen and a change is a new entry: the guard never re-runs an applied one. See
//!   `docs/src/editor/persistence.md#schema`.
//! - Everything runs inside one `BEGIN IMMEDIATE` transaction, the `user_version` bump included, so
//!   a crash part-way leaves the database at the version it started from rather than half-migrated.
//!   `BEGIN IMMEDIATE` also makes two processes starting at once safe: the second waits and then
//!   finds nothing to do.
//! - `PRAGMA foreign_keys` is **not** touched here. It is a documented silent no-op inside a
//!   transaction, so a migration that set it would appear to work and leave every `ON DELETE
//!   CASCADE` unenforced. It belongs to the per-connection init hook in [`super::init_connection`].

use super::{Database, DbError};

/// Ordered: index `i` is migration `i + 1`, and how many have been applied is
/// `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_initial.sql")];

impl Database {
    /// Apply every migration this database has not run, and return how many ran.
    ///
    /// Zero means the schema was already current, which is the normal case on
    /// every restart after the first.
    pub(super) async fn migrate(&self) -> Result<u32, DbError> {
        self.write(|tx| apply_outstanding(tx, MIGRATIONS)).await
    }

    /// The schema version this database reports.
    pub(crate) async fn schema_version(&self) -> Result<u32, DbError> {
        let version: i64 = self
            .read(|conn| conn.pragma_query_value(None, "user_version", |row| row.get(0)))
            .await?;
        Ok(version.max(0) as u32)
    }
}

/// Apply the entries of `migrations` past `user_version`, and return how many ran.
/// Takes the list so the upgrade path is testable while [`MIGRATIONS`] has one entry.
fn apply_outstanding(tx: &rusqlite::Transaction<'_>, migrations: &[&str]) -> rusqlite::Result<u32> {
    let known = migrations.len() as u32;
    let current: u32 = tx.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?.max(0) as u32;

    // Written by a newer release, i.e. a rollback to an older image: stop, and
    // name both versions, since no SQLite error would say why.
    if current > known {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
            Some(format!(
                "the database is at schema version {current}, but this build only knows {known} — it was written by a \
                 newer release of editor-server"
            )),
        ));
    }

    let mut applied = 0;
    for (index, statements) in migrations.iter().enumerate().skip(current as usize) {
        let version = index as u32 + 1;
        tx.execute_batch(statements)?;
        // PRAGMA values cannot be bound; `version` comes from a slice index.
        tx.pragma_update(None, "user_version", version)?;
        applied += 1;
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::tests::test_db;
    use super::*;
    use crate::db::Source;

    /// The version a fully migrated database reports.
    const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

    /// Every table the persistence layer is responsible for.
    const EXPECTED_TABLES: &[&str] = &[
        "approved_records",
        "drafts",
        "entity_proposals",
        "login_codes",
        "mail_sends",
        "review_rounds",
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

    #[test]
    fn test_a_database_at_the_previous_version_is_upgraded_in_place() {
        // A database at the previous version gains the next column and keeps its
        // rows. The second migration is a stand-in until a real one exists.
        const NEXT: &str = "ALTER TABLE users ADD COLUMN probe TEXT;";
        let mut conn = rusqlite::Connection::open_in_memory().expect("an in-memory connection should open");

        let tx = conn.transaction().unwrap();
        assert_eq!(apply_outstanding(&tx, MIGRATIONS).unwrap(), 1);
        tx.execute(
            "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
             VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', '2026-08-21 10:00:00+00:00')",
            [],
        )
        .expect("the pre-upgrade row should insert");
        tx.commit().unwrap();

        let upgraded = [MIGRATIONS[0], NEXT];
        let tx = conn.transaction().unwrap();
        assert_eq!(
            apply_outstanding(&tx, &upgraded).unwrap(),
            1,
            "only the outstanding migration runs"
        );
        assert_eq!(apply_outstanding(&tx, &upgraded).unwrap(), 0, "the upgrade must not run twice");
        tx.commit().unwrap();

        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        let (users, unset): (i64, i64) = conn
            .query_row("SELECT count(*), sum(probe IS NULL) FROM users", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("reading the upgraded table should succeed");
        assert_eq!((version, users, unset), (2, 1, 1));
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
