//! SQLite persistence: the connection pools, the PRAGMAs, the schema, and the
//! two entry points every query goes through.
//!
//! ## Why two pools
//!
//! [`Database`] holds a **writer** pool of exactly one connection and a
//! **reader** pool of several. That split is what makes the two rules this
//! layer must not get wrong structural rather than conventional:
//!
//! - Reader connections are opened with `query_only=ON` in the per-connection init hook, so a write
//!   physically cannot go through [`Database::read`]. The only path that can write is
//!   [`Database::write`], and that always opens `BEGIN IMMEDIATE`.
//! - SQLite allows one writer at a time regardless. A second writer connection would not add
//!   concurrency, it would move the queue from the pool (a bounded, observable wait) into SQLite
//!   (`SQLITE_BUSY` once `busy_timeout` runs out). One writer connection means writes serialise in
//!   the pool.
//!
//! ## Why `BEGIN IMMEDIATE` on every write
//!
//! Once `BEGIN IMMEDIATE` succeeds, SQLite guarantees nothing up to the
//! matching `COMMIT` returns `SQLITE_BUSY`. A deferred `BEGIN` takes a read lock
//! and tries to upgrade it at the first write, and that upgrade can fail — so
//! the transaction dies part-way with `database is locked`. It only happens
//! under concurrency, so it passes every test and then fails intermittently in
//! production, where the error reads like tuning and the reflex is to raise
//! `busy_timeout`, which cannot help: the lock is not being waited for, the
//! upgrade is being refused. Django added `transaction_mode: IMMEDIATE` in 5.1
//! for exactly this.
//!
//! ## Why every call goes through `interact`
//!
//! `rusqlite::Connection` is `!Sync`, so it cannot be shared behind an `Arc`,
//! and putting it behind a `std::sync::Mutex` invites holding the guard across
//! an `.await`, which stalls or deadlocks the executor. `deadpool-sqlite` keeps
//! each connection on a thread of its own and hands it out only inside an
//! `interact` closure, which is `FnOnce(&mut Connection) -> R + Send + 'static`
//! — the connection cannot escape and no `.await` can happen while it is held.
//! `pool.get()` is itself async, so nothing blocks a Tokio worker either.
//!
//! Long-lived read transactions are avoided the same way: a read is a closure
//! that runs to completion on a blocking thread, so no read transaction can be
//! left open across an `.await` to starve WAL checkpointing and let `-wal` grow
//! without bound.

mod approved_records;
mod drafts;
mod login_codes;
mod mapping;
mod schema;
mod sessions;
mod submissions;
mod users;

use std::path::{Path, PathBuf};
use std::time::Duration;

use deadpool_sqlite::{Config as PoolSource, Hook, HookError, Pool, PoolConfig, Runtime};
use editor_core::repository::RepositoryError;
use rusqlite::{Connection, Transaction, TransactionBehavior};

/// The database file inside the mounted directory. Fixed rather than
/// configurable, because config names the **directory**: SQLite needs to create
/// `-wal` and `-shm` siblings next to the file, which it cannot do if the file
/// itself is the mount point.
const DATABASE_FILE_NAME: &str = "editor.sqlite3";

/// How long `pool.get()` waits for a connection before giving up.
///
/// Deliberately longer than the SQLite `busy_timeout`, so a queue behind the
/// single writer resolves as a slow request rather than an error, and shorter
/// than forever, which is deadpool's default and would hang a request with no
/// diagnostic.
const POOL_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

/// The writer pool is exactly one connection. SQLite permits a single writer at
/// a time, so a second would not add concurrency — it would move the queue from
/// the pool (a bounded, observable wait) into SQLite (`SQLITE_BUSY` once
/// `busy_timeout` runs out).
const WRITER_CONNECTIONS: usize = 1;

/// A **floor** on the reader pool, not a ceiling — the configured default is 4,
/// and `EDITOR_DB_READERS` sets it. Without the floor, `EDITOR_DB_READERS=0`
/// would build a pool that can never hand out a connection, so every read would
/// wait out [`POOL_WAIT_TIMEOUT`] and then fail.
const MIN_READER_CONNECTIONS: usize = 1;

/// Where the database lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    /// A file in this directory. Production: a mount provided by Infra.
    Directory(PathBuf),
    /// A named, shared-cache, in-memory database.
    ///
    /// Never bare `:memory:`. Every `:memory:` database is distinct and visible
    /// only to the connection that opened it, so each pooled connection would
    /// get its own empty copy — and with a writer/reader split that is
    /// unconditional: readers could never see anything the writer wrote. The
    /// symptom is `no such table` that comes and goes with pool timing and test
    /// order, which reads exactly like a migration bug; the usual "fix" of
    /// migrating every connection hides it while making the tests prove nothing
    /// about migration ordering.
    Memory(String),
}

impl Source {
    /// An in-memory database with a unique name, for one test.
    ///
    /// The name has to differ per test: shared-cache in-memory databases are
    /// scoped to the process, so two tests using one name would share a database
    /// across parallel `cargo test` threads.
    #[cfg(test)]
    pub(crate) fn memory_for_test(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self::Memory(format!("editor-test-{label}-{}-{n}", std::process::id()))
    }

    /// What is handed to `rusqlite::Connection::open`. For the in-memory
    /// variant this is a URI, which works because rusqlite's default open flags
    /// include `SQLITE_OPEN_URI`.
    fn open_target(&self) -> PathBuf {
        match self {
            Self::Directory(dir) => dir.join(DATABASE_FILE_NAME),
            Self::Memory(name) => PathBuf::from(format!("file:{name}?mode=memory&cache=shared")),
        }
    }

    /// WAL is a file-database mode: an in-memory database supports only `memory`
    /// and `off`, and asking for WAL there silently leaves it as it was.
    fn uses_wal(&self) -> bool {
        matches!(self, Self::Directory(_))
    }

    /// How this source appears in a log line.
    fn describe(&self) -> String {
        match self {
            Self::Directory(_) => self.open_target().display().to_string(),
            Self::Memory(name) => format!("in-memory ({name})"),
        }
    }
}

/// Startup and connection failures, each phrased to name the actual cause.
///
/// The wrong-permissions cases matter most: without the pre-flight they all
/// surface as SQLite's `unable to open database file`, which reads like a bad
/// mount path and sends the reader hunting for a typo in a path that is
/// perfectly correct.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DbError {
    #[error(
        "the database directory {path} does not exist. Docker Swarm does not create a missing bind-mount host \
         directory the way `docker run -v` does, so it must exist, and be owned by uid 65532, before the stack starts"
    )]
    DirectoryMissing { path: PathBuf },

    #[error(
        "{path} is not a directory. EDITOR_DB_DIR names the directory that holds the database, not the database \
         file: SQLite has to create `{DATABASE_FILE_NAME}-wal` and `{DATABASE_FILE_NAME}-shm` next to it, which is \
         impossible if the file itself is the mount point"
    )]
    DirectoryNotADirectory { path: PathBuf },

    #[error(
        "the database directory {path} is not writable by this process. The image runs as uid 65532 (distroless \
         `:nonroot`; 65534 is `nobody`, a different account), so the mounted directory must be owned by 65532 — a \
         root-owned or 65534-owned mount fails here"
    )]
    DirectoryNotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to build the SQLite connection pool: {0}")]
    Pool(String),

    #[error("failed to check out a SQLite connection: {0}")]
    Checkout(String),

    #[error("the SQLite worker thread failed: {0}")]
    Interact(String),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

impl DbError {
    /// Whether this is a **uniqueness** constraint rejecting the write.
    ///
    /// Matched on the extended result code, not `ErrorCode::ConstraintViolation`:
    /// that covers `NOT NULL`, `CHECK` and `FOREIGN KEY` too, so a session
    /// inserted for a user that no longer exists would be reported as "session
    /// already exists" — a message that sends the reader looking for a duplicate
    /// that is not there.
    ///
    /// Which unique index it was is not recoverable from the error, so the entity
    /// name comes from the call site: each table that maps this to
    /// [`RepositoryError::Conflict`] has exactly one — a duplicate address for
    /// REQ-7.4, a second pending submission for one project.
    fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            Self::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                    || e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
        )
    }

    /// Map to [`RepositoryError::Conflict`] when a unique index rejected the
    /// write, and to a backend error otherwise.
    fn into_repository_error(self, entity: &'static str) -> RepositoryError {
        if self.is_unique_violation() {
            RepositoryError::Conflict { entity }
        } else {
            RepositoryError::backend(self)
        }
    }
}

impl From<DbError> for RepositoryError {
    fn from(error: DbError) -> Self {
        Self::backend(error)
    }
}

/// The persistence layer: two pools over one SQLite database.
///
/// Cheap to clone — both pools are handles.
#[derive(Clone)]
pub(crate) struct Database {
    /// Exactly one connection, no `query_only`. See the module docs.
    writer: Pool,
    /// Several connections, all `query_only=ON`.
    readers: Pool,
    source: Source,
}

impl Database {
    /// Open the database, run the write pre-flight, and apply any outstanding
    /// migrations.
    ///
    /// Migrations run through the writer pool on purpose: it leaves one
    /// connection open in that pool from here on, which the in-memory variant
    /// depends on — a shared-cache in-memory database exists only while at least
    /// one connection to it is open.
    pub(crate) async fn open(source: Source, reader_count: usize, busy_timeout: Duration) -> Result<Self, DbError> {
        if let Source::Directory(dir) = &source {
            preflight(dir)?;
        }

        let target = source.open_target();
        let writer = build_pool(&target, WRITER_CONNECTIONS, busy_timeout, source.uses_wal(), false)?;
        // `query_only` on every reader. Set here rather than toggled per call:
        // a toggle has to be undone on every path including the error ones, and
        // a missed reset leaves a pooled connection rejecting writes forever.
        let readers = build_pool(&target, reader_count.max(MIN_READER_CONNECTIONS), busy_timeout, false, true)?;

        let db = Self { writer, readers, source };
        let applied = db.migrate().await?;
        // The version is read back rather than reported from the constant, so the
        // log line says what the database is at and not what this build believes.
        tracing::info!(
            database = %db.source.describe(),
            schema_version = db.schema_version().await?,
            migrations_applied = applied,
            "SQLite ready"
        );
        Ok(db)
    }

    /// Read from the database.
    ///
    /// Runs on a reader connection, which is `query_only=ON` — an `INSERT` here
    /// fails with `attempt to write a readonly database` rather than quietly
    /// bypassing [`Self::write`]'s `BEGIN IMMEDIATE`.
    pub(crate) async fn read<T, F>(&self, f: F) -> Result<T, DbError>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.readers.get().await.map_err(|e| DbError::Checkout(e.to_string()))?;
        conn.interact(move |conn| f(conn))
            .await
            .map_err(|e| DbError::Interact(e.to_string()))?
            .map_err(DbError::Sqlite)
    }

    /// Read from the database inside one read transaction, for a read that takes
    /// more than one statement.
    ///
    /// In autocommit mode each statement gets its own implicit read transaction,
    /// so another connection's commit can land between two of them and the pair
    /// can disagree — a user read back with shortcode assignments it no longer
    /// has. A deferred `BEGIN` gives both statements one snapshot.
    ///
    /// Still a reader connection, so it cannot write, and still one closure, so
    /// the transaction cannot outlive the call and starve WAL checkpointing.
    /// Routing these through [`Self::write`] instead would have been consistent
    /// too, and would have put every authenticated request's user lookup behind
    /// the single writer connection.
    pub(crate) async fn read_tx<T, F>(&self, f: F) -> Result<T, DbError>
    where
        F: FnOnce(&Transaction<'_>) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.readers.get().await.map_err(|e| DbError::Checkout(e.to_string()))?;
        conn.interact(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
            let out = f(&tx)?;
            // A read transaction has nothing to commit; rolling back is the
            // cheaper way to end it and cannot fail on a conflict.
            tx.rollback()?;
            Ok(out)
        })
        .await
        .map_err(|e| DbError::Interact(e.to_string()))?
        .map_err(DbError::Sqlite)
    }

    /// Write to the database inside `BEGIN IMMEDIATE`.
    ///
    /// The transaction commits when the closure returns `Ok` and rolls back when
    /// it returns `Err` or panics. This is the only way to get a write handle,
    /// which is what makes "`BEGIN IMMEDIATE` on every write path" a property of
    /// the type rather than something each call site has to remember.
    pub(crate) async fn write<T, F>(&self, f: F) -> Result<T, DbError>
    where
        F: FnOnce(&Transaction<'_>) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.writer.get().await.map_err(|e| DbError::Checkout(e.to_string()))?;
        conn.interact(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let out = f(&tx)?;
            tx.commit()?;
            Ok(out)
        })
        .await
        .map_err(|e| DbError::Interact(e.to_string()))?
        .map_err(DbError::Sqlite)
    }
}

/// Verify the directory exists, is a directory, and can be written to.
///
/// The probe file is the point: a directory can exist and be readable and still
/// reject a write, and every one of those cases otherwise reaches the operator
/// as `unable to open database file`.
fn preflight(dir: &Path) -> Result<(), DbError> {
    let metadata = std::fs::metadata(dir).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            DbError::DirectoryMissing { path: dir.to_path_buf() }
        } else {
            DbError::DirectoryNotWritable { path: dir.to_path_buf(), source }
        }
    })?;
    if !metadata.is_dir() {
        return Err(DbError::DirectoryNotADirectory { path: dir.to_path_buf() });
    }

    // Named by process id so two processes sharing a directory cannot delete
    // each other's probe.
    let probe = dir.join(format!(".editor-write-probe-{}", std::process::id()));
    std::fs::write(&probe, b"editor write probe")
        .map_err(|source| DbError::DirectoryNotWritable { path: dir.to_path_buf(), source })?;
    // Removal is checked too: a directory with the sticky bit set, or one on a
    // filesystem mounted append-only, allows the create and refuses the unlink —
    // which would leave a probe file behind on every restart.
    std::fs::remove_file(&probe).map_err(|source| DbError::DirectoryNotWritable { path: dir.to_path_buf(), source })?;
    Ok(())
}

/// Build one pool over `target`, with the PRAGMAs applied per connection.
fn build_pool(
    target: &Path,
    max_size: usize,
    busy_timeout: Duration,
    journal_wal: bool,
    query_only: bool,
) -> Result<Pool, DbError> {
    let mut pool_config = PoolConfig::new(max_size);
    pool_config.timeouts.wait = Some(POOL_WAIT_TIMEOUT);
    let mut source = PoolSource::new(target);
    source.pool = Some(pool_config);

    source
        .builder(Runtime::Tokio1)
        .map_err(|e| DbError::Pool(e.to_string()))?
        // The PRAGMAs belong here, in the per-connection init hook, and not in a
        // one-off after the pool is built. Everything except `journal_mode` is
        // per-connection state, so central setup would leave every connection
        // after the first at `busy_timeout=0` and `foreign_keys=OFF` while the
        // code reads as though they were configured.
        .post_create(Hook::async_fn(move |conn, _metrics| {
            Box::pin(async move {
                conn.interact(move |conn| init_connection(conn, busy_timeout, journal_wal, query_only))
                    .await
                    .map_err(|e| HookError::message(format!("SQLite connection init failed: {e}")))?
                    .map_err(HookError::Backend)
            })
        }))
        .build()
        .map_err(|e| DbError::Pool(e.to_string()))
}

/// Apply the per-connection PRAGMAs.
///
/// Order matters twice over. `foreign_keys` must be set outside a transaction —
/// inside one it is a documented silent no-op, and the failure is invisible:
/// `ON DELETE CASCADE` never fires, orphaned `sessions` accumulate against
/// deleted `users`, and an integrity check passes because the constraint was
/// never enforced. And `query_only` goes last, because it would otherwise block
/// the PRAGMAs before it.
fn init_connection(
    conn: &Connection,
    busy_timeout: Duration,
    journal_wal: bool,
    query_only: bool,
) -> rusqlite::Result<()> {
    // Not inside a transaction: this runs on a freshly opened connection, before
    // anything can have begun one.
    debug_assert!(conn.is_autocommit(), "PRAGMAs must be applied outside a transaction");

    if journal_wal {
        // `PRAGMA journal_mode` returns the resulting mode, so it is a query,
        // not a statement. Database-level and persistent, unlike everything else
        // here, so re-asserting it per connection is a no-op after the first.
        let mode: String = conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
                Some(format!("journal_mode=WAL was refused; the database is in {mode} mode")),
            ));
        }
        // NORMAL rather than the FULL default: with WAL, a commit no longer
        // fsyncs, so a power loss or OS crash can lose the last transactions —
        // never corrupt the database, and never on an application crash. That
        // trade is right here because git holds everything irreplaceable and the
        // PRD makes backups optional; drafts and in-flight submissions are
        // re-creatable.
        conn.pragma_update(None, "synchronous", "NORMAL")?;
    }

    // Off by default in SQLite for backwards compatibility, whatever the build
    // flags say. `bundled` happens to compile with
    // -DSQLITE_DEFAULT_FOREIGN_KEYS=1, but that is a compile flag we do not
    // control, and the whole point of the schema's `ON DELETE CASCADE` and
    // `ON DELETE SET NULL` is that they fire.
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Per-connection, and zero by default: an unconfigured connection returns
    // SQLITE_BUSY immediately rather than waiting for the writer.
    conn.busy_timeout(busy_timeout)?;

    if query_only {
        conn.pragma_update(None, "query_only", "ON")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database for one test: in-memory, uniquely named, migrated.
    pub(crate) async fn test_db(label: &str) -> Database {
        Database::open(Source::memory_for_test(label), 4, Duration::from_secs(5))
            .await
            .expect("test database should open")
    }

    /// Row count of one table, so a test can assert on rows no repository method
    /// returns — cascades, and what a rollback left behind.
    pub(crate) async fn count(db: &Database, table: &'static str) -> u64 {
        let counted: i64 = db
            .read(move |conn| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)))
            .await
            .expect("counting rows should succeed");
        super::mapping::row_count(counted)
    }

    #[tokio::test]
    async fn test_in_memory_source_is_a_named_shared_cache_uri_not_bare_memory() {
        let target = Source::Memory("editor".into()).open_target();
        let target = target.to_string_lossy();
        assert!(target.starts_with("file:"), "{target}");
        assert!(target.contains("mode=memory"), "{target}");
        assert!(target.contains("cache=shared"), "{target}");
        assert_ne!(target, ":memory:");
    }

    #[tokio::test]
    async fn test_the_reader_pool_is_sized_from_config_and_not_capped_at_one() {
        // `reader_count.max(MIN_READER_CONNECTIONS)` is a floor, and Rust's `max`
        // returning the larger value reads as a ceiling to anyone who does not
        // hold that in mind — it was misread as one in review. Pinned here so the
        // sizing is a fact and not an inference from the expression.
        let db = Database::open(Source::memory_for_test("reader-count"), 4, Duration::from_secs(5))
            .await
            .expect("test database should open");
        assert_eq!(
            db.readers.status().max_size,
            4,
            "the reader pool is whatever EDITOR_DB_READERS says"
        );
        assert_eq!(
            db.writer.status().max_size,
            1,
            "exactly one writer is deliberate; see WRITER_CONNECTIONS"
        );
    }

    #[tokio::test]
    async fn test_zero_configured_readers_still_yields_a_usable_pool() {
        // What the floor is for. A zero-sized pool can never hand out a
        // connection, so every read would wait out POOL_WAIT_TIMEOUT and then
        // fail — fifteen seconds per request, from one nonsensical env var.
        let db = Database::open(Source::memory_for_test("zero-readers"), 0, Duration::from_secs(5))
            .await
            .expect("test database should open");
        assert_eq!(db.readers.status().max_size, MIN_READER_CONNECTIONS);
        db.read(|conn| conn.query_row("SELECT count(*) FROM users", [], |row| row.get::<_, i64>(0)))
            .await
            .expect("a read must still work with the floor applied");
    }

    #[tokio::test]
    async fn test_writer_and_readers_see_one_shared_in_memory_database() {
        // The trap the named shared-cache URI exists to avoid. With bare
        // `:memory:` each pooled connection gets its own empty database, so a
        // read after a write fails with `no such table` — intermittently, with
        // pool timing and test order, reading exactly like a migration bug.
        let db = test_db("shared-cache").await;
        db.write(|tx| {
            tx.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                 VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'depositor', '2026-08-21 10:00:00+00:00')",
                [],
            )
        })
        .await
        .expect("write should succeed");

        let count: i64 = db
            .read(|conn| conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)))
            .await
            .expect("read should succeed");
        assert_eq!(count, 1, "a reader connection must see what the writer wrote");
    }

    #[tokio::test]
    async fn test_every_reader_connection_sees_the_schema() {
        // Exercises more connections than one, so a per-connection database (the
        // bare `:memory:` failure) shows up rather than hiding behind a single
        // reader that happens to be reused.
        let db = test_db("all-readers").await;
        let mut handles = Vec::new();
        for _ in 0..8 {
            let db = db.clone();
            handles.push(tokio::spawn(async move {
                db.read(|conn| conn.query_row("SELECT count(*) FROM submissions", [], |row| row.get::<_, i64>(0)))
                    .await
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap().expect("read should succeed"), 0);
        }
    }

    #[tokio::test]
    async fn test_reader_connections_reject_writes() {
        // The structural half of "BEGIN IMMEDIATE on every write path": if a
        // reader could write, a caller could bypass `write()` and its
        // `BEGIN IMMEDIATE` without anyone noticing at review time.
        let db = test_db("query-only").await;
        let result = db
            .read(|conn| {
                conn.execute(
                    "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                     VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'depositor', '2026-08-21 10:00:00+00:00')",
                    [],
                )
            })
            .await;
        let error = result.expect_err("a write through read() must be refused").to_string();
        assert!(error.contains("readonly"), "expected a readonly failure, got: {error}");
    }

    #[tokio::test]
    async fn test_read_transactions_are_read_only_too() {
        // `read_tx` exists so a multi-statement read sees one snapshot. It must
        // not become a second write path in the process — that would put writes
        // outside `write()` and its `BEGIN IMMEDIATE`.
        let db = test_db("read-tx-query-only").await;
        let result = db
            .read_tx(|tx| {
                tx.execute(
                    "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                     VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'depositor', '2026-08-21 10:00:00+00:00')",
                    [],
                )
            })
            .await;
        let error = result.expect_err("a write through read_tx must be refused").to_string();
        assert!(error.contains("readonly"), "expected a readonly failure, got: {error}");
    }

    #[tokio::test]
    async fn test_a_read_transaction_does_not_block_writes() {
        // A read transaction that outlived its call would starve WAL
        // checkpointing and let `-wal` grow without bound. It cannot, because it
        // begins and ends inside one `interact` closure — so a write straight
        // after one goes through.
        let db = test_db("read-tx-no-block").await;
        let counted: i64 = db
            .read_tx(|tx| tx.query_row("SELECT count(*) FROM users", [], |row| row.get(0)))
            .await
            .expect("read transaction should succeed");
        assert_eq!(counted, 0);

        db.write(|tx| {
            tx.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                 VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', '2026-08-21 10:00:00+00:00')",
                [],
            )
        })
        .await
        .expect("a write after a read transaction must not be blocked");
    }

    #[tokio::test]
    async fn test_foreign_keys_are_enforced_on_every_connection() {
        // `PRAGMA foreign_keys` is per-connection and a silent no-op inside a
        // transaction. If it were applied centrally after the pool was built, or
        // from inside the migration transaction, this insert would succeed and
        // `ON DELETE CASCADE` would never fire anywhere.
        let db = test_db("foreign-keys").await;
        let result = db
            .write(|tx| {
                tx.execute(
                    "INSERT INTO sessions (id, user_id, created_at, last_seen_at, expires_at) \
                     VALUES ('s1', 'no-such-user', '2026-08-21 10:00:00+00:00', '2026-08-21 10:00:00+00:00', \
                     '2026-08-21 11:00:00+00:00')",
                    [],
                )
            })
            .await;
        assert!(result.is_err(), "a session for a nonexistent user must be rejected");

        // And that it is on for readers too, so a reader cannot be the one
        // connection that silently disagrees.
        let on: i64 = db
            .read(|conn| conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0)))
            .await
            .expect("pragma read should succeed");
        assert_eq!(on, 1);
    }

    #[tokio::test]
    async fn test_busy_timeout_is_set_on_every_connection() {
        // Per-connection and zero by default. A reader left at zero returns
        // SQLITE_BUSY the moment the writer holds the lock, which is precisely
        // the intermittent production failure this layer is built to avoid.
        let db = test_db("busy-timeout").await;
        // Concurrently, so more than one pooled connection is created and the
        // assertion covers connections beyond the first.
        let mut handles = Vec::new();
        for _ in 0..8 {
            let db = db.clone();
            handles.push(tokio::spawn(async move {
                db.read(|conn| conn.query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0)))
                    .await
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap().expect("pragma read should succeed"), 5000);
        }

        let writer_timeout: i64 = db
            .write(|tx| tx.query_row("PRAGMA busy_timeout", [], |row| row.get(0)))
            .await
            .expect("pragma read should succeed");
        assert_eq!(writer_timeout, 5000);
    }

    #[tokio::test]
    async fn test_failed_write_rolls_back() {
        let db = test_db("rollback").await;
        let result: Result<(), DbError> = db
            .write(|tx| {
                tx.execute(
                    "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                     VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'depositor', '2026-08-21 10:00:00+00:00')",
                    [],
                )?;
                // Same primary key: the statement fails, so the closure returns
                // Err and the whole transaction must go, first insert included.
                tx.execute(
                    "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                     VALUES ('u1', 'b@x.test', 'b@x.test', 'B', 'depositor', '2026-08-21 10:00:00+00:00')",
                    [],
                )?;
                Ok(())
            })
            .await;
        assert!(result.is_err());

        let count: i64 = db
            .read(|conn| conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)))
            .await
            .expect("read should succeed");
        assert_eq!(count, 0, "the first insert must have been rolled back with the second");
    }

    #[tokio::test]
    async fn test_concurrent_writes_all_commit() {
        // With a deferred BEGIN this is where `database is locked` appears: a
        // read transaction that cannot be upgraded fails part-way. With
        // BEGIN IMMEDIATE and one writer connection, they serialise and all
        // commit.
        let db = test_db("concurrent-writes").await;
        let mut handles = Vec::new();
        for i in 0..24 {
            let db = db.clone();
            handles.push(tokio::spawn(async move {
                db.write(move |tx| {
                    // A read followed by a write in one transaction — the shape
                    // a deferred BEGIN cannot upgrade.
                    let existing: i64 = tx.query_row("SELECT count(*) FROM users", [], |row| row.get(0))?;
                    tx.execute(
                        "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                         VALUES (?1, ?2, ?2, ?3, 'depositor', '2026-08-21 10:00:00+00:00')",
                        rusqlite::params![format!("u{i}"), format!("u{i}@x.test"), format!("User {existing}")],
                    )
                })
                .await
            }));
        }
        for handle in handles {
            handle.await.unwrap().expect("every concurrent write must commit");
        }

        let count: i64 = db
            .read(|conn| conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)))
            .await
            .expect("read should succeed");
        assert_eq!(count, 24);
    }

    #[test]
    fn test_preflight_reports_a_missing_directory_as_missing() {
        let missing = std::env::temp_dir().join(format!("editor-db-missing-{}", std::process::id()));
        let error = preflight(&missing).expect_err("a missing directory must fail the pre-flight");
        assert!(matches!(error, DbError::DirectoryMissing { .. }), "{error}");
        // The message has to name the cause, because SQLite's own wording for
        // all of these is `unable to open database file`.
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn test_preflight_rejects_a_file_where_a_directory_is_expected() {
        // The mount-the-directory-not-the-file mistake. Pointing at the file
        // works right up to the first WAL write and then fails, so catching it
        // at startup is the difference between a clear message and a corrupted
        // deployment.
        let file = std::env::temp_dir().join(format!("editor-db-file-{}.sqlite3", std::process::id()));
        std::fs::write(&file, b"not a directory").unwrap();
        let error = preflight(&file).expect_err("a file must fail the pre-flight");
        assert!(matches!(error, DbError::DirectoryNotADirectory { .. }), "{error}");
        assert!(error.to_string().contains("not a directory"), "{error}");
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn test_preflight_accepts_a_writable_directory_and_leaves_nothing_behind() {
        let dir = std::env::temp_dir().join(format!("editor-db-ok-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        preflight(&dir).expect("a writable directory must pass");
        let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        assert!(leftovers.is_empty(), "the probe file must be removed: {leftovers:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn test_preflight_reports_an_unwritable_directory_as_a_permissions_problem() {
        // The uid-65532 case: the directory exists and is readable, and only the
        // write fails.
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("editor-db-ro-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();

        let result = preflight(&dir);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        // root ignores the mode bits, so the write succeeds and there is nothing
        // to assert. Asserted rather than skipped when it does fail, because
        // that is the case the message has to get right.
        if let Err(error) = result {
            assert!(matches!(error, DbError::DirectoryNotWritable { .. }), "{error}");
            assert!(error.to_string().contains("65532"), "the message must name the uid: {error}");
        }
    }

    #[tokio::test]
    async fn test_file_database_uses_wal_and_creates_its_siblings_in_the_directory() {
        // WAL is why the *directory* has to be writable rather than just the
        // file: `-wal` and `-shm` are created next to the database.
        let dir = std::env::temp_dir().join(format!("editor-db-wal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let db = Database::open(Source::Directory(dir.clone()), 2, Duration::from_secs(5))
            .await
            .expect("a file database should open");
        let mode: String = db
            .read(|conn| conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)))
            .await
            .expect("pragma read should succeed");
        assert_eq!(mode.to_lowercase(), "wal");

        db.write(|tx| {
            tx.execute(
                "INSERT INTO users (id, email, email_normalized, name, role, created_at) \
                 VALUES ('u1', 'a@x.test', 'a@x.test', 'A', 'rdu', '2026-08-21 10:00:00+00:00')",
                [],
            )
        })
        .await
        .expect("write should succeed");

        assert!(dir.join(DATABASE_FILE_NAME).exists());
        assert!(
            dir.join(format!("{DATABASE_FILE_NAME}-wal")).exists(),
            "the -wal sibling must be next to the file"
        );

        drop(db);
        std::fs::remove_dir_all(&dir).ok();
    }
}
