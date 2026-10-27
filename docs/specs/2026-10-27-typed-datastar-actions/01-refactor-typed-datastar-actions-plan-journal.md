---
plan: docs/specs/2026-10-27-typed-datastar-actions/01-refactor-typed-datastar-actions-plan.md
target_repo: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository
base_commit: bb150399
branch: refactor/dev-7369-typed-datastar-actions
started: 2026-10-27
problem: >
  Datastar attributes and action expressions are plain strings in Maud markup, so a misspelled
  attribute, modifier or action renders fine and leaves the control inert. The only guard is a
  git-grep blacklist that grows one regex per shipped mistake. Actions move to a typed builder in a
  new shared-datastar crate, and attribute names (Maud literals) are held to a whitelist grammar by
  check-datastar-attributes.sh.
status: complete
---

# Execution Journal: 01-refactor-typed-datastar-actions-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | bb150399 | refactor/dev-7369-typed-datastar-actions | rebase | shipped | https://github.com/dasch-swiss/dsp-repository/pull/547 |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@bb150399 | 1 |
| 2 | reviewed | dsp-repository@bb150399 | 1 |
| 3 | reviewed | dsp-repository@bb150399 | 1 |
| 4 | reviewed | dsp-repository@bb150399 | 1 |

## Chunk queue

Phases overlap (workspace `CLAUDE.md` → Overlap Phases): commits land in plan order, one per phase; work runs in waves. Wave A: 1.1, 4.1, 1.3. Wave B (after 1.1): 1.2, 2.1, 3.1. Wave C: 4.2. Shared files (`Cargo.lock`, `ARCH-MAP.md`, `docs/src/repo_structure.md`) are edited by the session and split per phase at commit time.

| id | files | depends_on | checkboxes | acceptance | replaces |
|----|-------|------------|------------|------------|----------|
| 1.1 | shared/datastar/**, Cargo.toml | — | crate, unit tests, `&amp;` test | `cargo test -p shared-datastar` green, every output-table row pinned | — |
| 1.2 | dpe-web tabs mod.rs, project_search_input.rs, dpe-web Cargo.toml | 1.1 | two DPE migrations, dpe-web dep | no raw action in dpe-web production code; HTML unchanged | — |
| 1.3 | shared/README.md, CONTEXT.md, git-conventions.md, CONVENTIONS.md, eng.yaml, dpe project_structure.md, dpe architecture.md, dpe CLAUDE.md | — | Phase 1 doc checkboxes except ARCH-MAP, repo_structure | docs name the crate and the builder rule | — |
| 2.1 | editor section.rs, review.rs, entity/mod.rs, editor-web Cargo.toml, editor CLAUDE.md, docs/src/editor/rendering.md | 1.1 | Phase 2 except ARCH-MAP | no raw action in editor-web production code; HTML unchanged | — |
| 3.1 | cpe-components search_bar/compound_viewer/resource_list/facet_panel, cpe-core util.rs, media.rs, cpe-components Cargo.toml, ADR-0007 | 1.1 | Phase 3 except repo_structure/ARCH-MAP | no raw action and no `encode_get_arg` outside docs/specs; HTML unchanged | — |
| 4.1 | .github/scripts/check-datastar-*.sh, justfile, check-views-no-axum.test.sh | — | gate rewrite, test, recipe, comment | test script green; gate green on the migrated tree | — |
| 4.2 | docs/adr/0004, docs/adr/0002 | 4.1 | ADR rewrites | ADRs name the new gate and enforcement | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | phase-1 commit | `shared-datastar` crate, 7 tests after trimming, clippy and fmt clean | none |
| 1.2 | dsp-repository | complete | phase-1 commit | dpe-web tab link and search input through `datastar::get`; 130 tests, clippy clean | none |
| 1.3 | dsp-repository | complete | phase-1 commit | Phase 1 docs (shared/README, CONTEXT, scopes, eng.yaml, DPE docs, DPE CLAUDE.md); session added repo_structure row and the ARCH-MAP `shared/datastar` entry | none |
| 2.1 | dsp-repository | complete | phase-2 commit | editor's four `@post` actions through `datastar::post`; new test pins the entity form's action; editor CLAUDE.md and rendering.md; 315 tests, clippy clean | none |
| 3.1 | dsp-repository | complete | phase-3 commit | CPE's 11 actions through `datastar::get`, `encode_get_arg` deleted, ADR-0007 clause; cpe-components/core/web/server tests and clippy pass, no snapshot changed | none |
| 4.1 | dsp-repository | complete | phase-4 commit | `check-datastar-attributes.sh` and its test (45 cases after review), recipe renamed; rule 1 clean on the tree, rule 2 flagged the 17 baseline lines before migration | none |
| 4.2 | dsp-repository | complete | phase-4 commit | ADR-0004 Consequences and Enforced-by rewritten, ADR-0002 gate name | none |

## Deferrals

- Browser verification (`eng:test-browser`, eng.yaml `offer: ask`): skipped at intake; byte-identical HTML is pinned by the snapshots and contains-assertions.

## Deviations

- Gate H1 resolved by the stack, not by a merge: #547 is stacked on fix PR #546, so Phase 4's first checkbox (rebase onto `origin/main`) is moot; the rebase happens when #546 merges and #547 retargets. Owner confirmed at intake 2026-10-27.
- The plan names the refactor PR #545; it was closed and replaced by #547.

## Side findings

- ADR-0007 says `dsp-incubator/cpe` vendors `cpe-components`, `cpe-core`, `cpe-image-viewer`, `cpe-ports` and `sync-store`; the vendored `cpe-components` now also needs `shared-datastar`. Left unchanged (DEV-7408 scope); flag in the PR's Leftovers.
- The gate's grammar admits `.trailing` on `throttle` only, not on `debounce`; whitelist grows with the first use.
- Review (7 reviewers, one fan-out over the whole diff): fixed the gate's region rule (an out-of-line `#[cfg(test)] mod x;` ended scanning of the rest of 10 files), anchored the test-file skip (`contests.rs` was skipped), extend-here pointers in the gate's messages, `plus_js` takes `&'static str`, stale doc lists (ADR-0002, editor and DPE architecture, REVIEW.md), ADR-0007's vendoring line. Not fixed: whitelist narrower than the bundle (by design, grows with first use); line terminators and U+2028 in a path pass unencoded (inert control, not injection, unchanged from `encode_get_arg`; follow-up candidate); line-split `@get` bypasses (the gate targets mistakes, not adversaries).

## Closeout

- root_cause: Datastar reads plugin, key and modifiers out of the attribute name and the action out of its value, and both were hand-written strings in Maud markup. Maud accepts any spelling, a wrong one renders and leaves the control inert, and snapshot tests assert presence, not meaning. The only guard was a blacklist gate that grew one regex per mistake already shipped.
- investigation: Maud 0.27 takes only literal tokens as attribute names, so names cannot be typed; the value is a `Display` splice, so actions can be. The `datastar` crate is server-side SSE only, nothing to reuse. CPE already encoded `'`, `` ` `` and `\` (`encode_get_arg`); DPE and the editor did not. Review found the first gate's region rule ended a file's scan at an out-of-line `#[cfg(test)] mod x;` (10 files), and CI's `just check` stopped at `check-shared-paths` (the crate's doc comment named a service path), hiding a `maudfmt` failure behind it.
- solution: `shared-datastar` builds `@get`/`@post` (`Get::retry_never`, `Get::plus_js` taking `&'static str`, `Post::submitter_first`, `Post::form`), encodes the path, and relies on Maud's splice escaping. DPE (2), editor (4) and CPE (11) actions moved to it with rendered HTML unchanged; `encode_get_arg` went. `check-datastar-attributes.sh` replaces the blacklist: a whitelist grammar for every Datastar-shaped name and no raw action in production code. ADR-0004, ADR-0007, ADR-0002, ARCH-MAP and the docs describe it.
- prevention: The crate's tests pin every emitted shape and the Maud escaping; the gate's 45 cases pin grammar, region and file-skip rules; a test now pins the entity form's `formaction` action. A new plugin, modifier or action option is a grammar or crate change with a test, in the change that first needs it. Run `maudfmt` after `rustfmt` and check both, since they can disagree; `just check` stops at its first failing gate, so a red CI step can hide later ones.
