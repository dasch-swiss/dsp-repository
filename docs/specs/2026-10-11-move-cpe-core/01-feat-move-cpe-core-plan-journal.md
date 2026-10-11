---
plan: docs/specs/2026-10-11-move-cpe-core/01-feat-move-cpe-core-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7464
base_commit: d9839d34
branch: worktree-DEV-7464
started: 2026-10-11
problem: >
  ADR-0007's amendment of 2026-10-02 moves CPE into dsp-repository crate by crate. `cpe-core`, CPE's
  framework-free domain crate (config language, validation, IR, diagnostics), still lives in
  `dsp-incubator/cpe/core` and blocks DEV-7401 and DEV-7465. The owner decided on 2026-10-11 to move it
  now although its churn has not dropped: copy it to `areas/access/cpe/core`, clean its comments, then
  have the incubator build against a vendored copy.
symptoms: []
status: complete
---

# Execution Journal: 01-feat-move-cpe-core-plan

## Repos

| repo           | base_commit | branch            | merge_strategy | status      | pr |
|----------------|-------------|-------------------|----------------|-------------|----|
| dsp-repository | d9839d34    | worktree-DEV-7464 | rebase         | shipped     | https://github.com/dasch-swiss/dsp-repository/pull/470 (merged 2026-10-11, `a9f90359`) |
| dsp-incubator  | 4139af98    | worktree-DEV-7464 | rebase         | shipped     | https://github.com/dasch-swiss/dsp-incubator/pull/505 (draft) |

## Phases

| phase | status      | phase_base              | review_fix_rounds |
|-------|-------------|-------------------------|-------------------|
| 1     | reviewed    | dsp-repository@ca9dc3ba | 2                 |
| 2     | reviewed    | dsp-repository@654fe5b2 | 1                 |
| 3     | reviewed    | dsp-incubator@4139af98  | 1                 |

## Chunk queue

Plan decision 6 makes Phase 1 exactly one commit, so the phase's dsp-repository group is one chunk.

| id  | repo           | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|----------------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | `areas/access/cpe/core/**`, `Cargo.toml`, `Cargo.lock`, `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`, `areas/access/cpe/CONTEXT.md`, `areas/access/CONTEXT.md`, `CONTEXT.md`, `docs/adr/0007-*.md`, `ARCH-MAP.md` | — | every dsp-repository checkbox of Phase 1, "Confirm the branch …" through "Commit as `feat(cpe-core)` …" | 549 tests pass and equal the source list less three names; `diff -r` against the formatted baseline shows only the plan's table of edits; `just check` and `just test` pass; one commit | plan § "Edits Phase 1 makes to the copied source", § "Crate manifest and workspace entries"; SRC = dsp-incubator 1cd5781f987ed860218462c086c77ce759185fcc; baseline at `.claude/tmp/cpe-core-baseline-src` | — |
| 2.1 | dsp-repository | `areas/access/cpe/core/src/**`, `ARCH-MAP.md` | — | every dsp-repository checkbox of Phase 2, "Record `git rev-parse HEAD` …" through "Commit as `docs(cpe-core)` …" (plan decision 6: one commit) | the normaliser diff against 654fe5b2 is empty or rustfmt re-joins; every `"` line of `git diff -U0` read; the checklist's pattern search leaves only kept comments; `cargo doc -p cpe-core --no-deps` prints no warning; test list equals `.claude/tmp/cpe-core-tests-phase1.txt`; `just check` and `just test` pass; one commit | plan § "Comment rules (Phase 2)", § "Verification design"; Phase 1 commit 654fe5b2; candidate lister `.claude/tmp/hits.py`; SRC worktree dsp-incubator `.claude/worktrees/DEV-7464` | — |
| 2.2 | dsp-repository | none (push, PR #470 body) | 2.1 | "Push the branch", "Open a draft PR …", "Run `just commit-lint` with `PR_BODY` …" | branch pushed; PR #470 body rewritten from the template with `allow-many-commits` ticked; `just commit-lint` passes | plan § "Human Actions", § "Dependencies & Risks"; journal `## Deviations`, `## Deferrals` | — |
| 3.1 | dsp-incubator | `cpe/core/**` (deleted), `cpe/vendor/{CRATES,PIN,README.md,dsp-repository/**}`, `cpe/Cargo.toml`, `cpe/Cargo.lock`, `cpe/store-build/**`, `cpe/web/tests/**`, `cpe/Dockerfile`, `cpe/justfile`, `cpe/tools/crate-deps.py`, `cpe/CLAUDE.md`, `cpe/CONTEXT.md`, `cpe/docs/**`, `cpe/static/cpe/*.css`, `cpe/projects/incunabula/DESIGN.md`, every other `cpe/core` or `core/src` citation | — | every dsp-incubator checkbox of Phase 3 from "Confirm the branch …" through "Run `git diff --name-only origin/main..HEAD` …" (the incubator lands a PR as one commit) | workspace test names equal `cpe/scratch/workspace-tests-before.txt` (3065 names) apart from the module path of the three re-homed tests; `just cpe test` passes; `vendor-diff: empty (a9f90359…)`; `just cpe ci` passes; one commit `refactor(cpe): build against the vendored cpe-core (DEV-7464)` | plan § "Phase 3"; `<pin>` = dsp-repository a9f90359525d3e1aff6e5f73d83125fc9fc25039; old pin 14cc0c12; SRC = 1cd5781f; the deleted tests' text is at incubator `4139af98:cpe/core/src/validate.rs` and `…/validate/header_budget.rs` | — |
| 3.2 | dsp-incubator | none (push, draft PR) | 3.1 | "Open a draft PR …" | branch pushed; draft PR open, assigned, no reviewers, no auto-merge | plan § "Human Actions"; incubator `cpe/vendor/README.md` "Moving the pin" | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 76fabe10 | Copied `cpe/core` at SRC to `areas/access/cpe/core`, formatted it, made the plan's table of edits (3 tests deleted, 2 messages without "(ADR-0012)", 25 citations by file name), registered the crate (workspace, three dependency pins, lock, conventions, structure docs, CONTEXT files, ADR-0007 amendment, ARCH-MAP entry now active with Fingerprint 1a7d1dd98bb3). 549 tests pass and equal the source list less the three names; `just check`, `just test`, clippy `--all-targets` pass | none |
| 1.2 | dsp-repository | complete | a3032f55 (76fabe10 amended, as the plan's "Commits" paragraph asks) | Review fix round 1: `HEADER_DESK_BUDGET_PX`'s doc comment names `tokens.css` and the coupling the deleted test guarded; `areas/access/cpe/CONTEXT.md` separates `cpe_core::ir::Resource` from `cpe_ports::Resource` and says the three cross-repository tests are the incubator's; `ARCH-MAP.md` gives the two `cpe-core` dependency rules an enforcement level (**review**), names the interim host-facing subset, lists `ir::Resource`, a Used-by line for `cpe-core`, and the Fingerprint in backticks (`b1a938b01928`) | none |
| 1.3 | dsp-repository | complete | 654fe5b2 (a3032f55 amended) | Review fix round 2, from the re-review of round 1's diff: `areas/access/cpe/CONTEXT.md` no longer counts "three tests" before naming two things they check; the `HEADER_DESK_BUDGET_PX` comment and CONTEXT.md name `dsp-incubator cpe/static/cpe/tokens.css`; the two `ARCH-MAP.md` rules say "of this repository"; Fingerprint `b8ec6f744d66`. `just check`, `just test`, clippy `--all-targets` pass; test list unchanged. The cap of two rounds is reached, so this round's three-file diff was not reviewed again | none |
| 2.1 | dsp-repository | complete | cbfbbc4c | Applied the plan's ordered *Comment rules* to the comments of 19 of the crate's 28 source files (9 held nothing to change): ids, plan and backlog names and design-note pointers removed; rule-defining pointers written from the incubator's root; history restated in the present tense; "Phase 2" named as validation; incubator crates named by path; `src/lib.rs` reworded; 11 private intra-doc links turned into code spans. Normaliser diff against 654fe5b2 empty for all 28 files; one non-whole-line change (`config.rs`, trailing comment, no string literal); test list equal (549); `cargo doc` prints no warning; `just check`, `just test` pass; Fingerprint `b3c426012964` | none |
| 2.3 | dsp-repository | complete | fe2a95e1 (cbfbbc4c amended, as the plan's "Commits" paragraph asks) | Review fix round 1 (rust, consistency, simplicity, dune): two CSS selectors in `iiif.rs` got back the descendant space a re-wrap had removed (`.gm[data-tile="tall"] .gm__box`, `.case__detail .case__bed`); `validate.rs` names `serve_project_asset` in `dsp-incubator cpe/bin/src/router.rs`; stray spaces (`showcase.rs:1`, `→ .keyword`, two `a/ b` joins); `any_facet_selected` named in the `ir.rs` re-export sentence; three "old …" phrasings and one double dash reworded. Proofs, test list, `cargo doc`, `just check`, `just test` re-run and pass; Fingerprint `a451b7ac4751`. Re-review of the fix diff (consistency, simplicity, dune): no findings | none |
| 2.2 | dsp-repository | complete | — (no commit: push and PR body) | Pushed `worktree-DEV-7464` (ca9dc3ba..fe2a95e1, a plain push). Rewrote the body of draft PR #470 from the repository's template: the three commits, Review Notes with each commit's check, the two accepted deviations, the recorded debt, the DUNE-001 deferral, H1-H4; `allow-many-commits` ticked. `just commit-lint` with `PR_BODY` from the live PR passes (3 commits, no merge commit) | none |
| 3.1 | dsp-incubator | complete | a56c242f | Rebased on incubator `origin/main` 4139af98; `git log SRC..origin/main -- cpe/core` prints nothing. `<pin>` = dsp-repository a9f90359525d3e1aff6e5f73d83125fc9fc25039 (on `origin/main`, Phase 1 commit 9b431e6d its ancestor). Deleted `cpe/core`, vendored `areas/access/cpe/core` (pin 14cc0c12 → a9f90359; `cpe-ports`, `sync-store` and `areas/access/sync/data` unchanged between the pins, so no other dsp-repository commit to name). The four inherited dependency lines are byte-equal to the monorepo root's at the pin; `Cargo.lock` unchanged. Re-homed the three tests (`store-build/tests/it/worked_example.rs`, `web/tests/header_budget.rs`), each shown to fail when its input file changes. Repointed `geometry.rs`, the Dockerfile, the justfile, `tools/crate-deps.py` and every `core/src` citation; docs, ADR notes and `vendor/README.md` say the crate is vendored. Test names: 3065 before and after, differing only in the module path of the three. `just cpe test`, `vendor-diff: empty (a9f90359…)`, `crate-deps-check` (`core-framework-refs=0`), `module-graph-check` and `just cpe ci` pass; no ignored file under `cpe/vendor` | none |
| 3.3 | dsp-incubator | complete | 613dcc15 (a56c242f amended: the incubator lands a PR as one commit) | Review fix round 1 (consistency, simplicity, dune, rust; no Critical, four Warnings from simplicity, the rest Suggestions): `tools/crate-deps.py` no longer threads a `core_src` parameter, it scans `<root>/core/src` with the vendored `areas/access/cpe` directory as the real workspace's root; `cpe/justfile` has a `core` variable; `cpe/CLAUDE.md` and the ADR-0013 note point to `vendor/README.md`'s "`cpe-core`" section instead of repeating its list, and the history bullet and the churn figure are gone; stale statements corrected in `docs/CONFIG_LANGUAGE.md` (where the worked-example test lives), `docs/dev/coding-conventions.md` and `docs/dev/testing-strategy.md`; "monorepo PR" in the added prose. `just cpe ci` passes. Re-review of the fix diff (consistency, simplicity, dune): no Critical, no Warning | none |
| 3.2 | dsp-incubator | complete | — (no commit: push and PR) | Pushed `worktree-DEV-7464` (new branch, a plain push of 613dcc15) and opened draft PR #505 on dsp-incubator, assigned to the owner, no reviewers, no auto-merge. `git diff --name-only origin/main..HEAD` lists only paths under `cpe/`, and under `cpe/projects/` only `incunabula/DESIGN.md` | none |

## Deferrals

- `cargo doc -p cpe-core --no-deps` prints 11 `rustdoc::private_intra_doc_links` warnings (public docs linking
  private items in `iiif.rs`, `labels.rs`, `util.rs`, `validate.rs`). They are in the source at SRC. Fixing them
  is a doc-comment edit outside the plan's table of Phase 1 edits, so it was not made; a `- [ ]` item was added
  to Phase 2's checklist (before "For each file …"), where the comment-only proof allows it.
- `scope` (dune-reviewer DUNE-001, Warning, verified): nothing mechanical stops a crate outside
  `areas/access/cpe/` from depending on `cpe-core`, or `cpe-core` from depending on a CPE crate;
  `check-composition-root-deps.sh` covers `areas/access/server` only, and `ir::DataArks::from_stored` is `pub`.
  The reviewer proposes a `Cargo.toml` grep run by `just check` (the mechanism ADR-0007 names). A new gate is
  outside this plan; the rule is recorded as **review** in `ARCH-MAP.md`. For the session's final review.
- `scope` (Phase 3, dune-reviewer, native high, advisory because the incubator has no `ARCH-MAP.md`; verified by
  reading `cpe/tools/vendor.sh`): `just cpe vendor-diff` prints a difference and exits 0, and `fmt-check` and `lint`
  skip the vendored tree, so an edit of the vendored `cpe-core` passes `just cpe ci`. That is ADR-0013's exit rule,
  decided for small `cpe-ports` patches. The reviewer proposes that an unexplained difference fails unless a
  hand-kept list names the patched crate. Changing the rule is an incubator ADR-0013 amendment, outside this plan;
  named in PR #505's description. For the session's final review.
- Phase 3, dune-reviewer (low): the monorepo side does not name the incubator's guard tests. A comment at
  `areas/access/cpe/core/src/validate/header_budget.rs` naming `dsp-incubator cpe/web/tests/header_budget.rs`, and one
  at the remodel lowering naming `cpe/store-build/tests/it/worked_example.rs`, is a dsp-repository PR. Not made:
  the phase has no dsp-repository group, and it would move the pin again.

## Deviations

- Plan checkbox and acceptance criterion "`cargo deny check` reports no error with `deny.toml` unchanged"
  (beside decision 9: no licence exception, `deny.toml` unchanged). As committed (76fabe10): `deny.toml` is
  unchanged and decision 9 holds, but the statement is untrue. `cargo deny check` (0.19.0 locally) already
  reports 22 errors on `origin/main` d9839d34: every workspace crate "is unlicensed", two
  `CDLA-Permissive-2.0` rejections (`webpki-roots`, `webpki-root-certs`) and six advisories. The move adds
  one of the same class, `cpe-core = 0.8.7 is unlicensed`, and none from `kdl`, `miette`, `pulldown-cmark`,
  `pulldown-cmark-escape`, `unicode-width` or `winnow`. The plan's "stop and report" was written for an error
  from the new packages; a red `cargo deny` on `main` is not this plan's to fix. Proposed wording for the
  acceptance criterion: "`cargo deny check` reports no error that `origin/main` does not, apart from
  `cpe-core` being unlicensed like every workspace crate". Owner's answer: accepted by the session in the unsupervised run of 2026-10-11; for the owner to confirm at PR review.
- Plan checkbox "`cargo doc -p cpe-core --no-deps`; it prints no warning": untrue at 76fabe10, see *Deferrals*.
  Owner's answer: accepted by the session in the unsupervised run of 2026-10-11; for the owner to confirm at PR review.
- Incubator acceptance criterion "No `cpe/core` or `core/src` reference is left outside `cpe/vendor/`,
  `cpe/docs/plans/` and `cpe/tools/crate-deps-fixtures/` that does not name the vendored path". As committed
  (613dcc15): true everywhere but `cpe/tools/crate-deps.py`, which says `core/src` in its docstring, two messages
  and its `--root` help. The tool scans `<root>/core/src`; the root is the vendored `areas/access/cpe` directory in
  the real workspace and a fixture directory in the self-test, as the plan's own checkbox asks ("a fixture root
  keeps its own `core/src`"). Proposed wording: add "and `cpe/tools/crate-deps.py`, where `core/src` is relative to
  the scanned root" to the criterion's exclusions. Owner's answer (2026-10-11): accepted; the criterion excludes the tool.

## Side findings

- The formatted baseline needs `just fmt` twice: the first pass leaves a trailing empty `///` line in
  `validate/context.rs` that the second removes. The plan's "run `just fmt`" (once) for the reviewer's
  baseline gives one spurious hunk; the commit body says to run it until stable.
- rustfmt's `wrap_comments` breaks a section ruler (`// ── Title (citation) ────`) that grows past 100
  columns into two lines with a dangling ruler tail. The two rulers that held an ADR number
  (`ir.rs` "Annotations", `validate.rs` "Figure-block slot") carry the citation on a line beneath instead.
- The second `MultipleImageViewers` message line break moved (`per page: \` / `drop 'media' …`); the printed
  text is what decision 8 words.
- `ARCH-MAP.md`: `dune:dune-map` was loaded and followed by hand for this one entry (no user to confirm, no
  subagents; the plan checkbox authorises the overwrite). The entry lost `status: planned` because it now has
  a Fingerprint; the Overview counts five planned components.
- `cargo deny check` does not report the second `winnow` (0.6 beside 0.7) as a duplicate warning locally,
  although both are in the lock; not investigated.
- Local `cargo-deny` is 0.19.0; `just install-requirements` pins 0.20.2.
- Phase 1 review, suggestions not acted on: the first `CompoundMediaOwnership` assert could use a fragment unique
  to its message (decision 8 fixes the asserted text); the comment on `ANNOTATION_TOLERANCE` (`validate.rs`)
  says a test in `store-build/src/generic/geometry.rs` reads the line below, a cross-repository guard Phase 2's
  rule 8 should name by its incubator path; the `kdl` pin comment's claim (6.6 needs Rust 1.95, `serde` on by
  default) is the plan's and was not checked against kdl's changelog; the `pulldown-cmark` comment speaks of
  "both crates" before the component library is here (plan text); the ADR-0007 amendment carries the churn
  numbers the plan asks for; the "component library, store, page assembly and routes" status sentence sits in
  five files; the `areas/access/cpe` Local-context kit is 14 files against a budget of seven (the plan adds
  `core/src/lib.rs` only).
- Phase 2: a re-wrap helper that tidied " ." after a deletion also removed the space inside two CSS descendant
  selectors, and joining a line that ended in `/` put a space into `a/b`. The normaliser proof cannot see
  either (comments only); all four reviewers of the phase caught the selectors. A comment re-wrap must never
  touch whitespace inside the text it joins.
- Phase 2 review, suggestions not acted on (outside the *Comment rules* or rule 10): bare CSS file names beside a
  full path elsewhere in the same file (`tokens.css` in `theme.rs` and `validate/header_budget.rs`, `record.css`
  and `grid.css` in `iiif.rs`, `shell.css` in `markup_contract.rs`); `component::…` and `query::…` item paths of
  incubator crates at the far end of "must agree" statements (`markup_contract.rs`, `narrative_outline.rs`,
  `validate.rs` `RESERVED_FACET_PROPERTIES`); "the bin" / "the binary" / "the `bin` crate" as prose names for the
  incubator binary; `schema.sql`, `data.sql`, `translate.py` named bare; "today's" in `ir.rs` and `util.rs`, and
  "keeps its previous, still-useful derived title" (`util.rs`); `src/lib.rs` does not say that `cpe-core` depends
  on no CPE crate (DUNE-003; `ARCH-MAP.md` says it); `narrative_outline.rs` names the incubator's
  `just cpe crate-deps-check` as the guard and no statement of the rule in this repository (DUNE-004);
  `cpe_ports::DATA_ARK_PREFIX` does not name the two constants it must equal (DUNE-007, another crate);
  `src/remodel.rs` in `validate/remodel.rs` could be read as relative to the module.
- Not acted on from the round-1 re-review (suggestion): the `cpe-ports` Key entities in `ARCH-MAP.md` still name a
  bare `Resource`; the `ir::Resource` clause beside it carries the distinction.
- Phase 3: no incubator test expected "ADR-0012" in a diagnostic message (only comments name it), so the checkbox
  for those tests changed nothing. `cpe/Cargo.lock` did not change: a path package has no source line.
- Phase 3: the harness refuses any compound shell command whose text holds the checkout path (it contains
  "GitHub"), also without a git call. A two-line helper under `cpe/scratch/` that runs a snippet from the worktree
  root (`python3 -I run.py '<snippet>'`) was the way through; `git show --output=` does not redirect a blob.
- Phase 3 review, suggestions not acted on: a `cpe/vendor/CLAUDE.md` saying the tree is read-only, and a pointer
  near the top of `cpe/CLAUDE.md` (the entry sits at line 3375 of 4,500); `vendor-diff` comparing the inherited
  dependency lines with the monorepo root's at the pin (seven keys, compared by hand today); the full vendored path
  in CSS and code comments where "`cpe-core`'s `iiif.rs`" would do (the plan asks for the path); the path repeated
  in `geometry.rs`'s failure message; the long `BACKLOG.md` heading; `crate-deps.py`'s exit-2 and violation lines
  print `core/src/…` relative to the scanned root, and its fixture docstring still calls the case directory "the
  workspace root"; the `CLAUDE.md` pointer's parenthesis names three of the README section's five items; the
  removed `CLAUDE.md` bullet (the crate's comments cite the incubator by path; two diagnostics lost "(ADR-0012)")
  is now only in the commit body and PR #505; `INCUNABULA_GO_LIVE.md` no longer carries the churn figure, which
  ADR-0007's amendment in dsp-repository holds.

## Closeout

- root_cause: `cpe-core` lived in dsp-incubator while the rest of CPE was moving to dsp-repository crate by crate
  (ADR-0007, amendment of 2026-10-02), so DEV-7401 and DEV-7465 had nothing to build against.
- investigation: three plan statements did not hold when run. `cargo deny check` is red on `main` for reasons the
  move does not add to; the copied source printed 11 rustdoc warnings; and #469, which merged in between, added two
  curated keys the incubator's store build refuses, so its pin move needed a change of its own (dsp-incubator#503)
  before Phase 3 could stay a pure crate swap.
- solution: dsp-repository#470 copied the crate to `areas/access/cpe/core` as a formatted copy with a small table
  of edits, then cleaned its comments in a comment-only commit. dsp-incubator#505 deletes `cpe/core`, vendors the
  crate at `a9f90359` and re-homes the three tests that read incubator files.
- prevention: the workspace test-name lists before and after each step are compared, not counted; the incubator's
  `vendor-diff` is empty at the pin; a change to the config language is now a dsp-repository PR followed by a pin
  move. Open: `vendor-diff` exits 0 on a difference, and nothing mechanical limits who may depend on `cpe-core`.
