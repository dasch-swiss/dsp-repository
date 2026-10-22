---
plan: docs/specs/2026-10-22-editor-rules-into-core/01-refactor-editor-rules-into-core-plan.md
target_repo: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository
base_commit: 05cab0d9695d8004563295cc9fb5d9543bdcf5c6
branch: feature/dev-7526-editor-move-form-and-business-rules-into-editor-core-and
started: 2026-10-22
problem: >
  DEV-7375 brought the Deposit Area to ADR-0004's shape and deferred four cleanups to DEV-7526: the form rules (registry, obligations, submit validation) sit in editor-web although none of them renders, the submit and approve gate sequences are inline in axum handlers, editor-web carries five near-identical storage_error copies, and RightmostXffKeyExtractor exists as three byte-identical copies. This work moves the rules into editor-core and deduplicates the rest without changing behaviour.
status: complete
---

# Execution Journal: 01-refactor-editor-rules-into-core-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | 05cab0d9 | feature/dev-7526-editor-move-form-and-business-rules-into-editor-core-and | rebase | in-progress | https://github.com/dasch-swiss/dsp-repository/pull/531 |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@ed66bbf3 | 0 |
| 2 | reviewed | dsp-repository@81fb69a4 | 1 |
| 3 | reviewed | dsp-repository@99376beb | 1 |
| 4 | reviewed | dsp-repository@2a7d41e6 | 1 |
| 5 | reviewed | dsp-repository@9d74dfb8 | 1 |
| 6 | reviewed | dsp-repository@4e199424 | 0 |

## Chunk queue

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | editor/web/src/handlers/{mod,approved,entities,projects,review,sections}.rs | — | "one storage_error", "byte-identical", "test count", "check && test" | one fn in handlers/mod.rs, 17 call sites pass their module's const, log and page bytes unchanged | each module's `fn storage_error(` (approved 206, entities 658, projects 179, review 1148, sections 1901); owner: plan § storage_error dedup | — |
| 2.1 | dsp-repository | shared/rate-limit/**, Cargo.toml, Cargo.lock, access/dpe/server/src/{router,lib}.rs, access/server/src/serve.rs, editor/web/src/{routes,lib}.rs, identity/web/src/routes.rs, deposit/server/src/router.rs, consumer Cargo.toml files | 1.1 | crate exists, five consumers, copies gone, test count −14 | one `struct RightmostXffKeyExtractor`, in shared-rate-limit with its 7 tests | dpe router.rs:32 + tests 525; editor routes.rs:38 + tests 192; identity routes.rs:30 + tests 143; serve.rs:79; deposit router.rs:54; owner: plan § shared-rate-limit | — |
| 2.2 | dsp-repository | shared/README.md, docs/src/repo_structure.md, docs/src/git-conventions.md, CONVENTIONS.md, ARCH-MAP.md, areas/access/server/CLAUDE.md, areas/deposit/server/CLAUDE.md, eng.yaml | — | docs name the crate, ARCH-MAP section, CLAUDE.md boundaries, eng.yaml binding | every registration point names shared-rate-limit; extractor no longer listed in dpe-server/editor_web/deposit-server surfaces | owner: plan § Technical Considerations, registration points | — |
| 3.1 | dsp-repository | editor/core/src/{form.rs,form/*,multilingual.rs}, editor/core/tests/untouched_form_round_trip.rs, editor/core/Cargo.toml, editor/web/src/** (form/mod.rs, widgets.rs, pages, entity, handlers), editor/web/tests/depositor_vocabulary.rs, editor/web/Cargo.toml | 2.1 (runs during Phase 2 review) | registry/obligation/submit moved, multilingual helpers, round-trip test, editor-web paths, core docs rewritten, rustdoc, test count, check && test, e2e | editor_core::form::{registry,obligation,submit} via git mv; editor-web compiles against them; no re-export shim | web/src/form/{registry,obligation,submit,widgets,mod}.rs; core/src/form.rs 12-16, 31, 300; core/src/review.rs 6; owner: plan § Form rules into editor-core | — |
| 3.2 | dsp-repository | areas/deposit/editor/CLAUDE.md, ARCH-MAP.md, docs/src/editor/{architecture,project-form,entity-proposals}.md, areas/deposit/editor/CONTEXT.md | 2.2 | docs place the registry in editor-core | docs place registry, obligations and submit validation in editor-core | owner: plan Phase 3 checklist | — |
| 3.3 | dsp-repository | areas/deposit/editor/CLAUDE.md, docs/src/editor/{architecture,project-form,entity-proposals}.md, areas/deposit/editor/CONTEXT.md | — | 3.2's docs minus ARCH-MAP | as 3.2, for these files | as 3.2 | 3.2 |
| 3.4 | dsp-repository | ARCH-MAP.md | 2.1 (Phase 2 committed: 2.2 has uncommitted hunks in ARCH-MAP) | 3.2's ARCH-MAP lines | ARCH-MAP places registry, obligations and submit validation in editor-core | ARCH-MAP 193-195, 207, 230, 232, 260-261, 600 | 3.2 |
| 4.1 | dsp-repository | editor/core/src/submission.rs, editor/web/src/handlers/sections.rs | Phase 3 | check A-G, gate D parse, gate B test, gate D unreadable test | `editor_core::submission::check` returns the first refusal in order A-G; `submit` maps it through one match to today's texts, outcomes and log fields | sections.rs `submit` 1042-1256, context agents 495; owner: plan § Business rules into editor-core | — |
| 4.2 | dsp-repository | docs/src/editor/project-form.md, ARCH-MAP.md, areas/deposit/editor/CONTEXT.md, docs/src/editor/architecture.md | — (parallel with 4.1, docs only) | docs say the submit gates live in editor_core::submission | project-form.md § Submit lists the gates in code order and names `check` | project-form.md 152-177 (gate order there is wrong: lists to_raw before obligations) | — |

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 81fb69a4 | one storage_error in handlers/mod.rs taking a per-module StorageSurface const; 17 call sites; log and page literals unchanged | none |
| 2.2 | dsp-repository | complete | 99376beb | registered shared-rate-limit in README, repo_structure, commit-scope lists, ARCH-MAP, root CONTEXT.md, server CLAUDE.md boundaries, eng.yaml | none |
| 2.1 | dsp-repository | complete | 99376beb | shared/rate-limit crate with the extractor and its 7 tests; 5 consumers depend on it directly; 3 copies and 2 re-exports gone; SIPI cross-reference kept from the dpe copy | none |
| 2.3 | dsp-repository | complete | d74126b2, 60a0742b, 2ebe0ad9, 735aa1a4 | review fix: shared-rate-limit dependency moved above the shared-telemetry comment it had split (identity-web, deposit-server); architecture.md names shared-rate-limit; crate doc no longer narrates the refactor; ARCH-MAP rule label split (extractor logic: static-analysis; every limiter uses it: review); project_structure.md graph names the crate; editor-web manifest pending 3.1 | none |
| 3.1 | dsp-repository | complete | 8020e0c9 | registry, obligation, submit moved to editor_core::form (renames detected), language-map helpers to editor_core::multilingual, round-trip test to core/tests; editor-web paths direct, no shim; core docs rewritten; editor-core+web+deposit-server test count 715 before and after; no new rustdoc warnings | none |
| 3.3 | dsp-repository | complete | 8020e0c9 | editor CLAUDE.md, CONTEXT.md, architecture.md, project-form.md place the registry in editor-core; entity-proposals.md needed no edit (relative form::submit paths) | none |
| 3.4 | dsp-repository | complete | 8020e0c9 | ARCH-MAP editor Purpose, Key entities, kits and boundary rule name editor_core::form::registry | none |
| 3.5 | dsp-repository | complete | 377fc86f | review fix: form.rs no longer claims control and decoder are declared together; as_multilingual pub rationale names editor-web; ARCH-MAP Key entities resolve (form::Shape, editor_web::form::INTENT) and drop "still"; ADR-0002 and two sections.rs comments point at editor-core's round-trip test | none |
| 3.6 | dsp-repository | complete | f6c80110 | ARCH-MAP fingerprints refreshed for access/server, deposit/identity, deposit/server and the new shared/rate-limit (stale only through this branch) | none |
| 3.7 | dsp-repository | complete | 255ccd63 | ARCH-MAP dpe entry re-verified against the code: ARK resolver and / and /dpe redirects added to its routes, routing rule names the rate-limited sub-router; fingerprint refreshed | none |
| 4.1 | dsp-repository | complete | 9d74dfb8 | editor_core::submission::check runs gates A-G and returns SubmitRefusal / ProposalProblem{Unreadable, Finding}; submit maps it through one exhaustive match; literals identical (82 of 82); core tests for gate B and gate D unreadable | none |
| 4.2 | dsp-repository | complete | 9d74dfb8 | project-form.md lists the gates in code order (it had to_raw first) and names check; ARCH-MAP, editor CONTEXT.md, architecture.md place the submit gates in editor_core::submission | none |
| 4.3 | dsp-repository | complete | 00da8ddf (fixup) | review fixes: core tests pin obligation-before-conversion and agent-before-proposal; proposal_fault_message extracted; comments, to_raw rustdoc, form/submit.rs doc and project-form.md name check; ARCH-MAP key entities name check and SubmitRefusal | none |
| 5.1 | dsp-repository | complete | c3a1291e | SubmissionState::is_pending (exhaustive match) used by pending_submission, the review queue and editor-store open work; proposal_for in editor_core::proposals with a tie-break test | none |
| 6.1 | dsp-repository | complete | 8796251f | editor_core::review::approval runs approve gates 2-5 (ApproveRefusal); ReviewOutcome::requires_note is gate 1; decided_draft, decisions and DECISION_PREFIX in editor_core::review; literals identical (37 of 37), no assertion changed | none |
| 6.2 | dsp-repository | complete | 8796251f | review.md, entity-proposals.md, ARCH-MAP, architecture.md, editor CONTEXT.md place the approve gates in editor_core::review | none |
| 6.3 | dsp-repository | complete | 8242ba5b, d162bb6c (fixups) | review fixes: core review.rs module header, approval/decided_draft/substitute docs trimmed, Context.agents doc no longer points at the gate; ARCH-MAP fingerprints refreshed for access/dpe, access/server, deposit/editor, deposit/identity, shared/rate-limit | none |
| C.1 | dsp-repository | complete | 274b413f, ba814405, b45a0db5, 2272557c (fixups) | cleanup pass (user-approved): redundant comments trimmed in Phases 2 to 5, duplicate obligation-before-conversion core test removed | none |

## Deferrals

- Stop gate (Phase 6, after cleanup and delete-and-run): `just check` green; tests of editor-*, identity-*, deposit-server, deposit-shell, dpe-server, access-server, shared-rate-limit 1249 passed, 0 failed (user decision: the rest of the workspace is left to CI); `just test-e2e-editor` 73 passed, 4 skipped by design; `dev_reload` (dpe-server `--features dev`, outside the sandbox) passed; `cargo doc` no new warnings. Test count not measured (needs every workspace test binary): derived 6617 + 4 (gate B, gate D unreadable, agent-before-proposal, proposal_for tie-break) = 6621.
- User decision 2026-10-23: no per-phase test runs. Per commit only `cargo check`; once implementation is done, a cleanup pass removes unnecessary comments and over-testing across Phases 1 to 6, then delete-and-run, then one test run and the full gate. The scoped tests last ran on the rebased Phase 4 tip (9d74dfb8): 801 passed, 0 failed.
- Phases 4 to 6 (user direction: no waiting between phases): delete-and-run for Phase 4 runs together with Phase 6's after the Phase 6 commit, sequentially and as the only cargo job; a gate none catches gets a core test as a fixup of its phase. Per-phase `just check`, `just test` and `just test-e2e-editor` fold into the one gate at the stop; per commit: `cargo check` plus tests of editor-core, editor-web, editor-store and deposit-server.
- Deferred to Phase 6: the `areas/deposit/editor` ARCH-MAP fingerprint (checkbox added to Phase 6). Deferred to the final review: eng:test-browser. Nothing else from Phases 1 to 3.
- Stop gate at 377fc86f (covers Phases 1 and 3): `just check` green; every workspace test suite green; `dev_reload` green outside the sandbox; `just test-e2e-editor` 73 passed, 4 skipped by design (viewport per project; the image section has no repeatable field); `cargo doc` no new warnings; test count 6617, derived (no test attribute changes in Phase 3; editor-core+web+deposit-server 715 before and after).
- ARCH-MAP fingerprints at this stop (user decision): the four components only this branch made stale are refreshed, dpe after re-verification; `areas/deposit/editor` waits for the Phase 6 stop (Phases 4 to 6 change it again); `modules/mosaic` is stale from main (#529), not touched here.
- Phase 2 gate at 99376beb: `just check` green; test count 6617 (baseline 6631 minus 14); every workspace test suite green; `dpe-server` `dev_reload::asset_change_emits_reload_event` fails inside the agent sandbox (no file-watcher events), also at 81fb69a4 where dpe-server is unchanged, and passes outside it.
- Phase 1 test re-run stopped at user request (2026-10-22) to free the cargo lock: `just check` green at 81fb69a4; 24 + 27 test suites passed, 0 failed, before the stop; the stop gate runs every suite at the tip. Phase 1 test count derived, not measured: its diff changes no test attribute, and Phase 2 (6617 measured) removes exactly 14.
- Phase 3 gate (user decision 2026-10-22): no separate per-phase gate; one gate at the stop over the whole stack covers Phase 3 (`just check`, `just test` minus dsp-cli, test count, `cargo doc`, `just test-e2e-editor`).
- dsp-cli HTTP suites (`data_model_describe_http` and other wiremock tests): cannot bind a local port in the agent sandbox; not run locally, CI covers them; dsp-cli depends on no crate this branch changes.
- eng:test-browser (eng.yaml `areas/*/*/web/**`, offer: ask): deferred to the final review at intake; phases 1-3 change no markup.
- ARCH-MAP fingerprints (dune P1 DUNE-001, Warning): recomputed once with `/dune:dune-map update` after Phase 3, not per phase, since every phase changes the editor fingerprint again; covers editor, dpe-server, access-server, identity-web, deposit-server and the new shared/rate-limit section (none yet).

## Deviations

- none

## Side findings

- Delete-and-run (Phases 4 and 6, after the cleanup): every gate caught by at least one existing test. Submit A 2, C 2, D (finding branch) 1, E 1, F 1, G 3; approve 1 2, 2 1, 3 1, 4 1, 5 1; extra: gate 4's scope without the Reject filter 1 (approve_is_refused_when_the_payload_still_references_a_rejected_proposal). No test added for it.
- Phase 6 review: no Critical or Warning beyond the module header and the Context.agents doc, both fixed. Not taken: approval returning the decided draft (changes the plan's signature), core unit tests for approval (delete-and-run shows every gate covered by handler tests).
- Phase 5 review: entities.rs doc link (rustfmt would rewrap it) and the is_pending doc fixed; owners named in CONTEXT.md and ARCH-MAP. Not taken: a test tying context()'s lock to is_pending, rewording identity-ports prose.
- ARCH-MAP: modules/mosaic and dsp-cli are stale from main, not touched here.
- PR #531 conflicted with main after #527/#529 merged and main gained 7 commits (three ADR rewrites, CPE activation): rebased onto origin/main e9d6261b; conflicts in ARCH-MAP (dpe interface, access-server Depends on, four fingerprints taken from main), CONVENTIONS.md and git-conventions.md scope lists, access/server CLAUDE.md, ADR-0002 (editor). ADR-0004 (`deposit-server`'s dependencies) and root ADR-0002 (shared crates) did not name shared-rate-limit; fixed in the Phase 2 commit. The four ARCH-MAP fingerprints taken from main are stale until the Phase 6 stop's dune-map update.
- Phase 4 review (consistency, simplicity, dune, rust, security, ivan): no Critical or Warning beyond simplicity W1 and DUNE-001, both fixed in 00da8ddf. Not taken: AgentScope built twice (plan contract: check builds it), seven positional parameters, trimming the module doc's gate list, narrowing unresolved_temporal_coverage. accessibility-reviewer (eng.yaml) not run: no markup or rendered string changes, literals identical.
- History autosquashed at the Phase 3 stop: Phases 1 to 3 are now 7c0f6747, 48f4cb0e, 4d94d2f6 on main (parents #527, #529 merged); chunk SHAs above predate it.
- Phase 3 review: no Critical; Warnings fixed in 377fc86f. Follow-ups (not behaviour-preserving, out of scope): the Value to DraftMultilingual conversion exists three times in editor-core (`ProjectDraft::multilingual`, `as_multilingual` default on parse failure; `apply_multilingual_rows` drops the row), worth one helper with an explicit failure policy. Suggestions not taken: comment trims, `#[must_use]` on the moved helpers, a drift-guard line on the `Shape` enum doc, merging the identical ReferenceRows and TextOrReferenceRows arms in form::submit.
- Follow-up (not in plan scope, dune P2 DUNE-001): a clippy `disallowed-types` entry for `tower_governor::key_extractor::SmartIpKeyExtractor` would make "every per-IP limiter keys on RightmostXffKeyExtractor" static-analysis instead of review.
- Phase 1 review (consistency, simplicity, dune, rust, security+ivan+a11y): no Critical; one Warning (DUNE-001, deferred above); Suggestions not taken: `&StorageSurface` or `derive(Copy)`, a "deliberate copy" note on identity-web`s storage_error, a "one SURFACE per module" doc line.
- Root `CONTEXT.md` lists the shared crates; the plan did not name it as a registration point for shared/rate-limit. Added in 2.2.
- `dpe-server-fuzz` (own workspace) depends on dpe-core, not dpe-server, so it needs no shared-rate-limit entry.

## Closeout

- **root_cause:** the editor's rules grew inside the axum handlers and the web crate: the form registry sat in editor-web although none of it renders, the submit and approve gate sequences were inline in `submit` and `finish`, and two refactors left five `storage_error` copies and three `RightmostXffKeyExtractor` copies.
- **investigation:** gate order is itself a rule (the obligation gate must precede `to_raw`), so each sequence moved as one synchronous function returning its first refusal as data, with the handler keeping wording, spans and logs. The trap was the inputs: the approve gate's reference scope must come from the published agents plus the non-rejected proposals, never the page's own scope. project-form.md had listed the submit gates in the wrong order.
- **solution:** `editor_core::form::{registry, obligation, submit}`, `editor_core::submission::check`, `SubmissionState::is_pending`, `editor_core::proposals::proposal_for`, `editor_core::review::{approval, decided_draft, decisions}` and `ReviewOutcome::requires_note`; one `storage_error`; the `shared-rate-limit` crate. Literals and test assertions unchanged.
- **prevention:** exhaustive matches on `SubmitRefusal`, `ApproveRefusal`, `is_pending` and `requires_note`, so a new gate or state fails to compile until handled; delete-and-run shows each gate caught by a test; core tests for gate B, gate D's unreadable payload, the agent-before-proposal order and the `proposal_for` tie-break.

