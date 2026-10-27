---
title: "chore(ci): just test-changed, test only the packages a change touches"
type: chore
date: 2026-10-27
author: "Marcin Procyk"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7615
linear_project: Metadata Editor for RDU + depositing projects
---

# chore(ci): just test-changed, test only the packages a change touches

## Overview

Add `just test-changed [base]`: it maps the paths changed since `merge-base(base, HEAD)` (working tree and untracked files included) to workspace packages, adds their reverse dependencies, and runs `just test`'s commands restricted to them; `.github/scripts` tests run only for the scripts that changed; when in doubt it falls back to a full `just test`. CI keeps running the full suite, and `just test` stays the command to run before pushing.

The selection follows the model the established tools share (guppy's `determinator`, Zed's CI, Nx `affected`): a path is owned by its nearest package, a path outside every package forces a full run unless a rule says otherwise, and workspace-wide inputs (`Cargo.lock`, root `Cargo.toml`, `rust-toolchain.toml`, `.cargo/`, `flake.*`, `justfile`) force a full run.

**Delivery: one PR stacked on #541** (DEV-7614, base `chore/dev-7614-packed-debuginfo-on-macos`): this plan, then the recipe, two commits, `allow-many-commits` ticked. The stack exists for the measurement: the issue's timings must be taken with #541's `.cargo/config.toml` in effect. `dsp-repository` is rebase-merge, so once #541 merges, `git rebase --onto origin/main <old #541 tip> chore/dev-7615-just-test-changed`.

**Shelf life:** ADR-0001 moves the build to Bazel, where `bazel test //...` gives cached, affected-only test runs for free ("not scheduled", per the ADR). Until then this is one script plus its test, deletable in one commit.

## Problem Statement / Motivation

`just test` runs the whole workspace and every script test, whatever changed. Baseline from DEV-7614's measurement (#541 Test Plan, `split-debuginfo=off`: `touch shared/metadata/src/lib.rs`, then `just test`): **190 s**. Its log splits into 12 s of compilation (the two `Finished` lines, 9 s + 3 s), 41 s of test execution (sum of `finished in` over the 95 suites, run serially by `cargo test`), and the script tests: `just test-scripts` alone took **96 s** on the same Mac (measured 2026-10-27, inside the Claude Code sandbox).

A Deposit-only change compiles the same crates under either recipe: cargo already rebuilds only what changed, and no crate outside `areas/deposit` depends on a Deposit crate (`cargo metadata --no-deps`). The saving is therefore the run time of non-Deposit suites plus the script tests.

## Proposed Solution

### Selection rules

Every entry in the rule tables is a bash `case` pattern over a repo-root-relative path: a directory is written `dir/*`, a file by its exact path.

1. **Script tests.** `.github/scripts/<name>.sh` or `<name>.test.sh` selects `<name>.test.sh`; a `<name>.sh` without a test prints a warning and selects nothing. Any other path under `.github/scripts/` selects all script tests. (Every `*.test.sh` builds its fixtures in `mktemp -d` and sources only its own script.) Rules 2 to 6 do not apply to these paths.
2. **Extra edges: additive, checked for every path, before rules 3 to 6.** Every matching line adds its packages; the path is then still classified by rules 3 to 6. These are packages that read another package's files, or a file outside every package, without a Cargo edge to it:
   - `areas/access/dpe/server/data/*` → `dpe-core dpe-api-oai dpe-web editor-core`. DPE's published data set: `dpe-core`, `dpe-api-oai` and `dpe-web` read it through `CARGO_MANIFEST_DIR` joins, and `editor-core` through `DPE_DATA_DIR` / `checkout_dpe_data_dir()` (`areas/deposit/editor/core/src/repo_paths.rs`), which `editor-web` and `editor-pull-requests` call and the closure reaches.
   - `areas/access/cpe/server/tests/fixtures/*` → `cpe-core` (`include_str!` in `cpe-core/src/remodel.rs`).
   - `areas/access/cpe/store/schema.sql` → `cpe-store-build` (`include_str!` in `cpe-store-build/src/lib.rs`).
   - `docs/src/cpe/config-language.md` → `cpe-store-build` (`include_str!` in `tests/it/worked_example.rs`, which checks the doc's worked example against `media/data/0803/config.kdl`).
3. **Nearest package** (first of rules 3 to 6 to match wins): the longest manifest-directory prefix from `cargo metadata --no-deps`, matched as `<dir>/` so `areas/access/server/` does not capture `areas/access/cpe/server/`. A member's own `Cargo.toml` is owned here, correctly: its rdeps are what a manifest change affects. `cargo metadata` reads the working tree, so a package deleted or moved since the base leaves its old paths unowned (rule 6, full run).
4. **Unowned, mapped:** `areas/deposit/editor/public/*` and `areas/deposit/editor/style/*` → `deposit-server`, which serves them (`areas/deposit/server/src/config.rs`). Without this, every editor CSS change would force a full run.
5. **Unowned, no tests:** `docs/*`, unowned `*.md`, `.github/workflows/*`, `.claude/*`, `*/web-e2e-tests/*`, `modules/mosaic/playground-e2e-tests/*`. `just test` runs none of them (the Playwright suites are not part of it); the one test that reads a doc is covered by rule 2.
6. **Anything else forces a full run** (`just test`), naming the path. This covers the workspace-wide inputs above and the data directories tests read at run time from outside every package (`areas/access/sync/data/`, `areas/access/media/data/`, `areas/access/cpe/public/`, `areas/access/cpe/projects/`, `areas/access/dpe/public/`), with no list of triggers to maintain: unknown means full.

**Keeping rule 2 complete: `test-changed.sh --audit`.** Rule 2 is maintained by hand and is the weak spot. A raw grep is not enough:

- `git grep` matches line by line, so it cannot see a multi-line `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "…"))` such as the `worked_example.rs` read. Its `-z` flag does not change that.
- A whole-file probe returns 123 hits, nearly all of them reads inside the reading crate. Reconciling that list by hand is how the first pass missed 2 of the 4 entries.

So the script gets an `--audit` mode. It does three things:

- It finds every read whole-file: `perl -0777` over `git ls-files '*.rs'`, matching `include_str!`/`include_bytes!` and `CARGO_MANIFEST_DIR` joins up to their string literal. It also catches repo-root-relative path literals (`"areas/…"`, `"modules/…"`, `"shared/…"`, `"docs/…"`, the `DPE_DATA_DIR` kind).
- It resolves each read against the reading file's directory (`include_*!` with a plain relative path), against the reading crate's manifest directory (`CARGO_MANIFEST_DIR`), or against the repo root (root-relative literals).
- It prints only the reads whose target falls under one of the cases below. Exit status is 1 when it prints anything.
  - A package other than the reader's, which the reader does not reach through a Cargo edge or an `EXTRA_EDGES` line.
  - An unowned path that rule 5 swallows, as with `docs/`.

  Targets that rule 6 already sends to a full run are not printed.

On today's tree it must print nothing once the four rule-2 entries are in. Removing any one of them must make that read reappear in the output. That is the check that the audit sees all four.

### Closure and run

- **Reverse dependencies** over all edge kinds (normal, dev, build) of the workspace graph from `cargo metadata --format-version=1 --no-deps` (`.dependencies[] | select(.path)`). Dev edges matter: `cpe-server` dev-depends on `media-store` and `sync-store`.
- **Run** `just test`'s commands restricted to the selection, **skipping each command whose list is empty**: `cargo test --tests -p <pkg>...`; the dev-feature invocation only for whichever of `dpe-server` and `mosaic-playground` is selected, with only their `<pkg>/dev` features; the selected script tests. An empty `-p` list must never reach cargo, where it would mean the whole workspace.
- **Output:** print the selection and its reasons before running, e.g. `editor-core (areas/deposit/editor/core/src/form.rs) + rdeps: editor-web, editor-pull-requests, deposit-server` or `full run: Cargo.lock`. Nothing selected: `nothing to test since <base>`, exit 0.
- **Base:** `git merge-base <base> HEAD`. Any failure (unknown ref, no `origin` remote, no common ancestor) prints git's message and runs in full, the conservative choice determinator and Zed's CI make. No fetch: the base is whatever `origin/main` locally points at, as in `just commit-lint`.
- **Changed paths:** `cd "$(git rev-parse --show-toplevel)"` first, then `git diff -z --name-only --no-renames <merge-base>` (committed, staged and unstaged; both sides of a rename) plus `git ls-files -z --others --exclude-standard`, read with `read -r -d ''`. A merge from main in the range only widens the set.

### Shape

```just
# Run the tests of the packages changed since <base> and their dependents; falls back to `just test` (rules: .github/scripts/test-changed.sh)
test-changed base="origin/main":
    bash .github/scripts/test-changed.sh "{{ base }}"
```

```bash
# .github/scripts/test-changed.sh (sketch): rule tables at the top, one line per edge.
EXTRA_EDGES=(
  "areas/access/dpe/server/data/* dpe-core dpe-api-oai dpe-web editor-core"
  "areas/access/cpe/server/tests/fixtures/* cpe-core"
  "areas/access/cpe/store/schema.sql cpe-store-build"
  "docs/src/cpe/config-language.md cpe-store-build"
)
UNOWNED_TO_PACKAGE=(
  "areas/deposit/editor/public/* deposit-server"
  "areas/deposit/editor/style/* deposit-server"
)
UNOWNED_NO_TESTS=('docs/*' '*.md' '.github/workflows/*' '.claude/*' '*/web-e2e-tests/*' 'modules/mosaic/playground-e2e-tests/*')
```

`TEST_CHANGED_DRY_RUN=1` prints the commands instead of running them; the test drives the script through it.

## Alternative Approaches Considered

- **`cargo nextest run -E 'rdeps(=pkg) | ...'`**: nextest computes the closure and runs suites in parallel, and CI already uses it. Deferred, because:
  - `just test` runs `cargo test`, and `test-changed` should be `just test` restricted, not a second runner.
  - nextest is not installed on the planning machine (`cargo nextest`: "no such command") despite `just install-requirements` listing it.
  - Whether `rdeps()` follows dev and build edges is undocumented (guppy's `depends_on` includes the package itself).
  - `-E` filters execution only, so every test binary is still built.

  Revisit when `just test` moves to nextest: the selection then becomes the filterset and the jq closure goes.
- **guppy `determinator`**: the reference implementation of these rules, and it diffs `Cargo.lock` per package. But it is a library only (0.13.0, 2026-10-21) with no CLI, so it would mean a Rust binary in the workspace for one dev recipe.
- **`cargo-affected`**: coverage-based per-test selection. Its README says "extremely early", and it needs a recorded baseline run.
- **Cargo freshness** (run what `--message-format=json` reports as `fresh: false`): that means "changed since the last build", not "since base", so the rerun after a failure skips the failing binary.
- **Feature-unification churn as a reason to avoid `-p`**: measured, not real. On a warm target, alternating `cargo test --tests` with six package subsets (Deposit, `dsp-cli`, `cpe-core`, `shared-metadata`, `mosaic-tiles`, `cpe-store-build`) recompiled 0 crates each way, because cargo keeps each feature set's artifacts side by side.

## Technical Considerations

- **Blind spots CI still catches:**
  - run-time reads through a path `--audit` cannot resolve (assembled from variables, or read via a constant defined in another crate under a name the audit does not see);
  - new cross-package reads added after the last `--audit` run;
  - behaviour that differs under `-p` feature unification (same class, not observed).

  No `build.rs` exists today. determinator documents the same limits and advises periodic full runs; `just test` before pushing is that run.
- **Drift from `just test`:** the script mirrors the recipe's three commands. A comment on the `test` recipe points at the script, and a `justfile` change forces a full run, but nothing stops the two from drifting apart.
- **Doctests:** not run, as in `just test` (`--tests`). CI runs `cargo test --doc`.
- **Conventions:** fixtures follow `check-capability-deps.test.sh` (throwaway Cargo workspaces, not hand-written metadata JSON, so dev edges are what cargo reports). The script needs bash (3.2-compatible: no empty-array expansion under `set -u`), git, cargo and jq, all of which `just test-scripts` already needs, plus perl for `--audit` (system perl on macOS and on the Ubuntu runners).

## Implementation Approach

- [x] Add `.github/scripts/test-changed.sh` implementing the selection rules, closure, output, base handling and dry-run above.
- [x] Add the `--audit` mode to `test-changed.sh` as described under *Keeping rule 2 complete*.
- [x] Run `bash .github/scripts/test-changed.sh --audit`; it prints nothing and exits 0.
- [x] Delete each of the four `EXTRA_EDGES` lines in turn and rerun `--audit`. Each time it must print the read that line covers. Restore the line afterwards.
- [x] Add `.github/scripts/test-changed.test.sh`. Its fixture is a git repo holding a three-package Cargo workspace (`a`; `b` depends on `a`; `c` dev-depends on `a`), a nested package `a/inner`, `.github/scripts/x.sh` with its test, and an `EXTRA_EDGES`-style read. It asserts:
  - a change in `a` selects `a b c`, and a change in `b` selects `b` only;
  - a change in `a/inner` does not select `a`;
  - an extra-edge path adds its package on top of its owner, including when the path is under `docs/`;
  - a root `Cargo.lock` change, an unknown unowned path, a deleted package directory and an unknown base each run in full and name the cause;
  - a docs-only change runs nothing and exits 0;
  - an `x.sh` change selects `x.test.sh` and runs no cargo command;
  - an untracked file counts;
  - `--audit` reports a multi-line `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../<sibling>/f"))` into a sibling package with no Cargo edge. It stays silent once that read has an `EXTRA_EDGES` line, or once the reader depends on the sibling.
- [x] Add the `test-changed` recipe after `test` in `justfile`, and point the `test` recipe's comment at the script.
- [x] Add `just test-changed` to the recipe table in `docs/src/workflows.md`, referencing the script's rule tables.
- [x] Add one line under Development Workflow in `CLAUDE.md`: `just test-changed` while iterating, `just test` before pushing.
- [x] Add `Bash(just test-changed)` and `Bash(just test-changed:*)` to `.claude/settings.json`.
- [x] Run `just check && just test`; both pass, the new test included via `just test-scripts`.
- [x] Take the timings on one macOS machine, with a warm target and #541's setting in effect:
  - time `just test`, then `just test-changed`, after an edit to `areas/deposit/editor/core/src/lib.rs`;
  - time both again after appending a blank line to `Cargo.lock` (git sees the change before cargo rewrites the file; revert it afterwards);
  - run all four under the same conditions (sandboxed or not), and say which;
  - record the timings and the printed selections in the PR body's Test Plan.

## Acceptance Criteria

- [x] A Deposit-only change runs only Deposit packages (a subset of the 11 under `areas/deposit`) and no script tests.
- [x] A change under `areas/access/dpe/server/data/` runs `dpe-core`, `dpe-api-oai`, `dpe-web`, `editor-core` and their reverse dependencies.
- [x] A `Cargo.lock` change runs the full `just test` and names `Cargo.lock`.
- [x] An unowned path matched by no rule runs the full `just test` and names the path.
- [x] A change confined to `docs/` or unowned `*.md` runs nothing and exits 0, except `docs/src/cpe/config-language.md`, which runs `cpe-store-build`.
- [x] A change to `.github/scripts/check-adr-refs.sh` runs only `check-adr-refs.test.sh` and no cargo command.
- [x] The dev-feature invocation runs only when `dpe-server` or `mosaic-playground` is selected.
- [x] Uncommitted and untracked changes count, also when the script is run from a subdirectory.
- [x] `test-changed.test.sh` passes under `just test-scripts` and in CI's `test / scripts` job.
- [x] The PR's Test Plan carries the four timings.

## Dependencies & Risks

- **Stacked on #541:** after it merges, the branch needs the `--onto` rebase above. The commit-hygiene gate counts from the PR base, so the stack does not trip the one-commit cap.
- **False negative**, a test the change breaks is skipped locally. Only through a blind spot above, most likely a cross-package read missing from rule 2. Mitigations: the full-run default of rule 6, `--audit`, `just test` before push, full CI.
- **Narrow savings for cross-cutting changes:** anything touching `shared/`, DPE data or a workspace-wide file runs most or all of the suite. The saving is for area-local work, the DEV-7544 case.

## Success Metrics

Baseline: `just test` after a one-file touch, 190 s on the reference Mac (#541, `split-debuginfo=off`); `just test-scripts` alone, 96 s (sandboxed). **Projected, not measured:** for a Deposit-only change, `just test-changed` saves the non-Deposit share of the 41 s of serial suite runs plus the script tests. The measurement step replaces this projection with the four real figures.

## References

- `justfile` `test` and `test-scripts` recipes; `commit-lint base="origin/main"` as the base-parameter precedent.
- `.github/scripts/check-capability-deps.test.sh`: fixture-workspace test style.
- `.github/workflows/test.yml`: CI runs `cargo nextest run --locked --all-features --all-targets`, `cargo test --doc`, `just test-scripts`.
- `areas/deposit/editor/core/src/repo_paths.rs`: `DPE_DATA_DIR`, the cross-area read rule 2 must carry.
- `docs/adr/0001-bazel-builds-the-monorepo.md`: the Bazel direction that would supersede this recipe.
- `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`: nextest and `--run-ignored` in this repo.
- guppy `determinator`: https://docs.rs/determinator/latest/determinator/; default rules: https://github.com/guppy-rs/guppy/blob/main/tools/determinator/default-rules.toml
- nextest filtersets: https://nexte.st/docs/filtersets/reference/; incremental-run issue: https://github.com/nextest-rs/nextest/issues/174
- Cargo feature resolver: https://doc.rust-lang.org/cargo/reference/resolver.html
- Zed's changed-packages CI: https://github.com/zed-industries/zed/pull/48380
