---
title: "feat: Move dsp-cli into dsp-repository with release wiring and dsp-api drift detection"
type: feat
date: 2026-09-18
author: "Ivan Subotic"
status: reviewed
repository: dasch-swiss/dsp-repository
linear: DEV-7330
---

# feat: Move dsp-cli into dsp-repository with release wiring and dsp-api drift detection

## Enhancement Summary

**Deepened on:** 2026-09-19 (second review cycle plus plan-deepening)
**Sections enhanced:** Technical Considerations, Phases 2–7, Human Actions, Acceptance Criteria, Risk Analysis
**Research agents used:** specification-reviewer (two cycles), dune-reviewer, devops-reviewer, best-practices-researcher (GitHub Actions, crates.io trusted publishing, release-please source), framework-docs-researcher (clap/anstream, assert_cmd, cargo-nextest, Cargo reference), two general-purpose agents applying the Rust and Nix pattern skills against the dsp-cli sources and the flake
**Language skills applied:** rust-patterns, nix-patterns

### Key Improvements
1. The drift workflow no longer path-filters at workflow level (which would leave the required `gate` check pending on every unrelated PR); it uses the incubator's `changes` job pattern with an unconditional trigger.
2. The release-please lockfile amend becomes a matrix job over every pending release PR; `cargo publish --locked` needs no fallback (cargo#11148 fixed in Cargo 1.68).
3. dsp-cli's ADR-0002 rationale is its own (a client of every area that no area depends on), carved out of the dependency-arrow sentence; the area-crate ban is tagged static-analysis because the publish dry-run already enforces it; a second grep gate keeps every live test `#[ignore]`d.
4. Code deliverables are anchored to verified lines: one test helper for color pinning, thirteen copies of `require_env` to consolidate, four stdout write sites for BrokenPipe, and the "one classifier" idea replaced by extracting the one genuinely duplicated helper.
5. Dev-shell tooling (`cargo-nextest`, `cargo-deny`) comes from nixpkgs, current at the locked revision, with matching `install-requirements` pins.
6. A Bazel and RBE readiness table maps every Cargo-specific piece of the plan to its `rules_rust` counterpart and decides the publishing manifest now (decision: Ivan, 2026-09-19: land Cargo-native, Bazel-ready).

### New Considerations Discovered
- `NO_COLOR` beats `CLICOLOR_FORCE` in anstream's precedence, so pinning `NO_COLOR=1` in the one `assert_cmd` helper is sufficient; production clap code stays untouched.
- The `crates-io` GitHub environment is optional hardening in crates.io's trusted-publisher model, not a requirement.
- `separate-pull-requests: true` names the dsp-cli release branch `release-please--branches--main--components--dsp-cli`.

### Learnings Applied
- `dasch-specs/learnings/test-setup/docker-compose-dev-stack-startup-ordering.md`: fixtures before the API starts.
- `dasch-specs/learnings/configuration-errors/github-actions-composite-action-main-ref-pr-isolation.md`: workflows on `release:` events run from `main`, which is what makes the baseline-release sequencing safe.

## Overview

Move the `dsp-cli` crate (binary `dsp`, 0.2.1 on crates.io) from the `dsp-incubator`
prototype monorepo into the `dsp-repository` Cargo workspace as a root-level component
`dsp-cli/`, harmonize it with this repository's toolchain and conventions, give it an
independent release line that keeps publishing to crates.io, and add a CI job that runs its
live tests against a containerized dsp-api so API drift is caught in this repository instead
of by users. Seven phases delivered as **one PR** (#409) with `allow-many-commits` ticked, each
phase one or more commits in phase order (decision: Ivan, 2026-09-18); Phases 6 and 7 are the
post-merge checks and the incubator archive. The incubator stays the history archive. The two
incubator-side deliverables run in the `dsp-incubator` checkout at
`/Users/subotic/_github.com/dasch-swiss/dsp-incubator`.

## Problem Statement / Motivation

- dsp-cli's own ADR-0014 (2026-06-18) records the decision to migrate here after the first
  crates.io publish; that publish happened on 2026-07-17 and the crate is at 0.2.1. The
  migration is the open milestone (dsp-cli PROJECT_PLAN Phase 11).
- dsp-cli goes out of sync with dsp-api. Its 14 live tests are the only drift detector and are
  never run in CI (dsp-cli ADR-0009 deferred stack-based tests until "CI without credentials"
  or "isolation from a shared environment" was wanted; both are wanted now).
- The incubator's CI is a per-prototype `just ci` matrix with a Cloud Run deploy that means
  nothing for a CLI; it has no release automation, so every release is a manual
  `cargo publish` by one person.
- Ivan intends dsp-cli to grow into the command-line client of the Deposit, Archive and Access
  moduliths as well. It has to live where those are built.

## Proposed Solution

1. **Record the decision-record convention first** (root ADR-0006 plus a gate) as the PR's
   first commit, so the move commits comply with a rule that already exists in the history
   instead of introducing repo-wide policy inside a 96-file reformat commit.
2. **Land as a sixth root directory, `dsp-cli/`**, single crate, beside `mosaic/`, `vitrinli/`,
   `chischtli/` (decision: Ivan, 2026-09-18). ADR-0002 is amended with dsp-cli's own
   rationale: it is the platform's command-line client, today of the VRE through DSP-API, later
   of the three areas over the wire. Unlike Mosaic, Vitrinli and Chischtli, **no area depends on
   it**; it sits on neither side of the `areas → shared, mosaic, vitrinli, chischtli` arrow. It
   may depend on `shared-*` crates only if those are published to crates.io, never on an area
   crate.
3. **Pure move.** The move commits copy the crate, reformat it under this repo's rustfmt
   configuration, register it in the workspace and in every repo-level document, rewrite ADR
   citations to the qualified form, and change no behaviour. Version stays 0.2.1.
4. **Colocated ADRs, always-qualified citations** (decision: Ivan, 2026-09-18). dsp-cli keeps
   `0001`–`0016` under `dsp-cli/docs/adr/` and continues at `0017`. A bare `ADR-NNNN` always
   means a root ADR; a component ADR is always cited as `dsp-cli/ADR-NNNN`, also from inside
   `dsp-cli/`.
5. **Independent version line** as a second release-please package (`simple` release type,
   `extra-files` pointing at its own `Cargo.toml`, relative to the package directory), tags
   `dsp-cli-vX.Y.Z`, published through crates.io trusted publishing (OIDC). The root package
   excludes `dsp-cli/` paths.
6. **Drift CI**: a pinned fuseki + dsp-api stack in GitHub Actions running dsp-cli's live tests
   in a strict mode that fails on a missing server, as a PR check whenever `dsp-cli/**` changes;
   the same job nightly against `daschswiss/knora-api:latest`. A red nightly run is the signal;
   GitHub Issues are disabled on this repository (Linear is the tracker), so nothing is filed
   automatically.
7. **Hardening before the first release from the new home**: the three open dsp-cli backlog
   items (token fragment in `warn` logs, unvalidated `--server` scheme, BrokenPipe as
   internal error) and the `DSP-Client` header dsp-api's deprecation plan (DEV-6844) asks
   every DaSCH client to send. Then release 0.3.0 and archive the incubator prototype.

## Alternative Approaches Considered

- **Placement.** `modules/dsp-cli/` per dsp-cli ADR-0014 (rejected: `modules/` is removed by
  ADR-0002, so it would move twice); a new `tools/` root (rejected: a category with one
  member); inside an area (rejected: dsp-cli talks to every area over the wire, and area
  members are capabilities of a modulith per ADR-0003). These three land in ADR-0002's
  Considered Options so the reasoning outlives this spec.
- **Three-crate split** (`dsp-client` / `dsp-render` / `dsp-cli`, dsp-cli ADR-0008) in the
  move. Rejected for this plan: a 71k-line move plus a split is unreviewable in one commit.
  ADR-0008's layout keeps the split a directory rename later.
- **Git history import** via `git filter-repo` and a squash PR. Rejected in dsp-cli ADR-0014:
  the incubator is a clone-present archive; a forge-side copy is worse.
- **Inherit the workspace version** (0.2.1 would jump to 0.8.x). Rejected: every workspace
  release would bump dsp-cli with no dsp-cli change and the number would carry no meaning.
  The `rust` release type was also rejected: googleapis/release-please#2111 (open, fix PR
  #2895 unmerged as of 2026-09-10) writes a literal `version` into members that use
  `version.workspace = true`.
- **ADR homing.** One central `docs/adr/` with renumbering (rejected: ~360 citations
  rewritten, ownership invisible in the tree, and Vitrinli and Chischtli would renumber again
  on arrival); `docs/adr/dsp-cli/` subdirectories (rejected: decisions no longer travel with
  the code they bind, and the citation still needs qualifying).
- **Drift detector.** OpenAPI diff of `/api/docs/docs.yaml` between pinned and latest
  (kept as a possible cheap second signal, not the detector: dsp-api types every v2 body as an
  opaque string, so the JSON-LD keys dsp-cli hand-parses are invisible to it; admin and v3
  bodies are typed). Re-recording wiremock fixtures against the stack (rejected: dsp-cli
  embeds response bodies as inline literals, so this needs new capture tooling first).
  `repository_dispatch` from dsp-api's release workflow (rejected for now: dsp-api dispatches
  to no other repo today and the bot app is scoped to dsp-tools; the nightly job gives the
  same signal one day later without touching dsp-api).
- **Replace reqwest with ureq** to match the workspace's HTTP client. Deferred: it touches all
  3.8k production lines of `http.rs`; the mismatch is recorded, not fixed here.
- **GitHub Actions `services:` for the stack.** Rejected: service containers start in parallel
  before any step runs, so the fixtures-before-API ordering cannot be expressed; `docker
  compose up --wait` per service can.
- **One status classifier for the eight inline `401/403/404` sites in `http.rs`.** Rejected
  after reading them: the status branching is uniform but every message is bespoke and
  `map_unexpected_status` (`src/client/http.rs:733`) already documents that tailored
  endpoints keep their own arm. Only the repeated truncate-with-ellipsis logic is duplicated
  code; that is what gets extracted.

## Technical Considerations

**Placement and boundaries.** ADR-0002 says every crate lives under exactly one of five
roots; dsp-cli becomes the sixth, with a rationale of its own. ADR-0002's stated test for a
root peer ("used by more than one area", `docs/adr/0002-*.md:20`) does not describe dsp-cli:
no area depends on it. It is a client of every area, integrating with each over its public
HTTP surface, exactly as areas integrate with each other. The amendment therefore adds a
separate clause rather than riding on the Mosaic/Vitrinli/Chischtli sentence, and carves
dsp-cli out of the dependency-arrow sentence (`docs/adr/0002-*.md:29`): areas never depend on
it, it depends on no area crate. Boundary rules for ARCH-MAP, one per bullet with its own
enforcement tag: (1) no dependency on an area crate — **static-analysis** today, because every
other member is `publish = false` and `cargo publish -p dsp-cli --dry-run` in `check.yml`
fails on an unpublished path dependency; **structure** via Bazel visibility after ADR-0001;
(2) a `shared-*` dependency only if that crate is on crates.io — the same gate; (3)
over-the-wire integration only — **review**; (4) live tests never run in the default suite —
**static-analysis** (`#[ignore]` asserted by `check-live-tests-ignored.sh`, see below).
`.github/scripts/check-shared-paths.sh` only guards `shared/` (the first phase of DEV-7268,
landed with PR #391: `platform-*` is now `shared-*` under `shared/`), so it neither sees nor
needs to see `dsp-cli/`. Physically, dsp-cli lands at the repository root **now**, beside
`modules/`, `shared/` and the seed `CONTEXT.md` directories; it is unaffected by DEV-7268's
later `modules/` → `areas/` move and does not wait for it.

**Publishing constrains the dependency rule.** `cargo publish` rejects a path dependency on a
crate that is not itself on crates.io (Cargo reference, "Specifying dependencies": a path
dependency for a published crate must also carry a registry `version`), and every workspace
member today is `publish = false`. dsp-cli depends on no workspace crate, so nothing breaks
now; the ADR-0002 amendment states that a future dsp-cli dependency on a `shared-*` crate
requires that crate to be published too (or the shared concept to be vendored), and the
`publish --dry-run` gate is what catches it if forgotten.

**ADR-0004** ("every user-facing surface is a hypermedia server") scopes itself to "every
capability with a screen in any area's modulith" (`docs/adr/0004-*.md:8`). A command-line
client is not a surface in that sense. The ADR-0002 amendment says so in one sentence, and
ADR-0004 gets a one-line Consequences note pointing back, so a reader who opens ADR-0004 while
working on dsp-cli's terminal output sees the carve-out where they look for it.

**Decision-record convention (ADR-0006).** The layout is already policy in `ARCH-MAP.md`
("system-wide decisions in `docs/adr/`, context-internal ones under the context's own
`docs/adr/`", the "Colocated docs" line at `ARCH-MAP.md:518`) and in dsp-repository-design's
"where to write decisions" table; dsp-cli is the first component to exercise it. That line
today grants a component `docs/adr/` to "each bounded context, and each shared engine with its
own vocabulary"; dsp-cli is neither, so the line's scope is widened to any root-level
component with its own vocabulary or decision history, rather than only annotated. What is new
is the citation rule and its gate. The gate is a **resolvability check**, not a ban on bare
numbers inside components: every bare `ADR-NNNN` in the repository must resolve to
`docs/adr/NNNN-*.md`, and every `<component>/ADR-NNNN` must resolve to
`<component>/docs/adr/NNNN-*.md`. That catches dangling references and every unqualified
citation of a component ADR whose number has no root counterpart. The residual, stated in
ADR-0006: a stale bare `0001`–`0006` inside `dsp-cli/` resolves to a root ADR and passes the
gate; the one-time rewrite in the move and review carry that case. `docs/src/decisions.md` is
a book page, not a spec, so listing it in `SUMMARY.md` does not conflict with
`docs/specs/CLAUDE.md`. No maintained ADR tool is monorepo-aware (checked 2026-09-18:
log4brains stale since 2022, adr-tools single-directory, MADR tooling single-repo), so the
convention must survive without one. Vitrinli and Chischtli arrive with their own series in
dsp-repository-design and need no second migration.

**Baseline measured on 2026-09-18** (toolchain 1.93.0, this repo's `.rustfmt.toml`):

| Check | Result |
|---|---|
| `cargo build --all-targets` | 64 s wall (cold, includes downloads) |
| `cargo test --all-targets` | 1005 unit + 379 integration tests, 356 insta snapshots, ~65 s |
| `cargo clippy --all-targets --all-features -D warnings` | clean |
| `cargo machete` | clean |
| `cargo +nightly fmt --check` with this repo's config | 96 of 105 source files differ |
| `cargo doc --no-deps --all-features` | 6 rustdoc warnings (2 private-item links, 4 unresolved intra-doc links to `DspClient` methods) |
| `cargo deny` | not run yet (not installed in the sandbox; `deny.toml` exists here but nothing runs it) |
| Panic surface outside tests | none (every `unwrap`/`expect` is inside `#[cfg(test)]`) |
| Layering (dsp-cli ADR-0008) | no `client` ↔ `render` imports |

**Two CI traps found in the baseline.**
- `tests/cli.rs` help snapshots fail when `CLICOLOR_FORCE` is set in the environment. Color
  detection is anstream's, not clap's: it checks `NO_COLOR` first, then `CLICOLOR_FORCE`, then
  `CLICOLOR=0`, then the TTY (anstream `src/auto.rs`, `AutoStream::choice()`). Every test in
  `tests/cli.rs` builds its command through one helper, `fn dsp()` at `tests/cli.rs:45-53`,
  which already sets `TERM=dumb`, `COLUMNS=100` and `DSP_NO_UPDATE_CHECK=1`; adding
  `NO_COLOR=1` there and removing `CLICOLOR_FORCE` / `CLICOLOR` makes the snapshots
  deterministic. Production code is not touched: `Command::color(ColorChoice::Never)` on `Cli`
  (`src/cli/mod.rs:235-242`, no color attribute today) would also disable color for real
  terminal users.
- `test.yml` runs `cargo nextest run --all-features --all-targets`, which enables dsp-cli's
  `live` feature. The 14 `live_*` tests early-return with a **pass** when `DSP_TEST_SERVER` is
  unset. They would report green in every CI run, and a drift job pointed at a broken stack
  would also stay green. The fix is two-fold: mark every live test `#[ignore]` (`#[test]` then
  `#[ignore = "…"]`; nextest then reports them as skipped, never as passed) and add a strict
  mode (`DSP_LIVE_STRICT=1`) under which a missing variable is a failure. The `require_env` and
  `optional_env` helpers are copied verbatim into 13 of the 14 live files today; they move to
  `tests/common/mod.rs`, included with `mod common;`, keeping the `Option<String>` return so
  every call site's `Some(v) => v, None => return` stays a drop-in, and panicking instead of
  returning `None` when `DSP_LIVE_STRICT=1`. A gate script keeps the `#[ignore]` invariant
  (the plan's highest-rated risk) from eroding as tests are added.

**Formatting diff and reviewability.** The move reformats 96 files, and this repo's rustfmt
regroups and splits imports, so `--ignore-all-space` cannot prove the move is pure. Phase 2
is therefore three commits: (1) the verbatim copy, (2) `just fmt` only, (3) the enumerated
edits. The reviewer diffs commit 3 alone. The PR ticks `allow-many-commits`; every commit is
`type(scope): subject` and passes `just commit-lint`, and no commit is a fix of an earlier
commit in the same branch (`docs/src/git-conventions.md`). `.commitlintrc.yml` enforces only
that a scope is present, not which one, so commits 1 and 2 may use the `dsp-cli` scope before
commit 3 adds it to the advisory table in `CONVENTIONS.md`.

**Dependencies.** Same majors as `[workspace.dependencies]`: `serde`, `serde_json`
(`preserve_order`, exact feature match), `thiserror`, `tracing`, `tracing-subscriber`,
`chrono`, `insta` (`yaml`; the workspace has `yaml` + `filters`, a superset). Features given
beside `workspace = true` are additive (Cargo reference, "Inheriting a dependency from a
workspace"), so dsp-cli writes `tracing-subscriber = { workspace = true, features = ["fmt"] }`
and the workspace entry keeps `env-filter` alone; no other member pays for `fmt`. Crate-local:
`clap`, `reqwest` (blocking + rustls; reqwest 0.13's `rustls` feature selects aws-lc-rs, whose
`cmake` need `flake.nix:61-62` already covers), `jsonwebtoken` (`rust_crypto`, pure Rust),
`url` (already at `dsp-cli/Cargo.toml:69`, used by the Phase 5 `--server` validation),
`dotenvy`, `dirs`, `toml`, `rpassword`, `semver`, `percent-encoding`; dev: `wiremock`,
`assert_cmd` (`env_remove` and `env` are on `assert_cmd::Command`, docs.rs 2.2.2),
`predicates`, `tempfile`, `tokio` (`rt`, `macros`). `edition = "2024"` and
`rust-version = "1.92"` stay literal on the member; workspace-inheritable keys are opt-in per
member, and 1.92 is a floor under the 1.93 pin. Delete `dsp-cli/rust-toolchain.toml`
(1.92.0 would shadow the repo's 1.93.0 for commands run in that directory) and
`dsp-cli/.envrc`. Both repositories are Apache-2.0; the crate keeps its own `LICENSE` file
because the packaged crate must contain one (`include` lists it). Cargo does not auto-detect
`include_str!` inputs (rust-lang/cargo#13309), which is why `docs/topics/*.md` stays in the
`include` allowlist and the dry run guards it.

**Dev-shell tooling.** `cargo-nextest` and `cargo-deny` are not in the flake or in
`install-requirements`. nixpkgs-unstable at the locked revision (`a32edd76`, 2026-09-17) ships
`cargo-nextest` 0.9.144 and `cargo-deny` 0.20.2, both current with crates.io, so they join
`buildInputs` the way `cargo-watch` does (`flake.nix:67`), not through the `_ensure_tool`
binstall shellHook (reserved for tools nixpkgs lags on). `install-requirements` gets matching
`cargo binstall -y cargo-nextest@0.9.144` and `cargo binstall -y cargo-deny@0.20.2` lines.
Docker stays out of the flake (Docker Desktop on macOS is outside Nix; the drift job runs on
Linux runners with Docker preinstalled); the `dsp-cli-stack-*` recipes depend on a
`_check-docker` guard like `_check-node`. The repo stays at Nix rung L1 (dev shell only); a
`packages.dsp-cli` derivation is the natural later step, out of scope here.

**Release attribution is by path, not scope.** release-please assigns a commit to a package
when any touched file is under that package's path; the root `"."` receives every commit
unless `exclude-paths` excludes it, and a commit is excluded only when all its files fall
under an excluded path. So: root's `exclude-paths` becomes `[".github", "dsp-cli"]` (it is
`[".github"]` today); a dsp-cli commit that also touches the root `Cargo.lock` (a dependency
change) bumps both packages. That is accepted and documented rather than worked around; both
packages are pre-1.0 and release often. `separate-pull-requests: true` keeps the two release
PRs apart so merging a workspace release never ships dsp-cli; release-please then names the
dsp-cli branch `release-please--branches--main--components--dsp-cli` (`src/util/branch-name.ts`,
`BranchName.ofComponentTargetBranch`), and an open root release PR may be closed and reopened
under the analogous name on the first run after the change. The `simple` strategy updates
`version.txt` only if it exists (`src/strategies/simple.ts`, `createIfMissing: false`), which
is why this repo's root package has none; dsp-cli behaves the same. The existing post-step in
`release-please.yml` (lines 40-53) that runs `cargo update --workspace` and amends the release
PR selects `.[0]` of the pending release PRs and reads `steps.release.outputs.pr`, a
single-PR output that manifest mode with two packages no longer sets; with two packages the
step becomes a matrix job over every `autorelease: pending` branch (shape in Phase 3).
Bootstrapping: the manifest entry `"dsp-cli": "0.2.1"` needs a `dsp-cli-v0.2.1` baseline
release-please can find; release-please's documentation states `bootstrap-sha` bounds history
on a first run but not the fallback for an untagged manifest version, so the plan does not
rely on it. A GitHub Release `dsp-cli-v0.2.1` (tag on the PR's merge base on `main`, body
"baseline for release-please, not a publish") is created **before** the PR merges: at that
moment `publish-dsp-cli.yml` does not exist on `main`, so nothing publishes (workflows on
`release:` events run from `main`), and `dpe-release-publish.yml` filters on
`startsWith(tag_name, 'v')`, which `dsp-cli-v…` does not satisfy; it is the only workflow
listening on `release: published` today. The first dsp-cli release PR then contains exactly
the move and everything after it. The first dsp-cli release from here carries a
`Release-As: 0.3.0` footer so the version says "new home" rather than landing as 0.2.2 (with
`bump-minor-pre-major`, a `feat` alone gives 0.2.2). The root package is unaffected in kind: in
this repository every commit type bumps the root patch version (`docs/src/git-conventions.md`),
so the move commit adds one line to the root release PR that already exists after any merge;
merging that PR releases DPE as every root release does, and nothing here changes that.

**Publishing.** `cargo publish -p dsp-cli` from the workspace root, with `--locked`:
rust-lang/cargo#11148 (publish ignoring the workspace lockfile) was fixed by cargo#11477 in
Cargo 1.68, well below the 1.93 pin. Trusted publishing (`rust-lang/crates-io-auth-action@v1`,
one optional input `url`, one output `token`, `permissions: id-token: write`, token revoked in
the action's post step) needs a one-time trusted-publisher configuration on crates.io by an
existing owner (`BalduinLandolt` or a member of `github:dasch-swiss:everyone-private`), naming
this repository and the workflow file. The GitHub environment is optional in that model; the
plan uses `environment: crates-io` as defence in depth, and the trusted-publisher entry must
then name the same environment or authentication fails. `cargo publish --dry-run` verifies
packaging (the `include` allowlist ships `docs/topics/*.md`, which `include_str!` needs) but
not registry-side state. GitHub has no job-level path filters, so the dry-run job runs
unconditionally in `check.yml` (one compile of the crate, about a minute warm).

**Required checks and path filters.** GitHub's documentation is explicit: a workflow skipped
by `on.pull_request.paths` leaves its checks **pending**, which blocks merge when one of them
is required, whereas a job skipped by a job-level `if:` reports success. `paths:` exists only
at workflow level, so a path-filtered drift workflow would never run its `gate` job on an
unrelated PR and H3 could never be satisfied. The drift workflow therefore triggers
unconditionally on `pull_request` (plus `schedule` and `workflow_dispatch`), computes whether
`dsp-cli/**` or the workflow file changed in a first `changes` job (`git diff --name-only
base...head`, the incubator's `ci.yml` pattern), runs `pinned` only when it did, and ends in
`gate` with `needs: [changes, pinned]` and `if: always() && github.event_name ==
'pull_request'`, succeeding when `changes` succeeded and `pinned` succeeded or was skipped.
`gate` is the check H3 makes required. `Cargo.lock` is not part of the change test: a Docker
stack on every Dependabot bump is not worth it, and a dsp-cli dependency bump also edits
`dsp-cli/Cargo.toml`. `dorny/paths-filter` is not needed for this shape; if it is ever used,
pin it by commit SHA (`tj-actions/changed-files` was compromised in March 2025,
CVE-2025-30066).

**Bazel and RBE readiness (ADR-0001).** dsp-cli lands Cargo-native now, and the ADR-0001
migration re-points it together with every other crate (decision: Ivan, 2026-09-19). Nothing in
this plan is allowed to make that re-point harder than for the crates already here, so each
Cargo-specific piece is named with its Bazel counterpart, and two decisions are taken now:

| Cargo-native today | Under Bazel (`rules_rust`, `crate_universe` `from_specs`) |
|---|---|
| Workspace member; deps through `[workspace.dependencies]` | A `rust_library` + `rust_binary` target; third-party crates declared in `MODULE.bazel` like everyone else's |
| Eleven `include_str!("../../docs/topics/….md")` sites in `src/actions/docs.rs` | Same source; the topics become `compile_data` of the library target. Keeping them inside `dsp-cli/` (they are) is what makes this a one-line attribute |
| `assert_cmd::Command::cargo_bin("dsp")` in the two CLI-level test files (`tests/cli.rs`, `tests/docs.rs`), resolved through the Cargo-set `CARGO_BIN_EXE_dsp` | A `rust_test` with the binary in `data`, resolved through runfiles. Phase 2 keeps binary resolution in one helper per file (`fn dsp()` at `tests/cli.rs:45-53`; the same shape in `tests/docs.rs`), so the re-point is two functions |
| `insta` snapshots beside the tests | Snapshot files as `data`; `INSTA_UPDATE=no` under Bazel (no writable source tree); the `.snap.new` review loop stays a local Cargo activity until Bazel is the only build |
| Live tests `#[ignore]`d, gated by `check-live-tests-ignored.sh`, run with `--run-ignored only` | A separate `rust_test` target `//dsp-cli:live_tests` tagged `manual`, `external`, `no-remote-exec`, `requires-network`; `bazel test //...` never runs it, the drift job runs it by label. The `#[ignore]` gate retires with the Cargo build |
| `cargo nextest`, `cargo hack`, `cargo machete`, `cargo deny`, the dry-run job in `check.yml` | Retired or replaced by the migration for all crates alike; not dsp-cli's concern |
| `cargo publish -p dsp-cli` from the workspace root | **Decision:** `dsp-cli/Cargo.toml` and a `dsp-cli/Cargo.lock` stay as the publishing manifest after the root manifests are removed (rules_rust has no publish path, bazelbuild/rules_rust#458). The hermetic LLVM toolchain of ADR-0001 cross-compiles a Linux binary from macOS, which is what makes prebuilt release binaries (dsp-cli PROJECT_PLAN "Phase 11 follow-on") cheap afterwards; that follow-on, not this plan, may then retire crates.io as the primary channel |
| Drift stack via `docker compose` on the GitHub runner | Unchanged. Docker is not available to remote executors; dsp-api runs its Docker-dependent Bazel tests the same way (images loaded on the runner, then `bazel test` with the local strategy). The live-test target carries `no-remote-exec` for exactly this reason |

The ADR-0002 amendment records the publishing decision and points at this table's live-test
and drift-job rows so the ADR-0001 runbook finds dsp-cli's specifics without re-deriving them.

**Drift CI stack.** fuseki + dsp-api is sufficient: both dump live tests pass
`skip_assets=true`, so sipi and ingest are never exercised, and dsp-api's startup checks only
the triplestore. Fixtures: dsp-api's `modules/webapi/scripts/fuseki-init-knora-test.sh` is a
plain curl loop over `test_data/*.ttl` (projects 0001, 0801/beol, 0803/incunabula, 0804, 00FF,
0806); check out dsp-api sparsely at the pinned tag so fixtures and API version move together.
Load fixtures **before** starting dsp-api (learning: dsp-api fills in-process caches at
startup; data loaded later is invisible). `docker compose up --wait` per service, with
`depends_on: condition: service_healthy` on `api`, is the second guard beside the explicit
`/health` poll of `wait-for-api.sh` (Fuseki's container health can precede dataset readiness).
`ubuntu-latest` (24.04) ships Docker Compose v2 as `docker compose`; no setup step is needed.
SystemAdmin: `root@example.com` / `test`. SPARQL passthrough:
`KNORA_WEBAPI_ALLOW_SPARQL_PASSTHROUGH=true`. Pins live in this repository (`API=v38.1.0`,
`DB=v38.1.0`, the values dsp-tools pins today), not in dsp-tools' `versions.env`. The compose
file sets `KNORA_WEBAPI_KNORA_API_EXTERNAL_HOST=0.0.0.0` and
`KNORA_WEBAPI_KNORA_API_EXTERNAL_PORT=3333` as dsp-tools' and dsp-api's own compose files do,
so the stack serves external ontology IRIs as `http://0.0.0.0:3333/ontology/…`; the
`beol:page` class the live tests target is therefore
`http://0.0.0.0:3333/ontology/0801/beol/v2#page` (defined at
`dsp-api/test_data/project_ontologies/beol-onto.ttl:1410`), and project `0001` (anything)
carries four list roots in `test_data/project_data/anything-data.ttl`, so it serves as
`DSP_TEST_VOCAB_PROJECT`. `live_sparql_query.rs:192` is the one consumer of
`DSP_TEST_NON_ADMIN_TOKEN` (a permission-boundary case). Startup time is unmeasured; a JVM plus
Fuseki cold start is realistically one to three minutes once images are cached, and the drift
workflow carries its own 30-minute budget (target under 12 minutes), separate from
`check.yml`'s and `test.yml`'s 20. Caching the loaded dataset is not worth its invalidation
risk until measured (it would have to key on `stack.env` and the fixture script's hash).
Scheduled runs can be delayed under load and are disabled after 60 days without repository
activity, so `workflow_dispatch` stays beside `schedule`. Two live tests do not fit a fixture
stack: `live_vocabulary.rs` hardcodes production project 0838 with `.expect()` and exact
counts, and `live_update_check.rs` hits crates.io. The first is re-pointed at fixture data;
the second is excluded by the nextest filter
`binary(/^live_/) & !binary(live_update_check)` (one `-E` expression; several `-E` flags are
unioned; precedence `()` > `!` > `&` > `|`, verified against cargo-nextest 0.9.103 on a
throwaway crate). Ignored tests run with `--run-ignored only` (values: `default`, `only`,
`all`).

**dsp-api deprecation headers.** dsp-api flags four endpoints `.deprecated()` today (two
permissions routes, `/admin/lists/infos/{iri}`, `/admin/lists/nodes/{iri}`); dsp-cli uses the
canonical `/admin/lists/{iri}`, so nothing is affected. `Deprecation` / `Sunset` response
headers are planned in DEV-6844 but not implemented. Once they exist, the drift job should
fail on any response carrying `Deprecation`; noted as a follow-up, not a deliverable.

**Hardening details (Phase 5), anchored.** Auth-cache load failures log `error = %e` at
`warn` at `src/actions/auth/status.rs:50`, `src/actions/auth/token.rs:68`,
`src/actions/vre/project.rs:289` and `src/actions/vre/sparql.rs:243`; `init_tracing`
(`src/diagnostic.rs:131-142`) maps verbosity 0 to `warn`. `Config::resolve`
(`src/config/mod.rs:52-80`) passes a non-shortcut `--server` through verbatim; the validation
parses it with `url::Url`, treats `localhost`, `127.0.0.1` and `::1` as loopback
(`Url::host_str()` returns IPv6 hosts without brackets; a unit test asserts
`http://[::1]:3333` passes without the override), and refuses other `http://` unless
`--allow-insecure-server` or `DSP_ALLOW_INSECURE_SERVER=1` is set, flag before env per
dsp-cli/ADR-0007. `Diagnostic::from(io::Error)` (`src/diagnostic.rs:34-38`) maps every
`io::Error` to `Internal` and is reached from non-stdout paths too (reading `auth.toml`), so
BrokenPipe is handled at the stdout write sites instead: the renderers' `Box::new(io::stdout())`
sinks in `src/render/{prose,json,csv,tsv,lines}.rs`, `src/actions/docs.rs:229,245`,
`src/actions/vre/sparql.rs:275` and `src/actions/vre/project.rs:1360`; on
`ErrorKind::BrokenPipe` the process exits 0 silently, every other error still becomes
`Internal`. The eight inline status checks in `src/client/http.rs` (`401|403` at
2209-2210, 3007-3008, 3046-3047; `404` at 2219, 2263, 2634, 2996, 3514) differ in message,
not in branching, and `map_unexpected_status` (line 733) documents that design; the duplicated
80-character truncate-with-ellipsis logic (2265-2269, 2636-2640, and a third site near 3000)
is what gets one helper.

**Vocabulary collisions** for the root `CONTEXT.md` Flagged ambiguities, one bullet per term
with a Resolution, matching the file's existing entries: **Project** already has an entry
(`ProjectRaw` vs `dpe_core::Project`); it gains a third meaning, dsp-cli's VRE project as
DSP-API serves it, with the Resolution that the shortcode identifies the same research project
across the VRE and the Repository while the records differ (a live administrative record vs
an archived descriptive one), and that "dsp-cli's Project" is said when the VRE record is
meant. **Resource** and **Representation** (dsp-cli: a resource instance; a file-bearing
resource type) vs the Archive Area's Resource and Representation; **Field** / **Value** vs the
editor's `registry::FIELDS` and form fields; **Vocabulary** (dsp-cli's word for a DSP-API
"list"). dsp-cli is listed under the existing "Shared infrastructure (not bounded contexts)"
heading (`CONTEXT.md:15`), whose other members are things areas depend on, so its bullet
carries the distinguishing clause: no area crate ever depends on dsp-cli; it depends on their
public HTTP surface only. No new heading, since the design repo already owns the nouns
"RDU-Tooling" and "preservation admin tooling".

**What does not move.** `prototype.json` (incubator registry plumbing), `.envrc`,
`.env.example` (its guidance moves into `docs/src/dsp-cli/`), `rust-toolchain.toml`,
`justfile` (recipes merge into the root justfile), `docs/PROJECT_PLAN.md` (776 lines of phase
tracking; the decisions live in the ADRs), `docs/BACKLOG.md` (three open items become Phase 5
deliverables, two become Linear issues), `docs/design/plans/` (35 folders, 17k lines of
planning history; the incubator archive keeps them), the ~120-line phase history in
`dsp-cli/CLAUDE.md`. `docs/dev/*` is superseded by `CONVENTIONS.md`, `REVIEW.md` and
`docs/src/git-conventions.md`; its testing-strategy and domain-language content is folded
into `docs/src/dsp-cli/` and `dsp-cli/CONTEXT.md`. Deliberate patterns that stay: one local
`MockDspClient` per action file (dsp-cli CLAUDE.md states why), `#[allow(clippy::too_many_arguments)]`
on six action functions.

## Implementation Phases

Phases 1–5 are the commits of one PR against `dsp-repository`, in this order, with
`allow-many-commits` ticked in the PR body. That PR is #409 on branch
`docs/dsp-cli-migration-plan`, which already carries this plan as its first commit and is
stacked on #391 until that merges (decision: Ivan, 2026-09-19). Every commit leaves the tree
green under `just check` and `just test` (the history is bisectable; rebase-merge lands each
commit on `main` verbatim). Phases 6–7 are post-merge steps.

**Reviewer set** for every phase's diff (`eng:reviewing`, findings amended into that
phase's commit before the next phase starts): `eng:review:rust-reviewer`,
`eng:review:devops-reviewer` (workflows, justfile, compose), `eng:review:consistency-reviewer`
(renames, registrations, stale references), `eng:review:dune-reviewer` (ADR-0002, ARCH-MAP
and CONTEXT edits), `eng:review:code-simplicity-reviewer`; Phase 5 adds
`eng:review:security-reviewer`.

#### Phase 1: Record how decisions are homed and cited

One commit, `docs(docs,ci): record how decision records are homed and cited across components`.
First in the branch so the move commits follow an existing rule.

- [x] Write root `docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md` in the shape of ADRs 0001–0005: system-wide decisions in `docs/adr/`, component decisions under the component's own `docs/adr/` with their own sequence; a bare `ADR-NNNN` always names a root ADR; a component ADR is always cited as `<component>/ADR-NNNN`, also from inside the component; an amendment refines a decision in place under a dated `## Amendment` heading, a changed decision is a new ADR with `status: superseded by`; a root ADR that constrains an earlier component ADR names it in its Consequences and the component ADR gets a dated amendment pointing back; name `dsp-cli/` as the first component and Vitrinli and Chischtli as the next; state the gate's residual (a stale bare `0001`–`0006` inside a component passes); `Enforced by:` the gate below (static-analysis) and review for the amendment rule
- [x] Add `.github/scripts/check-adr-refs.sh`: a **bare** reference is `ADR-[0-9]{4}` not preceded by `/` or a word character (`grep -P '(?<![/\w])ADR-\d{4}\b'`, or the ERE `(^|[^/A-Za-z0-9_])ADR-[0-9]{4}`) and must resolve to `docs/adr/NNNN-*.md`; a **qualified** reference is `<dir>/ADR-[0-9]{4}` and must resolve to `<dir>/docs/adr/NNNN-*.md`, where `<dir>` is any directory containing `docs/adr/`; scan tracked text files; print each unresolved reference with file and line; exit non-zero on any
- [x] Add `.github/scripts/check-adr-refs.test.sh` in the style of `check-shared-paths.test.sh`, running against a temporary tree (a resolving bare reference passes, a dangling bare reference fails, a resolving qualified reference passes, a qualified reference to a missing file fails, a qualified reference is not also counted as a bare one)
- [x] Wire `check-adr-refs.sh` into `just check` (a `check-adr-refs` recipe, listed as a dependency of `check`) and the `.test.sh` into `just test`, matching how the other gates are wired
- [x] Widen the "Colocated docs" convention line in `ARCH-MAP.md` (line 518) from "each bounded context, and each shared engine with its own vocabulary" to any root-level component with its own vocabulary or decision history, citing ADR-0006; extend `docs/src/repo_structure.md`'s "Five ADRs are in place" sentence to six; add a "Decision records" page `docs/src/decisions.md` to the mdBook (`SUMMARY.md` under Repo Overview) that states the rule and links `docs/adr/` and each component's `docs/adr/` by their `https://github.com/dasch-swiss/dsp-repository/blob/main/…` URLs, as `git-conventions.md` links `.github/release-please/config.json` (mdBook serves only `src/`, so a relative `../adr/` link is dead on GitHub Pages)
- [x] Verify `just check` and `just test` pass with the five existing root ADRs and the 57 existing bare citations resolving
- [x] Run `eng:reviewing` (done 2026-09-19/20; findings amended into the phase commit) with the reviewer set on this phase's diff and amend the findings into the commit

#### Phase 2: Move the crate as a root peer, unchanged in behaviour

**Gate: H4** — resolve before starting this phase.

Three commits: `feat(dsp-cli): move dsp-cli from dsp-incubator into the workspace` (the
verbatim copy), `chore(dsp-cli): format under the workspace rustfmt configuration`, and
`build(dsp-cli,docs): register the crate and harmonize it with the workspace` (the enumerated
edits, which also touch repo-level documents). Version stays 0.2.1; nothing is published.
`.commitlintrc.yml` has no scope allowlist, so the `dsp-cli` scope is valid before commit 3
adds it to the advisory tables.

- [x] Commit 1: copy `src/`, `tests/` (including `tests/snapshots/` and `tests/fixtures/`), `docs/topics/`, `docs/adr/`, `CHANGELOG.md`, `README.md`, `LICENSE`, `idea.md`, `CONTEXT.md`, `CLAUDE.md`, `Cargo.toml` and `.gitignore` entries from the incubator's `dsp-cli/` at its current `main` commit to `dsp-cli/` at the repository root, and record that source commit SHA in the PR description and in the dsp-cli/ADR-0014 amendment; do not copy `prototype.json`, `.envrc`, `.env.example`, `rust-toolchain.toml`, `justfile`, `Cargo.lock`, `docs/PROJECT_PLAN.md`, `docs/BACKLOG.md`, `docs/dev/`, `docs/design/`
- [x] Commit 1: add `"dsp-cli"` to `[workspace] members` in the root `Cargo.toml` so commit 2's `just fmt` covers the crate, and let Cargo regenerate the root `Cargo.lock` (the incubator's `Cargo.lock` is not copied, so dsp-cli's dependencies re-resolve against the workspace; the Phase 2 test run is the check that the new resolution behaves)
- [x] Commit 2: run `just fmt` so the crate matches `.rustfmt.toml` (expect ~96 files to change) and nothing else
- [x] Commit 3: in `dsp-cli/Cargo.toml`, keep `version = "0.2.1"`, `edition = "2024"`, `rust-version = "1.92"` literal (never `version.workspace = true` and never `publish = false`, the pattern every other member uses); set `repository = "https://github.com/dasch-swiss/dsp-repository"` and `homepage = "https://github.com/dasch-swiss/dsp-repository/tree/main/dsp-cli"`; remove the comment that defers the metadata bump; leave `publish` unset (this is the one crate here that publishes)
- [x] Commit 3: route `serde`, `serde_json`, `thiserror`, `tracing`, `tracing-subscriber` and `insta` through `[workspace.dependencies]` with `.workspace = true`; write `tracing-subscriber = { workspace = true, features = ["fmt"] }` so the `fmt` feature stays crate-local (features beside `workspace = true` are additive); keep `chrono` crate-local as the one exception, because dsp-cli needs `default-features = false` and `default-features` cannot be overridden beside `workspace = true` the way features can (recorded as a comment in the manifest); keep every other dependency crate-local, `url` included
- [x] Commit 3: fix the 6 rustdoc warnings (`resolve_password` and `run_from_line` private-item links; four `DspClient::list_resources` / `describe_resource` intra-doc links) so `cargo doc --no-deps --all-features` is warning-free
- [x] Commit 3: in the `fn dsp() -> Command` helper at `tests/cli.rs:45-53` (the single construction site every test uses), add `.env("NO_COLOR", "1")`, `.env_remove("CLICOLOR_FORCE")` and `.env_remove("CLICOLOR")` beside the existing `TERM=dumb`, so help snapshots do not depend on the caller's terminal environment; do not add a `color` attribute to `Cli` in `src/cli/mod.rs`; keep `cargo_bin("dsp")` confined to that helper and to the equivalent single helper in `tests/docs.rs` (the two places the Bazel migration re-points to runfiles)
- [x] Commit 3: mark every test in `tests/live_*.rs` `#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]` (placed after `#[test]`) so `cargo nextest run --all-features` reports them as skipped instead of vacuously passed
- [x] Commit 3: move the 13 verbatim copies of `require_env` / `optional_env` into `tests/common/mod.rs` (keep `#![cfg(feature = "live")]` at the top of every live file; add `mod common;` to each), keeping `require_env`'s `Option<String>` return so every `Some(v) => v, None => return` call site is unchanged, and make it panic with the variable name instead of returning `None` when `DSP_LIVE_STRICT=1` is set
- [x] Commit 3: add `.github/scripts/check-live-tests-ignored.sh` (every `#[test]` in `dsp-cli/tests/live_*.rs` must carry an `#[ignore` attribute; print offenders; exit non-zero) with a `.test.sh` against a temporary tree, wired into `just check` and `just test` like the other gates
- [x] Commit 3: rewrite every citation of a dsp-cli ADR to the qualified form `dsp-cli/ADR-NNNN` across `dsp-cli/src`, `dsp-cli/tests`, `dsp-cli/docs`, `dsp-cli/CHANGELOG.md`, `dsp-cli/README.md`, `dsp-cli/CONTEXT.md`, `dsp-cli/idea.md`, `dsp-cli/CLAUDE.md` and `dsp-cli/Cargo.toml` (baseline: 175 files carry bare `ADR-NNNN`), leaving relative links such as `docs/adr/0009-testing-strategy.md` as they are; do this before writing any text that cites a root ADR from inside `dsp-cli/`; the one citation inside `docs/topics/*.md` ships in the binary, so rephrase that line to not cite an ADR rather than show end users a repository path
- [x] Commit 3: amend `docs/adr/0002-areas-at-the-repository-root.md`: add `dsp-cli/` as the sixth root with its own rationale clause (the platform's command-line client, VRE via DSP-API today, the Deposit, Archive and Access moduliths later, over the wire; unlike Mosaic, Vitrinli and Chischtli it is used by no area and is a client of every area); carve it out of the dependency-arrow sentence on line 29 (areas never depend on it, it depends on no area crate); allow a `shared-*` dependency only if that crate is itself published to crates.io; state that a command-line client is not a user-facing surface in ADR-0004's sense; state that it lands at the root now, independent of DEV-7268; add the three rejected placements (`modules/dsp-cli/`, `tools/`, inside an area) to Considered Options with their reasons; record under Consequences that `dsp-cli/Cargo.toml` plus a `dsp-cli/Cargo.lock` remain the publishing manifest after ADR-0001 removes the root manifests, that dsp-cli's live tests become a `manual`, `no-remote-exec` test target, and that the drift stack stays a local-execution job
- [x] Commit 3: add a dated one-line note to `docs/adr/0004-hypermedia-frontends.md` Consequences: a command-line client is outside this decision's scope, see ADR-0002's dsp-cli clause
- [x] Commit 3: amend `dsp-cli/docs/adr/0014-migration-to-dsp-repository.md` with a dated section recording how the migration was executed (root peer, single crate, independent version line, colocated ADRs with qualified citations, no history import, the incubator source SHA) and citing root ADR-0002 and ADR-0006
- [x] Commit 3: amend `dsp-cli/docs/adr/0005-rust.md`, `0011-distribution-and-discoverability.md` and `0015-update-check-and-self-update.md` where they name `dsp-incubator` as the repository, pointing at the new home; keep historical statements historical
- [x] Commit 3: rewrite `dsp-cli/CLAUDE.md` to the shape of `modules/dpe/CLAUDE.md`: what the crate is, build and test commands via `just`, the architecture paragraph, the non-obvious constraints (local `MockDspClient` per action file, no `unwrap` outside tests, vocabulary rules from `CONTEXT.md`), and a documentation index; drop the phase-by-phase history
- [x] Commit 3: add a `### dsp-cli` component entry to `ARCH-MAP.md` in the shape of the existing entries: Paths `:(glob)dsp-cli/**`; Purpose; Key entities `DspClient`, `HttpDspClient`, `Renderer`, `Diagnostic`, `Config`; Public interface: the `dsp` command surface and the crates.io crate; Local-context kit (7 files): `dsp-cli/CLAUDE.md`, `dsp-cli/CONTEXT.md`, `dsp-cli/src/cli/mod.rs`, `dsp-cli/src/client/mod.rs`, `dsp-cli/src/render/mod.rs`, `dsp-cli/src/diagnostic.rs`, `dsp-cli/docs/adr/0008-internal-architecture.md`; Depends on: nothing in the workspace; third-party clap, reqwest, serde, url, insta, wiremock; Used by: — (top of the dependency graph; a published binary); Boundary rules as separate bullets with their own tags: no dependency on an area crate (**static-analysis**, `cargo publish -p dsp-cli --dry-run` in `check.yml`; **structure** after ADR-0001), a `shared-*` dependency only if published (same gate), over-the-wire integration only (**review**), live tests never in the default suite (**static-analysis**, `check-live-tests-ignored.sh`); Durable state: `~/.config/dsp-cli/auth.toml`, single writer dsp-cli
- [x] Commit 3: add a banned-constructs row to `ARCH-MAP.md`: "A dsp-cli dependency on an area crate | Turns the CLI into a second, out-of-process consumer of code meant to run inside one area's modulith on its own origin | Call the area's public HTTP surface, as any external client does | static-analysis (`cargo publish --dry-run`) → structure (ADR-0001)"; update the Overview sentence and `last_verified_commit`
- [x] Commit 3: add dsp-cli to root `CONTEXT.md` under "Shared infrastructure (not bounded contexts)" with the ADR-0002 clause and the distinguishing sentence (no area crate ever depends on dsp-cli; it depends on their public HTTP surface only), pointing at `dsp-cli/CONTEXT.md`; extend the existing **Project** Flagged-ambiguity bullet with the VRE meaning and its Resolution; add one bullet each, with Avoid and Resolution, for **Resource**, **Representation**, **Field**, **Value** and **Vocabulary**
- [x] Commit 3: add `dsp-cli` to the crate scopes in `CONVENTIONS.md` and `docs/src/git-conventions.md`
- [x] Commit 3: update `docs/src/repo_structure.md` (tree and naming table: `dsp-cli` is a single crate named after the product, an explicit exception to `{module}-{role}`), `docs/src/SUMMARY.md` (new `## dsp-cli` section), `docs/src/decisions.md` (link `dsp-cli/docs/adr/`), root `CLAUDE.md` Project Overview and `README.md`
- [x] Commit 3: add `docs/src/dsp-cli/architecture.md` (from dsp-cli/ADR-0008 and the CLAUDE.md architecture paragraph), `docs/src/dsp-cli/testing-strategy.md` (from dsp-cli/ADR-0009 and the incubator's `docs/dev/testing-strategy.md`, including the strict live mode and the `.env.example` guidance), and `docs/src/dsp-cli/usage.md` (install, `dsp docs`, server shortcuts, auth; short, pointing at the README)
- [x] Commit 3: merge dsp-cli's justfile recipes into the root `justfile` under `[group('dsp-cli')]`: `dsp-cli-run *args`, `dsp-cli-test-live` (`DSP_LIVE_STRICT=1 cargo nextest run -p dsp-cli --features live --run-ignored only -E 'binary(/^live_/) & !binary(live_update_check)'`), `dsp-cli-snap-review`
- [x] Commit 3: add `pkgs.cargo-nextest` and `pkgs.cargo-deny` to `buildInputs` in `flake.nix` beside `pkgs.cargo-watch` (line 67), with version comments (0.9.144 and 0.20.2 at the locked nixpkgs revision), and add `cargo binstall -y cargo-nextest@0.9.144` and `cargo binstall -y cargo-deny@0.20.2` to `install-requirements` in the `justfile`
- [x] Commit 3: extend the Bash allowlist in `.claude/settings.json` with the new `just dsp-cli-*` recipes, `cargo nextest`, `cargo deny`, `cargo publish --dry-run` and `docker compose` (read-only and local commands; pushes and `gh` writes stay prompted)
- [x] Run `cargo deny --manifest-path dsp-cli/Cargo.toml --config deny.toml check` once and resolve any license or advisory finding in dsp-cli's dependency tree (cargo-deny is not wired into CI; that stays out of scope) — ran 2026-09-19: `CDLA-Permissive-2.0` on `webpki-root-certs`, advisories on `rustls` and `h2`, all present in the lockfile at the base commit before dsp-cli existed
- [ ] Resolve those three pre-existing `cargo deny` findings (deny allow-list or a workspace-wide lockfile bump): a repository policy decision, deferred out of this plan
- [x] File Linear issues in team DEV for the two dsp-cli backlog entries not fixed in Phase 5 (language-selection policy for multilingual labels; shared constant for the `count_cost` / `count_caveat` disclosure strings) and for the deferred `reqwest` → `ureq` alignment, so nothing in `docs/BACKLOG.md` is lost — filed 2026-09-20: DEV-7338, DEV-7339, DEV-7340
- [x] Verify `nix develop --command just check` and `nix develop --command just test` pass (including `check-adr-refs.sh` over the rewritten citations and `check-live-tests-ignored.sh`), `cargo nextest run --locked --all-features --all-targets` reports the live tests as skipped, and `cargo hack --feature-powerset --exclude-no-default-features check` passes with the `live` feature — all verified at the branch tip except `cargo hack`, which is not on the dev-shell PATH; the `hack` job in `check.yml` covers it on the PR
- [x] Verify commit 3's diff contains only the enumerated edits and that each of the three commits passes `just commit-lint` — messages pass; the one-commit cap is lifted by `allow-many-commits` in the PR body
- [x] Run `eng:reviewing` (done 2026-09-19/20; findings amended into the phase commit) with the reviewer set on this phase's three commits and amend the findings into the commit they belong to

#### Phase 3: Release wiring for an independent dsp-cli version line

One commit, `chore(ci): release dsp-cli on its own version line`.

- [x] In `.github/release-please/config.json`, add package `"dsp-cli"` with `release-type: simple`, `component: dsp-cli`, `package-name: dsp-cli`, `changelog-path: CHANGELOG.md`, `include-component-in-tag: true`, `extra-files: [{type: toml, path: Cargo.toml, jsonpath: $.package.version}]` (paths relative to the package directory); change the `"."` package's `exclude-paths` to `[".github", "dsp-cli"]`; set top-level `separate-pull-requests: true`
- [x] In `.github/release-please/manifest.json`, add `"dsp-cli": "0.2.1"`
- [x] In `.github/workflows/release-please.yml`, replace the single "Find release PR branch" / "Update Cargo.lock" steps (lines 40-53) with: an output `branches` on the `release-please` job from `gh pr list --repo "${{ github.repository }}" --label "autorelease: pending" --json headRefName --jq '[.[].headRefName]'` (drop the dead `steps.release.outputs.pr` branch, a single-PR output that manifest mode with two packages does not set), compacted with `jq -c .` before it is written to `$GITHUB_OUTPUT` because gh pretty-prints a non-empty array and a multi-line `GITHUB_OUTPUT` value is invalid, and a new job `amend-lockfile` with `needs: release-please`, `if: needs.release-please.outputs.branches != '[]'`, `strategy: { fail-fast: false, matrix: { branch: ${{ fromJSON(needs.release-please.outputs.branches) }} } }`, whose steps are the existing checkout (`ref: ${{ matrix.branch }}`, `fetch-depth: 0`, the `GH_TOKEN`), stable toolchain, `cargo update --workspace`, diff check, `git commit --amend --no-edit`, `git push --force-with-lease`, unchanged in body
- [x] Reconcile `dsp-cli/CHANGELOG.md` with release-please's writer: keep the existing Keep-a-Changelog entries for 0.1.0–0.2.1 below a `# Changelog` heading that release-please prepends to; drop the versioning paragraph release-please will not maintain
- [x] Run `cargo publish -p dsp-cli --dry-run --locked` locally once (cargo#11148 is fixed since Cargo 1.68, so `--locked` respects the workspace lockfile) and confirm the packaged file list includes `docs/topics/*.md`, `README.md`, `LICENSE`, `CHANGELOG.md`
- [x] Add `.github/workflows/publish-dsp-cli.yml`: trigger `release: published`, `if: startsWith(github.event.release.tag_name, 'dsp-cli-v')`, `permissions: id-token: write, contents: read`, `environment: crates-io` (optional in crates.io's model; used as defence in depth, so the trusted-publisher entry in H1 names it too), steps: checkout, stable toolchain, `cargo publish -p dsp-cli --dry-run --locked`, `rust-lang/crates-io-auth-action@v1`, `cargo publish -p dsp-cli --locked` with `CARGO_REGISTRY_TOKEN` from the action's `token` output
- [x] Add an unconditional `publish-dry-run` job to `check.yml` on the stable toolchain (`dtolnay/rust-toolchain@stable`, like the `hack` job) running `cargo publish -p dsp-cli --dry-run --locked`, so packaging breakage (the `include` allowlist, `include_str!` topics, an unpublished path dependency) fails the PR that causes it
- [x] Document the dsp-cli release path in `docs/src/deployment.md` (second release-please package, tag format, the `release-please--branches--main--components--dsp-cli` branch, trusted publishing, the double-bump rule for commits that also touch `Cargo.lock`, `Release-As:` for deliberate version choices) and in `docs/src/git-conventions.md` (keep dsp-cli commits under `dsp-cli/**` where possible)
- [x] Run `eng:reviewing` (done 2026-09-19/20; findings amended into the phase commit) with the reviewer set on this phase's diff and amend the findings into the commit

#### Phase 4: Drift CI against a containerized dsp-api

One commit, `chore(ci): run dsp-cli live tests against a pinned dsp-api stack`.

- [x] Re-point `dsp-cli/tests/live_vocabulary.rs` at fixture data: read the project from `DSP_TEST_VOCAB_PROJECT`, assert structure (root count ≥ 1, a tree with depth ≥ 2, every node has a label) rather than the exact geoarch counts, and replace `.expect()` on project resolution with the skip-or-strict handling from `tests/common/mod.rs`
- [x] Add `dsp-cli/ci/stack/docker-compose.yml` (services `db` = `daschswiss/apache-jena-fuseki:${DB}`, `api` = `daschswiss/knora-api:${API}` with `KNORA_WEBAPI_ALLOW_SPARQL_PASSTHROUGH=true`, `KNORA_WEBAPI_KNORA_API_EXTERNAL_HOST=0.0.0.0`, `KNORA_WEBAPI_KNORA_API_EXTERNAL_PORT=3333` and the triplestore connection settings copied from dsp-api's own `docker-compose.yml` `api` service at the pinned tag — note the Fuseki dataset is `dsp-repo`, **not** `knora-test` as first drafted, because `fuseki-functions.sh` defaults `REPOSITORY=dsp-repo` and dsp-api's justfile runs the init script with no arguments; dsp-api's OTEL, Pyroscope and sipi/ingest settings are deliberately not copied, since this stack runs neither a collector nor those services; health checks copied from the same file; `depends_on: db: condition: service_healthy` on `api`; no sipi or ingest) and `dsp-cli/ci/stack/stack.env` with `API=v38.1.0` and `DB=v38.1.0`, plus a `README.md` naming the pin as this repository's and how to bump it
- [x] Add `dsp-cli/ci/stack/load-fixtures.sh` that sparsely checks out `dasch-swiss/dsp-api` at the **`API`** tag in `stack.env` (the fixtures are dsp-api's, so the checkout is keyed on `API`, never `DB` — the two pins are independent) and runs `fuseki-init-knora-test.sh` against the running `db` before `api` is started; **three** sparse paths are needed, not two — `test_data/`, `modules/webapi/scripts/` and `modules/webapi/src/main/resources/knora-ontologies/`, because the init script also uploads five ontologies from the last of these — and it must run with the working directory set to `modules/webapi/scripts/`, since it uses paths relative to itself and does `source fuseki-functions.sh`
- [x] Add `dsp-cli/ci/stack/token.sh` that logs in as `root@example.com` / `test` via `POST /v2/authentication` and exports `DSP_TOKEN`, and logs in as a fixture non-admin user for `DSP_TEST_NON_ADMIN_TOKEN`, which `live_sparql_query.rs:192` uses for its permission-boundary case
- [x] Add `.github/workflows/dsp-cli-drift.yml` triggered unconditionally on `pull_request`, `schedule` (nightly) and `workflow_dispatch`, with `permissions: contents: read`; job `changes` (pull_request only): checkout with `fetch-depth: 0`, `git diff --name-only "$BASE_SHA...$HEAD_SHA"` and an output `dsp_cli=true` when any path matches `^dsp-cli/` or the workflow file itself
- [x] Add job `pinned` to that workflow: `needs: [changes]`, `if: github.event_name == 'pull_request' && needs.changes.outputs.dsp_cli == 'true'`, `timeout-minutes: 30`; steps: checkout, `docker compose --env-file dsp-cli/ci/stack/stack.env up -d --wait db`, load fixtures, `up -d --wait api`, `wait-for-api.sh` polling `/health`, token, then `just dsp-cli-test-live` with `DSP_TEST_SERVER=http://localhost:3333`, `DSP_TEST_PROJECT=0801`, `DSP_TEST_CLASS_IRI=http://0.0.0.0:3333/ontology/0801/beol/v2#page`, `DSP_TEST_VOCAB_PROJECT=0001`; dump `docker compose logs api` on failure; append the job's wall time to `$GITHUB_STEP_SUMMARY`
- [x] Add job `gate` to the same workflow: `needs: [changes, pinned]`, `if: always() && github.event_name == 'pull_request'`, succeeding when `changes` succeeded and `pinned` succeeded or was skipped, failing otherwise (the incubator `ci.yml` `all-green` shape), so `dsp-cli-drift / gate` can be made a required check without blocking PRs outside `dsp-cli/**`
- [x] Add job `latest` to the same workflow: `if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'`, same steps as `pinned` with `API=latest`, `DB=latest`; on failure, write the failing test names and the dsp-api image digest to the job summary so the red run explains itself; no issue is created (GitHub Issues are disabled; Linear is the tracker)
- [x] Add `just dsp-cli-stack-up`, `just dsp-cli-stack-down` and `just dsp-cli-stack-fixtures` recipes wrapping the compose and fixture scripts for local use, depending on a new `[private] _check-docker` guard recipe in the style of `_check-node`, and allowlist them in `.claude/settings.json`
- [x] Amend `dsp-cli/docs/adr/0009-testing-strategy.md`: option C4 (stack-managed tests) adopted, triggers "CI without credentials" and "isolation from a shared environment" met; live tests are `#[ignore]` in the default suite (gated by `check-live-tests-ignored.sh`), strict in the drift job; the pin lives in `dsp-cli/ci/stack/stack.env`
- [x] Correct the "Fuzz Testing" section of `docs/src/deployment.md`: `fuzz.yml` does not create a GitHub issue on a crash (Issues are disabled on the repository); it uploads the crash input as the `fuzz-crashes-<target>` artifact (90 days) and the run goes red, and a person files a Linear issue from the artifact; same signal model as the drift job's nightly run
- [x] Document the drift job in `docs/src/dsp-cli/testing-strategy.md` and `docs/src/deployment.md` (what it detects, including the JSON-LD key drift an OpenAPI diff cannot see; why `gate` and not `pinned` is the required check; where to look when the nightly run is red and that a Linear issue is then filed by hand; that `workflow_dispatch` exists because scheduled runs are delayed under load and disabled after 60 idle days; how to bump the pin)
- [x] Verify the `pinned` job passes on the PR well inside its 30-minute budget (target under 12 minutes) — first CI run on PR #409 (2026-09-20): passed in 2 min 39 s; if it does not, first measure whether fixture loading or image pulls dominate, and only then consider caching the loaded Fuseki volume keyed on `stack.env` plus the fixture script's hash
- [x] Run `eng:reviewing` (done 2026-09-19/20; findings amended into the phase commit) with the reviewer set on this phase's diff and amend the findings into the commit

#### Phase 5: Hardening before the first release from the new home

One commit, `fix(dsp-cli): close the pre-migration security and correctness backlog`. The
type is `fix` (four of the five items correct behaviour users of 0.2.1 can hit today, so
they are not in-branch fixups); the `DSP-Client` header rides along, and the `Release-As`
footer decides the version.

- [x] Auth-cache load failures: log a body-free message at `warn` and the `toml` error (`%e`) at `debug` only, so a malformed `auth.toml` never echoes a token fragment at default verbosity; add a unit test that a malformed cache with a token-like line produces no token bytes on stderr at verbosity 0. **There are fifteen load sites, not the four named here** (`auth/status.rs`, `auth/token.rs`, `vre/sparql.rs`, three in `vre/project.rs`, three in `vre/data_model.rs`, two each in `vre/resource_type.rs`, `vre/vocabulary.rs` and `vre/resource.rs`); nine of them use a multi-line `tracing::warn!` that a single-line grep does not find
- [x] `--server` validation in `Config::resolve`: parse the expanded value with `url::Url` (reached as `reqwest::Url` — reqwest re-exports it, so no new direct dependency and no root `Cargo.lock` change); accept `https://` and **loopback or unspecified** `http://` — not loopback alone, because the `local` shortcut expands to `http://0.0.0.0:3333` and `0.0.0.0` is the unspecified address, so a loopback-only rule would break the primary developer shortcut (`localhost`, any IPv4/IPv6 loopback, `0.0.0.0` and `::`; `Url::host_str()` returns IPv6 hosts **with** brackets, contrary to the note below, so the brackets are stripped before `IpAddr` parsing, with a unit test for `http://[::1]:3333`); refuse any other `http://` with a usage diagnostic naming the risk (a bearer token in cleartext), overridable with `--allow-insecure-server` or `DSP_ALLOW_INSECURE_SERVER=1` (a global clap flag, so it appears in every subcommand's help and all twenty-nine help snapshots change by that one line; `ArgAction::SetTrue`'s default `BoolValueParser` accepts only `true`/`false`, so `BoolishValueParser` is set explicitly for `=1` to parse); sanitize control characters from the server value before it appears in any diagnostic; snapshot the refusal in all five formats. `Config::resolve` gains a second parameter, threaded through its 39 call sites
- [x] `DSP_TOKEN` and a CWD `.env`: when `DSP_SERVER` comes from a `.env` file in the working directory and `DSP_TOKEN` comes from the process environment, print a one-line `warn` naming both sources before the first authenticated request, so the composed redirection path is visible; document the precedence in `dsp docs connecting`. Provenance is knowable because `main` snapshots both variables before `dotenvy::dotenv()` runs and dotenvy never overrides an already-set variable; `TokenOrigin` was deliberately not extended, since it tracks env-vs-cache, a different axis
- [x] BrokenPipe: leave `Diagnostic::from(io::Error)` (`src/diagnostic.rs:34-38`) as it is and handle `ErrorKind::BrokenPipe` at the stdout write sites by exiting 0 with no stderr output, every other write error still becoming `Internal`; add a test that pipes a large stdout into a closed reader. **Eight sink sites, and the list here is wrong in two ways**: `vre/project.rs` is not one (it has no direct stdout write — project dump goes through the renderer), and `actions/auth/token.rs` is one it omits. Implemented as a `BrokenPipeWriter` adapter wrapping each stdout handle where it is constructed, so no renderer-trait or public signature changed. The test must render more than a pipe buffer (~64 KiB) or it passes on unfixed code: every embedded `docs` topic is under 9 KiB, so the test relays a 512 KiB wiremock body through `vre sparql query` instead
- [x] Send `DSP-Client: dsp-cli/<version>` on every outgoing DSP-API request beside the existing `User-Agent` (DEV-6844's client-identification convention), asserted in the wiremock tests that already check `User-Agent`. Attached via `default_headers` on all three DSP-API client builders; the crates.io update-check client in `src/update/mod.rs` deliberately does not send it
- [x] Extract the duplicated 80-character truncate-with-ellipsis logic in `src/client/http.rs` into one helper beside `classify_sparql_status`, behaviour-preserving, covered by the existing wiremock error-path tests; leave the eight bespoke status arms as they are (`map_unexpected_status` documents why). **Five call sites, not the three named here.** The helper returns one `String`, since every site concatenated the prefix and suffix anyway
- [x] Add a `CHANGELOG.md` `Unreleased` section describing the user-visible changes (the `--server` scheme refusal is the one behaviour change users can hit) and the new-home note. Written in the file's own Keep-a-Changelog shape, per dsp-cli/CLAUDE.md; release-please writes `### Features` / `### Bug Fixes` and **prepends rather than merges**, so the release PR must drop this section by hand
- [x] Give this commit the footer `Release-As: 0.3.0`. It must share a paragraph with `Co-Authored-By:` — a blank line between them makes it a separate paragraph and git's trailer parser, which reads only the last one, does not see it (verify with `git log -1 --format='%(trailers)'`)
- [x] Run `eng:reviewing` (done 2026-09-19/20; findings amended into the phase commit) with the reviewer set plus `eng:review:security-reviewer` on this phase's diff and amend the findings into the commit
- [x] Rewrite PR #409's body with the repository template for the whole change (it was opened for the plan alone), retitle it `feat(dsp-cli): move dsp-cli into the workspace with release wiring and dsp-api drift CI`, tick `allow-many-commits`, add a "Review Notes" section saying the commits are the plan's phases and are meant to be reviewed one by one, copy the Human Actions table into the body, and mark the PR ready for review (done 2026-09-20)
- [x] (done 2026-09-20 ahead of the rebase: release `dsp-cli-v0.2.1` on `main`'s tip 78c4a7c7, which no dsp-cli commit precedes, while only `dpe-release-publish.yml` listened on `release:` and it filters on `v`) Last step before the PR merges, after its final rebase onto `main` (it is stacked on #391 until that merges) and while `publish-dsp-cli.yml` does not yet exist on `main`: create the GitHub Release `dsp-cli-v0.2.1` from a tag on the PR's merge base on `main` (`gh release create dsp-cli-v0.2.1 --target <merge-base-sha> --title "dsp-cli 0.2.1 (baseline)" --notes "Baseline for release-please. Not a publish; 0.2.1 was published from dsp-incubator."`), so release-please has the baseline the manifest names; a tag-only fallback is acceptable only if the first run demonstrably finds it

#### Phase 6: Check the release proposal (post-merge step)

Tracked in Linear as DEV-7343, a sub-issue of the umbrella DEV-7341 (project "Move dsp-cli to dsp-repository").

**Gate: H6** — resolve before starting this phase.

- [ ] Verify on the first `main` run after merge that release-please opens a separate `dsp-cli` release PR on branch `release-please--branches--main--components--dsp-cli` whose changelog contains the move commit and nothing older, and that the `.` release PR, if recreated under its own components branch, has unchanged content
- [ ] Verify the pending `dsp-cli` release PR proposes `0.3.0`, the tag `dsp-cli-v0.3.0`, `dsp-cli/Cargo.toml` at `0.3.0`, a changelog whose entries are only dsp-cli commits since the move, an unchanged root version, and that the `amend-lockfile` matrix job amended its `Cargo.lock`

#### Phase 7: First release from the new home, then archive the prototype (post-merge step)

Tracked in Linear as DEV-7347 (release and publish) and DEV-7349 (archive the prototype), sub-issues of DEV-7341. The two decisions the execution surfaced are DEV-7345 (pin one nightly rustfmt on both sides) and DEV-7346 (the pre-existing `cargo deny` findings).

**Gate: H2** (which presupposes H1) — resolve before starting this phase.

- [ ] Verify `publish-dsp-cli.yml` ran on the `dsp-cli-v0.3.0` release event, `cargo publish` succeeded, and `https://crates.io/api/v1/crates/dsp-cli` reports `max_version` `0.3.0` with `repository` pointing at dsp-repository
- [ ] Verify in a fresh container that the published crate installs and runs: `docker run --rm rust:1.93-bookworm sh -c 'cargo install dsp-cli && dsp --version && dsp docs && dsp vre project list --server stage'` reports 0.3.0 and exits 0
- [ ] Verify a 0.2.1 binary's interactive update check (`src/update/`) advises 0.3.0 from the crates.io sparse index (it reads crates.io, not the repository, so no change is expected)
- [ ] In the `dsp-incubator` checkout, run the `/archive-prototype dsp-cli` command documented in its `CONTRIBUTING.md` (locate the command's definition first; it is not under `.claude/commands/`), which moves `dsp-cli/` under `archive/`, removes `.github/workflows/dsp-cli.yml` and updates the index; add a first paragraph to `archive/dsp-cli/README.md` pointing at `dsp-repository/dsp-cli` and the release that moved it, and open the PR
- [ ] In `dsp-incubator`, close the stale Dependabot PR #170 (`quinn-proto` bump in `/dsp-cli`) with a comment pointing at the new home

## Human Actions

| Id | Action | Who | When | Why not the agent | Issue |
|----|--------|-----|------|-------------------|-------|
| H1 | Configure a crates.io Trusted Publisher for the `dsp-cli` crate: repository `dasch-swiss/dsp-repository`, workflow `publish-dsp-cli.yml` (the filename is final once the PR merges; renaming it later silently breaks publishing), environment `crates-io` (optional in crates.io's model; the workflow declares it, so the entry must name it); GitHub creates the environment on the workflow's first run, so creating it beforehand matters only if protection rules are wanted | a crate owner (`BalduinLandolt` or a member of `github:dasch-swiss:everyone-private`) with repo admin | after the PR merges, before H2 | crates.io owner credentials and repository settings | DEV-7342 |
| H2 | Review and merge the `dsp-cli` release PR checked in Phase 6, once H1 is done | Ivan | after Phase 6, after H1 | a release is a deliberate act; the agent opens PRs, a person merges them | DEV-7347 |
| H3 | Add the `dsp-cli-drift / gate` check to the `main` ruleset as required once it has been green for a week | Ivan | after the PR merges | repository ruleset settings | DEV-7344 |
| H4 | Tell Balduin that dsp-cli in the incubator is frozen from the moment the copy is taken: no merges to `dsp-incubator/dsp-cli/` until the archive lands in Phase 7; anything pending is re-opened against `dsp-repository/dsp-cli/` | Ivan | before Phase 2 (the copy) starts | a coordination message between people | done 2026-09-19, no issue |
| H5 | After the first trusted-publishing release succeeds, review the crate's owners on crates.io (the `everyone-private` team stays; decide whether the personal owner stays) and revoke any personal crates.io API token that was used for manual `cargo publish` | Ivan with Balduin | after Phase 7 | crates.io account settings | DEV-7348 |
| H6 | Review and merge PR #409 (after #391 has merged and the branch is rebased onto `main`) | a dsp-repository reviewer | after Phase 5, before Phase 6 | ordinary code review and merge | DEV-7330 |

## Acceptance Criteria

- [x] Root ADR-0006 exists, `check-adr-refs.sh` runs in `just check`, and it fails on a dangling or unqualified-and-unresolvable ADR reference
- [x] `dsp-cli/` exists at the repository root and is a workspace member; `just check` and `just test` pass at the branch tip and from Phase 2 commit 3 on (commits 1 and 2 of Phase 2 are gated by build plus tests: the verbatim copy is unformatted and cites its own ADRs bare by construction), and `check.yml`, `test.yml` and `commit-hygiene.yml` pass on the PR and on `main` after merge — all 19 checks green on PR #409 (2026-09-20, second run); on `main` after merge still to confirm
- [x] `cargo nextest run --locked --all-features --all-targets` reports every `live_*` test as skipped, not passed, and `check-live-tests-ignored.sh` fails when a live test lacks `#[ignore]`
- [ ] `just dsp-cli-test-live` with no `DSP_TEST_*` variables fails (strict mode), and passes against the pinned stack
- [x] `git grep -nP '(?<![/\w])ADR-\d{4}\b' -- dsp-cli ':!dsp-cli/docs/adr'` returns only references that resolve to `docs/adr/NNNN-*.md` (root ADRs); every dsp-cli ADR citation is in the `dsp-cli/ADR-NNNN` form
- [x] ADR-0002 is amended with dsp-cli's own rationale and the dependency-arrow carve-out, ADR-0004 carries the pointer, and `ARCH-MAP.md` (entry, widened "Colocated docs" line, banned-constructs row), root `CONTEXT.md` (bullet plus six Flagged-ambiguity entries with Resolutions), `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`, `docs/src/SUMMARY.md`, `docs/src/decisions.md`, root `CLAUDE.md` and `README.md` name dsp-cli
- [x] `tests/cli.rs` passes with `CLICOLOR_FORCE=yes` exported
- [x] `cargo doc --no-deps --all-features` emits no warnings for `dsp-cli`
- [ ] release-please opens separate release PRs for `.` and `dsp-cli` (the latter on `release-please--branches--main--components--dsp-cli`); a commit touching only `dsp-cli/**` does not bump the root version; the first dsp-cli release PR lists the move commit and nothing older; the `amend-lockfile` job amends every pending release PR
- [x] `dsp-cli/Cargo.toml` carries a literal `version` and no `publish = false`, and `cargo publish -p dsp-cli --dry-run --locked` passes in `check.yml`
- [ ] `dsp-cli-v0.3.0` is published to crates.io by the workflow, not by hand, and the crate's `repository` metadata points at dsp-repository
- [ ] The drift workflow runs on every PR; `pinned` runs only when `dsp-cli/**` or the workflow changed and finishes inside its 30-minute budget (target under 12); `gate` reports success on PRs that do not touch dsp-cli; the `latest` job runs nightly, and a failing run names the failing tests and the image digest in its summary
- [x] `dsp` refuses a non-loopback `http://` server by default (loopback IPv6 included), never prints token bytes from a malformed `auth.toml` at default verbosity, exits cleanly on a closed pipe, and sends `DSP-Client`
- [x] `nix develop --command cargo nextest --version` and `--command cargo deny --version` work, and `install-requirements` installs both tools; the versions differ (nixpkgs at the locked revision: cargo-nextest 0.9.132, cargo-deny 0.19.0; binstall: 0.9.144, 0.20.2), disclosed in the `flake.nix` comments and accepted
- [ ] The incubator has `archive/dsp-cli/` with a pointer to the new home and no `dsp-cli.yml` workflow

## Dependencies & Risks

- crates.io owner action (H1) sits on the critical path to Phase 7; Phases 1–6 do not depend on it.
- dsp-api images `daschswiss/knora-api` and `daschswiss/apache-jena-fuseki` are public on Docker Hub; the drift job needs no secrets. Fixture loading fetches dsp-api at a tag from GitHub, so the job depends on GitHub availability only.
- ADR-0001 (Bazel with RBE) is expected soon and will remove the root Cargo manifests. dsp-cli lands Cargo-native first; the readiness table under Technical Considerations is the contract the ADR-0001 runbook picks up, and the publishing manifest decision is recorded in ADR-0002.
- PR #391 must merge first (it introduces `docs/specs/`, `shared/` and `check-shared-paths.sh`, which this plan builds on); DEV-7268's later `modules/` → `areas/` move touches the same repo-level documents this plan edits, so land this PR before or after that work, not interleaved.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| The 96-file reformat hides a behaviour change in the move commit | M | H | Three commits (copy, fmt, edits), reviewer diffs the third alone; `allow-many-commits` keeps them separate on `main` |
| Live tests keep passing vacuously in `test.yml`, masking drift forever | H (today) | H | `#[ignore]` on every live test, kept by `check-live-tests-ignored.sh`, plus `DSP_LIVE_STRICT=1` in the drift job; acceptance criterion checks "skipped", not "passed" |
| Stack startup pushes the drift job past its 30-minute budget | M | M | fuseki + api only, no sipi/ingest; fixtures loaded by curl; `--wait` plus explicit health poll; wall time written to the step summary; measure before caching |
| A required path-filtered check deadlocks PRs outside `dsp-cli/**` | H (if done naively) | H | Unconditional workflow with a `changes` job; only the always-running `gate` job is made required (H3); the dry-run job runs unconditionally |
| A dsp-cli commit touching `Cargo.lock` bumps the root version too | H | L | Accepted and documented; `separate-pull-requests` keeps releases independent; the extra root changelog line is harmless pre-1.0 |
| The lockfile amend step handles only one release PR and the other fails `--locked` | H (with two packages) | M | The `amend-lockfile` matrix job over every `autorelease: pending` branch |
| release-please gathers the wrong history for the new package on its first run | M | M | A `dsp-cli-v0.2.1` GitHub Release on the PR's merge base, created before the publish workflow exists on `main`; `component` set explicitly; tag format verified before Phase 7 |
| A future dsp-cli dependency on a `shared-*` crate makes `cargo publish` fail (unpublished path dependency) | M (later) | M | Stated in the ADR-0002 amendment and tagged static-analysis in ARCH-MAP; the `publish --dry-run` job in `check.yml` fails the PR that adds it |
| Work lands in the incubator after the copy and diverges silently | L | M | H4 freeze gates Phase 2; source SHA recorded in the PR and in dsp-cli/ADR-0014 |
| `--server` scheme refusal breaks a user's local `http://` setup | M | M | Loopback exempt (IPv4 and IPv6); explicit `--allow-insecure-server` / `DSP_ALLOW_INSECURE_SERVER`; changelog entry; 0.3.0 is a pre-1.0 minor with a stated behaviour change |
| A stale bare `0001`–`0006` inside `dsp-cli/` resolves to a root ADR and passes the gate | L | L | One-time rewrite in Phase 2 before any root citation is added; residual stated in ADR-0006; review |
| Nightly `schedule` is delayed or auto-disabled after 60 idle days | L | M | `workflow_dispatch` beside `schedule`; documented in the testing-strategy page |
| Conflicts with #391 / DEV-7268 edits to `ARCH-MAP.md` / `CONTEXT.md` | M | L | Stacked on #391; sequence relative to later DEV-7268 work; both are documentation merges |
| Bazel migration (ADR-0001) removes the Cargo manifests `cargo publish` needs | H (soon) | M | Decided: `dsp-cli/Cargo.toml` + `Cargo.lock` stay as the publishing manifest; the readiness table maps every other Cargo-specific piece; prebuilt binaries are the follow-on the hermetic toolchain enables |
| A dsp-cli test locates the binary or its fixtures through Cargo-only environment variables in more than one place, multiplying the Bazel re-point | M | L | Binary resolution stays in one helper per test file (`fn dsp()` in `tests/cli.rs`); topics and snapshots stay inside `dsp-cli/` |

## Success Metrics

- Baseline (2026-09-18): 1005 unit + 379 integration tests, 356 snapshots; build 64 s, tests ~65 s; 6 rustdoc warnings; 175 files with bare `ADR-NNNN` under dsp-cli; 57 bare root citations in dsp-repository; 0 live tests run in any CI. Targets: test count unchanged or higher after Phase 2; 0 rustdoc warnings; 0 unresolvable ADR references; 13 live test binaries run strict in the drift job; drift job under 12 minutes inside a 30-minute budget.
- A dsp-api change that renames a JSON-LD key dsp-cli reads (for example `knora-api:textValueAsXml`) turns the nightly job red within 24 hours of `:latest` carrying it.
- Every dsp-cli release from Phase 7 on is produced by release-please and published by the workflow; no manual `cargo publish`.

## References

- dsp-cli migration decision: `dsp-incubator/dsp-cli/docs/adr/0014-migration-to-dsp-repository.md`; architecture `0008-internal-architecture.md`; testing `0009-testing-strategy.md` (option C4); distribution `0011-distribution-and-discoverability.md`; config precedence `0007-auth-and-environments.md`
- dsp-cli anchors: `tests/cli.rs:45-53` (`fn dsp()`), `src/cli/mod.rs:235-242`, `src/diagnostic.rs:34-38,131-142`, `src/config/mod.rs:52-80`, `src/client/http.rs:733,2209-2269,2634-2640,2996-3008,3046-3047,3514,3737`, `src/actions/auth/status.rs:50`, `src/actions/auth/token.rs:68`, `src/actions/vre/project.rs:289,1360`, `src/actions/vre/sparql.rs:243,275`, `src/actions/docs.rs:229,245`, `tests/live_sparql_query.rs:192`, `Cargo.toml:69` (`url`)
- Target layout: `dsp-repository/docs/adr/0001-bazel-builds-the-monorepo.md`, `0002-areas-at-the-repository-root.md` (lines 20, 29), `0003-one-modulith-per-area.md`, `0004-hypermedia-frontends.md` (line 8); `ARCH-MAP.md` (component entry shape, ≤7-file kit, "Colocated docs" line 518, banned-constructs table); root `CONTEXT.md` (line 15 heading, Flagged ambiguities); dsp-repository-design `CONTEXT-MAP.md` ("where to write decisions"); `flake.nix:61-62,67`; PR #391 (`docs/specs/README.md`, `shared/`)
- Release tooling: `.github/release-please/config.json`, `.github/workflows/release-please.yml:40-53` (lockfile amend step); release-please `docs/manifest-releaser.md` (`bootstrap-sha`, `separate-pull-requests`, `exclude-paths`, `include-component-in-tag`), `src/util/commit-split.ts` (path-based attribution; root `"."` receives all commits), `src/strategies/simple.ts` (`createIfMissing: false`), `src/util/branch-name.ts`; googleapis/release-please#2111, #1250; rust-lang/cargo#11148 (fixed by #11477, Cargo 1.68), #13309 (`include_str!` not auto-detected); bazelbuild/rules_rust#458; `rust-lang/crates-io-auth-action` `action.yml`/README; RFC 3691 (trusted publishing; environment optional)
- GitHub Actions: docs.github.com "Troubleshooting required status checks" (path-filtered workflow skips leave checks pending), "Events that trigger workflows" (`schedule` delays, 60-day disable); github.blog changelog 2026-01-30 (Compose 2.40.x on hosted runners); docs.docker.com `compose up --wait`; CVE-2025-30066 (`tj-actions/changed-files`)
- Drift stack: `dsp-api/modules/webapi/scripts/fuseki-init-knora-test.sh`, `wait-for-api.sh` (polls `/health`); `dsp-api/modules/webapi/src/main/resources/application.conf` (`app.allow-sparql-passthrough`); `dsp-api/test_data/project_data/admin-data.ttl` (root user); `dsp-api/test_data/project_ontologies/beol-onto.ttl:1410` (`:page`); `dsp-api/test_data/project_data/anything-data.ttl` (four list roots, project 0001); `dsp-tools/src/dsp_tools/resources/start-stack/docker-compose.yml` and `versions.env` (API `v38.1.0`, DB `v38.1.0`); the incubator's `.github/workflows/ci.yml` (`changes` / `all-green` jobs)
- dsp-cli routes: `dsp-incubator/dsp-cli/src/client/http.rs` (v2 JSON-LD hand-parsed; admin; v3 export and `resourcesPerOntology`; `POST /admin/sparql/query`); dsp-api Tapir endpoints `ResourcesEndpoints.scala`, `OntologiesEndpoints.scala`, `AdminListsEndpoints.scala`, `V3ProjectsEndpoints.scala`, `SparqlPassthroughEndpoints.scala`
- Toolchain docs: anstream `src/auto.rs` (`NO_COLOR` before `CLICOLOR_FORCE`); docs.rs assert_cmd 2.2.2 (`env_remove`); nexte.st filtersets reference and `--run-ignored`; Cargo reference "Workspaces" (additive features), "Specifying dependencies" (path deps need a registry version), "Publishing" (`include`); nixpkgs-unstable `a32edd76` (`cargo-nextest` 0.9.144, `cargo-deny` 0.20.2)
- dsp-api deprecation plan: `dasch-specs/specs/2026-07-24-dsp-api-surface-deprecation/01-refactor-dsp-api-surface-deprecation-plan.md` (DEV-6844; `DSP-Client` header)
- Institutional learnings: `dasch-specs/learnings/test-setup/docker-compose-dev-stack-startup-ordering.md` (load Fuseki before the API starts); `dasch-specs/learnings/integration-issues/openapi-required-field-with-backend-default.md`; `dasch-specs/learnings/configuration-errors/github-actions-composite-action-main-ref-pr-isolation.md`
- ADR conventions: Nygard 2011 (sequence, immutability, supersede); MADR (`adr.github.io/madr`, subdirectory-scoped numbering); log4brains monorepo ADR (2020, project stale since 2022); no maintained monorepo-aware ADR tool found on 2026-09-18
