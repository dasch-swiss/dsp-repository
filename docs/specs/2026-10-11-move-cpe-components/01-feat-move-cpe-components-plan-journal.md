---
plan: docs/specs/2026-10-11-move-cpe-components/01-feat-move-cpe-components-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator/.claude/worktrees/DEV-7401
base_commit: 4b1cb8201648157abb502a9b0f3bf1c1e25745c2
branch: worktree-DEV-7401
started: 2026-10-11
problem: >
  CPE moves into dsp-repository crate by crate (ADR-0007, amendment of 2026-10-02); `cpe-core` is in
  review as #470 (DEV-7464). The next crate, the component library `cpe-components`, with its
  stylesheets and the image viewer it depends on (as a cut-down `cpe-image-viewer`), still lives in
  dsp-incubator. This plan moves both into `areas/access/cpe`, adds the mechanical check of CPE's
  dependency seam that ADR-0007 names, and ends with the incubator building against the vendored copies.
symptoms: []
status: in-progress
---

# Execution Journal: 01-feat-move-cpe-components-plan

## Repos

| repo          | base_commit                              | branch            | merge_strategy | status      | pr |
|---------------|------------------------------------------|-------------------|----------------|-------------|----|
| dsp-incubator | 4b1cb8201648157abb502a9b0f3bf1c1e25745c2 | worktree-DEV-7401 | squash         | in-progress | —  |
| dsp-repository | 1a503920c11c73065b0b06e615c966df010b4ffe | worktree-DEV-7401-image-viewer (PR A), worktree-DEV-7401 (PR B) | rebase | shipped | https://github.com/dasch-swiss/dsp-repository/pull/471 (PR A), https://github.com/dasch-swiss/dsp-repository/pull/475 (PR B) |

## Phases

| phase | status      | phase_base                                             | review_fix_rounds |
|-------|-------------|--------------------------------------------------------|-------------------|
| 1     | reviewed    | dsp-incubator@4b1cb8201648157abb502a9b0f3bf1c1e25745c2, dsp-repository@1a503920c11c73065b0b06e615c966df010b4ffe | 1                 |
| 2     | reviewed    | dsp-repository@27e2379a                                | 1                 |
| 3     | reviewed    | dsp-repository@5f05aac4, dsp-incubator@4b1cb8201648157abb502a9b0f3bf1c1e25745c2 | 2                 |
| 4     | reviewed    | dsp-repository@3652cd0b                                | 1                 |
| 4a    | reviewed    | dsp-repository@a4c61106                                | 0                 |
| 5     | gated (H2, H3, H4) |                                                 | 0                 |

## Chunk queue

| id  | repo          | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|---------------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-incubator | none (creates the worktree) | — | "- [ ] Fetch, then create the worktree named in `repositories:` …" | the worktree exists on `worktree-DEV-7401` at `origin/main`; `SRC` recorded | plan § *The source commit* | — |
| 1.2 | dsp-incubator | none (scratch files outside the repository) | 1.1 | "- [ ] Save the test list of `shared/dasch-image-viewer` …", "- [ ] Save the test list of `cpe-components` …" | both sorted test lists saved under the scratch directory; the worktree stays clean | plan § *Verification design* → *Test lists* | — |
| 1.3 | dsp-repository | `docs/specs/README.md`, the plan file (staged by the plan's own instruction) | — | "- [ ] Confirm the branch is `worktree-DEV-7401` …", "- [ ] Create and switch to the branch …", "- [ ] In `docs/specs/README.md` → *Plans and PRs* …", "- [ ] Commit this plan and the README change …" | branch `worktree-DEV-7401-image-viewer` exists at the base; commit 1 holds the plan and the README reword, nothing else | plan § *Commits (dsp-repository)*; `docs/specs/README.md` § Plans and PRs | — |
| 1.4 | dsp-repository | `areas/access/cpe/image-viewer/**`, root `Cargo.toml`, `Cargo.lock`, `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`, `eng.yaml`, the three `CONTEXT.md`, `docs/adr/0011-*.md`, `docs/adr/0007-*.md`, `docs/src/decisions.md`, `ARCH-MAP.md` | 1.3 | every remaining Phase 1 dsp-repository checkbox, from "- [ ] Where `SRC` is not `1cd5781f…`" to "- [ ] Commit as `feat(cpe-image-viewer): …`" (one commit by owner decision, so one chunk) | the proofs of *Verification design* → *Phase 1* hold when measured on the committed tree; `just check` and `just test` pass | plan § *`cpe-image-viewer`* (edit table), § *Verification design*, decisions 3-5, 10-12, 14, 16, 17 | — |
| 2.1 | dsp-repository | none (branch switch) | — | "- [ ] Switch to `worktree-DEV-7401` and fast-forward it …" | `worktree-DEV-7401` is checked out at `27e2379a` | plan § Phase 2, first checkbox | — |
| 2.2 | dsp-repository | `areas/access/cpe/components/**`, root `Cargo.toml`, `Cargo.lock`, `areas/access/cpe/core/src/**` (comments), `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`, the three `CONTEXT.md`, `docs/adr/0007-*.md`, `eng.yaml`, `ARCH-MAP.md` | 2.1 | every remaining Phase 2 dsp-repository checkbox, from "- [ ] Export `cpe/components` …" to "- [ ] Commit as `feat(cpe-components): …`" (one commit by owner decision, so one chunk) | the proofs of *Verification design* → *Phase 2* hold on the staged tree; `just check` and `just test` pass | plan § *`cpe-components`* (edit table), § *Formatting*, § *Verification design*, decisions 6, 9, 16 | — |
| 3.1 | dsp-repository | `areas/access/cpe/components/src/**/*.rs`, `areas/access/cpe/components/style/*.css` (comments only), `ARCH-MAP.md` (Fingerprint) | — | every Phase 3 dsp-repository checkbox, from "- [ ] Record `git rev-parse HEAD` …" to "- [ ] Commit as `docs(cpe-components): …`" (one commit by owner decision, so one chunk) | proofs 1 to 4 of *Verification design* → *Phase 3* hold against `5f05aac4`; `cargo doc -p cpe-components --no-deps` prints no warning; `just check` and `just test` pass | plan § *Comment rules (Phase 3)*, § *Verification design* → *Phase 3*; the edits are the spec files `p3-g*.spec` in the scratch directory, applied to the Phase 2 commit's files by `p3-apply.py` | — |
| 3.3 | dsp-incubator | `cpe/Cargo.toml`, `cpe/Cargo.lock` (uncommitted by owner decision; nothing else) | 3.1 | "- [ ] Arrange the incubator worktree as *The uncommitted check in the incubator* describes …", "- [ ] Run the incubator's module-graph check on the moved source …", "- [ ] Run the incubator's own crates' tests against that arrangement …", "- [ ] Save the arrangement as a patch …" | `cargo metadata` resolves with `cpe-core`, `cpe-components` and `cpe-image-viewer` taken from the dsp-repository worktree at `7ee7cf7a`; the module-graph result and the test result are recorded, each failure classed as moved-crate or also-on-`SRC`; the patch is saved under the scratch directory; `git status` in the incubator shows the two files modified and no commit | plan § *The uncommitted check in the incubator* | — |
| 4.1 | dsp-repository | `.github/scripts/check-cpe-seam.sh`, `.github/scripts/check-cpe-seam.test.sh`, `justfile`, `docs/adr/0007-*.md`, `ARCH-MAP.md`, `docs/src/workflows.md`, `areas/access/cpe/CONTEXT.md` | — | every Phase 4 dsp-repository checkbox from "- [ ] Write `.github/scripts/check-cpe-seam.test.sh` …" to "- [ ] Commit as `chore(ci): check CPE's dependency seam …`" (one commit by owner decision, so one chunk); the `gh stack`, `gh pr edit` and `just commit-lint` checkboxes are the session's | the test script fails without the script and passes with it; `just check-cpe-seam` passes on the tree and fails, naming manifest, dependency and rule, with a `cpe-components` dependency in `areas/access/dpe/web/Cargo.toml`; `just check` and `just test` pass | plan § *The seam check*; prototype in the scratch directory `dryrun-seam/` | — |
| 4a.1 | dsp-repository | `areas/access/cpe/image-viewer/src/lib.rs`, `tests/render.rs`, `assets/image-viewer.js`, `scripts/vendor-osd.sh` (comments only) | — | "- [ ] `cpe-image-viewer`: apply the rules …", "- [ ] `image-viewer.js` is equal to its previous version …", "- [ ] Commit the viewer's change as `fixup! feat(cpe-image-viewer): …`" | both Rust files equal without comments; every changed line of `image-viewer.js` a whole-line comment; `node --check` passes; `APPROXIMATION` and `const BUILTIN_ENGINE = "osd-wc"` present; `cargo test -p cpe-image-viewer` passes; no file under `assets/vendor/` changed | plan § *Comment rules (Phase 3)* and the Phase 4a preamble; edits in `.claude/tmp/p4a-viewer.py` | — |
| 4a.2 | dsp-repository | `areas/access/cpe/core/src/**/*.rs` (comments only), `ARCH-MAP.md` (Fingerprint, which also covers 4a.1's files) | 4a.1 | "- [ ] `cpe-core`: apply the rules …", "- [ ] Commit as `docs(cpe-core): remove design and plan references the first pass left (DEV-7401)`", "- [ ] Recompute the `areas/access/cpe` Fingerprint …" | every `.rs` file equal to its previous version after the Phase 3 normaliser; test list of `cpe-core` unchanged; `cargo doc -p cpe-core --no-deps` prints no new warning | plan § *Comment rules (Phase 3)* and the Phase 4a preamble; edits in `.claude/tmp/p4a-core.py` | — |
| 4a.3 | dsp-repository | `areas/access/cpe/ports/src/**/*.rs` (comments only) | — | "- [ ] `cpe-ports`: apply the rules …", "- [ ] If it changed: commit as `docs(cpe-ports): …`" | the same proof as 4a.2, or a journal row saying no comment needs a change | plan § *Comment rules (Phase 3)* and the Phase 4a preamble | — |
| 4a.4 | dsp-repository | none (gate) | 4a.1, 4a.2, 4a.3 | "- [ ] `just check` passes", "- [ ] `just test` passes" | both exit 0 on the phase's head | — | — |

## Chunks

| id  | repo          | status   | commit(s) | summary | blocker |
|-----|---------------|----------|-----------|---------|---------|
| 1.1 | dsp-incubator | complete | — (no commit by design) | Fetched; created the worktree on new branch `worktree-DEV-7401` from `origin/main`. `SRC` = `4b1cb8201648157abb502a9b0f3bf1c1e25745c2`, not the plan's `1cd5781f…`. | none |
| 1.2 | dsp-incubator | complete | — (no commit by design) | Saved the source test lists at `SRC`: viewer 51 names (47 in `tests/render.rs` + 4 doc tests), components 980 names. Worktree clean afterwards. | none |
| 1.3 | dsp-repository | complete | 0aacd793 | Created `worktree-DEV-7401-image-viewer` at `1a503920`; committed the plan and the `docs/specs/README.md` reword (one plan may be one stack of PRs, one phase group per PR). | none |
| 1.4 | dsp-repository | complete | 134467ca | Added `cpe-image-viewer` (the incubator's viewer at `4b1cb820`, one engine, assets through an `include_bytes!` table, 44 files), its README, ADR-0011, ADR-0007's amendment and note, the registrations (`CONVENTIONS.md`, `docs/src`, `eng.yaml`, three `CONTEXT.md`, `ARCH-MAP.md`). Proofs measured on the tree: 41 vendored files match `VENDOR.lock`; vendor files identical to source; test list = source − 22 tests − 2 doc tests + 1; builder matrix 26/26 equal; clippy `--all-targets` and `cargo doc` without warning; `just check` and `just test` pass. | none |
| 1.5 | dsp-repository | complete | 9826fc39 (amended into 1.4's commit, which was 134467ca before) | Phase review, fix round 1: `docs/src/repo_structure.md` counts eleven ADRs and names ADR-0010 and ADR-0011; the dated note in ADR-0007's DEV-7464 amendment also covers that amendment's "Enforced by" line; ADR-0011's rejected option names the image viewer; the README says the element still observes `engine`. Fingerprint recomputed (`f7607a19c650`). `just check` passes. | none |
| 2.1 | dsp-repository | complete | — (no commit by design) | Switched to `worktree-DEV-7401` and fast-forwarded it from `1a503920` to `27e2379a` (head of `worktree-DEV-7401-image-viewer`). | none |
| 2.2 | dsp-repository | complete | 6c0405d3 (amended by 2.3 into d9bab73a) | Added `cpe-components` (the incubator's `cpe/components` at `4b1cb820`, 26 Rust files, 147 snapshots) and its seven stylesheets in `style/`; four fixtures under `src/component/testdata/`; the edit table's edits; four format-forced rewrites (`annotation.rs`, `showcase.rs` twice, `resource_detail.rs`, plus a reworded doc line in `editorial.rs`); `cpe-core`'s 16 + 4 citations repointed; registrations (`CONVENTIONS.md`, `docs/src`, three `CONTEXT.md`, ADR-0007's amendment, `eng.yaml`, `ARCH-MAP.md`). Proofs measured on the staged tree: snapshots identical; stylesheets differ in seven lines; fixtures equal the source, the extract reproducible; test list equal, 980; baseline diff 13 files, every hunk an edit-table row or a named rewrite; `cpe-core` equal under the Phase 3 normaliser; `Cargo.lock` gained one entry; clippy `--all-targets` clean; `cargo doc` 45 warnings; `just check` and `just test` pass; Fingerprint `a4f54d263ac2` (before 2.3). | none |
| 2.3 | dsp-repository | complete | d9bab73a (amended into 2.2's commit, which was 6c0405d3 before) | Phase review, fix round 1 (reviewers: consistency, rust): `areas/access/CONTEXT.md`'s opening paragraph names `cpe-components`; the comment in `cpe-image-viewer`'s `image-viewer.js` that names the twin of its overlay rules points at `areas/access/cpe/components/style/media.css`; `eng.yaml`'s comment says the browser check does not reach `components/style/`; ADR-0007's amendment heading names the stylesheets; `PROJECT_RENDERERS` is marked private in `ARCH-MAP.md` and CPE's `CONTEXT.md`; `incunabula_books()`'s doc comment says a book's lines are adjacent. Fingerprint recomputed (`4a29fa59861f`). Proofs, `just check` and `just test` run again and pass. The fix diff was not reviewed again (owner: one light round). | none |
| 3.1 | dsp-repository | complete | f7d00168 (amended by 3.2 into 7ee7cf7a) | Comment pass over `cpe-components` (26 Rust files, 7 stylesheets): plan, slice, step, decision, assumption and acceptance-criterion ids and pointers into `docs/plans/`, `docs/design/`, `DESIGN.md`, `BACKLOG.md`, `docs/cpe-poc.md` removed; change narration removed or put in the present tense; incubator crates and files named by path; `cpe-core` files as `areas/access/cpe/core/src/<file>`, stylesheets as `style/<file>`; the four Phase 2 leftovers taken (`lib.rs`, `media.css:887`, `annotation.rs`, `media.rs`); the 45 `cargo doc` warnings gone. Proofs against `5f05aac4`: Rust equal without comments (26 files, nothing printed); no changed non-comment Rust line with a double quote; CSS equal without comments (7 files); only `.rs` and `.css` files changed, no `.snap.new`; test list equal, 980; `cargo doc` without warning; `just fmt` stable on the second run; `just check` and `just test` (5493 passed) pass; Fingerprint `407bd363a6a4` (before 3.2). | none |
| 3.2 | dsp-repository | complete | 7ee7cf7a (amended into 3.1's commit, which was f7d00168 before) | Phase review, fix round 1 (reviewers: consistency, code-simplicity). Rewrites that had turned history into a false present-tense claim are gone (`chrome.rs`'s two "No `expect(dead_code)` of its own" paragraphs, `media.rs`'s `Icon` doc: the crate has no `dead_code` attribute). Comments that named a shared `component::pager::render_pager`, which does not exist, say what is there (`compound_viewer.rs` module doc, `render_compound_pager` doc and one inline comment, `resource_list.rs` module doc, `pager.rs`). `compound_viewer.rs`'s module doc no longer says a no-JS reload lands focus on the article; `POPSTATE_RELOAD_EXPR` keeps one line against header or signal discrimination; `record.css`'s rail comment no longer speaks of an anchored page step. Ids the first pass left: `Fix N`, `Fix A/B/C (round 2)`, "round C", "item 6, Backlog hardening batch 3", `Choice E/F/G`, `HDM SLICE 3`, "a later step", "proposed" beside two accepted ADR sub-items. Proofs 1 to 3, the search, `cargo doc`, `just check` and `just test` (5493 passed) run again and pass; test list not taken again (no code change by proof 1); Fingerprint `8a6ee970f829`. Commit: 31 files, 949 insertions, 1173 deletions. The fix diff was not reviewed again (owner: one light round). | none |
| 3.3 | dsp-incubator | complete | — (no commit by owner decision; `cpe/Cargo.toml` and `cpe/Cargo.lock` left modified in the worktree for H2) | Pointed the incubator's `cpe` workspace at the dsp-repository worktree's three crates at `7ee7cf7a` by absolute path (`cpe-core`, `cpe-components`, `dasch-image-viewer` with `package = "cpe-image-viewer"`) and removed the members `core` and `components`. Cargo resolved it. Module-graph check on the moved `components/src`: exit 0, no cycle, no root edge. `cargo test --workspace --all-targets --no-fail-fast` from `cpe/`: exit 0, 1530 passed, 0 failed, 3 ignored, in 13 test binaries. Patch saved in the scratch directory. No phase review (nothing committed). | none |
| 3.4 | dsp-repository | complete | e4ad8b9c (amended into the phase's commit, which was 7ee7cf7a before) | Fix round 2, the owner's instruction of 2026-10-11 ("more strict"): rule 3 read widely. Removed from the comments of 17 Rust files and 6 stylesheets: mentions of a design artefact without a path (design-doc and handoff sections, `strings.md` keys, `*.prompt.md`, `*.dc.html`, `icon-paths.md`, the mockup and its `H###`/`L###` line numbers); the state ids `E1`–`E7` and `N1`–`N3` (now "the empty class", "the unknown-path / unknown-project / unknown-record page"); `CV-2`, `RL-2`, `FP-2`, `F2`/`F7`/`F9`/`F14`, `AC<n>`, showcase case ids, `DEVIATION #n`, "Slice-3b status note"; dated owner decisions (the rule each marked stays); "the bin" (`dsp-incubator cpe/bin` where the comment says what it must do, else "the host"); history wording the first review listed ("retired plate", "the old", "now", "any more", "pre-existing", "this step wires in", "Found on hands-on pixel inspection", "before this refactor"). A `strings.md` key that named a message is the message's Rust name. `chrome.rs`'s section header names `component/project/incunabula/timeline.rs`. Review (code-simplicity only; consistency, dune, rust, maud-datastar, accessibility, ivan, security skipped by owner instruction): 9 findings, 7 fixed (one Critical: `render_not_found_breadcrumb`'s paragraph "the class crumb appears ONLY on this not-found variant", false against `render_breadcrumb`, deleted; four missing spaces from trimmed spec lines; three sentences that lost their referent), 1 refuted (`§3` in `media.css` is that file's own section 3), 1 left ("plate" in `media.css` section 1). Proofs against `5f05aac4`: Rust equal without comments (26 files, nothing printed); no changed non-comment Rust line with a double quote; CSS equal without comments (7 files); no file changed outside `src/**/*.rs`, `style/*.css` and `ARCH-MAP.md`; no `.snap.new`; test list equal, 980; `cargo doc` without rustdoc warning; `just fmt` stable; `just check` and `just test` pass (run again after the review fixes); Fingerprint `1737106bb5a8`. Commit: 31 files, 1387 insertions, 1699 deletions. The edits are `p4-g1.spec` … `p4-g6.spec` and `p4-fix.py` under `.claude/tmp/` (`p4-run.sh apply` reproduces them from `7ee7cf7a`). | none |
| 4.1 | dsp-repository | complete | 624641c5 (amended by 4.2 into 05420fbf) | Added `.github/scripts/check-cpe-seam.sh` and its test script (54 assertions; they failed without the script, 0 of 54, and pass under bash 3.2 and 5.x), the recipe `check-cpe-seam`, its entry in `just check`'s dependencies and in `just test`'s script list. Started from the scratch prototype; against the plan it gained the guidance that names `PORT_EXCEPTIONS` and `LEAVES` and no crate, `if` in every loop body, and test cases for `subpackage =`, `[build-dependencies.name]`, a quoted header with a trailing comment and an untracked exception path. On the tree: 21 manifests, 4 CPE and 2 Mosaic crates, passes; with `cpe-components` added to `areas/access/dpe/web/Cargo.toml` it fails and prints the manifest, the dependency and rule 1; manifest restored, `git status --porcelain areas/access/dpe Cargo.lock` empty. ADR-0007's amendment of 2026-10-11 (DEV-7401) states the three rules and its "Enforced by" line; `ARCH-MAP.md` (CPE's Boundary rules and Used by, the composition root's Depends on, the CI list), `areas/access/cpe/CONTEXT.md` and `docs/src/workflows.md` name the script. Fingerprint `27f56407b73d` over the staged tree. `just check` and `just test` pass. | none |
| 4.2 | dsp-repository | complete | 05420fbf (amended into 4.1's commit, which was 624641c5 before) | Phase review, fix round 1 (reviewers: devops, consistency; code-simplicity, dune and rust skipped by owner instruction). Each matcher or gate finding was first a failing test case (7 of 67 failed before the fix). Fixed: a table header with the spaces TOML allows (`[dependencies.cpe-core ]`, `[ dependencies . cpe-core ]`) was missed; a quoted rename key (`"package" = "cpe-core"`) was missed; a manifest without `[package]` under `areas/access/cpe/` was not read for a renamed Mosaic crate; a tracked manifest missing from the work tree was skipped and the gate passed (now an error, as is a failing `git ls-files` for the manifest list); the tests only sourced the script, so its own `set -euo pipefail` path was not run (two fixtures now execute it, one with two violations); the fixtures committed, which runs a developer's hooks and signing (they stage only: the gate reads the index). `ARCH-MAP.md`'s `areas/access/sync` entry said `sync-store` depending on no other CPE crate is review; it is static-analysis now. Refuted: the header's DEV-7465 sentence as a plan reference (the plan's checkbox asks for it). 67 assertions pass under bash 3.2 and 5.x; `just check` and `just test` pass; Fingerprint unchanged (`27f56407b73d`, no file under `areas/access/cpe/` touched). The fix diff was not reviewed again (owner: one light round). | none |
| 4a.1 | dsp-repository | complete | b1126078 (`fixup!` of PR A's commit, by the plan's checkbox; the session folds it) | Comment pass over `cpe-image-viewer`: 25 edits in `assets/image-viewer.js` (17), `tests/render.rs` (6) and `src/lib.rs` (2). Removed: "record-viewer plan", "Risk R1", "R1/AC16", "the design table", "B2" (four places), "Fix C", "crate step 1", "crate Step 2"; change narration ("added later", "Added additively … no existing field removed or renamed", "unchanged from before this method existed", "the pre-existing hardcoded `showRotationControl: true` this replaces", "the old hide-immediately", "keeps the existing"). Put in the present tense, the rule kept: the zoom-out floor at fit ("was tried … reintroduced" → what a sub-fit floor does), the relative home tolerance ("looked tight enough … was the direct cause"), the exposure and gamma ranges ("pinned by `tests/render.rs`"). False against the code and corrected: the file header called the crate `dasch-image-viewer`. `scripts/vendor-osd.sh` and the README needed no change; nothing under `assets/vendor/` touched. Proofs against `a4c61106`: both `.rs` files equal without comments; no changed non-comment line in any file; `node --check` exit 0; `APPROXIMATION` on 6 lines, `const BUILTIN_ENGINE = "osd-wc"` once; test list equal to Phase 1's, 28 names; `cargo test -p cpe-image-viewer` 26 + 2 doc tests pass; `cargo doc` without rustdoc warning; `just fmt` changed nothing; `just check` passes. No `ARCH-MAP.md` hunk in this commit (it will be folded into PR A): the Fingerprint moves in 4a.2's commit. Left as they are: "the metrics harness" (two comments; the harness is in dsp-incubator, its path not known here), "verified in Chrome" (three comments, evidence for a present workaround). | none |
| 4a.2 | dsp-repository | complete | 8eb5c7e2 | Comment pass over `cpe-core`: 48 edits in 11 of 28 files (`diagnostic.rs`, `iiif.rs`, `ir.rs`, `labels.rs`, `lang.rs`, `markup_contract.rs`, `theme.rs`, `util.rs`, `validate.rs`, `validate/context.rs`, `view_data.rs`). Removed: `CV-2` (two), `RL-2` (two), `§8 "warmth lever"`, "the reference design's `96`", "the design caps" / "the design cap" (the cap of three stays), "the design's eyebrow", "not implemented in this slice", "PoC", "the pre-existing fallback — unchanged behaviour", "pre-existing … already". "The bin" (30 places) is "the host", or `dsp-incubator cpe/bin` where the comment names that crate (`view_data.rs`), as Phase 3 did in `cpe-components`. "The design-system `tokens.css`" is `areas/access/cpe/components/style/tokens.css`. False against the code and corrected: "the engine needs its own copy since it cannot depend on the `bin` crate" and "the engine has no regex dependency" and "the engine owns the check" meant this crate; `RESERVED_FACET_PROPERTIES`' doc placed `parse_view_param` "inside the engine", it is in `cpe-components`. Left: "the engine" as the domain word (about 60 places), `v1` (the config language's version), "Until DEV-7465" in `lib.rs` (present state with its issue), pointers to `dsp-incubator cpe/CLAUDE.md`, `cpe/CONTEXT.md` and `cpe/docs/COMPONENT_LIBRARY.md` (files that state a rule the code follows), "Layer-4 test" in one test comment. Proofs against `b1126078`: 28 `.rs` files equal after the Phase 3 normaliser; no changed non-comment line; test list equal, 549 names (taken before and after); `cargo doc -p cpe-core --no-deps` no rustdoc warning before or after; `just fmt` changed nothing; `just check` and `just test` (5493 passed) pass. `ARCH-MAP.md`: `areas/access/cpe` Fingerprint `b4aeb393bad7` over the staged tree, covering 4a.1's files too. | none |
| 4a.3 | dsp-repository | complete | — (no commit: no comment needs a change) | `cpe-ports` read (four files, the wide search and a history-word search): no plan, journal, backlog, design-document, slice or step reference and no change narration. The hits are pointers that stay: `FORMAT.md` §10 and §12 D10/D11 (the pinned format, as in `sync-store`), ADR-0010 with "proposed; DEV-7488" (the ADR's status is `proposed`, checked), and the DAO decision and question ids `d.48`, `d.69`, `d.70`, `d.71`, `Q10`, `Q29`. Those ids are left by judgement: the module doc of `snapshot.rs` names their file (`dsp-repository-design`, `spycherli/decisions-active.md`) and says a provisional field "names the decision"; removing them would make that sentence false and lose what a reader needs to know when the field may change. For the owner if the strict reading should take them too. | none |
| 4a.4 | dsp-repository | complete | — (gate) | On `8eb5c7e2`'s tree: `just check` exit 0, `just test` exit 0 (5493 passed, every script test passed). | none |
| FR1.1 | dsp-repository | complete | 5712e285 (`fixup!` of PR A's commit; the session folds it) | Final review, fix round 1, viewer (V1–V10). `image-viewer.js`, comments only: the `atHome` rule moved above `atHome` with its own numbers (2% relative band, ~0.5% settling noise, ×1.5 steps; `atNative` is the 5% band); the `getZoom(false)` note is its rule; `viewer-rendered` is described as an event for hosts with its detail, no harness and no "switch" (also the two further harness mentions, at `crossOriginPolicy` and at the handlers); "verified in Chrome" (both), "verified:", "hue is ours", three Fragmentarium references (one in `lib.rs`) gone; the JS header no longer says "engine-agnostic … swappable" (same claim as V4, not in the list). `src/lib.rs`: module doc says one engine, one consumer; `Viewer::fallback` says the fallback is hidden on a failed open; new unit test `tests::asset_names_are_unique`. `tests/render.rs`: positive canary `"tile-sources"` in the observed-attributes slice; content type of `vendor/VENDOR.lock` and `LICENSE-openseadragon.txt` asserted. README: `viewer-rendered` and `viewer-warning` listed, `nosniff` sentence, `Viewer::new` trusts its caller, "ADR-0007's amendment". ADR-0007: a dated note under the DEV-7400 churn bullet and under the DEV-7464 "Enforced by" line (no existing sentence edited; the DEV-7400 "Enforced by" line at ~259 is about re-vendoring, not churn, and got none). V8 not applied: "must match the design range" at `render.rs` 274/278 are assertion message strings. Proofs against `e1b499dd`: no changed JS line that is not a whole-line comment; `node --check` exit 0; `APPROXIMATION` 6 lines, `const BUILTIN_ENGINE = "osd-wc"` once; nothing under `assets/vendor/` changed; `cargo test -p cpe-image-viewer` 1 + 26 + 2 doc tests pass; test list 29 names (28 + the new unit test); each new assertion seen red (duplicate `ASSETS` row; `lock` arm removed; `txt` arm removed; `"tile-sources"` renamed in the list), tree restored; `cargo doc` no rustdoc warning; `just check` exit 0. | none |
| FR1.2 | dsp-repository | complete | 95b9bfb9 (`fixup!` of the Phase 3 commit) | Final review, fix round 1, `cpe-components` comments (C1–C5), 17 edits in 5 Rust files and 4 stylesheets (`.claude/tmp/fr1-c.py`). C1: `media.rs`'s `TECH_STACK.md` pointer dropped, the rule kept (`dsp-incubator cpe/TECH_STACK.md` does not exist in the incubator worktree). C2: `incunabula.css` names `dsp-incubator cpe/CLAUDE.md` (exists). C3: "the design's …" gone at the ten listed sites and at `media.rs`'s test comment "The design table's slider …"; `project/mod.rs` lost the parenthesis "(the design's flat `/narrenschiff`)" whole, `tokens.css` says "a grouping only" for the design tool's Tweaks panel. C5: `compound_viewer.rs` header says "DataStar-enhanced"; `media.rs` three comments name the object-form `data-attr` binding where they named `data-attr-aria-disabled` / `data-attr-data-rail`. Not applied: C4, both sites are assertion message strings (`resource_detail.rs` "record-viewer plan step 4", `resource_list.rs` "Step 6; A12.1" twice); C5 at `media.rs` ~2319 and ~330, which name `data-attr-hidden` as the form never to write, and the test asserts its absence. Proofs against `5712e285`: 26 Rust files equal without comments; no changed line that is not a whole-line comment; 7 stylesheets equal without comments; no `.snap.new`; test list equal to Phase 3's, 980; `cargo doc -p cpe-components --no-deps` no rustdoc warning; `just fmt` changed nothing; the Datastar delimiter gate passes; `just check` exit 0. | none |
| FR1.3 | dsp-repository | complete | 5f39d278 (`fixup!` of 8eb5c7e2) | Final review, fix round 1, `cpe-core` comments (K2), 3 edits (`.claude/tmp/fr1-k.py`): `PageStatus`'s doc keeps "the host"; `PageParams`' and `RESERVED_FACET_PROPERTIES`' docs keep the path. Not applied: K1, `validate.rs` "in this slice)" is inside a `format!` diagnostic message. Proofs against `5712e285`: 28 Rust files equal without comments; no changed line that is not a whole-line comment; test list equal, 549; `cargo doc -p cpe-core --no-deps` no rustdoc warning; `just check` exit 0 (run on the tree holding FR1.2 and FR1.3 together). | none |
| FR1.4 | dsp-repository | complete | 9f226af4 (`fixup!` of 05420fbf; made last) | Final review, fix round 1, seam (S1–S5). Tests first: 27 new assertions in `check-cpe-seam.test.sh`, 18 of 94 failed before the script changed. S1: the matcher reads a name that is a key inside an inline table (after `{` or `,`, then `=` or `.`, on a line with no `#` before it), so `dependencies = { cpe-core = { … } }` under `[target.'cfg(unix)']` fails rule 1; also covered for rule 2 and rule 3; a longer name, a feature string and a commented line do not match. S2: a `[patch.<source>.name]` header matches (bare and quoted source); `[package.metadata.<name>]` does not. S3, both: `crate_name` reads an indented `name`, a `[ package ]` header with spaces or indentation, a quoted `"name"` key and a top-level `package.name`; `has_package` accepts the same spellings and `package =`; every `areas/access/cpe/*/Cargo.toml` and `modules/mosaic/*/Cargo.toml` whose name is still unreadable (inline `package = {…}`, no `name`, no package table) fails with `✗ <file>: cannot read the [package] name`; a crate outside with a `[ package ]` header is checked as a crate. S4: the header's three sentences. S5: `ARCH-MAP.md` no longer says "the rest is docs-only": the ban on DPE's and `sync`'s crates (ADR-0008) and `cpe-components`' own dependency list are **review**; rules 1 and 2 cover a new crate, a new leaf goes into `LEAVES` (also in CPE's `CONTEXT.md`); the `PreEscaped` line. Not applied: S6, the sibling recipes keep their ticket ids in the doc comment (`(DEV-7397, ADR-0007)`, `(DEV-6920)`, `(DEV-7330)`, `(DEV-7046)`), so `(DEV-7401, ADR-0007)` matches them. No rule added; tools unchanged (bash, git, awk, grep); `check-composition-root-deps.sh` untouched. Proofs: 94 assertions pass under `/bin/bash` 3.2.57 and bash 5.3.9; `just check-cpe-seam` on the tree: 21 manifests, 4 CPE and 2 Mosaic crates, passes; the negative smoke (`cpe-components` in `areas/access/dpe/web/Cargo.toml`, staged) fails with the manifest, the dependency and rule 1, and `git status --porcelain areas/access/dpe Cargo.lock` is empty after the restore; `shellcheck` is not installed here. Fingerprint `256145276030` over the staged tree (269 files), by the recipe of `.claude/tmp/p4-fingerprint.sh`. On the final tree: `just check` exit 0, `just test` exit 0 (every script test passes). | none |
| FR2.1 | dsp-repository | complete | 0bbea754 (`fixup!` of 05420fbf) | Final review, fix round 2, seam. Tests first: 9 new assertions in `check-cpe-seam.test.sh`, 2 of 103 failed before the script changed (the root fixture with a plain `cpe-core = { path = … }` under `[workspace.dependencies]` and a `[workspace.dependencies.foo]` table holding `package = "x"`: rule 1 reported and 9 manifests counted); the other 7 pin what must stay (a rename in a root dependency table fails; a crate outside with top-level `package.` keys is checked as a crate; a crate outside with `[dependencies.foo]` and `package = "cpe-core"` fails rule 1; a string value `description = "a, cpe-core. b"` is reported). `has_package` is awk, as `crate_name` is: the `[package]` header, with the same pattern as `crate_name`'s, counts anywhere; a `package.` or `package =` key counts only before the first table header. Measured on `/usr/bin/awk` and gawk 5.3.2 with the same answers: CRLF `[package]` yes, `[ package ] # c` yes, a `package =` key in a table body no, top-level `package = {…}` yes, `[package.metadata.x]` alone no. The inline-table matcher is unchanged; its comment says a string value containing `, name.` or `, name =` is reported and why. ADR-0007's amendment of 2026-10-11 gained two sentences at the end of the seam bullet (`LEAVES` is the list rule 3 goes by; what the script does not check is review, in `ARCH-MAP.md`'s words), and CPE's `CONTEXT.md` the second of them; no existing sentence edited. No rule added; tools unchanged; `check-composition-root-deps.sh` untouched. Proofs: 103 assertions pass under `/bin/bash` 3.2.57 and bash 5.3.9; `just check-cpe-seam` on the tree: 21 manifests, 4 CPE and 2 Mosaic crates, 1 excepted, as before; the negative smoke (`cpe-components` in `areas/access/dpe/web/Cargo.toml`) fails with the manifest, the dependency and rule 1, and `git status --porcelain areas/access/dpe Cargo.lock` is empty after the restore. Fingerprint `e730df0462f6` over the staged tree (269 files; the recipe hashes `areas/access/cpe/CONTEXT.md`). `just check` exit 0 and `just test` exit 0 on the tree holding FR2.1 and FR2.2. Commands in `.claude/tmp/fr2-proof.sh`, `fr2-gate.sh`, `fr2-commit.sh`. | none |
| FR2.2 | dsp-repository | complete | 93b5a770 (`fixup!` of PR A's commit; the session folds it) | Final review, fix round 2, ADR-0007: a blank line between the DEV-7464 amendment's "Enforced by" line and the dated note under it, which Markdown rendered as one paragraph. The other two dated notes (under the DEV-7400 churn bullet and under "The rule stands for the crates that follow") already have one. One inserted line, nothing else. | none |
| FR3.1 | dsp-repository | complete | e88c747e (`fixup!` of PR A's commit; the session folds it) | Finalizer proposals, `areas/access/cpe/image-viewer/src/lib.rs`, comments only, 2 edits. The comment in `Viewer::render` states the rule (both attributes omitted at their defaults, so a consumer that never calls `.chrome()` / `.page()` gets no `chrome` or `page` attribute) where it said "byte-identical markup to before these existed (additive)"; true to the code under it (`chrome` true gives `None`, `initial_page` `None` gives no `page`) and pinned by `chrome_is_on_by_default_and_omitted_from_markup` and `page_is_absent_by_default`. The module doc says "the bundled `image-viewer.js`" where it said "vendored"; the other three "vendored" in the file name OpenSeadragon and stay. `tests/render.rs` has neither pattern in a comment. Proofs: `git diff` 2 insertions, 3 deletions, whole-line comments only; `just fmt` changed nothing; `cargo test -p cpe-image-viewer` 1 + 26 + 2 doc tests pass; `cargo doc -p cpe-image-viewer --no-deps` no warning; `just check` exit 0. `ARCH-MAP.md` untouched. | none |

## Deferrals

- none

## Deviations

- **Phase 4, tools the seam check uses (committed, 05420fbf).** Decided: the check is "bash + git + awk only". Committed: it also calls `grep` (the matcher), and `wc` and `tr` for the success line, as `check-composition-root-deps.sh` and `check-shared-paths.sh` do; its header lists "bash (3.2 and later), git, awk, grep". Reason: the prototype the plan verified ("BSD and GNU `grep` and `awk`") is grep-based, and the plan's own matcher description is a line grep. No behaviour of the three rules differs. For the owner: keep, or ask for an awk-only matcher.

## Side findings

- **Final review, fix round 1 (FR1.1–FR1.4): for the session.**
  - **Folding stops twice on `ARCH-MAP.md`'s Fingerprint line.** `05420fbf` is not the last commit that touches
    `ARCH-MAP.md`: `8eb5c7e2` changes the Fingerprint line after it. A dry run in a throwaway detached worktree
    (`.claude/tmp/fr1-fold-dryrun.sh`, `rebase -i --autosquash 1a503920`, branch untouched, worktree removed)
    stops at `9f226af4` (take the fixup's line, `256145276030`) and again at `8eb5c7e2` (keep the folded line,
    `256145276030`); nothing else conflicts, and the folded tree equals the branch head's tree. The first run of
    that script had `rerere` on and recorded a preimage for `ARCH-MAP.md`, no resolution.
  - **Findings not applied, each because the text is a string literal or the finding's premise does not hold:**
    V8 (`render.rs` "must match the design range", assertion messages); C4 (`resource_detail.rs`, `resource_list.rs`,
    assertion messages); K1 (`validate.rs` "in this slice)", a diagnostic's `format!` text, so user-visible: for the
    owner if the message should change); S6 (sibling recipes carry ticket ids); C5 at `media.rs` ~2319/~330
    (`data-attr-hidden` is named as the form never to write).
  - **Beyond the list, same kind:** the JS file header's "engine-agnostic … swappable" (V4's claim); "the metrics
    harness" at `crossOriginPolicy` and "Metrics observation hooks" at the handlers (V2); "verified in Chrome" at the
    drawer option (V3); "Fragmentarium idiom … of our own" in `Viewer::adjust`'s doc (V3); `media.rs`
    `data-attr-data-rail` and the second `data-attr-aria-disabled` (C5); "The design table's slider" (C3).
  - **Counts that moved:** the viewer's test list is 29 names (the new unit test `tests::asset_names_are_unique`);
    the seam test script has 94 assertions (the plan's *The seam check* says 44, Phase 4 left 67).
  - **Not read by the seam check, by its design:** a `[patch]` table in the workspace root (the root has no
    `[package]`, so only a rename is read there, as for `[workspace.dependencies]`); an inline-table key on a line
    with a `#` inside a string before it.
  - `README.md` of the viewer says CPE filters each tile-source URI through `cpe_core::util::is_safe_external_url`;
    checked at `components/src/component/media.rs` (the filter over representations), not at every `Viewer::new`
    call site.

- **Phase 4 review, suggestions not taken.** (1) `PORT_EXCEPTIONS` emptied would abort under `set -u` on bash before 4.4 (`"${PORT_EXCEPTIONS[@]}"`); it fails loudly, and DEV-7465 extends the list. (2) The test script has no `trap` that removes its fixtures when it is interrupted, and does not unset `GIT_CONFIG_*`; the template scripts have neither. (3) A path with non-ASCII characters is quoted by `git ls-files` (`core.quotepath`) and would now be reported as missing from the work tree. (4) `ARCH-MAP.md`'s `areas/access/cpe` Local-context kit does not list the script (the plan adds no kit file); its "`cpe/ports` reaches only `std` and `shared-*`" and "`cpe-image-viewer` depends on `maud` alone" stay **review**, though rule 3 now covers their no-CPE-crate half. (5) No CI job runs `just test`, so no gate's `.test.sh` runs in CI (`.github/workflows/check.yml` runs `just check`); not specific to this phase. (6) `ARCH-MAP.md`'s cross-cutting CI list still omits `check-composition-root-deps.sh`. (7) The plan's *The seam check* says 44 assertions; the committed test script has 67.

- **`SRC` is `4b1cb8201648157abb502a9b0f3bf1c1e25745c2`**, one commit after the plan's `1cd5781f…`
  (`4b1cb820 refactor(cpe): read Incunabula's catalogue override from curation (#503)`).
  `git diff --stat 1cd5781f 4b1cb820 -- shared/dasch-image-viewer cpe/components cpe/static/cpe cpe/store/schema.sql`
  is empty, so the plan's source line numbers and counts for those paths still hold. #503 does change
  `cpe/vendor/PIN`, the vendored `areas/access/sync` tree, `cpe/CLAUDE.md`, `cpe/docs/INCUNABULA_GO_LIVE.md`
  and `cpe/store-build`, which Phase 3's incubator check and Phase 5 read.
- **Scratch files** (outside both repositories; the directory is session scratch and may not outlive the
  session), all under
  `/private/tmp/claude-501/-Users-balduinlandolt-Documents-GitHub-dasch-swiss-dsp-repository/c76325fa-6ce7-4eec-9291-53503a09a89b/scratchpad/dev7401/`:
  - `src-test-list-dasch-image-viewer.txt`: the viewer's source test list, 51 lines,
    sha256 `12926a483d47a3c5a4c6db2972f5f6306090229a3be6b1015928579cf2881c0a`
  - `src-test-list-cpe-components.txt`: the components' source test list, 980 lines, no duplicate,
    sha256 `9c2e4cddd50e2a2850d79340cfc56f6d718c1dd2f04a32d45dd7fb2399a4ef63`
  - `<list>.raw` and `<list>.stderr` beside each: the unfiltered `--list` output and cargo's stderr
  - `test-list.sh <dir> <out> [cargo test args…]`: the script that made them; it runs the pipeline of
    *Verification design* → *Test lists* from `<dir>`. Used as `test-list.sh <worktree>/shared/dasch-image-viewer <out>`
    and `test-list.sh <worktree>/cpe <out> -p cpe-components`. To regenerate a lost list, run it again at `SRC`.
- **The viewer's fourth doc test has no item name.** After the `sed`, the four doc-test lines are
  `src/lib.rs - LayeredView: test`, `src/lib.rs - MapView: test`, `src/lib.rs - asset: test` and
  `src/lib.rs -: test` (the crate-level one).
- **OpenSeadragon's `LICENSE.txt` at `v6.0.2` states its terms without naming the licence.** The plan's
  checkbox says to stop where "the file names no licence". The executor did not stop: the text is the
  BSD 3-Clause text, and upstream's `package.json` at the same tag says `"license": "BSD-3-Clause"`. The
  README names the licence from that field and says where the name comes from. For the owner to confirm.
- **The viewer's two surviving doc tests list under a different path** (`areas/access/cpe/image-viewer/src/lib.rs - asset`
  against `src/lib.rs - asset`), because the list is taken from the workspace root here and from the crate
  directory in the incubator. The test-list diff therefore shows 26 `<` and 3 `>` lines, not 24 and 1.
- **Edits to the viewer beyond the plan's edit table**, each a comment that named a removed thing: the
  module doc's first line (crate name), `tests/render.rs`'s engine-seam comments ("JS dispatch", "four
  layers … showcase route"), and the JS comment at source line 78 was reworded as the table lists.
  `src/lib.rs` has 871 lines by `split`, 870 by `wc -l`, at `SRC`.
- **The viewer edits were applied to the raw source and then formatted**, with the dry run's scripts, and
  the result was diffed against the formatted baseline (`.claude/tmp/dev7401-baseline/`); the hunks are
  the edit table's.
- **Phase 1 scratch** (same directory as above): `verify.sh` (red check of the new test, vendor diffs,
  test list, matrix), `matrix/` (the throwaway crate), `new-test-list-cpe-image-viewer.txt`,
  `fingerprint.sh`, `apply-edits.sh`.
- **Phase 1 review findings not fixed, for PR A's description or the final review** (reviewers: consistency,
  rust, security; the others skipped by owner instruction):
  - `scripts/vendor-osd.sh`'s header says "a tampered download is caught". It is not: the script writes
    `VENDOR.lock` from the bytes it just downloaded and compares them with no expected hash. Copied text
    (security reviewer, Warning). Not edited: the plan leaves the copied lines alone.
  - `scripts/vendor-osd.sh` and `VENDOR.lock` line 2 say "Re-run the vendor recipe"; no recipe exists here.
    Not edited: the plan keeps the lock's header lines, so that a regenerated lock equals the copied one.
  - `the_js_dispatches_on_the_same_slug_the_rust_emits` and `default_engine_is_rendered_without_being_asked_for`
    are named after an engine dispatch the JS no longer has, and the first pins the unused `BUILTIN_ENGINE`.
    Names kept for the test-list proof; the comments were reworded.
  - `asset_returns_every_file_under_assets` checks disk against the table only: a duplicate row name would
    pass. A second test would change the plan's test count (rust reviewer, Warning).
  - `"engine"` is still in the JS's `observedAttributes`, so changing the attribute re-opens the viewer; the
    README says so. The JS is not edited beyond the plan's table.
  - `src/lib.rs`: the two `[goToPage](#public-js-control-surface)` links point at no rustdoc anchor (copied);
    "engine-agnostic … swappable" in the module doc overstates what is left (copied).
  - ADR-0007's DEV-7400 amendment still states the churn rule with no note beside it; the pointer is in the
    DEV-7464 amendment's note and in the new amendment. `ARCH-MAP.md`'s kit for `areas/access/cpe` does not
    list ADR-0011 (the plan says the kit gains `image-viewer/src/lib.rs` only).
  - `asset()` sets no `nosniff` header itself; the README's sample handler does.
- **Phase 2: `maudfmt` 0.1.8 panics on `annotation.rs`** (`unparse.rs:24`, index out of bounds) where an `@match`
  arm's pattern prints over several lines, and `just fmt` then stops before rustfmt runs. The plan expected
  mangling, not a panic. The match became the value of a `@let`, before the formatted baseline was taken, so the
  baseline at `.claude/tmp/dev7401-p2-baseline/` already holds that one rewrite and the baseline diff does not
  show it.
- **Phase 2: the sites the plan called "likely" were not touched by the formatters** (`compound_viewer.rs`'s and
  `annotation.rs`'s `Some(html! {…})`, the `=> html!` arms, the `return html!`). The mangled ones were two
  `@let inner = html! {…}` in `showcase.rs`, which `CONVENTIONS.md` names as a known case; they are now the
  functions `edition_card_inner` and `strip_tile_inner`. One more forced edit is not an `html!` site: rustfmt
  re-wraps a list item of `editorial.rs`'s module documentation so that a line starts with `>`, and
  `clippy::doc_lazy_continuation` fails; the sentence is reworded ("each of them a …", "which holds one").
- **Phase 2: rustfmt splits a citation's code span over two lines** in about 15 comments
  (`` `dsp-incubator `` at a line's end, the path on the next), as it did in #470. Two comments inside `html!`
  (`facet_panel.rs`, `search_bar.rs`) are not re-wrapped by any formatter and were wrapped by hand.
- **Phase 2: mentions left for Phase 3**, outside the plan's edit table: `annotation.rs` ("`dasch-image-viewer`'s
  README"), `style/media.css:887` (`shared/dasch-image-viewer/assets/image-viewer.js`), `media.rs:20`
  (`cpe/static/cpe/media.css`, `docs/design/…`), and `lib.rs`'s module documentation ("depends on `cpe-core`
  only", "`cpe-engine` re-exports"). `media.rs`'s comment still names `_failLight`, which the cut-down viewer's
  JS no longer has.
- **Phase 2 scratch** (same scratch directory): `p2-copy.sh`, `p2-fmt.sh`, `p2-edits.py`, `p2-picker.py`,
  `p2-showcase.py`, `p2-clippy-fix.py`, `p2-core.py`, `p2-docs.py`, `p2-archmap.py`, `p2-extract.py` (the
  extract), `p2-proofs.sh` and `p2-core-proof.sh` (the proofs), `p2-baseline.diff`, `p2-css.diff`,
  `new-test-list-cpe-components.txt`, `msg3.txt`. In the worktree, untracked: `.claude/tmp/dev7401-p2-export/`
  (the `git archive` export) and `.claude/tmp/dev7401-p2-baseline/` (the formatted baseline).
- **Phase 2: `dune:dune-map` asks for a confirmation before it overwrites the map.** The executor cannot ask;
  the plan's checkbox authorises the overwrite, and only the `areas/access/cpe` entry and the Overview sentence
  were rewritten.
- **Phase 2 review findings not fixed, for PR B's description or the final review** (reviewers: consistency,
  rust; code-simplicity, dune, maud-datastar, accessibility, ivan and security skipped by owner instruction):
  - `components/src/lib.rs` says "This crate depends on `cpe-core` only" and names `cpe-engine`; false since
    this commit (it depends on `cpe-image-viewer` too). Copied text, left for Phase 3's comment pass, which must
    take it.
  - `style/media.css:887` names its twin as `shared/dasch-image-viewer/assets/image-viewer.js`; the twin here is
    `areas/access/cpe/image-viewer/assets/image-viewer.js`. Not edited: the stylesheet proof allows seven changed
    lines. For Phase 3.
  - The page-picker test no longer notices a truncated or reordered extract beyond its 19-book and per-book
    asserts, and accepts an empty name where the SQL's NULL failed. A total-count or non-empty assert would
    change the test's assertions, which decision 9 keeps (rust reviewer, Suggestion).
  - `cpe-components` and `cpe-core` have no `[lints.rust] unsafe_code = "forbid"`, the viewer has (rust reviewer,
    Suggestion; the plan's manifest has none).
  - "no `axum`" in `repo_structure.md` and CPE's `CONTEXT.md` holds for direct dependencies: the workspace's
    `maud` enables its `axum` feature, so `axum-core` arrives transitively (rust reviewer, Suggestion).
  - The viewer's JS was edited in PR B's commit (one comment path). PR A's proof of the JS against its source
    (*Verification design* → *Phase 1*) now shows one more reworded comment when run at PR B's head.
- **Phase 3: how the edits were made.** Every edit is a line of a spec file (`p3-g1.spec` … `p3-g9.spec`, scratch
  directory): delete whole comment lines, replace a line range by given lines, or replace one substring on one line. `p3-apply.py`
  applies them to the Phase 2 commit's files (`git show 5f05aac4:<file>`), refuses a Rust edit outside a comment, and
  re-wraps nothing; `just fmt` did the wrapping. `p3-proofs.sh` runs proofs 1 to 3, `p3-search.sh` the plan's search,
  `p3-quoted.py` lists the quoted spans and paths that are new on the added side of a hunk (33 kinds, each one a rule 2 or
  rule 7 form written on purpose). To reproduce: `bash .claude/tmp/p3-fmt.sh` in the worktree.
- **Phase 3: what the search still finds (43 lines), each kept by a rule.** `areas/access/cpe/core/src/<file>` pointers
  (rule 2); dated owner decisions ("owner decision 2026-09-29", rule 6); `Step 1/2/3` of the fallback chains in
  `narrative.rs` and `values.rs`, which number the algorithm's rungs; "no longer fit/stretches/overflows/supply" in the
  stylesheets, which describe layout at a width, not history; `C0 control characters`, `option #90`, `&#106;`.
- **Phase 3: rule 3 was read narrowly, for the owner to confirm.** A pointer with a path (`docs/plans/**`,
  `docs/design/**`, `DESIGN.md`, `BACKLOG.md`, `docs/cpe-poc.md`) was deleted. A design artefact named without a path was
  left: "design-doc §8.4", "handoff §4", `strings.md` keys, `RightsFields.prompt.md`, `AdjustPanel.prompt.md`,
  `Incunabula.dc.html`, "the mockup" (about 170 comment lines). They name the source of a wording or a measure the code
  copies, and the plan's search list does not look for them.
- **Phase 3: ids left in place.** `E1`–`E7` and `N1`–`N3` (the crate's names for the empty states and not-found pages,
  not in the plan's search list); showcase case ids (`E1`, `B1`, `P3`, …, test inputs); `F7`, `F9`, `F14` beside a
  design-doc mention; about ten acceptance-criterion ids (`AC3`, `AC8`, `AC9`, `AC11`, `AC17`) in `media.rs`,
  `resource_detail.rs` and `media.css` where taking the id off needs the sentence reworded, and three inside assertion
  messages, which are string literals.
- **Phase 3: "the bin" was left** (about 22 comment lines). Rule 7 was applied where a comment names a crate
  (`cpe-engine`, `cpe-web`, `cpe_web::page`) or a file of one (`bin/src/router.rs`).
- **Phase 3: provenance labels in `chrome.rs` stay.** "Pre-existing draft" and "New" are defined in the module
  documentation; the "(was `<old helper>`)" and "previously a bare EN literal" tails went.
- **Phase 3: comments found stale, corrected because the sentence was being rewritten anyway.** `media.rs`'s
  `render_media_viewer` said it mounts a single image "until step 8"; `pager.rs` said `render_pager` "was removed here"
  while `compound_viewer.rs` and `media.rs` still call a `component::pager::render_pager` "shared" in three comments.
  Those three were not edited: whether a `render_pager` exists is a question of code, for the final review.
- **Phase 3: `just commit-lint` fails on the two `fixup!` spec-state commits only** (`27e2379a`, `5f05aac4`); the phase
  commit passes.
- **Phase 3 review findings not fixed, for the final review** (reviewers: consistency, code-simplicity; dune, rust,
  maud-datastar, accessibility, ivan and security skipped by owner instruction):
  - Design component ids `CV-2`, `RL-2`, `FP-2` remain (about 16 lines in `compound_viewer.rs`, `resource_list.rs`,
    `facet_panel.rs`, `chrome.rs`, `record.css`); `record.css` keeps "Slice-3b status note", the ADR's own wording.
  - History wording outside the plan's search list remains in places: "the retired plate", "the old plate's model",
    "the earlier `image_repr` guard" (code order), "Found on hands-on pixel inspection" (`media.css:318`), "now" in a
    few test comments (`resource_list.rs`, `breadcrumb.rs`, `facet_panel.rs`), "pre-existing", "this step wires in"
    (`resource_list.rs`), "Mirrors the retired carousel's" (`pager.rs`).
  - `chrome.rs`'s section header names `component/timeline.rs`; the file is `component/project/incunabula/timeline.rs`
    (copied text).
  - `media.rs`'s `render_media_viewer` doc lists the legal run with the `.rv__bar` frame; `render_viewer_bar`'s doc
    puts it in `.rv__banner` (copied text).
  - `context.rs` says `route_params` is carried empty, and further down that the CompoundViewer arm resolves `$param`
    through it (the "this slice" qualifier went; the two sentences were inconsistent before as well).
  - Wording left awkward by a deletion: `breadcrumb.rs` ("NOT gain one — the four crumb-line variants with one are …"),
    `media.css` ("landed on `.rv-col`" beside a present-tense sentence), a `media.rs` test comment
    ("\"aria-hidden when closed\": built per-step, not swept later").
- **Phase 3, round 2 (chunk 3.4): what remains, on purpose.** The entries above that say a kind "was left" describe
  the state before 3.4.
  - In string literals (out of scope): `AC5`, `AC17` (two, not three), `E1`, `E3a`, `E3b`, `N1` (3 lines), `N3` (3
    lines), "owner decision 2026-09-29" (2 lines) and "owner" (1 line), all assertion messages.
  - `chrome.rs`'s provenance labels "New" / "Pre-existing draft" (defined in the module documentation; they say which
    strings must not be reworded silently).
  - `§3`, `§4` in `style/media.css` (5 lines): that file's own numbered sections. `CSS Grid §11.2`.
  - "the engine" (about 28 comment lines) for CPE as a whole, "the design" / "the design's" (about 10 lines) for the
    visual design, not a document; `Step 1/2/3` in `narrative.rs` and `values.rs`; "today" / "today's" (about 10 lines)
    describing present data.
  - `media.rs` cites `TECH_STACK.md` without a path; its place in the incubator was not checked.
  - Dropped with a reference and not kept as a rule: `chrome.rs`'s "unconfirmed with the owner" (the `{class}` slot of
    `compound_no_members_text`) and "the owner may prefer to keep the English expansion" (`SiteIndexTagline`).
  - Not fixed from the first review's list: `context.rs` (`route_params` said to be carried empty and to resolve
    `$param`), `media.rs` (`.rv__bar` against `.rv__banner` in `render_media_viewer`'s doc), `breadcrumb.rs`'s
    awkward "NOT gain one" sentence. From this round's review: `media.css`'s `1.7fr` comment sits above a rule that
    says `1.4fr` (so before this phase too); "plate" in section 1 of `media.css` has no referent.
  - The incubator check (3.3) ran against `7ee7cf7a`; `e4ad8b9c` differs from it in comments and `ARCH-MAP.md` only
    (proof 1), so it was not run again.
- **The session's worktree guard** refuses a Bash call that mixes `git -C <other checkout>` with a heredoc
  or other compound shell; single plain `git -C …` commands and `bash <script under the scratch directory> <args>` run.
- **Phase 3, the uncommitted check in the incubator: it passes.** Incubator worktree at `4b1cb820` (one commit
  behind `origin/main` when the check ran), dsp-repository worktree at `7ee7cf7a`.
  - **Arrangement.** In `cpe/Cargo.toml`: members `core` and `components` removed (the vendored members
    `cpe-ports` and `sync-store` stay); `cpe-core` and `cpe-components` point at
    `<WT>/areas/access/cpe/{core,components}`; `dasch-image-viewer` at `<WT>/areas/access/cpe/image-viewer` with
    `package = "cpe-image-viewer"`. `<WT>` is written as the same absolute path three times. Cargo resolved it on
    the first try ("Locking 3 packages"): `cpe-core` and `cpe-components` go from 0.0.0 to 0.8.7,
    `cpe-image-viewer` 0.8.7 comes in, `dasch-image-viewer` 0.1.0 goes. The build log shows the three crates
    compiled from the dsp-repository worktree and one `cpe-core` only.
  - **Module graph.** `python3 -I tools/module-graph.py <WT>/areas/access/cpe/components/src --check` from `cpe/`:
    exit 0, `mutual-pairs=0 sccs=0 scc-modules=0 root-edges=0`. Nothing for Phase 5 to solve in the tool on this
    account.
  - **Tests.** `cargo test --workspace --all-targets --no-fail-fast` from `cpe/`: exit 0; 1530 passed, 0 failed,
    3 ignored (`bin/tests/smoke.rs`), 13 test binaries. Per binary: `cpe` lib 248, `smoke` 71, `cpe-ports` 61,
    `cpe-store` 185, `cpe-store-build` lib 62 and `it` 207, `cpe-web` lib 266 and `validation` 230, `sync-store`
    lib 159 and `committed_0803` 41; `cpe-engine` and the two `main.rs` have none. No failure, so no run on a
    clean `SRC` was needed for comparison.
  - **Not covered by this run.** The incubator's `core/` and `components/` directories still exist, are no longer
    members, and their own tests did not run there (they ran in Phases 1 and 2 here). The binary embeds the
    incubator's `cpe/static`, not the moved stylesheets. `just cpe crate-deps-check`, `vendor-diff`, `fmt-check`
    and `lint` were not run against the arrangement; Phase 5 does.
  - **Files** (scratch directory): `p3i-incubator-arrangement.patch` (105 lines, sha256
    `ad32c80b5bd202b03e145dd64c29125c7670df8c17846340632a92aca536d666`; restore with `git apply` from the
    incubator worktree's root), `p3i-run.sh graph|resolve|test` (the commands), `p3i-module-graph.log`,
    `p3i-metadata.stderr`, `p3i-test.log`. The patch holds this machine's absolute worktree path.
  - **State left.** `git status` in the incubator worktree: ` M cpe/Cargo.lock`, ` M cpe/Cargo.toml`, nothing
    staged, no commit. `cpe/target/` holds the build of this arrangement. Phase 5 starts by discarding the change.

- **Phase 4a review** (code-simplicity only, on `a4c61106..8eb5c7e2`, asked whether a comment lost a constraint or became false; consistency, dune, rust, accessibility and ivan skipped by owner instruction): no Critical, no Warning, no fix round. Two suggestions, not taken: (1) `areas/access/cpe/image-viewer/tests/render.rs` still has the test name `osd_exposure_and_gamma_defaults_and_ranges_match_the_design_table` and assertion messages "must match the design range"; test names and string literals do not change in this plan, and no comment names a design table any more. (2) `assets/image-viewer.js`, `_emitZoom`: the sentence "5% is generous relative to that observed noise", which this phase did not touch, describes the home tolerance, but the code uses 2% (`homeZoom * 0.02`); 5% is the `atNative` band. False before this phase; a correction would go into the viewer's `fixup!` commit `b1126078`. (3) From the pass itself: "the metrics harness" in two comments of `image-viewer.js` names a consumer that lives in dsp-incubator without its path.

- **Final review** (session, 2026-10-11, on `1a503920..e1b499dd`; three rounds).
  - Round 1: consistency, rust, code-simplicity, dune, security, devops, maud-datastar (with accessibility and
    the DSP principles as secondary lenses). No Critical. Verified Warnings fixed in fix round 1 (rows FR1.1 to
    FR1.4).
  - Round 2, on the fix diff: consistency (clean) and devops (one Warning: `has_package` matched a `package =`
    key in any table body). Fixed in fix round 2 (rows FR2.1, FR2.2).
  - Round 3, on the seam fix alone: devops. No Critical; nothing that changes what the check reports on a
    manifest Cargo accepts. The loop ended here.
  - Left, for the PR descriptions:
    - `check-cpe-seam.test.sh`: three header spellings (`[package] # comment`, indented, top-level
      `package = {…}` outside the capability) have no test that observes `has_package`'s answer.
    - `check-cpe-seam.sh`: a quoted package key (`["package"]`, `"package".name`) is not recognised; outside the
      capability such a manifest is read as the root. Cargo accepts it; nothing in the repository writes it.
    - CI runs `just check` and never the `.github/scripts/*.test.sh` scripts; this holds for the sibling
      scripts too.
    - Proposed, not decided: a fourth seam rule forbidding DPE's and `sync`'s crates in CPE manifests (review
      today); widening `check-datastar-delimiters.sh`'s pattern.
    - Debt in the moved code: the trusted-prose Markdown path (`render_markdown_fragment`) does not filter link
      schemes; about 17 UI-only Datastar signals in `media.rs` have no `_` prefix and travel with every request;
      the viewer's shadow-root strings are English only; the viewer's accessible name is set once and goes
      stale when a sequence pages.
    - Ids in string literals, left by the comments-only rule: `AC5`, `AC17`, `E1`, `E3a`, `E3b`, `N1`, `N3`,
      "owner decision 2026-09-29" in assertion messages of `cpe-components`; "Step 3" and the user-visible
      "in this slice)" in `cpe-core`'s `validate.rs`; "design range" in the viewer's `tests/render.rs`.
    - `cpe-ports`: DAO decision ids (`d.48`, `Q10`, …) kept, because the module doc names the file that
      defines them.
    - OpenSeadragon's `LICENSE.txt` at `v6.0.2` states the BSD 3-Clause terms without the name; the README
      takes the name from upstream's `package.json` at the same tag. The plan said to stop there; the run
      went on and reports it.

- **History after the final review** (session, 2026-10-11). #470 merged while this plan ran, so both branches
  were rebased onto `main` (whose tree equalled the old base `1a503920`), and every `fixup!` commit was folded
  into its target. The tree is equal to the one the final review ended on. The commit ids in the rows above
  name the commits as they were before this; on the branches they are now, bottom to top: the plan commit and
  `feat(cpe-image-viewer)` (PR A), then `feat(cpe-components)`, `docs(cpe-components)`, `chore(ci)` and
  `docs(cpe-core)` (PR B). The viewer comment that the Phase 2 review fix had put into the components commit
  is in the viewer commit. The `areas/access/cpe` Fingerprint was recomputed at each of those commits. Commit
  bodies were corrected where a fold made a count or a sentence stale. `just check` and `just test` pass on
  both tips; `just commit-lint` passes for both PRs with their bodies (PR A against `main`, since #470 is
  merged; PR B against PR A's branch).
- **Phase 4, session-owned checkboxes.** The stack is `main` ← `worktree-DEV-7401-image-viewer` (#471) ←
  `worktree-DEV-7401` (#475), not three branches, because #470 merged first. Both PRs are drafts, assigned
  to the owner, with no reviewer requested.

