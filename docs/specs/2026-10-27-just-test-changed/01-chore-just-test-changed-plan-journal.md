---
plan: docs/specs/2026-10-27-just-test-changed/01-chore-just-test-changed-plan.md
target_repo: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7615
base_commit: 4dfb0c56084f6ea9c5da80886cafc8a158cb0dd8
branch: chore/dev-7615-just-test-changed
started: 2026-10-27
problem: >
  `just test` runs the whole workspace and every script test whatever changed (190 s after a one-file touch on the reference Mac, 96 s of it script tests), so area-local work pays for every other area's suites. `just test-changed [base]` maps the changed paths to their packages plus reverse dependencies and runs `just test`'s commands restricted to them, falling back to a full run when in doubt.
status: complete
---

# Execution Journal: 01-chore-just-test-changed-plan

merge_strategy: rebase (stacked on #541, `chore/dev-7614-packed-debuginfo-on-macos`). Chunk commits after the first land as `fixup!` of it, so the PR keeps two commits: the plan and the recipe.

## Chunk queue

| id | files | depends_on | checkboxes | acceptance | context | replaces |
|----|-------|------------|------------|------------|---------|----------|
| 1 | .github/scripts/test-changed.sh, .github/scripts/test-changed.test.sh | — | "Add test-changed.sh …", "Add test-changed.test.sh …" (all assertions but `--audit`) | test file passes; dry-run selections match rules 1-6, closure, base fallback | plan § Selection rules, § Closure and run; style owner: .github/scripts/check-capability-deps.test.sh; justfile:191-207 `test`/`test-scripts` | — |
| 2 | .github/scripts/test-changed.sh, .github/scripts/test-changed.test.sh | 1 | "Add the --audit mode …", "Run --audit; prints nothing", "Delete each EXTRA_EDGES line …", the `--audit` test assertion | `--audit` silent on the tree, each removed edge reappears, test cases pass | plan § Keeping rule 2 complete | — |
| 3 | justfile, docs/src/workflows.md, CLAUDE.md, .claude/settings.json | 1 | recipe, workflows table, CLAUDE.md line, settings allow-list | `just test-changed` runs the script; docs list it | justfile:191 `test`; docs/src/workflows.md:13-40 recipe table | — |
| 4 | — | 2, 3 | "Run just check && just test" | both pass | — | — |
| 5 | — | 4 | "Take the timings …" | four timings and selections recorded for the PR Test Plan | plan § Implementation Approach last item | — |

## Chunks

| id | status | commit(s) | summary | blocker |
|----|--------|-----------|---------|---------|
| 1 | complete | e6a96d6b | test-changed.sh (rules 1-6, rdeps closure via jq, base fallback, dry-run) and its fixture test, 37 cases, passing under bash 5.3 and 3.2 | none |
| 3 | complete | fec32486 (fixup) | `test-changed` recipe, `test` comment points at the script, workflows.md row, CLAUDE.md line, settings allow-list | none |
| 2 | complete | 31696618 (fixup) | `--audit`: perl -0777 matcher over `*.rs` (include_*!, CARGO_MANIFEST_DIR joins, root-relative literals), resolves and prints reads not covered by a Cargo edge or EXTRA_EDGES; silent on the tree, each removed entry reappears; 46 tests pass under bash 5.3 and 3.2 (3.2 outside the sandbox) | none |
| 4 | complete | — | `just check` passes (sandboxed); `just test` passes outside the sandbox (inside, dsp-cli's wiremock tests cannot bind a port: EPERM) | none |
| 5 | complete | — | Timings outside the sandbox, warm target, #541 in effect, base `HEAD`: editor-core edit `just test` 186 s / `just test-changed` 28 s (editor-core + rdeps deposit-server, editor-pull-requests, editor-store, editor-web); Cargo.lock blank line `just test` 167 s / `just test-changed` 129 s (`full run: Cargo.lock`; run 3 also recompiled editor-core after the revert, run 4 compiled nothing) | none |
| R1 | complete | 6a1ef792 (fixup) | Review fixes: rule 5 matches documentation by name (data markdown that tests read falls to rule 6), GIT_* unset in the test, `--audit` fails on missing perl, producer and jq failures run in full, `$base` recipe parameter (no shell splice under the allow-listed prefix), dependency lists, audit pointer in workflows.md; 52 tests pass under bash 5.3 and 3.2 | none |

## Deferrals

- Review leftovers, recorded in the PR body: wire `--audit` into `just check` (it stays a manual run, per the plan); the dev-feature package list lives in both the justfile `test` recipe and the script, linked by a comment only; the audit's root-literal prefixes (`areas|modules|shared|docs`) are fixed, so a new top-level directory such as `mosaic/` after ADR-0002's move is invisible to it; a `README.md`/`CLAUDE.md`/`CONTEXT.md` inside a data directory that tests read still selects nothing.

## Deviations

- none

## Side findings

- Against `origin/main` the stacked branch always runs in full: #541 changes `.cargo/config.toml` and this PR the `justfile`, both rule 6. The timing step uses base `HEAD`.
- Under the Claude Code sandbox, macOS `/bin/bash` 3.2 cannot write here-string temp files to `/tmp` and drops `sh-thd-*` files in the cwd, which then count as untracked changes. `just test-changed` calls the `bash` on PATH; sandbox-only.

- Shipped as PR #542: the chunk commits above were autosquashed into two, the plan and the recipe; CI green (`test`, `check`, Commit Hygiene) on `4f35760f`, whose code tree the final head keeps unchanged.

## Closeout
- root_cause: `just test` has no notion of what changed: it runs every workspace package's tests and every script test, so area-local work pays for every other area's suites (190 s after a one-file touch, 96 s of it script tests).
- investigation: The plan's rule set held against the code (the four cross-package reads, `deposit-server` serving editor `public/`, no non-Deposit crate depending on a Deposit crate). Review found rule 5's `*.md` pattern matching inside unowned data trees (`areas/access/media/data/0803/narrative/*.md`, `areas/access/cpe/projects/incunabula/*.md`) that `cpe-server`'s tests read, so a narrative edit printed "nothing to test"; the audit missed it because it compares directory targets, not the files under them. Inside the Claude Code sandbox, dsp-cli's wiremock tests cannot bind a port and macOS `/bin/bash` 3.2 drops here-string temp files into the cwd, so `just test` and the bash 3.2 pass ran outside it.
- solution: `.github/scripts/test-changed.sh` classifies each changed path (merge-base diff plus untracked files) by six ordered rules, adds reverse dependencies from `cargo metadata`'s path edges, and runs `just test`'s commands restricted to the selection, falling back to `just test` for anything unknown or on any failure; `--audit` scans `*.rs` with a whole-file perl matcher for cross-package reads that `EXTRA_EDGES` misses. Rule 5 names documentation files instead of `*.md`. Exposed as `just test-changed $base`, documented in `docs/src/workflows.md` and `CLAUDE.md`.
- prevention: `test-changed.test.sh` (52 cases, throwaway Cargo workspaces, run by `just test-scripts` and CI) covers every rule, the closure, the fallbacks and the audit's three read forms; `--audit` is silent on today's tree and reports each of the four reads when its `EXTRA_EDGES` line is removed. Anti-pattern: a catch-all glob such as `*.md` in a "no tests" table, which in a bash `case` also matches inside data directories. CI keeps running the full suite.
