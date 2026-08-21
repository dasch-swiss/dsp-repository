# Editor Architecture

The metadata editor is where depositing project teams edit their project metadata, RDU reviews it field by field, and approved records are collected into a pull request against this repository. Git stays the source of truth.

This page describes the service as it stands. Surfaces that are not built yet are named where they affect a decision taken now, and marked as such.

## Relationship to DPE

The editor is a **separate service** from DPE, not a section of it. They share `platform-telemetry` for the browser-beacon contract, and will share `mosaic-tiles` for components and `dpe-core` for the data contract — but not a process, an image, or an origin.

The separation is deliberate:

- DPE is public, unauthenticated and read-only. The editor is authenticated and writes state. A host-level compromise of one should not hand over the other's session cookies.
- The editor's CSRF defence requires `Sec-Fetch-Site: same-origin` on every state-changing request. On a shared origin, a request originating from DPE *is* same-origin — so any XSS in DPE, which has a far larger unauthenticated attack surface, could drive authenticated editor mutations. A `Path` on a cookie is not a security boundary and does not close this.

## Rendering model

Same as DPE: server-rendered HTML with **Maud**, served by **Axum**, with **Datastar** for interactivity over SSE. No client-side WASM, no hydration, no islands. The server is the single source of truth for UI state.

## Crates

| Crate | Folder | Role |
|-------|--------|------|
| `editor-core` | `editor/core` | Pure domain types and the persistence ports (no Axum, Maud or database dependency) |
| `editor-web` | `editor/web` | Maud view library — the document shell, pages and components |
| `editor-server` | `editor/server` | Composition root: configuration, observability, routing, persistence |

Dependency direction is `server → web → core`. Neither library depends on `mosaic-tiles` yet: the form widgets are the first surface to render a tile. The Tailwind entry already scans the crate, so component CSS ships regardless of the Cargo dependency.

Unlike DPE, the **HTML document shell lives in the view crate** (`editor-web/src/view.rs`), not the server crate. DPE keeps `head()` + `page()` in `dpe-server`; here the server is a composition root for routing, auth and persistence, and a document shell is a view concern like any other partial.

## Persistence

One SQLite database, `rusqlite` with the `bundled` feature — the amalgamation is compiled by `cc` into the binary, which is what keeps the static musl image self-contained. `editor-core` owns the records and one repository trait per aggregate; `editor-server/src/db/` implements all six against SQLite, so handlers depend on the ports and not on the driver.

`rusqlite` is pinned to **0.38, not 0.40**, because `deadpool-sqlite` 0.13 (the latest) requires `rusqlite ^0.38` and the two cannot coexist: `libsqlite3-sys` 0.36 and 0.38 both declare `links = "sqlite3"`, so cargo refuses to link both. Bump the pair together once `deadpool-sqlite` tracks 0.40.

### Two pools, and what that buys

`Database` holds a **writer** pool of exactly one connection and a **reader** pool of several. The split makes two rules structural instead of conventional:

- Reader connections carry `query_only=ON`, set in the pool's per-connection init hook, so a write cannot go through `Database::read`. The only way to write is `Database::write`, and that always opens `BEGIN IMMEDIATE` — after which SQLite guarantees nothing up to the matching `COMMIT` returns `SQLITE_BUSY`. A deferred `BEGIN` takes a read lock and can fail to upgrade it at the first write, which surfaces only under concurrency, as `database is locked`, and looks like something `busy_timeout` should fix.
- SQLite allows one writer at a time regardless, so a second writer connection would move the queue out of the pool (a bounded, observable wait) and into SQLite. One writer connection means writes serialise in the pool.

`rusqlite::Connection` is `!Sync`. `deadpool-sqlite` keeps each connection on a thread of its own and only lends it inside an `interact` closure, so the connection cannot escape, no `.await` can happen while it is held, and there is no `Mutex` guard to hold across one. `pool.get()` is async, so nothing blocks a Tokio worker either. The same shape is why no read transaction outlives a call, which would otherwise starve WAL checkpointing and let `-wal` grow without bound.

### PRAGMAs

All of them are applied in the pool's `post_create` hook, not once after the pool is built: everything except `journal_mode` is per-connection state, so central setup would leave every connection after the first at `busy_timeout=0` and `foreign_keys=OFF` while the code read as though they were configured. `foreign_keys` in particular is a documented **silent no-op inside a transaction**, so it must never be set from a migration — `ON DELETE CASCADE` would never fire, orphaned `sessions` would accumulate against deleted `users`, and an integrity check would pass because the constraint was never enforced.

File databases get `journal_mode=WAL` and `synchronous=NORMAL`; in-memory databases get neither, WAL being a file-database mode.

### Schema

A forward-only, append-only list of statement batches guarded by `PRAGMA user_version`, applied at startup — no migration framework and no added dependency. Everything runs in one `BEGIN IMMEDIATE` transaction including the version bump, so a crash part-way leaves the database at the version it started from. A database reporting a *higher* version than the build knows stops startup: that is a rollback to an older image, and running anyway would query columns that do not exist.

The tables are `users`, `user_shortcodes`, `sessions`, `login_codes`, `drafts`, `submissions` and `approved_records`, all `STRICT`. `drafts`, `submissions` and `approved_records` carry their body as an opaque JSON `payload` string; the permissive draft representation types it later, and this layer never interprets it.

### In-memory variant

Selected by leaving `EDITOR_DB_DIR` unset, which is the default — see [Operations](./operations.md#database) for why that is also the preview-safety default. It is a **named shared-cache URI** (`file:<name>?mode=memory&cache=shared`), never bare `:memory:`: every `:memory:` database is distinct and visible only to the connection that opened it, so each pooled connection would get its own empty copy, and with a writer/reader split readers could never see anything the writer wrote. The symptom is `no such table` that comes and goes with pool timing and test order, which reads exactly like a migration bug. Tests use a distinct name each, because a shared-cache in-memory database is scoped to the process and parallel `cargo test` threads share one.

## URL scheme

Paths are **root-mounted**. There is no `/editor` prefix.

DPE carries `/dpe/…` because it shares `repository.dasch.swiss` with other services. The editor gets its own hostname, so a prefix buys nothing — and adopting one would keep alive the path-routing option this design rejects, for the CSRF reason above.

| Path | Method | Purpose |
|------|--------|---------|
| `/healthz` | GET | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | Browser telemetry beacon. Untraced, rate-limited per IP. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

The scheme the remaining surfaces will occupy, settled up front because the router and the shell are built against it:

```
GET  /                                        → redirect to /projects
GET  /login                                   login form
POST /login                                   issue a one-time code
POST /logout
GET  /projects                                the depositor's assigned projects
GET  /projects/{shortcode}                    → redirect to the first section
GET  /projects/{shortcode}/sections/{section}  one form section
POST /projects/{shortcode}/sections/{section}
GET  /review                                  the review queue, oldest first
GET  /review/{shortcode}                      the field-by-field diff surface
```

Two decisions inside that:

- **Form sections are real URLs**, not fragment swaps. Bookmarkable, Back-friendly, and consistent with the repository's URL-based-navigation principle.
- **Review deep-links by shortcode**, not by submission id. A project has at most one pending submission, so the shortcode is unique for the purpose and reads better in a URL shared between reviewers.

## Traced and untraced routes

An Axum layer wraps only routes declared **before** it. The router therefore has two halves:

- `build_router` — everything wrapped by `OtelInResponseLayer` then `OtelAxumLayer`. `OtelInResponseLayer` is declared first so it runs *inner* and injects the `traceparent` response header; `OtelAxumLayer` is declared second so it runs *outer* and creates the server span.
- `build_app` — adds `/healthz` and `/telemetry/collect` **after** those layers, so neither is traced. A liveness probe every 30 seconds and a telemetry upload on every page view would otherwise mint a span each and bury the real traffic.

That split is positional, so it is invisible in the route table and reversible by moving one line. A test asserts `/healthz` and the beacon are absent from `build_router`.

## Datastar

The editor vendors Datastar **1.0.2**; DPE is still on 1.0.0-RC.8, and the two vendor directories are independent. See `modules/editor/public/vendor/README.md`.

Two things to get right, both of which fail quietly:

- **Keyed plugin attributes use `:`, not `-`.** `data-on:click`, `data-attr:disabled`, `data-class:open`, and `data-init` rather than `data-on-load`. This has been true since RC.6, so it matches DPE's markup too. The hyphen form produces a console error and an inert control: the page renders fine and a snapshot test asserting the attribute is present still passes.
- **`__prevent` combines with `__debounce` / `__throttle`** on 1.0.x. The rule against that combination in `docs/src/dpe/architecture.md` is about RC.8 and does not apply here.

## Styling

`modules/editor/style/main.css` is the single Tailwind entry, built by `just css-editor` (dev) or `just css-editor-release` (content-hashed). It imports the design tokens and the `mosaic-tiles` component barrel.

`@import 'tailwindcss' source(none)` means classes are collected **only** from the explicit `@source` globs, which must cover every crate that emits Tailwind classes. A missing glob produces no build error — just markup whose classes resolve to nothing. After a change that adds classes in a new location, grep the built stylesheet for them.

New Mosaic tiles are added **demand-driven**: a screen that needs a missing primitive adds it to `mosaic-tiles` with a playground showcase and a unit test at that point, rather than an up-front form kit. Their CSS goes in `mosaic-tiles/src/components/components.css`, the barrel every consumer imports.

**Check a tile against the surface you are putting it on.** Tiles are styled for light backgrounds — `link` is `text-primary-600`, which measures 2.35:1 on the footer's `bg-slate-800` and fails WCAG 2.1 AA. That is why the footer uses plain anchors inheriting `text-gray-300` (9.93:1), as DPE's does. A dark-surface variant of a tile is a design-system change, so it belongs in `mosaic-tiles` with its own showcase rather than being worked around locally.
