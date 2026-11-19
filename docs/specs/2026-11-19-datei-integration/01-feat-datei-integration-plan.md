---
title: "feat: bring DaTEI into the Access Area and serve Keller (083C) from it, hidden, on stage"
type: feat
date: 2026-11-19
author: "Balduin Landolt"
status: reviewed(5)
repository: dasch-swiss/dsp-repository
repositories:
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository
  - name: ops-deploy
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/ops-deploy
linear: DEV-7934
linear_project: integrate-da-tei-into-dsp-repository-bb143927be74
---

# feat: bring DaTEI into the Access Area and serve Keller (083C) from it, hidden, on stage

## Overview

DaTEI, the TEI edition presenter prototyped in `dsp-incubator/da-tei`, moves into dsp-repository as the Access Area's third capability (ADR-0009). Only the reading side moves: the engine, the Rebuild that builds a project's read model, and the presenter. The configurator stays in the incubator. The first project served is the Keller edition (eHKKA, 083C), hidden, on stage, so its polish (P-DEV-417) happens on the deployment the project will review.

The decisions are recorded in DEV-7934 (*Decided 2026-11-19*) and refined while planning (*Proposed Solution*). The work ships as:

| Phase | Repository | Pull request | Linear |
|---|---|---|---|
| 0 | dsp-incubator | DaTEI reads a project's files by name, through a layout and an optional manifest | DEV-7549 (prerequisite) |
| 1 | dsp-repository | ADRs and CONTEXT files rewritten for the decisions | DEV-7934 |
| 2 | dsp-repository | DaTEI's ports, core, store and store-build crates under Bazel | DEV-7549 |
| 3 | dsp-repository | DaTEI's web and server crates, moved, with hidden projects honoured | DEV-7549 |
| 4 | dsp-repository | Activation in the background, one project at a time, read models on disk | DEV-7574 |
| 5 | dsp-repository | `media` serves DaTEI's port; `access-server` mounts DaTEI at `/datei` | DEV-7574 |
| 6 | dsp-repository, ops-deploy | Image, CI and deployment wiring | DEV-7935 |

In dsp-repository, Phases 1 to 6 are one stack of single-commit pull requests on `main` (which has dsp-repository#606: rusqlite 0.40, deadpool-sqlite 0.14, Rust 1.95), one phase per pull request (`docs/src/git-conventions.md`, *One PR, several commits, or a stack*), managed with `gh stack`, as the Bazel migration's plan did. `docs/specs/README.md` asks for phases as commits within a pull request; here each phase is a change a reviewer judges on its own (docs, a move, new behaviour, wiring), and the stack keeps each pull request small enough to review against the 2026-12-04 target. Phase 0 is one pull request in dsp-incubator; Phase 6 adds one pull request in ops-deploy.

Target: 2026-12-04 (P-DEV-396).

## Problem Statement / Motivation

The Keller edition must be shown to its project in December as it will run in production: served by DaTEI from dsp-repository, not by the incubator's showcase. Dominique, who polishes the edition, is away from mid-December, so the integration must land early enough for the polish to happen on stage (P-DEV-417, target 2026-12-15).

DaTEI today is a separate service with its own binary (`tei-serve`), a prebuilt SQLite snapshot shipped in its image, projects found by scanning for those snapshots, a file port that serves files by content hash, and activation before the port is bound. Each conflicts with dsp-repository's rules: one crate per role (ADR-0003), read models rebuilt from empty and never shipped (ADR-0008, ADR-0009), deposited files by shortcode and name (ADR-0012), and activation after the bind, so one capability cannot hold up the Access Area.

Over time, edition pages are to be set up by RDU, not by developers: a project gets DaTEI pages when its configuration arrives, with no code or infrastructure change.

## Proposed Solution

**Data path.** A project's files reach the server through the VRE: RDU uploads them (for Keller, to a VRE test server, DEV-7921), and the *VRE bridge* (P-DEV-301, `areas/deposit/vre-bridge/`, a background task in `deposit-server`, planned separately) downloads the project's text files (TEI, ODD, CSS) without credentials, so only what the VRE serves publicly arrives, and stores them in `media`'s directory, `MEDIA_DATA_DIR/<shortcode>/`, flat, under their VRE-internal names (*stored names*). Images stay in the VRE and are served as IIIF by its Sipi. Files made with a DaTEI page rather than taken from the VRE (`datei.toml`, `file-layout.xml` and the *file mapping*, below) ship in the bridge's image, and the bridge writes them into the project's directory. After all of a project's data, the bridge writes a *Ready token* (holding the as-of time) into the project's directory; a failed sync refuses only that project and leaves its last good state. The bridge also keeps a project's files from changing while a reader reads them (confirmed in H1; its mechanism is H7). DaTEI reads its inputs only through its port, which `media` implements; it never reads the shared folder and knows nothing of the bridge.

**Port.** DaTEI's port in dsp-repository is the trait `DateiFiles` in `datei-ports`, with two operations, keyed like CPE's `DepositedFiles`:

- `projects()`: the shortcodes whose directory holds both the bridge's Ready token and DaTEI's configuration under its fixed name, `datei.toml`, so DaTEI never sees a half-written project. A project gets DaTEI pages when its configuration arrives; nothing else registers it. ADR-0012 gains one sentence allowing this listing of projects. It does not list files: ADR-0012's port carries files by name and nothing else, and the media-minimum plan rejected a listing because "a listing is what makes a port drift toward a file system". A list of the projects that hold one named file does not walk a project's files, and the Access bucket can answer it by prefix.
- `file(shortcode, name)`: a file's bytes by stored name, whole or refused, with *not found* distinct from *unavailable*.

**Within a project.** The Rebuild reads one project through a narrower trait, `FileSource::file(name)` (in `datei-ports` once moved). The configurator's directory adapter implements it for one project root; in dsp-repository a wrapper binds `DateiFiles` to one shortcode. DaTEI learns the project's files from two documents read by fixed names:

- `file-layout.xml`: every file of the edition by **logical path** (its path in the edition's own directory tree, with spaces, umlauts and all) and **role** (edition, odd, css, driving-file, toc). Its SHA-256 values are advisory: a mismatch is a warning naming the file.
- `file-manifest.json`, written by the bridge: `{"format": "file-manifest", "version": 1, "files": […]}`, one entry per key, either `{"key", "iri", "stored"}` for a downloaded file or `{"key", "iri", "iiif"}` for an image. The key is the logical path as `file-layout.xml` writes it, compared exactly. DaTEI finds each layout entry's stored name by its key. Without a manifest, stored names are the logical paths (the incubator, fixtures). DaTEI reads only the format and version it knows, and refuses others.

The bridge builds the manifest from the **file mapping**, made by hand with a DaTEI page and shipped in the bridge's image: `{"format": "file-mapping", "version": 1, "files": [{"key": …, "iri": …}]}`, one entry per file DaTEI references, from its logical path to the IRI of the VRE resource that holds the file. An IRI, not a download link or a stored name, survives replacing the file on the same resource; original file names are not used (not unique in the VRE, base name only). How an author finds the IRIs (id2iri, DSP-APP, by hand) is theirs. DaTEI does not read the mapping; it is the contract between a DaTEI page and the bridge, which resolves each IRI in the project export and refuses the project when one is missing or holds no file.

`datei.toml` is a `ConfigDoc` in the format the configurator's `backup` writes today. Logical paths stay DaTEI's identity for files and the manifest's keys, so cross-references, ODD includes and durable ids are unchanged. DaTEI's own name rule refuses only what could escape a project (`..`, absolute paths, `\`, NUL); `media` additionally enforces CPE's strict name rule, which stored names and the three fixed names satisfy.

**Crates (Phases 2, 3).** The engine is split into ADR-0003 roles: `datei-ports`, `datei-core`, `datei-store`, `datei-store-build`, `datei-web`, `datei-server`. `datei-web` reads through `datei-store`, the edge `cpe-web` has on `cpe-store`; the viewer's raw SQL moves into `datei-store` functions over a handle type, so `datei-web` never names `rusqlite`. `datei-store` holds the database type `IrDb` (opening, pragmas, schema, reads) and the read-only pool; `datei-store-build` holds what writes. The presenter's own SQL (search with its query deadline, index pages, finders) stays in `datei-server` for now; moving it into `datei-store` joins DEV-7939. `tei-serve` goes away; `datei-server` is a facade that `access-server` mounts. The edges: `datei-core` → {ports}; `datei-store` → {core, ports}; `datei-store-build` → {core, ports, store}; `datei-web` → {core, store} (and `store-build` for test seeding only, `testonly`); `datei-server` → {core, ports, store, store-build, web}.

**Hidden projects.** DEV-7919 is on `main`: `access-server` reads `ACCESS_HIDDEN_PROJECTS` once and hands the set to each presenting capability, and `areas/access/CONTEXT.md` lists four obligations for a hidden project, which DaTEI takes on in Phase 3 as CPE has. Keller is already hidden: `repository.yml` passes `ACCESS_HIDDEN_PROJECTS` with the default `0803,083C,086A` to every environment (ops-deploy#1485).

**Change.** New or changed data reaches DaTEI through `media`'s port, never through the folder: whether `media` announces it or DaTEI asks `media` for changes is DEV-7938's design. Until then, a restart picks it up.

**Activation (Phase 4).** `access-server` binds and reports healthy at once. Once CPE's activation has finished (or at once when CPE is not mounted, and also when CPE's activation panicked), DaTEI asks its port for projects and, one at a time, builds each read model from empty into `DATEI_READ_MODEL_DIR`. Every DaTEI route answers 503 with `Retry-After` until discovery has run; then a project answers 503 until its model is ready, and a project that fails answers 503 alone. After a restart DaTEI's projects are away until rebuilt (about 4.5 minutes for Keller). Keeping models across restarts and swapping them is DEV-7938, before prod. New or changed data reaches DaTEI on the next restart; restarts are routine.

**Mount (Phase 5).** `access-server` nests DaTEI's router at `DATEI_BASE`, as it nests CPE's, (`/datei` in the image): pages at `/datei/<shortcode>/…`, static files and the self-hosted fonts at `/datei/_static/…`, no overview at `/datei/`. Telemetry stays under `access-server`'s identity, spans named after the matched route.

**Deployment (Phase 6).** The image carries DaTEI's environment; ops-deploy gives the stage and prod containers a writable host directory for read models (on the repository VMs' data storage from DEV-7513) and enough memory for one rebuild. Dev gets neither: deposit has no dev VM, so no bridge feeds dev, DaTEI lists no projects there, and the image's `/tmp/datei` suffices. Which DaTEI projects reach a server is the bridge's allowlist; Keller goes on stage's allowlist only once a deploy with `ACCESS_HIDDEN_PROJECTS` containing 083C has reached stage (H6).

**Polish on stage.** Configuration and CSS changes reach stage through the bridge and a restart. If many arrive, Balduin and Dominique batch them by hand; no tooling for it is planned.

## Alternative Approaches Considered

- **A listing of a project's files on the port.** Rejected: the layout names every file with its role, and the media-minimum plan's reason against listings holds. The port lists projects only.
- **Projects listed per deployment (`DATEI_PROJECTS`) or as committed project folders, as CPE does.** Rejected: every new edition would need a developer or an infrastructure change, while edition pages are to be set up by RDU. CPE follows later (DEV-7948).
- **Port by content hash, as in the incubator.** Rejected: it rested on the assumption that dsp-repository would serve files by hash, which ADR-0012 decided against; strict hashes would refuse every file edited in the VRE.
- **`file-layout.xml` carries the stored names.** Rejected: it would need regenerating whenever a file is re-uploaded and gets a new internal name. The mapping by IRI survives re-uploads.
- **Join on original file names.** Rejected (P-DEV-301 discovery): the VRE keeps base names only, and they are not unique. The hand-made mapping by IRI replaces it.
- **`projects()` and `datei.toml` in the incubator too.** Rejected: the configurator keys projects by a project id and keeps its configuration in its database, so the incubator gets only the per-project `FileSource`.
- **Ship the prebuilt database, or rebuild before the bind.** Rejected: ADR-0009 rejects the shipped snapshot, and a rebuild before the bind would hold up DPE and CPE for minutes (Infra, 2026-11-19).
- **Keep one `datei-engine` crate.** Rejected: ADR-0003's one crate per role. Views over plain structs, with no `web` → `store` edge, and the presenter's SQL moved out of `datei-server`, is the cleaner shape but touches all rendering and search: DEV-7939.
- **Merge DaTEI's base-prefixed routes into DPE's router, with no fallback.** Rejected: a path under a project's prefix that matches no DaTEI route (including `{base}/{project}/`) would fall through to DPE's fallback, without `X-Robots-Tag` for a hidden project; and a catch-all route beside DaTEI's one-segment routes is refused by axum's router (matchit 0.8.4 rejects `/src/{*filepath}` beside `/src/{file}`). DaTEI is nested with its own fallback instead. Unlike CPE, whose router is a single fallback, DaTEI's pages are routes, so they keep their route template as span name without CPE's response-extension workaround; only unmatched paths get a bare one.
- **DaTEI's lint rules in `clippy.toml`.** Not possible per crate: rules_rust's clippy aspect reads one `clippy.toml` for the whole build. A gate script enforces them instead.

## Technical Considerations

**Dependencies.** `main` has rusqlite 0.40, deadpool-sqlite 0.14 and Rust 1.95. DaTEI also needs rusqlite's `hooks` and `cache` features (the `crate.spec` takes the union), and the new crates `lightningcss` (1.0.0-alpha.71) and `cel` (=0.14.3, exact), approved in DEV-7934 with follow-ups DEV-7940 and DEV-7945. Neither has been built under Bazel's musl transition: Phase 2 checks that before any code is copied, and stops if it fails.

**Lint rules.** A gate script in `just check` refuses `std::fs`, `std::path::Path` filesystem calls and `std::process::Command` in `datei-ports`, `datei-core` and `datei-store-build`, and `Connection::open` outside `IrDb`, with a negative test. Filesystem work (the read-model directory, the swap, the marker, start-up cleanup) lives in `datei-server`; the SQL steps that finish a model (checkpoint, journal mode) are an `IrDb` method in `datei-store`. `check-capability-deps.sh` gains a ban on database crates in the `web` role (for every capability if the existing crates already pass, otherwise for DaTEI's), so `datei-web` and `datei-core` never name `rusqlite`. `datei-server` may, for the presenter's SQL, until DEV-7939.

**Port and errors.** `datei-ports` is std only (ADR-0003). `PortError` mirrors `cpe_ports::FilesError`: a hand-written enum with `UnknownProject`, `NotFound`, `RefusedName`, `Unavailable(Box<dyn Error + Send + Sync>)`, `Display` that quotes names with `{:?}` (a name with a line break cannot forge a log line), and `source()`. `Unavailable` may carry a path: logged, never rendered. `Shortcode` and `StoredName` are newtypes whose constructors are the only validation point, with a private field and `as_str()`. The join and layout failures are a separate `ProjectFilesError` in `datei-core`, which carries the logical path. The trait is synchronous, `Send + Sync` and dyn-compatible. A `media-store` test pins that DaTEI's and CPE's name checks agree on the names both accept.

**Read model on disk.** The file handling follows the configurator's `rebuild_in_place` and `replace_database` (`crates/da-tei/src/rebuild.rs`, `replace.rs`). Being filesystem work, it moves into `datei-server` (the host crate, which owns start-up), not into `datei-store-build`, whose Rebuild writes only through `IrDb` to the path it is given:

- `<shortcode>.building.sqlite` is created with `create_new` (O_EXCL), built with `IrDb` (WAL, `temp_store=MEMORY` as today), checkpointed with `wal_checkpoint(TRUNCATE)` whose result must be `(0, 0)`, switched to `journal_mode=DELETE`, and closed; then no `-wal`/`-shm` may remain.
- The file is `fsync`ed, renamed to `<shortcode>.sqlite` in the same directory (one filesystem, never across mounts), and the directory is `fsync`ed.
- A drop guard deletes the building file and its sidecars on every exit before the rename; the writer is dropped before the guard runs.
- Readers open the finished file read-only without create (`mode=ro`, `immutable=1`, as the incubator opens its published rollback-mode snapshot), with `query_only`, a pool size and a checkout timeout (deadpool's default waits forever). A missing model is an error and answers 503, never an empty database. `immutable=1` must not survive into DEV-7938, which replaces files under open pools.
- The WAL can grow to several GB before the checkpoint; Phase 5 measures the peak, and H2 is sized from it.

**Start-up and the marker.** Models are rebuilt at every start, so start-up deletes every top-level file matching DaTEI's exact name pattern (`^[0-9A-F]{4}\.(sqlite|building\.sqlite)(-wal|-shm|-journal)?$`), regular files only, never following symlinks or recursing. A build writes `<shortcode>.rebuilding` (holding an attempt count and a start time, written through a temporary file and a rename, so a surviving marker is rewritten rather than refused) and removes it on success, on failure and on a caught panic (outside the unwound closure). On shutdown, `activate` stops at the next cancellation check (between projects, between Rebuild steps, and between files within a step), deletes the build's files and the marker, and returns. A marker survives only when the process dies (an out-of-memory kill, a `SIGKILL` after the stop grace); at start, a project whose marker shows two interrupted builds in a row is marked failed and skipped, with a log line naming it and how to clear it, so a crash loop ends and a single interrupted deploy does not disable a project. `activate` runs once per process; a second call is refused and logged.

**Per-project state.** `Arc<RwLock<HashMap<Shortcode, ProjectState>>>` plus a discovery flag; `ProjectState` is `Pending`, `Ready(Arc<Project>)` or `Failed(reason)`, where `Project` bundles the read model with what the presenter loads per project today (published views, search, index pages, finders), matched exhaustively. `std::sync::RwLock` only, never held across `.await` or a build: the builder works on its own data and takes the write lock only for the final swap; a request clones the `Arc` and drops the guard before checking out a connection. Lock poisoning is tolerated (`unwrap_or_else(PoisonError::into_inner)`, as `cpe-server`). The release profile must keep unwinding (no `-Cpanic=abort`), since `catch_unwind` relies on it.

**Reads off the async workers.** `datei-store` offers a `ReadModel` that opens the read-only pool, checks out under a timeout, runs every read inside `interact` (its own threads) and maps an `interact` panic to an error; `datei-web` names no `rusqlite` or `deadpool-sqlite`, test targets included; `datei-server` maps that error to a status (a panic: a logged 500, never retried). A slow DaTEI query cannot stall DPE or CPE (`docs/src/cpe/operations.md`, *Response times*).

**Configuration.** An empty `DATEI_BASE` leaves DaTEI unmounted, checked by the host as for `CPE_BASE`. A base equal to or under `/dpe`, `/healthz`, `/telemetry`, `/ark:` or CPE's base is refused, logged, and leaves DaTEI unmounted. DaTEI creates `DATEI_READ_MODEL_DIR` if it is missing, then checks it by writing and removing a probe file, as `editor-store`'s preflight does; a refused or unwritable directory makes every DaTEI route answer 503, logged at start.

**Mount and 503.** DaTEI's router registers its routes without the base and ends in its own fallback (DaTEI's 404); `access-server` nests it at `DATEI_BASE`, as it nests CPE's. Every path under the base is therefore DaTEI's: the 503 before discovery and for a pending or failed project come from a layer on DaTEI's router, which covers its fallback too, and a hidden project's prefix always carries `X-Robots-Tag`, including `{base}/{project}/` and unmatched deeper paths. The project page is `{base}/{project}`; nothing redirects. Links keep using the base as today. `_static` is registered as `_static/{*path}`. A shortcode is four hexadecimal digits, so `_static` can never be one. `access-server` builds and catches CPE's and DaTEI's mounts separately, each on a clone of the router before it, so a panic in one leaves the other serving.

**Hidden projects.** DaTEI keeps the four obligations of `areas/access/CONTEXT.md` for the set `access-server` hands it: `X-Robots-Tag: noindex, nofollow` on every response under a hidden project's prefix (a layer, as `cpe-server`'s `robots_for_hidden_projects`; the 503 before discovery is exempt), the robots meta tag in every page shell, no overview listing it, and otherwise the same answers. "Hidden" is not access control.

**Fragments.** DaTEI's one fragment endpoint (`/{project}/search/popover`) answers `text/html` with `datastar-selector` and `datastar-mode` headers, not server-sent events as ADR-0004 describes; the move keeps it, and ADR-0009 records the deviation. Its Datastar actions (built in `datei-web` and, for search, index pages and finders, in `datei-server`) are built with `shared_datastar` (the repository's `check-datastar-attributes.sh` refuses hand-built `@get(` strings), which gains the `.form()` option the scoped search submit needs. The edition body keeps its `data-ignore` container, which stops Datastar from reading deposited `data-*` attributes.

**Security.** DaTEI renders HTML from deposited TEI, applies a project's CSS (root-relative `url()` refused via `lightningcss`) and evaluates `cel` display conditions from the deposited configuration. `eng.yaml` gets the security reviewer on `areas/access/datei/store/**` and `server/**`, as CPE has. Static files are served from a fixed table with `X-Content-Type-Options: nosniff`; fonts keep their immutable cache policy, `style.css` and `datastar.js` do not. Path-traversal tests and the never-echo-the-path rule are copied from CPE. Rights per file are not carried yet (DEV-7934, decision 7a).

**Accessibility.** No control or view changes: the presenter's pages move as they are (Phase 3 proves it with the golden crawl), apart from the base path, static file locations and the robots meta tag. The incubator's ADR-0004 audit (DEV-7573) is re-checked with `eng:test-browser` in Phase 5. DaTEI gets no end-to-end suite with JavaScript on and off yet; ADR-0009 records that.

**Formatting.** `just check` runs maudfmt and the repository's rustfmt over every file. Moved code is formatted with `just fmt` once copied; the diff is not line-comparable with the incubator, and the golden crawl proves behaviour. `@match` scrutinees inside `html!` are bound with `@let` first, and `html!` passed as a call argument moves into a `let` or helper (`dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`).

### Reviewers

Each phase review uses, as the diff warrants: `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:dune-reviewer`, `eng:review:maud-datastar-reviewer` (web, server), `eng:review:security-reviewer` (store, server, `media`), `eng:review:ivan-reviewer` and `eng:review:accessibility-reviewer` (web), `eng:review:devops-reviewer` (BUILD files of the server, CI, ops-deploy), `eng:review:observability-reviewer` (`serve.rs`, span naming, activation logs).

## Implementation Phases

#### Phase 0: DaTEI reads a project's files by name, through a layout and an optional manifest (dsp-incubator)

One pull request, squashed to one commit after a rebase on `origin/main` (`CONTRIBUTING.md`). The configurator keeps working on its project directories.

### dsp-incubator

- [x] `FilePort` (`da-tei/crates/datei-engine/src/files/port.rs`) is replaced by `FileSource`: `file(&StoredName) -> Result<Vec<u8>, PortError>` and a provided `read_text` that moves the bytes into a `String` without copying; `layout()` and `bytes(&ContentHash)` are gone
- [x] `PortError` is the final enum `NotFound`, `RefusedName`, `NotUtf8`, `Unavailable(Box<dyn Error + Send + Sync>)`, each naming the stored name; `NotInLayout`, `MissingBytes` and `HashMismatch` are gone
- [x] `StoredName` accepts any relative name that cannot escape the project (no `..` segment, no leading `/`, no `\`, no NUL); its constructor is the only validation point
- [x] `ProjectFiles` (engine, `files/`) opens a project through a `FileSource`: it reads `file-layout.xml` and, when present, `file-manifest.json`, and finds each layout entry's stored name by its key
- [x] `ProjectFiles` answers `read(&LogicalPath)`, `read_text(&LogicalPath)`, `entries(role)`, `contains(&LogicalPath)` and `layout()` (for upload's collision check, `web/handlers.rs:1616-1625`)
- [x] `ProjectFilesError` carries the logical path and covers layout, manifest and join failures
- [x] The manifest parser reads `{"format": "file-manifest", "version": 1, "files": […]}` with `stored` and `iiif` entries, and refuses another format or version; `iiif` entries are parsed and kept for later use (nothing renders them yet, DEV-7895)
- [x] A manifest that is `NotFound` means stored names equal logical paths, logged at info; an `Unavailable` or unparseable manifest is an error
- [x] A layout entry the manifest does not hold is an error naming the logical path; a manifest entry the layout does not list is ignored
- [x] Two manifest entries with the same key, or the same stored name, are an error naming both; a layout entry whose manifest entry is `iiif` rather than `stored` is an error naming the key
- [x] `FileLayout::parse` accepts an entry without `idno type="sha256"`; `ProjectFiles` checks a present hash over the bytes it read, and a mismatch logs a warning naming the logical path and both hashes
- [x] `MemoryFiles` implements `FileSource` by stored name; `insert(path, role, bytes)` keeps its signature, records the layout entry and serves `file-layout.xml` serialised from its entries
- [x] `rebuild.rs` reads through `ProjectFiles`: editions, driving files and TOCs by logical path, ODD and CSS entries by role
- [x] The Rebuild stamps `source_files.content_hash` for ODD and CSS from the bytes it read (`rebuild.rs:313-326`)
- [x] The ODD chain (`odd/parser.rs`, `parent_odd`) checks a parent's existence with `ProjectFiles::contains`
- [x] Seeding (`pipeline.rs:788-908`) and `run_with_files` (`web/lifecycle.rs:215-240`, callers `web/handlers.rs:9021` and `10298`) take `&ProjectFiles`
- [x] The configurator's `DirectoryFiles` (`crates/da-tei/src/files/directory.rs`) implements `FileSource` for one project root: `file-layout.xml` at the root, every other name under `files/`; it no longer verifies hashes
- [x] Create and upload staging (`upload/create.rs:296-390`) keep their `MemoryFiles` for staged files and read the project's files through `ProjectFiles`
- [x] `load_artefact` (`files/artefact.rs:55-97`) reads through `ProjectFiles`
- [x] `print_dropped_artefacts` (`main.rs`) reads through `ProjectFiles`; with advisory hashes a changed artefact is offered as `--path`
- [x] Reload (`web/handlers.rs:2336-2382`) reads through `ProjectFiles`
- [x] The CLI `rebuild` (`crates/da-tei/src/rebuild.rs:61`) reads through `ProjectFiles`
- [x] `port_error_status` and `open_error_status` (`web/handlers.rs:10061-10068`) map the new errors; a missing or unparseable `file-layout.xml` keeps its 409 and the `write-file-layout` hint
- [x] `write_file_layout` keeps writing hashes
- [x] Test: an entry whose bytes changed loads, and the warning names the file (`tracing-test`)
- [x] Test: a manifest maps flat stored names to keys with a space, an umlaut and two files of the same base name in different folders; the Rebuild produces the same read model as without a manifest
- [x] Test: a manifest missing a layout entry fails, naming the logical path
- [x] Test: duplicate keys fail, naming both
- [x] Test: duplicate stored names fail
- [x] Test: an unparseable manifest, an unknown format or version, and an `Unavailable` manifest each fail
- [x] Test: `StoredName` refuses `..`, a leading `/`, `\` and NUL, and accepts a path with spaces and umlauts
- [x] Tests that asserted hash refusals or the removed variants are rewritten: `crates/da-tei/tests/directory_files.rs`, `seed_through_port.rs`, `cli_file_layout.rs` (the assertions that a changed artefact is offered as `--file` and that `toc.xml` no longer matches the file layout), `datei-engine/tests/file_port.rs`, `odd_chain_port.rs`, `rebuild.rs:223-253`, and the `files/layout.rs` tests
- [x] The reproduction fixture (`datei-test-support/src/rebuild_fixture.rs`) builds through the changed `MemoryFiles`
- [x] Docs: `docs/architecture/file-port.md` describes `FileSource`, `ProjectFiles` and the manifest; "dsp-repository serves an edition's files by hash" is gone
- [x] Docs: `docs/architecture/file-layout.md` describes advisory hashes
- [x] Docs: `docs/architecture/rebuild.md` describes reading through `ProjectFiles`
- [x] Docs: `da-tei/CLAUDE.md` and the `files/mod.rs` doc comment match
- [x] Run `just da-tei ci`; it passes
- [x] Before/after check: at the merge base, `just da-tei demo-bibles <scratch>/base/bibles-assets`; at the branch head, `just da-tei repro-bibles <scratch>/base`; `tei-compare` reports no difference
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 1: ADRs and CONTEXT files rewritten for the decisions (dsp-repository)

The first pull request of the stack, on `main`. Documentation only. ADRs are rewritten in place, with no amendment sections; the commit body records what each said before (`docs/adr/CLAUDE.md`). What Phases 2 to 6 build is written as `Not yet in code: … (DEV-nnnn)` lines, never as if built.

### dsp-repository

- [x] This plan is committed under `docs/specs/2026-11-19-datei-integration/`
- [x] ADR-0009: DaTEI's crates and roles, including `datei-store-build` (as ADR-0007 names it for CPE), and every edge as *Proposed Solution* lists them (`datei-web` → `datei-store`, `datei-store-build` → `datei-store`, `datei-server` → all, `datei-web` → `datei-store-build` for tests only), `IrDb` in `datei-store`, and the presenter's SQL in `datei-server` until DEV-7939
- [x] ADR-0009: the port (`projects()` by DaTEI's configuration, `file(shortcode, name)`), `datei.toml`, `file-layout.xml` and the manifest by fixed names, the Ready token, the file mapping as the contract between a DaTEI page and the bridge, logical paths as DaTEI's file identity and the manifest's keys, advisory hashes; it replaces "discovered as a CPE project is"
- [x] ADR-0009: activation in the background after the bind, one project at a time, into a writable directory; 503 per project until ready; a project fails alone; the marker; serving the old model during a rebuild is DEV-7938
- [x] ADR-0009: the mount (`DATEI_BASE`, `/datei`, `_static`, no overview, telemetry under `access-server`'s identity, DaTEI's own CSS) and the hidden-project obligations
- [x] ADR-0009: the interim deviations with their end conditions: no rights per file (until the port carries VRE permissions), only the current configuration format (until a second DaTEI project or a configuration we do not maintain), data refreshed by restart (until DEV-7622), images served by the VRE's Sipi (until `media` serves assets), fragments as `text/html` responses rather than server-sent events (ADR-0004), no end-to-end suite with JavaScript on and off
- [x] ADR-0009: the closing paragraph no longer describes the incubator's port by hash; the plain-git, 100 MB sentence reflects that stage data arrives through the VRE bridge
- [x] ADR-0009: `Not yet in code` lines for what Phases 2 to 6 build, each with its Linear issue
- [x] ADR-0009: its Consequences no longer say `ingest` is renamed on entry or the custom-element prefix is undecided (the incubator already uses `tei-` and `rebuild`)
- [x] ADR-0012: one sentence allowing a capability's port to list the projects that hold its configuration, with the reason a project listing does not make the port drift toward a file system
- [x] ADR-0012: the stand-in is written by the VRE bridge as well as by hand, and records its provenance either way; the Ready token and the file manifest are part of its layout, and change reaches a reader through its port, never the folder
- [x] ADR-0003: DaTEI is no longer only "planned beside them" (as a `Not yet in code` line until Phase 5), and the stand-in is not only "a committed directory"
- [x] `areas/access/CONTEXT.md`: the statements that DaTEI is "not built here yet", "a vocabulary seed" and waits to declare its port say what is decided, as `Not yet in code` where the code comes later
- [x] `areas/access/datei/CONTEXT.md`: vocabulary for stored name, logical path, file mapping, file manifest, Ready token, file source, project state (pending, ready, failed)
- [x] `areas/access/datei/CONTEXT.md`: the statements on reading versions N and N-1, rights per row, "no code here yet" and the open URL layout say what is decided
- [x] `areas/access/media/CONTEXT.md`: the stand-in on stage is written by the VRE bridge (not only "edited in place by a person and never generated", not only plain git), and the planned DaTEI adapter and `datei_ports` dependency are named
- [x] `docs/src/git-conventions.md` lists the `datei-*` crate scopes in its advisory vocabulary
- [x] Run `just check`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: DaTEI's ports, core, store and store-build crates under Bazel (dsp-repository)

**Gate: H3** — resolve before starting this phase.

The crates that need no web framework, with the Rebuild's test against a fake port (ADR-0008). Copied from dsp-incubator `main` at Phase 0's merge commit, pinned by SHA in the commit body, then split.

### dsp-repository

- [ ] `MODULE.bazel`: `crate.spec` for `lightningcss` (`1.0.0-alpha.71`) and `cel` (`=0.14.3`, with the exact-pin reason); the existing `unicode-normalization` spec is reused; rusqlite's features extended by `hooks` and `cache`; any further crate the engine names; `just repin`
- [ ] Copied files are registered with `git add -N areas/access/datei` before `just fmt` and `just check`, which only see tracked files (as does `check-datastar-attributes.sh`); the new gate script lists files the same way
- [ ] The new crates build for the musl platform before any DaTEI code is copied: `bazel build --platforms=//platforms:linux_x86_64_musl $(bazel query 'kind(alias, @crates//:*)')` (`docs/learnings/build-errors/crates-hub-aliases-are-manual-wildcard-builds-nothing.md`); if it fails, stop and report
- [ ] Placement table, before any code is copied: generated from the engine's and presenter's real `use` dependencies at the pin, assigning every module (or, for mixed modules, every function) to a crate under these rules, and committed in the commit body:
  `datei-ports` std only; `datei-core` no database crate, no Maud, no web framework, no `std::fs`; `datei-store` reads; `datei-store-build` writes; `datei-web` views (Maud), no database crate; `datei-server` routes, the presenter's SQL until DEV-7939, filesystem work. The table resolves each case below and any further one it finds; a module that fits no crate is split, not exempted
- [ ] Known case: `IrError` wraps `rusqlite::Error` and a deadpool variant (`ir/error.rs`) yet is used by core modules (`core/error.rs`, `ir/types.rs`, `ir/search_profile.rs`, `processing/css_root_url.rs`); split into a core error without `rusqlite` and a store error that wraps it
- [ ] Known case: `config/display_unit.rs` calls `ir::view_spec`, which runs SQL; split `view_spec` into types (core) and materialisation (store)
- [ ] Known case: `ir/search_strings` is mostly SQL (`mod.rs`, `walk.rs`, `windows.rs`); only its pure parts go to core
- [ ] Known case: the mixed read/write modules (`ir/db.rs`, `view_config`, `page_setup`, `view_files`, `view_edited_styles`, `driven_grid`, `ir/lifecycle.rs`) are split by function; `IrDb`'s connection and transaction accessors stay public for `datei-store-build` and `datei-server` (Rust has no friend crates; `datei-web` is kept from them by its BUILD dependencies)
- [ ] Known case: the viewer's `impl IrDb` blocks (`viewer/carried_query.rs`, `column_control.rs`, `render.rs`, `render_chunk.rs`) move into `datei-store` as free functions, or are left out where only the configurator calls them
- [ ] Known case: the pipeline calls `viewer::slot_render::rebuild_slot_relocation_membership_in_conn` (writes; to `datei-store-build`), which calls `apply_ref_filters` and `resolve_slot_root` (SQL reads the renderer also uses; to `datei-store`)
- [ ] Known case: the engine's `shortcode` module is replaced by `datei_ports::Shortcode`, so there is one shortcode type
- [ ] `areas/access/datei/ports`: `datei_ports` with the trait `DateiFiles` (`projects()`, `file(shortcode, name)`), the per-project trait `FileSource`, `Shortcode`, `StoredName` and `PortError` (std only, replacing the incubator's `thiserror` enum), and a comment naming the stored-name check's relation to `cpe_ports::is_file_name`
- [ ] `datei_ports`: a `testonly`, std-only fake keyed by stored name, implementing both traits, able to inject `NotFound`, `Unavailable` and a panic
- [ ] `datei_ports`: a contract check (as CPE's) that every adapter must pass: `projects()`, `file()`, `NotFound` versus `Unavailable`, a refused name, an unknown shortcode
- [ ] `areas/access/datei/core`: `datei_core` with the modules the table assigns, including `files::{LogicalPath, FileLayout, FileRole}`, the manifest parser, `ProjectFiles` and `ProjectFilesError` (with `NotUtf8`), reading through `datei_ports::FileSource`
- [ ] `datei_core`: a `testonly` builder that writes a project (layout, files, optional manifest) into the ports fake, replacing `MemoryFiles::insert`
- [ ] `areas/access/datei/store`: `datei_store` with `IrDb` (open, pragmas, schema, read accessors), the read-only pool and the read modules the table assigns, over a handle type for `datei-web`
- [ ] `areas/access/datei/store-build`: `datei_store_build` with the write functions over `IrDb`, the Rebuild, pipeline, loaders, config document, search and fields index builders and chunk label persistence
- [ ] `ReadModel` in `datei-store`: opens the read-only pool, checks out under a timeout, runs reads inside `interact` and maps an `interact` panic to an error
- [ ] `eng.yaml`: security reviewer on `areas/access/datei/store/**` and `areas/access/datei/server/**`, as for CPE
- [ ] `check-capability-deps.sh` bans database crates in the `web` role (test targets included), with a case in `check-capability-deps.test.sh`; for every capability if the existing crates pass, otherwise for DaTEI's
- [ ] A gate script, run by `just check`, refuses `std::fs`, `Path` filesystem calls and `Command` in `datei-ports`, `datei-core` and `datei-store-build`, and `Connection::open(` outside `IrDb` (`open_in_memory` in test code is allowed); its test rejects a fixture with each forbidden call and accepts one with `open_in_memory` in a test
- [ ] quick-xml: the engine parses deposited TEI with it, so the ignores of RUSTSEC-2026-0194 and RUSTSEC-2026-0195 in `justfile` and `osv-scanner.toml` ("nothing here reads XML with quick-xml") no longer hold; bump quick-xml past the fix, or rewrite both ignores with a reachability argument; the `serialize` feature the engine uses is added to the `crate.spec`
- [ ] The commit body lists every crate the move adds to `MODULE.bazel`; any that is not a small, established crate is raised in the pull request for an explicit OK, as `lightningcss` and `cel` were
- [ ] `include_str!`/`include_bytes!` resources (fonts, CSS, schema) are in `compile_data`, found with `git grep -nE 'include_(str|bytes)!|CARGO_MANIFEST_DIR'` over the copied crates
- [ ] The copied code passes the repository's clippy without blanket `#[allow]`s; production `unwrap()`/`expect()` the lints refuse are fixed, and any remaining allow is listed in the commit body
- [ ] `check-adr-refs` passes: incubator ADR numbers are qualified or removed
- [ ] The engine's tests move with their crates, with `testonly` test-support variants where several crates need them, and one `rust_test` per insta snapshot file
- [ ] Tests that read the incubator's `testcorpora` (edition inputs ADR-0009 keeps out of `areas/access/datei/`) are not moved; the commit body lists them, and they keep running in the incubator
- [ ] Test count: a script in the commit body counts tests per source file (`#[test]`, `rstest` cases, insta files) in the incubator at the pin and in the moved crates; the commit body records both numbers and lists every test not moved in this phase
- [ ] Test (ADR-0008): `datei-store-build` rebuilds the reproduction fixture from empty against the fake port twice; a dump of every table, excluding named volatile columns, is equal
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Remove the ADR-0009 and ADR-0003 `Not yet in code` lines this phase closes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: DaTEI's web and server crates, moved, with hidden projects honoured (dsp-repository)

A move: the presenter as a library, serving at a base, with its incubator activation (blocking, then serve) kept. Phase 4 changes activation. The only new behaviour is the hidden-project obligations, which `areas/access/CONTEXT.md` already requires of DaTEI.

### dsp-repository

- [ ] Golden crawl: at the Phase 2 pin, a recorded command serves the reproduction fixture with the incubator presenter at base `/datei` and writes every route's status, headers (`content-type`, `datastar-*`, `cache-control`, `x-content-type-options`) and body to a committed fixture; the command is in the commit body
- [ ] `areas/access/datei/web`: `datei_web` with the viewer and the presenter's page rendering, Maud only, no axum or tower
- [ ] The presenter's own SQL (`search_service.rs` with its query deadline, `serve.rs`, `serve/finder.rs`, `serve/index.rs`) stays in `datei-server`, calling `datei-store`'s `IrDb` accessors where it used `IrDb::*_in_conn`
- [ ] Phase 3's server reads prebuilt models from `DATEI_READ_MODEL_DIR/<shortcode>.sqlite`, the layout Phase 4 builds into; its tests build the models with `datei-store-build` into a temporary directory
- [ ] The 11 viewer files that take `&Connection` (`viewer/{carried_query,column_control,driven_printer,dt_printer,entity_links,footnotes,milestone_stamp,mod,render,render_chunk,slot_render}.rs`) call `datei-store` functions over its handle type; their raw SQL moves into `datei-store`
- [ ] `areas/access/datei/server`: `datei_server` with routes, configuration (`DATEI_BASE`, `DATEI_READ_MODEL_DIR`, the hidden set), search, index pages, finders and the facade `Datei`
- [ ] The routes `<base>/`, `<base>/style.css`, `<base>/datastar.js` and `<base>/fonts/{file}` are gone; `landing_view` is deleted
- [ ] `<base>/_static/{*path}` serves `style.css`, `datastar.js` and `fonts/{file}` from a fixed table, with a completeness test over the resources and `X-Content-Type-Options: nosniff`; fonts keep their immutable cache policy, the other two do not
- [ ] The font CSS prefix (`output/bundled_fonts.rs`), the page shell's stylesheet and script links and the reproduction list point at `<base>/_static`
- [ ] ADR-0007's seam rule 1 (no other CPE target grants a package outside the capability) is reworded to allow the shared vendored client
- [ ] The Datastar client is CPE's vendored file, shared through a filegroup in `areas/access/cpe/BUILD.bazel` (no new package under `cpe/public`, which would empty CPE's `public` glob) with visibility to `datei/server`; a test pins its SHA-256 (as the incubator's `tei_serve_boot.rs` does); `cpe/public/vendor/README.md` and `ARCH-MAP.md` record the shared file
- [ ] `datastar.rs`'s actions are built with `shared_datastar`; `shared_datastar` gains `.form()` with tests, including a URL with a newline that stays inside its string literal
- [ ] `shared/datastar/BUILD.bazel` grants `datei/web` and `datei/server`
- [ ] `@match` scrutinees in `html!` are bound with `@let`, and `html!` call arguments move into a `let` or helper, before `just fmt`
- [ ] Hidden projects: a layer on DaTEI's router adds `X-Robots-Tag: noindex, nofollow` to every response under a hidden project's prefix, comparing against upper-case shortcodes, as `cpe-server`'s `robots_for_hidden_projects`
- [ ] DaTEI's router registers its routes without the base and ends in a fallback answering DaTEI's 404; the hidden-project and 503 layers wrap the fallback as well
- [ ] Hidden projects: the page shell renders `<meta name="robots" content="noindex, nofollow">` for a hidden project
- [ ] `tei-serve` and `presenter.toml` remnants are not moved; the rule of `tei_serve_type_safety.rs` becomes a text test on `datei-server`: request handlers name only `ReadModel`, and `IrDb::open` appears only in the activation module
- [ ] A text test, as `no_pub_item_names_another_cpe_crate`, fails when a `pub` item of `datei-server` names another DaTEI crate
- [ ] The reproduction comparison (`serve/reproduction.rs`, `tei-compare`) moves as a `testonly` helper of `datei-server`
- [ ] The presenter's tests move (`tei_serve_boot.rs`, `presenter_base.rs`, `presenter_urls_from_project_root.rs` among them, including the `data-ignore` container check); the commit body lists moved, rewritten and dropped tests with counts
- [ ] Test: `datei-server` serving the fixture at `/datei` matches the golden crawl, except for the changes listed in the test: the removed routes, `_static` paths, `nosniff`, Datastar action markup, the robots header and meta tag for a hidden fixture project
- [ ] Test: a hidden project's pages, fragments and 404s carry `X-Robots-Tag` and the meta tag; a project not hidden carries neither
- [ ] Test: path traversal (`..%2f`, encoded and doubled) under `<base>/_static/` and `<base>/<shortcode>/…` is refused, and a deep `_static` path never reaches a project route
- [ ] Remove the ADR-0009 `Not yet in code` lines this phase closes
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: activation in the background, one project at a time, read models on disk (dsp-repository)

### dsp-repository

- [ ] `Datei::new(&DateiConfig)` does no I/O and cannot fail
- [ ] `DateiConfig` creates `DATEI_READ_MODEL_DIR` if missing and checks it by writing and removing a probe file; a refused or unwritable directory is logged and every DaTEI route answers 503
- [ ] `Datei::activate(&dyn DateiFiles, shutdown: Arc<AtomicBool>)` runs blocking, once per process (a second call is refused and logged), inside an outer `catch_unwind`
- [ ] Discovery: `projects()` `Ok` sets the project list; `Unavailable` is logged at error and every DaTEI route keeps answering 503
- [ ] Start-up cleanup deletes every top-level file matching DaTEI's exact name pattern, regular files only, without following symlinks or recursing
- [ ] Per project, in order: read `datei.toml` through the port and parse it as a `ConfigDoc`; refuse a file that is not UTF-8, does not parse or is not the current format; refuse a `meta.shortcode` that differs from the project's shortcode (a missing one is accepted, as the configurator leaves it optional)
- [ ] `datei-store`: an `IrDb` method finishes a model: checkpoint `(TRUNCATE)` asserting `(0, 0)`, `journal_mode=DELETE`, close; with a test
- [ ] `datei-store-build`: the Rebuild takes a cancellation check, called between steps and between files within a step
- [ ] `datei-store-build`: a test seam lets a test inject a write failure into a build
- [ ] Per project: open `ProjectFiles` through a wrapper binding the port to the shortcode, and build into `<shortcode>.building.sqlite` with `datei-server`'s port of `rebuild_in_place`, finishing with checkpoint `(0, 0)`, `journal_mode=DELETE`, close, sidecar check, `fsync`, rename, directory `fsync`
- [ ] Each project's build runs inside `catch_unwind`; on failure or panic the writer is dropped, the build files are deleted, the marker is removed, and then the project is marked failed
- [ ] `<shortcode>.rebuilding` is written before each build through a temporary file and a rename, holding an attempt count and a start time
- [ ] At start, a project whose marker shows two interrupted builds is marked failed and skipped, logging its name, the start time and how to clear it; with one, the count is raised and the build runs
- [ ] On shutdown, `activate` stops at the next cancellation check (between projects, between Rebuild steps, between files within a step), deletes the current build's files and its marker, and returns
- [ ] After a build, the project's published views, search, index pages and finders are loaded and swapped in with the model as one `Project`; a failure loading them fails only that project
- [ ] Project state: `Arc<RwLock<HashMap<Shortcode, ProjectState>>>` and a discovery flag, poison-tolerant, never locked across a build or `.await`
- [ ] A layer answers 503 with `Retry-After: 30` and `Cache-Control: no-store` for every DaTEI route before discovery and for a pending or failed project, on a copy of CPE's self-contained 503 page (`cpe-server`'s is crate-private); a shortcode not listed answers 404
- [ ] The read-only pool opens with `mode=ro`, `immutable=1`, `query_only`, a pool size and a checkout timeout; a missing model file is an error, never a new database
- [ ] Every read-model read runs inside the pool's `interact`; an `interact` panic is a logged 500
- [ ] Each project's build logs its start, its Rebuild steps and its end with `project_id` and `elapsed_ms`; a failed project logs an error naming it
- [ ] Test: a project whose build panics fails alone, leaving no marker and no build files, while another project serves
- [ ] Test: while one project builds, an already ready project serves
- [ ] Test: after a background build, the project's search and an index page answer
- [ ] Test: before discovery, every DaTEI route answers 503 with `Retry-After` and `no-store`; an unlisted shortcode then answers 404
- [ ] Test: after a successful build the directory holds exactly `<shortcode>.sqlite` for the project, and serving it creates no sidecar
- [ ] Test: a model file missing when the pool opens makes its project answer 503, not an empty page
- [ ] Test: planted model files, building files and sidecars are removed at start; a file outside the pattern and a subdirectory whose name matches the pattern are kept
- [ ] Test: one surviving marker leads to a build; two interrupted builds mark the project failed without building it
- [ ] Test: cancelling mid-build leaves no marker and no build files, and the next activation builds the project
- [ ] Test: an injected write failure leaves no files and no marker, and a failed project
- [ ] Test: a hidden project that is pending, and one that has failed, each answer 503 with `X-Robots-Tag`
- [ ] Test: a missing `DATEI_READ_MODEL_DIR` is created; one that cannot be created (a path under a regular file, which fails for root too) makes every DaTEI route answer 503
- [ ] Test: `projects()` `Unavailable` keeps every route at 503
- [ ] Test: a second call to `activate` is refused and logged
- [ ] Test: a panic inside `interact` answers a logged 500
- [ ] Test (table): a `datei.toml` that is `NotFound`, not UTF-8, unparseable, of an older format, or for another shortcode each fail the project alone
- [ ] Test: a listed project whose `file-layout.xml` is `NotFound` fails alone
- [ ] Remove the ADR-0009 `Not yet in code` lines this phase closes
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: `media` serves DaTEI's port; `access-server` mounts DaTEI at `/datei` (dsp-repository)

**Gate: H7** — resolve before starting this phase.

### dsp-repository

- [ ] `media-store` implements DaTEI's port beside `LiveDepositedFiles`: `projects()` lists the shortcode directories of `MEDIA_DATA_DIR` holding both the Ready token and `datei.toml`, skipping names that are not shortcodes and symlinks; a missing `MEDIA_DATA_DIR` lists none
- [ ] `media-store`'s `file()` reads `MEDIA_DATA_DIR/<shortcode>/<stored name>` with CPE's strict name check and symlink refusal; a missing file is `NotFound`, other I/O `Unavailable`
- [ ] `media/store/BUILD.bazel` names `datei_ports`; `media_store_for_tests` grants `//areas/access/datei/server`
- [ ] Test (`media-store`): the adapter passes `datei_ports`' contract check
- [ ] Test (`media-store`): `projects()` lists a directory with the Ready token and `datei.toml` and leaves out one missing either, a non-shortcode name and a symlinked directory; a missing `MEDIA_DATA_DIR` lists none
- [ ] Test (`media-store`): DaTEI's and CPE's name checks agree on a table of names both accept
- [ ] `media/CONTEXT.md` and `ARCH-MAP.md` (`areas/access/media`) say `media` serves DaTEI and lists its projects
- [ ] `access-server` constructs DaTEI's adapter over `MEDIA_DATA_DIR`, hands DaTEI the hidden set, and calls `Datei::activate` in `spawn_blocking` after the bind, once CPE's activation has finished, at once when CPE is not mounted, and also when CPE's activation panicked; `/healthz` does not wait for it
- [ ] `access-server`'s shutdown path sets DaTEI's cancellation flag before the graceful shutdown completes (the existing `shutdown_signal()` future is consumed by `with_graceful_shutdown`, so the flag is set from it)
- [ ] Test: DaTEI activates when CPE is not mounted, and when CPE's activation panics
- [ ] Test: a shutdown during a DaTEI build lets `access-server` exit and leaves no marker
- [ ] `access-server` nests DaTEI's router at `DATEI_BASE` (generalising `mount_cpe`, including its explicit `{base}/` route handling)
- [ ] `app()` and `assemble()` build CPE's and DaTEI's mounts separately, each inside its own `catch_unwind` on a clone of the router before it; a panicking DaTEI mount leaves DPE and CPE serving, with an error naming `DATEI_BASE`, and the other way round
- [ ] An empty `DATEI_BASE` leaves DaTEI unmounted; a base equal to or under `/dpe`, `/healthz`, `/telemetry`, `/ark:` or CPE's base is refused and logged
- [ ] Test: a `DATEI_BASE` that is empty, or equal to or under `/dpe`, `/healthz`, `/telemetry`, `/ark:` or CPE's base, leaves DaTEI unmounted with a log line
- [ ] Test: span names for DaTEI's routes are the route template (`GET /datei/{project}/{view}`); unmatched paths get the bare method, never a raw path
- [ ] The test that fails on a `"/cpe` literal in `access-server`'s `src/` (`no_source_outside_the_tests_writes_cpes_base`) also covers `"/datei`
- [ ] Test (`serve.rs`): with DaTEI mounted and its activation held, DPE and CPE answer as without DaTEI, and DaTEI's routes answer 503
- [ ] Test: a path under `/datei` that matches no project route answers DaTEI's 404, not DPE's
- [ ] Test (`serve.rs`): a hidden DaTEI project carries `X-Robots-Tag` through `access-server`, as the hidden CPE project does, on its page, on `/datei/<shortcode>/` and on an unmatched path under its prefix (`/datei/<shortcode>/a/b/c/d`), each answered by DaTEI
- [ ] `ARCH-MAP.md`: `areas/access/datei` loses `status: planned` and records its crates, kit, boundary rules and fingerprint (`/dune:dune-map`); `areas/access/server` records the new dependency
- [ ] Measure: the incubator's `just da-tei demo-bibles` at the Phase 2 pin, with its configuration exported by `da-tei backup` as `datei.toml`, is laid out in a scratch `MEDIA_DATA_DIR/086A/` as the bridge would (`files/*` flattened under stored names with a `file-manifest.json`, `file-layout.xml`, `datei.toml` and the Ready token beside them) and served by the recipe below; a release build's rebuild time, peak memory, peak WAL size, and the longest stretch between two cancellation checks (the finishing checkpoint, `fsync`s and rename included) are recorded, with the incubator's Keller baseline beside them
- [ ] `docs/src/datei/operations.md` (new, in `SUMMARY.md`): start-up, per-project 503, the two directories, the marker and how to clear it, the measured rebuild time, memory and WAL, what an operator reads in the logs, how a project gets DaTEI pages (its configuration arrives in `media`, then a restart), and how to hide it
- [ ] `docs/src/repo_structure.md` lists the crates
- [ ] A `just` recipe writes the reproduction fixture, with a manifest, flat stored names and `datei.toml`, with the Ready token into a scratch `MEDIA_DATA_DIR` and runs `access-server` against it with `DATEI_BASE=/datei` and a scratch `DATEI_READ_MODEL_DIR`
- [ ] `areas/access/CONTEXT.md`: DaTEI is in code; its remaining `Not yet in code` statements are removed
- [ ] Remove the ADR-0009 and ADR-0003 `Not yet in code` lines this phase closes, and update ADR-0009's `Enforced by` line
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] Run `eng:test-browser` on `/datei/<shortcode>` of the fixture served by that recipe; no defect reported
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: image, CI and deployment wiring (dsp-repository, ops-deploy)

**Gate: H2** — resolve before starting this phase.

### dsp-repository

- [ ] `areas/access/server/BUILD.bazel` image `env`: `DATEI_BASE=/datei` and `DATEI_READ_MODEL_DIR=/tmp/datei`, a path the nonroot user can write, so the image and Cloud Run previews run without an override; the image ships no DaTEI data and no read model
- [ ] `docs/src/dpe/operations.md` environment table lists both variables
- [ ] Workflow path filters include `areas/access/datei/**`: `a11y-dpe.yml` (push and pull request), `cloud-run-dpe-pull-request.yml`, `scout-dpe.yml`
- [ ] `docs/src/deployment.md` says so, and that previews hold no DaTEI project (their `/tmp` is memory)
- [ ] Diff dsp-incubator `main` against the Phase 2 pin over the moved engine and presenter paths; each change since is ported in this pull request or recorded as a Linear issue in P-DEV-396
- [ ] Remove the remaining ADR-0009 `Not yet in code` lines this phase closes
- [ ] `just image-smoke` also polls `/datei/0000` until its 503 turns into 404, proving DaTEI's discovery ran (no projects in the image)
- [ ] Run `just image-smoke`; it passes
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes

### ops-deploy

- [ ] `group_vars/repository_{stage,prod}.yml`: the read-model host path from H2; dev gets no read-model mount (no bridge feeds dev, the image's `/tmp/datei` applies)
- [ ] `repository.yml`: on stage and prod, a writable `deploy_mounts` entry for the read-model directory at that host path, `type: dir`, `uid`/`gid` 65532 (distroless nonroot), as `deposit.yml` mounts its database directory
- [ ] `repository.yml`: on stage and prod, `DATEI_READ_MODEL_DIR` points at the mounted container path
- [ ] `repository.yml`: on stage and prod, `deploy_memory` raised from `1G` to the size H2 confirms (at least `3G`)
- [ ] `repository.yml`: `ACCESS_HIDDEN_PROJECTS` stays as ops-deploy#1485 set it (default `0803,083C,086A`); no change
- [ ] `repository.yml`: a stop grace period above Phase 5's longest uninterruptible stretch, scaled to Keller's size with a margin, if Docker's 10 s default does not cover it; the `deploy` role's compose template (a Swarm stack) gains `stop_grace_period` if it lacks it
- [ ] `repository.yml`: the `SYNC_DATA_DIR` comment no longer says a Dockerfile copies the data (the image is built with Bazel)
- [ ] Run `just lint`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Done 2026-11-19: the bridge session accepted the contract below unchanged and added that a project's files do not change while a reader reads them (mechanism: H7). Confirm with the VRE bridge plan (P-DEV-301): the formats of `file-manifest.json` and the file mapping as written in *Proposed Solution* (keys are logical paths as `file-layout.xml` writes them), the Ready token's name, stored names within `media`'s rule (segments of `[A-Za-z0-9._-]`, no leading dot), and `datei.toml` (a `ConfigDoc`), `file-layout.xml` and the mapping shipped in the bridge's image | Balduin | before start | The pipeline is planned in another session; the contract is his decision |
| H2 | Get from Samuel, for stage and prod: where the repository VMs' data storage from DEV-7513 is mounted, and a writable read-model subdirectory on it (sized from Phase 5's model and WAL peak, about 10 GB to start; more when DEV-7938 keeps two models), and whether the VM has the memory for `deploy_memory` of at least 3G beside its other stacks. The same message asks how the bridge (deposit VM) reaches storage that `media` (repository VM) reads; that answer belongs to DEV-7514 and P-DEV-301 | Balduin, Samuel | before Phase 6 (Why mid-plan: Phases 0 to 5 do not need the answer, and waiting for Infra must not delay their start) | Infrastructure is Samuel's |
| H3 | Dominique reviews and merges the dsp-incubator pull request (Phase 0) on `main` | Balduin, Dominique | before Phase 2 (Why mid-plan: Phase 2 copies the engine from incubator `main` at Phase 0's merge commit, and that pull request only exists after Phase 0 has run) | Merging is the owner's decision |
| H4 | Merge the stack in order (`gh stack merge --rebase`), then the ops-deploy pull request | Balduin | after ship | Merging is the owner's decision |
| H5 | Cut a dsp-repository release, bump `dpe` in ops-deploy's `versions/repository-stage.json`, and deploy to stage (deploys are locked by Samuel until the dsp-api/ingest merge is released) | Balduin, Samuel | after ship | Release and deploy approval |
| H6 | Once DEV-7514's data mount exists on stage with `MEDIA_DATA_DIR` pointed at it (the bridge's 0803 included), a deploy carrying ops-deploy#1485's `ACCESS_HIDDEN_PROJECTS` has reached stage, Keller's file mapping is made (from DEV-7921's upload), and Keller is put on stage's bridge allowlist and synced (P-DEV-301, which needs DEV-7936): check that Keller is served at `/datei/083C` with `X-Robots-Tag`; record its rebuild time, memory and WAL peak in `docs/src/datei/operations.md` in a follow-up pull request; tell Dominique the polish can start | Balduin | after ship | Depends on work outside this plan |
| H7 | Learn from the VRE bridge session how it keeps a project's files from changing during a read (for example a symlink to the current snapshot, a directory rename, or files written beside and renamed); if it uses a symlink or replaces the project directory, Phase 5's adapter resolves the project directory once per activation instead of refusing symlinked directories and reopening the path per file | Balduin | before Phase 5 (Why mid-plan: Phases 0 to 4 do not read the shared folder, and the bridge's design is not final) | The bridge is planned in another session |

## Acceptance Criteria

- [ ] DaTEI's port in dsp-repository lists projects by their DaTEI configuration and serves files by shortcode and stored name, with no lookup by hash
- [ ] A project delivered with flat stored names and a manifest renders the same pages as the same project under its logical paths
- [ ] A file whose hash no longer matches the layout loads with a warning naming it
- [ ] A project gets DaTEI pages when its configuration appears in `media`, with no code or deployment change, after a restart
- [ ] A hidden project's responses carry `X-Robots-Tag` and its pages the robots meta tag; no overview lists it
- [ ] DaTEI's crates follow ADR-0003's roles and ADR-0009's edges; `datei-core` and `datei-web` name no `rusqlite`, enforced by `check-capability-deps.sh`; `just check` passes
- [ ] `access-server` reports healthy and serves DPE and CPE while DaTEI's projects build; a DaTEI project answers 503 with `Retry-After` until ready; a failing project fails alone; a routine restart during a build does not disable a project
- [ ] No half-built read model is ever served, and no file outside DaTEI's name pattern is deleted
- [ ] `datei-server` serves the fixture as the incubator presenter did, apart from the listed changes
- [ ] ADR-0003, ADR-0009 and ADR-0012 describe what is in code; ARCH-MAP and the operations docs match
- [ ] The image runs DaTEI at `/datei`; the stage container has a writable host directory, the memory for one rebuild and Keller in `ACCESS_HIDDEN_PROJECTS`
- [ ] `just check` and `just test` pass on every pull request of the stack; `just da-tei ci` passes in dsp-incubator

## Dependencies & Risks

- **The VRE bridge (P-DEV-301)** writes the project's files, the manifest, `datei.toml`, `file-layout.xml` and the Ready token. Until it runs on stage, Keller has no data there; this plan's work is testable with fixtures, not with Keller. Its contract is H1. Its first increment is 0803 for CPE; Keller is its first real user, and its project export needs a service identity in DSP-API (DEV-7936).
- **Keller's file mapping** is made by hand once its files are in the VRE (DEV-7921): one IRI per logical path.
- **DEV-7514** (the shared data mount) must exist on stage, with `MEDIA_DATA_DIR` pointed at it and Incunabula's data in it, before Keller can arrive; today `MEDIA_DATA_DIR` is baked into the image (`/app/media/data`). The bridge runs in `deposit-server` on the deposit VMs (`dasch-deposit-{stage,prod}-01`), `media` on the repository VMs, so the folder needs storage shared between two VMs or the bridge placed on the repository VM; this goes to Samuel with H2 and is decided in DEV-7514 and P-DEV-301.
- **DEV-7937** (incubator vendoring) is needed before the configurator builds against the moved crates; until then, engine fixes made during the polish land in dsp-repository first and are copied to the incubator by hand.
- **Stage deploys are locked** by Samuel until the dsp-api/ingest merge is released; this can delay H5 past 2026-12-04.
- **CPE** finds its projects by committed folders; DEV-7948 moves it to the same discovery.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `lightningcss` or `cel` does not build under Bazel's musl transition | M | H | First deliverable of Phase 2, before any copy; fall back to DEV-7940 / DEV-7945 for that crate |
| The mixed read/write `ir` modules resist a clean store / store-build split | M | M | Split by function, not module; visibility makes a wrong edge a build error |
| Store functions leak `rusqlite` types into `datei-web` | M | M | A handle type in `datei-store`; the `check-capability-deps.sh` ban on `web` makes a leak a build failure |
| A rebuild exceeds the container's memory and kills DPE and CPE | M | H | One project at a time, after CPE's activation; `deploy_memory` raised; the marker ends a crash loop after two kills |
| The WAL or model fills the disk during a build | M | M | Old models deleted at start; a failed build cleans up; H2 sized from the Phase 5 measurement |
| A slow DaTEI query stalls DPE and CPE | L | H | All reads inside the pool's `interact`; pool checkout timeout |
| The move changes rendered output silently | M | M | Golden crawl from the pinned incubator commit, with headers; Phase 3 is a move apart from the listed changes |
| The mapping or manifest does not match the layout on real data | M | H | Keys are the layout's logical paths, compared exactly; a missing or duplicate key fails the project naming it; the bridge refuses an IRI it cannot resolve |
| The stack outgrows 2026-12-04 | M | H | Phases 0 and 1 are small; Phases 2 and 3 are the bulk; Phases 5 and 6 are thin |

## Success Metrics

- Keller (083C) is served, hidden, at `/datei/083C` on stage from dsp-repository before 2026-12-15, so its polish happens there.
- Baseline from the incubator: a Keller rebuild takes about 4 min 31 s, peaks at 1 to 1.9 GiB, and produces a 2.2 GB database. The Phase 5 measurement on the Bibles corpus and H6's on Keller stay within the same order.
- `just check`, `just test`, `just image-smoke` and `just da-tei ci` are green on every pull request.

## References

- Decisions: DEV-7934 (*Decided 2026-11-19*); project P-DEV-396; issues DEV-7549, DEV-7574, DEV-7935; DEV-7616 (dsp-repository#606, merged); DEV-7919 (hidden projects, merged)
- Out of scope: DEV-7938 (keep and swap read models), DEV-7939 (views over plain structs), DEV-7940 (`cssparser`), DEV-7945 (drop `cel`), DEV-7937 (vendoring), DEV-7948 (CPE discovery), DEV-7514 (shared data mount), P-DEV-301 (VRE bridge)
- ADRs: `docs/adr/0003-one-modulith-per-area.md`, `0004-*`, `0007-cpe-joins-the-access-area-as-its-second-capability.md`, `0008-a-reading-capability-may-keep-a-derived-read-model.md`, `0009-datei-joins-the-access-area-as-its-third-capability.md`, `0012-build-time-inputs-come-through-two-channels-one-consumer-each.md`, `0013-authorisation-is-decided-centrally-and-enforced-locally.md`; `docs/adr/CLAUDE.md`
- Prior plan: `docs/specs/2026-10-21-media-minimum/01-feat-media-minimum-plan.md` (no list operation on the port)
- CPE as template: `areas/access/cpe/server/src/lib.rs`, `activation.rs`, `unavailable.rs`, `request.rs` (`robots_for_hidden_projects`), `static_files.rs`; `areas/access/server/src/serve.rs` (`app`, `assemble`, `mount_cpe`, `no_source_outside_the_tests_writes_cpes_base`); `areas/access/media/store/src/lib.rs`; `areas/access/cpe/ports/src/names.rs`; `areas/access/CONTEXT.md` (*Hidden project*)
- DaTEI today: `dsp-incubator/da-tei/crates/datei-engine/src/files/`, `rebuild.rs`, `viewer/`, `ir/readonly.rs`, `ir/schema.rs`, `output/bundled_fonts.rs`; `crates/datei-presenter/src/serve.rs`, `datastar.rs`, `search.rs`; `crates/da-tei/src/files/directory.rs`, `rebuild.rs`, `replace.rs`; `docs/architecture/file-port.md`, `file-layout.md`, `rebuild.md`, `reproduction.md`
- Earlier moves: `docs/specs/2026-10-11-move-cpe-store-web-server/` (journal: deferrals and side findings), `docs/specs/2026-11-17-bazel-migration/`
- Learnings: `docs/learnings/build-errors/crates-hub-aliases-are-manual-wildcard-builds-nothing.md`, `remote-download-minimal-leaves-toplevel-outputs-remote-in-ci.md`, `publish-build-trusts-nothing-a-pull-request-can-write.md`; `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`; `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`; `dsp-incubator/cpe/docs/learnings/best-practices/proving-a-move-only-refactor-byte-identical-takes-more-than-snapshots.md`
- External: SQLite WAL (`https://sqlite.org/wal.html`), temporary files (`https://sqlite.org/tempfiles.html`), URI parameters (`https://sqlite.org/uri.html`)
- Operations: `docs/src/bazel.md`, `docs/src/cpe/operations.md`, `docs/src/deployment.md`; `ops-deploy/repository.yml`, `ops-deploy/deposit.yml`
