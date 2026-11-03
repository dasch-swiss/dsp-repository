---
title: "fix: editor fixes before prod from the 2026-10-28 stage test"
type: fix
date: 2026-11-03
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7682
linear: DEV-7682
linear_project: 48e9fd8122fa
---

# fix: editor fixes before prod from the 2026-10-28 stage test

Group B of the [stage-testing triage update](https://linear.app/dasch/project/metadata-editor-for-rdu-depositing-projects-48e9fd8122fa/activity#project-update-0c95bb2e) (2026-11-02), with the decisions from its comment thread. One phase per issue. Runs in parallel with group A (`01-`); group C (`03-`, `04-`) builds on both.

## Overview

| Phase | Issue | Change |
|---|---|---|
| 1 | [DEV-7682](https://linear.app/dasch/issue/DEV-7682) (+ [DEV-7697](https://linear.app/dasch/issue/DEV-7697) wording) | One status message after submit, approve and discard; neutral colour; RDU wording to RDU; release-on-demand wait |
| 2 | [DEV-7683](https://linear.app/dasch/issue/DEV-7683) | "Add another" clears once its value lands, added values can be removed, data languages from an ISO 639 list |
| 3 | [DEV-7687](https://linear.app/dasch/issue/DEV-7687) | Review queue lists a submitted or approved project once |
| 4 | [DEV-7692](https://linear.app/dasch/issue/DEV-7692) | Button contrast passes WCAG AA in every state; no orange for normal states |
| 5 | [DEV-7701](https://linear.app/dasch/issue/DEV-7701) | No "Link N" on name and job-title rows |
| 6 | [DEV-7693](https://linear.app/dasch/issue/DEV-7693) | No internal names on screen: setting names, entity ids, "published set this deployment carries" |
| 7 | [DEV-7684](https://linear.app/dasch/issue/DEV-7684) | Legal info shows its content; no form buttons on a section with nothing to edit |

Settled in the thread: DEV-7697 is release on demand ("at most 2-4h after RDU approval", Balduin), so the code change is wording only; the thread dropped the follow-up project, so DEV-7683's ISO 639 list lands here rather than in group C.

## Problem Statement / Motivation

Every depositor sees the stacked boxes and the "a few weeks" promise; RDU sees a project twice in its queue; the primary button fails AA; several screens show developer names or "Link 1". Small, shared-widget fixes, all visible in the pilot. Daniela's quotes and steps are in each issue.

## Proposed Solution

- **DEV-7682.** Flash notices (after an action) and persistent state boxes are rendered independently (`pages/section.rs:553` vs `:617-625`, `:666`, `:714`), so both show after submit, approve or discard. Rule: a page shows **one** status box, by precedence: a flash right after an action, else the locked box, else the awaiting-release line, else a round box only for ChangesRequested or Rejected while the form is editable; on later visits that is one neutral line ("In review. You can't edit until RDU has looked at it."). Approve has no flash on the section page (it happens on the review surface), so the depositor's next GET shows two persistent boxes today, the round summary (`pages/section.rs:617`, Success) and `awaiting_release_notice` (`:618`, Info): the wait line merges into the approved round summary and `awaiting_release_notice` goes. The approved round box shows only while `awaiting_release` is true (`handlers/sections.rs:384`; today the round is the latest row unfiltered, `:399`, and keeps promising a release after the record is gone), and `awaiting_release` derives from `classify_record == WaitingForRelease` only, so a record that never releases (dropped upstream; an unreadable payload, `handlers/projects.rs:168-176`) never carries the same-day promise. Withdraw today renders both its flash (`pages/section.rs:570`) and its round box (`:207`); Discard (`sections.rs:1438`, deletes only the draft) shows its flash beside a stale round; the precedence covers both. Locked boxes move from `Warning` to `AlertVariant::Info`. RDU wording is a new `ProjectState::explanation_for(audience)` beside the viewer-less `explanation()` (`core/src/status.rs:65`, shared by the list column, `/states` and the awaiting box), applied to the flash (`pages/section.rs:563`), locked (`:151`) and round strings (`:188-211`) too, since RDU can submit and withdraw (`sections.rs:1842`); `SectionView.audience` already carries the viewer (`sections.rs:187`). RDU reads "waiting in the review queue" with a link to `/review`, never "Ask RDU". "Usually within a few weeks" lives in two places (`core/src/status.rs:81`, which feeds the list column, `/states` and the waiting box; and `pages/section.rs:204-205`), is pinned by six tests (`status.rs:459`, `pages/states.rs:60`, `handlers/projects.rs:495`, `handlers/sections.rs:5180`, `status.spec.ts:123`, `:238`), and becomes one string: DaSCH publishes approved changes on demand, usually on the same working day. "Editing again starts the next round" goes. After approval the form stays editable, and the line says so ("You can keep editing; your next changes go to RDU as a new submission"). The RDU process itself (collect, merge, release, tell the project) is documented in `docs/src/editor/publishing-to-git.md`.
- **DEV-7683.** The add input is empty in markup (`widgets.rs:1305`, `:705`), so the value that stays is live DOM state surviving the Datastar morph: the vendored 1.0.2 bundle resets an input's live `.value` only when its `value` attribute changed, and the add box renders none. Phase 2 confirms with a JS-pass Playwright test and fixes it server-side, giving the add input an id that changes with the stored list (its count as suffix) so the morph replaces the node instead of patching it; a signal reset cannot work, since the enhanced path answers patch-elements only (`handlers/sections.rs:1757`). Removal: every write replaces the stored list with the posted values (`core/src/form.rs:648-665`) and the options are offered-plus-stored (`widgets.rs:1288-1293`), so once the add box clears, unticking a custom value and saving drops it; Daniela saw it persist because the add box re-added it. The phase 2 test unticks, waits for the autosave and asserts the value is gone; if it is, the hint (`widgets.rs:1318`) is reworded ("untick it and save to remove it") and no Remove control is built. If it is not, the control is a named-submit intent addressed by row and index (the propose-changes pattern, `pages/section.rs:84`), never the `/fields/{f}/{key}/remove` route, which `404`s for a `StringList` (`handlers/sections.rs:972-979`, `has_rows` false) and, for a role, drops the whole contributor (`form.rs:98-108`). Data languages: the add box offers language names and stores the code, as every committed `dataLanguage` does (24 distinct codes, `de`, `fr`, `la`, `grc`, …). The list is the prototype's `DATA_LANGUAGES` (191 entries) plus `pez` and `x-cpa`, which the corpus uses and the prototype lacks, bundled in `editor-core`; it also replaces the five-entry `language_name` table (`widgets.rs:161`, which claims to cover the corpus and misses 19 of its 24 codes). Offered as a `<datalist>` through the `text_field` tile's `list` builder (`text_field/mod.rs:144`): `no_page_ships_the_whole_agent_set_and_every_picker_can_be_searched` (`pages/section.rs:1188`) fails on any `<datalist>`, but its rationale (`widgets.rs:365-369`) is the agent set's size and a datalist filtering against a box that already holds the id, neither of which applies to an empty add box over 193 names, so the test narrows to "no datalist of agents" and its comment says why. The datalist is rendered once with a stable id inside the region, its option `value` is the language name (the browser inserts `value` into the field) and the server maps name to code. A name resolves to its code before the dedupe (`core/src/form.rs:654-657` compares raw strings, so "Hungarian" beside a ticked `hu` would store both); "Hungarian, Belgian" splits on commas and the split output counts against `MAX_VALUES_PER_FIELD`; an unrecognised name is kept as typed in the draft with a hint (REQ-1.9) and is a per-field refusal at submit unless already stored. Roles stay free text, never split on commas, with clear and remove.
- **DEV-7687.** `DraftRepository::list` is unfiltered (`handlers/review.rs:87`) and submission and approval keep the draft. "Drafts in progress" lists only drafts with no submission row and edited after the project's newest Approved review round (`ReviewRoundRepository::list_for_shortcode`, `core/src/repository.rs:160`; rounds survive Online, approved records do not: they are replaced per approval, `store/src/review_rounds.rs:120`, and deleted at release, `reconcile.rs:81`), comparing the draft's `updated_at` with the round's time. A no-op Save bumps `updated_at` (`handlers/sections.rs:824-843`) and re-lists the draft; accepted. The project name renders the same way in both lists. `the_queue_shows_every_draft_as_well` (`review.rs:1802`) pins the old behaviour and is rewritten.
- **DEV-7692.** `modules/mosaic/tiles/src/components/button/button.css`: secondary hover goes darker with dark text (2.4:1), primary hover goes lighter; both get text and background tokens that pass 4.5:1 in default, hover and focus, and the focus outline gets an explicit colour. This file is shared by DPE and the mosaic playground, so their a11y suites run too. Normal states move off `Warning` onto existing variants, no new one (`badge/mod.rs:13-21`, `alert/mod.rs:22-28`): the `Waiting` badge (`pages/review.rs:198`) to `BadgeVariant::Secondary` (gray; `Info` aliases the secondary blue), the entity `Closed`/`Accepted` boxes (`entity/mod.rs:223`, `:378`) to `AlertVariant::Info`. Section locked boxes are phase 1's. The playground's `/button` showcase (`modules/mosaic/playground/src/app.rs:53`) is scanned by nothing, so the playground check is `eng:test-browser` on it.
- **DEV-7701.** `string_rows` hard-codes `ROW_VALUE_LABEL = "Link"` (`widgets.rs:338`, `:1147`) and the entity form uses it for given names, family names and job titles. It takes the row label from the field; the numbered label becomes visually hidden (kept for screen readers). DEV-7690 (group C) extends the same visually-hidden pattern to the other numbered rows.
- **DEV-7693.** `EDITOR_RDU_EMAILS` (`identity/web/src/pages/depositors.rs:51`, `handlers/depositors.rs:54`) → "RDU members are set up by DaSCH and can't be changed here." Entity ids → the entity's name wherever one is resolved: entity `<h1>` and `<title>` (`entity/mod.rs:323`, `handlers/entities.rs:620`), proposals box (`pages/section.rs:807-827`), "Proposal started" (`:727-750`), findings (`:780`), discard confirmation (`entity/mod.rs:492`), review surface (`pages/review.rs:685`, `:774`). "Published set this deployment carries" → plain wording at `pages/section.rs:482` and `pages/review.rs:541`. **Not `pages/projects.rs:49`:** group A (DEV-7695) rewrites that message with project creation.
- **DEV-7684.** `value_markup` (`widgets.rs:114-156`) renders any structured value as a count and is shared with the review surface (its snapshots `review_diff_*.snap` and the "N entries" diffs for publications and funding), so `legalInfo` is special-cased in `value_display` (`:105`) only: licence identifier, date and URI, copyright holder and authorship per entry (`shared/metadata/src/project.rs:243-258`), tolerating the `["MISSING"]` placeholder (`core/src/form/submit.rs:579`). A section with no editable field for the viewer (`section.fields_for(audience).any(Field::is_editable)`, `core/src/form/registry.rs:145`; only `legal` qualifies, `:575-580`, and it is RDU-only) hides Save draft, Submit for review and Discard draft inside `controls(view)` (`pages/section.rs:659`, `:874`) but keeps "Take the submission back" (`:911-921`) and the "Maintained by RDU" note.

## Technical Considerations

- **Depositor vocabulary** (REQ-2.1, REQ-2.2): new depositor strings pass `web/tests/depositor_vocabulary.rs`; RDU strings may name the mechanism.
- **Datastar morph and focus:** the add-input fix must keep focus behaviour pinned by `widgets.rs:1538` and the row-control sweep in `row-controls.spec.ts`.
- **Mosaic is shared** (phase 4): run the DPE a11y suite and check the playground; token changes in `theme_provider/tokens.css` would reach every app, so prefer changing the button classes over the tokens. Each app builds its own stylesheet with `source(none)`, so a class in a new CSS file exists only once that file is `@import`ed (dasch-specs learning `build-errors/tailwind-source-none-silent-class-loss.md`).
- **Contrast is measured, not assumed:** axe scans default states only and does not see hover. Phase 4 measures the hover and focus states once with `eng:test-browser` (computed colours, ≥ 4.5:1) rather than adding a hover test to the suite, since this is a one-time CSS fix on two classes; it adds the submitted-locked and awaiting-release pages to the a11y scan.
- **Sandbox:** CSS rebuild, Playwright and `just test` run outside the sandbox; check the stylesheet is fresh before trusting a contrast result.

Bindings (`eng.yaml`): test `just test`, lint `just check`, gate `just check && just test`; conventions `CLAUDE.md`, `CONVENTIONS.md`, `areas/deposit/editor/CLAUDE.md`, `areas/deposit/identity/CLAUDE.md`, `modules/mosaic/CLAUDE.md`. Reviewers for every phase review: `eng:review:accessibility-reviewer`, `eng:review:ivan-reviewer`, `eng:review:security-reviewer`, `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`. Verify skill: `eng:test-browser`.

## Execution and Delivery

- **One PR**, branch `fix/dev-7682-editor-pre-prod-polish`, plan commit first, then one commit per issue (`allow-many-commits`, Review Notes mapping commit → issue). No issue commits are merged: they touch different screens and each closes its own Linear issue. Review fixes are `fixup!` commits folded in phase 8.
- **Parallel lanes** (file ownership, so workers never share a file):

  | Lane | Phases, in order | Files it owns |
  |---|---|---|
  | Section page | 1 → 7 (section part) | `core/src/status.rs`, `pages/section.rs`, `handlers/sections.rs`, `/states` page, `docs/src/editor/{status,publishing-to-git}.md` |
  | Form widgets | 2 → 5 → 7 (widget part) | `form/widgets.rs`, `core/src/form.rs`, language list data |
  | Review, entity, mosaic | 3 → 4 → 6 | `handlers/review.rs`, `pages/review.rs`, `store/src/drafts.rs`, `button.css`, `entity/mod.rs`, `handlers/entities.rs`, identity depositor pages |

  Phase 6 also edits `pages/section.rs` (proposals box, notices) and waits for phase 1 there; phase 7 waits for phases 1 and 5. Commits still land in plan order. Group A edits `pages/section.rs`, `handlers/sections.rs`, `form/widgets.rs` and `core/src/status.rs` too, in other regions (its error rendering, `apply_and_store`, the format hooks in the row widgets, the classification); whichever PR merges second rebases on `origin/main` and resolves textual conflicts there. `pages/projects.rs` is A's alone.
- **Tests per phase are targeted** (named crates and one spec). Full gate and full E2E run once, in phase 8. **Push once,** at ship.

## Implementation Phases

#### Phase 1: One status message (DEV-7682, DEV-7697 wording)

### dsp-repository
- [x] A page shows one status box, by precedence: flash (Submitted, Withdrawn or Discarded; Saved, Changed, Proposed and Refused notices still render beside the persistent box, accepted by the owner 2026-11-03), else locked, else a ChangesRequested/Rejected round box when it is the latest round (owner, 2026-11-03), else awaiting-release; `awaiting_release_notice` merges into the approved round summary, which shows only while awaiting release; `awaiting_release` derives from `WaitingForRelease` only
- [x] Persistent locked boxes use `AlertVariant::Info` instead of `Warning`
- [x] RDU viewers get RDU wording through `explanation_for(audience)` on locked, approved, flash and round strings; no "Ask RDU" for RDU
- [x] One release-on-demand wait string in `ProjectState::Approved.explanation()`, used by the approved round summary too; "Editing again starts the next round" removed; the approved line says the form can be edited again
- [x] Document RDU's release-on-demand steps (collect, merge, release, tell the project) in `docs/src/editor/publishing-to-git.md`, and the expected wait in `docs/src/editor/status.md`
- [x] Update the six tests pinning "few weeks" (`status.rs:459`, `pages/states.rs:60`, `handlers/projects.rs:495`, `handlers/sections.rs:5180`, `status.spec.ts:123`, `:238`)
- [x] Handler tests: after submit, approve, withdraw and discard exactly one status box renders, on both renderings (`page::region` and `render_page`, since the status div sits inside the morphed region, `pages/section.rs:459`, `:555`); a later GET renders the persistent line; RDU sees RDU wording; a dropped-upstream record shows no wait promise
- [x] Run `cargo test -p editor-core -p editor-web` and `cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/status.spec.ts`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: "Add another" clears and values can be removed (DEV-7683)

### dsp-repository
- [x] A JS-pass Playwright test reproducing the kept add-box value (Data languages and "Add another role") that fails on the current code
- [x] The add box is empty after its value lands, in both passes, including a save that leaves the stored count unchanged: its id changes whenever the request posted add-box text, and the replacement keeps focus (owner, 2026-11-03)
- [x] Untick-and-save removes an added value (data languages and roles), proven by the JS-pass test and reflected in the hint; a Remove control only if it does not, as a row-and-index named-submit intent
- [x] Bundle the language list in `editor-core`, extracted once from the prototype's `DATA_LANGUAGES` (`dsp-incubator/metadata-editor-v2/src/workflows/store.jsx:57-125`: ISO 639-1 plus the classical ISO 639-3 codes common in DaSCH corpora) plus `pez` and `x-cpa`, and check every committed `dataLanguage` code is in it; `language_name` reads from it; the data-language add box suggests names through a `<datalist>` (the agent-datalist test narrowed to agents) and stores the code, resolved before the dedupe; comma-separated input becomes separate values within the field cap; an unknown name is kept as typed with a hint and refused at submit
- [x] Tests: added values clear and are removable; "Hungarian, Belgian" stores `hu` and an unknown entry; presets keep their "German (de)" labels
- [x] Run `cargo test -p editor-core -p editor-web` and `cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/row-controls.spec.ts`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Review queue lists a project once (DEV-7687)

### dsp-repository
- [x] "Drafts in progress" excludes drafts with any submission row and drafts not edited after the project's newest Approved review round
- [x] Project names render the same way in the queue and in "Drafts in progress"
- [x] Rewrite `the_queue_shows_every_draft_as_well` to the new rule; add a test that a submitted and an approved project are not listed as drafts, and an edited-after-approval one is
- [x] Run `cargo test -p editor-store -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Button contrast and neutral states (DEV-7692)

### dsp-repository
- [x] `button.css`: primary and secondary pass 4.5:1 in default, hover and focus; explicit focus outline colour
- [x] `Waiting` badge uses `BadgeVariant::Secondary`; the entity `Closed`/`Accepted` boxes use `AlertVariant::Info`
- [x] Measure hover and focus contrast of "Save draft" and "Submit for review" with `eng:test-browser` (computed colours, at least 4.5:1) and record the figures in the PR; no hover test in the suite
- [x] Add the submitted-locked section and the awaiting-release section to `tests/accessibility.spec.ts`
- [x] Run `cargo test -p mosaic-tiles -p editor-web`, `just css-editor-release`, `just test-a11y-editor` and `just test-a11y-dpe` outside the sandbox; all pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: No "Link N" on person rows (DEV-7701)

### dsp-repository
- [x] `string_rows` takes its row label from the field; the numbered row label is visually hidden
- [x] Check the other entity repeatable fields for the same label and fix them the same way
- [x] Rendering test: given names, family names and job titles show no visible "Link N", and each input keeps a distinct accessible name
- [x] Run `cargo test -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: No internal names on screen (DEV-7693)

### dsp-repository
- [x] `/depositors` intro and the RDU-immutable message without `EDITOR_RDU_EMAILS`
- [x] Entity names instead of ids in the entity heading and title, the proposals box, "Proposal started", findings, the discard confirmation and the review surface; the id only where no name is resolved
- [x] Plain wording for "not in the published set this deployment carries" on the section and review pages
- [x] Update the tests and snapshot that pin the old strings (`handlers/depositors.rs:831`, `review_diff_unpublished.snap`) and add one asserting the name in "Proposal started"
- [x] Run `cargo test -p identity-web -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: Legal info shows its content (DEV-7684)

### dsp-repository
- [x] `legalInfo` renders read-only as licence, copyright holder and authorship per entry, special-cased in `value_display`, tolerating the `MISSING` placeholder; the review snapshots are unchanged
- [x] No Save draft / Submit for review / Discard draft on a section without an editable field for the viewer; "Take the submission back" stays
- [x] Rendering test over a published project's legal section: licence text visible, no form buttons
- [x] Run `cargo test -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: Clean up and gate

### dsp-repository
- [x] Trim comments added on this branch to the "why not the obvious approach" kind
- [x] Delete tests added on this branch that would fail only if a line were deleted
- [x] Fold every `fixup!` commit into its phase commit (`GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash origin/main`), leaving the plan commit plus one commit per issue
- [x] Run `just fmt`, then `just check && just test` outside the sandbox; it passes
- [x] Run `cargo build -p deposit-server --release`, `just css-editor-release` and `just test-e2e-editor` outside the sandbox; it passes in both passes
- [x] Run `eng:test-browser` on a submitted, an approved and a discarded project, `/review`, a person and an organisation page, the legal section and `/depositors`; no defect reported
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Confirm the release-on-demand wording ("usually on the same working day") in the PR | Balduin, RDU | after ship | A promise to depositors is RDU's call |
| H2 | Merge this PR and group A's before DEV-7618's release workflow | Marcin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [ ] After submit, approve and discard a page shows one status box, in a neutral colour; no "a few weeks"; RDU reads RDU wording
- [ ] "Add another" clears; an added data language or role can be removed; data languages are stored as ISO 639 codes
- [ ] A submitted or approved project appears once on `/review`
- [ ] Buttons pass 4.5:1 in every state; no orange for a state that needs no action
- [ ] No "Link N", `EDITOR_RDU_EMAILS`, `organization-032` or "published set this deployment carries" on screen
- [ ] Legal info shows its content and no form buttons
- [ ] `just check && just test`, `just test-e2e-editor` and `just test-a11y-dpe` pass

## Dependencies & Risks

- **Mosaic change reaches DPE** (phase 4): DPE's button looks change too; the DPE a11y suite and a playground check cover it.
- **`pages/projects.rs`** belongs to group A; touching it here would conflict.
- **DEV-7683 cause** is inferred (Datastar morph); the reproduction test settles it before the fix.

## Success Metrics

Daniela's re-test of B6, B10, B21, B22, B23, B24 and B2 passes on stage.

## References

- `areas/deposit/editor/web/src/pages/section.rs:151-221`, `:553-625`, `:659`, `:714`, `:727-827`, `:874`
- `areas/deposit/editor/core/src/status.rs:81`
- `areas/deposit/editor/web/src/form/widgets.rs:105-156`, `:161`, `:320-338`, `:365-369`, `:640-721`, `:1141-1147`, `:1286-1332`; `pages/section.rs:1188` (datalist test)
- `areas/deposit/editor/core/src/form.rs:98-108`, `:648-665`; `core/src/repository.rs:160`; `store/src/review_rounds.rs:120`; `web/src/reconcile.rs:81`
- `areas/deposit/editor/web/src/handlers/review.rs:82-155`, `pages/review.rs:114-271`
- `modules/mosaic/tiles/src/components/button/button.css`, `badge/badge.css`, `alert/alert.css`
- `areas/deposit/editor/web/src/entity/mod.rs:214-223`, `:323-332`, `:378`, `:492`; `handlers/entities.rs:620`
- `areas/deposit/identity/web/src/pages/depositors.rs:51`, `handlers/depositors.rs:54`
