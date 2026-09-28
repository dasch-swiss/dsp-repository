---
plan: docs/specs/2026-09-28-move-dpe-to-access-area/01-refactor-move-dpe-to-access-area-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/move-dpe-to-access-area
base_commit: 7ab01642
branch: worktree-move-dpe-to-access-area
started: 2026-09-28
problem: >
  ADR-0002 removes `modules/`. The editor has moved to `areas/deposit/editor`; DPE is the remaining
  application under `modules/dpe`. Moving it to `areas/access/dpe` (DEV-7396) completes the Access Area
  half of ADR-0002 and unblocks the Access Area composition root `areas/access/server` (DEV-7397), which
  composes area crates, not `modules/` crates.
status: complete
---

# Execution Journal: 01-refactor-move-dpe-to-access-area-plan

The tree does not build between the move and the repointing, so every chunk lands in one refactor commit
(the plan's own rule), together with the plan and this journal. Chunk rows name it "refactor commit": a SHA written into the commit that carries it cannot be its own.

## Chunk queue

| id | files | depends_on | checkboxes | acceptance | context | replaces |
|----|-------|------------|------------|------------|---------|----------|
| 1 | — | — | "Baseline before the move" | app.css SHA-256 recorded in `.claude/tmp/` | justfile `css` recipe | — |
| 2 | modules/dpe → areas/access/dpe | 1 | "Move" (3 items) | `modules/dpe` gone, `areas/access/CONTEXT.md` present | — | — |
| 3 | 57 files naming `modules/dpe` | 2 | "Repo-root-relative" section | `git grep modules/dpe` outside specs/CHANGELOG/ADRs is empty | sed-style rewrite, `CONTEXT.md` special-cased | — |
| 4 | DPE Cargo.toml ×5, 3 `include_str!`, style/main.css, playwright.config.ts | 2 | "Relative paths out of DPE" section | `cargo check`, fuzz `cargo metadata`, `just css` byte-identical | — | — |
| 5 | .github/scripts/check-shared-paths.test.sh | 3 | the two fixture items | both roots still covered; 8/8 pass | make_repo, cases 2, 3, 6 | — |
| 6 | justfile, repo_structure.md, CLAUDE.md, ADR-0002, ADR-0007 | 3 | prose items, ADR amendment | hand-written text reads right | — | — |
| 7 | ARCH-MAP.md | 3 | "ARCH-MAP.md" item | `dune:dune-map` update, schema 2 (user decision) | DPE/editor entries re-verified | — |
| 8 | — | 1–7 | "Verification" section | `just check`, `just test`, browser check, review | — | — |

## Chunks

| id | status | commit(s) | summary | blocker |
|----|--------|-----------|---------|---------|
| 1 | complete | — (no code) | baseline `c8a3acf7…` in `.claude/tmp/dpe-app-css-baseline.sha256` | none |
| 2 | complete | refactor commit | `git mv`; ignored build output moved with the directory, stale `app*.css` deleted before rebuild | none |
| 3 | complete | refactor commit | scripted rewrite of 57 files; also `areas/deposit/editor/CONTEXT.md`, which the plan's list missed | none |
| 4 | complete | refactor commit | one extra `../` per path; Mosaic is `../../../../modules/mosaic`, not the plan's `../../../modules/mosaic`; CSS byte-identical | none |
| 5 | complete | refactor commit | make_repo carries `modules/widget` and `areas/access/dpe`; case 2 targets the neutral module, cases 3 and 6 the area | none |
| 6 | complete | refactor commit | verify-checksums comment, repo_structure tree and crate table, CLAUDE.md pointer, ADR-0002 line 24 + amendment, ADR-0007 bullet | none |
| 7 | complete | refactor commit | `dune:dune-map` update: DPE and editor entries re-verified by Explore agents (no corrections), `areas/access/CONTEXT.md` added to DPE's Paths, schema 1 → 2 with a fingerprint per component (user decision) | none |
| 8 | complete | refactor commit | grep gate empty; CSS byte-identical; fuzz `cargo metadata` resolves; `validate-data`, `lint-e2e`, `just check`, `just test` pass; browser check on `/dpe/projects`, `/dpe/projects/0803`, `/dpe/oai?verb=Identify` (all assets 200, styled render, no page console errors); review of 6 reviewers, 2 verified findings fixed | none |

## Deferrals

- Browser verification (`eng:test-browser`): run unattended at the quality check (intake decision), not deferred.
- Push, draft PR and CI: done after the user confirmed (PR #444). The branch was squashed to one commit first, at
  the user's request, so the plan and this journal land in the refactor commit.
- `gh workflow run fuzz.yml --ref worktree-move-dpe-to-access-area`: dispatched after the user confirmed (run
  36417680916, on the commit before the ADR status changes; the fuzz crate is identical). `query_params` and
  `tab_validation`, the two targets in the workflow's matrix, built and ran from `areas/access/dpe/server`. No
  deferral remains.

## Side findings

- The plan's Mosaic spelling `../../../modules/mosaic/...` is one level short: from
  `areas/access/dpe/<crate>/` the repo root is four levels up. `just css` and `cargo check` both failed on it.
- `areas/access/dpe/server/fuzz/Cargo.lock` records the workspace crates at 0.8.5 while they are at 0.8.6;
  `cargo metadata` rewrites it. Pre-existing and out of scope, so the lock was restored to main's copy.
- The plan listed six violation-fixture lines in `check-shared-paths.test.sh`; case 6 (line 115) also named
  `modules/dpe` and moved too.

- Review (consistency, devops, dune, code-simplicity, rust, ivan; accessibility and security not dispatched: no
  markup change, the editor edits are comments and one constant). Two stale-prose findings, both verified by reading
  the lines and fixed before the commit: ARCH-MAP's Overview and `docs/src/repo_structure.md`'s first paragraph still
  put DPE under `modules/`. The grep gate could not catch them: they name DPE without the `modules/dpe` string.
- Pre-existing, out of scope (DevOps review): `.dockerignore` re-includes no `areas/deposit/editor/Dockerfile`
  (the editor builds from a staging directory, so likely harmless); `fuzz.yml`'s matrix runs 2 of the fuzz
  crate's 5 targets. ARCH-MAP's banned-constructs row still says `modules/<service>/` only, although the gate
  covers `areas/` too since the editor move.
- CI on #444: every check green except `a11y / editor / WCAG 2.1 AA + no-JS` on the first run (72 passed, 2 flaky,
  1 failed: a 30 s timeout in `collection.spec.ts:185`, after its serial predecessors `:68` and `:81` had needed
  retries). The suite reads DPE's data through the moved `EDITOR_DATA_DIR`, so the failure was checked rather than
  assumed: a re-run of the failed job on the same commit passed 75/75 with no retries. A flake, not the move.
  `a11y-dpe`, `scout-dpe` and the DPE preview triggered on the new path and passed; the preview served
  `/dpe/projects`, `/dpe/projects/0803`, `/dpe/oai?verb=Identify` and a cover image with 200.
- ADR-0007 moved from `proposed` to `accepted` in this PR at the user's request (DEV-7393 was already Done and its
  PR #440 merged, but the file still said `proposed`). DEV-7397 builds on it. ADR-0008 was accepted in the same PR, also at the user's request.
- `eng:test-browser` resolved only to its command wrapper, which delegates back to itself; the browser check ran
  directly through the Chrome tools instead.

## Closeout

- root_cause: ADR-0002 replaces `modules/` with one directory per area. After the editor's move, DPE was the last
  application under `modules/`, and the Access Area composition root (DEV-7397) cannot compose it from there.
- investigation: the plan's inventory was close but not exact. Its Mosaic spelling was one `../` short
  (`cargo check` and `just css` caught it), it missed `areas/deposit/editor/CONTEXT.md` and one gate fixture, and
  its grep gate cannot see prose that names DPE's location without the `modules/dpe` string; review found two such
  sentences. `cargo metadata` on the fuzz workspace rewrites a lock file that is stale on main, which was restored.
  The one red CI job was a flake, shown by a clean re-run on the same commit.
- solution: one `git mv`, `CONTEXT.md` moved up to the area level, then every reference repointed in the same commit:
  one extra `../` for paths out of DPE, `modules/dpe` → `areas/access/dpe` for repo-root-relative strings, one line
  per toolchain on the editor side. ARCH-MAP moved to schema 2 with fingerprints (user decision); ADR-0002 gained a
  dated amendment.
- prevention: the CSS byte-identity check and the grep gate caught the silent-failure paths (Tailwind `@source`,
  CI filters, runtime defaults). The grep gate needs a companion for prose: search for the application's name next
  to the old root (`DPE … modules/`), not only for the path string. `check-shared-paths.test.sh` now covers both
  roots with fixtures that do not depend on which applications still live under `modules/`.
