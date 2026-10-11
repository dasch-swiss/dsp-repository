---
title: "feat: move CPE's domain crate (cpe-core) into areas/access/cpe"
type: feat
date: 2026-10-11
author: "Balduin Landolt"
status: implemented
linear: DEV-7464
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7464
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator/.claude/worktrees/DEV-7464
---

# feat: move CPE's domain crate (cpe-core) into areas/access/cpe

## Overview

`cpe-core` is CPE's framework-free domain crate: the KDL config language and its validation, the IR,
diagnostics, labels, language, theme, the per-request view data and the helpers they share. It lives in
`dsp-incubator/cpe/core` (28 Rust files, 32,404 lines, 552 tests, at incubator `1cd5781f`). This plan moves
it to `areas/access/cpe/core` in this repository, brings its comments in line with `CONVENTIONS.md`, and
then has the incubator build against a vendored copy of it, as it already does for `cpe-ports` and
`sync-store`.

There is no PRD. The requirements are Linear DEV-7464 (its note of 2026-10-10 wins over the original
description) and the owner's decisions of 2026-10-11 recorded below.

## Problem Statement / Motivation

ADR-0007's amendment of 2026-10-02 (DEV-7400) moves CPE into this repository crate by crate. `cpe-ports`
was written here; `cpe-core` is the first crate to move in. It depends on no other CPE crate, so nothing
has to move before it, and two tickets wait for it: DEV-7401 (the component library, which shares
`pulldown-cmark` types with `cpe-core`) and DEV-7465 (store, page assembly and the wiring in
`access-server`).

**This plan reverses one sentence of that amendment for this crate.** The amendment says: "A crate moves
when its churn in the incubator has dropped, not in dependency order". The incubator's go-live plan gives
the reason (`dsp-incubator cpe/docs/INCUNABULA_GO_LIVE.md`, "Moving"): "moving a still-busy crate turns
every prototype change into a cross-repo change".

`cpe-core`'s churn has not dropped:

- 30-day churn on its paths (`cpe/core` and, before the split of 2026-10-10, `cpe/engine/src`), measured
  2026-10-11 on incubator `origin/main`: 23 commits, 12 of them in October.
- dsp-incubator#502 (`1cd5781f`, 2026-10-11) added `remodel.rs` and `validate/remodel/*`, about 5,400
  lines.
- The incubator's `BACKLOG.md` holds about eight open items that touch the crate, and the go-live plan
  expects the next projects to "keep changing" the config language.

The owner decided on 2026-10-11 to move it now. The reason behind the old rule still holds, and its cost
is accepted: from the move on, every change to the config language is a PR here, followed by a pin move in
the incubator. What this plan can state as grounds, for the owner to correct: the evidence above shows no
quiet period to wait for, and waiting keeps DEV-7401 and DEV-7465 blocked. The rule itself stays for the
crates that follow. ADR-0007 gets a dated amendment that records the exception.

## Proposed Solution

Three phases, two repositories, two PRs.

1. **dsp-repository, Phase 1.** Copy `cpe/core` from the incubator at a pinned commit to
   `areas/access/cpe/core`, with the smallest set of edits this repository's gates require, and register
   the crate everywhere a crate is registered.
2. **dsp-repository, Phase 2.** Remove history and plan references from the crate's comments, as a
   commit of its own that changes comments only. Phases 1 and 2 are one PR.
3. **dsp-incubator, Phase 3.** After the dsp-repository PR has merged: vendor `areas/access/cpe/core` at
   the merged commit, delete `cpe/core`, re-home the three tests the move drops, and repoint the tooling
   and the prose that name `core/src`.

### Decisions (owner, 2026-10-11)

1. **Move now; no quiet condition for this crate.** See *Problem Statement*.
2. **Not stacked on #469.** #469 (DEV-7496) is documentation for `areas/access/sync`. It shares only
   `ARCH-MAP.md` with this work, in another component's entry. This branch is cut from `main`.
3. **The three tests that read incubator files leave the crate and are re-homed in the incubator.**
   `cpe-core` has two tests that `include_str!` `cpe/docs/CONFIG_LANGUAGE.md` and
   `cpe/projects/incunabula/config.kdl`, and one that reads `cpe/static/cpe/tokens.css`. None of those
   files moves. Phase 1 deletes the tests; Phase 3 writes them again in incubator crates that stay there.
   They need public API only (`cpe_core::validate::load_remodel`, `cpe_core::remodel::Remodel`'s
   `PartialEq`, `cpe_core::markup_contract::HEADER_DESK_BREAKPOINT_PX`).
4. **The config-language specification and the CPE glossary stay in the incubator until DEV-7465.**
   `cpe/docs/CONFIG_LANGUAGE.md` and `cpe/CONTEXT.md` are not moved or copied. `areas/access/cpe/CONTEXT.md`
   gains a pointer to them. A config-language change therefore lands in two steps: the code here, then
   the document in the incubator's pin-move PR.
5. **Comments are cleaned on entry; test names are not.** Phase 2 removes history and plan references
   (*Comment rules* below). The 549 remaining tests keep their sentence names (`the_worked_example_…`),
   which do not follow `test_{what}_{condition}_{expected}`. Renaming them would break the incubator
   documents that cite them and would remove the one cheap proof that the move lost no test. This is
   recorded debt.

### Decisions (this plan; the owner may overrule at review)

6. **Commit types.** Phase 1 is `feat(cpe-core)`, as the commit that added `cpe-ports` was `feat`: the
   workspace gains a crate. Phase 2 is `docs(cpe-core)`: it changes comments only. With the plan commit
   that is three commits, so the PR ticks `allow-many-commits`. The two code commits are kept apart
   because each has its own proof (*Verification design*), and one squashed commit would have neither.
7. **Incubator ADR numbers are cited by file name.** `check-adr-refs.sh` resolves every bare `ADR-NNNN`
   in the tree against `docs/adr/`, string literals included. `cpe-core` cites the incubator's ADR-0001,
   0002, 0011, 0012 and 0014 at 30 sites: three of those numbers do not exist here and fail the gate, and
   two resolve to an unrelated record, which the gate cannot see. ADR-0007 already says the prototype's
   records "are cited by file name, never by number". Five of the sites are the messages and asserts of
   decision 8. Each of the other 25 becomes `dsp-incubator cpe/docs/adr/<file name>`.
8. **Two diagnostics lose "(ADR-0012)".** `CompoundMediaOwnership` and `MultipleImageViewers`
   (`validate.rs:4864` and `:4897` in the source) print the number in their message, and two tests assert
   `msg.contains("ADR-0012")`, one with a failure string that names it too (`validate.rs:12713`). The
   parenthetical is removed and the tests assert on the instruction the message keeps
   (`drop 'media' from the resource-detail`). The second message then ends "one image viewer per page:
   drop 'media' from the resource-detail". This is the move's only change to what the crate prints. Both
   are boot diagnostics of a config error; no rendered page contains them.
9. **No workspace-level lint, format or licence exception.** The crate is formatted with this
   repository's rustfmt configuration and must pass `just check` as it is.
10. **The ADR-0007 change is an amendment, not a new record.** It makes an exception for one crate and
    leaves the rule standing, which narrows the decision (ADR-0006). If the owner drops the rule for the
    remaining crates as well, that is a changed decision and a new ADR.

### Crate manifest and workspace entries

`areas/access/cpe/core/Cargo.toml`, following `areas/access/cpe/ports/Cargo.toml`:

```toml
[package]
name = "cpe-core"
version.workspace = true
edition = "2021"
publish = false

[dependencies]
kdl = { workspace = true }
percent-encoding = { workspace = true }
pulldown-cmark = { workspace = true }
thiserror = { workspace = true }
```

Root `Cargo.toml`: `"areas/access/cpe/core"` in `members`, before `"areas/access/cpe/ports"`, and in
`[workspace.dependencies]` the incubator's three lines, features included, each with its reason as the
other pins have:

```toml
# cpe-core's config language. `span` is required: diagnostics carry source positions. Held at 6.5:
# kdl 6.6 and later need Rust 1.95 and turn `serde` on by default.
kdl = { version = "~6.5", features = ["span"] }
percent-encoding = "2"
# cpe-core renders authored Markdown. Its types cross into CPE's component library, so both
# crates take this one entry (DEV-7401). Without default features: they add only the CLI's `getopts`.
pulldown-cmark = { version = "0.13", default-features = false, features = ["html"] }
```

`thiserror = "2.0.12"` is already there and equals the incubator's line. The three dependency lines must
stay byte-equal to the incubator's `cpe/Cargo.toml` lines (comments apart): `cpe/vendor/README.md` ("What
an empty `vendor-diff` does not cover") requires every key a vendored manifest inherits to be the
monorepo's at the pin.

This workspace's lock file has never held `kdl`, `miette`, `pulldown-cmark`, `pulldown-cmark-escape` or
`unicode-width`, so the first build resolves them fresh. The crate's tests are proven against the
incubator's lock: `kdl` 6.5.0, `miette` 7.6.0, `pulldown-cmark` 0.13.4. `~6.5` admits only 6.5.0; the
other two may resolve newer and are then held at the incubator's version with `cargo update --precise`.
`winnow` enters a second time (0.6 beside 0.7), which `deny.toml` reports as a warning.

### Edits Phase 1 makes to the copied source

| Edit | Where (source line numbers at `1cd5781f`) |
|------|-------------------------------------------|
| Delete `worked_example` and the two tests that use it | `src/validate.rs:8061-8108` |
| Delete `tokens_match_the_budget_formula` | `src/validate/header_budget.rs:194-213` |
| Reword the doc comment that names the deleted test | `src/validate/header_budget.rs:18-22` |
| Remove "(ADR-0012)" from two messages; change two asserts and one failure string | `src/validate.rs:4864`, `:4897`, `:12643`, `:12712-12713` |
| Rewrite the other 25 bare incubator ADR numbers to file-name citations | `diagnostic.rs` (3), `ir.rs` (8), `validate.rs` (13), `validate/project_components.rs` (1) |

The manifest is replaced and the source is formatted before these edits, so that neither shows in the
proof (*Verification design*).

Incubator ADR file names: `0001-crate-layout.md`, `0002-deployment-topology.md`,
`0011-per-project-components.md`, `0012-one-member-navigator-declared-layout.md`,
`0014-annotations-are-a-first-class-engine-store.md`.

### Comment rules (Phase 2)

The scope is history and references: the *Delete* rule of `CONVENTIONS.md` → *Comments*. It is not a
general trim. Rules are applied in this order; the first that fits a sentence decides.

1. **An ADR citation stays**, in the form Phase 1 gave it, even when the ids beside it go.
2. **A pointer to a document that defines a rule the code enforces stays**, written from the incubator's
   root: `dsp-incubator cpe/docs/CONFIG_LANGUAGE.md`, `dsp-incubator cpe/docs/SCHEMA.md`, and
   `dsp-incubator cpe/static/cpe/<file>` where the code mirrors a CSS value. The path is the one the
   file has in the source tree at `SRC`.
3. **A pointer to a plan, a journal, a backlog item or a design note is deleted** (`docs/cpe-poc.md`,
   `DESIGN.md`, `docs/design/**`, `BACKLOG.md`, `docs/plans/**`).
4. **A parenthetical made only of a project name, slice, step or decision ids is deleted whole**:
   `(Incunabula Slice 8, decision 9)`, `(HdM Slice 3, decision 13)`, `(search-bar plan step 5b, #178
   reconciliation)`, `D2`, `G11`, `R8`, `O6`, `round-1 review Critical C1`.
5. **A paragraph that only narrates who called, removed, reviewed or rejected something is deleted
   whole**, the backstory of an attribute included.
6. **History that justifies a present invariant is rewritten in the present tense**, not deleted: "an id
   was previously interpolated unescaped" becomes a statement of what the allow-list prevents.
7. **"Phase 2" as a name for validation is replaced** by "validation (the `validate` module)". Where
   that leaves a sentence in the future tense ("Phase 2 adds …"), it is rewritten as a present-tense
   statement of the boundary between parsing and validation.
8. **A crate that stays in the incubator is named by its path**, `dsp-incubator cpe/engine` for
   `cpe-engine`, likewise `cpe/web`, `cpe/store` and the binary. A pointer to this crate's own files is
   an item path or `src/<file>`, never `core/src/<file>`.
9. **A project name stays where the prose uses it as an example or it identifies a test's input.**
10. **Nothing else changes.** No code, no string literal, no test name. A comment that is long, or that
    restates the code, stays: trimming those is not this plan.

One doc comment is reworded although it states a contract, because the contract names the wrong home:
`src/lib.rs`'s crate documentation says the host's surface is what `cpe-engine` re-exports. It is
reworded to say that `pub` in this crate means visible to CPE's other crates, and that until DEV-7465 the
host's surface is what `dsp-incubator cpe/engine` re-exports.

The source holds 6,412 comment lines. A rough search for slice, step, phase and decision ids finds
about 250 of them. It holds no `TODO` and no `FIXME`.

### Verification design

Each code commit has a proof a reviewer can re-run after the fact.

- **The test list.** `cargo test -p cpe-core --lib -- --list --format terse | grep ': test$' | LC_ALL=C sort`.
  The `grep` drops libtest's summary line. A name is a full module path (`validate::tests::…`). The crate
  has no integration test and no doc test.
- **Phase 1 lost no test and changed none.** The test list here equals the test list of the incubator at
  `SRC`, less exactly three names:
  `validate::tests::the_worked_example_of_the_config_language_lowers_to_incunabulas_block`,
  `validate::tests::the_worked_example_differs_from_incunabulas_block_when_one_word_changes` and
  `validate::header_budget::tests::tokens_match_the_budget_formula`. All of them pass.
- **Phase 1 edited only what the table lists.** The *formatted baseline* is the incubator's `cpe/core/src`
  at `SRC`, formatted by this repository's `just fmt` and nothing else. `diff -r` between the formatted
  baseline and `areas/access/cpe/core/src` at the Phase 1 commit shows the table's edits, rustfmt's
  re-wrap of the comment paragraphs that hold a rewritten citation, and any code line rustfmt re-joined
  after a message got shorter. Every other hunk is a failure. A reviewer rebuilds the baseline on a
  scratch branch cut from the Phase 1 commit: replace `areas/access/cpe/core/src` with the export's
  `src`, stage it, and run `just fmt`. The manifest and the workspace entries are already there.
- **Phase 2 changed comments only.** Two checks between the Phase 1 commit and the Phase 2 commit, over
  every file under `areas/access/cpe/core/src`:
  1. The two versions are equal once comments are removed by one normaliser:
     `grep -v '^[[:space:]]*//' | sed 's,[[:space:]]*//.*$,,' | cat -s`. It drops whole-line comments, cuts
     trailing ones and squeezes blank lines. A difference it prints is rustfmt re-joining a line after a
     trailing comment went, or a real code change; each is read.
  2. The normaliser also cuts after `//` inside a string literal (`"https://"`, `src/util.rs`). So in
     `git diff -U0`, each added or removed line that is not a whole-line comment and contains a `"` is
     read: its removed and added forms are equal once the trailing comment is cut, or it is rustfmt
     re-joining a line.

  The source has no block comment and no `#[doc = …]`, which neither check would see. The test list is
  unchanged.

## Alternative Approaches Considered

- **Wait until the crate is quiet.** What ADR-0007's amendment says. Rejected by the owner (decision 1).
- **Move `CONFIG_LANGUAGE.md` with the crate.** Keeps the specification beside the code it specifies and
  lets the worked-example test stay in the crate. Rejected for now: the comparison against Incunabula's
  `config.kdl` has to live in the incubator either way, the incubator would have to vendor a document
  through `vendor/DATA`, and where CPE's documentation lives is DEV-7465's question.
- **One commit for the move and the comments.** What the one-commit rule asks by default. Rejected: a
  reviewer could then check neither that the copy is faithful nor that the clean-up touched comments only.
- **A first commit that is the verbatim copy.** Would make the copy checkable byte for byte. Rejected:
  that commit fails `just check` (formatting, ADR references) and its tests do not compile (the
  `include_str!` paths), and every commit lands on `main` verbatim. The formatted baseline gives the same
  proof without the commit.
- **Import the crate's history** with `git filter-repo` and a merge of unrelated histories. Rejected: this
  repository rebase-merges, so the merge commit cannot land, and the incubator's commits pass neither
  commit-lint nor the one-commit cap. The source sha in the commit body is the link to the history.
- **Rename the tests too.** Rejected, decision 5.

## Technical Considerations

**The incubator worktree.** The path in `repositories:` does not exist until the first checkbox of
Phase 1 creates it. The owner's main checkout of the incubator was two commits behind `origin/main` on
2026-10-11; nothing in this plan reads it.

**The source commit.** `SRC` is the sha the incubator worktree is created at: the incubator's
`origin/main` when Phase 1 starts. On 2026-10-11 that is `1cd5781f987ed860218462c086c77ce759185fcc`, and
every source line number in this plan refers to it. If `cpe/core` has changed since, the line numbers are
found again by content and the counts in *Success Metrics* are taken again. `SRC` goes into the Phase 1
commit body as a full sha: the copy carries no history, and the sha is the link to it.

**Drift between the copy and the re-vendor.** Nothing freezes `cpe/core` in the incubator between `SRC`
and Phase 3. A change that lands there in between exists only in the incubator. Phase 3 checks for it
first and stops if it finds one: the change has to be made here, as its own PR, before the incubator can
delete its copy. H3 asks the owner to keep that window empty.

**Formatting.** The incubator formats with stable rustfmt defaults (width 100). This repository uses
nightly-only options (`.rustfmt.toml`: width 120, `wrap_comments`, `comment_width = 100`,
`imports_granularity = "Module"`, `group_imports`). The source's comments are already wrapped at 100, so
the visible change is code lines joining up to 120 columns and `use` trees being regrouped. The incubator
never formats vendored crates (`cpe/tools/own-crates.sh`), so the formatted text is what it vendors.
`maudfmt` leaves the crate alone: it has no `html!`. Two nightly rustfmt builds can wrap comments
differently (`docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`): if
CI's format check disagrees with the local one, compare `cargo +nightly fmt --version` with the `rustc`
line of the CI log before touching any comment by hand.

**Lints and toolchain.** Both repositories pin Rust 1.93.0 and neither has a `[workspace.lints]` table.
The incubator already runs clippy on this crate with `--all-targets --all-features -- -D warnings`, which
is stricter than `just check` (no `--all-targets`). The crate has no `unsafe`, no inner attributes, no
`unwrap()` outside tests and no features. `cargo machete` passes: all four dependencies are used.
Neither workspace declares a `rust-version`, so resolver 3 here and resolver 2 there resolve alike; only
the two lock files differ.

**Checks `just check` and `just test` do not run.** `just check` runs clippy without `--all-targets`, so
this plan runs it on the crate with that flag. CI builds the documentation on nightly
(`cargo doc --no-deps --all-features`) and only warns; the crate has intra-doc links, so this plan runs
`cargo doc -p cpe-core --no-deps`. CI also runs doc tests, of which the crate has none. `cargo deny` is
installed by `just install-requirements` and run by no recipe or CI job; this plan runs it once, because
the crate brings five packages this workspace has not had. Their licences (Apache-2.0, MIT) are on
`deny.toml`'s list according to crates.io; `cargo deny` confirms it.

**Gates that read the index.** `check-composition-root-deps.sh`, `check-adr-refs.sh` and the maudfmt loop
use `git ls-files` or `git grep`. New files must be staged before `just fmt` and `just check` see them.

**`check-composition-root-deps.sh`.** It forbids `access-server` any dependency on a crate at
`areas/<area>/<capability>/{core,web,api-*}`. From Phase 1 on, `cpe-core` is on that list. That is the
intended rule (ADR-0003), and no edit is needed.

**Framework-freedom is not gated here.** In the incubator, `tools/crate-deps.py` fails the build when
`cpe-core`'s manifest or sources name `maud`, `axum` or `rusqlite`. This repository has no such check.
After the move the rule is **review** here, and the incubator's `just cpe crate-deps-check` still catches
a violation at the next pin move. `ARCH-MAP.md` records both.

**Release.** `cpe-core` needs no release-please entry. It falls under the root package, and the `feat`
commit raises the workspace's minor version
(`docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`). No commit
carries a `Release-As` footer.

**`ARCH-MAP.md`.** The `areas/access/cpe` entry is `status: planned` with `Fingerprint: none`.
`dune:dune-map` decides whether the entry becomes active now that the component holds a second crate. Its
fingerprint covers `areas/access/cpe/**` and is computed from the index, so Phase 2's commit makes the
value Phase 1 wrote stale and writes it again. The entry's Local-context kit lists 13 files against a
default budget of seven; this plan adds `areas/access/cpe/core/src/lib.rs` and no other.

**Commits (dsp-repository).**

1. `docs(docs): add the plan for moving cpe-core into areas/access/cpe (DEV-7464)`
2. `feat(cpe-core): move CPE's domain crate in from dsp-incubator (DEV-7464)`
3. `docs(cpe-core): remove history and plan references from the comments (DEV-7464)`

A review finding is amended into the commit it belongs to. Where that is the Phase 1 commit after Phase 2
exists, the Phase 2 commit is rebased onto it, the proofs of *Verification design* and the Fingerprint are
run again, and the branch is pushed with `--force-with-lease`. A PR with several commits carries a
"Review Notes" section (`docs/src/git-conventions.md`). `just commit-lint` for more than one commit needs
the PR body: `PR_BODY="$(gh pr view <n> --repo dasch-swiss/dsp-repository --json body -q .body)"
just commit-lint`.

**The incubator PR is one squashed commit.** A pin move and the changes it forces are one PR there
(`cpe/vendor/README.md`), and the vendored tree, the deletion and the re-homed tests only build together.

**Commands.** `eng.yaml` resolves `just check` (lint), `just test` (test) and `just check && just test`
(gate) for every path this plan touches here. Under Nix they are prefixed with `nix develop --command`.
The incubator has no `eng.yaml`; its commands are `just cpe test` and `just cpe ci`, run from its
repository root. Its Cargo workspace is `cpe/`, so a bare `cargo` command there runs from `cpe/`.
`just cpe ci` needs network access. Never run `cargo fmt` in `cpe/`.

**Reviewers.** `eng:review:rust-reviewer`, `eng:review:dune-reviewer`, `eng:review:consistency-reviewer`
and `eng:review:code-simplicity-reviewer`. For Phase 1 the reviewers judge the edits and the
registrations, not the 32,000 copied lines: findings about the copied code's design are out of scope and
go to the PR description. For Phase 2 they read the comment diff for a deleted contract, which no
mechanical check finds.

## Implementation Phases

#### Phase 1: `cpe-core` enters `areas/access/cpe/core`

### dsp-incubator

- [x] Fetch, then create the worktree named in `repositories:` on the branch `worktree-DEV-7464` from
      `origin/main`; `SRC` is its `HEAD` sha
- [x] Save the test list (*Verification design*) of `cpe-core` at `SRC`, run from `cpe/`, under
      `cpe/scratch/` as the source test list

### dsp-repository

- [x] Confirm the branch is `worktree-DEV-7464`, `git fetch --prune`, and rebase on `origin/main`
- [x] Confirm this plan is committed on the branch as
      `docs(docs): add the plan for moving cpe-core into areas/access/cpe (DEV-7464)`; commit it where
      it is not
- [x] Where `git log 1cd5781f..SRC -- cpe/core` in the incubator is not empty, read those commits, find
      this plan's source line numbers again by content, and count files, lines and tests again
- [x] Export `cpe/core` at `SRC` with `git archive` into `.claude/tmp/` and copy it to
      `areas/access/cpe/core`
- [x] Replace `areas/access/cpe/core/Cargo.toml` with the manifest in *Crate manifest and workspace
      entries*
- [x] Add `"areas/access/cpe/core"` to the root `Cargo.toml`'s `members`, before
      `"areas/access/cpe/ports"`
- [x] Add `kdl`, `percent-encoding` and `pulldown-cmark` to `[workspace.dependencies]` as written in
      *Crate manifest and workspace entries*; the three dependency lines are byte-equal to the incubator's
      `cpe/Cargo.toml` at `SRC`
- [x] Stage the crate and the root `Cargo.toml`
- [x] Run `just fmt`
- [x] Copy `areas/access/cpe/core/src` to `.claude/tmp/` as the formatted baseline, before any edit
- [x] Delete `worked_example` and the two tests that call it from `src/validate.rs`
- [x] Delete `tokens_match_the_budget_formula` from `src/validate/header_budget.rs`
- [x] Reword the doc comment of `HEADER_DESK_BUDGET_PX` in that file so that it states the assumption
      without naming the deleted test
- [x] Remove "(ADR-0012)" from the messages of `CompoundMediaOwnership` and `MultipleImageViewers`, as
      decision 8 words them
- [x] Change the two tests that assert `msg.contains("ADR-0012")` to assert that the message contains
      `drop 'media' from the resource-detail`, and take "ADR-0012" out of their failure strings
- [x] Rewrite every bare incubator `ADR-NNNN` that is left in the crate to
      `dsp-incubator cpe/docs/adr/<file name>`, with the file names listed under *Edits Phase 1 makes*
- [x] Run `git grep -n 'ADR-[0-9]' -- areas/access/cpe/core` on the staged tree; it prints nothing
- [x] Run `just fmt`; then run it again, and the second run changes no file
- [x] Run `diff -r` between the formatted baseline and `areas/access/cpe/core/src`; it shows only the
      edits in the table under *Edits Phase 1 makes* and the rustfmt re-wraps *Verification design*
      allows around them
- [x] Run `cargo build -p cpe-core` and stage the `Cargo.lock` it writes
- [x] Compare the versions of `kdl`, `miette` and `pulldown-cmark` in `Cargo.lock` with the incubator's
      `cpe/Cargo.lock` at `SRC`; hold a differing one at the incubator's version with
      `cargo update -p <crate> --precise <version>`
- [x] Run `cargo test -p cpe-core`; it passes
- [x] Compare the test list with the source test list; the only difference is the three names in
      *Verification design*
- [x] Run `cargo doc -p cpe-core --no-deps`; it builds, and its only warnings are the 11
      `rustdoc::private_intra_doc_links` the source has at `SRC` (the remainder is the Phase 2 item
      "Turn the 11 intra-doc links …")
- [x] Run `cargo clippy -p cpe-core --all-targets --all-features -- -D warnings`; it prints no warning
- [x] Run `cargo deny check` with `deny.toml` unchanged, here and on `origin/main`; the move adds no
      advisory, ban, source or third-party licence error. It adds `cpe-core = 0.8.7 is unlicensed`, the
      error every workspace crate already has on `origin/main`, where `cargo deny check` reports 22
      errors (journal: *Deviations*)
- [x] Add `cpe-core` to the Crates row of the scope table in `CONVENTIONS.md`, after `cpe-ports`
- [x] Add `cpe-core` to the "Crate scopes" list in `docs/src/git-conventions.md`, after `cpe-ports`
- [x] In `docs/src/repo_structure.md`, add a `core/` line to the tree under `cpe/` and reword "only its
      port so far"
- [x] In `docs/src/repo_structure.md`, add a `cpe-core` row to the crate table that names its four
      dependencies and says it depends on no CPE crate and on no `maud`, `axum` or `rusqlite`
- [x] In `areas/access/cpe/CONTEXT.md`, rewrite the opening paragraph for a capability that holds its
      port and its domain crate
- [x] In `areas/access/cpe/CONTEXT.md`, add a short section that names what `cpe-core` holds and points to
      `dsp-incubator cpe/CONTEXT.md` and `dsp-incubator cpe/docs/CONFIG_LANGUAGE.md` as the glossary and
      the config-language specification until DEV-7465
- [x] Correct the sentence about CPE in `areas/access/CONTEXT.md` ("Only its port … is declared")
- [x] Read the Access Area bullet in the root `CONTEXT.md`; correct it where it says CPE is only a port
- [x] Add "Amendment (2026-10-11, DEV-7464) — `cpe-core` moved before its churn dropped" to ADR-0007:
      the sentence it makes an exception to and that sentence's reason, the churn measured that day, the
      owner's decision, that the rule stands for the crates that follow, the consequence for
      config-language changes, that `CONFIG_LANGUAGE.md` and the glossary stay in the incubator until
      DEV-7465, and the line "Enforced by: review"
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry and the Overview sentence about CPE with
      `dune:dune-map` (this checkbox authorises the overwrite). Done when Paths names `core/` and the
      crate `cpe-core`; Key entities name its config language, validation, IR and diagnostics; Depends on
      says `cpe-core` depends on `kdl`, `pulldown-cmark`, `percent-encoding` and `thiserror` and on no
      CPE crate; Boundary rules say that `cpe-core` naming no `maud`, `axum` or `rusqlite` is **review**
      here and is checked by the incubator's `just cpe crate-deps-check` at each pin move; the Local-context
      kit gained `areas/access/cpe/core/src/lib.rs` only; and the Fingerprint is the value computed over
      the staged tree
- [x] Grep the repository outside `docs/specs/` for `only its port`, `engine, store and routes are not
      built` and `Only \`ports/\` exists`; every remaining hit is true after the move
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as `feat(cpe-core): move CPE's domain crate in from dsp-incubator (DEV-7464)`, with `SRC` as
      a full sha in the body, the three deleted tests named, and no `Release-As` footer

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: the crate's comments follow `CONVENTIONS.md`

### dsp-repository

- [x] Record `git rev-parse HEAD` as the Phase 1 commit
- [x] Apply the *Comment rules* to `src/config.rs`, `src/ir.rs` and `src/remodel.rs`
- [x] Reword the crate documentation in `src/lib.rs` as the *Comment rules* section says
- [x] Apply the *Comment rules* to `src/diagnostic.rs`, `src/labels.rs`, `src/lang.rs`, `src/theme.rs`,
      `src/iiif.rs`, `src/util.rs`, `src/markup_contract.rs`, `src/narrative_outline.rs` and
      `src/view_data.rs`
- [x] Apply the *Comment rules* to the non-test part of `src/validate.rs`
- [x] Apply the *Comment rules* to the three test modules of `src/validate.rs`
- [x] Apply the *Comment rules* to every file under `src/validate/`
- [x] Turn the 11 intra-doc links to private items that `cargo doc -p cpe-core --no-deps` warns about
      (`iiif.rs`, `labels.rs`, `util.rs`, `validate.rs`) into plain code spans, or decide to keep the
      warnings; `cargo doc -p cpe-core --no-deps` then prints no warning (deferred from Phase 1: a
      comment-only change, which Phase 1's baseline proof does not allow and this phase's normaliser does)
- [x] Run `just fmt`
- [x] For each file under `areas/access/cpe/core/src`, `diff` the Phase 1 commit's version against the
      working file, both passed through the normaliser of *Verification design*; the diff is empty, or
      each line it prints is rustfmt re-joining a line whose trailing comment was removed
- [x] Run `git diff -U0 <Phase 1 commit> -- areas/access/cpe/core` and read each added or removed line
      that is not a whole-line comment and contains a `"`; its removed and added forms are equal once
      the trailing comment is cut, or it is rustfmt re-joining a line
- [x] Select the crate's comment lines (`rg -n '^\s*//' areas/access/cpe/core/src`) and search them for
      `Slice [0-9]`, `[Ss]tep [0-9]`, `Phase [0-9]`, `decision [0-9]`, `\b[CDGORS][0-9]{1,2}\b`,
      `\bplan\b`, `journal`, `review`, `BACKLOG`, `backlog`, `#[0-9]{2,4}\b`, `core/src/`, `cpe-engine`,
      `[Pp]reviously`, `[Oo]riginally`, `no longer`, `now uses` and `was changed`; every hit that remains
      is a comment one of the rules keeps
- [x] Compare the test list with the list of Phase 1; they are equal
- [x] Recompute the Fingerprint of `ARCH-MAP.md`'s `areas/access/cpe` entry over the staged tree and write
      it; the entry's other fields do not change
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as `docs(cpe-core): remove history and plan references from the comments (DEV-7464)`
- [x] Push the branch
- [x] Open a draft PR against `main`, assigned to the owner, from the repository's PR template, with
      `allow-many-commits` ticked, a "Review Notes" section that says what each commit proves, and the
      recorded debt of *Dependencies & Risks* listed
- [x] Run `just commit-lint` with `PR_BODY` set from the PR; it passes

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: the incubator builds against the vendored `cpe-core`

**Gate: H1** — resolve before starting this phase.

### dsp-incubator

- [x] Confirm the branch is `worktree-DEV-7464`, `git fetch --prune`, and rebase on `origin/main`
- [x] Run `git log SRC..origin/main -- cpe/core`. Where it prints a commit, stop and report it: that
      change has to land in dsp-repository before `cpe/core` can be deleted here
- [x] Set `<pin>` to the full sha of the commit on dsp-repository's `origin/main` whose subject is the
      Phase 2 commit's; confirm `<pin>:areas/access/cpe/core/Cargo.toml` exists and that the Phase 1
      commit is its ancestor
- [x] Save the names of every test in the workspace before any change (from `cpe/`:
      `cargo test --workspace --lib --tests -- --list --format terse | grep ': test$' | LC_ALL=C sort`)
      under `cpe/scratch/`
- [x] Add `areas/access/cpe/core` to `cpe/vendor/CRATES`
- [x] In `cpe/Cargo.toml`, replace the member `"core"` with
      `"vendor/dsp-repository/areas/access/cpe/core"` and the workspace dependency with
      `cpe-core = { path = "vendor/dsp-repository/areas/access/cpe/core" }`
- [x] `git rm -r cpe/core`
- [x] Run `just cpe vendor <pin>`; the file `cpe/vendor/PIN` holds `<pin>`
- [x] Compare the `kdl`, `percent-encoding`, `pulldown-cmark` and `thiserror` lines of `cpe/Cargo.toml`
      with dsp-repository's root `Cargo.toml` at `<pin>`; they are byte-equal, features included
- [x] Review what else the pin move changed under `cpe/vendor/dsp-repository/`; where `cpe-ports`,
      `sync-store` or `areas/access/sync/data` differ from the old pin, name the dsp-repository commits
      in the PR description
- [x] Re-home the two worked-example tests in `cpe/store-build/tests/it/`, calling
      `cpe_core::validate::load_remodel`, with their `worked_example` helper, the same names and the
      same assertions
- [x] Re-home `tokens_match_the_budget_formula` in a new file directly under `cpe/web/tests/`, reading
      `../../static/cpe/tokens.css` and `cpe_core::markup_contract::HEADER_DESK_BREAKPOINT_PX`, with the
      same name and the same assertions
- [x] Correct `cpe/CLAUDE.md` and `cpe/docs/adr/0005-css-baseline.md` where they say in which crate
      `tokens_match_the_budget_formula` lives
- [x] Repoint `cpe/store-build/src/generic/geometry.rs`'s `include_str!` of `core/src/validate.rs`, its
      failure message and its doc comment to the vendored path; the test still finds the
      `ANNOTATION_TOLERANCE` line
- [x] Delete `COPY cpe/core ./cpe/core` from `cpe/Dockerfile`
- [x] In `cpe/justfile`, repoint `watch`'s two `core` paths and the `core` entry of the `module-graph` and
      `module-graph-check` loops to the vendored path
- [x] Change `cpe/tools/crate-deps.py` so that its framework scan of the real workspace reads
      `vendor/dsp-repository/areas/access/cpe/core/src`, while a fixture root keeps its own `core/src`;
      no fixture file changes
- [x] Run `just cpe crate-deps-check`; it ends `core-framework-refs=0`
- [x] Run `just cpe module-graph-check`; it passes with `MODULES["core"]` unchanged
- [x] Run `git grep -n 'cpe/core'` over the whole incubator repository, outside `cpe/vendor/` and
      `cpe/docs/plans/`; repoint each hit, workflow path filters and root documents included
- [x] Search `cpe/` outside `cpe/vendor/`, `cpe/docs/plans/` and `cpe/tools/crate-deps-fixtures/` for
      `core/src`; repoint each hit to the vendored path (the CSS comments under `cpe/static/cpe/`,
      `cpe/projects/incunabula/DESIGN.md`, `cpe/web/src/page.rs`, `cpe/tools/dao-lift/src/ark.rs`,
      `cpe/CONTEXT.md`, `cpe/CLAUDE.md`)
- [x] Search `cpe/` outside `cpe/vendor/` for tests that expect "ADR-0012" in a diagnostic message; change
      them to the message the crate at `<pin>` prints
- [x] Add `cpe-core` to `cpe/vendor/README.md`: the four dependency keys it inherits, that
      `tools/module-graph.py`'s `MODULES["core"]` has to follow a module added upstream in the same
      pin-move PR, and that the three re-homed tests are this workspace's guards on a vendored crate
- [x] Tick DEV-7464 under "Moving" in `cpe/docs/INCUNABULA_GO_LIVE.md` and note that the crate moved
      before its churn dropped, by the owner's decision of 2026-10-11
- [x] Add a dated note to `cpe/docs/adr/0001-crate-layout.md`: `cpe-core` is vendored from dsp-repository,
      and `just cpe crate-deps-check` scans the vendored source
- [x] Add a dated note to
      `cpe/docs/adr/0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`: `cpe-core` is
      the third vendored crate, and a change to the config language is a dsp-repository PR followed by a
      pin move that carries the `CONFIG_LANGUAGE.md` edit
- [x] Correct `cpe/CLAUDE.md` where it says `cpe-core` is developed in this workspace
- [x] Correct `cpe/docs/ARCHITECTURE.md` in the same way
- [x] Add to `cpe/docs/CONFIG_LANGUAGE.md` that the language is implemented in dsp-repository's
      `areas/access/cpe/core` and that this document changes in the pin-move PR that brings the code
- [x] Compare the workspace's test names with the list saved before the change; they are equal apart
      from the module path of the three re-homed tests
- [x] Run `just cpe test`; it passes
- [x] Run `git status --porcelain --ignored cpe/vendor`; it lists no ignored file
- [x] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<pin>)`
- [x] Run `just cpe ci`; it passes
- [x] Commit `cpe/vendor/PIN`, the vendored tree, the deletion and every forced change together as
      `refactor(cpe): build against the vendored cpe-core (DEV-7464)`
- [x] Run `git diff --name-only origin/main..HEAD`; outside `cpe/` it lists only files the `cpe/core`
      search repointed, and under `cpe/projects/` only `cpe/projects/incunabula/DESIGN.md`
- [x] Open a draft PR with `gh pr create --repo dasch-swiss/dsp-incubator --head worktree-DEV-7464 --base
      main --draft --assignee @me`; never enable auto-merge

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge the dsp-repository PR for this plan | Balduin | before Phase 3 (Why mid-plan: the vendor pin must be a commit on `main`, which a rebase-merge mints only on merge, and the incubator cannot delete `cpe/core` before that commit exists) | Merging is the owner's decision |
| H2 | Merge the dsp-incubator PR | Balduin | after ship | Merging is the owner's decision |
| H3 | From the start of execution until H2: hold incubator PRs that change `cpe/core`, or land their change in dsp-repository first | Balduin | before start | Which PRs merge is the owner's decision |
| H4 | Decide whether the recorded debt (test names, long comments) becomes a Linear issue | Balduin | after ship | A backlog decision |

## Acceptance Criteria

### dsp-repository

- [x] `areas/access/cpe/core` is a workspace member named `cpe-core`, with the manifest of *Crate manifest
      and workspace entries*, and `Cargo.lock` holds `kdl`, `miette` and `pulldown-cmark` at the
      incubator's versions
- [x] `cpe-core`'s test list equals the incubator's at `SRC` less the three named tests, and all pass
- [x] The Phase 1 commit differs from the formatted baseline only by the edits of *Edits Phase 1 makes*
      and the rustfmt re-wraps around them
- [x] Between the Phase 1 and the Phase 2 commit, only comments changed under `areas/access/cpe/core`
- [x] No `ADR-` followed by a digit is left in the crate; `just check-adr-refs` passes
- [x] `CompoundMediaOwnership` and `MultipleImageViewers` print no ADR number
- [x] Every comment in the crate that names a slice, step, plan phase, decision id, backlog item or PR
      number is one the *Comment rules* keep
- [x] `cargo deny check`, with `deny.toml` unchanged, reports no error that `origin/main` does not, apart
      from `cpe-core` being unlicensed like every workspace crate
- [x] `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`,
      `areas/access/cpe/CONTEXT.md`, `areas/access/CONTEXT.md`, the root `CONTEXT.md` and `ARCH-MAP.md`
      describe a CPE capability that holds `cpe-ports` and `cpe-core`
- [x] ADR-0007 carries the amendment of 2026-10-11
- [x] `just check` and `just test` pass on each of the two code commits
- [x] The PR has three commits, `allow-many-commits` ticked, and `just commit-lint` passes

### dsp-incubator

- [x] `cpe/core` no longer exists; `cpe-core` is a workspace member at
      `vendor/dsp-repository/areas/access/cpe/core`
- [x] `just cpe vendor-diff` prints `vendor-diff: empty (<pin>)`, and `<pin>` is a commit on
      dsp-repository's `main`
- [x] The workspace's test names are the ones before the change, apart from the module path of the three
      re-homed tests, which exist and pass
- [x] No `cpe/core` or `core/src` reference is left outside `cpe/vendor/`, `cpe/docs/plans/`,
      `cpe/tools/crate-deps-fixtures/` and `cpe/tools/crate-deps.py`, where `core/src` is relative to the
      scanned root, that does not name the vendored path
- [x] `cpe/vendor/README.md`, `cpe/docs/INCUNABULA_GO_LIVE.md`, `cpe/docs/CONFIG_LANGUAGE.md`,
      `cpe/CLAUDE.md`, `cpe/docs/ARCHITECTURE.md` and the two ADR notes say that `cpe-core` is vendored
- [x] `just cpe ci` passes
- [x] No file under `cpe/projects/` changed except the path citations in `DESIGN.md`

## Dependencies & Risks

- **Blocked by** DEV-7461, DEV-7462 and DEV-7463, all merged. **Blocks** DEV-7401 and DEV-7465.
- **The crate keeps changing.** Each change after the move costs two PRs. Accepted (decision 1).
- **A change lands in `cpe/core` between `SRC` and Phase 3.** Phase 3's first check finds it and stops.
  H3 keeps the window empty where it is followed. The fix is a dsp-repository PR that carries the
  change, then a new pin.
- **`cargo deny` reports an error.** Not expected: the new packages are Apache-2.0 or MIT. If it does,
  stop and report: changing `deny.toml` is the owner's decision.
- **rustfmt builds disagree.** See *Formatting*.
- **A re-homed test needs something that is not public.** Checked during planning: `load_remodel` is
  `pub` (`validate.rs:228`), `Remodel` derives `PartialEq` and `Debug`, `HEADER_DESK_BREAKPOINT_PX` is
  `pub`. If Phase 3 still finds a private item, it stops; widening `cpe-core`'s surface is a
  dsp-repository PR.
- **The formatted source breaks an incubator tool.** `tools/module-graph.py` parses `use` trees, which
  `imports_granularity` rewrites, and `geometry.rs` searches `validate.rs` for one exact line. Both are
  run in Phase 3. A failure there is fixed in the incubator's tool or test, never by editing the vendored
  tree.
- **The pin move carries other changes.** `just cpe vendor` copies `cpe-ports`, `sync-store` and
  `areas/access/sync/data` at `<pin>` too. #469 merged first, and it did change a data file: it added two
  curated keys to `0803-curation.csv`, which the incubator's store build refuses until its mapping declares
  them. dsp-incubator#503 moved the pin to #469's commit and made that change, so Phase 3's pin move
  carries the three commits of this plan and nothing else.
- **Phase 2 deletes a comment that carried a contract.** The rules keep any sentence that states one, and
  the phase review reads the diff for this. No mechanical check catches it.
- **Recorded debt.** 549 test names that do not follow `CONVENTIONS.md`; long comments; a specification
  and a glossary that live in another repository than the code. Each is named in the PR description.

## Success Metrics

- Source: dsp-incubator `1cd5781f987ed860218462c086c77ce759185fcc` — 28 `.rs` files, 32,404 lines,
  552 `#[test]` functions in `cpe/core`.
- After Phase 1: 549 tests in `cpe-core`, all passing; the three missing names are the ones in
  *Verification design*.
- After Phase 2: the same 549 names.
- After Phase 3: `vendor-diff: empty (<pin>)`; the incubator's workspace test count equals its count before
  the change.
- Churn baseline for the ADR amendment: 23 commits in 30 days on `cpe/core` and `cpe/engine/src`,
  measured 2026-10-11.

## References

- Linear: DEV-7464 (this move), DEV-7400 (parent), DEV-7401 and DEV-7465 (blocked), DEV-7463 (the crate
  split, dsp-incubator#501)
- `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`, amendment of 2026-10-02
- `docs/adr/0003-one-modulith-per-area.md` (the capability's crate anatomy),
  `docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md` (citations, amendments)
- New-crate registration precedent:
  `docs/specs/2026-09-29-cpe-archive-projection-port/01-feat-cpe-archive-projection-port-plan.md`
- Move precedent: `docs/specs/2026-09-28-move-dpe-to-access-area/01-refactor-move-dpe-to-access-area-plan.md`
- Incubator-phase precedent: `docs/specs/2026-10-09-cpe-port-curation/01-feat-cpe-port-curation-plan.md`,
  Phase 4
- `areas/access/cpe/ports/Cargo.toml` (manifest template), `.rustfmt.toml`, `deny.toml`, `eng.yaml`,
  `.github/scripts/check-adr-refs.sh`, `.github/scripts/check-composition-root-deps.sh`
- dsp-incubator: `cpe/vendor/README.md` ("Adding a crate", "What an empty `vendor-diff` does not cover"),
  `cpe/tools/vendor.sh`, `cpe/tools/crate-deps.py`, `cpe/tools/module-graph.py`,
  `cpe/docs/plans/engine-crate-split/implementation-plan.md`, `cpe/docs/INCUNABULA_GO_LIVE.md` ("Moving"),
  `cpe/docs/adr/0001-crate-layout.md`,
  `cpe/docs/adr/0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`
- Learnings: `docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`,
  `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`,
  `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`
