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
status: in-progress
---

# Execution Journal: 01-feat-move-cpe-core-plan

## Repos

| repo           | base_commit | branch            | merge_strategy | status      | pr |
|----------------|-------------|-------------------|----------------|-------------|----|
| dsp-repository | d9839d34    | worktree-DEV-7464 | rebase         | in-progress | —  |

## Phases

| phase | status      | phase_base              | review_fix_rounds |
|-------|-------------|-------------------------|-------------------|
| 1     | reviewed    | dsp-repository@ca9dc3ba | 2                 |
| 2     | reviewed    | dsp-repository@654fe5b2 | 1                 |
| 3     | gated (H1)  |                         | 0                 |

## Chunk queue

Plan decision 6 makes Phase 1 exactly one commit, so the phase's dsp-repository group is one chunk.

| id  | repo           | files | depends_on | checkboxes | acceptance | context | replaces |
|-----|----------------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | `areas/access/cpe/core/**`, `Cargo.toml`, `Cargo.lock`, `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`, `areas/access/cpe/CONTEXT.md`, `areas/access/CONTEXT.md`, `CONTEXT.md`, `docs/adr/0007-*.md`, `ARCH-MAP.md` | — | every dsp-repository checkbox of Phase 1, "Confirm the branch …" through "Commit as `feat(cpe-core)` …" | 549 tests pass and equal the source list less three names; `diff -r` against the formatted baseline shows only the plan's table of edits; `just check` and `just test` pass; one commit | plan § "Edits Phase 1 makes to the copied source", § "Crate manifest and workspace entries"; SRC = dsp-incubator 1cd5781f987ed860218462c086c77ce759185fcc; baseline at `.claude/tmp/cpe-core-baseline-src` | — |
| 2.1 | dsp-repository | `areas/access/cpe/core/src/**`, `ARCH-MAP.md` | — | every dsp-repository checkbox of Phase 2, "Record `git rev-parse HEAD` …" through "Commit as `docs(cpe-core)` …" (plan decision 6: one commit) | the normaliser diff against 654fe5b2 is empty or rustfmt re-joins; every `"` line of `git diff -U0` read; the checklist's pattern search leaves only kept comments; `cargo doc -p cpe-core --no-deps` prints no warning; test list equals `.claude/tmp/cpe-core-tests-phase1.txt`; `just check` and `just test` pass; one commit | plan § "Comment rules (Phase 2)", § "Verification design"; Phase 1 commit 654fe5b2; candidate lister `.claude/tmp/hits.py`; SRC worktree dsp-incubator `.claude/worktrees/DEV-7464` | — |
| 2.2 | dsp-repository | none (push, PR #470 body) | 2.1 | "Push the branch", "Open a draft PR …", "Run `just commit-lint` with `PR_BODY` …" | branch pushed; PR #470 body rewritten from the template with `allow-many-commits` ticked; `just commit-lint` passes | plan § "Human Actions", § "Dependencies & Risks"; journal `## Deviations`, `## Deferrals` | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 76fabe10 | Copied `cpe/core` at SRC to `areas/access/cpe/core`, formatted it, made the plan's table of edits (3 tests deleted, 2 messages without "(ADR-0012)", 25 citations by file name), registered the crate (workspace, three dependency pins, lock, conventions, structure docs, CONTEXT files, ADR-0007 amendment, ARCH-MAP entry now active with Fingerprint 1a7d1dd98bb3). 549 tests pass and equal the source list less the three names; `just check`, `just test`, clippy `--all-targets` pass | none |
| 1.2 | dsp-repository | complete | a3032f55 (76fabe10 amended, as the plan's "Commits" paragraph asks) | Review fix round 1: `HEADER_DESK_BUDGET_PX`'s doc comment names `tokens.css` and the coupling the deleted test guarded; `areas/access/cpe/CONTEXT.md` separates `cpe_core::ir::Resource` from `cpe_ports::Resource` and says the three cross-repository tests are the incubator's; `ARCH-MAP.md` gives the two `cpe-core` dependency rules an enforcement level (**review**), names the interim host-facing subset, lists `ir::Resource`, a Used-by line for `cpe-core`, and the Fingerprint in backticks (`b1a938b01928`) | none |
| 1.3 | dsp-repository | complete | 654fe5b2 (a3032f55 amended) | Review fix round 2, from the re-review of round 1's diff: `areas/access/cpe/CONTEXT.md` no longer counts "three tests" before naming two things they check; the `HEADER_DESK_BUDGET_PX` comment and CONTEXT.md name `dsp-incubator cpe/static/cpe/tokens.css`; the two `ARCH-MAP.md` rules say "of this repository"; Fingerprint `b8ec6f744d66`. `just check`, `just test`, clippy `--all-targets` pass; test list unchanged. The cap of two rounds is reached, so this round's three-file diff was not reviewed again | none |
| 2.1 | dsp-repository | complete | cbfbbc4c | Applied the plan's ordered *Comment rules* to the comments of 19 of the crate's 28 source files (9 held nothing to change): ids, plan and backlog names and design-note pointers removed; rule-defining pointers written from the incubator's root; history restated in the present tense; "Phase 2" named as validation; incubator crates named by path; `src/lib.rs` reworded; 11 private intra-doc links turned into code spans. Normaliser diff against 654fe5b2 empty for all 28 files; one non-whole-line change (`config.rs`, trailing comment, no string literal); test list equal (549); `cargo doc` prints no warning; `just check`, `just test` pass; Fingerprint `b3c426012964` | none |
| 2.3 | dsp-repository | complete | fe2a95e1 (cbfbbc4c amended, as the plan's "Commits" paragraph asks) | Review fix round 1 (rust, consistency, simplicity, dune): two CSS selectors in `iiif.rs` got back the descendant space a re-wrap had removed (`.gm[data-tile="tall"] .gm__box`, `.case__detail .case__bed`); `validate.rs` names `serve_project_asset` in `dsp-incubator cpe/bin/src/router.rs`; stray spaces (`showcase.rs:1`, `→ .keyword`, two `a/ b` joins); `any_facet_selected` named in the `ir.rs` re-export sentence; three "old …" phrasings and one double dash reworded. Proofs, test list, `cargo doc`, `just check`, `just test` re-run and pass; Fingerprint `a451b7ac4751`. Re-review of the fix diff (consistency, simplicity, dune): no findings | none |
| 2.2 | dsp-repository | complete | — (no commit: push and PR body) | Pushed `worktree-DEV-7464` (ca9dc3ba..fe2a95e1, a plain push). Rewrote the body of draft PR #470 from the repository's template: the three commits, Review Notes with each commit's check, the two accepted deviations, the recorded debt, the DUNE-001 deferral, H1-H4; `allow-many-commits` ticked. `just commit-lint` with `PR_BODY` from the live PR passes (3 commits, no merge commit) | none |

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
