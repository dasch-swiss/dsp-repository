---
plan: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7495/docs/specs/2026-10-09-cpe-port-curation/01-feat-cpe-port-curation-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7495
base_commit: 8a680637816e550a6b65689c8842c44895728e8d
branch: feature/dev-7495-serve-cpes-project-curation-through-sync-and-cpe-ports
started: 2026-10-09
problem: >
  DEV-7402 builds Incunabula's CPE store from the port alone (ADR-0008), but thirteen of its 52 store
  outputs are not archive data: they come from hand-authored curation files and constants in the
  incubator that the port does not serve. DEV-7488 decided, as an interim, that `sync` serves a
  project's per-resource curation, so the read model stays fed by its port. This plan adds
  `ProjectSnapshot.curation`, a strict CSV reader in `sync-store`, Incunabula's committed curation
  file, and ADR-0010 (proposed) recording the change to ADR-0007 and ADR-0008.
symptoms: []
status: in-progress
---

# Execution Journal: 01-feat-cpe-port-curation-plan

## Repos

| repo           | base_commit | branch                                                                  | merge_strategy | status      | pr |
|----------------|-------------|-------------------------------------------------------------------------|----------------|-------------|----|
| dsp-repository | 8a680637    | feature/dev-7495-serve-cpes-project-curation-through-sync-and-cpe-ports | rebase         | shipped     | https://github.com/dasch-swiss/dsp-repository/pull/468 |

## Phases

| phase | status      | phase_base              | review_fix_rounds |
|-------|-------------|-------------------------|-------------------|
| 1     | reviewed    | dsp-repository@6f27df56 | 2                 |
| 2     | reviewed    | dsp-repository@af5c2e25 | 2                 |
| 3     | reviewed    | dsp-repository@5b9974f4 | 1                 |

## Chunk queue

- **1.1** (dsp-repository) ADR-0010 and its index row.
  - files: `docs/adr/0010-the-port-also-serves-per-resource-curation.md`, `docs/src/decisions.md`
  - depends_on: —
  - checkboxes: "Run `git rev-parse HEAD`…", "Write `docs/adr/0010-…`", "Add ADR-0010's row…"
  - acceptance: the record is `status: proposed` with decision bullets, Considered Options, Consequences and an
    `Enforced by:` line; `just check` (check-adr-refs) passes
  - context: plan § Technical Considerations "The ADRs"; form of `docs/adr/0008-*.md` and `0009-*.md`
  - commit: `feat(cpe-ports,sync-store): serve project curation through the port (DEV-7495)` (the plan's one feature commit)
  - replaces: —
- **1.2** (dsp-repository) `CuratedValue`, `ProjectSnapshot.curation`, `contract::is_curation_name`, and every literal.
  - files: `areas/access/cpe/ports/src/{snapshot,lib,contract,fake}.rs`,
    `areas/access/sync/store/src/ark_tests.rs`, `areas/access/sync/store/src/mapping/mod.rs`
  - depends_on: 1.1 (the docs cite ADR-0010)
  - checkboxes: "Add `CuratedValue`…" through "Re-export `CuratedValue`…", "Add `contract::is_curation_name`…",
    the `is_curation_name` and sort tests, the four `curation:` literal sites
  - acceptance: the workspace builds; the two new tests pass; the fake's unchanged-snapshot test covers curation order
  - context: plan § Boundary DTO; `snapshot.rs:1-9`, `:66-79`; `lib.rs:1-35`; `contract.rs:447-449`; `fake.rs:63-102`
  - commit: `fixup!` of the feature commit (rebase-merge repo, one feature commit per PR, no mid-run amend)
  - replaces: —
- **1.3** (dsp-repository) The three curation violations.
  - files: `areas/access/cpe/ports/src/contract.rs`
  - depends_on: 1.2
  - checkboxes: "Declare `Violation::DanglingCuration`…" through "Extend `contract.rs`'s module doc…", the padded-text
    test, "Run `just check && just test`", "Commit the files of this phase…"
  - acceptance: every named `test_violations_…curation…` and `test_display_…curation…` test passes and fails without
    its check; `just check && just test` pass
  - context: plan § Contract; `contract.rs:20-68` (enum), `:79-160` (Display), `:165-304` (violations), `:915` (order test)
  - commit: `fixup!` of the feature commit
  - replaces: —

Not chunked, left for the session: "Push with `git push --force-with-lease`…" and "Edit the body of PR #468…" (the brief
forbids the executor to push or post to GitHub), and the Phase review checkbox.

- **2.1** (dsp-repository) Incunabula's committed curation file, its provenance and its line-end rule.
  - files: `areas/access/sync/data/0803-curation.csv`, `areas/access/sync/data/PROVENANCE`, `.gitattributes`
  - depends_on: — (the drift check against the incubator pin comes first and blocks the phase on a difference)
  - checkboxes: "Run `git rev-parse HEAD`…", "Compare the curation sources with the pin…", "Derive `…/0803-curation.csv`…",
    "Check the file with plain commands…", "Append a `0803-curation.csv` section to `…/PROVENANCE`…", "Add `…*-curation.csv text eol=lf`…"
  - acceptance: the plan's five plain-command checks print 138, the header, 137, no CR and 3; `git check-attr eol` ends in
    `eol: lf`; `just check && just test` pass
  - context: plan § Incunabula's file, § Facts; script at `.claude/tmp/derive_0803_curation.py` (not committed)
  - commit: `fixup!` of 3cb35391
  - replaces: —
- **2.2** (dsp-repository) The curation error types.
  - files: `areas/access/sync/store/src/error.rs`, `areas/access/sync/store/src/lib.rs`, `areas/access/sync/store/src/snapshot_tests.rs`
  - depends_on: —
  - checkboxes: "Add `InvalidCuration`…", "Add `CurationFault`…", "Add `SnapshotError::Curation`…", "Widen `SnapshotError`'s doc…",
    "Re-export `CurationFault` and `InvalidCuration`…", "Write `test_display_curation_error_shows_path_and_line`…"
  - acceptance: the display test passes; the variants, fields and messages are the plan's table
  - context: plan § Reader, "Errors"; `error.rs:8-36`
  - commit: `fixup!` of 3cb35391
  - replaces: —
- **2.3** (dsp-repository) The reader, its tests, and `serve` reading the file.
  - files: `areas/access/sync/store/src/curation.rs`, `areas/access/sync/store/src/curation_tests.rs`,
    `areas/access/sync/store/src/lib.rs`, `areas/access/sync/store/src/snapshot_tests.rs`
  - depends_on: 2.1 (the committed test reads the file), 2.2
  - checkboxes: the fixture helpers and the three extended `snapshot_tests.rs` tests, "Create `src/curation_tests.rs`…", the
    seventeen `test_curation_parse_…` tests, the six `test_snapshot_…curation…` tests, "Add `src/curation.rs`…", "Register
    `curation` and `curation_tests`…", "Read the curation file and call `curation::parse` in `serve`…", the three doc
    updates of `lib.rs`, the `! grep` for Incunabula's keys, "Run `just check && just test`", "Amend Phase 2's files…"
  - acceptance: every table row of the plan is asserted; each new test fails without the code it covers; the `! grep`
    exits 0; `just check && just test` pass
  - context: plan § The curation file, § Reader, "Test tables"; `lib.rs:65-81`; `snapshot_tests.rs:10-13`, `:138-170`
  - commit: `fixup!` of 3cb35391 (one commit: `parse` is `pub(crate)` and unused, a clippy error, until `serve` calls it)
  - replaces: —

Not chunked in Phase 2, left for the session: "Push with `git push --force-with-lease`…" and the Phase review checkbox.

- **3.1** (dsp-repository) The committed curation tests of 0803.
  - files: `areas/access/sync/store/tests/committed_0803.rs`
  - depends_on: —
  - checkboxes: "Run `git rev-parse HEAD`…", "Extend `committed_0803.rs`'s module doc…", the fourteen
    "Write a committed test: …" items
  - acceptance: every count and literal is the plan's *Facts*; each test fails on a mutated copy of the file or of the
    served curation; `just check && just test` pass
  - context: plan § Facts, § Incunabula's file; `tests/committed_0803.rs:1-80`
  - commit: `fixup!` of 3cb35391
  - replaces: —
- **3.2** (dsp-repository) Curation in the two `CONTEXT.md` files and the crate table.
  - files: `areas/access/cpe/CONTEXT.md`, `areas/access/sync/CONTEXT.md`, `docs/src/repo_structure.md`, and any file the
    plan's `grep` sweep finds stale
  - depends_on: —
  - checkboxes: the four `areas/access/cpe/CONTEXT.md` items, the five `areas/access/sync/CONTEXT.md` items, the two
    `docs/src/repo_structure.md` rows, "Read every hit of `grep -rnE …`"
  - acceptance: no hit of the plan's grep says the snapshot holds archive facts only or that `sync` holds or reads the
    snapshot file alone; `just check` passes
  - context: plan Phase 3 checklist; `areas/access/cpe/CONTEXT.md:15`, `:27`, `:33-50`; `areas/access/sync/CONTEXT.md:3-36`
  - commit: `fixup!` of 3cb35391
  - replaces: —
- **3.3** (dsp-repository) `ARCH-MAP.md`'s `areas/access/cpe` and `areas/access/sync` entries, with `dune:dune-map`.
  - files: `ARCH-MAP.md`
  - depends_on: 3.1, 3.2 (the fingerprints cover the entries' paths)
  - checkboxes: the two "Refresh `ARCH-MAP.md`'s … entry with `dune:dune-map`" items, "Run `just check && just test`",
    "Amend Phase 3's files…" (read as: fixup commits, per the brief)
  - acceptance: the two checkboxes' "Done when" lists; the `areas/access/sync` Fingerprint is no longer `ec1861e4ca2c`
  - context: `ARCH-MAP.md`, entries `areas/access/cpe` and `areas/access/sync`
  - commit: `fixup!` of 3cb35391
  - replaces: —

Not chunked in Phase 3, left for the session: the `just commit-lint` checkbox (it cannot pass while `fixup!` subjects
exist), "Push with `git push --force-with-lease`…", "Edit the body of PR #468…" and the Phase review checkbox.

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 3cb35391 | ADR-0010 (proposed) and its row in `docs/src/decisions.md`; phase base is 6f27df56 | none |
| 1.2 | dsp-repository | complete | 252907ab | `CuratedValue`, `ProjectSnapshot.curation`, `contract::is_curation_name`, docs, and `curation` at the four literal sites; a `fixup!` of 3cb35391 | none |
| 1.3 | dsp-repository | complete | 1a50639b | `DanglingCuration`, `DuplicateCuration` and `MalformedCuration` with `Display`, the checks in `violations`, 14 tests and the module doc; a `fixup!` of 3cb35391; `just check && just test` pass | none |
| 1.4 | dsp-repository | complete | 85c8ce52 | Review fix, round 1: ADR-0010 states its departure from ADR-0006 (a pointing amendment, no `superseded by`); the fake's fixture doc reworded; a `fixup!` of 3cb35391 | none |
| 1.5 | dsp-repository | complete | ec3cbd7a | Review fix, round 2: ADR-0010's last Considered Option and its Consequences agree on ADR-0006, and the amendment's content on acceptance is stated; a `fixup!` of 3cb35391 | none |
| 2.1 | dsp-repository | complete | 3442e72a | `0803-curation.csv` derived from the incubator at `6e8e4063` (sources unchanged on `origin/main`; 138 lines, 341 values on 137 resources, per-key counts equal the plan's Facts), its `PROVENANCE` section and the `eol=lf` attribute; a `fixup!` of 3cb35391; phase base is af5c2e25 | none |
| 2.2 | dsp-repository | complete | ba3b6838 | `InvalidCuration` (13 variants), `CurationFault` and `SnapshotError::Curation` in `error.rs`, re-exported from `lib.rs`, with the display test; a `fixup!` of 3cb35391 | none |
| 2.3 | dsp-repository | complete | d1a04371 | `curation.rs` (`parse`, a hand-written strict reader), `curation_tests.rs` (17 table tests, one row beyond the plan: `DuplicateRow` before `PaddedValue`), `serve` reading `<shortcode>-curation.csv`, six file-level tests, `write_0803` writing both files, crate docs; 19 mutations of the reader each fail a test; the key `! grep` exits 0; `just check && just test` pass; a `fixup!` of 3cb35391 | none |
| 2.4 | dsp-repository | complete | 458e1c0a | Review fix, round 1: `test_source_outside_tests_names_no_committed_curation_key` holds ADR-0010's rule mechanically (a canary `"keep"` in `curation.rs` fails it); the `Read`-error tests share `expect_unavailable` and `assert_unreadable`; a `fixup!` of 3cb35391 | none |
| 2.5 | dsp-repository | complete | eeb7f7fe | Review fix, round 1: `PROVENANCE` and the crate doc state up front that the curation file is hand-authored and never regenerated, and that a regenerated `0803.nq` that drops a curated resource refuses the project; "snapshot file" where the crate doc and `SnapshotError::Contract` mean it alone; a `fixup!` of 3cb35391 | none |
| 2.6 | dsp-repository | complete | 930a4bfb | Review fix, round 2: the curation-key test's doc states that it reads every `KNOWN` file and is a text check of string literals; its failure message names where a key's rule belongs and how to treat a word that only collides; a `fixup!` of 3cb35391 | none |
| 3.1 | dsp-repository | complete | 4ce04e72 | Fourteen `test_committed_0803_curation_…` tests in `tests/committed_0803.rs` with the plan's literal counts and values, and the module doc's paragraph on curation expectations; 16 one-cell mutations of `0803-curation.csv` each fail at least one of them, and `values.reverse()` for `values.sort()` in `curation.rs` fails the sorted test; `just check && just test` pass; a `fixup!` of 3cb35391; phase base is 5b9974f4 | none |
| 3.2 | dsp-repository | complete | 9b24e826 | **Curation** in `areas/access/cpe/CONTEXT.md` (term, Project snapshot, a row and a lead sentence of "What the port serves"); **Committed curation**, the intro, Committed snapshot, Known project and `LiveArchiveProjection` in `areas/access/sync/CONTEXT.md`; the two crate rows of `docs/src/repo_structure.md`; from the grep sweep, `areas/access/CONTEXT.md:3` and, not a hit but the same statement, the Access Area bullet of the root `CONTEXT.md`; the two `ARCH-MAP.md` hits are left to 3.3; `just check` passes; a `fixup!` of 3cb35391 | none |
| 3.3 | dsp-repository | complete | 61fb9894 | `ARCH-MAP.md`, with `dune:dune-map` in `update` mode on two entries: `areas/access/cpe` (Purpose, Key entities, kit, a boundary rule; still `status: planned`, `Fingerprint: none`) and `areas/access/sync` (Paths, Purpose, Key entities, kit, Boundary rules, Durable state; Fingerprint `ec1861e4ca2c` → `9adf91884314`); frontmatter `date` 2026-10-09; the skill's overwrite confirmation was taken from the plan's checkboxes, no user being reachable; `just check && just test` pass; a `fixup!` of 3cb35391 | none |
| 3.4 | dsp-repository | complete | 1f2bb9c0 | Review fix, round 1: ADR-0010's `Enforced by:` line names `test_source_outside_tests_names_no_committed_curation_key` as static analysis for committed keys, as `ARCH-MAP.md` does; `ARCH-MAP.md`'s Overview and the tree of `docs/src/repo_structure.md` name the curation beside the snapshot; **Committed curation** says that `.gitattributes` sets LF only and the reader enforces UTF-8 without a BOM, and names comment columns; the map tags the file-and-expectations rule **review**; the name test prints the misplaced IRIs; the sync Fingerprint is `9686ad16817f`; `just check && just test` pass; a `fixup!` of 3cb35391 | none |
| 3.5 | dsp-repository | complete | ae607324 | Final-review fix, round 1 (Warning): the doc on `curation::parse` states the fault order the code has: a refused character or misplaced quote first, then the header's columns, then for a row blank line, cell count, unknown IRI, second row, and only then a padded value, leftmost first; no behaviour change; three rows in `test_curation_parse_returns_the_first_fault_only` pin what no test held (a stray quote before cell count and IRI, a control character before a duplicate row, the leftmost of two padded values); the module doc and `lib.rs` restate no order; a `fixup!` of 3cb35391 | none |
| 3.6 | dsp-repository | complete | 1bc670c9 | Final-review fix, round 1 (suggestion): `InvalidCuration::DuplicateRow` prints its IRI with `{iri:?}`, as `UnknownResource` does; no test asserts either message (the one display test uses `BlankLine`), so no expectation changed; a `fixup!` of 3cb35391 | none |
| 3.7 | dsp-repository | complete | 984ee4a1 | Final-review fix, round 1 (suggestion): `CuratedValue.text`'s doc in `cpe-ports` names no adapter: the contract promises no more about the text, an adapter may be stricter; a `fixup!` of 3cb35391 | none |
| 3.8 | dsp-repository | complete | 8820328c | Final-review fix, round 1 (suggestion): the `.dockerignore` comment on `areas/access/sync/data/` names the curation beside the snapshot; a `fixup!` of 3cb35391 | none |
| 3.9 | dsp-repository | complete | 308b5d4b | Final-review fix, round 1 (suggestion): `ARCH-MAP.md`'s Durable state of `areas/access/sync` and **Committed curation** in `areas/access/sync/CONTEXT.md` say that until DEV-7402 lands the file is a hand-synced copy of the incubator's CSV files, which stay authoritative; the sync Fingerprint is `9686ad16817f` → `cf39e565315e`, recomputed from the index because this round's edits to `curation.rs`, `curation_tests.rs`, `error.rs` and `CONTEXT.md` lie under the component's glob; `just check && just test` pass over the round's whole tree; a `fixup!` of 3cb35391 | none |

## Deferrals

## Side findings

- Phase 1 review, suggestions not applied:
  - `CuratedValue::text`'s doc (`snapshot.rs`) names `sync-store`'s padding and control-character rule, a rule of another
    crate; the wording is the plan's. Two reviewers would keep only "the contract refuses an empty text; a reader may
    refuse more".
  - `ProjectSnapshot.curation`'s doc says "(ADR-0010, proposed; DEV-7488)"; the status word goes stale on acceptance, so
    H3's follow-up PR has to touch it.
  - `curation_name` renders `key@lang`, so a malformed key `a@b` without a language prints like key `a` in language `b`;
    a diagnostic only.
  - No `violations` test pins a value that is both dangling and duplicated or malformed, nor a non-empty bad language
    such as `DE`; the code handles both and `is_curation_name`'s own test covers the predicate.
  - `test_violations_uppercase_curation_key_reports_malformed_curation` takes the same branch as the empty-key test, and
    `test_curated_values_sort_by_resource_key_and_language` pins a derive; both are named by the plan and stay.
- Phase 1 review, refuted: ADR-0010's `Enforced by:` line names `sync-store`'s tests, which Phases 2 and 3 of the same PR
  add (the missing-file test and the committed-curation tests are on their checklists). Before the PR merges, check that
  they exist. `mapping::map` returning `curation: Vec::new()` is the plan's design: `serve` sets the field in Phase 2.
- An "adapter names no project's key" check exists only as Phase 2's one-off `! grep` step; a committed test would hold
  the rule ADR-0010 leaves to review.
- The plan has review fixes amended into the feature commit and pushed per phase. The executor may not amend or push, so
  Phase 1 is one `feat` commit plus three `fixup!` commits of it; they need one autosquash before the push.
- Phase 1 review ran two fix rounds, the cap; round 2's diff (ADR-0010 wording only, `1a50639b..ec3cbd7a` holds both
  rounds) was not reviewed again. No Critical was raised in either round.
- Phase 2 review (rust, consistency, simplicity, dune): no Critical. Four Warnings, three fixed in round 1 (2.4, 2.5).
- Phase 2 review, Warning not applied: `curation.rs`'s `//!` block is 15 lines, past the ~12-line trigger. It is the
  format's one definition and a documentation-shaped module doc, which the comment conventions exempt from the
  trigger. The reviewer's alternative is a data-side `areas/access/sync/data/CURATION-FORMAT.md` with a short pointer in
  the module; that is a new document the plan does not have, so it is the owner's call.
- Phase 2 review, suggestions not applied:
  - `read_row` could return `(iri, values)` and read `rows` only, dropping two out-parameters; the curried `fault`
    closure in `parse` could be two plain closures; `is_value_column` could be inlined.
  - `test_snapshot_unknown_shortcode_with_curation_file_present_returns_unknown_project` proves little beyond its
    sibling, since `KNOWN` gates before any read; the plan names it.
  - Unpinned reader behaviour: a quoted `"iri"` header cell is accepted, a quoted `"#x"` is a comment column, and a
    stray quote before a control character on one line reports the quote.
  - `DuplicateRow`'s doc line restates its field; `lang` and `name` as keys sit beside the `key@lang` column syntax
    and could be misread as reserved (a line in `PROVENANCE` would say they are ordinary keys).
  - The `#note` cells cite incubator documents (`D7`, `DATA_ISSUES A6`) that this repository does not hold.
  - `tests/committed_0803.rs`'s module doc and `snapshot()` doc do not mention the curation file; Phase 3 extends that
    module doc.
- The incubator's `origin/main` was at c159e417 when Phase 2 compared the curation sources with the pin `6e8e4063`:
  the three CSV files are unchanged and `translate.py` holds both strip constants.
- Phase 2 review ran two fix rounds, the cap. Round 1's re-review (consistency, simplicity, dune) raised no Critical and
  two dune Warnings on the new key test's message and stated limits, fixed in round 2 (2.6). Round 2's diff, a doc
  comment and an assert message in `snapshot_tests.rs`, was not reviewed again.
- The key test is a text gate: a key assembled with `format!` or `concat!`, or placed in a file named `*_tests.rs`,
  passes it. A doc example that quotes a common key (`"name"`, `"lang"`) in non-test source fails it.
- Round 1's re-review, suggestion not applied: `lib.rs`'s crate-doc paragraph on the curation file has one short line
  after the inserted sentence (`cargo fmt` does not re-flow it).
- Phase 3 review (rust, consistency, simplicity, dune): no Critical. Verified Warnings fixed in round 1 (3.4).
- Phase 3 review, Warning not applied: the committed tests pin counts, not every value. Which Book has which
  `office`, 18 of the 19 `date_display` texts, the German teasers and three English ones, which annotations have
  `keep` = `no`, and which have `lang` = `de` are unpinned, so a swap inside one class passes. The tests assert what
  the plan's checkboxes name, and the plan's *Facts* hold no per-resource literal for these; taking them from the
  committed file would check the file against itself. Pinning them needs literals from the incubator's CSVs, which is
  the owner's call.
- Phase 3 review, suggestions not applied:
  - A slug swap between two Books whose cover Pages share a label (`brandan` and `zeitgloecklein-1490`, both `a1r`)
    passes the slug and the cover-label tests.
  - `committed_0803.rs` indexes maps with `[]` (`slugs[book]`, `dates[…]`), whose panic names no IRI.
  - `curation.len() == 341` and `names.len() == 35` follow from the per-key count map; both stay as headlines.
  - `ARCH-MAP.md` states "a key means nothing to the port" in the `areas/access/cpe` Purpose, Key entities and
    Boundary rules; the Purpose wording is the plan's.
  - `committed_0803.rs:1-3`, not touched by this phase, names the 2026-07-19 stage dump, while `PROVENANCE` leads with
    the prod dump of 2026-10-07 and says the stage dump gave the same bytes.
- `dune:dune-map` asks for a confirmation before it overwrites a map. Phase 3 took it from the plan's two checkboxes
  ("this checkbox authorises the overwrite"), since the executor cannot reach the user. Only the two named entries, one
  Overview clause and the frontmatter `date` changed; no other component was re-explored or re-fingerprinted.
- ADR-0010's `Enforced by:` line names no test by name except the key test added in 3.4; the tests it refers to
  exist: `curation_tests.rs`, the `test_snapshot_…curation…` tests and `tests/committed_0803.rs`.
- Phase 3 review ran one fix round. Its re-review (consistency, simplicity, dune) over `61fb9894..1f2bb9c0` raised no Critical
  and no Warning; two edited `ARCH-MAP.md` lines run past the file's wrap width, a cosmetic point left as it is.
