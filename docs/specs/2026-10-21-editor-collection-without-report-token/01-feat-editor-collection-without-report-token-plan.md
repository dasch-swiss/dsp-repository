---
title: "feat: Collect editor records without a report token"
type: feat
date: 2026-10-21
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dasch-specs
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dasch-specs
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository
linear: DEV-7595
linear_project: Metadata Editor for RDU + depositing projects
---

# feat: Collect editor records without a report token

## Overview

Drop `POST /api/v1/collection-report` and `EDITOR_COLLECTION_TOKEN`. The collector stops reporting back to the editor and writes a per-record table to the run's job summary instead; RDU read collection state on GitHub, which `/collection` links to per project. Approving a project always supersedes its earlier approved record. The editor then holds no secret for CI, Infra vaults one value fewer, and most of DEV-7544's `collection` renames disappear.

Source: the proposal [Editor collection without a report token](https://claude.ai/artifact/Q39GNBRdGYm64oWU3bPECu), mirrored in DEV-7595. The skip-rule fix it depends on (DEV-7576) is on main via #521.

**Deadline:** before the first deployment (DEV-6921). Until then `areas/deposit/editor/store/src/migrations/0001_initial.sql` is edited in place (`areas/deposit/editor/CLAUDE.md`, "A schema change edits `0001` until the first deployment").

**Delivery:** one draft PR in `dasch-specs` (Phase 1) and one PR in `dsp-repository` (Phases 2 to 4), one commit per phase with `allow-many-commits` ticked and a Review Notes section (`docs/src/git-conventions.md`, "One PR, several commits, or a stack"). This plan's own commit is squashed into the Phase 2 commit.

## Problem Statement / Motivation

The report loop exists for PRD REQ-5.7 (per-record failure visible on an RDU surface, `04-feat-editor-collection-endpoint-plan.md` in `dasch-specs/specs/2026-05-08-rdu-metadata-editor/`). It costs a bearer token in two places (GitHub secret, ops-deploy vault), a `PROD` startup check, a CSRF carve-out, five columns, nine record classifications, a `refresh` workflow mode, and a refusal gate on approval. Nothing served or published depends on it (`docs/src/editor/collection.md`, "The invariant"). GitHub already holds the true state; the editor's copy is only as fresh as the last run.

## Proposed Solution

1. **Collector** (`areas/deposit/editor/collector`): stops calling the editor except `GET /api/v1/approved-records`. After each record it appends one row to `$GITHUB_STEP_SUMMARY` (when set): project, record id, outcome (opened, updated, skipped, failed), pull request link, reason. A failed record still fails the run. The `collect`/`refresh` argument goes; the binary takes no argument.
2. **Approval always supersedes**: the approve transaction deletes every earlier approved record of the shortcode. The collector force-pushes the newest record onto an open pull request (existing) or opens a new one after a merge (existing since #521). New: a `new` entity whose file already landed through a merged collection pull request on this project's branch is neither renumbered nor rewritten, so a re-approval after merge-before-release does not duplicate the entity.
3. **`/collection`** keeps only what the startup comparison derives: badge Waiting for release / Published / Project dropped upstream / Payload unreadable / Unexpected comparison; a "Pull requests on GitHub" link built from the stored shortcode; Discard on every row.
4. **Removed**: the report endpoint, `COLLECTION_REPORT_PATH`, its CSRF carve-out, `CollectionToken` and `subtle`, the token in config/settings/state and its `PROD` check, columns `collected_at`, `reported_at`, `pull_request_url`, `pull_request_state`, `last_failure`, repository methods `mark_collected` and `report_collection`, the `collection` member of the approved-records payload, `CollectionReport`, `CollectionStateView`, `PullRequestState::is_live`, the `stranded` reconcile counter, the workflow's `mode` input, token guard and token env.

**Kept**: public `GET /api/v1/approved-records` and its rate limit; skip rule, renumbering and force-push guard; `PullRequestState` (the collector's forge uses it); `EDITOR_BASE_URL` (prod only, never stage) and its workflow guard; `GH_TOKEN`; the CSRF test that enumerates every state-changing path.

## Technical Considerations

- **Link pattern**: `https://github.com/dasch-swiss/dsp-repository/pulls?q=is%3Apr+head%3Aeditor-collect%2F<shortcode>`, built from `record.shortcode` (the stored, lowercase spelling the collector's branch uses, `collector/src/run.rs:116`), never `shortcode_as_published`. One constant replaces `PULL_REQUEST_PREFIX` (`web/src/handlers/collection.rs:48`); the repository slug is duplicated with the collector's `COLLECTOR_REPOSITORY`, and the constant's doc names that coupling.
- **Summary rows**: one pure function renders a row; failure reasons are multi-line `git`/`gh` stderr, so the cell goes through the existing `one_line` (`collector/src/run.rs:429`) and escapes `|`. Rows are appended as each record finishes, so a run that aborts on a listing error keeps the rows it already wrote. The writer takes the path as a parameter so tests need no env var.
- **Opened vs updated**: `Outcome::Published` splits into `Opened { url }` and `Updated { url }` at `collector/src/run.rs:206-218`.
- **Landed entities**: ids from `entity_id_at` over `files` of merged pull requests whose `head_ref` is this record's branch; such `new` entities are excluded from `renumber::plan` and from the write loop (`collector/src/run.rs:159-178`). The project keeps referencing the landed id. An entity the merged pull request renumbered landed under its new id: the collector reads that pull request's renumbering back from a `Renumbered: <from> <to>` body line, written and parsed by a pair beside `record_line`/`record_in` (`collector/src/forge.rs:41-57`), and maps the proposed id to the landed one in the rewrite.
- **Supersede SQL**: `DELETE FROM approved_records WHERE shortcode = ?1` in the same `write` closure as the INSERT (`store/src/review_rounds.rs:113-162`), so there is no window with zero records.
- **Stage database**: stage already carries an earlier `0001`; editing `0001` leaves stage with five unused nullable columns until it is wiped. Harmless: the new code never selects them and `store/src/schema.rs` checks table names only.
- **Not edited**: `docs/specs/2026-10-13-deposit-area-target-architecture/` and `CHANGELOG.md` are records of what was intended or shipped.

## Implementation Phases

Commands (from `eng.yaml`): `just test`, `just check`; E2E `just test-e2e-editor` after `cargo build -p deposit-server --release` and `just css-editor-release`. Reviewers for every phase review: `eng:review:rust-reviewer`, `eng:review:security-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`; add `eng:review:devops-reviewer` (Phase 2, workflow) and `eng:review:accessibility-reviewer`, `eng:review:ivan-reviewer` (Phase 4, `/collection`).

#### Phase 1: Amend the PRD

### dasch-specs

- [x] On branch `specs/dev-7595-collection-without-report` from `origin/main`, amend REQ-5.1 in `specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md`: drop the advisory `collection` block sentence; keep "no state-dependent filter" and "skip derived from GitHub".
- [x] Amend REQ-5.3: an open pull request for the project is updated with the newest record; a merged pull request skips only the record its body names.
- [x] Amend REQ-5.7: a failed record stays and is retried next run; the workflow writes one job-summary row per record as it handles it, with the reason; a failed record fails the run; the editor is not told.
- [x] Add a resolution note under Edge Case 4: the newest approval supersedes; an open pull request is updated; after a merge a new pull request opens on main; a `new` entity already landed through the merged pull request is not created again. Caveat: a re-approval built from a discarded draft drops the earlier approved changes, and the review diff (against the published set) does not show it.
- [x] Grep the PRD for `report`, `collection surface`, `last reported` and fix every other sentence that describes the report loop.
- [x] Commit `docs(specs): drop the collection report from the editor PRD (DEV-7595)`, push, open a draft PR per `eng/references/specs-repo-resolution.md` Section 6.
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: The collector stops reporting and writes a run summary

### dsp-repository

- [x] `collector/src/config.rs`: remove `Mode`, `collection_token`, `EDITOR_COLLECTION_TOKEN`, and tests `a_mode_is_the_one_positional_argument`, `anything_but_the_two_modes_is_refused_by_name`.
- [x] `collector/src/editor.rs`: remove `report()` and the token; `Editor::new(base_url)`.
- [x] `collector/src/run.rs`: remove `refresh`, `newest_for`, `Outcome::Silent`, `report_for`, `Summary.unreported` and the report call in `finish`; remove the four `refresh_*` tests and the `report_for` assertions in `a_record_with_no_publishable_project_becomes_a_failure_report`.
- [x] Split `Outcome::Published` into `Opened` and `Updated`; adjust the skip-rule tests that assert it.
- [x] Add the row renderer and the per-record append to `$GITHUB_STEP_SUMMARY` (header before the first record); unit-test the header, one row per outcome, and a reason containing `|` and a newline.
- [x] Force-push-guard failure reason (`collector/src/run.rs:140-148`) names the remedy: merge or close the pull request, or delete the branch.
- [x] `collector/src/main.rs`: no argument dispatch; exit `FAILURE` when any record failed; stdout line without `unreported`.
- [x] `.github/workflows/collect-editor-records.yml`: remove the `mode` input, the token from guard and env (keep the `EDITOR_BASE_URL` guard), use fixed job and step names, run `cargo run --release -p editor-collector`; rewrite the header, concurrency and guard comments.
- [x] Docs: `docs/src/editor/collection.md` workflow modes, "Running it" table, job-fail rule and the staleness/refresh section (`:121-139`), plus one sentence "dispatch the workflow after approving"; `areas/deposit/editor/CONTEXT.md:52`; `ARCH-MAP.md:220-221`.
- [x] Run `just check`; it passes.
- [x] `just test` passes in CI on PR #522 (not run locally at the user's instruction)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Approval always supersedes

### dsp-repository

- [x] `web/src/handlers/review.rs`: remove the live-pull-request gate (`:472-492`), `APPROVE_REFUSED_COLLECTING`, the `PullRequestState` import, and tests `approve_is_refused_while_an_earlier_records_pull_request_is_live`, `a_storage_failure_reading_approved_records_is_not_the_live_pull_request_refusal`.
- [x] `store/src/test_support.rs`: remove `Faults.approved_records_find_by_shortcode` if nothing else uses it.
- [x] `store/src/review_rounds.rs`: supersede DELETE keyed on shortcode alone; rewrite its comment; replace `test_the_supersede_delete_agrees_with_pull_request_state_is_live` with a test that approving over an earlier record with pull request state none, open, merged and closed leaves exactly the new record.
- [x] `core/src/records.rs`: remove `PullRequestState::is_live`.
- [x] `web/src/handlers/collection.rs`: remove `a_record_reported_closed_keeps_collected_at_and_stops_blocking_a_new_approval`.
- [x] Collector: exclude landed `new` entities from renumbering and writing (Technical Considerations); test: merged pull request on the branch naming record A with `persons/<id>.json`, record B carrying the same `new` entity, expect no renumbering and no entity file written; and the renumbered case: the merged pull request landed `person-417` as `person-418`, record B carries `person-417`, expect no entity file written and the project referencing `person-418`.
- [x] Update the doc comment on `approved_comparison` (`web/src/handlers/projects.rs:161`).
- [x] Docs: `docs/src/editor/collection.md` "One live record per project" (`:111`) and the retirement paragraph (`:39-41`, add the landed-entity rule); `areas/deposit/editor/CONTEXT.md:48` and `:95`.
- [x] `just check` and `just test` pass in CI on PR #522 (not run locally at the user's instruction)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: The editor drops the report

### dsp-repository

- [x] `core/src/status.rs`: `RecordClassification` becomes `Published`, `WaitingForRelease { changed }`, `RemovedUpstream`, `Unreadable`, `Anomalous`; `classify_record` reads no pull request column; replace the PR-state classification tests with one for `WaitingForRelease`.
- [x] `core/src/records.rs`: remove the five columns from `ApprovedRecord`; fix every struct literal (`web/src/handlers/sections.rs`, `review.rs`, `reconcile.rs`, `handlers/collection.rs`, `core/src/status.rs`, `core/src/collection.rs`, store fixtures, `collector/src/run.rs`).
- [x] `core/src/collection.rs`: remove `CollectionStateView`, `CollectionReport` and `ApprovedRecordView.collection`; update `serializes_only_the_allow_listed_keys` to the new key set deliberately.
- [x] `core/src/repository.rs`: remove `mark_collected` and `report_collection`; update `list_all`'s doc to give the no-filter reason without the reported state.
- [x] `store/src/migrations/0001_initial.sql`: remove the five columns and their comments; `store/src/approved_records.rs`: SELECT, INSERT, `map_row`, remove both methods and their tests; `store/src/review_rounds.rs` INSERT; `store/src/test_support.rs` forwarders.
- [x] `web/src/lib.rs`, `shell.rs`, `routes.rs`, `fixtures.rs`: remove `CollectionToken`, `collection_token`, `COLLECTION_REPORT_PATH`, the report route, `state_with_collection_token`; remove `subtle` from `web/Cargo.toml`.
- [x] `web/src/handlers/collection.rs`: remove `report`, `validate`, `bearer_token`, `PULL_REQUEST_PREFIX` and the report tests; `overview` rows carry no reported state; add the link constant.
- [x] `web/src/pages/collection.rs`: columns Shortcode, Project, Approved, Status, Actions; status cell = badge plus "Pull requests on GitHub" link (aria-label names the shortcode); Discard on every row; intro text without the staleness sentence; `confirm_discard` says the pull request is not touched and must be closed on GitHub, and shows the link; fix the `discard()` and `DiscardImpact` docs; replace `the_discard_control_appears_only_on_a_stranded_row_and_is_a_link`; regenerate the `collection_list` snapshot.
- [x] `web/src/reconcile.rs` and `web/src/startup.rs`: remove `stranded` and its log field; `needs_attention` without it; rewrite the stranded test as a waiting test.
- [x] `areas/deposit/server/src/config.rs`, `editor.rs`, `csrf.rs`: remove the token field, blank and `PROD` checks and their tests, the token mapping and `the_collection_token_reaches_the_settings`, the carve-out; keep and reframe `test_every_other_state_changing_path_in_the_app_still_requires_the_header` as "every state-changing path requires the header".
- [x] `web-e2e-tests/playwright.config.ts`: remove `COLLECTION_TOKEN` and `EDITOR_COLLECTION_TOKEN`; `tests/collection.spec.ts`: approve, then `/collection` shows Waiting for release and a GitHub link whose `href` names `editor-collect/<shortcode>`, then Discard on that row removes it; no report POST.
- [x] Docs: `docs/src/editor/collection.md` (invariant, the `collection` member, the report-endpoint section, "What the editor does with a report", discard); `areas/deposit/editor/CLAUDE.md` index line and "Collection holds two invariants" (no filter on the endpoint; the editor holds no credential at all and makes no outbound GitHub call); `docs/src/editor/architecture.md` (diagrams, CI-to-editor boundary); `routing.md` (`:33`, `:37`); `operations.md` (token row, startup log line); `status.md:21`; `areas/deposit/docs/adr/0003-online-is-derived-at-startup.md` (stranded consequence); `areas/deposit/docs/adr/0004-two-capabilities-identity-and-editor.md:65`.
- [x] Run `dune:dune-map` on the editor and `deposit-server` components; `ARCH-MAP.md` reports no stale entry and names no removed item.
- [x] `grep -rn -E 'collection-report|COLLECTION_REPORT|EDITOR_COLLECTION_TOKEN|CollectionToken|CollectionReport|CollectionStateView|pull_request_state|last_failure|collected_at|reported_at|is_live|[Ss]tranded' -- areas docs .github ARCH-MAP.md` returns only `docs/specs/`, `CHANGELOG.md` and unrelated hits (`identity/web/src/handlers/login.rs`, `editor/core/src/submission.rs`, `cpe/core/src/validate/remodel.rs`, project data files).
- [x] `cargo clippy` on the touched crates, `just fmt` and `cargo machete` pass locally.
- [x] `just check` and `just test` pass in CI on PR #522 (not run locally at the user's instruction)
- [x] `just test-e2e-editor` passes in both the JS and no-JS projects in CI (`a11y / editor` on PR #522)
- [ ] `eng:test-browser` on `/collection` and the discard confirmation (skipped at the user's instruction; badge, link and button contrast, reflow and computed styles unmeasured)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Confirm on the dasch-specs PR that collection state in two places (editor list and GitHub, no status badge for the pull request) is acceptable | RDU | after ship | RDU's product decision |
| H2 | Merge the dasch-specs PR, then the dsp-repository PR | Marcin | after ship | Merging is the owner's decision; gated on H1 |
| H3 | Delete the `EDITOR_COLLECTION_TOKEN` repository secret if present | Marcin | after ship | Repository settings |
| H4 | Drop the token from DEV-6921 and tell Infra in the INFRA-1422 handover that the image no longer needs it; drop the obsolete `collection` rows from DEV-7544 | Marcin | after ship | Cross-team communication |

## Finalize

- [x] At the end of finalize (user request): find and remove unnecessary comments and tests that over-test (assert constants, std or implementation details) across the whole PR diff.

## Acceptance Criteria

- [x] No route, config key, env var, column, type or doc in `dsp-repository` (outside `docs/specs/` history and `CHANGELOG.md`) refers to the collection report or its token.
- [x] A collector run writes one job-summary row per record with outcome and link, and fails when any record failed.
- [x] Approving a project with an earlier record in any pull request state succeeds and leaves one record.
- [x] Re-approving after a merge, before release, opens a new pull request without duplicating an already landed `new` entity.
- [x] `/collection` shows each record with a working GitHub link built from the stored shortcode and a Discard control.
- [x] PRD REQ-5.1, REQ-5.3, REQ-5.4, REQ-5.7 and Edge Case 4 describe the shipped mechanism (dasch-specs#244).
- [x] `just check`, `just test` and `just test-e2e-editor` pass.

## Dependencies & Risks

- **Stale merged-with-edits records**: a record whose pull request merged with reviewer edits reads Waiting for release indefinitely; RDU spot it through the link and discard it. Accepted by the proposal; documented in `collection.md`.
- **Stale open pull request**: after re-approval the open pull request holds the older record until someone dispatches the workflow; the docs say to dispatch after approving.
- **Approval during a run**: a run that fetched records before a re-approval publishes the older one; the next run force-pushes the newer one.
- **Sandbox**: Playwright cannot bind its port and `just css-editor-release` may leave a stale stylesheet inside the sandbox (`04-feat-editor-collection-endpoint-plan-journal.md`); run E2E outside it.

## Success Metrics

Every check in Acceptance Criteria passes; the Phase 4 grep returns no live reference.

## References

- Proposal: https://claude.ai/artifact/Q39GNBRdGYm64oWU3bPECu
- Original design and rationale: `dasch-specs/specs/2026-05-08-rdu-metadata-editor/04-feat-editor-collection-endpoint-plan.md` and its journal
- Contract: `docs/src/editor/collection.md`; invariants: `areas/deposit/editor/CLAUDE.md`
- Skip-rule fix: DEV-7576, dasch-swiss/dsp-repository#521
- Learnings: `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md` (Maud edits in `pages/collection.rs`), `dasch-specs/learnings/best-practices/recompute-figures-reread-inherited-claims.md` (doc counts)
