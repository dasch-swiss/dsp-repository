---
title: "feat: media at its minimum, and CPE reads its deposited files through a bytes port"
type: feat
date: 2026-10-21
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7512
linear_project: CPE establish production path
---

# feat: media at its minimum, and CPE reads its deposited files through a bytes port

## Overview

ADR-0012 (PR #494) decides that a project's configuration, curation and narrative are deposited files, that they
reach a reader as bytes through a port against `media`, and that a CPE project folder holds code, not data. This
plan builds the minimum of that decision for CPE and Incunabula, in one PR covering two tickets:

- **DEV-7511**: `cpe-ports` gains a bytes port, CPE reads its KDL, narrative and curation through it, the curation
  reader moves to CPE's side, and the code that followed the superseded ADR-0010 (`ProjectSnapshot.curation`, the
  contract's curation rules, `sync-store`'s curation reader and committed curation file) is retired.
- **DEV-7512**: `areas/access/media/` arrives with a `media-store` crate whose `LiveDepositedFiles` implements
  that port over `MEDIA_DATA_DIR`, a hand-fed stand-in for the Access bucket. Incunabula's files move into it,
  with a `PROVENANCE`, and `access-server` and the image ship it as they ship `sync`'s data.

Decided with the developer (2026-10-21):

- One PR for both tickets.
- A CPE project folder declares its shortcode in a one-line `SHORTCODE` file.
- DaTEI's port and adapter wait until DaTEI has code.
- `assets/` stay in CPE's project folder until `media` has request-time routes.
- `Cpe::from_sql_folders` (incubator host and tests only, removable at DEV-7498) keeps reading `config.kdl` and
  `narrative/` from its folders; only activation from the ports changes.
- The store header is corrected to name both ports, and the pinned store hash is re-pinned, with a separate pin
  proving the text below the header is unchanged.
- An absent `curation.csv` means no curation, unless the remodel block reads a curation key; then the project is
  not ready.

The PR lands as one commit (root `CLAUDE.md`); the phases below are reviewed one by one and squashed in the last.

## Problem Statement / Motivation

Today CPE reads `config.kdl` and `narrative/` from `areas/access/cpe/projects/incunabula/`, and the curation
travels inside `sync`'s snapshot (`ProjectSnapshot.curation`, filled from `areas/access/sync/data/0803-curation.csv`).
That contradicts ADR-0012 in three ways: `sync` serves a non-fact; a project folder holds data; and there is no
place where the archive's second channel, the Access bucket, enters the Access Area. DEV-7514 (ops-deploy mounts)
and DEV-7513 (VM storage) wait on `MEDIA_DATA_DIR` existing.

## Proposed Solution

### The bytes port (`cpe-ports`)

A second trait beside `ArchiveProjection`, std-only like the rest of the crate:

```rust
/// What CPE reads from the Access Area's `media`: the files a project deposited, by name, whole.
/// It gives no file a meaning and announces no change; CPE calls it inside `spawn_blocking`.
pub trait DepositedFiles: Send + Sync {
    /// The file's bytes, whole. A read that fails midway is `Unavailable`, never a prefix.
    fn file(&self, shortcode: &str, name: &str) -> Result<Vec<u8>, FilesError>;
}

pub enum FilesError {
    /// `media` holds no project under this shortcode.
    UnknownProject { shortcode: String },
    /// The project holds no file under this name.
    UnknownFile { shortcode: String, name: String },
    /// The file could not be read; the source is the adapter's own error.
    Unavailable(Box<dyn Error + Send + Sync>),
}
```

- **No list operation.** ADR-0012 says the port carries "files by shortcode and name … and nothing else". CPE
  learns which narrative files to ask for from its KDL (below), so it never needs a listing.
- **Shortcodes.** `pub fn is_shortcode(s: &str) -> bool`: four hex digits, upper case as DSP writes them. Used by
  discovery and by both adapters. `cpe-core` keeps its own rule for the KDL's `remodel { shortcode }`
  (`validate/remodel.rs:142`), since it may not depend on `cpe-ports` (seam rule 3); the comparison of the two is
  plain string equality.
- **Names.** `pub fn is_file_name(name: &str) -> bool`: one or more `/`-separated segments; each segment non-empty,
  of `[A-Za-z0-9._-]`, and not starting with `.`. Both adapters answer `UnknownFile` for any other name before
  touching storage, so the fake and the live adapter agree and no caller string leaves the project's subtree.
  `cpe-core`'s narrative-name rule (`validate.rs:4794-4806`) is looser; a KDL name the port refuses surfaces as
  `NarrativeFileNotFound` naming that file, and `cpe/CONTEXT.md` says so.
- **`FilesError`** is written by hand like `ProjectionError` (`Debug` derived, `Display` including an `Unavailable`
  source's message, `Error::source`, a `#[must_use]` `unavailable(..)` constructor): `cpe-ports` is std-only, so
  no `thiserror`.
- **`&str`, not newtypes.** The trait takes `&str` as `ArchiveProjection::snapshot` does; a validated `Shortcode`
  type would have to change that port too and is left for later. Both adapters validate, since each is a trust
  boundary. `is_shortcode` and `is_file_name` live in a small `names.rs` module re-exported from `lib.rs`.
- **`FakeDepositedFiles`** in `cpe-ports/src/fake.rs`, builder-style like `FakeArchiveProjection` (`#[must_use]`
  builders, last registration wins): `with_file(shortcode, name, bytes)`, `with_unavailable(shortcode)`,
  `with_unavailable_file(shortcode, name)`, and `requests()` returning the `(shortcode, name)` pairs asked for, in
  order, recorded behind a `Mutex` because `file` takes `&self`.
- Names are the trait's and the error's working names; the executor may pick better ones, but uses one name per
  concept across code and `cpe/CONTEXT.md` (**Bytes port** stays the vocabulary term).

### `media` at its minimum (`areas/access/media/`)

```
areas/access/media/
  CONTEXT.md
  store/            # crate `media-store`, depends on `cpe-ports`
  data/
    PROVENANCE
    0803/
      config.kdl
      curation.csv
      narrative/<name>.<lang>.md
```

- `media_store::LiveDepositedFiles::new(dir)` reads `<dir>/<shortcode>/<name>` on every call.
- **Shortcode.** Accepted only if `is_shortcode` holds and `<dir>/<shortcode>` is a directory; otherwise
  `UnknownProject`. Unlike `sync`, there is no `KNOWN` list: the directory is the list, as the bucket's prefixes
  will be.
- **No symlinks, per component.** `fs::symlink_metadata` is checked on `<dir>/<shortcode>` and on every component
  of the name below it: the project directory and each intermediate directory must be real directories, and no
  component may be a symlink. A symlinked or non-directory project is `UnknownProject`; a symlinked component of the
  name is `UnknownFile`. Checking only the last component would let `narrative -> /etc` escape `<dir>`. Errors are
  matched on their kind, not collapsed with `is_ok_and` as `discovery.rs` does: `NotFound` and `NotADirectory`
  are `UnknownProject`/`UnknownFile`, anything else (`PermissionDenied` included) is `Unavailable`. The window
  between the check and the read is accepted, since only an operator writes `MEDIA_DATA_DIR`, and the doc comment
  says so; `openat2(RESOLVE_NO_SYMLINKS)` or `cap-std` is the route when the real bucket's adapter replaces this one.
- **Whole or refused.** `fs::read` into memory; text only, so no size cap yet. A directory under the file's name
  (`IsADirectory`) is `Unavailable`. No `From<io::Error>` for `FilesError`, so `?` cannot turn a not-found into
  `Unavailable`. Error messages carry the shortcode and name, never `<dir>`'s absolute path; an `Unavailable` source
  may, since it is logged, not served.
- **Anonymous consumer.** The stand-in holds only public files, so the adapter serves all it holds; its doc
  comment and `media/CONTEXT.md` say this is where a rights check goes when rights exist.

### CPE reads through the port

- **Discovery** (`cpe/server/src/discovery.rs`). Activation from the ports discovers a project as a folder with a
  regular, non-symlinked `SHORTCODE` file, replacing the `config.kdl` test at `:197`. The file holds the shortcode,
  trimmed of surrounding whitespace, and must satisfy `is_shortcode`; a malformed one is skipped with a warning, as
  a folder whose name fails the id rule is today. Folders that share a shortcode are well-formed projects:
  discovery returns each with a refusal naming the other folder, and `load_all` turns that refusal into not ready
  (503) without fetching anything, never a silent 404.
  `DiscoveredProject` gains `shortcode: Option<String>`, `None` on the legacy `config.kdl` path. Discovery returns
  what it skipped and why, so tests assert on values; the repository has no log capture.
- **`from_sql_folders` keeps its folders.** It and its test helpers (`sources()`, `narrative_files()`,
  `narrative_contents()`) keep discovering by `config.kdl` and reading `narrative/` from the folder until DEV-7498
  removes them. Discovery therefore has two predicates, each named for its caller, and the module doc says which
  is legacy. The `cpe/server/tests/fixtures/discovery/*` fixtures that serve `from_sql_folders` keep `config.kdl`.
- **Activation** (`cpe/server/src/activation.rs`), per project:
  1. Fetch `config.kdl` through the bytes port; decode it as UTF-8; parse it.
  2. Refuse the project if the KDL's `remodel { shortcode }` differs from `SHORTCODE`.
  3. Collect the narrative names the KDL names, from the raw parse (`crate::config::parse`, as `load_remodel` does
     at `validate.rs:220-221`) so names are found even when lowering fails. They sit in two fixed places: the
     page-body `narrative` (`validate.rs:4601`, lowered in `lower_narrative` at `:4706-4725`) and
     `resource-detail > narrative-for` (`:6364-6373`). Fetch each base name and its `variant_filename(file, lang)`
     for every site language; `UnknownFile` means absent. The accessor returns the base names only; activation takes
     the site languages from the same raw parse (`declared_languages(&raw)`, unverified — confirm during
     implementation). A name containing `/` or `\` is not fetched: validation
     refuses it as `InvalidNarrativePath` (`:4794-4806`). Each fetched file is decoded as UTF-8. For Incunabula
     this is 8 base requests, all `UnknownFile`, and 16 variant requests.
  4. Fetch `curation.csv`. `UnknownFile` means no curation, unless `Remodel::reads_curation()` holds, in which
     case the project is not ready. The predicate is new in `cpe-core` (`remodel.rs`) and covers all four ways a
     remodel reads curation: a class's `curated` nodes and `curated-key=`/`display-key=`/`cover curated-key=`
     attributes (`RemodelClass::curated_keys()`, `remodel.rs:208-247`), a hook's keys (`RemodelHook.keys`,
     `:522`), and annotations, whose `keep` key is required (`Remodel.annotations`, `:109`).
  5. `projection.snapshot(shortcode)`, then parse the curation against the snapshot (`CuratedValues::parse`).
  6. `cpe_store_build::build(&snapshot, &curation, …)`, then `Project::load` as today. `build` takes
     `&CuratedValues`, since `Curation<'a>` borrows from it as it borrows from the snapshot today.
  - Validation is told the narrative inventory is **complete** whenever it came from the KDL, so an empty inventory
    still checks existence; today an empty slice skips the check (`validate.rs:4809-4811`). `from_sql_folders` keeps
    the old behaviour. `ValidationContext` (`cpe-core/src/validate/context.rs:10-33`) is not `#[non_exhaustive]`, so
    the new field reaches every struct literal: `cpe-core` `validate.rs` (6950, 6962, 6978, 6994, 7010, 8432, 8536,
    9993), `cpe-web` `tests/validation.rs` (62, 131, 498), `cpe-server` `activation.rs:161` and `router.rs` (2712,
    3527, 3630, 3652, 3675, 3699, 5066, 5160, 5423, 5572). `for_tests` (`context.rs:43-50`) defaults to "not
    complete". `load` (`activation.rs:152-184`) is shared by both paths, so the flag is a parameter of `load`.
  - A port error other than `UnknownFile`, a non-UTF-8 file, a shortcode mismatch and a validation failure each
    leave the project not ready (503), as a missing `SYNC_DATA_DIR` does today; "not ready is not not-there" holds.
  - Diagnostics name the file by its media location (`0803/config.kdl`), not `{id}/config.kdl` (`activation.rs:131,171`).
- **`Cpe::activate`** takes both ports: `activate(&self, projection: &dyn ArchiveProjection, files: &dyn
  DepositedFiles)`. Its signature names only `cpe-ports` types, which `no_pub_item_names_another_cpe_crate`
  allows.

### Curation moves to CPE's side

- The curation parser (`sync/store/src/curation.rs`, which already refuses dangling resources, duplicate rows and
  control characters) is copied into `cpe-store-build` in Phase 3 behind `CuratedValues`, a tuple struct with a
  private field and no `From<Vec<_>>`:
  - `CuratedValues::parse(bytes, &ProjectSnapshot) -> Result<Self, CurationFault>`, pure: no I/O.
  - `CuratedValues::none()` for a project without a curation file.
  - `len()` and `as_slice()` for logging and tests.
  - `#[doc(hidden)] pub fn from_unchecked(Vec<CuratedValue>)` for `tests/it`, which is a separate crate. About 159
    call sites across nine files build curation there, and some deliberately use values the parser refuses
    (`strips.rs:103` pads, `refusals.rs:1010` pads and adds a control character) to test the hook API's
    `curated_served` "whitespace and all" (`src/hook.rs:405-413`).
  - `build` takes `CuratedValues` instead of reading `snapshot.curation`; production code reaches it only through
    `parse` or `none`. `refuse_interior_nuls` (`lib.rs:236-238`) and `Curation::read` (`generic/curation.rs:38-47`)
    switch to it, and `build`'s doc (`lib.rs:37-53`, "from `snapshot`, from `mapping` and from nothing else") is
    rewritten.
  - In Phase 3 the integration tests keep their curation as follows, so the 159 call sites stay as they are and no
    test silently loses its curation. `main.rs` gains `static CURATION: LazyLock<CuratedValues>`, the media CSV
    parsed against `SNAPSHOT`, and `STORE` builds with it. The `build(snapshot)` wrapper (`main.rs:50-52`), which
    `support.rs`, `refusals.rs`, `annotations.rs`, `hook_rules.rs` and `committed_0803.rs` use for snapshots derived
    with `with_curation`, passes `from_unchecked(snapshot.curation.clone())`. `hooks.rs` and `hookless.rs` import
    `cpe_store_build::build` directly (`hooks.rs:16`, `hookless.rs:20`); they switch to a `support.rs` helper
    `build_with(snapshot, mapping, hook)` that does the same. The contract still checks `snapshot.curation`
    during Phase 3; that is redundant with the parser, not contradictory, since `sync` and `media` hold the same
    CSV until Phase 4.
  - The copied parser and its tests must pass `tests/it/hygiene.rs` (`BINARY_ONLY` at `:17-27`, `PROJECT_NOUNS`
    at `:276`); the `sync` originals do.
- In Phase 4, `CuratedValue` and `is_curation_name` move from `cpe-ports` into `cpe-store-build`, and
  `ProjectSnapshot.curation`, the contract's `DanglingCuration`/`DuplicateCuration`/`MalformedCuration`, `sync-store`'s
  reader and errors, and `sync/data/0803-curation.csv` are deleted. The test helpers' curated values move off the
  snapshot into a fixture type that `support.rs` turns into `from_unchecked`. Splitting it this way keeps every
  phase compiling: the type cannot leave `cpe-ports` while the snapshot still carries it.
- The order-independence tests (`hookless.rs:1326-1339`, `committed_0803.rs:578-594`) reverse
  `snapshot.curation`. In Phase 3 the reversal still reaches the build through `from_unchecked`; in Phase 4 they
  reverse the fixture values instead.
- The store's header lines (`store-build/src/sql.rs:82-83`) name both ports; everything below them is unchanged.
  `sql::write` emits the header, a blank line, then the tables (`sql.rs:110-112`), and every header line is
  `"-- "` plus escaped text (`sql.rs:91-106`), so the header ends at the first `"\n\n"`. The baseline pin splits
  there: `let (header, body) = sql.split_once("\n\n")`, asserts every header line starts with `"-- "`, and pins
  `body.len()` and the SHA-256 of `body`.

### Prior decisions this touches

- **The store text is byte for byte the prototype's (ADR-0007, DEV-7465 amendment).** ADR-0007 renders SQL text
  because "in exchange the text for Incunabula is byte for byte the file the prototype embeds, which a test holds",
  and the store-web-server plan (decision 7) calls the header strings "Not to be edited … a change makes the
  rendered SQL differ from the prototype's `data.sql`". What changed: ADR-0012 makes the header's second line false
  ("Not from the port: the remodel block of projects/{project}/config.kdl"), and a pinned falsehood is worse than a
  re-pin. The equality the ADR wants still holds where it matters: the body is pinned unchanged, and the incubator's
  freshness test regenerates its `data.sql` with the vendored builder, so the two agree again on its next re-vendor.
  ADR-0007 gains a dated amendment saying so (Phase 6).
- **`from_sql_folders` keeps reading folders.** ADR-0008's amendment says a deposited file never comes "from a file
  beside the code"; the store-web-server plan's decision 2 already carves out the SQL-text entry as "outside the
  production path … until DEV-7498 removes it. Production activation goes through the port only." This plan keeps
  that carve-out and nothing more: production activation reads only the ports.
- **`assets/` stay in the folder.** ADR-0012 narrows the folder to code and the documents describing the data, and
  sends assets through `media` at request time. Until `media` has those routes (out of scope), CPE serves
  `assets/` from the folder as today; `cpe/CONTEXT.md` names this as the interim and its end.
- **An absent curation file.** The cpe-port-curation plan made an absent file `Unavailable` so that "no test can
  pass on a file that is not there". That followed the superseded ADR-0010, but the reason stands, which is why an
  absent `curation.csv` is "no curation" only when the remodel block reads no curation key.

### Wiring and image

- `access-server/src/serve.rs`: `DEFAULT_MEDIA_DATA_DIR = "areas/access/media/data"`, `MEDIA_DATA_DIR` read beside
  `SYNC_DATA_DIR` through one pure `data_dir(value: Option<String>, default: &str) -> String` that treats an empty
  value as unset for both (learning `empty-composite-input-beats-compose-env-file`; `SYNC_DATA_DIR` does not today,
  `serve.rs:209`), a warning when the directory is missing (not fatal), and `LiveDepositedFiles` handed to
  `activate_in_background`. `serve()` reads the environment and is not unit-tested, so `data_dir` is.
- Image: `staging/media` in `.github/actions/build-dpe/action.yml`, `COPY media/data /app/media/data` and
  `ENV MEDIA_DATA_DIR="/app/media/data"` in `areas/access/server/Dockerfile`, `just build-docker-dpe`, and
  `docs/src/dpe/operations.md`. PR previews build through `build-dpe` (`cloud-run-dpe-pull-request.yml:45`), so they
  carry the data once their path filter lets a media change trigger them.

## Alternative Approaches Considered

- **Two stacked PRs (media first with copies of the files, then the CPE switch).** Rejected by the developer in
  favour of one PR; the copies would have existed only between merges.
- **A list operation on the port.** Would let CPE glob `narrative/` as today, but ADR-0012 limits the port to files
  by name, and a listing is what makes a port drift toward a file system. The KDL already names every narrative;
  for Incunabula it names all eight bases of the sixteen files, so the inventory is the same.
- **Shortcode from the folder name or from the hook.** A folder named `0803` drops `incunabula` as the URL segment
  and hook key; a constant in code moves identity into compiled code. A `SHORTCODE` file keeps both (decided).
- **The curation parser in `cpe-server`.** `cpe-store-build` already owns `Curation` and the key semantics, and its
  seam rule admits exactly the crates the parser needs.
- **Moving `from_sql_folders` and its fixtures onto `FakeDepositedFiles`.** One model everywhere, but a large test
  rewrite and a change to the incubator's host for code DEV-7498 deletes (decided against).

## Technical Considerations

- **Architecture.** `media-store` → `cpe-ports` is the store-implements-consumer-port shape `sync-store` already has;
  `check-cpe-seam.sh` rule 1 admits `cpe-ports`. (`check-capability-deps.sh` would allow it too, but checks only
  the Deposit Area today.) `check-cpe-seam.sh` gains the `media` edge: no CPE manifest names a
  `areas/access/media/*` crate outside dev-dependencies, nor renames one in the workspace root (ADR-0012 *Enforced
  by*). `check-composition-root-deps.sh` already admits a `store` crate.
- **Security.** The adapter is the first code that builds a path from strings a caller passes. `is_shortcode`,
  `is_file_name`, `<dir>/<shortcode>` existing and the symlink refusal are each tested. Narrative names come from the
  KDL, itself a deposited file, so they are not trusted either.
- **Size.** Text only: `config.kdl`, 16 narrative files, the curation CSV. No LFS. The `.gitattributes:14` rule
  (`*-curation.csv text eol=lf`) gains a sibling for `areas/access/media/data/**/curation.csv`. The CSV's `note`
  column ships in the image as it does today.
- **Docs that stay.** Incunabula's `PROVENANCE.md`, `CURATION.md`, `DATA_ISSUES.md`, `SHAPE.md` and `assets/README.md`
  describe the project's data and stay in its folder (ADR-0012: "with the documents that describe the project's
  data"); only their path references change. The provenance of the files themselves goes in `media/data/PROVENANCE`.
- **Tests that read real files.** `store-build/tests/it/main.rs:38` and `worked_example.rs:7` `include_str!` the
  KDL, and `main.rs:57-63` takes curation from `sync`'s snapshot; `store-build/tests/it/hooks.rs:1069-1155` scans
  `../projects` for each row's KDL; `docs/src/cpe/config-language.md:616` names the path and `worked_example.rs:33`
  pins it (`:611` names `PROVENANCE.md`, which stays). `cpe-store-build`'s tests read the media files directly
  (`fs::read`/`include_str!`). `cpe-server`'s tests read the committed media directory through `LiveDepositedFiles`,
  so `media-store` becomes a `[dev-dependencies]` entry of `cpe-server`, which the Phase 2 seam rule allows.
- **`Cpe::activate`'s call sites.** `serve.rs` (407, 421, 599, 738, 1085), `activation.rs` (314, 429, 462, 476, 499,
  516, 541, 542, 562, 611) and `base_tests.rs` (43, 602).
- **Mostly unaffected.** `cpe/web/tests/projects/` (the `broken-*` configs) call `Project::load` with contents
  directly; only the inventory-completeness field reaches `cpe/web/tests/validation.rs`.
- **The incubator.** Its host calls `from_sql_folders` (unchanged) and vendors `cpe-*`; `Cpe::activate`'s new
  signature and `build`'s new argument reach it on its next re-vendor.
- **Reviewers for each phase review:** `eng:review:rust-reviewer`, `eng:review:dune-reviewer`,
  `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`, plus `eng:review:security-reviewer` and
  `eng:review:ivan-reviewer` (bound in `eng.yaml`). `eng:review:devops-reviewer` joins for Phase 5.

## Implementation Phases

#### Phase 1: The bytes port in `cpe-ports`

**Tests first**

- [x] Write a test: `is_shortcode` accepts `0803` and `0AB1`, and refuses `803`, `08030`, `0ab1`, `ZZZZ` and `../x`
- [x] Write a test: `is_file_name` accepts `config.kdl`, `curation.csv` and `narrative/home.de.md`
- [x] Write a test: `is_file_name` refuses an empty name, an empty segment (`a//b`), a trailing `/`, `..`, a segment starting with `.` (`narrative/.x`), a leading `/`, a backslash and a non-ASCII name
- [x] Write a test: `FakeDepositedFiles` serves what was registered last when a file and `with_unavailable` name the same project
- [x] Write a test: `FakeDepositedFiles` serves a file it holds, whole
- [x] Write a test: `FakeDepositedFiles` answers `UnknownProject` for a shortcode it does not hold and `UnknownFile` for a name it does not hold
- [x] Write a test: `FakeDepositedFiles` answers `UnknownFile` for a name `is_file_name` refuses, even if it was added under that name
- [x] Write a test: `with_unavailable` answers `Unavailable` for every file of the project, and `with_unavailable_file` for that file only
- [x] Write a test: `requests()` returns every `(shortcode, name)` asked for, in order
- [x] Write a test: `FilesError`'s `Display` names the shortcode and file, and `Unavailable` includes its source

**Implementation**

- [x] Implement the `DepositedFiles` trait and `FilesError` in `areas/access/cpe/ports/src/lib.rs`
- [x] Implement `is_shortcode` and `is_file_name`
- [x] Implement `FakeDepositedFiles` in `areas/access/cpe/ports/src/fake.rs`
- [x] Update the crate doc of `cpe-ports` to name both ports

**Verification**

- [x] Run `cargo clippy -p cpe-ports --all-targets -- -D warnings`; it passes (`just check` does not lint test targets)
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: `media-store` and its adapter

**Scaffold**

- [x] Create `areas/access/media/store` as crate `media-store` (depends on `cpe-ports`, plus `thiserror` only if the adapter defines its own error and `tracing` only if it logs, since `just check` runs `cargo machete`; `tempfile` as dev-dependency), matching `sync-store`'s manifest layout
- [x] Add `areas/access/media/store` to the root `Cargo.toml` workspace `members`
- [x] `git add` the new manifest before running the gates (`check-cpe-seam.sh` reads manifests from the index, `:56-57`)

**Tests first** (each over its own `tempfile` directory, never over `areas/access/media/data`)

- [x] Write a test: `LiveDepositedFiles` serves `<dir>/<shortcode>/config.kdl` whole
- [x] Write a test: `LiveDepositedFiles` serves a nested name (`narrative/home.de.md`)
- [x] Write a test: a shortcode without a directory is `UnknownProject`
- [x] Write a test: a shortcode `is_shortcode` refuses is `UnknownProject`, even when a directory of that name exists inside or beside `<dir>`
- [x] Write a test: a missing file in a known project is `UnknownFile`
- [x] Write a test: a name `is_file_name` refuses (`../0804/config.kdl`) is `UnknownFile`, even when that path exists
- [x] Write a test: a symlinked project directory is `UnknownProject` and a symlinked file is `UnknownFile`
- [x] Write a test: a symlinked intermediate directory (`narrative -> elsewhere`) is `UnknownFile`, even when the target holds the file
- [x] Write a test: a regular file under the shortcode's name is `UnknownProject`
- [x] Write a test: a file that cannot be read (a directory under the file's name) is `Unavailable`
- [x] Write a test (Unix, skipped when running as root): a project directory without read permission is `Unavailable`, not `UnknownProject`
- [x] Write a test: the adapter re-reads on every call, so a changed file is served changed

**Implementation**

- [x] Implement `LiveDepositedFiles` in `areas/access/media/store/src/lib.rs`, with a crate doc that names the stand-in, the anonymous consumer and ADR-0012

**The seam gate**

- [x] Extend `.github/scripts/check-cpe-seam.sh` with `areas/access/media/*` manifests: an absence error, readable names, forbidden to CPE manifests outside dev-dependencies, forbidden as workspace-root rename targets
- [x] Update `check-cpe-seam.sh`'s header, `Forbidden names` paragraph, failure text and success line for the `media` edge
- [x] Add a `media-store` crate to `base_repo` in `.github/scripts/check-cpe-seam.test.sh` and update the manifest counts (`:126`, `:621`)
- [x] Add the mirrored rule-4 cases for `media-store`: dependency fails, dev-dependency passes, rename fails, a second media crate is forbidden without an edit
- [x] Add a dropped-path case for `areas/access/media` and a rule-1 case for `media-store` → `cpe-ports`

**Verification**

- [x] Run `cargo clippy -p media-store --all-targets -- -D warnings`; it passes
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: CPE reads Incunabula through the bytes port

**Baseline pin (before any other change in this phase)**

- [x] Extract a helper `incunabula_store_text()` from `incunabulas_store_text_is_the_one_the_prototype_embeds` (`activation.rs:586-606`), so later steps change only the helper
- [x] Write a test whose name contains `incunabula` (so `cargo test -p cpe-server incunabula` selects it), using that helper, that pins the length and SHA-256 of Incunabula's store text below the header (split at the first `"\n\n"`, as in Proposed Solution), computed on the unchanged code; it passes

**Data**

- [x] `git mv` `areas/access/cpe/projects/incunabula/config.kdl` to `areas/access/media/data/0803/config.kdl`
- [x] `git mv` `areas/access/cpe/projects/incunabula/narrative/` to `areas/access/media/data/0803/narrative/`
- [x] Copy `areas/access/sync/data/0803-curation.csv` to `areas/access/media/data/0803/curation.csv` (`sync`'s copy is removed in Phase 4)
- [x] Add `areas/access/cpe/projects/incunabula/SHORTCODE` holding `0803`
- [x] Add a `.gitattributes` rule giving `areas/access/media/data/**/curation.csv` `text eol=lf`

**Tests first**

- [x] Write a test: discovery for activation finds a folder with a `SHORTCODE` file and carries its shortcode
- [x] Write a test: discovery for activation skips a folder without `SHORTCODE`, one whose `SHORTCODE` is a symlink, and one whose `SHORTCODE` fails `is_shortcode`, and returns each skipped folder with its reason
- [x] Write a test: for two folders with the same `SHORTCODE`, discovery returns each with a refusal naming the other, and both prefixes answer 503
- [x] Write a test: `from_sql_folders` still serves its `config.kdl` + `data.sql` fixtures as before
- [x] Write a test: the `cpe-core` accessor lists every narrative file the KDL names, from the page-body `narrative` and from `resource-detail > narrative-for`, and still lists them when the KDL fails to lower
- [x] Write a test: `Remodel::reads_curation` holds for each of its four sources (a `curated` node, a `curated-key=` attribute, a hook key, annotations) and not for the THINGS fixture (`cpe/server/tests/fixtures/activation/things/config.kdl`)
- [x] Write a test: validation with a complete, empty inventory reports `NarrativeFileNotFound` for a KDL that names `home.md`
- [x] Write a test: activation fetches exactly the KDL, the curation file, and the narrative bases and variants the KDL names, and no KDL narrative name containing `/` or `\`: every request is `config.kdl`, `curation.csv` or `narrative/<one segment>` (`FakeDepositedFiles::requests()`)
- [x] Write a test: a missing non-default narrative variant falls back as today
- [x] Write a test: a KDL that names a narrative the port does not hold leaves the project not ready, with `NarrativeFileNotFound`
- [x] Write a test: a KDL whose `remodel` shortcode differs from `SHORTCODE` leaves the project not ready, with a reason naming both, asserted on the returned report as `a_store_that_cannot_be_built_is_reported_with_its_cause` does (`activation.rs:376-404`)
- [x] Write a test: a project whose `config.kdl` is `UnknownFile` answers 503 on its prefix
- [x] Write a test: a `SHORTCODE` naming a project `media` does not hold (`UnknownProject`) answers 503 on its prefix
- [x] Write a test: a project with one narrative file `Unavailable` answers 503 on its prefix
- [x] Write a test: a non-UTF-8 `config.kdl` or narrative file leaves the project not ready
- [x] Write a test: a project without `curation.csv` whose remodel reads no curation key (THINGS) builds with no curated values
- [x] Write a test: a project without `curation.csv` whose remodel reads a curation key is not ready
- [x] Write a test: the curation parser in `cpe-store-build` refuses a dangling resource, a duplicate row, a malformed key or language, and a control character
- [x] Extend `the_store_is_rebuilt_from_the_port_alone_and_follows_it` (`activation.rs:508`): a changed fake KDL or narrative changes the served page, as a changed snapshot does (ADR-0012's rebuild-from-empty test per port)
- [x] Write a test in `cpe-server`: with `LiveDepositedFiles` over a missing directory, Incunabula answers 503 and the overview does not list it, beside `without_the_data_directory_incunabula_answers_503_on_its_prefix` (`activation.rs:609`)
- [x] Write a test in `access-server`: `data_dir` returns the default for `None` and for `Some("")`, and the value otherwise
- [x] Write a test in `access-server`: the default media directory is the committed one, beside `the_default_sync_data_directory_is_the_committed_one`

**Implementation**

- [x] Add the narrative-name accessor to `cpe-core`, over the raw parse
- [x] Add `Remodel::reads_curation` to `cpe-core` (`remodel.rs`)
- [x] Add the inventory-completeness field to `ValidationContext` (`context.rs:10-33`), defaulting to "not complete" in `for_tests`, and update its doc (`context.rs:16-19`, `validate.rs:4774-4779, 4808-4811`)
- [x] Add the new field to every `ValidationContext` struct literal listed in Proposed Solution
- [x] Copy the curation parser and its errors into `cpe-store-build` behind `CuratedValues` (`parse`, `none`, `from_unchecked`), with `curation_tests.rs`
- [x] Change `cpe_store_build::build` to take `&CuratedValues`, switching `refuse_interior_nuls` (`lib.rs:236-238`) and `Curation::read` (`generic/curation.rs:38-47`) to it and rewriting `build`'s doc (`lib.rs:37-53`)
- [x] Update the unit-test literals at `generic/curation.rs:183-195`
- [x] Add `static CURATION: LazyLock<CuratedValues>` to `store-build/tests/it/main.rs` (the media CSV parsed against `SNAPSHOT`) and build `STORE` with it
- [x] Make the `build(snapshot)` wrapper (`main.rs:50-52`) pass `CuratedValues::from_unchecked(snapshot.curation.clone())`
- [x] Add a `build_with(snapshot, mapping, hook)` helper to `tests/it/support.rs` that does the same, and switch `hooks.rs` and `hookless.rs` from their direct `cpe_store_build::build` imports (`hooks.rs:16`, `hookless.rs:20`) to it
- [x] Change the store header in `store-build/src/sql.rs:82-83` to name the archive projection and the bytes port, and update the header pins in `hookless.rs:1205-1206, 1355-1356` and `sql.rs:523-578` (including the line count at `:533` if the header gains a line)
- [x] Re-pin `INCUNABULA_STORE_BYTES` and `INCUNABULA_STORE_SHA256` (`activation.rs:592-593`) to the new header
- [x] Rename `incunabulas_store_text_is_the_one_the_prototype_embeds` and rewrite its doc (`activation.rs:586-606`): the whole text is no longer the prototype's, the header-stripped pin carries that lineage
- [x] Split discovery into a `SHORTCODE` predicate for activation and the `config.kdl` predicate for `from_sql_folders`, with the module doc and warnings saying which is which
- [x] Add `SHORTCODE` fixtures for the activation discovery tests; leave the `from_sql_folders` fixtures as they are
- [x] Change `load_all` (`activation.rs:57`) and `load` (`:156-159`) to take a discovery function and a per-project source of config source, narrative inventory and contents, completeness flag and store text; `from_sql_folders` supplies the folder's, `from_projection` the ports'
- [x] Rewrite activation's first step: fetch `config.kdl` through the port, decode it as UTF-8, compare its shortcode with `SHORTCODE`
- [x] Rewrite activation's narrative step: fetch the names the accessor lists, and pass the completeness flag to `load`
- [x] Rewrite activation's curation step: fetch `curation.csv`, apply `reads_curation` to an absent file, `CuratedValues::parse` against the snapshot
- [x] Name files by their media location in activation's diagnostics (`activation.rs:131,171`), and log the curated-value count from `CuratedValues` (`activation.rs:141`)
- [x] Change `Cpe::activate` to take `&dyn DepositedFiles`, and every call site listed in Technical Considerations
- [x] Change `activate_in_background` (`serve.rs:160-166`) and its test call sites (`serve.rs:960`, `:988`) to take both adapters
- [x] Add `data_dir` to `serve.rs` and read `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` through it, with `DEFAULT_MEDIA_DATA_DIR`
- [x] Warn when the media directory is missing, as `serve.rs:226-231` does for `sync`'s
- [x] Construct `LiveDepositedFiles` in `serve()` and hand it to `activate_in_background`
- [x] Add `media-store` to `access-server`'s `[dependencies]` and to `cpe-server`'s `[dev-dependencies]`
- [x] Change the `activation.rs` test helpers (`projects()`, `committed_projects()`, `sync_data()`) to supply `FakeDepositedFiles` or the committed media directory
- [x] Change `base_tests.rs` (`:25-47`, `:602`) to activate Incunabula over the committed media directory
- [x] Change `access-server`'s `incunabula()` helper (`serve.rs:416-424`) to pass `LiveDepositedFiles` over the committed media directory
- [x] Change `access-server`'s `projects_folder` helper (`serve.rs:391-400`) and its THINGS fixtures to supply `FakeDepositedFiles`
- [x] Retarget the traversal tests (`serve.rs:798-817`, `:1037`, and `base_tests.rs:468`'s `/cpe/incunabula/_assets/config.kdl` 404) at a file that stays in the project folder (`SHORTCODE` or `PROVENANCE.md`), asserting on its content, so they cannot pass vacuously
- [x] Repoint `MAPPING` in `store-build/tests/it/main.rs` and the `include_str!` in `worked_example.rs` to the media KDL
- [x] Rewrite the hook-table tests (`store-build/tests/it/hooks.rs:1069-1155`): real projects resolve folder → `SHORTCODE` → `media/data/<shortcode>/config.kdl`; the fixture controls keep their folder layout through a resolver parameter
- [x] Repoint `docs/src/cpe/config-language.md:616` to `areas/access/media/data/0803/config.kdl`
- [x] Reword `docs/src/cpe/config-language.md:2410` ("present in the `narrative/` directory") and the `NarrativeFileNotFound` message (`validate.rs:4840`), which name a directory the port does not have

**Verification**

- [x] Run `cargo test -p cpe-server incunabula`; the header-stripped pin passes with its constants unchanged from the baseline step
- [x] Run `cargo clippy --workspace --all-targets -- -D warnings`; no warning in a crate this branch touches
- [ ] The same run passes workspace-wide: it fails only in `mosaic-playground` (`src/app.rs:188`) and `dpe-api-oai`
  (`metadata/corpus.rs:586,621`), which this branch never touches (out of scope; see the journal's `## Deferrals`)
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Retire curation from `sync` and the snapshot

**Implementation**

- [x] Move `CuratedValue` and `is_curation_name` from `cpe-ports` into `cpe-store-build`, and repoint the `CuratedValue` imports in `tests/it` (`hooks.rs:15`, `support.rs:12`) to `cpe_store_build`
- [x] Remove `ProjectSnapshot.curation` and every literal that sets it (`cpe-ports`, `cpe-store-build` tests, `cpe-server`, `access-server` `serve.rs:386`, `sync-store`'s `src/mapping/mod.rs:42`, `cpe-store-build`'s `generic/membership.rs:136`)
- [x] Move the `cpe-store-build` test helpers' curated values off the snapshot into a fixture type that the `build` wrapper and `build_with` turn into `from_unchecked`, carrying over the non-empty literals' contents (`hookless.rs:152`, `:1866`; `hooks.rs:88`, `:625`, `:908`; `support.rs:454-462`, where `minimal_snapshot` takes `curation` as a parameter). The fixture type is the argument of the `build` wrapper and of `build_with`; `with_curation`, `with_curation_in` and `recurated` (`support.rs:310-336`) push to and retain on its values; the committed data's fixture is `SNAPSHOT` plus `CURATION`'s values, and no constructor defaults curation to empty for a snapshot of the committed data
- [x] Replace the reads of `SNAPSHOT.curation` (`committed_0803.rs:189-190`) with `CURATION`
- [x] Make every build derived from the committed data take `CURATION`'s values: `hooks.rs` (`:1350`, `:1390`, `:1399`), `without()` (`committed_0803.rs:455-456`, used at `:521` and `:548`), `file_of(&SNAPSHOT)` (`:570`) and the reordered build (`:583ff`), so `test_build_reads_every_declared_property` and its annotation sibling cannot pass with an empty `unread`
- [x] Change the order-independence tests (`hookless.rs:1326-1339`, `committed_0803.rs:578-594`) to reverse the fixture values
- [x] Remove curation from `cpe-ports`'s docs (`lib.rs:1-7, 28-37`, `contract.rs:11-14`, and reword `:16-18` if it names curation, `snapshot.rs:1-2, 67, 80-87`)
- [x] Remove the contract's curation violations, `curation_name`, their `Display` arms and their tests from `cpe-ports/src/contract.rs`, and the matching `contract_error` arms in `cpe-store-build/src/lib.rs:130-132`
- [x] Remove the curated values from `cpe-ports/src/fake.rs`'s tests
- [x] Remove `read_curation`, the `curation` module, `InvalidCuration`/`CurationFault`/`SnapshotError::Curation`, `curation_tests.rs` and the curation cases in `snapshot_tests.rs`, `ark_tests.rs` and `tests/committed_0803.rs` from `sync-store`
- [x] Rewrite `sync-store`'s crate doc and `LiveArchiveProjection`'s doc to name only the snapshot
- [x] `git rm` `areas/access/sync/data/0803-curation.csv` and the `.gitattributes:14` rule for it
- [x] Move the curation section of `areas/access/sync/data/PROVENANCE` into `areas/access/media/data/PROVENANCE`, and reword `PROVENANCE:26-28`: a row of `media/data/0803/curation.csv` naming a dropped resource now leaves Incunabula not ready in CPE
- [x] Update `areas/access/sync/CONTEXT.md`: its intro (`:5-8`), **Committed curation**, **Known project** and **`LiveArchiveProjection`** no longer mention curation
- [x] Write the `config.kdl` and `narrative/` entries of `areas/access/media/data/PROVENANCE`: where each was authored, the PR (DEV-7511/DEV-7512) it moved here in, how to change it

**Verification**

- [x] Run `rg -l 'CuratedValue|read_curation|-curation\.csv' areas/access/sync/store areas/access/sync/data areas/access/cpe/ports/src areas/access/sync/CONTEXT.md`; it prints nothing
- [x] Run `cargo test -p cpe-server incunabula`; both store pins pass with their constants unchanged from Phase 3
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: The image ships `media`'s data

- [x] Stage `areas/access/media/data` as `staging/media/data` in `.github/actions/build-dpe/action.yml`
- [x] Add `COPY media/data /app/media/data` and `ENV MEDIA_DATA_DIR="/app/media/data"` to `areas/access/server/Dockerfile`
- [x] Update the Dockerfile's header comment (`:6-7`): `sync/data` holds snapshots only, `cpe/projects` holds assets and documents, `media/data` holds the deposited files
- [x] Stage `areas/access/cpe/projects` in `just build-docker-dpe` (`justfile:553-593`), which the Dockerfile's `COPY cpe/projects` already needs and the recipe lacks
- [x] Stage `areas/access/media/data` in `just build-docker-dpe`
- [x] Add `areas/access/media/**` to both triggers of `.github/workflows/a11y-dpe.yml`
- [x] Add `areas/access/cpe/**`, `areas/access/sync/**` and `areas/access/media/**` to the path filter of `.github/workflows/cloud-run-dpe-pull-request.yml`, so a change to a presentation's files gets a preview
- [x] Add the same three paths to `.github/workflows/scout-dpe.yml`, which scans the same image
- [x] Add `areas/access/media/data/` to `.dockerignore` beside `sync`'s entry (`:44-45`; it affects only the repo-root-context builds)
- [x] Update `docs/src/dpe/operations.md`: the data bullet (`:15-17`), a `MEDIA_DATA_DIR` row beside `SYNC_DATA_DIR` (`:91`), the `SYNC_DATA_DIR` and `CPE_PROJECTS_DIR` rows' contents (`:91`, `:93`), and the PR-preview note (`:102`)
- [x] Update `docs/src/cpe/operations.md` for the media directory: `:12-13`, the variable list (`:21`), the failure rows (`:36-37`), the size line (`:53`) and the staged-size paragraph (`:81-83`)
- [x] Update the docs that restate the path filters: `docs/src/deployment.md` (`:20`, `:114`, `:116`), `docs/src/repo_structure.md:117`, `docs/src/security.md:26`
- [x] Run `just build-docker-dpe`; the image (tag `dpe`) builds
- [x] Run `docker export "$(docker create dpe)" | tar -t | grep -E '^app/media/data/0803/(config\.kdl|curation\.csv)$'`; both paths are listed (the image is distroless, so files are checked from the host), then remove the created container
- [x] Run `docker run -d --rm --name dpe-media -p 18080:8080 dpe`, poll `curl -s -o /dev/null -w '%{http_code}' http://localhost:18080/cpe/incunabula/` until 200 (up to 30 s; CPE answers 503 until activation finishes), then `curl -fsS http://localhost:18080/cpe/incunabula/ | grep -c 'Die Bilderfolgen der Basler Frühdrucke'` prints at least 1; `docker stop dpe-media`
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: Vocabulary, docs and the map

- [x] Write `areas/access/media/CONTEXT.md` in `sync/CONTEXT.md`'s shape: the stand-in, `MEDIA_DATA_DIR`, the layout, `LiveDepositedFiles`, the anonymous consumer, what `media` does not do yet (derive, serve assets)
- [x] Update `areas/access/cpe/CONTEXT.md`: **Bytes port** and **Curation** no longer "planned"/"until DEV-7511", the project folders section (`SHORTCODE`, what stays, `from_sql_folders` as legacy), `CpeConfig`, and the port's stricter file-name rule
- [x] Update `areas/access/CONTEXT.md` and `areas/access/datei/CONTEXT.md` (`media` exists; DaTEI's adapter waits for its port)
- [x] Update `vitrinli/CONTEXT.md` (the stand-in is built) and the root `CONTEXT.md` Access Area line
- [x] Add a dated `## Amendment` to `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`: the store header names both ports, the body stays byte for byte the prototype's, and the incubator's `data.sql` follows on its next re-vendor
- [x] Update `areas/access/server/CLAUDE.md`: the dependency boundary names `LiveDepositedFiles` from `media-store`; the image section names `media/data` and the widened path filters
- [x] Repoint path references in Incunabula's `PROVENANCE.md`, `CURATION.md`, `DATA_ISSUES.md` (`:249`) and `assets/README.md`
- [x] Repoint path references in `areas/access/cpe/store-build/README.md` (`:7-11` for `build`'s signature and caller flow, `:79`, `:95-96`, `:100`, `:210`; the `tests/fixtures/*/config.kdl` lines stay), and `build`'s signature in `areas/access/cpe/CONTEXT.md:142`
- [x] Reword the comments in the moved `config.kdl` that name `DESIGN.md` (`:6`, `:12`, `:60` and later), which is not in this repository and already points nowhere
- [x] Add `media-store` to the scope tables in `CONVENTIONS.md` and `docs/src/git-conventions.md`
- [x] Update the tree in `docs/src/repo_structure.md`: `cpe/ports` (`:26`), `cpe/projects` (`:27`), `sync` (`:34-36`), and a new `media` entry before `dpe/` (`:37`); add `media` to the bounded-context list (`:7`)
- [x] Update the crate table in `docs/src/repo_structure.md`: `access-server` (`:80`), `cpe-ports` (`:85`), `cpe-store-build` (`:90`), `cpe-server` (`:92`), `sync-store` (`:93`), and a `media-store` row after it
- [x] Update `ARCH-MAP.md`: register `areas/access/media` as built and refresh the `cpe`, `sync`, `datei` and `access/server` entries, with `/dune:dune-map`, or by hand at the overview (`:20-25`), `access/server` (`:137-196`), `cpe` (`:830-1046`), `datei` (`:1047-1089`), `sync` (`:1090-1179`) and `media` (`:1180-1226`)
- [x] Run `rg 'projects/incunabula/(config\.kdl|narrative)|0803-curation\.csv' --glob '!docs/specs/**' --glob '!docs/adr/**' --glob '!CHANGELOG.md' --glob '!areas/access/media/data/PROVENANCE'`; it prints nothing (`CHANGELOG.md` is generated history; `PROVENANCE` records where the files came from)
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: History

Runs after every other phase review, so that no reviewed commit is rewritten before it has been reviewed.

- [x] Squash the branch to one commit, `feat(media-store,cpe-server): …` naming DEV-7511 and DEV-7512, with scopes per `CONVENTIONS.md`
- [x] Run `just commit-lint`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge #494 (ADR-0012) before this PR, or retarget this PR's base after it merges | Developer | after ship | Merging is the owner's decision |
| H2 | Move DEV-7511 and DEV-7512 through review and close them on merge | Developer | after ship | Ticket ownership |
| H3 | On the incubator's next re-vendor, re-source its copy of the curation file and Incunabula's KDL and narrative from `areas/access/media/data/0803/` | Developer | after ship | dsp-incubator is another repository, outside this plan |

## Acceptance Criteria

- [ ] `areas/access/media/store` (`media-store`) implements `cpe_ports::DepositedFiles` as `LiveDepositedFiles` over `MEDIA_DATA_DIR`, default `areas/access/media/data`
- [ ] No caller-supplied string reaches the file system outside `<MEDIA_DATA_DIR>/<shortcode>/`; each refusal is tested
- [ ] `areas/access/cpe/projects/incunabula/` holds no `config.kdl`, no `narrative/` and a `SHORTCODE` file
- [ ] `areas/access/sync/data/` holds no curation file; `ProjectSnapshot` has no `curation`; `CuratedValue` lives in `cpe-store-build`
- [ ] CPE activates Incunabula from the two ports; its store text below the header is byte-identical to before
- [ ] A missing narrative, a missing required curation file, a non-UTF-8 file, a shared `SHORTCODE` and a shortcode mismatch each leave a project not ready, not silently degraded
- [ ] `from_sql_folders` serves its fixtures as before, with its existence check unchanged
- [ ] An empty `MEDIA_DATA_DIR` or `SYNC_DATA_DIR` is treated as unset
- [ ] A rebuild-from-empty test runs CPE's activation against both `Fake` ports, and follows each
- [ ] `check-cpe-seam.sh` forbids CPE crates from depending on `media` crates outside dev-dependencies, with tests
- [ ] The image carries `/app/media/data` and sets `MEDIA_DATA_DIR`; `just build-docker-dpe` builds an image that serves Incunabula
- [ ] A change under `areas/access/cpe/`, `sync/` or `media/` triggers a PR preview and a Scout scan
- [ ] `media`'s `CONTEXT.md` and `PROVENANCE` exist; `cpe` and `sync`'s `CONTEXT.md` describe the ports as built; ARCH-MAP lists `areas/access/media` as built
- [ ] `just check` and `just test` pass; the PR is one commit and passes `just commit-lint`

## Dependencies & Risks

- **#494 must merge first.** This branch is rebased on it; ADR-0012 is the record cited throughout.
- **Risk: the store text changes beyond the header.** Curation changes owner and the narrative inventory changes
  origin. The header-stripped pin, written against the unchanged code before Phase 3 touches anything, catches it
  in Phases 3 and 4. The narrative never reaches the store text, so that pin cannot catch a narrative regression;
  `INCUNABULA_PAGES` (`activation.rs:577-584`) does.
- **Risk: the narrative inventory differs.** Today it is every file in `narrative/`; after, it is what the KDL
  names. For Incunabula the two are equal (eight bases, sixteen files). The completeness flag keeps an empty
  inventory from switching the existence check off.
- **Risk: Phase 3 is large.** It moves data, discovery, activation and the curation reader together, since none of
  them passes alone. Phase 4 keeps the removals out of it, and the checklist splits it per file.
- **`just build-docker-dpe` is broken on `main`.** It does not stage `cpe/projects`, which the Dockerfile copies;
  Phase 5 fixes it because its verification needs the image.
- **The incubator.** `Cpe::activate`'s and `build`'s signatures change under its next re-vendor; `from_sql_folders`,
  which its host calls, does not. Its vendored copy of `0803-curation.csv` comes from `sync/data` today
  (`areas/access/cpe/projects/incunabula/PROVENANCE.md:49`); it must be re-sourced from
  `areas/access/media/data/0803/curation.csv` (H3).
- **Deployment.** DEV-7514 mounts host storage at `MEDIA_DATA_DIR` in ops-deploy; until then the image's copy is
  served. Nothing here depends on it.

## Success Metrics

- The SHA-256 of Incunabula's store text with its header lines removed, pinned before Phase 3 changes anything, is
  unchanged at the end of Phases 3 and 4.
- `INCUNABULA_STORE_BYTES` and `INCUNABULA_STORE_SHA256` (`areas/access/cpe/server/src/activation.rs:592-593`, today
  `5_631_851` bytes) change only by the header lines.
- A locally built image serves `/cpe/incunabula/` with 200.

## References

- ADR-0012 `docs/adr/0012-build-time-inputs-come-through-two-channels-one-consumer-each.md`
  (PR #494); ADR-0009 and its amendment; ADR-0010 (superseded)
- Pattern: `areas/access/sync/store/src/lib.rs:65-111` (`LiveArchiveProjection`, the `KNOWN` guard)
- Pattern: `areas/access/cpe/ports/src/lib.rs:33-76`, `fake.rs` (port, error, fake)
- Composition root: `areas/access/server/src/serve.rs:11,160-166,209-238`
- Discovery and activation: `areas/access/cpe/server/src/discovery.rs:82-213`, `activation.rs:31-184`;
  `from_sql_folders` at `lib.rs:134` and `activation.rs:38-46`
- Narrative validation: `areas/access/cpe/core/src/util.rs:307` (`variant_filename`), `validate.rs:4783-4849` (`check_narrative_file`)
- Curation parser: `areas/access/sync/store/src/curation.rs`; store header `areas/access/cpe/store-build/src/sql.rs:82-83`
- Gate: `.github/scripts/check-cpe-seam.sh` (rule 4, `:15-22`; `main` at `:248-374`) and its `.test.sh`
- Prior plan in the same shape: `docs/specs/2026-09-29-minimal-sync-capability/02-feat-minimal-sync-capability-plan.md`
- Learnings: `docs/learnings/configuration-errors/empty-composite-input-beats-compose-env-file.md`,
  `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`
- Linear: DEV-7511, DEV-7512; blocks DEV-7514; related DEV-7513
