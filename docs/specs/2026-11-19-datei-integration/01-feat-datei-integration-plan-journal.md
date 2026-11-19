---
plan: docs/specs/2026-11-19-datei-integration/01-feat-datei-integration-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator
base_commit: 788e8147270ea719434add8576377d730a367738
branch: feat/dev-7549-datei-reads-files-by-name
started: 2026-11-19
problem: >
  The Keller edition must be served by DaTEI from dsp-repository on stage, not by the
  incubator's showcase. DaTEI today reads a project's files through a port keyed by content
  hash, ships a prebuilt snapshot and activates before the bind, each in conflict with
  dsp-repository's rules (ADR-0003, ADR-0008, ADR-0009, ADR-0012). Phase 0 changes the
  incubator so that DaTEI reads a project's files by stored name, through its file layout and
  an optional manifest written by the VRE bridge, with advisory hashes.
symptoms:
status: in-progress
autonomy: decide
---

# Execution Journal: 01-feat-datei-integration-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-incubator | 788e8147 | feat/dev-7549-datei-reads-files-by-name | squash | in-progress | — |
| dsp-repository | 7d5eae08 | docs/dev-7934-datei-integration-adrs | rebase | in-progress | — |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 0 | reviewed | dsp-incubator@788e8147 | 2 |
| 1 | reviewed | dsp-repository@22ecfbec | 2 |

## Archived phases

- Phase 0: 8 chunks, ids 0.1 to 0.8, highest number 0.8, in 01-feat-datei-integration-plan-journal-archive.md
- Phase 1: 9 chunks, ids 1.1 to 1-review-2, highest number 1.7, in 01-feat-datei-integration-plan-journal-archive.md

## Chunk queue

Paths are relative to `da-tei/crates/` in dsp-incubator.

Phase 1: paths are relative to the dsp-repository root.

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|


## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|

## Deferrals

- Phase 0: scope (Warning): `DirectoryFiles::file` follows symlinks under `files/` and reads without a size or file-type limit
  (security review). A symlink, FIFO or huge file delivered into a project directory would be read. Not fixed: the incubator
  configurator's adapter only; the bridge/media side in dsp-repository (Phase 5) should refuse non-regular files and cap sizes.

## Deviations

- Phase 0: proposed: `StoredName` accepts `""`, `.`, empty and `.` segments (decided: refuses only `..` segments, leading `/`, `\`,
  NUL). Rust and security reviewers propose refusing an empty name and empty/`.` segments too: `""` or `.` resolves to the
  `files/` directory (EISDIR, surfaced as `Unavailable`/500 rather than a manifest error), and `a.xml` vs `./a.xml` escape
  the duplicate-stored-name check. Left as decided; for the owner.
- Phase 0: provisional: `ProjectFiles::open` also refuses a layout entry whose stored name is `file-layout.xml` or
  `file-manifest.json` (`ProjectFilesError::ReservedName`), so a file of the edition can never alias the layout or manifest.
  Alternatives: reserve the names in `LogicalPath::new` (touches upload), or document the quirk. Chosen as least scope. dd4f70de.
- Phase 0: provisional: the configurator opens projects with `da_tei::files::open_project` (returns `ProjectFiles`);
  `DirectoryFiles` is private. `OpenError` is `NoLayout`/`Unparseable` (with hints, 409) or `Files(ProjectFilesError)`;
  `files_error_status` maps `Unavailable` to 500, all else 409. Error messages quote names and paths with `{:?}`, so
  configurator messages now read `"odd/main.odd"` where they read backticks. fbb75244, dd4f70de.
- Phase 1: provisional: beyond the plan's file list, the review fixes also touched ADR-0004 (a Not-yet line for DaTEI's
  `text/html` popover and missing end-to-end suite), the root `CONTEXT.md` and `vitrinli/CONTEXT.md` (stand-in no longer
  "hand-fed", DaTEI no longer "a seed"), and `CONVENTIONS.md`'s copy of the scope list. Alternative: leave them stale
  until a later phase. Chosen because each contradicted the rewritten ADRs. 43069509, 46e524da, 37de5d57.
- Phase 1: provisional: ADR-0009 and `datei/CONTEXT.md` name Keller (083C) as the first project and 086A second (they
  said 086A first), following the plan's Overview; the fragment and end-to-end deviations say "No end is scheduled",
  since the plan gives them no end condition. 88147799, 54ce5c9d.

## Side findings

- Phase 0: `PortError::Unavailable` and `ProjectFilesError::Read` both print their cause and declare it `#[source]`, so a
  chain-walking reporter repeats it; `LayoutUnreadable`/`ManifestUnreadable` print but do not chain. Pick one policy when
  the port moves to `datei-ports` (the plan wants `source()`).
- Phase 0: `LayoutError::BadPath` (pre-existing) quotes paths with backticks, not `{:?}`; a crafted `file-layout.xml` target
  can put a line break into a log line. Worth fixing when the layout moves.
- Phase 0: `write_file_layout` and `load_artefact` resolve `files/{logical path}` directly: a configurator project directory
  is "stored name = logical path, no manifest". A `files/file-manifest.json` dropped into one would disagree with them.
- Phase 0: after fix round 2 (cap reached) no third review ran; open Suggestions: name the fixed-name set once (`FIXED_NAMES`)
  for `ProjectFiles::open` and `DirectoryFiles`; "port" wording in a few comments (`pipeline.rs`, `parser.rs`, `create.rs`, reload).
- Phase 0: dune review suggests renaming `PortError`/`file-port.md`/`*_port.rs` tests to `FileSource` vocabulary; not done
  (the plan names `PortError`).
- Phase 0 (review loop 2026-11-19, 1987f16d): `DirectoryFiles` now reads `file-manifest.json` from the project root, beside
  `file-layout.xml`, not from `files/`: an uploaded `files/file-manifest.json` (a `.json` upload is `Ignored` but still written)
  made every `open_project` fail with 409. `file-port.md` now says a configurator project has no manifest.
- Phase 0: open (Warning, owner): an upload whose TEI file sits at logical path `file-layout.xml` is registered, written into
  the layout, and then every `ProjectFiles::open` fails with `ReservedName` (409) until the file is deleted. Worked before this
  branch. Fix belongs with the open `ReservedName` deviation (reserve the names at upload / in `LogicalPath::new`).
- Phase 0: open Suggestions from the final review: the "no file-manifest.json" info log fires on every configurator open
  (debug?); `ReservedName` message gives no reason or remedy; layout location hard-coded in `read_layout_or_empty`,
  `write_atomically` and `print_dropped_artefacts`; `StoredName` does not refuse `:` (Windows only); `iiif` values unchecked
  (check scheme when dereferenced); stale "SHA-256" wording in the generated layout header, `file-layout.md:4`,
  `assets.md:41`; "file port" in `glossary.md` (Rebuild row).
- Phase 1: ARCH-MAP.md's `areas/access/datei` and `areas/access/media` entries still say DaTEI's port "waits" and
  `datei-server` is "expected"; they contradict ADR-0009 until Phase 5 refreshes them (`/dune:dune-map`). Phase 2 agents
  load the datei kit from ARCH-MAP; read ADR-0009 first.
- Phase 1: the Ready token's fixed file name and whether `datei.toml` carries a format identifier and version are written
  nowhere in this repository (H1 settled them with the bridge session). Phase 4 ("refuse a file that is not the current
  format") and Phase 5 (`projects()` checks the token by name) need both; record them in `datei/CONTEXT.md` then.
- Phase 1: reviewers (simplicity, dune) find ADR-0009 heavy with mechanics (file names, marker, mount paths, header names)
  that docs/adr/CLAUDE.md would put in CONTEXT or docs/src. Kept: the plan's Phase 1 checklist asks ADR-0009 to state them.
  Worth trimming once `docs/src/datei/operations.md` exists (Phase 5).
- Phase 1: a failed project (crash-loop marker included) answers 503 with `Retry-After`, though it needs an operator
  (architecture review). As planned; consider a different signal when Phase 4 lands.
- Phase 1: nothing fences new SQL out of `datei-server` beyond the three named sites until DEV-7939 (dune review).
- Phase 1: the plan's Phase 2 placement rule says "`datei-store` reads", but the step that finishes a built model
  (checkpoint, journal mode, close) is an `IrDb` method in `datei-store` (Technical Considerations, Phase 4), now stated in
  ADR-0009 and `datei/CONTEXT.md`. Phase 2's placement table should allow that one write.
- Phase 1: review cap reached after fix round 2; no third review ran. Open Suggestions left: shorter ADR-0009 mechanics
  (see above), a `Rebuild marker` term in `datei/CONTEXT.md`, the `datei-store-build` → `datei-store` BUILD grant noted
  as a Not-yet line once Phase 2 decides it.

## Owner decisions

- none
