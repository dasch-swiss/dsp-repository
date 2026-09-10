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

Dependency direction is `server → web → core`. `editor-web` depends on `editor-core` for the project representation it renders, and on `mosaic-tiles`; the login screens' submit buttons are the first surface to render a tile. Component CSS is collected from the Tailwind entry's `@source` globs rather than from the crate graph, so it ships independently of that dependency.

Unlike DPE, the **HTML document shell lives in the view crate** (`editor-web/src/view.rs`), not the server crate. DPE keeps `head()` + `page()` in `dpe-server`; here the server is a composition root for routing, auth and persistence, and a document shell is a view concern like any other partial.

## Persistence

One SQLite database, `rusqlite` with the `bundled` feature — the amalgamation is compiled by `cc` into the binary, which is what keeps the static musl image self-contained. `editor-core` owns the records and one repository trait per aggregate; `editor-server/src/db/` implements all nine against SQLite, so handlers depend on the ports and not on the driver.

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

The tables are `users`, `user_shortcodes`, `sessions`, `login_codes`, `mail_sends`, `drafts`, `submissions`, `review_rounds`, `approved_records` and `entity_proposals`, all `STRICT`. Migration `0002` added `users.failed_login_at` (a lockout has to be measured from somewhere, because the counter it gates resets only on success) and `login_codes.browser_token` (the pre-auth binding — see [Authentication](./authentication.md)). Migration `0003` added `mail_sends`, the append-only send log the daily caps count; it replaced counting live `login_codes` rows, which under-reported because a sign-in deletes codes that were mailed. `submissions.review_state` (the per-field decisions and substitutions RDU records while a review is in progress) and the whole `review_rounds` table are in `0001` rather than in a fourth migration: the service has never been deployed, so a migration would record a history nobody lived through. The list becomes append-only at the first deployment. `entity_proposals` is in `0001` for the same reason. `drafts`, `submissions` and `approved_records` carry their body as an opaque JSON `payload` string. It holds a serialized `editor_core::draft::ProjectDraft`, which is `#[serde(transparent)]` over the project's members, so the column already contains the project object and needs no migration to become typed; the persistence layer never interprets it.

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

### Entity proposals

A depositor can propose a new person or organisation, or a change to one their project references (US-3). Proposals live in `entity_proposals`, **not** in the draft payload: `ProjectDraft` is `#[serde(transparent)]` over the project's members and `to_raw` deserializes it into `ProjectRaw`, so a non-contract member would be dropped on the way to a file with nothing saying so.

The table is keyed by shortcode rather than being a child of `submissions`, although REQ-3.3 carries a proposal inside the project's pending submission. Every review outcome deletes the submission row while the proposal outlives it — an accepted one is on its way into a `persons/` or `organizations/` file, and a returned one is still the depositor's to finish. `review_rounds` is keyed the same way for the same reason.

**`status` and `decision` are separate columns.** `status` is the lifecycle (`draft`, `submitted`, `accepted`, `rejected`, `withdrawn`); `decision` is what RDU recorded in the round now running (`accept`, `reject`, or null while undecided). The split is the one `submissions.review_state` makes for a project field: a decision is taken during a review and only becomes a status when the round ends, which is what lets request-changes hand the proposal back as a draft while retaining what was decided — REQ-4.5 requires exactly that for fields, and a proposal reviewed on the same surface must not lose it.

Two predicates read those, and confusing them is the mistake to avoid. `EntityProposal::is_live` (`draft` or `submitted`) is "still in play — the depositor may edit it and a reviewer may still decide it". `is_referenceable` adds `accepted`: an accepted entity is not a file yet, so a project field naming it must still resolve, while a rejected one must fail resolution — that failure is what makes the referential-integrity gate possible. **Neither says anything about the allocated id**, which every row holds permanently.

#### Id allocation is collision-free within the editor, and never reuses

REQ-3.6 allocates `person-NNN` / `organization-NNN` at proposal time; REQ-5.4 renumbers only on collision *with the repository*, so nothing in the requirements stops two proposals inside the editor taking the same next id. Two things close that:

- **The allocation is one transaction.** `EntityProposalRepository::create_new` selects the ids already taken and inserts the new row inside a single `write` closure, which is `BEGIN IMMEDIATE` on the pool's single writer connection. The second of two concurrent proposals always sees the first one's insert.
- **A partial unique index makes it structural.** `entity_proposals_allocated_id` is `UNIQUE (entity_id) WHERE operation = 'new'`. It is partial because a `change` names an id somebody else allocated, and several projects may propose changes to one entity.

The allocator is `proposals::next_entity_id`: one past the highest number it has seen, over the union of the published store and every id the editor has ever allocated — **terminal rows included**. Gaps are therefore deliberate. Nothing depends on the sequence being dense, while reuse would hand an id to a second entity after a sibling collection pull request may already carry it. `Agents::highest_id_number` supplies the published half, which the persistence layer cannot see.

A second partial index, `entity_proposals_live_per_entity` on `(shortcode, entity_id) WHERE status IN ('draft', 'submitted')`, allows two projects to hold change proposals for one entity but not one project to hold two live ones for it. Two competing rows would show a reviewer separate decisions over one file, and whichever applied last would silently win.

#### A change proposal is seeded with the whole published entity

Accepting a change proposal writes its payload as the entity file, so a payload holding only the members a form happens to render would silently drop `affiliations`, `sameAs`, `email`, `alternativeName`, `canton` and `additional`. This is the property `ProjectDraft` gives a project — carry every member the editor does not manage unchanged (REQ-1.7), survive a member added to the contract without an editor change (REQ-1.8) — and an entity needs it for the same reason, so `Agents` keeps each entity's file body and `Agents::seed_payload` hands it over with `id` removed. A save merges into the stored payload rather than rebuilding it.

**The payload never carries `id`.** It lives in `entity_id`, the column the allocator and the uniqueness index work on; a second copy would be free to drift. Every reader fills it in from `entity_id` and overwrites whatever it finds, so a payload that does carry one cannot make an entity resolve under an id nothing claimed.

#### The entity form

`GET | POST /projects/{shortcode}/entities/{proposal}`, plus the two row-action paths the repeatable fields need. A write shares the `GET` that renders it, like every other write here.

`{proposal}` is a proposal's **`entity_id`** (`person-417`), not its row `id`. That is the value the propose controls post back and the summary links carry, and it makes a readable URL. A project may hold several rows for one `entity_id` over time — a withdrawn proposal, then a fresh one — but never more than one *live* one, because `entity_proposals_live_per_entity` says so; the resolver prefers the live row and otherwise the most recently touched, so a stale link still shows something coherent. A proposal under the wrong shortcode is a 404 rather than a 403, for the reason an unknown section id is: the reader invented the pairing.

**A save merges into the stored payload and never rebuilds it.** That is the seeding property above, enforced at the write: only the members this form posts are applied, so anything else in the payload survives untouched.

`jobTitles` is the one field whose applier needs help. `Shape::StringRows` removes a member when no row survives, which is right for every field of that shape except this one: `check_person` reads an *absent* `jobTitles` as unanswered and an *empty* one as a person with no job title, which 59 of the 416 committed persons already are. So an emptied `jobTitles` is kept as `[]` rather than dropped.

REQ-3.1's start controls live on the agent rows themselves, where the picker already says an id resolves to nobody — the form is where it can be fixed, the same argument that comment makes about saying it at the row as well as at submit. Both a person and an organisation are offered, because the picker cannot know which was meant. A row whose id *does* resolve offers propose-changes instead (REQ-3.2). All three are named submits on the section's own form, so they need no route of their own.

#### An incomplete address inherited from the published entity is passed through

REQ-3.4 asks a proposed organisation for all four of `street`, `postalCode`, `locality` and `country`, or no `address` at all. Six of the 142 committed organizations satisfy neither — `organization-009`, `-033`, `-065`, `-089`, `-090` and `-137` are missing `street`, `postalCode` or both, and three of them sit outside Switzerland where a postal code may not apply. Applied literally, the rule would leave a depositor proposing any other change to one of those six a choice between inventing a street and deleting a locality and country that *are* recorded.

So `check_organization` judges what the depositor wrote, not what they inherited: an `address` byte-equal to the published one passes, and any other incomplete one is refused. This is the carve-out `form::submit::typed_sentinels` already makes for a reference's `url` — `0110_h-steiner` holds `{"url": "MISSING"}` — and for the same stated reason: a live record must not become unsubmittable over data it did not write. `proposals::tests::six_committed_organizations_carry_an_incomplete_address` enumerates the six and fails if the corpus stops needing the carve-out.

#### The review surface gives a proposal its own rows

REQ-4.3 reviews changed project *fields*, and a proposed person is not one, so entity proposals would otherwise reach an approval without ever being displayed — and PRD Edge Case 3's referential-integrity question could not even arise, there being no per-entity action to invoke. Proposed entities therefore get their own rows below the field diff, each with an accept/reject control posting under `entity.{entity_id}`. That namespace cannot collide with the field surface's `decision.{field}` or with a member name.

Approve gains two refusals beside the undecided-fields one:

- **while any proposal is undecided**, for the reason the field gate exists: approving an undecided row commits bytes nobody looked at. `review_rounds`' `approve` leaves an undecided proposal `submitted` and names this gate as what prevents that, so the two halves are one coupling.
- **while the payload still references a rejected entity** — PRD Edge Case 3. The approval is blocked and the referencing fields are named, so RDU substitutes them in place or requests changes; nothing is stripped silently.

The second gate has a trap worth stating, because getting it wrong disables it without any test failing. At approve time every proposal's `status` is still `submitted` — the statuses change inside `approve`'s own transaction — so `is_referenceable`, which is status-based, answers **true** for a proposal RDU has just rejected. The scope the check runs against is therefore built from only the proposals that will *survive* the approval, filtered on `decision`, and the check runs over the **decided** payload rather than the submitted one, since a revert can remove a reference and a substitution can add one. It reuses `form::submit::unresolved_agents` rather than walking references a second way.

#### What the two projects case does

Two projects may propose changes to one entity, and the last collected wins. The review surface says so rather than blocking: blocking would strand one project on another project's review, while a silent overwrite would let RDU approve a change that is about to be replaced with nothing saying so.

### The form

`editor_core::form` reads a posted body back into a draft; `editor_web::form::registry` says what the form knows about each field. The split is deliberate: the decoder knows *shapes* (a scalar, a language map, a closed choice, a URL slot, a list of strings, and four kinds of row) and has no idea which field is which, so choosing a shape per field belongs with the registry, keyed by the same field ids as the renderers. A field's control and its decoder are then declared together and cannot drift, and the audience check — which fields a depositor may write at all — has one home rather than one in the view and one in the handler.

The body is read as `Form<Vec<(String, String)>>`, not as a struct. `axum::Form` deserializes with `serde_urlencoded` 0.7 (via axum 0.8), which **errors** on a repeated key rather than collecting it into a `Vec`, and cannot deserialize a struct containing a `Vec` at all — so a checkbox group and a repeatable list's row keys have no struct representation. The pair list gives body order with duplicates intact, which is what opaque row keys plus DOM order need, and adds no second urlencoded parser. `serde_html_form`, which does decode repeated keys, is not a dependency of this tree.

Three rules make an untouched save a no-op, all three pinned by `editor-web/tests/untouched_form_round_trip.rs`:

- A stored `MISSING`/`CALCULATED` placeholder survives an empty submit. Those sentinels are filtered out of DPE's UI and of OAI-PMH's output, so a control holding one renders empty and an untouched form posts empty for it. 131 across the 85 files, 24 of them `endDate`.
- A value differing from the stored one only in surrounding whitespace is left alone; a genuinely new value is stored trimmed. Counted over the whole corpus rather than over the fields the form happens to read today: 20 of the 85 files carry a leading or trailing space somewhere, one of them (`0816_vitrocentre.json`, `shortDescription`) in a field a declared shape already reads. The rest sit in `disciplines.text` (7 files), `publications.text` (6), `attributions.contributorType` (3), `abstract.en` (2), and one each in `keywords.ar`, `description.ar`, `spatialCoverage.text` and `legalInfo.license.licenseURI` — so **every field now reads one of them**, and the row appliers had to grow a way to preserve them: `resolve_against` looks a submitted value up against every value the field already holds, rather than against one position, because a row key is opaque and need not map to a position — after one removal the body carries `r0` and `r2` against a two-row list, so a positional lookup would preserve one row's bytes into another.

  It prefers an **exact** byte match before the whitespace-insensitive one, which is not redundant: `0121_societesavoie` holds both `"Project Member, Data Collector"` and the same value with a trailing space, in one project, so the loose comparison cannot tell them apart and whichever came first rewrote the other. Both of those were caught by the corpus round trip and neither by a unit test — the reason that test runs over committed bytes rather than a fixture.
- Newlines are normalised to `\n` — a native submit posts CRLF where `FormData` posts LF, so the no-JS and enhanced paths would otherwise write different bytes for the same value in 26 of the 85 files. Normalisation applies to **both sides of the comparison but only to a value being stored**, because 10 committed abstracts hold a bare `\r` that a `<textarea>` converts to `\n` before any submit.

#### Where a field's shape and empty state are declared

`registry::Field` carries a `shape: Option<Shape>`, and that is the only place either is stated. `Shape` is `editor_core::form`'s, one arm per applier, and `editor_core::form::apply` is the single entry point a handler uses — so naming a shape is the only way to reach an applier, and the shape a field declares is the applier that runs.

`WhenCleared` rides *inside* `Shape::Text` rather than beside it. It is meaningful for nothing else — a language map's empty state is "no tags", and there is no placeholder to write — so carrying it separately would have let a `Multilingual` field declare one and a `Text` field declare none. Passing it per call would let a handler disagree with the registry: `Drop` on a field the contract types as a required `String` makes every ongoing project unpublishable until an end date it does not have is entered, with no test failing.

Two registry tests hold it to the data rather than to a list repeated in prose:

- `Text(Placeholder)` requires the member to be present in **all 85** committed projects, which is what a required `String` looks like; `Text(Drop)` requires it to be null or absent in at least one, which is what an `Option` looks like. That is the inversion above, checked in the direction the corpus can actually decide.
- A declared shape has to match the JSON kind the contract holds, so a `Multilingual` on a string member — a control posting under names no applier reads, and a save that is silently a no-op — fails rather than shipping.

A field with **no** shape is display-only (REQ-1.5, written back unchanged per REQ-1.7). It used to also mean "a control that has not landed", of which there were eighteen; that set is now empty, and the registry test that pinned it exactly is what keeps it so — a new `ProjectRaw` member placed in a section without a shape fails there rather than rendering as a note nobody notices.

The note itself stays in `widgets::stated`, unreached by any field today and deliberately kept: it is the fallback such a field lands on, and a depositor who cannot find a field the published page shows would otherwise conclude the form lost it.

`untouched_form_round_trip.rs` derives its table from `FIELDS` and therefore lives in **`editor-web`**: the dependency direction is `server -> web -> core`, so a test in `editor-core` cannot read the registry at all. Deriving it is the point — a field whose shape is declared is covered automatically, where a hand-written table agrees with the registry only by inspection.

A depositor who types the word `MISSING` is refused at submit, by `form::submit::typed_sentinels`. The sentinels are recognised by an exact string match and a submitted value is stored verbatim, so typing one stores something the rest of the platform reads as "no value" — filtered out of DPE and of OAI-PMH — and because a recognised placeholder renders as an empty control, the next empty submit leaves it alone rather than clearing it. The field then reads as empty, will not clear, and is only editable by typing some other value first.

The check reads the **declared shape**, not the value, because a stored sentinel is usually correct: `endDate` is `"MISSING"` in 24 of the 85 committed projects, and `WhenCleared::Placeholder` means the editor writes one itself when a depositor clears the field. So the question is not "is this a sentinel" but "could clearing this field have produced one" — `Placeholder` yes and allowed (typing one is then indistinguishable from clearing, and has the same effect), `Drop` no and refused, `Multilingual` no and refused per tag, since an empty text drops its tag. Deriving it from the shape also answers for a project with no published counterpart (REQ-1.1), which a comparison against the published value could not.

**One number caps what a field may carry, and it is both bounds at once.** `MAX_VALUES_PER_PREFIX` is 64. As a bound on **work** it always mattered: every `FormBody` reader is linear in the number of pairs, and `entries` — which discovers a field's language tags — returns each value with its suffix rather than leaving the caller to fetch each with `get`, which would be quadratic. `DraftMultilingual` is an order-preserving `Vec` whose `get` and `set` scan, right for a map the data holds two entries of and wrong for one holding twenty thousand: 20,000 tags under one prefix measured 2.6 s of CPU in a debug build, from a single request, against Axum's 2 MB limit of roughly 100,000 short pairs.

It is now also the cap a depositor can **see**. `form::submit::over_cap` refuses a body carrying more, with a field-level error naming the field and the number, and `FormBody::exceeds_entries` is what counts — `entries` cannot, because it stops at the cap, so through it a body of twenty thousand tags and one of sixty-four are indistinguishable.

Two facts about where it runs, both load-bearing:

- **Before any applier, and on a save as much as a submit.** An applier truncates silently, so an over-cap save left to submit would store the truncated value and the submit after it would see a draft already within the cap and pass. Nothing is written on the refused branch, which is what makes "nothing was saved" true.
- **One number, not a visible cap above the work bound.** A product cap set higher would silently discard every value between the two — exactly the failure a visible cap exists to remove. So `entries`' truncation is now a fail-safe floor rather than the behaviour: unreachable through the route, kept because `FormBody` is public and a future caller might not check first.

Sixty-four is sixteen times the four tags the UI offers and twenty-one times the three the widest committed language map holds, and a test asserts the cap stays above what the corpus carries — so the day a project needs more, the cap is raised deliberately rather than a save being refused.

### Canonical form

`editor_core::canonical::write_project` is the single decision about what a `projects/*.json` file looks like: members in `ProjectRaw`'s declaration order at every depth, `null` members dropped recursively, language keys alphabetical, four-space indent, a trailing newline, non-ASCII unescaped. An approved submission is then byte-comparable with what is committed, so a review diff shows only what the depositor changed.

Two things make that work and are easy to undo by accident:

- The workspace enables `serde_json`'s **`preserve_order`**. The writer round-trips through `serde_json::Value` to strip nulls, and `Value` is `BTreeMap`-backed without that feature, which would alphabetise every key in every file. Under the feature, `Map::remove` is swap-remove: use `retain` or `shift_remove`.
- Multilingual fields are `platform_metadata::utils::Multilingual` (a `BTreeMap`), not `HashMap`. Under `preserve_order` a `HashMap` field serializes in its own randomised iteration order, which would make the round-trip test flaky.

`ProjectRaw` deliberately carries no `skip_serializing_if`: `dpe-server`'s `fragments.rs` serializes it through `axum::Json`, so the attribute would drop null members from DPE's API responses too. Stripping happens in the writer instead.

The 85-file round-trip test (`editor-core/tests/canonical_round_trip.rs`) asserts `load -> draft -> write` is byte-identical for the whole published corpus, and regenerates it under `CANONICALIZE_PROJECT_FILES=1`. Generating the corpus from the writer rather than a sibling script is the point: a script has to agree with the writer by inspection, and a near-miss surfaces later as a failing round-trip that looks like a writer bug.

### Submission checks

`editor_core::submission::unresolved_temporal_coverage` applies REQ-1.14: every `temporalCoverage` entry must resolve to a structured date, which `dpe-server validate` does not block on and OAI-PMH needs. It reuses `platform_metadata::temporal_coverage::completeness_gap`, the same decision `validate` and `dpe-api-oai`'s `every_committed_temporal_coverage_resolves` apply, and adds the entry index so the form can mark a row rather than the whole field. REQ-1.15 is settled as refusal: a depositor who needs a period the enrichment table does not know uses the `Reference` variant, which always resolves — and since the variant chooser landed that escape route is one a depositor can actually take, where before the refusal named a way out the form did not offer.

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
| `/projects/{shortcode}` | GET | signed in + assigned | 303 to the first form section. 403 otherwise (REQ-1.3). |
| `/projects/{shortcode}/sections/{section}` | GET, POST | signed in + assigned | One form section, and the save, autosave, submit, withdrawal or discard it makes. 200 even when the project is unpublished, per REQ-2.3. |
| `…/sections/{section}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field. Under the section's URL so it resolves through the same `context()`. |
| `…/sections/{section}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row. The key is in the path, not a button's name and value, because a programmatic submit omits the submitter's. |
| `/projects/{shortcode}/entities/{proposal}` | GET, POST | signed in + assigned | One entity proposal's form, and the save or discard it makes. `{proposal}` is the proposal's `entity_id`. |
| `…/entities/{proposal}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field on the entity form. |
| `…/entities/{proposal}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row, for the reason the section's own row path gives. |
| `/review` | GET | RDU | The review queue: every pending submission oldest first, and every draft. |
| `/review/{shortcode}` | GET, POST | RDU | The field-by-field diff, and the claim, decision save, approve, request-changes or reject it makes. |
| `/depositors` | GET, POST | RDU | The account list, and creating a depositor. |
| `/depositors/new` | GET | RDU | The create form. |
| `/depositors/{id}/edit` | GET, POST | RDU | The edit form, and the change it makes. |
| `/depositors/{id}/remove` | GET, POST | RDU | The removal confirmation, and the removal. |
| `/healthz` | GET | public | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | public | Browser telemetry beacon. Untraced, rate-limited per IP. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

The two row paths are the one exception to the rule below, and deliberately: they are `POST`-only because both change the form and a `GET` that did would be a state change on a `GET`. They never strand a refusal, because they re-render the section rather than redirecting, and a reload of one lands on the section's own `GET`.

Every write shares a URL with the `GET` that renders its form, so a rejected submission re-renders somewhere that still answers `GET`. A write-only path leaves a reloaded rejection at a bare 405, the same dead end REQ-1.3's 403 is rendered as a page to avoid.

`/` is a redirect rather than a page so that exactly one place decides what a signed-out visitor gets. It is therefore absent from `page_url.rs`'s `KNOWN_ROUTES`: a redirect renders no beacon script, so no beacon can report it.

There is deliberately no resend endpoint: asking again is another `POST /login`, under the same cooldown, which keeps the number of endpoints that can send mail at one. See [Authentication](./authentication.md).

`/projects` lists the published projects a reader may reach — every project for an RDU member, the intersection of assignments and the published set for a depositor. `/projects/{shortcode}` is a **redirect** into the form's first section, so exactly one place decides where a project link lands, and there is no per-project landing page between the list and the form. It is therefore absent from `page_url.rs`'s `KNOWN_ROUTES` for the same reason `/` is: a redirect renders no beacon script, so no beacon can report it. The redirect target is the same section for both audiences — a destination that depended on the role is one more thing to get wrong in a link shared between a depositor and a reviewer.

Two decisions about that scheme:

- **Form sections are real URLs**, not fragment swaps. Bookmarkable, Back-friendly, and consistent with the repository's URL-based-navigation principle.
- **Review deep-links by shortcode**, not by submission id. A project has at most one pending submission, so the shortcode is unique for the purpose and reads better in a URL shared between reviewers.

### The form's two renderings

`POST /projects/{shortcode}/sections/{section}` is one handler answering two ways, discriminated on the `Datastar-Request` header the vendored bundle sets on every fetch it makes:

| path | outcome | answer |
|---|---|---|
| no script | saved | `303` to this section's `GET` |
| no script | refused | `200`, the whole page re-rendered at the same URL |
| Datastar | saved | `200`, the section region as `text/html` |
| Datastar | refused | `200`, the same |

The plain path redirects because a `POST` left in the history re-posts on refresh. The enhanced path does not need to and must not: it never navigated, so a refresh re-issues the last `GET` — and a 303 followed by a full document would hand Datastar an `<html>` to patch. A refusal re-renders on both paths, because a redirect would throw away what the depositor typed.

Three things about the enhanced path fail quietly if changed:

- **A `text/html` response *is* an implicit `datastar-patch-elements`.** With no selector it matches by `id` in `outer` mode, so what comes back is the region under one id, not a document.
- **The region is bigger than the form.** It is the rail, the status and the form together — everything a save can change. Returning only the `<form>` leaves the rail showing the obligation counts from *before* the save, so the depositor fills in the last required field, the field goes quiet, and the rail still says something is missing.
- **A refusal must still answer 200.** Datastar processes a response body only on a 200; any other status aborts the fetch and the message the response was carrying never reaches the page. The outcome goes on the span instead, which is where alerting reads it from — the same reasoning as the account forms' redisplayed 200.

`data-on:submit` carries no `__prevent`: the bundle calls `preventDefault` unconditionally for a `submit` event on a form element, so one would be noise. The form is *not* `novalidate` and no field is `required`, which looks contradictory and is not — see `editor-web/src/pages/section.rs`, where the `type="date"` reason is argued.

### Three accessibility decisions the markup depends on

Each of these fails silently: the page renders correctly, and only a screen-reader user or a keyboard user notices.

- **A field's obligation lives inside its own `<label>` (or `<legend>`), not beside it.** No input carries `required` or `aria-required`: a draft may be missing anything (REQ-1.9) and a browser refusing to save one is the opposite of REQ-1.10. That leaves the accessible name as the only channel the tier has, so a pill rendered as a sibling is visible and nothing else: a reader tabbing to the control hears "Name, edit text". Five control builders compose their own label, so one test asserts the obligation per field across every section rather than on an example; forgetting it in one builder renders identically to a sighted reader.
- **The status region is `aria-live="polite"` and holds no element with a live role of its own.** A refusal renders `AlertVariant::Warning` rather than `Danger` for that reason: `Danger` carries `role="alert"`, an implicit *assertive* region, and screen readers do not agree on which politeness wins when one is nested inside a polite region — some interrupt, which is the behaviour the polite region exists to avoid. The region announces; the alert only styles.
- **A rail link states its accessible name.** The section title and its progress are adjacent `<span>`s with no whitespace between them, because a flex column is what puts them on two lines — so the name computation concatenates them into "Overview5 of 5 required". The `aria-label` starts with the visible title, which is what WCAG 2.5.3 asks of an `aria-label` over visible text, and is omitted for a section with no requirements where the title is already the whole name.

The colour pairings are measured against the design tokens, with the method cross-checked on the four ratios `text_field.css` already documents: `warning-800` on `warning-50` 11.30:1, `info-800` on `info-50` 11.18:1, `neutral-700` on `neutral-100` 7.78:1, and `neutral-600` on the `gray-50` page 5.73:1 — all above WCAG 2.1 AA's 4.5:1 for the 12px bold pill text and the hint text.

### The draft's storage key folds case

`drafts.shortcode` is an exact-match column, while `PublishedProjects::get` and `User::may_reach` both fold ASCII case — the published set mixes `080C` with `0801a`, so a link typed either way reaches the same project. Keying a draft on the path segment as typed would therefore give `/projects/080c` and `/projects/080C` a **row each** for one project, and two people editing it would each keep half the edits with nothing to say so. The section handler folds, in one named place, and `is_valid_shortcode` admits only ASCII alphanumerics so the fold is total and agrees with the other two by construction.

The form's own behaviour — which fields submit insists on, how an agent reference is picked, where a vocabulary is closed, how a variant row narrows, row actions, discard, the concurrent-save refusal and autosave — is in [The Project Form](./project-form.md), together with the corpus measurements those decisions rest on.

## The review surface

`GET /review` is the queue and `GET /review/{shortcode}` the field-by-field diff. Both take the `Rdu` extractor, which puts the access rule in one place — access is role-based, so there is no assignment to check and no per-project 403 to render, and an RDU account's assignment set is empty by design. The queue carries a second table of every draft, because those are visible to RDU too; a draft is not reviewable, so it is a separate table rather than a row with no controls.

### The diff is one form, not one request per field

The surface offers accept, revert and edit-in-place *per field*. That does not mean a request per field, and what settles it is what batching is for: a reviewer who accepts eight fields and loses the ninth to a dropped connection has a submission half-decided with nothing saying which half. One `<form>` posting one body is that batching natively — every decision and every substituted value arrives together and is written in one transaction — and it keeps the surface working without JavaScript, which every other authenticated surface here does.

Three things hold it together, each of which fails quietly if changed:

- **A decision posts under `decision.{field}`; a substituted value posts under the field's own name** — `{field}` for a scalar, `{field}.{tag}` for one language of a map. That is exactly what the section form posts, and therefore exactly what `editor_core::form`'s appliers read. No registry id begins with `decision.`, so the two namespaces cannot collide.
- **A substitution is computed by running the field's own applier over a clone of the submitted draft**, then comparing. That is the only way a reviewer's edit obeys the rules a depositor's does — trimming, newline normalisation, a stored `MISSING` surviving an empty submit — rules whose whole purpose is that an untouched value writes no bytes. A second comparison here would agree with them only by inspection.
- **The in-place editor *is* the depositor's control**, `editor_web::form::widgets::control` over a one-member draft holding what would be committed. A second dispatch diverged as soon as it existed: keyed off whether the value happened to hold a newline, `startDate` rendered as free text where the form gives a date picker, and `shortDescription` lost the 200-character cap its own hint promises — neither caught server-side, because the cap is an HTML attribute.
- **The intent rides on the submit button**, as `name="intent"` — one definition, in `editor_web::form`, since both write surfaces post the same pair. A native submit posts the activated button's name and value, and Datastar 1.0.2's form mode appends them too, from `SubmitEvent.submitter`, so every control on this surface is a named submit on one form: "Save review decisions", "Accept all remaining" and the three that finish the round. `formaction` would not do — the bundle posts to the URL in `@post`, so a second destination is silently ignored on the enhanced path and honoured on the plain one. It also means the terminating controls live **inside** the diff form: the note and every recorded decision have to arrive with the action, or approving would commit the decisions as they were last *saved* rather than as they stand on screen, a difference nothing on the page would explain.

`POST /review/{shortcode}` answers the same two ways as the form's save, for the same two reasons: a 303 on the plain path so a `POST` left in the history does not re-post, and the region as `text/html` on the enhanced one, always 200 because Datastar processes a body only on a 200.

### The submitted payload is never rewritten

A reviewer's substitution goes to `submissions.review_state`, never over `payload`. A depositor's submission needs no second approver, so a value RDU put in place of the depositor's is seen by nobody unless the submitted one survives beside it — and an overwritten payload cannot answer what was submitted. `Some(Value::Null)` in a field's stored review is a reviewer *clearing* a field, which is a real substitution and not the absence of one.

Only fields the submission actually changes are read back, and a revert is refused on a project with no published counterpart. Both are the same rule: an unchanged field renders no decision control and an unpublished project never offers revert, so a decision naming either came from a hand-built body — and storing one records a decision the surface can never show and therefore never undo. A stored revert on an unpublished project renders "Reverted — keeps published" beside a "Not published yet" column, in a radio group with no matching option, so it reads as undecided and cannot be cleared.

### The comparison is over top-level members, not registry fields

`editor_core::review::diff` compares the union of both sides' JSON members. Enumerating the registry instead would show a reviewer only the fields the *form* knows, so a change arriving through any other path — a member no applier touches, a field added to the contract without an editor change — would be approved without ever being displayed. A member the registry does not know keeps its own name as its label, and a member the submission *dropped* is still a row: a removal is a change somebody has to see.

Equality is on the stored `Value`, which is stricter than the comparison `editor_core::form` applies to a submitted value. It has to be: those forgiving rules exist so that *saving* an untouched form writes no bytes, and by the time a submission exists they have already run. What survives them is a real difference in what would be committed.

### A project with no published counterpart

A project can exist only locally while the comparison assumes a published value per field, so for the first project created through the editor it degenerates. The surface says so once, in a banner rather than per field — it is a fact about the record, not about any one field, and a paragraph rendered beside a control is not part of that control's accessible description, so a reader tabbing straight to the input would never hear it. Per row it renders the published column as "Not published yet" rather than "Not set", since a reader told the latter looks for the field rather than for the record. And it **offers no revert**. Revert means keeping the published value, and there is none; offering it would silently unset a field the contract requires. Accept and edit-in-place still apply, which is the whole of what a reviewer can do to a record that is new.

### Concurrent review is visible, not locked

Two RDU members can open one submission and the PRD defines no claim. Opening one from the queue is a `POST` that claims it — `Submitted` to `InReview`, `reviewed_by` set — which is also the first producer of `SubmissionState::InReview`, a state the project form already reads as a reason to lock the depositor out. A second reviewer is told who has it and offered a take-over; nothing is blocked and the last save wins, as it does for a draft. The reader is only shown as the holder once the write has actually succeeded — naming them on a refusal would also suppress the take-over banner, telling them the opposite of what the row says at exactly the moment it matters. The take-over carries the current filter, for the same reason the diff form does.

A lock was the alternative and costs more than it buys: it needs a release path and a stale-lock timeout, and strands a submission whenever somebody closes a tab. What must not happen *silently* is one reviewer overwriting another, and the banner plus the queue's "With …" column is what stops that.

Claiming is a `POST` and not a `GET` because it changes state, and the `Sec-Fetch-Site` control exempts `GET` by necessity — a navigation from anywhere is a `GET`. It shares the review URL rather than taking one of its own, so a refused claim re-renders somewhere that still answers `GET`.

### Ending a review round

Approve, request-changes and reject are three `intent` values on the existing `POST /review/{shortcode}`, and the depositor's own withdrawal is a fourth on `POST /projects/{shortcode}/sections/{section}`. No new routes: a write sharing the `GET` that renders its form is the rule the whole service follows, and `page_url.rs`'s `KNOWN_ROUTES` is therefore untouched.

All four end the round the same way — the `submissions` row is deleted — so all four go through `ReviewRoundRepository`, whose three writing methods each run in **one** transaction spanning three tables. The delete's own row count is the terminal-state guard, and it has to come before every other statement in the closure; `editor_server::db::review_rounds`' module documentation states the two properties that depend on that and what breaks without them.

**Approve is always an explicit click, for an RDU member's own submission too.** The PRD removes the second *approver*, not the approve step, and it also asks that RDU direct editing produce a pending submission identical in shape to a depositor's — which auto-approval would make never pending. So nothing distinguishes an RDU member's own submission from anyone else's: there is no second-approver step to waive and no self-approval check to add.

**Approve is refused while any changed field is undecided.** Committing an undecided row ships bytes nobody looked at, which is the one thing a field-by-field surface exists to prevent, and "Accept all remaining" makes clearing it a single click — so the refusal is never a dead end. What an approval commits is the submitted draft with every decision applied: an accepted field takes the reviewer's substitute where there is one, and a reverted field goes back to the published value, or is removed where the published side has no such member. `submissions.payload` stays the depositor's own until the row is deleted.

**Request-changes and reject both require a note; approve does not.** For reject the note is the entire signal: the submission is discarded, notifications are out of scope, and the depositor-facing state list has no Rejected — so without a note the work vanishes with nothing saying why. For request-changes it is the only thing telling the depositor what to change. An approval needs none, because the depositor is *shown* what changed rather than told about it.

**Reject and withdraw both leave the draft.** Reject must not destroy work RDU merely declined, and a withdrawal reads as "take it back so I can keep editing" — request-changes already establishes submission-becomes-draft as the direction. The abandoned-draft problem belongs to [discarding a draft](#discarding-a-draft), not to these.

Each of the four asks once before writing, on the same URL, discriminated by a hidden `confirmed` pair the prompt carries. The intent names *what* is being done and stays the same across both posts, so the two-step shape does not double the verb list — and a body naming an intent without `confirmed` gets the prompt, never the write. An unknown intent falls back to a plain save on both surfaces: every terminating action is irreversible and a save is not.

### What a finished round leaves, and where the depositor reads it

`review_rounds` is append-only: one row per finished round, written by the transition that ended it and never updated, so the rows for one project are its review history. One table rather than four columns, because four separate things read it — a rejection being visible at all, a returned draft being distinguishable from one never submitted without adding a sixth state, the per-field accepted state surviving the return, and the depositor seeing what RDU substituted for their values. The last two read the round's `review_state` snapshot, which is a snapshot and not a reference because the submission carrying it is deleted by the same transaction.

The depositor reads all of it on the project form, inside the region a save replaces, and **every outcome is shown** with its own wording — one banner reading "RDU asked for changes" whatever happened would tell somebody whose work was rejected to answer a closed round. It shows until the next submission, which starts the next cycle. `drafts` carries no note column: the note has to be read beside the outcome it belongs to, and only the round has both. A save cannot disturb it, which the column needed a rule to guarantee.

**Fields RDU accepted are fixed while the round is being answered.** The per-field state is retained across the return, and nothing stopped the depositor altering an accepted field, which then re-entered review still flagged accepted. The gate is that `sections::act` skips those fields' appliers: not rendering the control stops an ordinary browser, and only the skip stops a hand-built body. It applies **only** while the latest round asked for changes — a reject's or a withdrawal's decisions are moot, since the submission they were recorded against is gone. A reverted field is deliberately not locked: its submitted value was discarded, so the depositor has nothing to preserve and every reason to try again.

### Submit

`POST /projects/{shortcode}/sections/{section}` with `intent=submit` records the draft as the project's pending submission. The draft is written first and on both intents, so a refused submission costs the depositor the submission and never the editing.

Seven gates, in order:

1. **The draft must be a complete `ProjectRaw`.** A type-level failure means a member the contract requires has no value, and no per-field rule below can say anything useful about a shape that does not exist.
2. **Every `Obligation::Required` field the submitter sees must be answered**, through `obligation::unsatisfied_required`. Narrower than it looks beside gate 1 and not redundant with it: every required field is a non-`Option` contract member, so an *absent* one already failed above, and what this catches is present-and-empty — `[]`, `{}`, `""`, a `MISSING` sentinel. It reads presence through the same function the section rail counts with, and the tier it gates on is bounded by what the published corpus answers; see [Obligation is a submit gate](#obligation-is-a-submit-gate-and-the-corpus-bounds-which-fields-carry-it).
3. **Every agent reference must resolve**, through `form::submit::unresolved_agents`. The applier stores whatever id arrives, because a draft may hold a value that does not validate (REQ-1.9) — this is what stops an unresolvable reference reaching a published file, where the public project page would render a bare `person-001`. It resolves against the published store **plus this project's own referenceable proposals**, so a depositor can reference an entity they have just proposed. `funding[].funders` is included; see [the agent store](./project-form.md#the-agent-store-and-how-a-reference-is-picked) for how long it was not.
4. **Every live entity proposal must satisfy its own rules**, through `proposals::check_person` and `check_organization` — REQ-3.4's organisation rules, REQ-3.5's person rules, and the project-role guard on `jobTitles` that the PRD does not ask for but `dpe-server validate` enforces, so without it the editor could produce data that fails validation in a crate it never touches. It runs directly after the reference gate above, because both are about the entities a project points at and a depositor fixing one is usually fixing the other. The refusal names each proposal and each finding, with a link to the form that can fix it, the way a field outside the current section is already listed with a link to its section.
5. **No field may hold a placeholder sentinel a depositor typed**, through `form::submit::typed_sentinels`. Decided from the field's declared shape rather than from the value, because a stored sentinel is usually correct; see [where a field's shape and empty state are declared](#where-a-fields-shape-and-empty-state-are-declared) above.
6. **Every `temporalCoverage` entry must resolve**, through `editor_core::submission::unresolved_temporal_coverage` — the same decision `dpe-server validate` and `dpe-api-oai` apply, over the same two tables, which `AppState` reads once at startup from `EDITOR_DATA_DIR`. With no data directory the tables are empty, so every free-text period is unresolvable and the submission is refused: the fail-safe direction, since the alternative opens a pull request that fails CI in a crate the editor never touches. It re-runs on **every** submit, which is what makes a resubmission revalidated rather than trusted because it was reviewed once.
7. **The submission must change something.** The comparison is `editor_core::review::diff`, the one the review surface itself renders from, so "changes nothing" means the same thing in both places. Allowed through, an unchanged submission locks the depositor's own form on a queue entry a reviewer can only clear by rejecting it.

Two further checks run on **every** write rather than only on a submit, and therefore before these: the per-field cap, because an applier truncates silently and deferring it would let an over-cap save store the truncated value; and the concurrent-save baseline, because a save is exactly what would overwrite somebody else's work.

A refusal re-renders rather than redirecting, so nothing typed is lost, and per-field errors render beside the control they name. Submit validation is whole-project while the form is sectioned, so an error routinely names a field the depositor is not looking at — those are listed separately with a link to the section that holds them, otherwise the refusal says "the fields below say what needs changing" and nothing below says anything.

RDU submits through the same path: `User::may_reach` is already true for every project for an RDU account, so the direct-editing half was already there and this is the half that makes the result reviewable.

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

The editor vendors Datastar from `modules/editor/public/vendor/`, whose README is the version of record; do not restate the version here. DPE vendors its own copy and is bumped independently.

One thing to get right, and it fails quietly: **keyed plugin attributes use `:`, not `-`** — `data-on:click`, `data-attr:disabled`, `data-class:open`, and `data-init` rather than `data-on-load`. This has been true since RC.6, so it matches DPE's markup too. The hyphen form produces a console error and an inert control: the page renders fine and a snapshot test asserting the attribute is present still passes.

## Styling

`modules/editor/style/main.css` is the single Tailwind entry, built by `just css-editor` (dev) or `just css-editor-release` (content-hashed). It imports the design tokens and the `mosaic-tiles` component barrel.

`@import 'tailwindcss' source(none)` means classes are collected **only** from the explicit `@source` globs, which must cover every crate that emits Tailwind classes. A missing glob produces no build error — just markup whose classes resolve to nothing. After a change that adds classes in a new location, grep the built stylesheet for them.

New Mosaic tiles are added **demand-driven**: a screen that needs a missing primitive adds it to `mosaic-tiles` with a playground showcase and a unit test at that point, rather than an up-front form kit. Their CSS goes in `mosaic-tiles/src/components/components.css`, the barrel every consumer imports.

**Check a tile against the surface you are putting it on.** Tiles are styled for light backgrounds — `link` is `text-primary-600`, which measures 2.35:1 on the footer's `bg-slate-800` and fails WCAG 2.1 AA. That is why the footer uses plain anchors inheriting `text-gray-300` (9.93:1), as DPE's does. A dark-surface variant of a tile is a design-system change, so it belongs in `mosaic-tiles` with its own showcase rather than being worked around locally.
