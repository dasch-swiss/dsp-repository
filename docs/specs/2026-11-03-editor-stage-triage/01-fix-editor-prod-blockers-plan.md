---
title: "fix: editor prod blockers from the 2026-10-28 stage test"
type: fix
date: 2026-11-03
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7711
  - name: dasch-specs
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dasch-specs
linear: DEV-7711
linear_project: 48e9fd8122fa
---

# fix: editor prod blockers from the 2026-10-28 stage test

Group A of the [stage-testing triage update](https://linear.app/dasch/project/metadata-editor-for-rdu-depositing-projects-48e9fd8122fa/activity#project-update-0c95bb2e) (2026-11-02), with the decisions from its comment thread. One phase per issue. Sibling plans in this folder: `02-` (group B, runs in parallel), `03-` and `04-` (group C, after A and B merge).

## Overview

Four issues stand between the editor and its first prod deploy (target Fri 2026-11-06):

| Phase | Issue | Change |
|---|---|---|
| 1 | [DEV-7711](https://linear.app/dasch/issue/DEV-7711) | A value with a format is never emptied; a format problem shows as an inline error at its control, the same on Save, Add and Submit |
| 2 | [DEV-7685](https://linear.app/dasch/issue/DEV-7685) | Link and DOI rules plugged into that error path |
| 3 | [DEV-7695](https://linear.app/dasch/issue/DEV-7695) | RDU creates a new project (shortcode + name); it is listed, editable, submittable and collected like a published one |
| 4 | [DEV-7677](https://linear.app/dasch/issue/DEV-7677) | Depositor form hints, and a warning on a shortcode that is neither published nor created |

Out of this plan, settled elsewhere:

- **DEV-7618** (release workflow, freeze `0001`) runs after this PR and group B's PR merge, with `eng:working` against the issue itself (H2). Its schema question is answered: option A, freeze both `0001` files at the first prod deploy (Marcin's pick, not objected to in the thread; Balduin asked only for the downsides, which the issue lists).
- **DEV-7697** is settled as process (release on demand, at most 2-4 h after approval); its wording lands in group B (DEV-7682).
- **DEV-7680, DEV-7681, DEV-7688** are fixed and wait for Daniela's re-test (H1).
- **DEV-7676** (framing headers) merged on 2026-11-02; nothing left.
- **DEV-7677** is not in the update; it is phase 4 here because its warning depends on phase 3's definition of a known project.

## Problem Statement / Motivation

- **DEV-7711.** A Link holding `Basel` survives Save but is emptied by Add, without a message; the form has no inline errors for format problems at all. This contradicts REQ-1.9 (a draft keeps invalid values) and loses depositor input.
- **DEV-7685.** Link and DOI fields accept `www.dasch.swiss` and `doi.asdasd..asdasd`; a broken link reaches the public page.
- **DEV-7695.** "Creating a new project is a requirement" (Balduin, 2026-11-02). Today RDU can assign a depositor an unpublished shortcode, but the depositor sees nothing to fill in. The PRD has no requirement for it; REQ-2.3 already assumes the case ("a project present only locally is new and unpublished").
- **DEV-7677.** A mistyped shortcode on `/depositors` is accepted silently; with project creation, the warning becomes "this shortcode is neither published nor created".

## Proposed Solution

### Phases 1-2: one format-check path

- A **format rule** is a pure function in `editor-core` (`editor_core::form::format`, new module) per value kind: `Link` (absolute `http`/`https` URL with a host, parsed with the `url` crate if already in `Cargo.lock`, else a strict hand check) and `Doi` (`10.<registrant>/<suffix>`, or that behind `https://doi.org/` / `https://dx.doi.org/`).
- **Draft writes never drop a non-empty value for a format reason.** Save, Add, Remove and autosave store the value as typed (REQ-1.9). Findings are computed from the draft at render time, in `view()`, as a pure function of the stored values: the plain-POST path (a `303` then a GET, `handlers/sections.rs:1539-1547`, whose `show` renders no errors, `:540-546`) and the Datastar path then show the same errors, and the write path carries nothing. A finding is keyed by posted row key when rendering from a body and by stored position on a GET, because blank rows are dropped on store (`core/src/form.rs:526`, `:950`) and an index-keyed finding would land on the wrong row. The autosave re-render (`data-on:change__debounce.1s`, `pages/section.rs:645`, no intent) adds no finding, so a half-typed address is not flagged mid-typing; Save, Add, Remove, Submit and a GET show them. The mosaic `text_field` and `textarea` tiles already render `error` with `aria-describedby` and `aria-invalid` (`text_field/mod.rs:120`, `:213-214`; `textarea/mod.rs:125-126`); nothing to add there. Field and error ids stay stable across renders, since the morph matches by id, and a finding is never wrapped in a `Danger` alert, which carries `role="alert"` inside the polite status region (`docs/src/editor/rendering.md`).
- **Submit refuses** a draft with a format finding, as a new gate in `editor_core::submission::check` with per-field errors, like the existing `Invalid` refusal.
- **Published data must keep passing.** 122 committed string values are `http://` on `6bc64dd3` (walking every string in `areas/access/dpe/server/data/projects/*.json`: discipline references 57, temporal coverage 36, spatial coverage 19, project `url` 3, publication PIDs 3, funding 2, and 2 `legalInfo` licence URIs, which are display-only, REQ-1.5, and outside the rule). So the rule accepts `http` and `https`; the hint recommends https. Two references in `0110_h-steiner.json` (`:54`, `:60`) hold `"url": "MISSING"`, the corpus placeholder: a stored placeholder passes the format rule (the sentinel gate in `submission::check`, `core/src/submission.rs:134-142`, already refuses a typed one), and 0110 is a named test case. The submit gate is scoped by the viewer's audience (`check` takes `audience`, `:88`), so a depositor is never refused for an RDU-only field they cannot see. A test runs every committed project through the rules and requires zero findings, or the rule would block RDU from submitting unrelated changes to those projects.

### Phase 3: a created project is a registration plus a seeded draft

From the thread: RDU only; RDU enters only what researchers cannot know; the project lives in the editor's database from creation and reaches the repository through the normal collection pull request once approved. The prototype's `NewProjectModal` (`dsp-incubator/metadata-editor-v2/src/workflows/screen-projects.jsx`) is the UX: shortcode with live availability, assign to a depositor, "the PID is minted by DaSCH".

- **Fields at creation:** shortcode (`shared_metadata::is_valid_shortcode`, `shared/metadata/src/project.rs:32`; not published, not already created, and with no draft, submission or approved record, since any assigned depositor can already save a blank draft for an assigned shortcode, `handlers/sections.rs:314-336`, and a last-write-wins seed would overwrite it) and name. `id` = shortcode (holds for all 85 published files) and `pid` = `https://ark.dasch.swiss/ark:/72163/1/<shortcode>` (holds for 80: the five `0801a`–`0801e` sub-projects share `0801`'s ARK, which a new shortcode never does) are derived; the ARK string is derived, not minted, so it resolves only once DSP knows the project. The registration is keyed by `normalize_shortcode`, as the published check folds case (`core/src/published.rs:152`). Balduin's "shortname" has no member in `ProjectRaw`, so it is not collected.
- **Registration table** `created_projects` (shortcode PK, name, created_by, created_at), added to `editor-store`'s `0001_initial.sql` in place (pre-prod rule in `areas/deposit/editor/CLAUDE.md`). Not a draft row alone: "Discard draft" would delete the project, and `status::was_published` reads a non-empty `id`/`pid` as "published once", which would classify the new project `RemovedUpstream`.
- **Classification:** `depositor_state` needs no change (an approved record gives Approved, `core/src/status.rs:243-251`; none gives Draft, `:259-262`). The fault is `classify_record(record, published)` (`:190`): a seeded `id`/`pid` makes `was_published` true (`:209`), so a created project's approved record classifies `RemovedUpstream`, which the RDU approved page shows as "Project dropped upstream" (`pages/approved.rs:109`) and startup counts as needing attention. `classify_record` gains the created set (three call sites: `web/src/reconcile.rs:73`, `handlers/approved.rs:93`, `:153`, plus tests); a created shortcode classifies `WaitingForRelease` with every member changed; `Comparison::classify` is untouched. Once the published set carries the shortcode, a startup sweep over `created_projects` deletes the registration; it is its own pass, since `reconcile_published` walks approved records only (`reconcile.rs:63-73`) and a project published by a hand-merged file, or never submitted, has none. The journey test asserts what the depositor reads after release (Online, not Draft: confirm against `depositor_state`, `status.rs:253-262`, since approval keeps the draft row and only Discard deletes it).
- **Listing:** RDU `/projects` gets a "Create a project" action and a "New projects" group above the published list; creating lands on the new project's overview. A created project with no submission or record can be removed by RDU from "New projects" (a mistyped shortcode otherwise counts as known for `/depositors` and needs database surgery once `0001` is frozen); rename is deferred. A depositor's assigned created project appears in "Your projects". The depositor message for an assigned shortcode that is neither published nor created is reworded in plain language (the third row of DEV-7693's table; group B leaves `pages/projects.rs` to this phase).
- **Discard draft** on a created project reseeds shortcode, `id` and `pid` instead of leaving an empty draft (the name is a creation-time seed only, so a renamed project is not reset), with its own confirmation copy and notice condition: today's "discarded" notice requires `record.is_none()` (`handlers/sections.rs:564`), `may_discard` is `record.is_some()` (`:1847`), and the copy promises a re-open from published metadata.
- **Collection:** `editor-pull-requests` already writes a new project to `projects/{shortcode}_{slug(name)}.json` (`pull-requests/src/layout.rs:23`); a test covers a created project end to end (approved record → canonical file → manifest).
- **PRD** (dasch-specs): a requirement for project creation, the pilot scope, and REQ-2.6's wording for release on demand.

### Phase 4: depositor form

As DEV-7677 specifies, with the predicate widened to "published or created". Not the sync `bool` the issue sketches: `deposit-server` may not depend on `editor-core` (`check-composition-root-deps.sh`), `Identity::start` runs before `Editor::start` (`server/src/serve.rs:81`, `:109`), which is where the published set is loaded (`editor/web/src/lib.rs:49`), and a sync call cannot read `created_projects`. It follows the `OpenWork` seam instead: an async known-project method on `identity_ports::OpenWork` (`identity/ports/src/lib.rs:57`) or a sibling trait, implemented by `editor-store`'s `LiveOpenWork` (`editor/store/src/open_work.rs:14`), which reads `created_projects` from SQLite and loads the published shortcode set itself through `editor-core` (store → core and store → `identity-ports` are allowed edges, `check-capability-deps.sh:99`), wired at `serve.rs:84`; `FakeOpenWork` (`store/src/test_support.rs:302`) updated. Group C's review-queue count (`04-` phase 5) rides on the same seam. Create and update redirect straight to `/depositors` (`identity/web/src/handlers/depositors.rs:213`, `:309`) and identity has no notice marker, so the warning is recomputed per account on the list and edit pages from the stored shortcodes, which also covers accounts created before this change. The monospace gap in the shortcode hint is fixed (Daniela's comment).

## Alternative Approaches Considered

- **Draft row as the registration** (no new table): cheapest, but discard deletes the project and `was_published` misclassifies it (above).
- **Create through a pull request first** (a stub file in git): Balduin ruled it unnecessary for two or three members, and the depositor would wait for a release before seeing the project.
- **Strict `https://` rule** (as DEV-7711's suggested message says): blocks 120 committed values.

## Technical Considerations

- **Reproduce before fixing (DEV-7711).** Reading the code, Save and Add both store `Basel` (`handlers/sections.rs:705`, `:996`, both through `apply_and_store`); the difference is the re-render (`keep_posted: true` on Add). Candidates: the Datastar morph of the region, a posted `ref.type` mismatch, or a stage-only difference. Phase 1 starts with a failing Playwright test in both passes (JS and no-JS).
- **Error rendering today** is an `alert` beside the field for submit refusals only (`pages/section.rs:655`), field-granular with no row address (`:386-400`). Phase 1 moves format findings onto the control, keyed by row; submit refusals keep their current place.
- **Other drop paths stay.** `apply_publication_rows` drops a row with a DOI but no citation (`core/src/form.rs:970`), `apply_funding` a grant without a funder (`:1030`), `reference_row` a type not offered (`:924`), pinned by `an_explicit_save_still_drops_a_row_with_nothing_storable` (`handlers/sections.rs:2713`). The acceptance is format reasons only; the phase 1 reproduction runs per reference row shape (spatial, temporal, disciplines, publications, funding), not only spatial coverage.
- **Datastar row controls** post `evt.submitter?.formAction` (`areas/deposit/editor/CLAUDE.md`, Common Pitfalls); any new control uses `datastar::post(action).submitter_first()`.
- **Depositor vocabulary** (REQ-2.2): new depositor-facing strings pass `web/tests/depositor_vocabulary.rs`.
- **Capability gate:** `check-capability-deps.sh` keeps `identity-*` off editor crates (phase 4).
- **Schema:** `0001` is edited in place until DEV-7618 freezes it; the freeze lands after this PR, so phase 3's table is part of the frozen baseline. Stage keeps its volume across deploys and the `user_version` guard never re-runs `0001` (`editor/store/src/schema.rs:42-61`), so this PR's stage deploy fails on its first `created_projects` query until the volume is wiped (H5).
- **Sandbox:** CSS rebuild, Playwright and `just test` (dsp-cli wiremock) fail inside the sandbox; run them outside it, and confirm the stylesheet is fresh before trusting an axe result (`04-feat-editor-collection-endpoint-plan-journal.md` in dasch-specs).

Bindings (`eng.yaml`): test `just test`, lint `just check`, gate `just check && just test`; conventions `CLAUDE.md`, `CONVENTIONS.md`, `areas/deposit/editor/CLAUDE.md`, `areas/deposit/identity/CLAUDE.md`, `areas/deposit/server/CLAUDE.md`, `modules/mosaic/CLAUDE.md`. Reviewers for every phase review: `eng:review:security-reviewer`, `eng:review:accessibility-reviewer`, `eng:review:ivan-reviewer`, `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`. Verify skill: `eng:test-browser` on the changed pages.

## Execution and Delivery

- **One PR in dsp-repository**, branch `fix/dev-7711-editor-prod-blockers`, one commit per phase (`allow-many-commits` ticked, a Review Notes table mapping commit → issue), plus the plan commit first. Keep one commit per issue: each closes its own Linear issue and must stay revertable on its own. Review fixes land as `fixup!` of their phase's commit; the final phase autosquashes them.
- **One PR in dasch-specs** for the PRD amendment, branch `specs/dev-7695-editor-project-creation` from `origin/main`.
- **Runs in parallel with group B** (`02-`, its own worktree and session). `pages/projects.rs` is this plan's alone and B does not touch it. Both plans edit `pages/section.rs`, `handlers/sections.rs`, `form/widgets.rs` and `core/src/status.rs` in different regions (A: the error rendering at `section.rs:655`, `apply_and_store`, the format hooks in the row widgets, the classification; B: the status boxes, the add box, row labels, `value_display`, the wait string). No semantic dependency either way; whichever PR merges second rebases on `origin/main` and resolves textual conflicts in those four files.
- **Overlap inside the session:** phase 4 (identity + server) is file-disjoint from phases 1-3 and may run alongside phase 1. Phase 3's PRD chunk runs alongside anything. Phases 1 → 2 are sequential (2 plugs into 1), and 3 follows 2 because both edit `handlers/sections.rs`.
- **Tests per phase are targeted:** the touched crates with `cargo test -p <crate>…` and the phase's own Playwright spec. The full gate and the full E2E suite run once, in phase 5.
- **Push once,** at ship: every push to an open PR re-runs the full `test` matrix (stable + beta) and the editor a11y E2E job.

## Implementation Phases

#### Phase 1: Never empty a value; inline format errors (DEV-7711)

### dsp-repository
- [x] Add a Playwright test in both passes reproducing DEV-7711's steps (Spatial coverage Link `Basel`, then Add) that fails on the current code, in `areas/deposit/editor/web-e2e-tests/tests/row-controls.spec.ts`, repeated for the temporal, discipline, publication and funding row shapes; it captures the POST body (`page.waitForRequest`) so "never posted" and "posted, then dropped in `apply_and_store`" are told apart before the handler is touched
- [x] Fix the cause found by that test, so Add, Remove, Save and autosave all keep a non-empty typed value
- [x] Add `editor_core::form::format` with the finding type and a registry hook naming each field member's value kind (no rules yet beyond an empty set)
- [x] Compute format findings in `view()` from the draft, keyed by posted row key from a body and by stored position on a GET; `apply_and_store` keeps storing values as typed and carries nothing; the autosave re-render adds no finding
- [x] Render a finding as an inline error on its control through the tiles' existing `error` builder (`aria-invalid` and `aria-describedby` come with it)
- [x] Add a submit gate in `editor_core::submission::check` that refuses a draft with a format finding, with per-field errors, scoped to the viewer's audience
- [x] Rendering tests: a finding is announced with its control; a finding lands on the right row when a blank row precedes it (the Save/Add/Submit parity moved to phase 2, below: with no rule yet, no handler can produce a finding)
- [x] Run `cargo test -p editor-core -p editor-web -p mosaic-tiles` and the new Playwright test (`cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/row-controls.spec.ts`); both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Link and DOI rules (DEV-7685)

### dsp-repository
- [x] Implement the `Link` rule (absolute `http`/`https` URL with a host) and the `Doi` rule (`10.x/…`, bare or behind `https://doi.org/` / `https://dx.doi.org/`)
- [x] Assign `Link` to every link-shaped member (reference `url` of disciplines, temporal and spatial coverage, `documentationMaterial`, `additionalMaterial`, project `url`/`secondaryUrl`, funding `url`, entity `url` and `sameAs`) and `Doi`-or-`Link` to the publication `pid.url`
- [x] Compute and render format findings on the entity form too (`url`, `sameAs`): phase 1 wired them into the section form only, and the entity form passes none (`entity/mod.rs`, `Rows::findings`)
- [x] Handler tests, deferred from phase 1: Save, Add and Submit show the same error for the same value, on the plain-POST path too, the autosave render shows none, and Submit refuses with `SubmitRefusal::Format`
- [x] Hints on those controls name the format ("A full web address, starting with https://"); per-field examples beyond this belong to DEV-7690
- [x] Unit tests: `www.dasch.swiss` and `doi.asdasd..asdasd` are refused with the issue's messages; `https://…`, `http://…`, `10.1234/abcd` and `https://doi.org/10.1234/abcd` pass
- [x] Test that every project under `areas/access/dpe/server/data/projects/` produces zero format findings, with `0110_h-steiner` (stored `MISSING` references) named
- [x] Run `cargo test -p editor-core -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: RDU creates a project (DEV-7695)

### dasch-specs
- [x] Amend `specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md`: a REQ for project creation (RDU only; shortcode and name entered, `id` and `pid` derived; lives in the editor until collected), the pilot scope (published and newly created projects), and REQ-2.6's expected wait under release on demand; a dated `last_revised`

### dsp-repository
- [x] Add `created_projects` to `areas/deposit/editor/store/src/migrations/0001_initial.sql` and `EXPECTED_TABLES`, with a repository port in `editor-core` and its SQLite implementation in `editor-store`
- [x] `GET`/`POST /projects/new` (RDU only, `Rdu` extractor): shortcode with format, published, already-created and existing-draft/submission/record checks, keyed by `normalize_shortcode`, and name; on success the registration and a draft seeded with `shortcode`, `id`, `pid` and `name`, landing on the new project's overview
- [x] RDU `/projects`: "Create a project" action and a "New projects" group listing created, unpublished projects, each removable while it has no submission or record
- [x] Depositor `/projects`: an assigned created project is listed with its state; the message for an assigned shortcode that is neither published nor created reads in plain language
- [x] `classify_record` takes the created set (three call sites) and classifies a created shortcode's record `WaitingForRelease`; a startup sweep deletes a registration once the published set carries the shortcode
- [x] The startup sweep keeps a released created project's draft, deleting only its registration, so edits a depositor saved after approval survive the release; a test covers it (owner decision, 2026-11-03, reverses "and its draft")
- [x] Display-only members (`howToCite`, `legalInfo`) are owned by git for any project the published set carries: `Comparison::classify` skips them, so a released created project whose licence RDU added in the collection PR classifies `Published` (Online, record cleared), and the collection run writes them from the published file when one exists, so a later submission never writes the seeded `[]` or citation back; tests cover both (owner decision, 2026-11-03)
- [x] "Discard draft" on a created project reseeds shortcode, `id` and `pid`, with its own confirmation copy and notice
- [x] Test the collection path for a created project: approved record → `projects/{shortcode}_{slug}.json` in canonical form → corpus manifest entry
- [x] Creation seeds `howToCite` (`<name> [database] (DaSCH), <pid>.`) and `legalInfo` (`[]`), both still display-only, so a created project passes `ProjectDraft::to_raw` at submit; a test over the committed corpus (`areas/access/dpe/server/data/projects/`, run by `just test`) refuses a project with an empty `legalInfo`, naming the file, so a created project's collection PR fails CI until RDU adds its licence (owner decision, 2026-11-03)
- [x] Rendering and handler tests: a depositor cannot reach `/projects/new`; a published shortcode, or one with a registration, draft, submission or record, is refused with a field error; a created project opens, saves and submits; the RDU approved page shows it waiting for release, not dropped upstream
- [x] Update `docs/src/editor/project-representation.md` and `docs/src/editor/status.md` to the as-built behaviour, and drop the "unreachable" note on `Comparison::NewAndUnpublished`
- [x] Run `cargo test -p editor-core -p editor-store -p editor-web -p editor-pull-requests` and `cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/journey.spec.ts`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Depositor form hints and the unknown-shortcode warning (DEV-7677)

### dsp-repository
- [x] Hints on Name and Email address, wired with `aria-describedby` like the shortcode hint
- [x] Fix the wide gap between `0801,` and `080C` in the shortcode hint
- [x] Add the known-project query (published or created) to the `identity_ports::OpenWork` seam, implemented in `editor-store`'s `LiveOpenWork` and wired by `deposit-server`
- [x] Save an account with an unknown shortcode; the list and edit pages name each unknown shortcode per account (there is no post-redirect confirmation surface)
- [x] Tests: the hints are announced with their fields; a known shortcode saves with no warning; an unknown one saves and the list names it
- [x] Run `cargo test -p identity-web -p deposit-server` and `bash .github/scripts/check-capability-deps.sh`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: Clean up and gate

### dsp-repository
- [x] DPE renders a publication's bare DOI (`10.1234/abcd`, accepted by `DoiOrLink`) as a working link, prefixing `https://doi.org/` in the `href` (phase 2's `scope (Warning)` deferral, `areas/access/dpe/web/src/pages/project/`); a test covers a bare DOI and an existing link; lands as `fixup! fix(editor-core): check the format of links and DOIs (DEV-7685)` (owner decision, 2026-11-03)
- [x] A format finding inside the current section shows only inline at its control: the format refusal's per-field errors name only fields outside this section, so no field shows the same problem twice (owner decision 2026-11-03, reverses phase 1's provisional "names every field once"); `fixup!` of the DEV-7711 commit
- [x] Format messages in `widgets::format_message`, owner's wording (2026-11-03): Link "Enter a full web address, such as https://www.dasch.swiss."; DOI or link "Enter a DOI, such as 10.1234/abcd, or a full web address, such as https://doi.org/10.1234/abcd."; data management plan "Enter a full web address, such as https://www.dasch.swiss, or “not accessible”."; hints unchanged; `fixup!` of the DEV-7685 commit
- [x] RDU review and approve re-check format findings, RDU-only fields included, so RDU cannot approve a value the format rules refuse into the collection (owner decision 2026-11-03, phase 1 side finding (a)); `fixup!` of the DEV-7685 commit
- [x] A person or organization proposal refusal names the field in words, not `sameAs[1]: …` (owner decision 2026-11-03); `fixup!` of the DEV-7685 commit
- [x] Revise ADR-0003's decision (`areas/deposit/docs/adr/0003-online-is-derived-at-startup.md`): the startup pass deletes approved records, retires proposals and ends the registration of a created project the published set carries, and writes nothing else; confirm its "the comparison is `editor_core::review::diff` and no second implementation" still holds with display-only members skipped (owner decision 2026-11-03, authorises this edit to the decision); `fixup!` of the DEV-7695 commit
- [x] Depositor form hints, owner's wording (2026-11-03, DEV-7677's meaning): Name "How the editor shows this person, for example in the page header after sign-in."; Email address "The sign-in code is sent here, so use a mailbox the person reads. One account per address."; the unknown-shortcode lines unchanged; `fixup!` of the DEV-7677 commit
- [x] Revise ADR-0004's decision (`areas/deposit/docs/adr/0004-two-capabilities-identity-and-editor.md`): the `OpenWork` port also answers which assigned shortcodes are neither published nor created (`unknown_projects`), beside its two reads (owner decision 2026-11-03, authorises this edit to the decision); `fixup!` of the DEV-7677 commit
- [x] Trim comments added on this branch to the "why not the obvious approach" kind; no narration, no issue history
- [x] Delete tests added on this branch that would fail only if a line were deleted (config values, mirrored implementation)
- [x] Fold every `fixup!` commit into its phase commit (`git rebase -i --autosquash origin/main`, non-interactively with `GIT_SEQUENCE_EDITOR=:`), leaving the plan commit plus one commit per issue
- [x] Run `just fmt`, then `just check` outside the sandbox; it passes. The full `just test`, the release build and `just test-e2e-editor` are left to CI on the draft PR (owner decision 2026-11-03)
- [x] Run `eng:test-browser` on `/projects`, `/projects/new`, a created project's sections and `/depositors`; no defect reported. Playwright fallback on port 4111, owner decision 2026-11-03 (`agent-browser` not installed); results in `_qa-01/`; the fixes after it are covered by the accessibility spec
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Re-test DEV-7680, DEV-7681, DEV-7688 on stage and close them | Daniela | after ship | Acceptance by the tester; independent of this plan's phases |
| H2 | After this PR and group B's PR merge: run DEV-7618 with `eng:working` against the issue (release workflow, freeze `0001`, option A); Infra wipes `/data/db/deposit-stage-01` before the first prod deploy | Marcin, Infra | after ship | Merge order and an Infra-owned volume |
| H3 | Run the release-on-demand process after each approval: open the collection PR; for a created project add its `legalInfo` (licence) and complete the citation's authors and year, which CI requires before merge; merge it, cut a release, tell the project | RDU | after ship | Process, not code; RDU already has review rights (Balduin, answer 6) |
| H4 | Merge the dasch-specs PRD PR | Marcin | after ship | Merging is the owner's decision |
| H5 | Wipe `/data/db/deposit-stage-01` right after this PR's stage deploy and before H1's re-test; tell Daniela her stage drafts and accounts go | Marcin, Infra | after ship | An edited `0001` is skipped by the `user_version` guard, so stage errors on `created_projects` until the volume is fresh; Infra owns the volume |
| H6 | Confirm in the PR that RDU does not expect a "shortname" for a created project (`ProjectRaw` has no such member, so it is not collected); the PR body asks Balduin this as a named question | Balduin | after ship | A product answer only RDU can give |

## Acceptance Criteria

- [ ] A typed value is never removed for a format reason by Save, Add, Remove or autosave; a format problem shows inline at the control, the same on all three actions and with JavaScript off, and blocks Submit only
- [ ] `www.dasch.swiss` and `doi.asdasd..asdasd` are refused with a clear inline message; every committed project passes
- [ ] RDU creates project `1111` with a name; depositor DanielaS (assigned `1111`) sees it, fills it in, submits; RDU approves; the collection writes a new canonical file
- [ ] `/depositors` warns on a shortcode that is neither published nor created
- [ ] `just check && just test` and `just test-e2e-editor` pass

## Dependencies & Risks

- **DEV-7711's cause is unknown** until the reproduction runs; if it is stage-only, phase 1 still ships the format path and records the finding in the PR.
- **Prod date:** phase 3 is the largest; if it runs late, phases 1, 2 and 4 are still worth shipping, but prod waits for phase 3 (it is a requirement, not a nice-to-have).
- **Shortname** is not collected (no `ProjectRaw` member); confirm with Balduin in the PR if RDU expects it.

## Success Metrics

Daniela's re-test of the B30, B9 and M1 steps on stage passes; RDU creates the first pilot project without hand-written JSON.

## References

- Linear thread: [project update](https://linear.app/dasch/project/metadata-editor-for-rdu-depositing-projects-48e9fd8122fa/activity#project-update-0c95bb2e), comments by Balduin Landolt, 2026-11-02
- `areas/deposit/editor/web/src/handlers/sections.rs:588` (`act`), `:812` (`apply_and_store`), `:934` (`row_action`), `:1677` (`refused_with`)
- `areas/deposit/editor/core/src/form.rs:921` (`reference_row`), `core/src/submission.rs:88` (`check`), `core/src/status.rs:105-135`, `:209`
- `areas/deposit/editor/web/src/handlers/projects.rs:36`, `web/src/pages/projects.rs:46-51`
- `areas/deposit/editor/pull-requests/src/layout.rs:23`, `pull-requests/src/run.rs:218`
- `areas/deposit/identity/web/src/pages/depositors.rs:161`, `handlers/depositors.rs:78-121`, `:213`, `:309`
- `areas/deposit/identity/ports/src/lib.rs:57` (`OpenWork`), `areas/deposit/editor/store/src/open_work.rs:14`, `areas/deposit/server/src/serve.rs:81-109`, `areas/deposit/editor/store/src/schema.rs:42-61`
- `areas/deposit/editor/web/src/reconcile.rs:63-73`, `handlers/approved.rs:93`, `:153`, `pages/approved.rs:109`
- Prototype: `dsp-incubator/metadata-editor-v2/src/workflows/screen-projects.jsx` (`NewProjectModal`), `store.jsx` (`emptyProject`)
- PRD: `dasch-specs/specs/2026-05-08-rdu-metadata-editor/02-metadata-editor-v1-PRD.md` (REQ-1.5, 1.9, 1.12-1.15, 2.3-2.6)
