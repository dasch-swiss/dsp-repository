---
plan: docs/specs/2026-11-19-hidden-projects/01-feat-hidden-projects-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7919
base_commit: 65c3194fef23ec5d3ca13bc34a8ff8b2a3c026de
branch: feature/dev-7919-hide-a-project-on-prod-no-crawler-indexing
started: 2026-11-19
problem: >
  dsp-repository has no way to serve a presentation without advertising it: CPE's overview lists
  every project that loaded, and nothing stops a crawler that finds a link from indexing a
  project's pages. Keller (083C) and the Sephardic Bibles (086A) must run on stage and prod before
  they go public, reachable only through their link (DEV-7919, blocking DEV-7935).
status: complete
autonomy: decide
---

# Execution Journal: 01-feat-hidden-projects-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | 65c3194 | feature/dev-7919-hide-a-project-on-prod-no-crawler-indexing | rebase | in-progress | — |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@f5e538e | 1 |

## Archived phases

- Phase 1: 7 chunks, ids 1.1 to 1-review-1, highest number 1.6, in 01-feat-hidden-projects-plan-journal-archive.md

## Chunk queue

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.7 | dsp-repository | done | 51317bf | final-review fix (docs): overview leaves Hidden projects out (operations, ADR-0007, render_site_index); non-UTF-8 row, `hidden projects:` log line, no X-Robots-Tag on the pre-activation 503; DPE named as the exception; root normalises, CPE re-upper-cases as fail-closed guard | — |
| 1.8 | dsp-repository | done | 52183c2 | final-review fix (simplicity): `hidden_projects_from_env` out of `serve()`; `router::build` takes `&Arc<BTreeSet>`; test helpers deduplicated (serve.rs `get`/fixtures, base_tests statics) | — |

## Deferrals

- eng:test-browser verify deferred to the final review by the owner (2026-11-19); run via the Agent Browser CLI (`agent-browser`), not Chrome
- Phase 1: scope (Warning): dune-reviewer DUNE-001 asks for a reusable conformance test of the four Hidden-project
  obligations outside `cpe-server`, so DaTEI's first PR inherits a failing test. A new shared test component is not in
  the plan; left for DEV-7574 (the ARCH-MAP boundary rule added in 74a32ddc names it as review until then).

## Deviations

- Phase 1: provisional: the plan lists `/cpe//things/` among the paths that carry no `X-Robots-Tag`. axum's `nest` hands
  CPE `/things/` for it, and CPE answers the Hidden project's page (200, with the meta), so the test expects the
  header there; leaving it off would leak the Hidden page unmarked. Alternative set aside: normalising or refusing the
  double slash in CPE, which changes routing beyond this plan. Commit 371b588a. Settled: accepted by the owner, 2026-11-19.

## Side findings

- Phase 1: test names follow the sentence style of the files they sit in (`serve.rs`, `activation.rs`, `base_tests.rs`,
  `page.rs` have no `test_` names), not `CONVENTIONS.md`'s `test_{what}_{condition}_{expected}`, which the plan asked for.
  `CONVENTIONS.md` and the code disagree; one of them wants updating.
- Phase 1: `/cpe//things/` is served as `/cpe/things/` (axum `nest` collapses the slash), so a Hidden project's pages
  also answer under a double-slash URL. The header covers it; worth knowing for canonical URLs.
- Phase 1: four reviewers (rust, consistency, security, dune) suggest logging a hidden shortcode that matches no folder at
  `warn`, not `info`: a well-formed typo leaves the project indexable with only an info line. The plan chose `info`
  because listing a project before it is deployed is legitimate; left as planned for the owner to weigh.
- Phase 1: the pre-activation 503 and the 503 of a refused configuration carry no `X-Robots-Tag` (outside the activated
  router). The plan accepts this (a crawler does not index a 503); `areas/access/CONTEXT.md` says "every response under
  the project's prefix" without that exception.
- Phase 1: suggestions not taken: carrying `Indexing` on `Project` or `RenderContext` (both rejected by the plan's
  Alternatives), passing the `hidden_ids` `Arc` into `router::build` instead of cloning the set once, trimming the
  Hidden-project prose repeated in `docs/src/cpe/operations.md`, and `parse_hidden_projects` logging its own rejects.
- Phase 1: trace spans (`GET /cpe/<project id>/…`) export a Hidden project's path to the trace backend; not public, but
  an egress the term's caveats do not name. `Cpe::projects()` still includes Hidden projects (documented); a future
  host listing built from it would publish them.
- Phase 1: dune-reviewer leans to an ADR ("area-wide settings are read by the root and passed as values") once a
  second capability (DaTEI) adopts the set; none now, as the plan decided.
- Phase 1: re-review suggestions not taken: state in the Hidden project term that capabilities match the set's
  Shortcodes in any case, and name DEV-7574's pending conformance test in `datei/CONTEXT.md`'s boundary commitment;
  rename the shared-shortcode activation test to mention case. `just check && just test` passed after the fix round
  (90 tests).

## Owner decisions

- 2026-11-19, final review (answered: 2026-11-19, the owner accepted all defaults): keep the header on `/cpe//things/`;
  keep CPE's second upper-casing of the set; keep the one-clause ADR-0007 edit; keep the dropped DPE test and the
  unchanged `Project`; keep the unmatched-shortcode log at `info`. Shortcodes for ops-deploy: `083C,086A`
  (dasch-swiss/ops-deploy#1485). Side findings went to Linear follow-ups; the two term caveats (the 503 before
  activation, trace spans) were folded into `areas/access/CONTEXT.md`.

## Closeout

- Phase 1 landed in 7 commits, plus a final-review fix round (2 commits) and one doc-wording commit. `just check` and `just test` pass.
- Final review: 3 rounds. Round 1 ran 8 reviewers (ivan, security, accessibility, rust, maud-datastar, consistency, dune, simplicity) and found no Critical. The doc and simplicity Warnings were fixed. Rounds 2 and 3 fixed doc wording only. Round 3 had no findings.
- Verify: at the end, as the owner asked, through `agent-browser` (2026-11-19). `/cpe/` shows the empty state. `/cpe/incunabula/` renders, with `X-Robots-Tag` and the robots meta. `/dpe/projects` has no header.
- Side findings for the owner:
  - CPE's shared static `incunabula.css` is linked from the overview's head, so a hidden project's id appears in the overview's source. This predates the change.
  - The overview's empty-state paragraph sits outside the page gutter.
