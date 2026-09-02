# Editor Architecture

The metadata editor is where depositing project teams edit their project metadata, RDU reviews it field by field, and approved records are collected into a pull request against this repository. Git stays the source of truth.

This page describes the service as it stands. Surfaces that are not built yet are named where they affect a decision taken now, and marked as such.

## Relationship to DPE

The editor is a **separate service** from DPE, not a section of it. They share `platform-telemetry` for the browser-beacon contract, `platform-metadata` for the research-metadata contract and `mosaic-tiles` for components — but not a process, an image, or an origin.

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

Dependency direction is `server → web → core`. `editor-web` depends on `editor-core` for the project representation it renders, and on `mosaic-tiles`; the login screens' submit buttons are the first surface to render a tile. The Tailwind entry scanned the crate before that dependency existed, so component CSS shipped regardless of it.

Unlike DPE, the **HTML document shell lives in the view crate** (`editor-web/src/view.rs`), not the server crate. DPE keeps `head()` + `page()` in `dpe-server`; here the server is a composition root for routing, auth and persistence, and a document shell is a view concern like any other partial.

## Persistence

One SQLite database, `rusqlite` with the `bundled` feature — the amalgamation is compiled by `cc` into the binary, which is what keeps the static musl image self-contained. `editor-core` owns the records and one repository trait per aggregate; `editor-server/src/db/` implements all seven against SQLite, so handlers depend on the ports and not on the driver.

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

The tables are `users`, `user_shortcodes`, `sessions`, `login_codes`, `mail_sends`, `drafts`, `submissions` and `approved_records`, all `STRICT`. Migration `0002` added `users.failed_login_at` (a lockout has to be measured from somewhere, because the counter it gates resets only on success) and `login_codes.browser_token` (the pre-auth binding — see [Authentication](./authentication.md)). Migration `0003` added `mail_sends`, the append-only send log the daily caps count; it replaced counting live `login_codes` rows, which under-reported because a sign-in deletes codes that were mailed. `drafts`, `submissions` and `approved_records` carry their body as an opaque JSON `payload` string. It holds a serialized `editor_core::draft::ProjectDraft`, which is `#[serde(transparent)]` over the project's members, so the column already contains the project object and needs no migration to become typed; the persistence layer never interprets it.

### In-memory variant

Selected by leaving `EDITOR_DB_DIR` unset, which is the default — see [Operations](./operations.md#database) for why that is also the preview-safety default. It is a **named shared-cache URI** (`file:<name>?mode=memory&cache=shared`), never bare `:memory:`: every `:memory:` database is distinct and visible only to the connection that opened it, so each pooled connection would get its own empty copy, and with a writer/reader split readers could never see anything the writer wrote. The symptom is `no such table` that comes and goes with pool timing and test order, which reads exactly like a migration bug. Tests use a distinct name each, because a shared-cache in-memory database is scoped to the process and parallel `cargo test` threads share one.

## Project representation

The editor's path is `ProjectRaw` -> draft -> `ProjectRaw`, never through DPE's `Project` view model: `impl From<&Project> for ProjectRaw` rewrites `url` into the object form and hardcodes `clusters: None`, both lossy in exactly the places REQ-1.7 requires the editor to preserve.

`editor_core::draft::ProjectDraft` is the project's JSON members rather than a struct mirroring `ProjectRaw` with 36 `Option` fields. Three requirements pull that way at once: a draft must hold a field the depositor has not filled in and a value that is present but invalid (REQ-1.9), it must carry every field the editor does not manage unchanged (REQ-1.7), and it must survive a field being added to the contract without an editor change (REQ-1.8). An absent key is a missing field, any value is retained whether it validates or not, and validity is decided once, at `to_raw`, which is the submission boundary.

The three `#[serde(untagged)]` enums therefore need no stored variant tag. Untagged deserialization takes the first variant that fits, but a value that keeps its JSON kind verbatim cannot be forced into the wrong one: a string can only be `Funding::Text`, because `Grants` needs an array. `funding_shape` and the two `*_shapes` accessors derive the variant in serde's own attempt order, so what they report can never disagree with what the written file is built from.

`url` keeps the form it was read in. Zero of the 85 committed files use the structured object form (36 hold a one-element string array, 38 a two-element array, 11 omit `url`), so writing the object form would rewrite 74 files. It is used only where there was no prior value: new projects, and those 11 files.

### The published set

`editor_core::published::PublishedProjects` reads `$EDITOR_DATA_DIR/projects/*.json` once at startup and holds them in memory, keyed by case-folded shortcode. The set cannot change without a redeployment, so nothing polls and nothing invalidates. It is not behind a repository port: the ports exist because the editor writes through them and a test has to be able to make a write fail, and this is a read of an immutable snapshot, so a trait would buy an indirection with one implementation.

Three properties of the committed corpus decide the shape, each measured over all 85 files rather than sampled:

- **The `shortcode` field is the key, not the filename.** Five files disagree with the shortcode they hold — `projects/0801_bebb.json` is project `0801d`, and its siblings under `0801_*` are `0801a` through `0801e`. Keying on the filename stem would file all five under `0801`, which no project actually has, so all five would be unreachable by the code they are addressed by and four would be dropped as duplicates.
- **Lookup folds case.** 24 shortcodes are mixed case (`080C`, `081B`, `085F`), and no two collide when folded. This matches `User::may_reach`, which folds for the same reason. Two files claiming one folded shortcode is reported rather than resolved, so which project answers can never depend on directory order.
- **Nothing about the load is fatal.** An unset `EDITOR_DATA_DIR` is a configured state (the PR preview has no snapshot), and one malformed file among 85 is a problem with the image rather than a reason to refuse every request. Both are reported at `warn` with a count and one line per failing file, because "84 of 85" is findable where an exited process says only that it exited.

`get` returning `None` does **not** mean the project does not exist. REQ-2.3 allows a project that exists only locally, whose form opens blank and whose REQ-1.1 pre-fill is empty, so a 404 needs the draft and submission records too — which is why `/projects/{shortcode}` answers 200 for an unpublished shortcode.

### Canonical form

`editor_core::canonical::write_project` is the single decision about what a `projects/*.json` file looks like: members in `ProjectRaw`'s declaration order at every depth, `null` members dropped recursively, language keys alphabetical, four-space indent, a trailing newline, non-ASCII unescaped. An approved submission is then byte-comparable with what is committed, so a review diff shows only what the depositor changed.

Two things make that work and are easy to undo by accident:

- The workspace enables `serde_json`'s **`preserve_order`**. The writer round-trips through `serde_json::Value` to strip nulls, and `Value` is `BTreeMap`-backed without that feature, which would alphabetise every key in every file. Under the feature, `Map::remove` is swap-remove: use `retain` or `shift_remove`.
- Multilingual fields are `platform_metadata::utils::Multilingual` (a `BTreeMap`), not `HashMap`. Under `preserve_order` a `HashMap` field serializes in its own randomised iteration order, which would make the round-trip test flaky.

`ProjectRaw` deliberately carries no `skip_serializing_if`: `dpe-server`'s `fragments.rs` serializes it through `axum::Json`, so the attribute would drop null members from DPE's API responses too. Stripping happens in the writer instead.

The 85-file round-trip test (`editor-core/tests/canonical_round_trip.rs`) asserts `load -> draft -> write` is byte-identical for the whole published corpus, and regenerates it under `CANONICALIZE_PROJECT_FILES=1`. Generating the corpus from the writer rather than a sibling script is the point: a script has to agree with the writer by inspection, and a near-miss surfaces later as a failing round-trip that looks like a writer bug.

### Submission checks

`editor_core::submission::unresolved_temporal_coverage` applies REQ-1.14: every `temporalCoverage` entry must resolve to a structured date, which `dpe-server validate` does not block on and OAI-PMH needs. It reuses `platform_metadata::temporal_coverage::completeness_gap`, the same decision `validate` and `dpe-api-oai`'s `every_committed_temporal_coverage_resolves` apply, and adds the entry index so the form can mark a row rather than the whole field. REQ-1.15 is settled as refusal: a depositor who needs a period the enrichment table does not know uses the `Reference` variant, which always resolves.

## URL scheme

Paths are **root-mounted**. There is no `/editor` prefix.

DPE carries `/dpe/…` because it shares `repository.dasch.swiss` with other services. The editor gets its own hostname, so a prefix buys nothing — and adopting one would keep alive the path-routing option this design rejects, for the CSRF reason above.

| Path | Method | Access | Purpose |
|------|--------|--------|---------|
| `/` | GET | public | 303 to `/projects`. |
| `/login` | GET, POST | public | The address form, and issuing a one-time code. POST rate-limited per IP. |
| `/login/code` | GET, POST | public | The code form, and spending the code. POST rate-limited per IP. |
| `/logout` | POST | public | Delete the session and clear the cookie. |
| `/projects` | GET | signed in | The projects this account may edit, named from the published set. |
| `/projects/{shortcode}` | GET | signed in + assigned | One project. 403 otherwise (REQ-1.3); 200 even when unpublished, per REQ-2.3. |
| `/depositors` | GET, POST | RDU | The account list, and creating a depositor. |
| `/depositors/new` | GET | RDU | The create form. |
| `/depositors/{id}/edit` | GET, POST | RDU | The edit form, and the change it makes. |
| `/depositors/{id}/remove` | GET, POST | RDU | The removal confirmation, and the removal. |
| `/healthz` | GET | public | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | public | Browser telemetry beacon. Untraced, rate-limited per IP. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

Every write shares a URL with the `GET` that renders its form, so a rejected submission re-renders somewhere that still answers `GET`. `POST /depositors/{id}` briefly did not, and reloading a rejected edit produced a bare 405 — the same dead end REQ-1.3's 403 is rendered as a page to avoid.

`/` is a redirect rather than a page so that exactly one place decides what a signed-out visitor gets. It is therefore absent from `page_url.rs`'s `KNOWN_ROUTES`: a redirect renders no beacon script, so no beacon can report it.

There is deliberately no resend endpoint: asking again is another `POST /login`, under the same cooldown, which keeps the number of endpoints that can send mail at one. See [Authentication](./authentication.md).

`/projects` lists the published projects a reader may reach — every project for an RDU member, the intersection of assignments and the published set for a depositor. `/projects/{shortcode}` names the project and is otherwise still a placeholder: what these two carry today is the **scope** (REQ-1.2, REQ-1.3) and the projects' names, not the editing surface. The scheme the remaining surfaces will occupy, settled up front because the router and the shell are built against it:

```text
GET  /projects/{shortcode}                    → redirect to the first section
GET  /projects/{shortcode}/sections/{section}  one form section
POST /projects/{shortcode}/sections/{section}
GET  /review                                  the review queue, oldest first
GET  /review/{shortcode}                      the field-by-field diff surface
```

Two decisions inside that:

- **Form sections are real URLs**, not fragment swaps. Bookmarkable, Back-friendly, and consistent with the repository's URL-based-navigation principle.
- **Review deep-links by shortcode**, not by submission id. A project has at most one pending submission, so the shortcode is unique for the purpose and reads better in a URL shared between reviewers.

## Request middleware

Two layers wrap the app, in this order from the outside in:

1. **CSRF** — `Sec-Fetch-Site: same-origin` is required on every non-`GET`/`HEAD` request, failing closed on everything else including an absent header. It is applied **last** in `build_app`, which makes it outermost and therefore the one layer the positional traced/untraced split cannot route around: inside `build_router` it would have missed `/telemetry/collect`, the only pre-auth POST in the app, with no test failing. See [Authentication](./authentication.md#csrf) for why `SameSite` and `__Host-` do not close this.
2. **OTel** — the traced/untraced split below.

Access control is **not** a third layer. It is two extractors, `Authenticated` and `Rdu`, and that is the design rather than an omission: a handler that names one cannot run without the check, because the argument is what runs it, and a handler that names neither is visibly public at the point anyone reads its signature. A middleware over a sub-router would have added a second positional invariant of exactly the shape this module already regrets — the traced/untraced split is invisible in the route table and reversible by moving one line — and here the failure mode is an unauthenticated route rather than a missing span. See [Authentication](./authentication.md#authorization).

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
