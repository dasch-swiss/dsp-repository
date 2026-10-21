---
plan: docs/specs/2026-10-21-editor-collection-without-report-token/01-feat-editor-collection-without-report-token-plan.md
target_repo: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dasch-specs
base_commit: 32dd0e2
branch: specs/dev-7595-collection-without-report
started: 2026-10-21
problem: >
  The collector reports each record's pull request state back to the editor over
  POST /api/v1/collection-report, authenticated by EDITOR_COLLECTION_TOKEN. The
  loop exists only so RDU see per-record failure on an editor surface (REQ-5.7),
  yet costs a secret in two places, a PROD startup check, a CSRF carve-out, five
  columns, nine record classifications, a refresh mode and a refusal gate on
  approval, while GitHub already holds the true state. Drop it: the collector
  writes a job-summary row per record, /collection links to GitHub, and approval
  always supersedes the earlier record.
status: complete
---

# Execution Journal: 01-feat-editor-collection-without-report-token-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|-----|
| dasch-specs | 32dd0e2 | specs/dev-7595-collection-without-report | squash | shipped | https://github.com/dasch-swiss/dasch-specs/pull/244 |
| dsp-repository | 638e66e3 | feature/dev-7595-editor-collection-without-a-report-token-drop-the-collection | rebase | shipped | https://github.com/dasch-swiss/dsp-repository/pull/522 |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dasch-specs@32dd0e2 | 1 |
| 2 | reviewed | dsp-repository@261266ad | 1 |
| 3 | reviewed | dsp-repository@7505b4ab | 1 |
| 4 | reviewed | dsp-repository@b902c689 | 1 |

## Chunk queue

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dasch-specs | specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md | — | all Phase 1 edit items | REQ-5.1/5.3/5.7 and Edge Case 4 describe the shipped mechanism | PRD:127-133 § US-5, :206 § Edge Cases | — |
| 2.1 | dsp-repository | collector/src/{config,editor,run,main}.rs | — | Phase 2 items 1-7 | no report, no mode, Opened/Updated split, per-record job-summary rows, guard remedy, exit on failure | run.rs:30-376 § Outcome, collect, finish; config.rs; editor.rs; main.rs | — |
| 2.2 | dsp-repository | .github/workflows/collect-editor-records.yml | — | Phase 2 workflow item | no mode input, no token, fixed names | whole file | — |
| 2.3 | dsp-repository | docs/src/editor/collection.md, areas/deposit/editor/CONTEXT.md, ARCH-MAP.md | 2.1, 2.2 | Phase 2 docs item | docs describe the shipped workflow | collection.md:119-139, CONTEXT.md:52, ARCH-MAP.md:220-221 | — |
| 3.1 | dsp-repository | web/src/handlers/{review,collection,projects}.rs, store/src/{review_rounds,test_support}.rs, core/src/records.rs | — | Phase 3 items 1-5, 7 | approval over a record in any PR state succeeds and leaves one record; gate, is_live, fault gone | review.rs:472-492, review_rounds.rs:113-162, projects.rs:155-175 | — |
| 3.2 | dsp-repository | collector/src/{run,forge}.rs | Phase 2 review fixes | Phase 3 item 6 + renumber deviation | landed new entities neither renumbered nor rewritten; renumbered landing mapped | run.rs:150-200, 241-282; forge.rs:22-57, 140-210 | — |
| 3.3 | dsp-repository | docs/src/editor/collection.md, areas/deposit/editor/CONTEXT.md | 3.1, 3.2 | Phase 3 docs item | docs describe supersede and landed-entity rule | collection.md:39-41, :111; CONTEXT.md:48, :95 | — |
| 4.1 | dsp-repository | areas/deposit/editor/{core,store,web,collector}/**, areas/deposit/server/src/{config,editor,csrf}.rs | — | Phase 4 Rust items | no route, token, column, classification or type of the report remains; /collection link + Discard every row | plan Phase 4 | — |
| 4.2 | dsp-repository | web-e2e-tests/{playwright.config.ts,tests/collection.spec.ts} | — | Phase 4 E2E item | spec without report POST: Waiting for release, GitHub link, Discard | collection.spec.ts | — |
| 4.3 | dsp-repository | docs/src/editor/*.md, areas/deposit/editor/CLAUDE.md, areas/deposit/docs/adr/000{3,4}*.md | — | Phase 4 docs item | docs describe the editor without the report | plan Phase 4 docs item | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dasch-specs | complete | 26c1a24 | amended REQ-5.1/5.3/5.7, resolved Edge Case 4 for collection; draft PR #244 | none |
| 1.2 | dasch-specs | complete | 8f6240a | review fixes: REQ-5.4 landed-entity exception, supersede rule once in REQ-5.3, stale REQ-5.1 rationale, vocabulary | none |
| 2.1 | dsp-repository | complete | 7505b4ab | collector: no report/mode/token; Opened/Updated/Skipped/Failed; per-record job-summary rows; guard remedy | none |
| 2.2 | dsp-repository | complete | 7505b4ab | workflow: no mode input, no token, fixed names, EDITOR_BASE_URL guard only | none |
| 2.3 | dsp-repository | complete | 7505b4ab | docs: collection.md Running it, staleness section removed; CONTEXT.md:52; ARCH-MAP.md:220 | none |
| 2.4 | dsp-repository | complete | c81d01fa | review fixes: summary write error no longer aborts the run; collect_records split + wiring tests; code-span cells against Markdown/HTML injection; doc reason wording | none |
| 3.1 | dsp-repository | complete | 910c5a1c | approval supersedes on shortcode alone; live-PR gate, APPROVE_REFUSED_COLLECTING, is_live, fault removed; 4-state supersede test | none |
| 3.2 | dsp-repository | complete | 910c5a1c | landed new entities neither renumbered nor written; Renumbered: body line pair; renumbering-target guard (session) | none |
| 3.3 | dsp-repository | complete | 910c5a1c | docs: landed-entity rule, One record per project, CONTEXT.md:48,:95 | none |
| 3.4 | dsp-repository | complete | f29e649c, b902c689 | review fixes: ARCH-MAP supersede sentence; Renumbered pairs must target a file the PR carried, same kind; round-trip and guard tests; nothing-to-commit is skipped (or failed naming a stale open PR); docs wording | none |
| 4.1 | dsp-repository | complete | 93c86f2b | editor drops report: route, token, PROD check, CSRF carve-out, subtle, five columns, classifications, stranded; /collection GitHub link + Discard every row | none |
| 4.2 | dsp-repository | complete | 93c86f2b | E2E: Waiting for release, GitHub link, Discard; no report POST | none |
| 4.3 | dsp-repository | complete | 93c86f2b | docs: collection, architecture, routing, operations, status, editor CLAUDE.md, ADR-0003/0004 | none |
| 4.4 | dsp-repository | complete | 829a1160 | review fixes: PullRequestState moved to the collector; shared BRANCH_PREFIX in editor-core; per-row snapshot links; CONTEXT/CLAUDE wording | none |
| F.1 | dsp-repository | complete | b68819cc, 69661937, d262399a, ae7d53fa | finalize cleanup (user request): redundant comments and over-testing removed, attributed per phase by blame; ARCH-MAP entries and fingerprints | none |

## Deferrals

- eng:test-browser deferred from the Phase 3 checkpoint to Phase 4's (intake decision); runs only at Phase 4.

## Deviations

- Phase 3 landed-entity rule widened (intake, user asked for the best non-overengineered fix): the collector also reads its own `Renumbered:` body line of a merged pull request, mirroring `record_line`/`record_in`, so an entity renumbered at first collection is matched under its landed id and the project references it. Plan as written matched by id only, which duplicates a renumbered entity.

## Side findings

- `just test` cannot run inside the sandbox: dsp-cli wiremock tests need to bind a port. Phase 2 gate: `just check` passed locally; `just test` left to CI on user instruction.
- Workers ran `cargo test` on touched crates after the user said to leave tests to CI (the brief told them to); from Phase 4 on, workers run only clippy on touched crates.

## Closeout

- root_cause: the editor kept its own copy of collection state (five columns, nine classifications) fed by a bearer-token report from CI, solely so RDU could see per-record failures in the editor; that copy could only lag GitHub, so it needed a refresh mode, a staleness caveat, a refusal gate on approval and a secret in two places.
- investigation: the plan's landed-entity rule matched a re-approved `new` entity by id only; an entity renumbered at first collection lands under another id, so it would be duplicated. Fixed by a `Renumbered: <from> <to>` body line parsed back like `Approved-record:`, accepted only when the target is a file the same merged PR carried and of the same kind. Review also found: a summary write error aborted the run mid-way; job-summary cells rendered depositor/editor text as Markdown links and images; re-approving content main already carries produced an empty commit that failed every run; a renumbering's target id could be mistaken for a different new entity.
- solution: the collector writes one job-summary row per record (opened, updated, skipped, failed) and fails the run on a failure; approval deletes every earlier record of the shortcode in the approve transaction; the collector skips `new` entities that already landed through merged PRs on the project's branch and points references at the landed id; the editor drops the report endpoint, token, columns and classifications, and `/collection` links each project's pull requests on GitHub with Discard on every row.
- prevention: tests pin the summary wiring (rows in order, write failure still attempts every record), Markdown neutralisation, landed and renumbered-landed entities, the renumbering-target guard, the Merged-only and New-only guards, nothing-to-commit handling, and that approving supersedes the project's earlier record; the branch prefix is one `editor-core` constant shared by collector and editor link. Tests were left to CI at the user's instruction; `eng:test-browser` and the E2E run were skipped, so `/collection` contrast and reflow are unmeasured locally.
