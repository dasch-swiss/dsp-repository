---
title: "feat: move CPE's store, builder and page assembly, add its host crate, and mount it under /cpe"
type: feat
date: 2026-10-11
author: "Balduin Landolt"
status: reviewed(2)
linear: DEV-7465
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7465
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator/.claude/worktrees/DEV-7465
---

# feat: move CPE's store, builder and page assembly, add its host crate, and mount it under /cpe

## Overview

This is the last step of DEV-7400. `cpe-ports`, `cpe-core`, `cpe-components` and `cpe-image-viewer` are
in `areas/access/cpe/`. This plan moves what is left of the prototype's engine (`cpe-store`,
`cpe-store-build`, `cpe-web`), adds the host crate `cpe-server` from what `cpe/bin` holds, builds
Incunabula's store from `sync`'s port when the process starts, and ends with `access-server` serving CPE
under `/cpe` (DEV-7438, pulled into this plan by the owner on 2026-10-11). The incubator then builds
against the vendored crates and its engine is deleted.

The work is one stack of seven PRs in dsp-repository and one PR in dsp-incubator:

| PR | Phases | Repository | Branch | Base | Content |
|----|--------|------------|--------|------|---------|
| A | 1, 2 | dsp-repository | `worktree-DEV-7465-store` | #478's branch, or `main` once it has merged | this plan, `cpe-store` |
| B | 3, 4 | dsp-repository | `worktree-DEV-7465-store-build` | PR A | `cpe-store-build` (library), Incunabula's folder, the config-language specification |
| C | 5, 6 | dsp-repository | `worktree-DEV-7465-web` | PR B | `cpe-web`, CPE's shared static files |
| D | 7, 8, 9 | dsp-repository | `worktree-DEV-7465-server` | PR C | `cpe-server`, activation through the port |
| E | 10 | dsp-repository | `worktree-DEV-7465-handle` | PR D | the store owns its connection |
| F | 11 | dsp-repository | `worktree-DEV-7465-base` | PR E | CPE builds its URLs from a configured base (DEV-7438) |
| G | 12, 13 | dsp-repository | `worktree-DEV-7465` | PR F | `access-server` constructs and mounts CPE; image and data shipping |
| H | 14 | dsp-incubator | `worktree-DEV-7465` | `main` | re-vendor, thin host, engine deleted |

There is no PRD. The requirement is DEV-7465, DEV-7438, DEV-7400's done-criterion, and ADR-0003, ADR-0007,
ADR-0008 and ADR-0011.

**What the numbers refer to.** dsp-repository `bb040999` and dsp-incubator `origin/main`
`d68f214bf50c8999e1c7750160822ffa063a5c33` (2026-10-11, with DEV-7401's re-vendor, #508). Every source line number, edit table and
count in this plan was read at that incubator commit, without building or running anything. `SRC` is
taken again when Phase 1 starts; where it has moved, the lines are found again by content and the counts
in *Success Metrics* are retaken.

## Problem Statement / Motivation

Incunabula cannot be served from this repository until the store, the page assembly and a router are
here, and something builds the store from the port at run time. The ticket describes a move. Five things
it says, or leaves out, do not match what the two repositories hold:

- **Nothing builds the store from the port at run time.** `cpe-store-build` is a command-line tool. Its
  output, `projects/incunabula/data.sql`, is committed, embedded with `include_dir` and loaded by
  `Project::load`. `LiveArchiveProjection` has no caller but that tool. The ticket's "construct
  `LiveArchiveProjection` and inject it into CPE" needs an activation path that does not exist.
- **The builder is not named in the ticket.** The incubator's ADR-0016 says DEV-7465 "is where the
  builder and the mapping move to the monorepo" and that it "carries the amendment of the monorepo
  clause" on where a project's hook lives.
- **`cpe-web` holds no routes.** The router, the request preferences, the origin configuration and
  project discovery are in `cpe/bin` (about 9,300 lines in `router.rs`, two thirds of it tests).
- **The prototype has no base path.** Links are built as `/{project_id}…`, the stylesheets and logos are
  at `/_static/…`, project assets at `/_assets/{id}/…`, both cookies carry `Path=/`, and the Datastar
  URLs, both redirects and the share links are built from the request path. Under `nest` axum strips the
  prefix from that path, so a mount alone would serve pages whose links, redirects and cookies leave
  `/cpe`. That is DEV-7438's work, and the reason it is part of this plan.
- **Some tests lean on projects that do not move.** 38 of the router's 190 tests load Alice, and
  `bin/tests/smoke.rs` boots the real three-project bundle. ADR-0007 moves Incunabula first and alone,
  and Alice and HdM have no projection until DEV-7498. The 234 fixture folders of `cpe-web` each hold a
  `data.sql`.
- **Authors write root-absolute links.** Incunabula's narratives hold 42 Markdown links of the form
  `](/incunabula/books/…)`, and the renderer passes a link through unchanged.

Three items of the ticket are already done and need no work here: the `_settings` page was deleted on
2026-10-08, `CATALOGUE_OVERRIDES` is curation since DEV-7497, and Incunabula's `translate.py` was deleted
by DEV-7496.

## Proposed Solution

Each crate moves as the two earlier plans moved theirs: exported from the incubator at one commit `SRC`,
copied, formatted, then edited against the formatted baseline, with the proof that nothing else changed.
Behaviour changes come after the moves, each in its own PR: the store-owned connection (E), the base
path (F), the wiring and the mount (G).

### Decisions (owner, 2026-10-11)

1. **The builder moves as its own crate, as is.** `cpe-store-build` lands at
   `areas/access/cpe/store-build` as a library. Its hook stays in that crate behind the const `HOOKS`
   table, and ADR-0007 is amended to say so (*Prior decisions this plan changes*).
2. **The store keeps an entry that opens it from SQL text, outside the production path.** The moved
   tests run against their `data.sql` fixtures unchanged, and the incubator's host uses it for Alice and
   HdM until DEV-7498 removes it. Production activation goes through the port only.
3. **DEV-7438 is the last part of this plan.** The plan ends with Incunabula served at
   `/cpe/incunabula/…` from the area binary.
4. **One plan, one `gh stack`.**
5. **`sync-store` is a dev-dependency of `cpe-store-build` and `cpe-server`.** Tests that build
   Incunabula need the real 0803 snapshot, and only `sync-store` can read it. Chosen over keeping those
   tests in `access-server` (which `ARCH-MAP.md` holds to wiring only) and over a committed snapshot
   fixture (a second copy of the data). ADR-0008 is amended (*Prior decisions this plan changes*).
6. **Alice and HdM do not enter this repository, as data or as test fixtures.** Tests that read their
   bundles stay in the incubator and run there against the vendored router (PR H): the 38 Alice tests
   of `router.rs`, the one test of `bin/src/lib.rs` that loads the shipped projects, and all of
   `smoke.rs`. Tests that use a crate-local fixture or an inline project move. No test needs HdM or the
   real Incunabula bundle. Chosen over moving Alice as a fixture and over rewriting
   those tests onto Incunabula. The cost: a regression those tests would catch shows at the incubator's
   pin move, not in this repository's CI.

### Decisions (this plan; the owner may overrule at review)

7. **Activation renders SQL text and opens it.** `cpe-server` asks the port for the snapshot, calls
   `cpe_store_build::build`, renders with `sql::to_sql` and the builder's own `sql::header`, and opens the
   result with the store's SQL entry. The text is then byte for byte the `data.sql` the prototype embeds,
   because the incubator's freshness test holds that file to exactly that call. A sink that writes SQLite
   directly (incubator ADR-0015) is left for later; it would save a parse, and it would remove this
   argument.
8. **CPE never stops or delays the area binary.** `Cpe::new` does no I/O and cannot fail.
   `Cpe::activate` is blocking and runs in `spawn_blocking` after the listener is bound, as `Dpe::warm`
   does. A project that fails to activate is logged at error level with its id and stays inactive; the
   others and DPE keep serving. "Not ready" is not "not there": while `activate` runs, every path under
   the base is answered 503 with `Retry-After` and `Cache-Control: no-store`; afterwards the prefix of a
   project that failed is answered the same way, and 404 is kept for a path that names no project. A
   missing data directory makes each project fail (503 on its prefix). A missing projects directory
   leaves no project known, so every path is CPE's not-found. The 503 has a plain body in CPE's shell;
   the designed offline page and the strict `validate` verb stay DEV-7403
   (*Prior decisions this plan changes*).
9. **The seam exception names `cpe-server` only.** `ARCH-MAP.md` and ADR-0007's amendment of 2026-10-11
   expect `cpe-server` "and, for a `Live<Port>` adapter, `cpe-store`" in `check-cpe-seam.sh`'s
   `PORT_EXCEPTIONS`. The `Live<Port>` adapter is `sync-store`'s, so `access-server` never names
   `cpe-store`. PR G records the narrower list in the ADR, `ARCH-MAP.md` and the script's header.
10. **`cpe-server` is a facade in the shape of `dpe-server`.** No type of `cpe-core`, `cpe-store`,
    `cpe-store-build`, `cpe-web` or `cpe-components` appears in a `pub` signature, `CpeError` included;
    `cpe-ports` types may. `check-composition-root-deps.sh` already forbids `access-server` from depending
    on `cpe-core` and `cpe-web`. That also closes the doc-hidden `DataArks::from_stored` for the host,
    which the ticket asks for.
11. **Incunabula's folder is `areas/access/cpe/projects/incunabula/`.** The folder name is the project's
    id, the URL segment and the key `hooks::resolve` looks up, so it stays `incunabula`. It holds what the
    process reads (`config.kdl`, `narrative/`, `assets/`) and the documents that describe its data
    (`SHAPE.md`, `CURATION.md`, `PROVENANCE.md`, `DATA_ISSUES.md`). `data.sql` is not copied. The design
    documents and their images stay in the incubator. The folder arrives with the builder (PR B), whose
    integration tests include its `config.kdl` at compile time; the config-language specification
    arrives there too, because one of those tests includes it.
12. **CPE's shared static files live in `areas/access/cpe/public/` and `cpe-server` embeds them.** The
    two logos and `dasch-fingerprint.svg` go to `areas/access/cpe/public/`, `datastar.js` to
    `areas/access/cpe/public/vendor/` with a README row, where `verify-checksums.sh` finds it. They arrive
    with `cpe-web` (PR C), whose tests read `datastar.js`. `cpe-server` serves them and the seven
    stylesheets of `cpe-components`' `style/` from an `include_bytes!` table, as `cpe-image-viewer` serves
    its own, with a test that every file of the three directories has a row. `shell.css` refers to the
    fingerprint by a relative URL, so both are served from one directory. Only project folders and `sync`'s
    data are read from disk. No `include_dir`, and no project bundle is embedded (ADR-0007).
13. **The site index leaves the router.** `/cpe/` is CPE's not-found (ADR-0007, DEV-7405). `Cpe` exposes
    a list of its projects (id, title and the project's URL, a type of `cpe-server`), and the incubator's
    host renders its own index from it.
14. **Configuration.** `CpeConfig` is read with `figment` in `cpe-server`, as `DpeConfig` is, from
    `cpe.toml` and `CPE_*`: `CPE_PROJECTS_DIR` (default `areas/access/cpe/projects`),
    `CPE_PUBLIC_ORIGIN` and, from PR F, `CPE_BASE`. A public origin with a path, and a base that does not
    start with `/` or ends with one, are refused at load with an error that names the variable. The
    directory of `sync`'s data is `access-server`'s, read from `SYNC_DATA_DIR` (default
    `areas/access/sync/data`, for a run from the checkout) beside `DPE_SITE_ADDR`. That name and the
    image path `/app/sync/data` are fixed by DEV-7407 (#478 and ops-deploy#1460) and are not changed
    here: ops-deploy sets the same value, and its environment overrides a Dockerfile `ENV`. `clap` and `figment` stay inline dependencies, as in
    `dpe-server`. `pico-args` and `anyhow` do not enter the workspace.
15. **The base has one source.** `access-server` mounts with `nest(cpe.base(), cpe.router())` and writes
    no `"/cpe"` of its own. `CPE_BASE` defaults to `/cpe` in the image and to empty at a root mount.
16. **The connection handle is a `Mutex` inside `cpe-store`, not a pool.** PR E moves the open, the facts,
    the search indexes, the ARKs and the reads behind one owning type. `cpe-web` loses `rusqlite`, and the
    lock rule becomes a property of the type's privacy, so it needs no gate. The editor rejects a
    `std::sync::Mutex` around a connection (`areas/deposit/editor/server/src/db/mod.rs`) because a guard
    can be held across an `.await`; here no guard leaves the store crate. The prototype runs its reads on
    the async worker thread, and this plan does not change that: moving them to `spawn_blocking`, or to
    `deadpool-sqlite`, changes concurrency inside a move and is recorded as debt in `ARCH-MAP.md`.
17. **`rusqlite` errors leave the public error types in PR E** where a caller outside `cpe-store` can
    reach them (`DbError`, `QueryError`): the variant keeps a `#[source]` of a crate-private wrapper type.
18. **Cookies are scoped to the base.** `lang` and `appearance` are set with `Path=<base>` (`/` at a root
    mount), so they are shared by CPE's projects, as today, and are not sent to DPE.
19. **`cpe-server` installs no request span.** `access-server`'s OTel layers trace CPE's routes; the
    incubator's host keeps the prototype's `request_span` for itself.
20. **Pages without a project load the shared static files from `<base>/_static/…`.** CPE's own
    not-found page and the 503 have no project, so ADR-0007's `<base>/<shortname>/_static/…` cannot be
    built for them. The same table is served at both places; a project's pages use only their own. The
    ADR's reason for the per-project path (a project on its own hostname is self-contained) is untouched,
    since those two pages do not exist on a project's hostname. PR F amends the ADR.
21. **A narrative link that starts with one `/` gets the base in front** (owner, 2026-10-11). The
    Markdown renderer and the invite lists send such a link through the URL builder; content is not
    changed. Chosen over project-relative links (a change to the config language and to all three
    projects' content) and over rewriting Incunabula's links to `/cpe/…` (which breaks at any other
    base). The content keeps naming the project segment, against ADR-0007's "resolved in one place";
    H8 records that as follow-up.

### Prior decisions this plan changes

**ADR-0007, "A project is a folder"** (accepted): "its KDL, its narrative, its assets, and its hook if it
has one. Adding a project adds a folder and touches no shared registry", and a hook is "one Rust function
per project, over the port's types, in the project's own folder". The ADR gives no separate reason; both
follow from the heading. The incubator's ADR-0016 moved the hook into the builder crate because project
folders there are embedded whole with `include_dir`, and says that reason does not hold here. The reasons
that do hold here: a function the builder calls must be linked into the builder, which needs one place
that names it; Rust under `projects/` would need its own rules for the workspace, formatting, linting and
`cargo machete`; and the builder moves as is (decision 1). The cost is the one ADR-0016 accepted: a
project with a hook adds a module and one table row to `cpe-store-build`. A hook gets the mapped draft
and never the snapshot, so "over the port's types" is reworded. Phase 8 amends the ADR.

**ADR-0007, "A project fails alone"** (accepted): "a project that does not validate is offline on its own
prefix while DPE and every other project keep serving", with a strict mode that fails the start. Decision
8 keeps the first half in its plainest form (the project is inactive and its prefix is answered 503)
and does not build the strict mode or the designed page. Nothing is reversed; the amendment in Phase 8
says what is built and that DEV-7403 owns the rest.

**ADR-0008, "Enforced by"** (accepted): "a reading capability's crates see its own `ports` crate and never
`sync`'s store crates or Chischtli". Its reason is the decision above it: "The only feed is the
capability's own port against `sync`". Decision 5 lets two CPE crates name `sync-store` under
`[dev-dependencies]`. The reason still holds: the tests build a `LiveArchiveProjection` and feed the
builder through the port, and no CPE library code can name the adapter. What changed is that the read
model's tests need the committed snapshot, which only the adapter reads. Phase 3 amends ADR-0008 and adds
the rule to `check-cpe-seam.sh`, so the carve-out is checked and not left to review.

**ADR-0003's crate anatomy** names a domain crate, a store, a web crate and `ports`. `cpe-store-build` is
a role it does not list, as `cpe-components` and `cpe-image-viewer` were. Phase 3 records it in the
ADR-0007 amendment.

### The crates

| Crate | Folder | Depends on (CPE) | From |
|-------|--------|------------------|------|
| `cpe-store` | `areas/access/cpe/store` | `cpe-core` | `cpe/store`, with `schema.sql` |
| `cpe-store-build` | `areas/access/cpe/store-build` | `cpe-core`, `cpe-ports` | `cpe/store-build`, without `main.rs` |
| `cpe-web` | `areas/access/cpe/web` | `cpe-core`, `cpe-store`, `cpe-components`, `cpe-image-viewer` | `cpe/web` |
| `cpe-server` | `areas/access/cpe/server` | all of the above | `cpe/bin` (`router.rs`, parts of `lib.rs`, `discovery.rs`) |

`cpe-store-build` reads `../../store/schema.sql` with `include_str!`; the relative path is the same here.
It does not depend on `cpe-store`. None of the four is a leaf in `check-cpe-seam.sh`'s sense.

Not moved: `cpe-engine` (a facade for `cpe/bin`, deleted in PR H), `cpe/bin`'s `main.rs` and `build.rs`,
the site index, the builder's command line and its `cli.rs`, `fresh.rs` and `dockerfile.rs` tests, Alice
and HdM, `smoke.rs`.

Every manifest follows `areas/access/cpe/core/Cargo.toml`: `version.workspace = true`,
`edition = "2021"`, `publish = false` (this workspace's `[workspace.package]` has no `publish`), sibling
CPE crates by `path = "../<dir>"`, third-party crates by `{ workspace = true }`.

### The source at `d68f214b`

Line numbers are the incubator's, before formatting. "Test module" is the line of the file's first
`#[cfg(test)]`.

**`cpe-store`** (9,392 lines in five files and `schema.sql`; 185 tests, all inline, none reads a file
outside the crate; nothing is left behind).

| Where | Now | Becomes |
|-------|-----|---------|
| `src/db.rs:20` | `#[error("failed to apply engine schema")]` | `"failed to apply the store's schema"` |
| `src/query.rs:1778`, `:6820`; `src/query/reads.rs:401`; `schema.sql:103` | bare `ADR-0014` | `dsp-incubator cpe/docs/adr/0014-annotations-are-a-first-class-engine-store.md` |
| `src/query/reads.rs:252` | bare `ADR-0008` (it would resolve to this repository's ADR-0008, a different record) | `dsp-incubator cpe/docs/adr/0008-images-are-embedded-only-resources.md` |

The ADR rows are in the move commit, not the comments commit: `check-adr-refs` runs in `just check` and
reads every tracked file. One `#[allow(clippy::too_many_arguments)]` (`query.rs:2455`). Left for Phase 2:
`reads.rs:12-14` (the sentence about DEV-7402), `lib.rs:12-16` (`cpe-engine`), the `docs/SCHEMA.md`
citations (`db.rs:32`, `schema.sql:2`, `:3`, `:14`, `:57`, `:105`), 13 plan-step and 16 "Slice N"
references. "Query plan" and Rust slices stay.

The public surface PR E closes (`C` names `rusqlite::Connection`, `E` exposes `rusqlite::Error`):
`DbError` (`db.rs:15`, E), `open_project_db` (`db.rs:36`, C, E), `QueryError` (`query.rs:49`, E),
`LiveProjectFacts::new` (`query.rs:89`, C), `build_search_indexes` (`query.rs:769`, C), `data_arks`
(`query.rs:1328`, C), `ProjectReads::new` (`reads.rs:90`, C). Their only callers outside the crate are in
`web/src/project.rs` (`:105`, `:106`, `:129`, `:132`, `:148`).

**`cpe-store-build`** (62 unit tests and 183 integration tests move; 26 tests stay behind with
`cli.rs` 21, `fresh.rs` 4, `dockerfile.rs` 1).

| Where | Now | Becomes |
|-------|-----|---------|
| `Cargo.toml` | `pico-args`, `sync-store` under `[dependencies]` | no `pico-args`; `sync-store = { path = "../../sync/store" }` under `[dev-dependencies]` |
| `src/generic/geometry.rs:267`, `:272`, `:11` | `../../../vendor/dsp-repository/areas/access/cpe/core/src/validate.rs` | `../../../core/src/validate.rs`; the message and the comment follow |
| `tests/it/main.rs:21`, `:23`, `:24` | `mod cli;`, `mod dockerfile;`, `mod fresh;` | deleted |
| `tests/it/main.rs:61-64` (`DATA_DIR`) | `/../vendor/dsp-repository/areas/access/sync/data` | `/../../sync/data` |
| `tests/it/main.rs:66-67` (`PROJECTS_DIR`) | used by `fresh.rs` only | deleted |
| `tests/it/hygiene.rs:13`, `:164`, `:176-179`, `:357` | the `BINARY` constant, the partition on it, the check that `main.rs` names `sync_store`, `BINARY` in the searched list | removed; the test and its other assertions stay |
| `tests/it/worked_example.rs:15-18` | `/../docs/CONFIG_LANGUAGE.md` | `/../../../../docs/src/cpe/config-language.md` |
| `README.md:64`, `:137`, `:188` | bare `ADR-0015`, `ADR-0016`; the command-line sections | rewritten for the library, incubator ADRs cited by file name |

Unchanged, and the reason for the phase order: `tests/it/main.rs:41-44` and `worked_example.rs:7-10`
include `/../projects/incunabula/config.kdl` at compile time, and `hooks.rs:1168`, `:1227` read
`/../projects`. The paths are right here; the files have to exist when the builder lands.
`src/lib.rs:35` (`../../store/schema.sql`) is right as it is. 28 tests run on the real 0803 data through
`sync_store`. **Not to be edited:** the header strings in `sql.rs:94`, `:100`, `:583`, `:599`, `:600`,
`:627`, `:629` ("DO NOT hand-edit; regenerate with: just cpe store-build …"). They are output, and a
change makes the rendered SQL differ from the prototype's `data.sql`. The moved sources hold no plan or
ticket reference; Phase 4 rewords "the binary" and "vendored".

**`cpe-web`** (`page.rs` 10,695 lines, test module at 3049; `project.rs` 2,226, at 509; `share_link.rs`
776, at 304; 497 tests; 30 snapshots; 234 fixture folders with 577 files, 2.3 MB, none special to
`.gitattributes` or `.gitignore`).

| Where | Now | Becomes |
|-------|-----|---------|
| `src/page.rs:5688`, `:6284`, `:6287` | `../../vendor/dsp-repository/areas/access/cpe/components/style/<file>.css` | `../../components/style/<file>.css` |
| `src/page.rs:6338` | `../../static/datastar.js` | `../../public/vendor/datastar.js` |
| `tests/header_budget.rs:13` | `../../vendor/dsp-repository/…/style/tokens.css` | `../../components/style/tokens.css` |
| `src/page.rs:2422`, `:2443` | `blocks.push(html! { … })` over several lines | a `let` binding, then `blocks.push` (`CONVENTIONS.md`) |
| `src/page.rs:78`, `:1521`, `:2497`, `:2842`; `src/project.rs:217`; `tests/validation.rs:792`, `:2424` | bare incubator `ADR-0005`, `0007`, `0012`, `0011`, `0014`, `0014`, `0012` | qualified by file name, in the move commit |

The crate already names `cpe_image_viewer`. It has no `@match`, one `@if let` over a call (`:1750`) and
one `#[allow(clippy::too_many_arguments)]` (`:1859`). `rusqlite` is named at `project.rs:10` and `:40`
only. Left for Phase 6: `lib.rs:9-13` and `page.rs:1812` (`cpe-engine`, "the `cpe` binary"), the comments
that say "the bin" (`share_link.rs:11`, `:220`, `:289`; `project.rs:44`, `:91`, `:157`, `:308`, `:420`),
75 "plan", 51 "slice" and 102 "step N" lines.

**`cpe-server`, from `cpe/bin`** (`router.rs` 9,395 lines, test module at 3240; `lib.rs` 1,210, at 563;
`discovery.rs` 381, at 208).

| From | Moves | Stays in the incubator's host |
|------|-------|-------------------------------|
| `lib.rs` | `RequestedLang` (20), `RequestedAppearance` (40), `normalize_lang` (54), `resolve_request_prefs` (80), `extract_lang_prefs` (100), `extract_appearance` (151), `query_pairs` (189), the query and cookie readers (201-264), `OriginConfig` (312), `derive_request_origin` (377) | `PROJECTS` and `STATIC_ASSETS` (276, 277), `BuildError` (283), `build_app` (406), `check` (433), `check_in` (439), `load_projects` (479), `request_span` (544) |
| `router.rs` | `build` (208) without its two `Dir` parameters, the page and fragment handlers, `fallback_handler` (656), the query readers, the cookie and redirect helpers (1045-1132), `serve_viewer_asset` (3169), `content_type` (3183) | the `/` route (286-329), `site_index` (2893), `ProjectSummary` (106), `serve_static`, `serve_static_or_stylesheet`, `serve_project_asset` and `COMPONENT_STYLES` (written anew over the table and the inventory) |
| `discovery.rs` | its rules: a subfolder with a `config.kdl` is a project, `data.sql` is optional, `narrative/` is flat, `assets/` is recursive | the `include_dir` implementation |

`load_projects`' steps (sources, `ValidationContext` with the reserved ids, `Project::load`, warnings) are
what `Cpe::activate` does over a directory and the port. Handlers capture their `Arc<Project>` when the
route is registered, so the router is built once, after every project has loaded. The moved code holds
no `html!` once the site index is gone, and no `#[allow]` or `#[expect]`.

The tests, 249 in the three files:

| Rule | Count | Goes |
|------|-------|------|
| `router.rs`: named `site_index_*` (11 need no project, 10 use fixtures, 2 use Alice) | 23 | stays, with the site index |
| `router.rs`: calls `load_alice_project` or `load_alice_with_order_options` (the 2 above not counted again) | 36 | stays |
| `router.rs`: `static_route_serves_vendored_stylesheets_beside_the_static_bundle` (5152) and `static_route_returns_404_for_empty_production_bundle` (5127) | 2 | rewritten here over the table |
| `router.rs`: the 4 `serve_static_*` and 7 `serve_project_asset_*` / `assets_route_*` tests (3351-3474) | 11 | rewritten here over the table and the inventory |
| `router.rs`: every other test (fixtures `delta` to `theta`, inline projects, pure functions) | 118 | moves |
| `lib.rs`: the 9 tests of `load_projects` and `check` over `tests/fixtures/validation`, and `shipped_projects_fit_the_desk_header_row` (754) | 10 | stays |
| `lib.rs`: every other test | 38 | moves |
| `discovery.rs`: all tests | 11 | rewritten here over a directory |

`tests/fixtures/discovery` and `tests/fixtures/static` move; `tests/fixtures/validation` stays. Five of
the Alice tests (`fallback_unknown_top_level_path_uses_site_chrome` 4884,
`fallback_project_id_match_does_not_fold_case` 4990, `…does_not_percent_decode` 5025,
`matched_route_keeps_method_behaviour_on_post` 5061, `fallback_never_reflects_the_failed_path` 5089) need
only a project with that id. Phase 7 writes their assertions again over a fixture project, because they
pin the fallback's safety rules and should fail in this repository.

Comments Phase 7 corrects as it moves the code: `router.rs:1860`, `:9294` (`cpe_engine::component::…`),
`:8548` (`engine/tests/projects/…`), `:1612`, and about 26 doc lines that name `cpe_engine`.

### The host crate and activation (PR D)

```rust
// cpe-server
impl Cpe {
    pub fn new(config: &CpeConfig) -> Self;                       // no I/O, cannot fail
    pub fn activate(&self, projection: &dyn ArchiveProjection);   // blocking; logs, never panics
    pub fn router(&self) -> axum::Router;                         // ends in CPE's own fallback
    pub fn base(&self) -> &str;                                   // from PR F
    pub fn projects(&self) -> Vec<ProjectSummary>;
}
```

`router()` returns a router whose one service forwards each request to the activated router once
`activate` has stored it, and answers 503 until then (decision 8). It always carries an explicit fallback:
a nested router without one inherits the outer router's, which is DPE's `ServeDir`.

`activate`, for each subfolder of the projects directory that holds a `config.kdl`: read the KDL, the
narrative and the asset files from disk; lower the `remodel {}` block, which declares the shortcode; ask
the port once for that shortcode's snapshot; resolve the hook by folder name; build, render and open the
store (decision 7); validate and load the project as `Project::load` does today. The port is synchronous
and `LiveArchiveProjection` reads 20 MB from disk on each call, so it is called once per project, never
per request. Each project logs its id, the snapshot's size and the time taken.

**Files from disk.** Discovery does not follow symbolic links and takes a folder as a project only when
its name passes the id rule `cpe-core` already applies. Project assets are read into an inventory at
activation, and a request is answered by looking its path up in that inventory. No request path is ever
joined onto a directory. The project segment of a URL is looked up among the active projects and is never
a path component. The content type comes from the file extension.

### The base path and the mount (PRs F and G)

ADR-0007's amendment for DEV-7405 fixes the layout: the router does not know its prefix, CPE builds every
URL from a configured base, a project lives at `<base>/<shortname>/…`, shared static files at
`<base>/<shortname>/_static/…`, project assets at `<base>/<shortname>/_assets/…`, and the root routes
`/cpe/` to CPE's not-found explicitly, because `nest` does not match it.

- One URL builder, in `cpe-core`, takes the base and the project segment. `RenderContext` carries the
  base. `LinkResolver`, the breadcrumbs, the record links, the stylesheet and logo URLs, the page head,
  and `cpe-server`'s asset URLs go through it. The project segment is resolved in one place, so it can
  become the shortcode later.
- **The request path.** Handlers under `nest` see the path with the prefix removed. `current_path` (from
  which the Datastar `@get(…)` URLs, the language and appearance links and the share links are built) and
  the target of both redirects (`canonical_page_redirect`, `redirect_without_keys`) are the base plus that
  path, composed by the builder. No handler writes a `Location` any other way.
- **The asset layout changes, not only the prefix:** `/_assets/{id}/…` becomes `<base>/{id}/_assets/…`,
  and `/_static/…` becomes `<base>/{id}/_static/…`. `cpe-core`'s rule that no segment of a project's
  route starts with `_` keeps its meaning. The viewer's `head_tag` is given
  `<base>/{id}/_static/image-viewer`, the directory its module and OpenSeadragon share.
- **Share links** are the public origin, the base and the path.
- **Cookies** follow decision 18, **pages without a project** decision 20, **narrative links**
  decision 21.

The sites, as counted at `d68f214b` in production code:

| What | Where | How it gets the base |
|------|-------|----------------------|
| Links to a resource, a member, a partner | `LinkResolver::href` (`components/src/component/links.rs:66`), called from 10 sites | the builder, inside the resolver |
| Project home and browse links | `breadcrumb.rs:103`, `:125`, `:169`, `:217`; `resource_detail.rs:1085`, `:1089`, `:1149`, `:1174`; `web/src/page.rs:1085`, `:1123`, `:1302`, `:1586`, `:1600`, `:2262` | the builder; the breadcrumb and not-found functions take `project_id` today and gain the base |
| The "CPE" crumb and brand links to `/` | `breadcrumb.rs:222`, `:275`; `page.rs:657`, `:662`, `:1583` | the base itself |
| Stylesheets, `datastar.js`, logos | `page.rs:72-84`, `:688`, `:691`, `:1730`, `:1732`; `data_links.rs:156`, `:162` | the builder's static URL; `document_head` gains the static prefix as a parameter |
| The viewer's script | `page.rs:91` | the per-project `_static/image-viewer` |
| `current_path` and what is built from it | set at `bin/src/router.rs:707`, `:1218`, `:1340`, `:1433`, `:1700` from `uri().path()`; read by the language and appearance links, the forms' `action`, the pager, facet, sort, search and compound hrefs, and 10 `@get(…)` expressions | set once per request as base plus path; the components need no change |
| Redirects | `router.rs:866`, `:1118` | built from the same base plus path |
| Share URLs and the reader's history URLs | `compose_absolute_url` at `page.rs:873`, `:965`, `:1029`, `:1178`, `:1246`, `:1430`; `router.rs:1848-1866`; `compound_viewer.rs:1180` | through `current_path`; the reader compares it with `location.pathname`, so a path without the base reloads the page on Back and Forward |
| Cookies | `router.rs:1047`, `:1066`, `:1068` | `Path=<base>` |
| Routes | `router.rs:331`, `:342`, `:354`, the per-page registrations | base-free under `nest`; the static and asset routes are registered per project |
| Cover image URL | `router.rs:271` | the builder's asset URL |
| Narrative links | `markdown.rs`, `narrative_outline.rs:101` | decision 21 |
| `url("dasch-fingerprint.svg")` | `components/style/shell.css:833`, `:834` | nothing: it is relative, and the table serves the stylesheet and the SVG from one directory |

Nothing in `datastar.js` or the viewer's JavaScript holds an absolute path. 91 of the 147 snapshots of
`cpe-components` and 29 of the 30 of `cpe-web` hold a URL that changes.

What each request gets after Phase 12, each with a test:

| Request | Answer |
|---------|--------|
| `/cpe/incunabula/` | Incunabula's start page |
| `/cpe/incunabula` | the same page, status 200, no redirect (as the prototype answers `/incunabula`) |
| `/cpe/_static/tokens.css`, `/cpe/incunabula/_static/tokens.css` | the stylesheet |
| `/cpe` and `/cpe/` | CPE's not-found, status 404 |
| `/cpe/<unknown>/…` | CPE's not-found |
| any path under `/cpe` while CPE activates | 503, `Retry-After`, `Cache-Control: no-store` |
| `/cpe/<project that failed to activate>/…` | the same 503 |
| `/cpe/incunabula/<unknown>` | the project's not-found |
| `/cpe/incunabula/_static/<unknown>` and `/_assets/<unknown>` | 404 |
| `/cpe/%2e%2e/x`, `/cpe/incunabula/_assets/..%2fconfig.kdl` | 404 |
| `/cpe//evil.example/x` | CPE's not-found, no redirect, the path not echoed |
| an unknown path outside `/cpe` | DPE's answer, unchanged |

CPE's routes sit inside the OTel layers; `/healthz` and `/telemetry/collect` stay outside. CPE sends no
browser beacons, and the telemetry identity constant stays DPE's (`areas/access/server/CLAUDE.md`).

### Verification design

- **Each move** is proved as in the earlier plans: `diff -r` of the formatted baseline against the
  result shows only the listed edits; the test list (`cargo test -p <crate> -- --list --format terse`,
  sorted) equals the one at `SRC` apart from the listed tests; `diff -r` of the snapshot folders is empty;
  no `.snap.new` exists; `cargo clippy -p <crate> --all-targets -- -D warnings` (which `just check` does
  not run with that flag); `cargo doc -p <crate> --no-deps`.
- **A moved test must not pass by skipping.** No moved test is marked `#[ignore]`. Every moved test that
  reads `schema.sql` or a fixture path is checked to fail when that path is missing
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`).
- **ADR-0008's test.** One test in `cpe-server` activates a project from empty against a
  `FakeArchiveProjection` twice and asserts the pages served are the same, and once more against a fake
  with one changed value and asserts the page shows it.
- **No URL leaves the base.** One test in `cpe-server` renders every page and fragment of a fixture
  project with the base `/cpe`, CPE's not-found and the 503, and fails on any `href`, `src`, `action`,
  `@get(…)`, `url(…)` or `Location` that starts with `/` and not with the base, and on any URL that
  starts with the public origin and does not continue with the base (the share links and the reader's
  history URLs). The fixture project has a narrative with a root-absolute link, a cover image and a
  compound viewer.
- **Parity at the root (Phase 8).** A scratch binary outside the repository serves `cpe-server` at `/`
  over the real projection. The prototype is built from `git archive SRC cpe shared`. Both run with the
  same public origin and absolute data paths. A crawl from Incunabula's start page follows `href`s and
  `@get(…)` URLs (with `Datastar-Request: true`) and compares status, body bytes and `Location` without
  following redirects, as the incubator's learning
  `proving-a-move-only-refactor-byte-identical-takes-more-than-snapshots.md` describes. Expected
  differences, listed before the run: none in Incunabula's pages; the site index and the other two
  projects are absent.
- **Parity under `/cpe` (Phase 13).** The same crawl against `access-server`, with one normaliser that
  removes the base, maps `<base>/incunabula/_static/…` and `_assets/…` back to the prototype's paths and
  maps the cookies' `Path`. Any other difference is unexpected. The scratch binary and the crawl script
  are not committed.
- **Results that go into a PR description** are written to `.claude/tmp/dev-7465/pr-<letter>.md` when
  they are produced; Phase 13 builds each PR's body from that file.
- **Reviewers for the phase reviews:** `eng:review:rust-reviewer`, `eng:review:maud-datastar-reviewer`,
  `eng:review:ivan-reviewer`, `eng:review:dune-reviewer`, `eng:review:consistency-reviewer`,
  `eng:review:code-simplicity-reviewer`; `eng:review:security-reviewer` for PRs D, F and G, with
  *Files from disk*, the redirects and the cookies as its checklist; `eng:review:accessibility-reviewer`
  for PRs C and F; `eng:review:devops-reviewer` and `eng:review:observability-reviewer` for PR G.

## Alternative Approaches Considered

- **Fold the builder into `cpe-store`.** It fits ADR-0003's role list, but it merges two crates during a
  move, so the move can no longer be proved verbatim. Rejected by the owner (decision 1).
- **Port Alice and HdM first (DEV-7498).** Nothing would need `data.sql`. DEV-7498 presupposes the
  vendored monorepo CPE and has an open design question. Rejected by the owner (decision 2).
- **Construct CPE in `access-server` and leave the mount to a later session.** `main` would carry an
  unserved capability. Rejected by the owner (decision 3).
- **Fail the start when a project does not activate,** as the prototype does. Production DPE would then
  depend on Incunabula's data, and ADR-0007 says the opposite. Rejected (decision 8).
- **Activate before the listener binds.** Simpler, with no forwarding service. `/healthz` would not answer
  while 20 MB are parsed, against a health check with a five-second start period. Rejected.
- **Shared static files once at `<base>/_static/`.** One URL per file and better caching. ADR-0007 puts
  them under each project so that a project on its own hostname is self-contained. Not reopened.
- **`deadpool-sqlite` for the store.** See decision 16.

## Technical Considerations

**The source commit.** `SRC` is the incubator's `origin/main` when Phase 1 starts. All four
crates are exported from that one commit, and each code commit carries it as a full sha in its body.

**Branches.** The executor works in this worktree on `worktree-DEV-7465` only and checks out no other
branch. When the last phase of a PR is committed, it points that PR's branch at `HEAD`
(`git branch -f <branch> HEAD`). Phase 13 adopts the seven branches with `gh stack init`, bottom to top,
and submits.

**The stack sits on #478 while it is open.** DEV-7407's #478 changes the Dockerfile, the `build-dpe`
action and `docs/src/dpe/operations.md`, which only Phase 13 touches. So that a hands-off run does not
stop there, Phase 1 rebases this branch onto `origin/worktree-DEV-7407` when #478 has not merged, and
Phase 13 runs `gh stack init --base worktree-DEV-7407 …`: that branch is the stack's trunk and is not one
of its layers, so nothing here writes to it. It is checked out in DEV-7407's worktree; if `gh stack`
cannot fast-forward the local copy for that reason, the stack is built with `gh stack link` over the
seven branch names. When #478 merges, GitHub retargets PR A to `main`, and the stack is re-rooted with
`gh stack unstack --local`, `gh stack init --base main …` and a rebase. A force-push to #478 replays all
seven branches. A review fix low in the stack is a rebase of everything above it; the proofs of
*Verification design* are run again after a rebase that touches a commit of this plan.

**Drift.** Nothing freezes `cpe/store`, `cpe/store-build`, `cpe/web`, `cpe/bin`, `cpe/static` or
`cpe/projects/incunabula` between `SRC` and Phase 14. H2 keeps the window empty, and Phase 14 checks it.

**Formatting.** The incubator formats with stable rustfmt. This repository runs `maudfmt`, then nightly
rustfmt. `cpe-web` and `cpe-server` are full of `html!`. `just fmt` is run until a run changes nothing.
`maudfmt` rewrites `@match f(…)` into a form that fails `clippy::blocks_in_conditions`, and can mangle an
`html!` passed as an argument; the fix is a `@let` or `let` binding, named in the commit body. An
`#[expect(…)]` that no longer fires is removed. Where CI's format check disagrees with the local one,
compare `cargo +nightly fmt --version` on both sides before touching a comment by hand.

**Snapshots.** The 30 snapshots of `cpe-web` keep their file names and the incubator's `source:` lines.
They are never accepted in bulk; a changed snapshot is accepted with `cargo insta review -p <crate>` only.

**Gates that read the index.** `check-cpe-seam.sh`, `check-adr-refs` and the datastar delimiter check
read tracked files; new files are staged before `just check`.

**Incubator ADR numbers fail `check-adr-refs`.** The script resolves every bare `ADR-NNNN` in a tracked
file outside `docs/specs` against this repository's `docs/adr/`, which ends at 0011. The moved sources
cite the incubator's series by bare number. Each move commit therefore qualifies them
(`dsp-incubator cpe/docs/adr/<file>`), as *The source at `d68f214b`* lists; the comments commits handle
everything else.

**Dependencies.** `rusqlite` here is 0.38 with `bundled` and `chrono`; the incubator's has `bundled`
only, and nothing in the store uses a chrono type. `unicode-normalization` (`0.1.25`) and `futures-util`
are new workspace entries. The builder parses geometry with `serde_json`'s `float_roundtrip`, which this
workspace's entry lacks. Features are unified across a workspace, so the feature goes on the workspace
entry beside `preserve_order` and reaches the editor's crates too; `just test` covers them, and Phase 3
compares the rendered SQL with the committed file. `preserve_order` cannot change the builder's output:
it parses and looks up, and serialises no map. `tempfile` stays an inline dev-dependency, as in
`sync-store`, and `tower` gets `features = ["util"]` as `cpe-server`'s dev-dependency, as in
`dpe-server`. `cargo machete` runs in `just check`, so a dependency is added in the phase that first uses
it; `cpe-server` lists `maud` only if its code still names it once the site index is gone.

**Start-up.** DPE's defaults are relative to the working directory, and Playwright and `just dev` start
the binary at the repository root. CPE's defaults follow the same rule, and decision 8 makes a missing
directory harmless. The image sets absolute paths. The PR preview runs with 512 MiB; Phase 13 measures
activation's time and peak memory.

**What DEV-7407 already ships.** #478 stages `areas/access/sync/data` into the image at `/app/sync/data`
and sets no `ENV` for it; ops-deploy#1460 sets `SYNC_DATA_DIR=/app/sync/data` for the deployed service.
Two runs do not go through ops-deploy: local development, which the checkout default covers, and the
Cloud Run PR previews, which set only DPE's three base URLs. Phase 13 therefore sets
`ENV SYNC_DATA_DIR="/app/sync/data"` in the Dockerfile, beside `DPE_DATA_DIR`. STAGE and PROD run images
older than #478 until they are upgraded, with the variable set and no directory behind it; decision 8
makes that a logged error and a project answered 503, and a test pins it.

**CI that does not see CPE.** `a11y-dpe.yml` starts the binary but is filtered to DPE's, the server's and
`shared/` paths. Phase 13 adds `areas/access/cpe/**` and `areas/access/sync/**`.

**Rollback.** Reverting PR G unmounts CPE. The store is rebuilt from the port at every start (ADR-0008),
so a rollback leaves nothing behind.

**Commits.** `type(scope): subject`. New scopes `cpe-store`, `cpe-store-build`, `cpe-web`, `cpe-server`
are added to both scope tables in the commit that adds each crate. Every PR with more than one commit
ticks `allow-many-commits` and carries a "Review Notes" section. A fix to a commit of this plan is
amended into it.

**`ARCH-MAP.md`** is changed with `dune:dune-map`, and the Fingerprints of `areas/access/cpe`,
`areas/access/server` and `areas/access/sync` are recomputed in each PR that changes one.

## Implementation Phases

#### Phase 1: `cpe-store` enters `areas/access/cpe/store`

### dsp-incubator

- [ ] Create the worktree at `origin/main`; record its sha as `SRC`
- [ ] Export `cpe/store`, `cpe/store-build`, `cpe/web`, `cpe/bin`, `cpe/static`, `cpe/projects/incunabula`
      and `cpe/docs/CONFIG_LANGUAGE.md` at `SRC` with `git archive` into `.claude/tmp/`
- [ ] Save the sorted test list of every crate of the `cpe` workspace under an absolute scratch path

### dsp-repository

- [ ] `git fetch`; where #478 has merged, rebase this branch onto `origin/main`; where it has not, onto
      `origin/worktree-DEV-7407`
- [ ] Commit this plan as `docs(docs): add the plan for moving CPE's store, page assembly and host (DEV-7465)`
- [ ] Copy `cpe/store` (sources and `schema.sql`) to `areas/access/cpe/store`
- [ ] Write its manifest in the shape of `cpe/core/Cargo.toml`: `cpe-core` by path, `rusqlite`,
      `thiserror` and `tracing` from the workspace
- [ ] Add the member to the root `Cargo.toml`
- [ ] Stage the new files and run `just fmt` until a run changes nothing; save the formatted baseline
- [ ] Change `DbError::Schema`'s message to "failed to apply the store's schema"
- [ ] Qualify the five bare incubator ADR references of *The source at `d68f214b`*, `schema.sql`'s
      included; `just check-adr-refs` passes
- [ ] Run the move's proofs of *Verification design* for `cpe-store`
- [ ] Add `cpe-store` to the scope tables in `CONVENTIONS.md` and `docs/src/git-conventions.md`
- [ ] Add the crate to `docs/src/repo_structure.md`
- [ ] Update `areas/access/cpe/CONTEXT.md` and `ARCH-MAP.md` (with `dune:dune-map`) for the store
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-store): move CPE's read-side store in from dsp-incubator (DEV-7465)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: the store's comments follow `CONVENTIONS.md`

### dsp-repository

- [ ] Remove plan, ticket and history references from `cpe-store`'s comments, the stale sentence in
      `query/reads.rs` about DEV-7402 included; rewrite incubator ADR numbers as
      `dsp-incubator cpe/docs/adr/<file>`
- [ ] Prove the commit changes comments only, with the comment-stripping comparison of the earlier plans
- [ ] Run `just check`; it passes
- [ ] Commit as `docs(cpe-store): remove history and plan references from the comments (DEV-7465)`
- [ ] Point `worktree-DEV-7465-store` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: `cpe-store-build` enters `areas/access/cpe/store-build`

### dsp-repository

- [ ] Copy `config.kdl`, `narrative/`, `assets/`, `SHAPE.md`, `CURATION.md`, `PROVENANCE.md` and
      `DATA_ISSUES.md` to `areas/access/cpe/projects/incunabula/` (decision 11); qualify any bare
      incubator ADR reference in them
- [ ] Move `cpe/docs/CONFIG_LANGUAGE.md` to `docs/src/cpe/config-language.md` under a new `## CPE`
      section of `docs/src/SUMMARY.md`; the heading `` ## `remodel` block `` keeps its level, which
      `worked_example.rs:19-21` splits on
- [ ] Repoint every reference to the incubator's `CONFIG_LANGUAGE.md` under `areas/access/cpe/`
      (`rg -l CONFIG_LANGUAGE areas/access/cpe`), `CONTEXT.md` included
- [ ] Copy `cpe/store-build` without `src/main.rs`, `tests/it/cli.rs`, `tests/it/fresh.rs` and
      `tests/it/dockerfile.rs`
- [ ] Write its manifest: `cpe-core` and `cpe-ports` by path; `serde_json`, `thiserror` and
      `unicode-normalization` from the workspace; `rusqlite` and `sync-store` as dev-dependencies; no
      `pico-args`
- [ ] Add `unicode-normalization` to `[workspace.dependencies]` and `float_roundtrip` to `serde_json`'s
      features, with a comment that names the builder's geometry parsing
- [ ] Add the member to the root `Cargo.toml`
- [ ] Stage, run `just fmt` until stable, save the formatted baseline
- [ ] Make the edits of the `cpe-store-build` table in *The source at `d68f214b`*: the three `mod`
      lines, `DATA_DIR`, `PROJECTS_DIR`, the four `hygiene.rs` sites, `geometry.rs`'s include, and
      `worked_example.rs`'s path
- [ ] Rewrite `README.md` for the library: no command line, no `data.sql`, incubator ADRs by file name
- [ ] Confirm the header strings of `sql.rs` are byte-equal to `SRC`'s
- [ ] Render 0803 with `sql::to_sql` and `sql::header("incunabula", …)` in a scratch test and compare the
      text with `cpe/projects/incunabula/data.sql` from the export; it is byte-equal; write the result to
      `.claude/tmp/dev-7465/pr-b.md`; commit neither the test nor the file
- [ ] Run the move's proofs for `cpe-store-build`; the test list equals `SRC`'s minus the 26 tests of
      the three files left behind (245 remain)
- [ ] Add rule 4 to `.github/scripts/check-cpe-seam.sh`: no manifest under `areas/access/cpe/` names
      `sync-store` outside `[dev-dependencies]`; add its cases to `check-cpe-seam.test.sh`
- [ ] Amend ADR-0008's "Enforced by" as *Prior decisions this plan changes* describes
- [ ] Amend ADR-0007: `cpe-store-build` is a crate of the capability beside the roles ADR-0003 lists;
      the config-language specification now lives here; Incunabula's folder and what it holds
- [ ] Add `cpe-store-build` to both scope tables and to `docs/src/repo_structure.md`
- [ ] Update `areas/access/cpe/CONTEXT.md` and `ARCH-MAP.md`: the builder, its hook table, and the rule on
      `sync-store` with its new enforcement level
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-store-build): move the store builder and the remodelling in from dsp-incubator (DEV-7465)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: the builder's comments follow `CONVENTIONS.md`

### dsp-repository

- [ ] Remove plan, ticket and history references from `cpe-store-build`'s comments
- [ ] Prove the commit changes comments only
- [ ] Run `just check`; it passes
- [ ] Commit as `docs(cpe-store-build): remove history and plan references from the comments (DEV-7465)`
- [ ] Point `worktree-DEV-7465-store-build` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: `cpe-web` enters `areas/access/cpe/web`

### dsp-repository

- [ ] Copy the two logos and `dasch-fingerprint.svg` to `areas/access/cpe/public/`
- [ ] Copy `datastar.js` to `areas/access/cpe/public/vendor/` with a `README.md` row in the form of
      `areas/access/dpe/public/vendor/README.md`; `just verify-checksums` passes
- [ ] Copy `cpe/web` with `tests/`, its 234 fixture folders and `src/snapshots/`
- [ ] Write its manifest: `cpe-core`, `cpe-store`, `cpe-components`, `cpe-image-viewer` by path; `maud`,
      `rusqlite`, `thiserror` from the workspace; `insta` as a dev-dependency
- [ ] Add the member to the root `Cargo.toml`
- [ ] Stage, run `just fmt` until stable, save the formatted baseline
- [ ] Bind the two `html!` blocks passed to `blocks.push` (`page.rs:2422`, `:2443`) with `let`; fix any
      other `html!` site that `maudfmt` or clippy rejects the same way
- [ ] Repoint the five `include_str!` of the `cpe-web` table in *The source at `d68f214b`*
- [ ] Qualify the seven bare incubator ADR references of that table; `just check-adr-refs` passes
- [ ] Run the move's proofs for `cpe-web`: 497 tests, the snapshot folders equal by `diff -r`, no test
      ignored
- [ ] Add `cpe-web` to both scope tables and to `docs/src/repo_structure.md`
- [ ] Update `areas/access/cpe/CONTEXT.md` and `ARCH-MAP.md` for the page assembly and the static files
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-web): move CPE's page assembly in from dsp-incubator (DEV-7465)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: the page assembly's comments follow `CONVENTIONS.md`

### dsp-repository

- [ ] Remove plan, ticket and history references from `cpe-web`'s comments
- [ ] Prove the commit changes comments only
- [ ] Run `just check`; it passes
- [ ] Commit as `docs(cpe-web): remove history and plan references from the comments (DEV-7465)`
- [ ] Point `worktree-DEV-7465-web` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: `cpe-server` holds CPE's router

### dsp-repository

- [ ] Assign every test of `cpe/bin/src/router.rs`, `cpe/bin/src/lib.rs` and `cpe/bin/src/discovery.rs`
      at `SRC` to "moves", "rewritten here" or "stays in the incubator" by the rules of the test table in
      *The source at `d68f214b`*; the three lists together equal the list at `SRC` (249 at `d68f214b`);
      write them to `.claude/tmp/dev-7465/pr-d.md`
- [ ] Write again, over a fixture project, the assertions of the five fallback tests that use Alice's id
      only as a project id
- [ ] Create `areas/access/cpe/server` with `router.rs`, the request preferences, the query readers and
      `OriginConfig` from `cpe/bin`, naming the four crates directly in place of `cpe_engine::`; leave
      `request_span` behind (decision 19)
- [ ] Serve the seven stylesheets and the four files of `areas/access/cpe/public` from an
      `include_bytes!` table at their present URLs, with `text/css` and `image/svg+xml` among its content
      types; remove the `include_dir` parameters from the router's build function
- [ ] Add a test that every file of `components/style`, `public` and `public/vendor` has a row
- [ ] Rewrite discovery to read a directory on disk as *Files from disk* describes; write its tests
      first: a symbolic link is not followed, and a folder whose name fails the id rule is not a project
- [ ] Serve project assets from the inventory; write the tests first: `..`, `%2e%2e`, `%2f`, a backslash,
      a NUL byte and an absolute path in the tail are answered 404, and so is an unknown project segment
- [ ] Remove the site index route; add `Cpe::projects()` (decision 13)
- [ ] Give the router an explicit fallback: the project's not-found under a known project, CPE's
      otherwise; it never redirects and never echoes the path
- [ ] Add `CpeConfig` (decision 14, without the base) with tests for the refused public origin
- [ ] Add the `Cpe` facade with `new`, `router` and `projects`, and the doc-hidden constructor that loads
      project folders holding a `data.sql` (decision 2), with a doc comment that names DEV-7498 as its end
- [ ] Add a test in the form of `dpe-server`'s that no `pub` item names a type of another CPE crate but
      `cpe-ports`
- [ ] Move the tests assigned "moves", with `cpe/bin/tests/fixtures/discovery` and `…/static`; correct
      the comments that name `engine/src/…`, `engine/tests/…` or `cpe_engine::`
- [ ] Add `futures-util` to `[workspace.dependencies]`; `tower` with `util` is a dev-dependency
- [ ] Stage, run `just fmt` until stable
- [ ] Run `cargo clippy -p cpe-server --all-targets -- -D warnings`; it passes
- [ ] Add `cpe-server` to both scope tables and to `docs/src/repo_structure.md`
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-server): add CPE's host crate from the prototype's binary (DEV-7465)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: a project's store is built through the port

### dsp-repository

- [ ] Write the tests first, over a `FakeArchiveProjection`: an activated fixture project serves its
      pages; before `activate` every path is answered 503 with `Retry-After` and `Cache-Control:
      no-store`; with an unknown shortcode and with an unavailable projection `activate` returns, that
      project's prefix is answered 503, a second, valid project is served, and a path that names no
      project is 404; with a missing projects directory every path is CPE's not-found; ADR-0008's test of
      *Verification design*
- [ ] Implement `Cpe::activate` and the forwarding service as *The host crate and activation* describes
- [ ] Add `sync-store` as a dev-dependency of `cpe-server` and a test that Incunabula activates from
      `LiveArchiveProjection` over `areas/access/sync/data` and serves its start page, a book page and a
      record page with status 200; the test fails when the data directory is missing
- [ ] Amend ADR-0007 as *Prior decisions this plan changes* describes: the hook's place and the table;
      "over the port's types"; what of "a project fails alone" is built and what DEV-7403 owns; the store
      is built at activation from SQL text
- [ ] Update `areas/access/cpe/CONTEXT.md`, `areas/access/CONTEXT.md` and `ARCH-MAP.md`: the host crate,
      the project folder, the durable-state entry (the store is built at every start and kept nowhere)
- [ ] Run the root parity crawl of *Verification design*; no unexpected difference; write the URL count
      and the result to `.claude/tmp/dev-7465/pr-d.md`
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-server): build a project's store from the archive projection at activation (DEV-7465)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 9: the host crate's comments follow `CONVENTIONS.md`

### dsp-repository

- [ ] Remove plan, ticket and history references from `cpe-server`'s comments, DEV-7498 in the doc-hidden
      constructor's comment excepted
- [ ] Prove the commit changes comments only
- [ ] Run `just check`; it passes
- [ ] Commit as `docs(cpe-server): remove history and plan references from the comments (DEV-7465)`
- [ ] Point `worktree-DEV-7465-server` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 10: the store owns its connection

### dsp-repository

- [ ] Write the tests first for the owning type: open from SQL text, facts, search indexes, ARKs, reads
- [ ] Add the owning type to `cpe-store`; `open_project_db`, `LiveProjectFacts::new`,
      `build_search_indexes`, `data_arks` and `ProjectReads::new` stop being `pub`
- [ ] Remove `rusqlite::Error` from the fields of `DbError` and `QueryError` that a caller outside the
      crate can reach (decision 17)
- [ ] `Project` holds the owning type; remove `rusqlite` from `cpe-web`'s manifest
- [ ] Confirm `rg 'rusqlite' areas/access/cpe --glob '!store/**' --glob '!store-build/**'` finds nothing
      outside dev-dependencies
- [ ] Record in `ARCH-MAP.md` that `rusqlite` is named by `cpe-store` alone among CPE's `[dependencies]`
      (review), and the debt of decision 16: reads run on the async worker thread
- [ ] Run `cargo test -p cpe-web` and `cargo test -p cpe-server`; no snapshot changes
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `refactor(cpe-store): own the connection, so the page assembly names no rusqlite (DEV-7465)`
- [ ] Point `worktree-DEV-7465-handle` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 11: CPE builds its URLs from a configured base

### dsp-repository

- [ ] Write the tests first for the URL builder in `cpe-core`: page, fragment, static, asset and share
      URLs for an empty base and for `/cpe`; a project segment; a path with a query
- [ ] Write the tests first in `cpe-server`, with the base `/cpe`: the "no URL leaves the base" test of
      *Verification design*; both redirects' `Location`; both cookies' `Path`; a refused base at load
- [ ] Add the URL builder; `RenderContext` carries the base
- [ ] Send every URL of `cpe-components`, `cpe-web` and `cpe-server` through it, row by row of the site
      table in *The base path and the mount*
- [ ] Set `current_path` to the base plus the request path at the five sites that build a
      `RenderContext`, and build both redirects from the same value
- [ ] Send a narrative link that starts with one `/` through the builder, in the Markdown renderer and
      the invite lists (decision 21); a link that starts with `//`, a scheme or `#` is left as it is;
      write the tests first
- [ ] Move static files to `<base>/<shortname>/_static/…` and project assets to
      `<base>/<shortname>/_assets/…`; serve the static table at `<base>/_static/…` too, and have CPE's
      not-found and the 503 load from there (decision 20)
- [ ] Pass the viewer's `head_tag` `<base>/<shortname>/_static/image-viewer`
- [ ] Amend ADR-0007: `<base>/_static/…` for pages without a project; narrative links get the base
- [ ] Add `CPE_BASE` to `CpeConfig` and `Cpe::base()`; set both cookies' `Path` from it
- [ ] Confirm `rg -n '"/_static|"/_assets|href="/|src="/|format!\("/|Path=/' areas/access/cpe --glob '*.rs'
      --glob '!**/tests/**' --glob '!**/snapshots/**'` lists test modules only; put the list in
      `.claude/tmp/dev-7465/pr-f.md`
- [ ] Confirm `rg -n 'url\(/' areas/access/cpe/components/style` finds nothing
- [ ] Review every changed snapshot with `cargo insta review -p cpe-components` and `-p cpe-web`; each
      change is a URL and nothing else (about 91 and 29 files)
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(cpe-core): build CPE's URLs from a configured base (DEV-7438)`
- [ ] Point `worktree-DEV-7465-base` at `HEAD`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 12: `access-server` constructs and mounts CPE

### dsp-repository

- [ ] Add `cpe-server` to `PORT_EXCEPTIONS` in `.github/scripts/check-cpe-seam.sh`, correct the script's
      header, and add a case to `check-cpe-seam.test.sh` (decision 9)
- [ ] Add `cpe-server`, `cpe-ports` and `sync-store` to `areas/access/server/Cargo.toml`
- [ ] Write the tests first in `serve.rs`, with a `Cpe` activated over a `FakeArchiveProjection`: every
      row of the request table in *The base path and the mount*; DPE's routes unchanged; a CPE route
      carries a `traceparent` and `/healthz` does not; the span of a CPE request is named by its route
      template; a cookie set under `/cpe` has `Path=/cpe`
- [ ] `app()` takes the `Cpe` and mounts it with `nest(cpe.base(), cpe.router())` before the OTel layers,
      with the explicit route for the base followed by `/`
- [ ] In `serve()`, build `Cpe` from `CpeConfig`, construct `LiveArchiveProjection` over `SYNC_DATA_DIR`,
      and run `cpe.activate` in `spawn_blocking` after the bind, beside `dpe.warm()`
- [ ] Add a test that with `SYNC_DATA_DIR` pointing at a directory that does not exist the binary's
      router still answers `/healthz` and `/dpe/projects`, and `/cpe/incunabula/` is answered 503
- [ ] Port the prototype's open-redirect tests; run them under the mount and at a root mount
- [ ] Update `areas/access/server/CLAUDE.md`, the "Today it mounts DPE alone" text and the exception-list
      sentences in `ARCH-MAP.md`, and `areas/access/sync/CONTEXT.md` ("nothing constructs it")
- [ ] Amend ADR-0007: the seam exception is `cpe-server` alone
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `feat(access-server): construct CPE over sync's archive projection and mount it under /cpe (DEV-7438)`

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 13: the image carries CPE's data, and the done-criterion is checked

### dsp-repository

- [ ] Add `areas/access/cpe/projects` to `.github/actions/build-dpe/action.yml`'s staging and to
      `areas/access/server/Dockerfile`, in the shape #478 gave `sync/data`; leave `/app/sync/data` as it is
- [ ] Set `ENV SYNC_DATA_DIR="/app/sync/data"`, `CPE_PROJECTS_DIR` (absolute) and `CPE_BASE=/cpe` in the
      Dockerfile
- [ ] Add `SYNC_DATA_DIR`, `CPE_PROJECTS_DIR`, `CPE_BASE` and `CPE_PUBLIC_ORIGIN` to the environment
      table of `docs/src/dpe/operations.md`, and correct its "Docker Image" sentence that nothing in the
      binary reads `/app/sync/data`
- [ ] Add `areas/access/cpe/**` and `areas/access/sync/**` to the path filter of
      `.github/workflows/a11y-dpe.yml`
- [ ] Add an `eng.yaml` override for `areas/access/cpe/store/**` and `areas/access/cpe/server/**` naming
      `eng:review:security-reviewer`; validate it with `eng:eng-yaml`
- [ ] Stage a directory laid out as the image, start the release binary in it with the Dockerfile's
      environment, and confirm `/healthz` answers before the log line that reports Incunabula active
- [ ] Measure the time from process start to that log line and the process's peak memory; write both, and
      the size of the staged directory before and after, to `.claude/tmp/dev-7465/pr-g.md` and to
      `docs/src/cpe/` as an operations note
- [ ] Amend ADR-0007: DEV-7400's done-criterion, how it was checked, and the expected differences
- [ ] Update `docs/src/` where it describes what the area binary serves
- [ ] Run the `/cpe` parity crawl of *Verification design*; no unexpected difference
- [ ] Record the crawl's slowest five responses, with their URLs, in the operations note: the baseline
      for the debt of decision 16
- [ ] Run `eng:test-browser` on Incunabula's start page, a book page and a record page under `/cpe`; no
      defect reported
- [ ] Confirm `rg -n 'include_dir|pico-args|anyhow' Cargo.toml areas/access/cpe` finds nothing and
      `Cargo.lock` holds one `rusqlite`
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Commit as `build(access-server): ship sync's data and CPE's projects in the image (DEV-7465)`
- [ ] Adopt the seven branches with `gh stack init`, bottom to top, with `--base worktree-DEV-7407`
      where #478 is still open, and submit with `gh stack submit --auto`; where `gh stack` cannot use
      that base, build the stack with `gh stack link`
- [ ] Set each PR's body from the template and its file under `.claude/tmp/dev-7465/`, with *Human
      Actions* copied in, and the assignee with `gh pr edit`; every PR is a draft; request no reviewer
- [ ] Confirm CI is green on each of the seven PRs

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 14: the incubator builds against the vendored CPE

**Gate: H3, H4** — resolve before starting this phase.

### dsp-incubator

- [ ] Rebase the branch on `origin/main`; run `git diff SRC origin/main -- cpe/store cpe/store-build
      cpe/web cpe/bin cpe/static cpe/projects/incunabula`; where it shows a change, stop and report it
- [ ] Set `<pin>` to the commit on dsp-repository's `origin/main` that PR G's last commit became
- [ ] Add the four crate directories to `cpe/vendor/CRATES`, and `areas/access/cpe/projects` and
      `areas/access/cpe/public` to `cpe/vendor/DATA`; run `just cpe vendor <pin>`
- [ ] Make every `[workspace.dependencies]` key the vendored manifests inherit byte-equal to
      dsp-repository's at `<pin>`, features included
- [ ] `git rm -r cpe/store cpe/store-build cpe/web cpe/engine cpe/projects/incunabula`; remove from
      `cpe/static` the files the vendored tree now holds
- [ ] Reduce `cpe/bin` to a host over `cpe-server`: the site index from `Cpe::projects()`, the
      prototype's `request_span`, Alice and HdM through the doc-hidden constructor, Incunabula through
      `LiveArchiveProjection` over the vendored data; an empty base; workspace dependencies only; no
      `include_dir`, no `build.rs`
- [ ] Keep the tests assigned "stays in the incubator" in Phase 7 and `smoke.rs`, running against the
      vendored router; correct their comments that name `engine/…`
- [ ] Update `cpe/Dockerfile`, `cpe/justfile` (`store-build`, `translate-check`, `watch`, the module-graph
      loops), `cpe/tools/crate-deps.py` and `cpe/tools/module-graph.py`
- [ ] Update `cpe/vendor/README.md`, `cpe/CLAUDE.md`, `cpe/CONTEXT.md`, `cpe/docs/ARCHITECTURE.md`,
      `cpe/docs/INCUNABULA_GO_LIVE.md` (DEV-7464, DEV-7401, DEV-7465 and DEV-7438 ticked) and
      `cpe/docs/BACKLOG.md` (the items this plan closes); replace `cpe/docs/CONFIG_LANGUAGE.md` with a
      pointer to dsp-repository
- [ ] Add dated notes to `cpe/docs/adr/0001-crate-layout.md`, `0013`, `0015` and `0016`
- [ ] Compare the workspace's test names with the list saved in Phase 1; the difference is the tests that
      moved and the tests of the files left behind in Phase 3
- [ ] Run the parity crawl over Alice, HdM and Incunabula against the prototype built at `SRC`; no
      unexpected difference
- [ ] Run `just cpe test`; it passes with no `.snap.new`
- [ ] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<pin>)`
- [ ] Run `just cpe ci`; it passes
- [ ] Commit as `refactor(cpe): build against the vendored store, page assembly and host (DEV-7465)`
- [ ] Open a draft PR with `--assignee @me`; never enable auto-merge

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Keep DEV-7407's worktree clean and its branch level with its upstream while #478 is open | Balduin | before start | That branch is checked out in another worktree, and `gh stack` replays onto it |
| H2 | From the start until H5: hold incubator PRs that change `cpe/store`, `cpe/store-build`, `cpe/web`, `cpe/bin`, `cpe/static` or `cpe/projects/incunabula`, or land the change in dsp-repository first | Balduin | before start | Which PRs merge is the owner's decision |
| H3 | On PR G's preview, open an Incunabula page with the image viewer under `/cpe` and use the zoom and paging controls | Balduin | before Phase 14 (Why mid-plan: it needs the stack's build and must come before PR G merges in H4) | The viewer's JavaScript cannot be exercised from an automated browser |
| H4 | After H3: merge #478 if it is still open, then PRs A to G bottom-up, each with green CI after the one below it merged. Merging PR G puts `/cpe` into the next image | Balduin | before Phase 14 (Why mid-plan: the vendor pin must be a commit on `main`, which a rebase-merge mints only on merge, and the incubator cannot delete its engine before that commit exists) | Merging is the owner's decision |
| H5 | Merge PR H | Balduin | after ship | Merging is the owner's decision |
| H6 | Check whether `ops-deploy` needs `CPE_PROJECTS_DIR`, `CPE_BASE` and `CPE_PUBLIC_ORIGIN` set, or more memory for the area binary (`SYNC_DATA_DIR` is ops-deploy#1460) | Balduin | after ship | A deployment decision outside both repositories |
| H7 | Tell Ivan that ADR-0010 is still proposed while CPE now serves curated values (DEV-7488) | Balduin | after ship | An owner conversation |
| H8 | File two follow-up issues: store reads off the async worker thread (decision 16, with Phase 13's timings), and project-relative narrative links (decision 21) | Balduin | after ship | A backlog decision |

## Acceptance Criteria

### dsp-repository

- [ ] `cpe-store`, `cpe-store-build`, `cpe-web` and `cpe-server` are workspace members under
      `areas/access/cpe/`, named in both scope tables and in `ARCH-MAP.md`
- [ ] `access-server` depends on `cpe-server`, `cpe-ports` and `sync-store`, and on no other CPE crate;
      `check-cpe-seam.sh` and `check-composition-root-deps.sh` pass
- [ ] `access-server` constructs `LiveArchiveProjection` and hands it to CPE; no CPE crate names
      `sync-store` outside `[dev-dependencies]`, and `check-cpe-seam.sh` checks that
- [ ] Incunabula's store is built from the port after the process starts; no `data.sql` for it is in the
      repository
- [ ] One test rebuilds a store from empty against `FakeArchiveProjection` and compares what is served
- [ ] The area binary answers `/healthz` and DPE's routes while CPE activates, and when a project fails
      to activate
- [ ] Incunabula's pages under `/cpe/incunabula/…` equal the prototype's at `SRC` byte for byte once the
      base, the static and asset paths and the cookies' `Path` are normalised
- [ ] No rendered page, fragment or redirect under `/cpe` holds a root-absolute URL outside the base,
      narrative links, share links and the reader's history URLs included
- [ ] Every row of the request table is a passing test in `access-server`
- [ ] No request path is joined onto a directory; the traversal tests of Phase 7 pass under `/cpe` too
- [ ] The workspace has no `include_dir`, no `pico-args`, and one `rusqlite`; `cpe-web` names no
      `rusqlite`
- [ ] ADR-0007 and ADR-0008 carry the amendments of *Prior decisions this plan changes*
- [ ] `docs/src/cpe/` holds the config-language specification and the operations note
- [ ] The image holds `sync`'s data and Incunabula's folder
- [ ] `just check` and `just test` pass at the last commit of every PR, and CI is green on each

### dsp-incubator

- [ ] `cpe/store`, `cpe/store-build`, `cpe/web` and `cpe/engine` are gone; `cpe/bin` depends on
      `cpe-server` and names no `cpe_engine`
- [ ] Alice, HdM and Incunabula render as at `SRC`, by the crawl
- [ ] `just cpe vendor-diff` is empty at the pin and `just cpe ci` passes

## Dependencies & Risks

- **No open blocker.** DEV-7401's incubator re-vendor (#508) merged on 2026-10-11; DEV-7402, DEV-7464
  and DEV-7497 are in QA with no open PR. DEV-7407's #478 is open, and the stack sits on it. Its
  ops-deploy#1460 matters at deployment only.
- **ADR-0010 is proposed.** A rejection removes curation from the port, and with it the catalogue values
  Incunabula's hook reads. The builder would have to change here.
- **The stack is long.** A review fix low in the stack replays every PR above it.
- **The forwarding service is new code** that the prototype does not have. It sits in front of every CPE
  request, and the parity crawl at the root runs through it.
- **`serde_json`'s features differ between the two workspaces.** If the builder's output moves, Phase 3
  stops there: the fix is in the builder, not in the comparison.
- **`maudfmt` on 14,000 lines of `html!`.** It may reject more sites than the component crate had.
- **Start-up cost.** Activation parses 20 MB of N-Quads and executes about 51,000 lines of SQL. It no
  longer delays the listener, but its memory peak is unknown and the preview has 512 MiB. Phase 13
  measures it before anything merges.
- **Reads block an async worker.** Unchanged from the prototype, and recorded as debt (decision 16).
- **The Alice tests guard moved code from the other repository** (decision 6). Phase 14's crawl is the
  check at the pin move.
- **Between PR H's merge and DEV-7498** the store has two entries, one of them not for production.
- **Not in this plan:** the designed offline page and the strict `validate` verb (DEV-7403), the test
  pyramid (DEV-7404), Alice and HdM through the port (DEV-7498), deleting the incubator's remaining engine
  parts (DEV-7408), a direct SQLite sink, cache headers for the embedded static files, CPE's own telemetry
  identity, the joint `rusqlite` and `deadpool-sqlite` bump.

## Success Metrics

Counts as read at incubator `d68f214b`; they are taken again at `SRC`.

- `cpe-store`: 185 tests. `cpe-store-build`: 245 of 271 (26 stay behind).
- `cpe-web`: 497 tests, 30 snapshots and 234 fixture folders, all present after the move.
- `cpe/bin`: of 249 tests in `src/`, 156 move, 24 are rewritten here and 69 stay; the 74 of `smoke.rs`
  stay.
- Parity crawls: zero unexpected differences. The incubator's last crawl of this kind reached 40,504 URLs
  per server over three projects.
- `rg -c '\.lock\(\)' areas/access/cpe --glob '!store/**'` is zero after Phase 10.
- Time from process start to Incunabula active, and peak memory, measured in Phase 13.

## References

- Linear: DEV-7465, DEV-7438, DEV-7400, DEV-7403, DEV-7405, DEV-7498, DEV-7488
- `docs/adr/0003-one-modulith-per-area.md`, `0007-cpe-joins-the-access-area-as-its-second-capability.md`,
  `0008-a-reading-capability-may-keep-a-derived-read-model.md`,
  `0010-the-port-also-serves-per-resource-curation.md`, `0011-cpe-crates-move-in-dependency-order.md`
- `ARCH-MAP.md` (`areas/access/server`, `areas/access/cpe`, `areas/access/sync`)
- `docs/specs/2026-10-11-move-cpe-core/01-feat-move-cpe-core-plan.md` and
  `docs/specs/2026-10-11-move-cpe-components/01-feat-move-cpe-components-plan.md`: the move procedure
- `docs/specs/2026-09-28-access-area-composition-root/`: the root's fallback and the store-crate rule
- `.github/scripts/check-cpe-seam.sh`, `.github/scripts/check-composition-root-deps.sh`,
  `.github/scripts/verify-checksums.sh`
- `areas/access/server/src/serve.rs`, `areas/access/dpe/server/src/lib.rs`: the facade `cpe-server`
  follows; `areas/access/cpe/image-viewer/src/lib.rs`: the `include_bytes!` table
- `areas/deposit/editor/server/src/db/mod.rs`: the editor's rule on a `Mutex` around a connection
- dsp-incubator: `cpe/docs/adr/0001-crate-layout.md`, `0013`, `0015`, `0016`; `cpe/docs/BACKLOG.md`;
  `cpe/docs/INCUNABULA_GO_LIVE.md`; `cpe/docs/plans/engine-crate-split/`; `cpe/vendor/README.md`;
  `cpe/docs/learnings/best-practices/proving-a-move-only-refactor-byte-identical-takes-more-than-snapshots.md`
- Learnings: `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`,
  `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`,
  `docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`,
  `dasch-specs/learnings/best-practices/visual-parity-oracle-worktree-ab-for-big-bang-ui-refactors.md`,
  `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`
