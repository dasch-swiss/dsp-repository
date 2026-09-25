# Editor Persistence

How the editor stores its working state: one SQLite database behind the repository ports `editor-core` declares. The decision that nothing in it is irreplaceable is `areas/deposit/ADR-0001`; the target of one database file per capability is `areas/deposit/ADR-0004`.

One SQLite database, `rusqlite` with the `bundled` feature — the amalgamation is compiled by `cc` into the binary, which is what keeps the static musl image self-contained. `editor-core` owns the records and one repository trait per aggregate; `editor-server/src/db/` implements all nine against SQLite, so handlers depend on the ports and not on the driver.

`rusqlite` is pinned to **0.38, not 0.40**, because `deadpool-sqlite` 0.13 (the latest) requires `rusqlite ^0.38` and the two cannot coexist: `libsqlite3-sys` 0.36 and 0.38 both declare `links = "sqlite3"`, so cargo refuses to link both. Bump the pair together once `deadpool-sqlite` tracks 0.40.

## Two pools, and what that buys

`Database` holds a **writer** pool of exactly one connection and a **reader** pool of several. The split makes two rules structural instead of conventional:

- Reader connections carry `query_only=ON`, set in the pool's per-connection init hook, so a write cannot go through `Database::read`. The only way to write is `Database::write`, and that always opens `BEGIN IMMEDIATE` — after which SQLite guarantees nothing up to the matching `COMMIT` returns `SQLITE_BUSY`. A deferred `BEGIN` takes a read lock and can fail to upgrade it at the first write, which surfaces only under concurrency, as `database is locked`, and looks like something `busy_timeout` should fix.
- SQLite allows one writer at a time regardless, so a second writer connection would move the queue out of the pool (a bounded, observable wait) and into SQLite. One writer connection means writes serialise in the pool.

`rusqlite::Connection` is `!Sync`. `deadpool-sqlite` keeps each connection on a thread of its own and only lends it inside an `interact` closure, so the connection cannot escape, no `.await` can happen while it is held, and there is no `Mutex` guard to hold across one. `pool.get()` is async, so nothing blocks a Tokio worker either. The same shape is why no read transaction outlives a call, which would otherwise starve WAL checkpointing and let `-wal` grow without bound.

## PRAGMAs

All of them are applied in the pool's `post_create` hook, not once after the pool is built: everything except `journal_mode` is per-connection state, so central setup would leave every connection after the first at `busy_timeout=0` and `foreign_keys=OFF` while the code read as though they were configured. `foreign_keys` in particular is a documented **silent no-op inside a transaction**, so it must never be set from a migration — `ON DELETE CASCADE` would never fire, orphaned `sessions` would accumulate against deleted `users`, and an integrity check would pass because the constraint was never enforced.

File databases get `journal_mode=WAL` and `synchronous=NORMAL`; in-memory databases get neither, WAL being a file-database mode.

## Schema

A forward-only list of statement batches guarded by `PRAGMA user_version`, applied at startup — no migration framework and no added dependency. Everything runs in one `BEGIN IMMEDIATE` transaction including the version bump, so a crash part-way leaves the database at the version it started from. A database reporting a *higher* version than the build knows stops startup: that is a rollback to an older image, and running anyway would query columns that do not exist.

The tables are `users`, `user_shortcodes`, `sessions`, `login_codes`, `mail_sends`, `drafts`, `submissions`, `review_rounds`, `approved_records` and `entity_proposals`, all `STRICT`, created by one baseline migration, `0001_initial.sql`. Its comments carry the reason for each column and index.

**Until the first production deployment ([DEV-6921](https://linear.app/dasch/issue/DEV-6921)), a schema change edits `0001` in place** rather than adding `0002`. No database exists that anyone needs to keep, so a numbered migration would record a history nobody lived through. **From that deployment on, `0001` is frozen and every schema change is a forward migration** (`0002_*`, …) under the same mechanism: the `user_version` guard never re-runs an applied migration, so an edited `0001` would leave every deployed database on the old shape. Resetting the database on each schema change instead was considered and rejected: the database holds nothing irreplaceable (see [Operations](./operations.md#backups)), but production keeps it on a volume, and a reset would log everyone out, discard unsubmitted drafts and in-flight reviews, and make RDU re-create every depositor account.

`drafts`, `submissions` and `approved_records` carry their body as an opaque JSON `payload` string. It holds a serialized `editor_core::draft::ProjectDraft`, which is `#[serde(transparent)]` over the project's members, so the column already contains the project object and needs no migration to become typed; the persistence layer never interprets it.

## In-memory variant

Selected by leaving `EDITOR_DB_DIR` unset, which is the default — see [Operations](./operations.md#database) for why that is also the preview-safety default. It is a **named shared-cache URI** (`file:<name>?mode=memory&cache=shared`), never bare `:memory:`: every `:memory:` database is distinct and visible only to the connection that opened it, so each pooled connection would get its own empty copy, and with a writer/reader split readers could never see anything the writer wrote. The symptom is `no such table` that comes and goes with pool timing and test order, which reads exactly like a migration bug. Tests use a distinct name each, because a shared-cache in-memory database is scoped to the process and parallel `cargo test` threads share one.

## The draft's storage key folds case

`drafts.shortcode` is an exact-match column, while `PublishedProjects::get` and `User::may_reach` both fold ASCII case — the published set mixes `080C` with `0801a`, so a link typed either way reaches the same project. Keying a draft on the path segment as typed would therefore give `/projects/080c` and `/projects/080C` a **row each** for one project, and two people editing it would each keep half the edits with nothing to say so. The section handler folds, in one named place, and `is_valid_shortcode` admits only ASCII alphanumerics so the fold is total and agrees with the other two by construction.
