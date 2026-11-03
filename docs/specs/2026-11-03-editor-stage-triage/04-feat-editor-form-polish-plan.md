---
title: "feat: editor form polish from the 2026-10-28 stage test"
type: feat
date: 2026-11-03
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7690
linear: DEV-7690
linear_project: 48e9fd8122fa
---

# feat: editor form polish from the 2026-10-28 stage test

Group C, part 2, of the [stage-testing triage update](https://linear.app/dasch/project/metadata-editor-for-rdu-depositing-projects-48e9fd8122fa/activity#project-update-0c95bb2e) (2026-11-02). Stacks on `03-` (the pickers), because several items here (numbered labels, the Search button, the proposal page) look different once those land.

## Overview

| Phase | Issue | Change |
|---|---|---|
| 1 | [DEV-7703](https://linear.app/dasch/issue/DEV-7703) | Keywords and Alternative names: one header row, text first, no English default |
| 2 | [DEV-7690](https://linear.app/dasch/issue/DEV-7690) | Per-field hints and examples, no visible numbered labels, `lang` on language-tagged values, Country and Canton dropdowns |
| 3 | [DEV-7689](https://linear.app/dasch/issue/DEV-7689) | Short description: live character counter, no silent stop, a server-side limit |
| 4 | [DEV-7691](https://linear.app/dasch/issue/DEV-7691) | Form alignment: repeatable fields flush, section heading vs field label, select arrow, review-queue row |
| 5 | [DEV-7694](https://linear.app/dasch/issue/DEV-7694) | Header navigation on every page, with the review-queue count for RDU |
| 6 | [DEV-7696](https://linear.app/dasch/issue/DEV-7696) | Project image section says how to provide an image |

## Problem Statement / Motivation

Each issue is a gap Daniela hit on stage; together they decide whether a depositor can fill the form without asking RDU what a field means or how to get back to a page. The thread asked for all of them before closing the project (Balduin: "fix all these issues in a single Claude session").

## Proposed Solution

- **DEV-7703.** `multilingual_rows` (`widgets.rs:1163`) labels every line ("Alternative name 1", "Language of alternative name 1"). One visible header row ("Name", "Language") per field; per-line labels visually hidden; text field first, language second; a new row starts on "Choose language" (the select tile's `placeholder()`, `select/mod.rs:95`; the empty tag is skipped when non-UI languages are appended at `widgets.rs:1213`) instead of `BLANK_ROW_LANGUAGE = "en"` (`:316`). A text line with no language is dropped silently today (`core/src/form.rs:727` skips a tag failing `is_language_tag`, and `""` fails) and `row_lines` (`widgets.rs:297`) filters it out of the re-render, so the autosave fired by the typed keyword (change event, `pages/section.rs:645`, no intent) would erase the line while the user tabs to the select. So: `row_lines` keeps the empty tag, autosave stores nothing for that row and shows no finding, and a body-level check before the appliers (like `over_cap`, `handlers/sections.rs:665`) makes explicit Save flag the row and Submit refuse it (on the inline-error path from group A, findings computed in `view()`). The header text ("Name", "Language") is a prefix of each line's accessible name ("Name 1", WCAG 2.5.3); a stored two-language entry renders as one row with two lines and one Remove (`widgets.rs:1194`) and gets a per-row cue.
- **DEV-7690.** Hints on the controls listed in the issue (Documentation/Additional material Link, Publications Reference and Persistent identifier, Authority records Source/Link/Shown as, Address Canton/Country/Additional address line); a greyed example as `placeholder`, only in addition to a hint. The mosaic `text_field` and `textarea` tiles gain `placeholder` and `lang` (neither exists today). Every remaining visible numbered label ("Reference 1", "Persistent identifier 1", …) is visually hidden, the pattern group B introduced for DEV-7701. Country becomes a dropdown over a bundled list of English names that includes every committed value (Switzerland, Germany, Belgium, Tanzania, Egypt, Italy, Monaco, United States, Faroe Islands; 51 organisations have none); Canton a dropdown of the 26 cantons stored as the English name (no committed organisation has one; `shared/metadata/src/organization.rs:29` types it as free text), always rendered and only hidden by Datastar when Country is not Switzerland (a no-JS user can still reveal it; `apply_text` returns early on an absent key, `core/src/form.rs:582`, so an unrendered Canton would never clear), cleared on apply when the country, ignoring case and surrounding spaces, is not Switzerland; a stored value outside either list stays selectable, so committed organisation data keeps rendering. `lang` is omitted when the tag is empty.
- **DEV-7689.** Following the GOV.UK character-count component: no `maxlength`; a counter under the field ("You have 37 characters remaining" / "12 characters too many"), announced politely after typing pauses: a wrapper `data-signals:_remaining` rendered by the server (right on load and after every morph), the textarea's `"data-on:input__debounce.500ms"` updating it from `[...el.value].length` (spread, so the client counts scalars like the server, not UTF-16 units), and a `p aria-live="polite" data-text` showing it; `data-computed` fails the attribute grammar, so `data-signals` plus `data-text` or a grammar line with a test. The page test that forbids `data-on:input` (`pages/section.rs:1115`) is narrowed to the form tag by `03-` phase 1; if this lands first, this phase narrows it; over the limit is a finding on save and a refusal on submit, counted in characters over the stored LF-normalised value (`core/src/form.rs:253`) with the counter using the same unit, as a gate after the sentinel gate in `submission::check`'s fixed order (`core/src/submission.rs:3`) with its arm in the exhaustive `SubmitRefusal` match (`handlers/sections.rs:1078`). The refusal applies only when the value differs from the published one, the address carve-out's rule (`submission.rs:182-186`): `0850_martcult` (237 characters) and `0101_religious-speech` (215) are published over the cap and must keep submitting unrelated changes. RDU's in-place review editor shares the control and its test asserts `maxlength="200"` (`handlers/review.rs:2040`); it gets the same server check and that test changes. Without JavaScript: the hint and the server check. Today there is no server check at all (`SHORT_DESCRIPTION_MAX` is only a `maxlength` attribute, `widgets.rs:1394`).
- **DEV-7691.** In mosaic CSS: repeatable fields flush with plain fields (`form/choice.css:18` `.field-group pl-3`, `repeatable_list.css`); a section with a single field whose label equals the section heading hides the label visually; a custom select arrow with `appearance: none` and matching inner spacing (`select/select.css:8-12` keeps the native arrow because `appearance: none` loses it in forced-colors mode, which no a11y suite covers; the custom arrow keeps a `@media (forced-colors: active)` fallback to the native control, and the comment says so); review-queue tag and button vertically centred, a compact button size in tables. The Search-button offset is re-checked after `03-`, whose combobox replaces that layout.
- **DEV-7694.** One header menu from `deposit-shell`'s header (`areas/deposit/shell/src/components/global/header.rs:14`). For RDU: Projects · Review queue (N) · Approved records · Accounts; for depositors: Projects. `Viewer` (`shell/src/view.rs:13`, today only `name`) gains the nav items; the count is a dedicated count query (`SubmissionRepository::list`, `store/src/submissions.rs:132`, deserialises every payload), computed once per request; on a storage error the header renders without a count. Identity's pages (`/depositors`) render the same header through the existing `identity_ports::OpenWork` seam (`identity/ports/src/lib.rs:57`, implemented by `editor-store`'s `LiveOpenWork`, `store/src/open_work.rs:14`, wired at `server/src/serve.rs:84`; `FakeOpenWork`, `store/src/test_support.rs:302`, updated): `deposit-server` never depends on a core crate (`check-composition-root-deps.sh`), so it cannot name `SubmissionRepository`, and the count is a method on that port; group A's known-project query (`01-` phase 4) uses the same seam. The signed-out `404` (`editor/web/src/shell.rs:93-103`) and the root fallback stay without nav; `forbidden` gets it. `aria-current="page"` marks the item whose path prefix matches (`/review/{shortcode}` under Review queue). The links at the bottom of RDU `/projects` go.
- **DEV-7696.** Image upload needs a decision (Balduin: "needs discussion"). Until then, the section says so above the credit field: "Image upload isn't available yet. To add or change the project image, send it to RDU." No preview: the editor image carries no cover images (they live in DPE's `public/assets/images/`).

## Technical Considerations

- **Accessible names** stay distinct per row after labels become visually hidden; the a11y suite and a rendering test per widget pin it.
- **`lang` goes stale** when the dropdown changes until the next re-render; accepted in DEV-7690's comment.
- **Mosaic CSS is shared** with DPE and the playground: phase 4 runs the DPE a11y suite.
- **Capability gate** (phase 5): `check-capability-deps.sh` keeps identity off editor crates; the count crosses through `identity-ports` → `editor-store`, as `OpenWork` does, with no gate change.
- **Group A's error path** (`01-` phase 1: findings computed in `view()`, keyed by row) is what phases 1 and 3 plug into; this plan starts after it merged.
- **Mosaic CSS build:** each app's stylesheet uses `source(none)`, so a class in a new CSS file exists only once that file is `@import`ed (dasch-specs learning `build-errors/tailwind-source-none-silent-class-loss.md`).
- **Sandbox:** CSS rebuild, Playwright and `just test` run outside the sandbox.

Bindings (`eng.yaml`): test `just test`, lint `just check`, gate `just check && just test`; conventions `CLAUDE.md`, `CONVENTIONS.md`, `areas/deposit/editor/CLAUDE.md`, `areas/deposit/identity/CLAUDE.md`, `areas/deposit/server/CLAUDE.md`, `modules/mosaic/CLAUDE.md`. Reviewers for every phase review: `eng:review:accessibility-reviewer`, `eng:review:ivan-reviewer`, `eng:review:security-reviewer`, `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`. Verify skill: `eng:test-browser`.

## Execution and Delivery

- **One PR stacked on `03-`'s branch** (`gh stack`), branch `feat/dev-7690-editor-form-polish`, plan commit first, then one commit per issue (`allow-many-commits`, Review Notes). When `03-` merges, rebase with `git rebase --onto origin/main <old 03 tip>` (rebase-merge rewrites the parent's SHAs).
- **Parallel lanes:**

  | Lane | Phases | Files |
  |---|---|---|
  | Form widgets | 1 → 2 → 3 | `form/widgets.rs`, `core/src/form/registry.rs`, mosaic `text_field`/`textarea`, `entity/mod.rs` address |
  | Mosaic CSS | 4 | `modules/mosaic/tiles/src/components/**/*.css`, `pages/review.rs` row |
  | Shell | 5 | `areas/deposit/shell`, `editor/web/src/shell.rs`, `pages/projects.rs`, identity shell, `deposit-server` |
  | Image | 6 | image section in `registry.rs` hint or `pages/section.rs` |

  Phase 6 is small enough to run inside any lane's gap. Commits still land in plan order. `pages/projects.rs` is rewritten by group A's phase 3; this plan edits it only after A merged.
- **Tests per phase are targeted**; full gate and E2E once in phase 7. **Push once,** at ship.

## Implementation Phases

#### Phase 1: Language rows (DEV-7703)

### dsp-repository
- [x] Rebase the branch onto `03-`'s branch tip
- [x] One visible header row per field; per-line labels visually hidden
- [x] Text field before the language select
- [x] A new line starts on the select's placeholder; a non-empty line without a language survives autosave untouched (`row_lines` keeps the empty tag), is a finding on explicit Save through a body-level check before the appliers, and a refusal on submit
- [x] Header text is a prefix of each line's accessible name; a two-language row shows a per-row Remove cue
- [x] Rendering and handler tests for the above, including distinct accessible names per line
- [x] Run `cargo test -p editor-core -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Hints, labels, `lang`, address dropdowns (DEV-7690)

### dsp-repository
- [x] `placeholder` and `lang` on the mosaic `text_field` and `textarea` tiles
- [x] Hints and examples on the controls listed in DEV-7690
- [x] Remaining visible numbered row labels visually hidden
- [x] `lang` set on language-tagged inputs (Keywords, Alternative names, per-language Description and Abstract)
- [x] Country dropdown over a bundled list and Canton dropdown (English names), Canton always rendered and only hidden with JavaScript, cleared when the country, ignoring case and surrounding spaces, is not Switzerland, keeping any stored value outside the lists
- [x] Tests: each hinted control announces its hint; every committed country is in the list; a stored non-list country still renders selected; `lang` matches the line's language and is absent on an empty tag
- [x] Run `cargo test -p mosaic-tiles -p editor-core -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Character counter (DEV-7689)

### dsp-repository
- [x] Drop `maxlength` on Short description; a counter under the field, announced politely after a pause
- [x] Over 200 characters is a finding on save and a refusal on submit, server-side, only when the value differs from the published one; the review-surface edit path checks it too
- [x] Tests: a 250-character paste is kept; submit refuses it with a field error; `0850_martcult`'s published short description still submits unchanged; the no-JS page shows the hint and the server error
- [x] A Playwright test of the counter in the JS pass
- [x] Run `cargo test -p editor-core -p editor-web` and that Playwright test; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Form alignment (DEV-7691)

### dsp-repository
- [x] Repeatable fields flush with plain fields
- [x] A single-field section hides a label equal to its heading
- [x] Custom select arrow with matching inner spacing and a forced-colors fallback to the native arrow
- [x] Review-queue tag and button vertically centred, compact button in tables
- [x] Check the contributor search layout after `03-` and fix any remaining offset
- [x] Run `cargo test -p mosaic-tiles`, `just css-editor-release`, `just test-a11y-editor` and `just test-a11y-dpe` outside the sandbox; all pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: Header navigation (DEV-7694)

### dsp-repository
- [x] `deposit-shell` header renders nav items from `Viewer`, marking the current page with `aria-current`
- [x] Editor pages pass RDU or depositor nav items, with the pending-review count for RDU
- [x] Identity pages get the same nav through a count method on `identity_ports::OpenWork`, implemented in `editor-store` and wired by `deposit-server`; the header renders without a count on a storage error; `forbidden` gets the nav
- [x] Remove the RDU links at the bottom of `/projects`
- [x] Tests: RDU and depositor headers on an editor and an identity page; the count matches pending submissions; `aria-current` on `/review/{shortcode}`
- [x] Run `cargo test -p deposit-shell -p editor-web -p identity-web -p deposit-server` and `bash .github/scripts/check-capability-deps.sh`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: Project image notice (DEV-7696)

### dsp-repository
- [x] The image section states that upload isn't available yet and to send the image to RDU, above the credit field
- [x] Rendering test for the notice
- [x] Run `cargo test -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: Clean up and gate

### dsp-repository
- [x] Trim comments added on this branch to the "why not the obvious approach" kind
- [x] Delete tests added on this branch that would fail only if a line were deleted
- [x] Fold every `fixup!` commit into its phase commit (`GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash <03- branch>`), leaving the plan commit plus one commit per issue
- [x] Run `just fmt`, then `just check && just test` outside the sandbox; it passes
- [x] Run `cargo build -p deposit-server --release`, `just css-editor-release` and `just test-e2e-editor` outside the sandbox; it passes in both passes
- [x] Run `eng:test-browser` on Overview, Dataset, Publications, an organisation page, `/review`, `/depositors` and the image section; no defect reported
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Decide whether image upload belongs in v1 (DEV-7696) | Balduin, RDU | after ship | Product decision |
| H2 | Merge `03-` before this PR | Marcin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] Language rows: one header row, text first, no English default
- [x] Every field in DEV-7690's table has a hint; no visible "Link 1"/"Reference 1"; Country and Canton are dropdowns
- [x] Short description accepts a long paste, counts it, and refuses it only at submit
- [x] Repeatable fields align with plain fields; selects have even spacing; queue rows are aligned
- [x] Every page with a session has the header menu; RDU sees the pending count
- [x] The image section says how to provide an image
- [x] `just check && just test` and `just test-e2e-editor` pass

## Dependencies & Risks

- **Stacked on `03-`:** a change requested in `03-`'s review may ripple here; the stack rebase is mechanical, conflicts are in `widgets.rs`.
- **Mosaic CSS** changes DPE's look (phase 4); covered by the DPE a11y suite.

## Success Metrics

Daniela's re-test of B4, B5, B9, B11, B16, B17, B23 (look), M2 and M3 passes on stage.

## References

- `areas/deposit/editor/web/src/form/widgets.rs:316`, `:1141-1163`, `:1394`, `:1432`
- `areas/deposit/editor/web/src/entity/mod.rs:165-175` (address members)
- `modules/mosaic/tiles/src/components/form/{text_field,textarea,select}/`, `form/choice.css`, `repeatable_list/`
- `areas/deposit/shell/src/components/global/header.rs:14`, `shell/src/view.rs:13`, `areas/deposit/editor/web/src/shell.rs:39-52`, `:93-103`, `pages/projects.rs:76-81`
- `areas/deposit/identity/ports/src/lib.rs:57` (`OpenWork`), `areas/deposit/editor/store/src/open_work.rs:14`, `store/src/submissions.rs:132`, `areas/deposit/server/src/serve.rs:84`
- `areas/deposit/editor/core/src/form.rs:253`, `:582`, `:727`; `core/src/submission.rs:3`, `:182-186`; `web/src/handlers/sections.rs:665`, `:1078`; `handlers/review.rs:2040`
- GOV.UK character count: https://design-system.service.gov.uk/components/character-count/
