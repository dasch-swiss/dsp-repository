---
plan: docs/specs/2026-09-18-dsp-cli-migration/01-feat-dsp-cli-migration-plan.md
target_repo: /Users/subotic/_github.com/dasch-swiss/dsp-repository (worktree .claude/worktrees/dsp-cli-move)
base_commit: 4f3247d8
branch: docs/dsp-cli-migration-plan
started: 2026-09-19
problem: >
  dsp-cli (binary `dsp`, 0.2.1 on crates.io) lived in the dsp-incubator prototype monorepo
  with no release automation and with live tests that were the only dsp-api drift detector
  yet never ran in CI and passed vacuously without a server. Its own ADR-0014 decided to move
  it into dsp-repository after the first publish. The plan moves it in as a root peer, gives
  it an independent release line with crates.io trusted publishing, adds a drift-CI job
  against a containerized dsp-api, records the decision-record convention the move needs
  (root ADR-0006), and closes the pre-migration security backlog before the first release
  from the new home.
status: in-progress
---

# Execution Journal: 01-feat-dsp-cli-migration-plan

Written by five `eng:work-orchestrating` rounds (2026-09-19 to 2026-09-20). The sections below
follow `eng/references/execution-journal.md`; the per-round narratives that the orchestrators
wrote follow after them and are the detailed record.

## Chunks

SHAs are those on the branch after the review fixups of 2026-09-20. The rebase onto `main` after #391 merges rewrites every one of them; the commit subjects are the stable keys.

| id | status | commit(s) | summary | blocker |
|----|--------|-----------|---------|---------|
| P1 | complete | 64122d98 | Root ADR-0006 (colocated component ADRs, qualified citations), `check-adr-refs.sh` gate + test, justfile wiring, decisions page, ARCH-MAP "Colocated docs" widened | none |
| P2-1 | complete | 84bbb662 | Verbatim copy of dsp-cli from dsp-incubator cde8b317 (467 tracked files), workspace member, lockfile re-resolved | none |
| P2-2 | complete | 78267ff0 | `just fmt` only, 96 files under `dsp-cli/` | none |
| P2-3 | complete | f608736c | Manifest metadata and inherited deps, rustdoc fixes, `NO_COLOR` pinning, live tests `#[ignore]`d with strict mode and gate, 363 citations qualified, ADR-0002/0004 amended, ARCH-MAP/CONTEXT/conventions/book, justfile recipes, flake tools, `publish-dry-run` job | none |
| P3 | complete | a50f5230 | Second release-please package, matrix lockfile amend, `publish-dsp-cli.yml`, changelog reconciled, release docs | none |
| P4 | complete | be7251ea | `dsp-cli-drift.yml` (changes/pinned/gate/latest) with a composite action, compose stack + fixture/token/wait scripts, `live_vocabulary.rs` on fixture data, ADR-0009 amended, docs | none |
| P5 | complete | 53c39471 | `--server` scheme and control-character refusal with override, body-free auth-cache warnings, `.env`/`DSP_TOKEN` warning, BrokenPipe exit 0, `DSP-Client` header, truncate helper; `Release-As: 0.3.0` | none |
| P6 | complete | f6f2b957 | Book page lists the insecure-server override (root path, kept out of P5 for release attribution) | none |
| ship | complete | the `docs(docs): record the execution …` commit | Plan checkboxes, journal, ARCH-MAP `last_verified_commit` | none |
| deps | complete | the `chore(deps): update rust-overlay …` commit at the tip | `flake.lock`: rust-overlay 2026-04-07 → 2026-09-19 so the dev shell's nightly rustfmt (2026-09-18) matches CI's `dtolnay/rust-toolchain@nightly`; six dsp-cli files re-wrapped into the Phase 2 format commit | none |

Post-merge phases (plan Phases 6 and 7) and the CI-dependent checkboxes are not chunks of this
run; see Deferrals.

## Deferrals

- **Before merge (plan line 562, human):** create the GitHub Release `dsp-cli-v0.2.1` on the PR's
  merge base after the final rebase onto `main`, while `publish-dsp-cli.yml` is not yet on `main`.
  Impossible in this run: PR #391, which this branch is stacked on, is still open.
- **Plan line 542, CI:** the `pinned` drift job was never exercised locally (ports 3030/3333 are
  held by the developer's own dsp-api stack); its first run on PR #409 is the real check of the
  compose stack, the composite action, and the 30-minute budget.
- **`cargo deny` (plan line 506):** three findings, all in the lockfile at the base commit
  (`CDLA-Permissive-2.0` on `webpki-root-certs`; advisories on `rustls` and `h2`). Repository
  policy, not this plan's; not resolved.
- **`cargo hack --feature-powerset`:** not on the dev-shell PATH; the `hack` job in `check.yml`
  covers it.
- **Human actions H1, H2, H3, H5, H6** per the plan's table; H4 was cleared 2026-09-19.
- **Backlog items** moved to Linear: DEV-7338 (language selection), DEV-7339 (shared disclosure
  constant), DEV-7340 (reqwest → ureq).
- **Snapshot hygiene:** 26 pre-existing insta snapshots carry `assertion_line:` metadata that
  churns on line shifts; a separate cleanup some day.
- **`ARCH-MAP.md` `last_verified_commit`** is set to a1db6c23 here and goes stale again on the
  rebase after #391 merges; refresh it then.

## Side findings

- `just check` is red on `main` itself: `just --check --fmt` wants one blank line at
  `justfile:315` (57ce87db). Fixed on PR #406; every round verified with that hunk applied and
  reverted. `check.yml` on PR #409 stays red on that step until #406 lands.
- The plan's "every commit passes `just check`" cannot hold for Phase 2 commits 1 and 2 by
  construction (unformatted verbatim copy; bare self-citations the new gate rejects). Kept the
  three-commit split for reviewability; those two commits are gated by build plus tests.
- Five documentation enumeration points no gate enforces: ARCH-MAP's CI bullet (gate scripts),
  `docs/src/workflows.md` (recipe rows), `docs/src/deployment.md` (check.yml bullet, reusable
  actions table), `check.yml`'s header comment, `dsp-cli/CLAUDE.md`'s command block. Workers
  missed them in three rounds.
- Subagent handback failed for every worker in rounds 3–5 (seventeen of seventeen); a report
  file under `.claude/tmp/` is the only reliable channel. `GIT_EDITOR=true` in the agent
  environment silently no-ops `--fixup=amend:` rewords. Both are in memory.
- Plan anchors that were wrong: fifteen auth-cache warn sites not four; `local` expands to
  `http://0.0.0.0:3333` (unspecified, not loopback); `Url::host_str()` keeps IPv6 brackets;
  clap `SetTrue` rejects `=1` for the env override; `vre/project.rs` is not a stdout sink and
  `auth/token.rs` is; five truncate sites not three; the Fuseki dataset is `dsp-repo`; the
  fixture checkout needs three sparse paths and is keyed on the `API` pin; `gh --jq` pretty-prints
  arrays (a `$GITHUB_OUTPUT` break the plan carried).
- Round 2 also found the 10 help snapshots fail whenever `CLICOLOR_FORCE` is set in the caller's
  environment, which is exactly what the `fn dsp()` pinning fixed.
- **The first CI run after the push (2026-09-20) failed `check` on `cargo +nightly fmt --check`,
  not on the justfile step.** CI's `dtolnay/rust-toolchain@nightly` was rustfmt 2026-09-18; the
  flake's `rust-bin.nightly.latest` was 2026-04-07 (the lock was five months old). The two wrap
  comments at different widths and disagree in both directions (the older one re-joins three of
  the paragraphs the newer one wraps), so no text satisfies both. Fix: `nix flake update
  rust-overlay` (stable stays pinned by `rust-toolchain.toml`), then `cargo +nightly fmt` with the
  matching rustfmt, folded into the Phase 2 format commit. Ten comment blocks in six files, all
  text the verbatim copy brought in. This recurs whenever the lock and CI's nightly drift apart;
  pinning CI's fmt nightly to a date the flake also pins would make it deterministic (not done here,
  a repository policy).
- CI's `just --check --fmt` passed on the runner's `just`; the local 1.49.0 wants the blank line
  #406 adds. The justfile defect is a local-only red, not a CI one.
- `Docker Scout / editor` failed on the same run with `Unexpected HTTP response: 403` from the
  Scout API and then a missing SARIF file; the editor image is untouched by this PR and the job
  passes on the base branch. Transient, not this PR's.
- The `pinned` drift job passed on its first real run in 2 min 39 s (image pulls, fixtures, API
  start, 13 live test binaries), far inside the 30-minute budget and the 12-minute target.

## Closeout

- root_cause: dsp-cli was built as a prototype in a repository with no release automation and a
  CI matrix that meant nothing for a CLI; its drift detector (the live tests) early-returned as a
  pass without a server, so nothing ever told anyone when dsp-api changed a JSON-LD key. Release
  was a manual `cargo publish` by one person, and the crate cited its own ADRs with bare numbers
  that collide with the target repository's root series.
- investigation: the plan was validated at intake against the tree (all file and line anchors
  held; two contradictions found before any code: the ADR gate would fail on the spec itself, and
  `just check` cannot pass on a verbatim unformatted copy). Execution then found the anchors that
  were wrong in kind rather than in line number (see Side findings), a plan-level CI defect
  (`gh --jq` output shape), a false-drift risk in the vocabulary test (list order is unspecified
  and one fixture list is flat), a latent bug keying the dsp-api checkout on the Fuseki pin, and a
  token echo in the token script's CI log. Reviews per phase (rust, devops, consistency, dune,
  simplicity, security on Phase 5) with each finding verified against the files before dispatch;
  the security probe found the control-character sanitization covered one message of ~25.
- solution: eight commits, one PR. A root ADR and gate first so the move follows an existing
  rule; the move as three commits (copy, format, edits) so the reviewable diff is the third;
  release-please as a second package with separate PRs and a matrix lockfile amend; a drift
  workflow that triggers unconditionally and gates on an always-running job so it can be made
  required; hardening that refuses cleartext non-local servers and control characters, logs
  auth-cache errors body-free, exits 0 on a closed pipe, and identifies the client; `Release-As`
  confined to a commit that touches only `dsp-cli/`.
- prevention: `check-adr-refs.sh` and `check-live-tests-ignored.sh` in `just check`; strict live
  mode so a missing server is a failure in CI; `publish-dry-run` in `check.yml` so an unpublished
  path dependency fails the PR that adds it; tests that were shown to fail before the fix (token
  leak, broken pipe); the `gate`-not-`pinned` rule written in the workflow, the job and the docs;
  a `dsp-cli-stack-test` composite action so `pinned` and `latest` cannot drift apart; the
  enumeration points and the plan corrections recorded here for the next plan.

---

# Round narratives

One section per orchestration round. Decisions taken mid-run, findings that are not
the plan's business, and anything a later round must not re-litigate.

## Round 1 — 2026-09-19 — Phase 1

Base `4f3247d8`. Landed as one commit, `a383968b`
`docs(docs,ci): record how decision records are homed and cited across components`
(9 files, +330/-8). Checkboxes 1–6 ticked; checkbox 7 (`eng:reviewing`) deferred
to the session as tier-1 work.

### What landed

| Artefact | Note |
|---|---|
| `docs/adr/0006-…-colocated-and-cited-qualified.md` | The decision, both residuals stated |
| `.github/scripts/check-adr-refs.sh` | Two-pass gate, sourceable, `main()` + `BASH_SOURCE` guard |
| `.github/scripts/check-adr-refs.test.sh` | 7 cases, all passing |
| `justfile` | `check-adr-refs` recipe, dependency of `check`, `.test.sh` in `test` |
| `ARCH-MAP.md` | "Colocated docs" widened; gate added to the CI script list |
| `docs/src/repo_structure.md` | Five ADRs → six, plus a clause for what ADR-0006 binds |
| `docs/src/decisions.md` + `SUMMARY.md` | New page under Repo Overview, absolute GitHub URLs |
| `docs/src/workflows.md` | Recipe table row |

Acceptance, as settled at intake: `check-adr-refs.sh` exits 0 on the tree
(139 bare references, 0 qualified, all resolve), and the distinct set of bare
citations outside `docs/specs/` is exactly `ADR-0001`–`ADR-0006`.

`mdbook build docs` runs clean and emits `docs/book/decisions.html`, so the new
`SUMMARY.md` entry resolves.

### Corrected before the commit was final

Three defects were caught on review of the first draft and amended in:

- ADR-0006 claimed ADR-0005 exemplifies the dated `## Amendment` heading. It
  does not: its three amendments are bold dated paragraphs inside Consequences.
  The record now says so and scopes the heading to amendments from here on.
- The `ARCH-MAP.md` widening covered only the `docs/adr/` half of the
  "Colocated docs" bullet. The `CONTEXT.md` half still granted a colocated
  vocabulary file to bounded contexts and named engines only, which would not
  have covered `dsp-cli/CONTEXT.md` in Phase 2. Both halves now widen together.
- Test case 5 used the same fixture and expectation as case 3, so it asserted
  nothing new. Case 3 already proves a qualified reference is not double-counted
  as a bare one, because the temporary root series has no matching number; case
  5 now covers the other half of that rule, that a qualified reference never
  falls back to the root series.

### Decisions taken in the round

- **Two enumeration points the plan does not name were updated anyway.**
  `ARCH-MAP.md`'s CI bullet lists the gate scripts individually, and
  `docs/src/workflows.md` has one table row per gate recipe. Leaving the new
  gate out of either would have been a consistency-review finding. Both are
  one-line additions inside the phase's own concern.
- **`last_verified_commit` in `ARCH-MAP.md` was left alone.** Phase 2's
  checklist updates it together with the Overview sentence, and bumping it here
  would assert a re-verification of the whole map that this phase did not do.
- **The commit trailer is `Co-Authored-By: Claude Opus 5 (1M context)`**, not
  the `Claude Fable 5.1` line the orchestration brief specified. The
  session-level attribution rule and every existing commit on `main` use the
  Opus line; flagged to the session rather than silently split the convention.

### Finding: `just check` is red on `main`, independently of this work

`just --check --fmt --unstable`, the first step of `just check`, fails on the
base commit and on `main`. The justfile formatter takes only the last comment
line above a recipe as its doc comment and separates preceding context with a
blank line; commit `57ce87db` rewrapped the `_tailwind-bin` comment block so
the formatter now wants a blank line inserted mid-sentence. One blank line is
the entire difference, at `justfile:315` on `main`.

This is already fixed on branch `chore/justfile-fmt` (`ebdb1224`,
`chore(ci): keep the justfile formatter-clean`, pushed, not yet merged). It was
**not** folded into the Phase 1 commit: duplicating it would conflict when that
branch merges.

Phase 1 was verified with `ebdb1224`'s hunk applied to the working tree and
then reverted, so the tree committed here is unchanged by it. With the fix
applied, `just check` passes end to end and `just test` passes: 115 + 6 unit
tests, and 104 gate-script cases across the six `.test.sh` suites, including
the 7 new ones. Without it, `just check` stops at the formatter before reaching
any gate.

**For Phase 2:** if `chore/justfile-fmt` has not merged by then, `just check`
will still be red for reasons that are not this PR's. Land that branch first.

### Not started

Phase 2 is gated on **H4**, which the session clears between rounds. Nothing in
Phase 2 was read, planned or touched.

## Round 2 — 2026-09-19 — Phase 1 review findings, then Phase 2

Gate H4 cleared by the user: the incubator copy is frozen at
`cde8b317d40bc3cf7597f5fb51a566529de9799e` (dsp-incubator `main`, in sync with origin).

| Chunk | Landed as | Note |
|---|---|---|
| 0 — Phase 1 review findings | `64122d98` (amend of `a383968b`) | Three edits, `just check-adr-refs` green (140 bare, 0 qualified, all resolve) |

### Chunk 0 — what changed

- `docs/adr/0006-…:11` no longer asserts that ADR-0002 already names dsp-cli. It now reads
  that dsp-cli "arrives as a root peer of the areas by an amendment to ADR-0002 that lands
  with the crate", so the sentence is true at this commit and stays true after Phase 2.
- ADR-0006's amendment bullet now also grandfathers ADR-0002's in-place revision notes
  ("layout revised 2026-09-17" at `docs/adr/0002-areas-at-the-repository-root.md:10`,
  "replaced 2026-09-17" at lines 37 and 42) as pre-acceptance drafting of a record dated
  2026-09-16, not amendments, setting no precedent. Phase 2's amendment to ADR-0002 therefore
  uses the dated `## Amendment` heading.
- `ARCH-MAP.md`'s "Colocated docs" summary line regained the "also from inside that component"
  clause that ADR-0006 carries and the map had dropped.

### Commit 1 — `84bbb662` `feat(dsp-cli): move dsp-cli from dsp-incubator into the workspace`

470 files, +80450/-15. Source: dsp-incubator `cde8b317d40bc3cf7597f5fb51a566529de9799e`.

Copied with `git -C <incubator> archive cde8b317 dsp-cli | tar -x`, so only tracked
content at that commit came across and the four untracked `tests/snapshots/*.snap.new`
files in the incubator working tree were never a risk. The exclusion list was then deleted
from the copy. Verified by diffing `git ls-tree -r --name-only cde8b317 dsp-cli` minus the
exclusions against `find dsp-cli -type f`: **467 files, exact match, no diff**.

The incubator's `dsp-cli/Cargo.toml` carries no `[workspace]` table, so the crate needed no
edit at all to become a member. The copy is verbatim.

In place of the incubator's `.gitignore`, the root one grew a two-line block for
`dsp-cli/.env` and `dsp-cli/.env.*` (dsp-cli/ADR-0007). The incubator `.gitignore`'s other
entries (`/target/`, `.direnv/`, `.DS_Store`, `.vscode/`, `.idea/`, `*.swp`,
`.claude/settings.local.json`) are already covered by the root file; its
`!.env.example` negation is moot because `.env.example` was not copied.

**Gate: build + tests, not `just check`** — as agreed at intake. `just check` cannot pass at
this commit by construction: `cargo +nightly fmt --check --all` sees an unformatted copy
(commit 2 fixes that) and `check-adr-refs` sees dsp-cli's own bare `ADR-0007`–`ADR-0016`
citations (commit 3 fixes that). What was run instead:

- `cargo build -p dsp-cli --all-targets` — clean.
- `cargo nextest run -p dsp-cli` — **1557 passed, 0 skipped**, so the re-resolution of
  dsp-cli's dependencies against the workspace lockfile behaves.

### Commit 2 — `c5497ca4` `chore(dsp-cli): format under the workspace rustfmt configuration`

`just fmt`, 96 files changed, **all under `dsp-cli/`**, +2765/-8836. Nothing outside the
crate moved. maudfmt is a no-op on it (no `html!` macros); the whole diff is
`cargo +nightly fmt --all` applying `.rustfmt.toml`. 1557 tests still pass, no snapshot
changed.

`just fmt` does **not** run the justfile formatter (it is `maudfmt` + `cargo fmt` only), so
the `justfile:315` hunk was never at risk of being pulled into this commit.

#### Finding: `just check` is red at commits 1 and 2 for a second reason the brief did not name

Round 1 recorded the justfile-formatter defect. There is a second, and it is this PR's own:
`check-adr-refs` (a dependency of `check`, so it runs before the formatter steps) reports
**363 unresolved references** at commit 2. Every one is a bare `ADR-0007`–`ADR-0016` inside
`dsp-cli/`, i.e. dsp-cli's own series, which the root `docs/adr/` (0001–0006) cannot resolve.
Verified: **zero** failing references outside `dsp-cli/`. Distribution: 0007×66, 0008×56,
0009×35, 0010×15, 0011×8, 0012×41, 0013×26, 0014×4, 0015×12, 0016×14, plus 0001–0006 inside
dsp-cli which resolve *wrongly but silently* to root records (ADR-0006 states that residual).

Commit 3's chunk 3d rewrite to `dsp-cli/ADR-NNNN` is what clears it. So the plan's "every
commit leaves the tree green under `just check`" holds from commit 3 on, not from commit 1.
**For the session:** if a bisectable-green history matters more than the plan's three-commit
split, the fix is to pull the mechanical citation rewrite forward into commit 2. Not done
here — that is a plan change, not an execution decision.

Everything else in `just check` was verified green at commit 2, step by step, with the
`chore/justfile-fmt` hunk applied and then reverted before staging: `verify-checksums`,
`check-shared-paths`, `check-datastar-delimiters`, `just --check --fmt`, the maudfmt no-op
sweep, `cargo +nightly fmt --check --all`, `cargo clippy --all-features -- -D warnings`
(dsp-cli compiles clippy-clean as-is) and `cargo machete` (no unused dependencies).

### Side findings from commits 1–2

- **The `[profile.release]` warning is real and appears on every cargo invocation.** Verbatim:
  `warning: profiles for the non root package will be ignored, specify profiles at the
  workspace root:` / `package: …/dsp-cli/Cargo.toml` / `workspace: …/Cargo.toml`. The section
  stays (it governs `cargo install dsp-cli` from crates.io, where the crate's manifest is the
  root); chunk 3a adds the two-line comment saying so.
- **The 10 help-snapshot tests fail whenever `CLICOLOR_FORCE` is set in the caller's
  environment**, which it is in this session (`CLICOLOR_FORCE=yes`, `CLICOLOR=1`). The diff is
  pure ANSI SGR codes around otherwise identical help text, so this is not clap-version drift
  from the re-resolution. It is exactly the fragility commit 3's `fn dsp()` env pinning
  removes. Commits 1 and 2 were therefore gated with `env -u CLICOLOR_FORCE -u CLICOLOR`, and
  the generated `.snap.new` files were deleted before staging. Nothing was accepted.
- **Every snapshot's `source:` metadata line is now stale**: insta records the path relative
  to the workspace root, so `tests/cli.rs` became `dsp-cli/tests/cli.rs`. insta compares
  content, not metadata, so nothing fails; the header rewrites itself whenever a snapshot is
  next accepted. Left alone deliberately — a bulk re-accept would bury the real snapshot
  changes chunk 3d needs to show.
- **`cargo nextest` is already on the dev-shell PATH** before chunk 3g touches `flake.nix`
  (it resolves from the user's cargo install). The `flake.nix` addition is still worth making,
  so CI and a fresh clone get it, but it is not blocking chunk 3c.

### Commit 3 — `8fcaf60f` `build(dsp-cli,docs): register the crate and harmonize it with the workspace`

128 files, +1333/-974. Built as six worker chunks, each verified on the tree and amended into
the one commit. Checkboxes 483–504 all ticked.

| Chunk | What |
|---|---|
| 3d | ADR citation rewrite across `dsp-cli/` + dsp-cli ADR amendments |
| 3a | `dsp-cli/Cargo.toml` metadata, workspace-inherited deps, profile comment |
| 3b | the 6 rustdoc warnings |
| 3c | test-env pinning, `#[ignore]` on live tests, `tests/common/mod.rs`, the new gate |
| 3e | root docs: ADR-0002 amendment, ADR-0004 note, ARCH-MAP, CONTEXT, conventions, mdBook |
| 3f | `dsp-cli/CLAUDE.md` rewrite + three `docs/src/dsp-cli/` pages |
| 3g | justfile recipes, `install-requirements`, `flake.nix`, `.claude/settings.json` |

Notable results, chunk by chunk:

- **3d.** 363 bare citations became `dsp-cli/ADR-NNNN`. **No snapshot changed** — the
  citations live in comments and doc strings, not in rendered output — so nothing was accepted
  and `cargo insta accept` was never run. Two bare citations were then written deliberately
  inside `dsp-cli/`: `ADR-0002` and `ADR-0006` in the dsp-cli/ADR-0014 amendment, which are
  root records and correctly bare. `dsp-cli/docs/topics/` now contains zero occurrences of
  `ADR-`; the one citation there was rephrased, not qualified, because that text ships in the
  binary. dsp-cli/ADR-0015's mention of dsp-incubator is a historical statement about a
  user-agent string and was left alone; 0005 and 0011 got new pointers.
- **3a.** Feature parity checked one by one against `[workspace.dependencies]`: `serde_json`
  **does** carry `preserve_order` at the root, so dsp-cli's `_meta`-first envelope contract
  survives inheritance. `insta` at the root is `["yaml", "filters"]`, a superset.
  `tracing-subscriber` at the root has `env-filter`; `fmt` is added beside `workspace = true`.
  **`chrono` was deliberately NOT inherited**: dsp-cli needs `default-features = false` and
  the root entry leaves defaults on, and `default-features` cannot be overridden beside
  `workspace = true` the way features can. That is a deviation from the plan's list, and the
  reason is now a comment in the manifest.
- **3b.** The two private-item links (`resolve_password`, `run_from_line`) became plain
  backticked text — they are private functions and not linkable from public docs; the four
  `DspClient` links got fully-qualified paths. No `#[allow]`, no visibility change.
  `cargo doc --no-deps --all-features -p dsp-cli` is warning-free (verified against a forced
  rebuild, not a cached no-op).
- **3c.** `CLICOLOR_FORCE=yes cargo nextest run -p dsp-cli --test cli` now passes: 51 tests,
  colour forced on, snapshots still match. 20 live tests across 14 files carry `#[ignore]`;
  `cargo nextest run --all-features` reports **20 skipped** where it previously passed them
  vacuously. The new gate has 8 test cases and is wired into `check` and `test` exactly as
  `check-adr-refs` is.
- **3e.** ADR-0002's amendment is under a dated `## Amendment` heading per ADR-0006, not the
  in-place style the record's own drafting notes use.
- **3g.** `cargo deny` in the locked nixpkgs is **0.19.0**, not the 0.20.2 the plan names, and
  `cargo-nextest` is **0.9.132**, not 0.9.144. The `flake.nix` comments state the real
  versions and note that `install-requirements` pins the newer ones via binstall.

#### The 3a/3d interaction that needed a fixup

The 3d rewrite lengthened doc comments by nine characters each, which pushed several past the
rustfmt width. `cargo +nightly fmt --check --all` went red on files 3d had touched. `just fmt`
was re-run and the result folded into commit 3 rather than left for a later commit, so
commit 3 is itself formatter-clean. Two files outside 3d's own edit set
(`dsp-cli/src/model/data_model.rs`, `dsp-cli/tests/resource_type_describe_snapshots.rs`) moved
only for that reason.

### Final verification at `8fcaf60f`

| Gate | Result |
|---|---|
| `just check` | **passes end to end** (with the `chore/justfile-fmt` hunk applied, then reverted before staging) |
| `just test` | **exit 0** — unit tests plus 112 gate-script cases across seven `.test.sh` suites |
| `cargo nextest run --locked --all-features --all-targets` | **3401 passed, 20 skipped** — the 20 are the live tests |
| `mdbook build docs` | clean; every new `SUMMARY.md` entry resolves |
| `just check-adr-refs` | 155 bare, 559 qualified, all resolve |
| `just commit-lint 4f3247d8` | **messages: all four OK.** Count cap fails, by design — see below |
| `cargo hack --feature-powerset` | **not verified** — `cargo-hack` is not on the dev-shell PATH and was not added (the plan only asks for `cargo-nextest` and `cargo-deny`) |

Note the recipe takes a positional argument: `just commit-lint 4f3247d8`, not
`just commit-lint base=4f3247d8`, which `just` passes through as a literal and which then
silently lints the empty range `..HEAD`.

### Blockers and deferrals for the session

- **`cargo deny` findings are all pre-existing; none is dsp-cli's to fix.** Plan line 505 is
  left **unticked**. The run is done —
  `cargo deny --manifest-path dsp-cli/Cargo.toml check --config deny.toml` (note: `--config`
  goes **after** `check` in cargo-deny 0.19) — and it reports `advisories FAILED, bans ok,
  licenses FAILED, sources ok`:
  - `webpki-root-certs 1.0.6` is `CDLA-Permissive-2.0`, which `deny.toml`'s allow-list does
    not carry. Reached via `reqwest 0.13.4 → rustls-platform-verifier 0.6.2`.
  - `RUSTSEC` advisory on `rustls 0.23.36` (handshake messages accepted at the wrong
    encryption level; fixed in >= 0.23.45).
  - `RUSTSEC-2026-0258` on `h2` (unbounded empty DATA frames).
  **All three are present in the lockfile at the base commit `4f3247d8`, before dsp-cli
  existed** — verified: `rustls 0.23.36`, `h2 0.4.13` and `webpki-root-certs 1.0.6` are all in
  `git show 4f3247d8:Cargo.lock`. cargo-deny has simply never been run in this repository (it
  is not wired into CI, and the plan keeps it out of CI deliberately). Resolving them means
  either adding `CDLA-Permissive-2.0` to `deny.toml` or bumping the lockfile repository-wide,
  both of which are policy decisions outside this phase. Not done.
- **The commit-count cap needs `allow-many-commits` in the PR body.** The branch carries four
  commits over `4f3247d8` and the gate caps a PR at one. The plan already provides for this
  ("with `allow-many-commits` ticked in the PR body"), but the box is a PR-description action
  at ship time, so plan line 508 stays unticked until it is.
- **Plan line 507 stays unticked** solely because `cargo hack --feature-powerset` could not be
  run. Everything else that line asks for is verified above.
- **`ARCH-MAP.md`'s `last_verified_commit` is set to `c5497ca4e209a876b972d957f10b54255e20f114`**,
  the second commit, not the third. Commit 3 cannot name its own SHA, and it was amended six
  times during this round, so any value naming it would have gone stale immediately. It also
  goes stale when the branch is rebased after #391 merges. **Refresh it at ship time**, after
  the final rebase.
- **Plan line 506, Linear issues.** Not filed (session's job at ship time). The exact wording
  from the incubator's `docs/BACKLOG.md` at `cde8b317`, so it need not be read again:
  1. **"Language selection for multilingual labels — needs a CLI-wide policy"** — surfaced
     2026-07-28 reviewing the vocabulary-inspection design. DSP-API returns
     `labels`/`comments` as language-tagged arrays; dsp-cli has no policy and no `--language`
     flag. `project describe` keeps every description (`ProjectDescription { value, language }`),
     the `/v2/node` list-node label lookup in `resource describe --values` takes whatever
     `extract_string_value` finds, and JSON-LD `rdfs:label` is read the same way. Measured on
     prod 2026-07-28: of **499** list roots, **53%** carry more than one label and **~34%**
     carry **no English label at all**, so any implicit "prefer en" rule silently falls back
     for a third of real data. Decide once, cross-command: preference order, a `--language`
     flag, or all-languages-always. Urgency: the `vre vocabulary` noun-group ships all
     languages everywhere (owner's decision 2026-07-28), making the divergence user-visible.
     Consolidating plan 034's `LocalizedText` with `ProjectDescription` belongs to this item.
  2. **"Shared constant for hand-copied `count_cost`/`count_caveat` disclosure strings"** —
     surfaced 2026-07-30 reviewing plan 034 (`vre vocabulary list`/`describe`). The
     disclosure string is hand-copied verbatim between `src/actions/vre/vocabulary.rs` and
     `tests/vocabulary_list_snapshots.rs`'s `count_cost_all_failed_meta()` helper, with no
     shared constant across the action/test boundary; mirrors the existing `COUNT_CAVEAT`-copy
     pattern in `tests/resource_type_list_snapshots.rs`. A wording change to either string can
     silently desync test expectations from production text.
  3. **The deferred `reqwest` → `ureq` alignment** (from the plan, not the backlog).
  Three further open backlog entries are Phase 5 deliverables per the plan and were **not**
  collected here: the `auth.toml` parse-error token-fragment leak at default verbosity, the
  unvalidated `--server` scheme, and broken-pipe surfacing as `internal error`.
- **Plan line 509 (`eng:reviewing`)** is tier-1 work, not started.

### Post-gate review of commit 3 (amended in, final SHA `f20fc561`)

Three checks the automated gates cannot make, run after the tree was green:

- **The two enumeration points round 1 identified were missed again, and are now fixed.**
  `ARCH-MAP.md`'s CI bullet lists each gate script by name and `docs/src/workflows.md` has one
  table row per gate recipe; neither the 3c worker nor the 3g worker was told about them, and
  both had dropped `check-live-tests-ignored.sh` and the three `dsp-cli-*` recipes. Added.
  **For any future round: these two files are enumeration points that no gate enforces.**
- **The ADR-0006 residual was checked by hand.** `git grep -nP '(?<![/\w])ADR-000[1-6]\b' --
  dsp-cli docs/src/dsp-cli` returns three hits, all deliberate root citations: the ADR-0006
  pointer in the rewritten `dsp-cli/CLAUDE.md`, and root ADR-0002 and ADR-0006 in the
  dsp-cli/ADR-0014 amendment. `docs/src/dsp-cli/` has none. Nothing bare inside `dsp-cli/`
  means a dsp-cli record.
- **`.claude/settings.json` parses** (`jq empty`, exit 0). No gate reads it, and a malformed
  file would break every later session in this repository.

Also noted, not a defect: `ARCH-MAP.md`'s dsp-cli boundary rule and ADR-0002's `Enforced by:`
line both name `cargo publish -p dsp-cli --dry-run` "in `check.yml`". Phase 3's checkbox at
plan line 521 is what actually adds that job. The map and the ADR are therefore one phase
ahead of CI until Phase 3 lands — intended by the plan, worth knowing if Phase 3 slips.

### Operational finding: the implementation-worker handback is broken

**All six** `eng:workflow:implementation-worker` subagents this round ended without delivering
a report ("The subagent ended without delivering a report through SubagentHandback"), and each
one's unsent text was discarded. Every chunk was recovered by verifying the tree directly, so
nothing landed unchecked — but the workers' own side findings are gone, including 3a's
feature-parity notes, 3f's account of which constraints it rescued from the old CLAUDE.md
history, and 3g's report of what `just --check --fmt` complained about. The facts recorded in
this journal were re-derived from the diffs and from re-running the gates, not taken from a
worker report. **Budget for that verification cost in later rounds**, and treat the worker
agent definition's handback path as suspect.

## Round 3 — 2026-09-19 — Phase 2 review findings, Phase 3, Phase 4

Base `4f3247d8`, HEAD at start `f20fc561`. Chunk 0 amends the Phase 2 commit 3 with the
seven confirmed review findings; Phases 3 and 4 then land one commit each.

### Chunk 0 — Phase 2 review findings, amended into `e2190b3b`

Commit 3 re-amended (was `f20fc561`). 131 files, +1394/-997 against `c5497ca4`.
All seven findings (a–g) landed. Three parallel workers wrote the edits; **all three lost
their reports again** ("ended without delivering a report through SubagentHandback"), so
every change below was verified by reading the diff, not from a worker report. The round-2
finding about the handback path holds for plain `general-purpose` subagents too, not just
`eng:workflow:implementation-worker`.

| Finding | What landed |
|---|---|
| a | `.claude/settings.json`: `Bash(cargo publish --dry-run:*)` → `Bash(cargo publish -p dsp-cli --dry-run:*)`; `jq empty` clean |
| b | `dsp-cli/tests/live_update_check.rs`: the `#[ignore]` message now says it hits crates.io and is excluded from `just dsp-cli-test-live` |
| c | `check.yml` gained the `publish-dry-run` job; plan lines 519 and 521 ticked |
| d | Root `CONTEXT.md` Vocabulary entry now names the competing "closed vocabulary" sense |
| e | `docs/src/dsp-cli/testing-strategy.md`: `DSP_TEST_CLASS_IRI` and `DSP_TEST_NON_ADMIN_TOKEN` rows added |
| f | Stale `docs/dev/`, `docs/design/`, `.env.example` references re-pointed across 20 files |
| g | Plan line 484 rewritten to record the `chrono` exception |

**Plan line 521 landed in Phase 2, not Phase 3.** `ARCH-MAP.md`'s dsp-cli boundary rules and
ADR-0002's `Enforced by:` line both state in the present tense that `cargo publish -p dsp-cli
--dry-run` runs in `check.yml`; that had to be true at the commit asserting it. Phase 3's
checkbox is therefore already ticked when Phase 3 starts.

**Where the new CI job was enumerated.** Two places enumerate `check.yml`'s contents and both
were updated: the header comment block at the top of `check.yml` itself (which lists fmt,
clippy, checksums, doc, hack) and the `- **check.yml** — …` bullet at `docs/src/deployment.md:11`.
`ARCH-MAP.md` was **not** touched: its CI bullet lists the gate *scripts*, not `check.yml`'s
jobs, so a job addition does not belong there. Record this alongside round 1's two enumeration
points — there are now three, and no gate enforces any of them.

**Finding f did not trigger a rustfmt reflow.** Round 2's 3d/3a interaction (a nine-character
lengthening pushing doc comments past the width) did not repeat, even though
`docs/src/dsp-cli/testing-strategy.md` is eight characters longer than `docs/dev/testing-strategy.md`:
the workers wrapped as they went, and `just fmt` afterwards changed nothing further.

**Two worker outputs were corrected before staging.** `src/client/mod.rs` and `src/model/mod.rs`
had been re-pointed at a bare `CONTEXT.md`, which is now ambiguous with the root file; both are
`dsp-cli/CONTEXT.md`. The `deployment.md` bullet read "dsp-cli packaging dry-run-checked" and was
reworded.

#### Verification at `e2190b3b`

| Gate | Result |
|---|---|
| `just check` | **passes end to end** (`chore/justfile-fmt` hunk applied via `git apply`, then `git apply -R` before staging) |
| `just test` | **exit 0** — 115 + 6 unit tests, 112 gate-script cases across seven `.test.sh` suites |
| `cargo nextest run -p dsp-cli --all-features` | **1557 passed, 20 skipped**, no `*.snap.new` generated |
| `check-adr-refs` (via `just check`) | 156 bare, 555 qualified, all resolve |
| `cargo doc --no-deps --all-features -p dsp-cli` | warning-free |
| `cargo publish -p dsp-cli --dry-run --locked --allow-dirty` | **72 files packaged**, 1.8 MiB |
| `cargo package -p dsp-cli --list` | `README.md`, `LICENSE`, `CHANGELOG.md` and **10** `docs/topics/*.md` all present — plan line 519 verified |
| `just commit-lint 4f3247d8` | messages all OK; count cap fails by design (four commits, cap 1) |

No ADR citation was lost to finding f: `git diff -U0 HEAD` shows every removed `ADR-` line has a
matching added line carrying the same citation, plus two new `dsp-cli/ADR-0014` citations in the
0007 and 0010 amendments.

**`--allow-dirty` was needed** for the local `cargo publish --dry-run` because the working tree
carries the uncommitted plan, journal and justfile hunk. CI's tree is clean, so the new
`publish-dry-run` job needs no such flag.

#### Tooling finding: `actionlint` is available, PyYAML is not

`python3` on this machine has **no `yaml` module**, so the brief's
`python3 -c 'import yaml; yaml.safe_load(...)'` validation does not run. `actionlint` is not on
the dev-shell PATH either, but `nix run nixpkgs#actionlint -- <file>` works (1.7.12) and is now
warm in the store. `check.yml` lints **clean** under it. Use that for Phases 3 and 4.

Also confirmed for Phase 3: `check.yml`'s jobs use **no** rust-cache action, and pin third-party
actions **by tag** (`@v4`, `@v1`, `@main`), not by SHA.

### Phase 3 — `85c79459` `chore(ci): release dsp-cli on its own version line`

7 files, +86/-26. Two parallel workers (CI wiring; changelog + docs). **Both lost their
reports too** — that is six of six this round, on plain `general-purpose` subagents. Treat the
handback path as broken for every subagent type, not just `eng:workflow:implementation-worker`,
and budget the verification cost accordingly.

Checkboxes 515, 516, 517, 518, 520 and 522 ticked. 519 and 521 were ticked in chunk 0.
523 (`eng:reviewing`) left unticked — tier-1 work.

#### The one deviation from the plan's checkboxes

**The top-level `extra-files` block was moved inside the `"."` package.** Plan line 515 asks
only for dsp-cli's own `extra-files`. But a top-level `extra-files` in a release-please manifest
config is a *default inherited by every package*, and its jsonpath `$.workspace.package.version`
does not exist in `dsp-cli/Cargo.toml`. A per-package `extra-files` on dsp-cli overrides the
default, so this is belt-and-braces rather than a fix — but leaving a wrong default in place for
a new package is a trap for the next person who adds one. For the `"."` package the path
`Cargo.toml` still resolves to the same root file, so root behaviour is bit-identical.

#### What the workers were told, and why

- `include-component-in-tag: true` sits on the dsp-cli **package**, overriding the top-level
  `false`. The root's `v*` tag format had to survive: `dpe-release-publish.yml` triggers on it.
- The `amend-lockfile` matrix job keeps the three moved steps' `run:` bodies **verbatim**, and
  carries their explanatory comment blocks with them. Only the "we check for any open release PR"
  comment was reworded, because with `separate-pull-requests` there can now be two.
- `publish-dsp-cli.yml` pins third-party actions **by tag** (`@v4`, `@v1`), which is what every
  other workflow in this repository does. No SHA pinning anywhere here.
- The repository has **no `.gitmodules`**, so the `submodules: true` that `check.yml`'s jobs pass
  to `actions/checkout` is vestigial. `publish-dsp-cli.yml` omits it; nothing depends on it.

#### Verification at `85c79459`

| Gate | Result |
|---|---|
| `nix run nixpkgs#actionlint` on `release-please.yml`, `publish-dsp-cli.yml`, `check.yml` | **clean**, exit 0 |
| `jq empty` on `config.json` and `manifest.json` | both valid |
| `just check` | passes end to end (justfile hunk applied, then reverted before staging) |
| `just test` | exit 0 |
| `mdbook build docs` | clean |
| `just commit-lint 4f3247d8` | messages all OK; count cap fails by design (five commits) |
| `dsp-cli/CHANGELOG.md` entries | **9 before, 9 after** — every release entry preserved |
| cross-reference anchors | `deployment.md#dsp-cli` and `git-conventions.md#scopes` both exist |

#### Deferral for the session: the crates.io trusted publisher (H1)

`publish-dsp-cli.yml` authenticates through `rust-lang/crates-io-auth-action@v1`. That needs a
one-time trusted-publisher entry created on crates.io by an existing owner of the crate
(`BalduinLandolt`, or a member of `github:dasch-swiss:everyone-private`), naming this repository,
the workflow filename **and** the `crates-io` environment — the job sets `environment: crates-io`
as defence in depth, and the trusted-publisher entry must match it or authentication fails. That
crates.io entry is the one hard prerequisite, and it is **not done**; the GitHub environment itself
needs no manual setup, since GitHub creates a referenced environment on first use (adding
protection rules to it is optional). Until the crates.io entry exists, a published `dsp-cli-v*`
release will fail at the auth step; the dry-run step before it will still have passed. This is gate H1 in the plan, and it is the session's to clear, not an
execution blocker for this phase.

### Phase 4 — `42878542` `chore(ci): run dsp-cli live tests against a pinned dsp-api stack`

14 files, +655/-66. Four parallel workers (stack; workflow + justfile + settings; the live test;
the docs). **All four lost their reports.** Ten of ten subagents this round returned nothing;
every line below was verified by reading the tree.

Checkboxes 529–540 ticked. 541 and 542 left unticked — see the deferrals.

#### Three plan corrections, each forced by reading dsp-api at the pin

1. **The Fuseki dataset is `dsp-repo`, not `knora-test`.** Plan line 530 says to set
   `KNORA_WEBAPI_TRIPLESTORE_FUSEKI_REPOSITORY_NAME=knora-test`. That is wrong.
   `modules/webapi/scripts/fuseki-functions.sh` defaults `REPOSITORY=dsp-repo`, and dsp-api's own
   justfile runs the init script with **no arguments** under the comment "initializes the dsp-repo
   repository". Setting `knora-test` would have pointed the API at an empty dataset. The compose
   file sets `dsp-repo`.
2. **The sparse checkout needs three paths, not two.** Plan line 531 names `test_data/` and
   `modules/webapi/scripts/`. But `fuseki-init-knora-test.sh` also uploads five ontologies from
   `../src/main/resources/knora-ontologies/`, so
   `modules/webapi/src/main/resources/knora-ontologies` is required too. It also uses paths
   relative to its own directory and does `source fuseki-functions.sh`, so it must be run with the
   working directory set to `modules/webapi/scripts/`. `load-fixtures.sh` does both.
3. **dsp-api's own compose pins both images to `latest`, not to a version.** The plan's
   `API=v38.1.0` / `DB=v38.1.0` still stands — both tags were confirmed to exist on Docker Hub for
   `daschswiss/knora-api` and `daschswiss/apache-jena-fuseki` — but the Fuseki image is versioned on
   its own line (currently `v39.0.0-N-gSHA`), not in lockstep with dsp-api. `stack.env` and the
   stack README both say the pin is this repository's choice rather than a dsp-api release signal.

#### The api environment, and what was deliberately not copied

The `api` service's environment is dsp-api's own, minus everything that needs a service this stack
does not run. Dropped: all `OTEL_*`, `PYROSCOPE_*` and `JAVA_TOOL_OPTIONS` variables (they point at
an `alloy` collector), and the sipi and ingest settings. dsp-api's compose has `api` depending on
`sipi`; ours does not. Healthchecks were copied verbatim (`/healthcheck.sh` for db,
`bash /opt/docker/scripts/healthcheck.sh` for api), with the api's `start_period` raised to 180s and
retries to 6, because it is a JVM loading a triplestore.

#### The non-admin fixture user, verified rather than guessed

`token.sh` logs the non-admin in as **`anything.user01@example.org`** / `test`. Confirmed against
`test_data/project_data/admin-data.ttl` at tag `v38.1.0`: that user carries
`knora-admin:isInProject <http://rdfh.ch/projects/0001>`, `isInGroup .../thing-searcher`, and
`isInSystemAdminGroup "false"`. It feeds `DSP_TEST_NON_ADMIN_TOKEN`, which
`live_sparql_query.rs` uses for its permission-boundary case.

#### Corrections made to worker output before staging

- **`token.sh` echoed both JWTs into the CI step log.** Its stdout contract (two `export` lines) is
  what a human sourcing it wants, but in CI the tokens reach later steps through `$GITHUB_ENV`, so
  the workflow now redirects stdout to `/dev/null` in both the `pinned` and `latest` jobs. The
  tokens are throwaway fixture credentials, but they do not belong in a run log.
- `live_vocabulary.rs`'s test function was still named `..._on_geoarch` after geoarch was removed
  from it; renamed to `live_vocabulary_list_and_describe`.
- That file's module header still claimed live tests are "**not** in CI", which Phase 4 makes
  false. It now says they are not in the *default suite* and names the drift workflow.

#### Verification at `42878542`

| Gate | Result |
|---|---|
| `nix run nixpkgs#actionlint -- .github/workflows/dsp-cli-drift.yml` | **clean**, exit 0 |
| `docker compose … config --quiet` | **valid** |
| `bash -n` on all three stack scripts | all parse |
| `just check` | passes end to end (justfile hunk applied, then reverted before staging) |
| `just test` | exit 0 |
| `cargo nextest run -p dsp-cli --all-features` | **1557 passed, 20 skipped** |
| `mdbook build docs` | clean |
| `jq empty .claude/settings.json` | valid |
| `fuzz.yml` artifact claim | **verified against the workflow**: `fuzz-crashes-${{ matrix.target }}`, `retention-days: 90` |

#### Plan line 541 is a deferral to CI, not done — and why

**The stack was never exercised locally.** Ports 3030 and 3333 are held by a pre-existing
`dsp-api-*` compose stack that has been running on this machine for two days (six containers, all
healthy — the developer's own dev environment). `docker compose … up -d --wait db` fails with
`Bind for 0.0.0.0:3030 failed: port is already allocated`. That remnant was torn down with
`down -v` scoped to the `stack` project; **the developer's `dsp-api-*` stack was not touched and is
still up.**

This means any local "it works" signal is unavailable, and worth stating plainly: a worker that
believed it had verified the stack would in fact have been talking to the developer's running
dsp-api on port 3333, not to the pinned one. No such claim is recorded here. What *is* verified is
that the compose file parses, the scripts parse, the image tags exist, the dataset name matches
dsp-api's own init script, and the fixture user exists.

**The first CI run of the `pinned` job is therefore the real check** — of correctness, and of
whether it fits the 30-minute budget (target under 12 minutes). If it overruns, measure whether
image pulls or fixture loading dominate before reaching for a cached Fuseki volume.

#### Deferrals for the session

- **`dsp-cli-drift / gate` as a required check** is a repository-settings action. Until it is set,
  the `gate` job runs but gates nothing.
- **The crates.io trusted publisher and the `crates-io` environment** — see the Phase 3 deferral.
- **The baseline release `dsp-cli-v0.2.1`** must be created as a GitHub Release on the merge base
  **before this PR merges**, or release-please has no tag anchoring the manifest's `"dsp-cli":
  "0.2.1"`. At that moment `publish-dsp-cli.yml` does not yet exist on `main`, so nothing publishes,
  and `dpe-release-publish.yml` filters on `startsWith(tag_name, 'v')`, which `dsp-cli-v…` does not
  satisfy.
- **The open root release PR may be closed and reopened** under the components-style branch name on
  the first release-please run after `separate-pull-requests: true` lands. Expected, not a fault.

#### Post-gate review of Phase 4 (amended in, final SHA `5baa9279`)

Five findings from a review after the tree was already green.

- **`load-fixtures.sh` cloned dsp-api at the `DB` tag.** A latent bug: `TAG="${DB}"`, the
  empty-check and the error message all used the **Fuseki** pin to check out the **dsp-api**
  repository. It worked only because both pins currently read `v38.1.0`. The Fuseki image is
  versioned on its own line (`v39.0.0-N-gSHA`), so the first independent `DB` bump would have
  failed the clone with "remote branch not found". Now keyed on `API`, with the reason in a
  comment. Side effect, also commented in the script: because it re-sources `stack.env`, the
  nightly `latest` job loads the **pinned** tag's fixtures against a `latest` API. That is the
  only coherent choice — `latest` has no corresponding dsp-api ref to check out.
- **`vocabs[0]` would have reported false drift.** The test asserted depth >= 2 on whichever
  vocabulary DSP-API returned first, and the list order is unspecified. dsp-api's `anything`
  fixture holds four list roots, and they are **not** uniformly nested: `testList`'s three
  children (`testList01`–`03`) are all leaves, so that root is depth 1 under this crate's
  semantic, while `treeList`, `otherTreeList` and `notUsedList` are deeper. A first-root pick
  would have failed roughly a quarter of the time for no real reason. The test now scans roots
  for one of depth >= 2 and asserts D2 against that, while still checking that **every**
  vocabulary in the project is fully labelled.
- **Strict-mode env coverage was checked and is complete.** `require_env` across
  `dsp-cli/tests/live_*.rs` names eight variables: `DSP_TEST_SERVER`, `DSP_TEST_PROJECT`,
  `DSP_TEST_CLASS_IRI`, `DSP_TEST_VOCAB_PROJECT` (all four in the workflow's `env:` block),
  `DSP_TOKEN` and `DSP_TEST_NON_ADMIN_TOKEN` (both via `$GITHUB_ENV` from `token.sh`), plus
  `DSP_TEST_USER` and `DSP_TEST_PASSWORD`. The last two are **unreachable in CI**:
  `live_project_dump.rs:63` takes `optional_env("DSP_TOKEN")` first and only falls back to
  user+password when it is absent, and the drift job always sets `DSP_TOKEN`. So nothing panics
  under `DSP_LIVE_STRICT=1`. **If `token.sh`'s `$GITHUB_ENV` write ever breaks, those two
  become reachable and the run turns into a strict-mode panic rather than a skip** — worth
  knowing when reading a first red run.
- **A fifth enumeration point: `dsp-cli/CLAUDE.md`.** Its "Build & Test Commands" block lists
  the `just` recipes an agent should use and had none of the three stack recipes. Added. The
  full list of enumeration points that **no gate enforces** is now: `ARCH-MAP.md`'s CI bullet
  (gate scripts), `docs/src/workflows.md` (one row per public recipe), `docs/src/deployment.md`
  (the check.yml bullet, and the "every push and pull request runs" list), `check.yml`'s own
  header comment, and `dsp-cli/CLAUDE.md`'s command block.
- **Plan lines 530 and 531 were ticked while still describing what was not built.** Both are
  corrected in place, as line 484 was: 530 now records the `dsp-repo` dataset name and that
  dsp-api's OTEL/Pyroscope/sipi settings are deliberately not copied; 531 records the `API`
  keying, the third sparse path, and the working-directory requirement.

The Phase 3 deferral above was also softened: the crates.io trusted-publisher entry is the one
hard prerequisite. The `crates-io` GitHub environment needs no manual creation, since GitHub
creates a referenced environment on first use; adding protection rules to it is optional.

`just commit-lint 4f3247d8` at `5baa9279`: messages all OK, count cap fails by design (six
commits over the base, cap 1 — `allow-many-commits` in the PR body is plan line 508).

### Addendum — three worker reports arrived after the round closed

Three of the ten subagents eventually relayed their reports through a different channel, after
their chunks had already been verified from the tree and committed. **All three corroborate the
independent verification; none contradicts it.** Two facts they add are worth keeping:

- **The root `v*` release trigger provably does not fire on a dsp-cli tag.**
  `dpe-release-publish.yml` triggers on `startsWith(github.event.release.tag_name, 'v')`. A
  dsp-cli tag is `dsp-cli-v0.2.1`, which starts with `d`, so it cannot match. The plan's Phase 3
  rationale asserted this; it is now checked against the workflow rather than assumed. It is what
  makes the pre-merge baseline release `dsp-cli-v0.2.1` safe to create.
- **`rust-lang/crates-io-auth-action` is a first use in this repository**, so there was no local
  precedent to match beyond the tag-pinning convention. Worth knowing if its usage ever needs
  revisiting.

Also recorded for completeness: the nine preserved `dsp-cli/CHANGELOG.md` entries are 0.2.1,
0.2.0, 0.1.6, 0.1.5, 0.1.4, 0.1.3, 0.1.2, 0.1.1 and 0.1.0. And the docs worker additionally
converted the two adjacent `config.json` / `manifest.json` references in `deployment.md` to full
GitHub blob URLs, matching that page's link convention — a small scope extension, kept because it
is consistent with the surrounding text.

## Round 4 — 2026-09-20 — Phase 3/4 review findings, then Phase 5

Base `4f3247d8`, HEAD at start `5baa9279`. Backup branch `backup/dsp-cli-move-5baa9279`
created before any rewrite.

### Chunk A — Phase 3 review findings, folded into the Phase 3 commit

| Finding | What landed |
|---|---|
| A1 (Critical) | `release-please.yml` `find-prs`: the `gh --jq '[.[].headRefName]'` output is now piped through `jq -c .` |
| A2 | `dsp-cli/Cargo.toml`: one comment above `version` saying release-please writes the field |
| A3 (message) | The commit body's `extra-files` sentence reworded: clarity, not a fix |

**A1 was a real break, not a tidy-up.** `gh --jq` pretty-prints a non-empty array over several
lines, and `echo "branches=$BRANCHES" >> "$GITHUB_OUTPUT"` then writes an invalid multi-line
value, so the step failed whenever a release PR was open — which is nearly always. `jq -c .`
(jq is preinstalled on ubuntu runners) compacts it to one line; `[]` still serialises as `[]`,
so `amend-lockfile`'s `if: … != '[]'` is unaffected. Plan line 517 carried the same defect and
was corrected in place.

### Chunk B — Phase 4 review findings, folded into the Phase 4 commit

| Finding | What landed |
|---|---|
| B1 (Major) | `Swatinem/rust-cache@v2` added after the toolchain step (every other Rust workflow here has it) |
| B2 | The nine steps `pinned` and `latest` shared verbatim extracted to `.github/actions/dsp-cli-stack-test/action.yml` |
| B3 | `wait-for-api.sh`: `TIMEOUT_SECONDS=300` hardcoded; no caller ever passed an argument |
| B4 | `load-fixtures.sh`: the Fuseki poll now carries its reason (container health precedes dataset readiness) |
| B5 | `ci/stack/README.md`: the pin section rewritten as two independent pins |
| B6 | `dsp-cli-drift.yml` header and `deployment.md` now say the nightly `latest` job loads **pinned** fixtures |
| B7 | `justfile`: `DSP_TEST_VOCAB_PROJECT` added to the live-test env enumeration |
| B8 | `docs/src/dsp-cli/testing-strategy.md`: stack-up already loads fixtures; `DSP_TEST_VOCAB_PROJECT` takes a shortcode **or** IRI |

#### Three composite-action constraints that shaped B2, worth not rediscovering

- **`actions/checkout` must stay in the job.** A local `uses: ./.github/actions/…` cannot
  resolve before the repository is checked out. "Record start time" and "Report wall time"
  also stay: the message differs per job and `steps.start.outputs.epoch` is job-scoped.
- **`api`/`db` were deliberately NOT made composite inputs.** A composite input with an empty
  default becomes a set-but-empty shell variable, and a set-but-empty shell variable **beats**
  `--env-file` in docker compose interpolation — `pinned` would have pulled
  `daschswiss/knora-api:` with an empty tag. The `latest` job's job-level `env:` block remains
  the only override, and job-level `env` flows into composite steps.
- **`steps.test.outcome` is not visible from the calling job** once the test step lives inside
  the composite; the job sees only the `uses:` step's outcome. The "Summarize failure" step
  therefore moved **inside** the action, gated by a `summarize-failures` input (default
  `'false'`, `latest` passes `'true'`), and the test step now always `tee`s `test-output.log`
  so there is one code path. The hardcoded `daschswiss/knora-api:latest` in
  `docker image inspect` became `${API}` with a `stack.env` fallback.

#### `actionlint` does not lint a composite action file

It parses every file as a *workflow*, so `action.yml` fails its schema immediately ("jobs"
missing, unexpected `runs`). `dsp-cli-drift.yml` itself lints clean, and the action file was
validated by parsing it with `yq-go` and by reading each `if:` expression by hand. **For any
later round: there is no linter covering `.github/actions/*/action.yml` in this repository.**

### `GIT_EDITOR=true` is set in the agent environment, and it silently ate a reword

`git commit --fixup=amend:<sha>` opens an editor pre-filled with `amend! <subject>` plus the
**original** message. `-F` is rejected outright ("options '-F' and '--fixup' cannot be used
together"), and `-c core.editor=…` does **not** help, because `GIT_EDITOR` wins over
`core.editor`. The first attempt therefore produced a well-formed `amend!` commit carrying the
old message, autosquashed cleanly, and changed nothing — a silent no-op. The working form is an
explicit env prefix:

```sh
GIT_EDITOR='cp /abs/path/to/message.txt' git commit --allow-empty --fixup=amend:<sha>
```

where the file's **first line** is `amend! <subject of the target commit>`, then a blank line,
then the full replacement message. Verify with `git log -1 --format=%B` on the fixup commit
*before* rebasing, not after.

Autosquash ran as `GIT_SEQUENCE_EDITOR=true git rebase -i --autosquash --autostash 4f3247d8`.
`--autostash` was needed for the uncommitted plan and journal; it stores its stash in the rebase
state directory, not on `refs/stash`, so the shared-worktree stash-stack hazard does not apply.

### Verification after the autosquash

| Gate | Result |
|---|---|
| `git rev-list --count 4f3247d8..HEAD` | **6**, no `fixup!` or `amend!` subject remains |
| `nix develop --command just check` | passes end to end (`chore/justfile-fmt` hunk applied, then reverted) |
| `nix develop --command just test` | exit 0 — unit tests plus 112 gate-script cases |
| `nix run nixpkgs#actionlint` on `release-please.yml` and `dsp-cli-drift.yml` | clean |
| `jq empty` on `config.json`, `manifest.json` | valid |
| `docker compose … config --quiet` | valid |
| `bash -n` on the three stack scripts | all parse |
| `mdbook build docs` | clean |
| `just commit-lint 4f3247d8` | messages all OK; count cap fails by design (6 commits, cap 1) |
| `check-adr-refs` | 156 bare, 556 qualified, all resolve |

### Commit SHAs after the round-4 fixups

| Phase | Was | Now |
|---|---|---|
| 1 | `64122d98` | `64122d98` (unchanged) |
| 2 commit 1 | `84bbb662` | `84bbb662` (unchanged) |
| 2 commit 2 | `c5497ca4` | `c5497ca4` (unchanged) |
| 2 commit 3 | `e2190b3b` | `e2190b3b` (unchanged) |
| 3 | `85c79459` | `ee5694a3` |
| 4 | `5baa9279` | `d26a6df9` |

**The stack was still never exercised.** Ports 3030/3333 remain held by the developer's own
dsp-api stack. Round 3's deferral stands: the first CI run of the `pinned` job is the real check,
and it is now also the first exercise of the composite action.

### Phase 5 — `04f5858d` `fix(dsp-cli): close the pre-migration security and correctness backlog`

74 files, +1200/-175. Five workers across two waves, each amending the one commit. **All five lost
their reports through the handback channel again** (fifteen of fifteen subagents over rounds 3 and
4), but every one of them wrote its report to `.claude/tmp/<chunk>-report.md`, which was given as
the durable channel this round. **That worked: all five reports survived.** Use it in every later
round — a worker that only returns its report returns nothing.

Checkboxes 551–558 ticked. 559–561 are tier-1 and untouched.

The commit touches **only files under `dsp-cli/`** — deliberately, and verified with
`git diff --stat HEAD~1 HEAD -- . ':!dsp-cli'` (empty) after every amend. release-please attributes
a commit to every package whose paths it touches and applies `Release-As` per attributed package,
so one root file in this commit would have bumped the workspace to 0.3.0 as well. That is why
`url` was **not** added as a direct dependency (it would have changed the root `Cargo.lock`);
`reqwest::Url` is used instead, since reqwest re-exports it.

#### Six places the plan described something other than what exists

The plan's Phase 5 anchors were measured before the rustfmt commit, and several were wrong about
more than line numbers. All six are corrected in place in the checkboxes.

1. **Fifteen auth-cache load sites, not four.** Nine use a multi-line `tracing::warn!(` whose
   `error = %e` sits on the next line, so the first worker's single-line grep found only six. The
   missed nine are in `vre/data_model.rs` (3), `vre/resource_type.rs` (2), `vre/vocabulary.rs` (2)
   and `vre/resource.rs` (2). **The pattern to use is `grep -rn -B 8 "error = %e" … | grep "warn!"`.**
   One multi-line hit in `vocabulary.rs` is a per-vocabulary tree fetch, not an auth-cache load, and
   was correctly left alone.
2. **The `local` shortcut expands to `http://0.0.0.0:3333`, and `0.0.0.0` is not loopback.** A
   loopback-only rule, which is what the plan specifies, would have made `dsp --server local` exit 2
   — the primary developer shortcut, documented in dsp-cli/ADR-0007, with its own unit test, and the
   value of `DSP_TEST_CLASS_IRI` in the drift job. The accepted set is loopback **or unspecified**,
   plus the domain `localhost`.
3. **`Url::host_str()` returns IPv6 hosts WITH brackets** (`"[::1]"`), the opposite of what the plan
   asserts. reqwest does not re-export `url::Host`, so the brackets are stripped before `IpAddr`
   parsing. The `http://[::1]:3333` test is what settles it either way.
4. **`ArgAction::SetTrue`'s default value parser rejects `DSP_ALLOW_INSECURE_SERVER=1`.** clap's
   `BoolValueParser` accepts only the literal strings `true`/`false`, so `=1` would have failed as a
   usage error. `BoolishValueParser` is set explicitly. Flag-before-env is clap's own default for an
   `env`-attributed arg and is covered by a test rather than assumed.
5. **`vre/project.rs` is not a stdout write site.** `grep -n stdout` on it returns nothing; project
   dump writes through the renderer. **`actions/auth/token.rs` is a site the plan omits** — its own
   comment already documented the exact bare-`?` path this phase fixes.
6. **Five truncate call sites in `client/http.rs`, not three.**

#### What the fix shapes are

- **Auth-cache leak.** `tracing::debug!(error = %e, …)` carries the `toml` error; the `warn` line is
  body-free. No shared helper — fifteen in-place edits, per YAGNI.
- **BrokenPipe.** A `BrokenPipeWriter` adapter in `src/util/mod.rs` wraps each stdout handle *where
  it is constructed*, so every later `write!`/`writeln!` through that sink is covered without
  touching the renderer trait or any public signature. `Diagnostic::from(io::Error)` is untouched:
  it is still reached from `auth.toml` reads, where a broken pipe is meaningless.
- **`--server` validation.** `Config::resolve` gained a second parameter, threaded through all 39
  call sites (18 in `lib.rs`, one in `main.rs`, the rest in tests). `--server` is **not** a global
  clap option — it is repeated on every subcommand's args struct — but `--allow-insecure-server` is,
  in the style of `--verbose`.

#### Both ADRs that this contradicts carry a dated amendment in the same commit

dsp-cli/CLAUDE.md requires it. dsp-cli/ADR-0007's "any other value is treated as a literal URL" is
no longer unconditional; dsp-cli/ADR-0012's exit-code table needed the carve-out that a broken pipe
is not a runtime error. Both use the dated `## Amendment` heading per root ADR-0006.

#### Test vacuity was checked, not assumed

Two of the three behavioural tests were run against the unfixed code and observed to FAIL first:
- the auth-cache test panicked showing the leak verbatim (`token = "eyFAKE.TOKEN.LEAK-…` quoted
  inside the `toml` parse error);
- the broken-pipe test failed with `Error: internal error: io error: Broken pipe (os error 32)` and
  exit 1.
The `--server` refusal test could not be run against the unfixed code in place, because the
signature and the validation landed in one edit; the equivalent was demonstrated in a scratch
binary instead. **The broken-pipe test is the one worth guarding:** it is vacuous if the whole
output fits the ~64 KiB pipe buffer before the reader closes, and every embedded `docs` topic is
under 9 KiB, so it relays a 512 KiB wiremock body through `vre sparql query` instead.

#### Two snapshot-hygiene findings

- **Accepting a snapshot by renaming `.snap.new` keeps insta's `assertion_line:` metadata**, which
  `cargo insta accept` strips. A snapshot carrying it churns on every unrelated line-number shift.
  It was stripped from the 34 snapshots this phase touched. Twenty-six *other* snapshots already
  carried it from earlier work; the strip was reverted on those, because the brief allows snapshot
  changes only where the change is the point. **Worth a separate cleanup commit some day.**
- **Every snapshot's stale `source:` metadata line finally rewrote itself.** Round 2 recorded that
  insta had started recording `dsp-cli/tests/cli.rs` instead of `tests/cli.rs` and that the headers
  would fix themselves on the next accept. That is what the 34 changed help snapshots show.

#### The `Release-As` footer must share a paragraph with `Co-Authored-By`

Written as its own paragraph first, and `git log -1 --format='%(trailers)'` then showed only
`Co-Authored-By` — git's trailer parser reads the **last** paragraph only. Collapsed into one
paragraph and re-verified; both now appear. **Check with `%(trailers)`, never by eye.**

#### Verification at `04f5858d`

| Gate | Result |
|---|---|
| `nix develop --command just check` | passes end to end (justfile hunk applied, then reverted) |
| `nix develop --command just test` | exit 0 |
| `cargo nextest run -p dsp-cli --all-features` | **1588 passed, 20 skipped** |
| `cargo clippy -p dsp-cli --all-features --all-targets -- -D warnings` | clean |
| `cargo doc --no-deps --all-features -p dsp-cli` | warning-free |
| `cargo publish -p dsp-cli --dry-run --locked --allow-dirty` | **73 files packaged**, 1.8 MiB (72 before; `connecting.md` grew, no new file) |
| `git log -1 --format='%(trailers)'` | `Release-As: 0.3.0` and `Co-Authored-By:` both present |
| `git diff --stat HEAD~1 HEAD -- . ':!dsp-cli'` | **empty** — no root file, so no root version bump |
| `just commit-lint 4f3247d8` | messages all OK; count cap fails by design (7 commits, cap 1) |
| `cargo test -p dsp-cli --lib corrupt_cache` | 13 passed — the one-off failure another worker saw was a concurrent-edit race in the shared worktree, not a flaky test |

#### Deliberately left out

- **`docs/src/dsp-cli/usage.md`** — a root file, so including it would have pulled the `Release-As`
  bump onto the workspace package. It does not enumerate flags exhaustively (it points at
  `dsp docs connecting`), so nothing in it is now false; but **the new `--allow-insecure-server`
  flag and `DSP_ALLOW_INSECURE_SERVER` are documented only inside `dsp-cli/`**. If that page should
  list them, it needs its own commit outside this one.
- `dsp-cli/README.md` and `dsp-cli/CLAUDE.md` were read and not changed: both are example-driven and
  defer to `dsp docs connecting` rather than enumerating options.

### Final commit SHAs for round 4

| Phase | SHA |
|---|---|
| 1 | `64122d98` |
| 2 commit 1 | `84bbb662` |
| 2 commit 2 | `c5497ca4` |
| 2 commit 3 | `e2190b3b` |
| 3 | `ee5694a3` |
| 4 | `d26a6df9` |
| 5 | `04f5858d` |

#### Late amend: one stale doc comment the signature change created

`Config::resolve` gaining a second parameter left `vre/project.rs:3820`'s doc comment reading
"`Config::resolve(None)` with no server yields a Usage error" — the claim is still true, the call
shape shown is not. Corrected to `Config::resolve(None, _)` and amended in; Phase 5 is `04f5858d`,
not `5016a3e2`. Suite re-run: 1588 passed, 20 skipped. Trailers, the empty
`':!dsp-cli'` diff and the count of 7 all re-verified after the amend.

**The general point for a later round:** a signature change is a rename for consistency-review
purposes. `git grep` the old call shape in prose — doc comments, `CONTEXT.md`, `docs/src/` — not
just in code, because the compiler cannot see a stale comment.

#### Two stale relays arrived after the round closed

The chunk-B-docs worker and the session both relayed messages describing work already landed: the
docs worker's six edits (verified from the tree and committed as part of `d26a6df9`), and the
`project.rs:3824` compile break (fixed by the orchestrator before the Phase 5 commit was created).
Neither needed action. Both corroborate the independent tree verification; neither contradicts it.

The session's note about serializing Phase 5 is worth keeping though: the parallel dispatch **did**
create that cross-chunk break, because `Config::resolve`'s signature change reached a file assigned
to a different worker. It cost one line to fix, and the two waves were otherwise disjoint, so the
parallelism paid for itself — but **a chunk that changes a shared signature should either own every
call site or be run alone.**

## Round 5 — 2026-09-20 — review fixes, then eight commits

Base `4f3247d8`, HEAD at start `04f5858d`. Backup branch `backup/dsp-cli-move-04f5858d` created
before any rewrite. Two workers, both sequential; both wrote their report to
`.claude/tmp/<chunk>-report.md` and both reports survived, while both returns were lost again
(seventeen of seventeen over rounds 3–5). **The file channel is the only channel.**

### Chunk 1 — amended into Phase 5

| Finding | What landed |
|---|---|
| 1a (Major, security) | A server value containing a control character is now **refused**, not sanitized |
| 1b | One `warn_auth_cache_load_failed` helper in `src/util/mod.rs`, called at all fifteen sites |
| 1c | Deleted `dsp_client_header_matches_user_agent` — it compared two identical `concat!`s |
| 1d | `is_local_host`'s doc comment corrected: `.host()` *is* reachable; `url::Host` is not |
| 1e | `docs/topics/errors.md`: one line on the broken-pipe exit-0 carve-out |
| 1f | "loopback or unspecified (127.0.0.0/8, ::1, 0.0.0.0, ::) or localhost" in the refusal string and `connecting.md`; `DSP_ALLOW_INSECURE_SERVER` out of "Environment-variable credentials" |
| 1g | `README.md` status line no longer names a version series |

#### Why 1a refuses rather than sanitizes

Round 4 sanitized the value only where it was interpolated into the `http://` refusal message. The
raw value still landed in `Config.server` and reached ~25 other diagnostics. Sanitizing the stored
value instead would have been worse than it looks: that string **is** the auth-cache key and the
base URL of every outgoing request, so stripping bytes from it would silently retarget both. A URL
never legitimately contains a raw control character, so the value is rejected outright, on any
scheme, and the rejection is **not** overridable by `--allow-insecure-server`.

**Placement matters: the check sits before `tracing::debug!(server = url, "resolved server")`,
not after.** Put after, a crafted value still reaches stderr verbatim at `-vv`. It also sits after
shortcut expansion and checks the expanded `url`, not the raw `s`.

#### 1a collided with an existing test, and the collision was the tell

`control_character_is_stripped_from_refusal_message` fed an **`http://`** value. After 1a it hits
the control-character refusal before the scheme check ever runs, so it would have kept passing
while testing something other than its name. It was converted in place — `https://` input, refused,
and the diagnostic asserted to carry no byte below `0x20` other than `\n` — rather than leaving a
stale test beside a new near-duplicate.

#### 1f is a user-facing string, not a comment

`config/mod.rs:112` is inside the `format!` of the refusal, so the wording change churned five
snapshots (`cli__insecure_http_refused_{prose,tsv,json,csv,lines}`). Expected and in scope. The
`--allow-insecure-server` clap help was checked and does **not** carry the same wording, so none of
the ~34 help snapshots moved.

**1f said "move" the `DSP_ALLOW_INSECURE_SERVER` bullet; the bullet was removed instead.** It is not
a credential, and the "Plain `http://` is refused" paragraph it would have moved next to already
names the variable and states the flag-beats-env precedence, so re-adding a bullet there would have
duplicated that sentence. Coverage is retained; the diff reads as a deletion. Embedded docs topic **bodies** are not snapshotted (only the docs
index, list and not-found output are), so 1e and 1f's `connecting.md`/`errors.md` edits produced no
snapshot churn at all.

#### 1b: the helper's home, and what the brief got wrong about it

`src/util/mod.rs`, called fully-qualified so no `use` block moved. The brief's justification was
that neither `auth/` site imports `actions/auth_state.rs`; that is true only of `auth/token.rs` —
`auth/status.rs` does import `read_auth_state`. The two sites that genuinely do not are
`auth/token.rs` and `vre/sparql.rs`, so the conclusion holds on corrected grounds. The three-line
"a `toml` parse error quotes the offending source line" comment now lives once, in the helper's doc
comment; fifteen copies deleted. `auth/status.rs`'s
`corrupt_cache_does_not_leak_token_bytes_at_default_verbosity` passed untouched, which is the point
of keeping it.

The module doc of `src/config/mod.rs` was not in either worker's named scope and summarized the
scheme validation without the new refusal; the orchestrator added the two-line correction.

### Chunk 2 — folded into Phase 3 (`ee5694a3` → `e1178630`)

`release-please.yml`'s `amend-lockfile` gate became
`needs.release-please.result == 'success' && needs.release-please.outputs.branches != '[]'`.
`actionlint` clean. Worth recording that this is **belt and braces, not a live break**: a job whose
`needs` was skipped is itself skipped under the implicit `success()`, so `fromJSON('')` was never
actually reached on a fork. The explicit gate states the intent rather than relying on that.

### Chunk 3 — folded into Phase 4 (`d26a6df9` → `bfa2a69a`)

`docs/src/deployment.md` and `docs/src/dsp-cli/testing-strategy.md` both described `stack.env` as
holding one pin and never said that `API` also selects the `dasch-swiss/dsp-api` tag
`load-fixtures.sh` checks the fixtures out from. `dsp-cli/ci/stack/README.md` had it right; both
book pages now match it. The testing-strategy heading became "Bumping the pins" — checked for
inbound `#bumping-the-pin` anchors first, and the only hits were in gitignored `docs/book/` output.

The "Reusable Actions" table's intro sentence claims to list the composite actions in
`.github/actions/`, so it gets **both** missing rows — `dsp-cli-stack-test` and the pre-existing
`commit-lint` — not just the one this migration added.

### Chunk 4 — a new eighth commit, `a1db6c23`

`DSP_ALLOW_INSECURE_SERVER` added to the "General configuration variables" table that
`docs/src/dsp-cli/usage.md` points readers at. Round 4 deliberately left this out of Phase 5
because it is a **root-path** file and would have pulled that commit's `Release-As: 0.3.0` onto the
workspace package; as its own commit it is attributed to the root package, which bumps anyway.
`usage.md` itself has no flags list, so nothing was added there.

### Verification at `a1db6c23`

| Gate | Result |
|---|---|
| `nix develop --command just check` | passes (`chore/justfile-fmt` hunk applied, then reverted) |
| `nix develop --command just test` | exit 0 |
| `cargo nextest run --locked --all-features --all-targets` | **3432 passed, 20 skipped** |
| `cargo nextest run -p dsp-cli --all-features` | 1588 passed, 20 skipped |
| `cargo clippy -p dsp-cli --all-features --all-targets -- -D warnings` | clean |
| `cargo doc --no-deps --all-features -p dsp-cli` | warning-free |
| `cargo publish -p dsp-cli --dry-run --locked --allow-dirty` | 73 files, 1.8 MiB |
| `nix run nixpkgs#actionlint` on `release-please.yml` | clean |
| `mdbook build docs` | clean |
| `check-adr-refs` | 156 bare, **566** qualified (556 before), all resolve |
| `git log -1 --format='%(trailers)'` on Phase 5 | `Release-As: 0.3.0` and `Co-Authored-By:` both present, re-checked after the autosquash |
| `git diff --stat 68f02289~1 68f02289 -- . ':!dsp-cli'` | **empty** |
| `just commit-lint 4f3247d8` | messages all OK; count cap fails by design (8 commits, cap 1) |

The `just --fmt` drift on `justfile` is pre-existing and unrelated to this branch; it is applied to
get a green `just check` and reverted before staging, exactly as in round 4.

### Final commit SHAs for round 5

| Phase | Was (round 4) | Now |
|---|---|---|
| 1 | `64122d98` | `64122d98` (unchanged) |
| 2 commit 1 | `84bbb662` | `84bbb662` (unchanged) |
| 2 commit 2 | `c5497ca4` | `c5497ca4` (unchanged) |
| 2 commit 3 | `e2190b3b` | `e2190b3b` (unchanged) |
| 3 | `ee5694a3` | `e1178630` |
| 4 | `d26a6df9` | `bfa2a69a` |
| 5 | `04f5858d` | `68f02289` |
| 6 (new) | — | `a1db6c23` |

**The stack is still never exercised.** Ports 3030/3333 remain held by the developer's own dsp-api
stack. The first CI run of the `pinned` job stays the real check, now in its third deferred round.

### Two mechanics worth carrying forward

- **A plain `git commit --fixup=<sha>` needs none of round 4's `GIT_EDITOR='cp …'` trick.** That
  trick is only for `--fixup=amend:`, which rewrites the message. Chunks 2 and 3 changed no message,
  so plain `--fixup` plus `GIT_SEQUENCE_EDITOR=true git rebase -i --autosquash --autostash` was
  enough.
- **Two chunks that touch the same file cannot be parallel, even on disjoint line ranges.** Chunks 3
  and 4 both edit `docs/src/dsp-cli/testing-strategy.md` and belong to different commits, so they
  ran strictly in sequence with the chunk-3 fixup committed before chunk 4's edit was made. The
  same reasoning kept the two chunk-1 workers sequential: disjoint files still share one crate and
  one `target/`, and a worker running `cargo nextest` over another's half-finished edit is the
  concurrent-edit race round 4 recorded.
