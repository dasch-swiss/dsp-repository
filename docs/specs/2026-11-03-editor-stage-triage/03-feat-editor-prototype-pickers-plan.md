---
title: "feat: editor pickers as in the prototype"
type: feat
date: 2026-11-03
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-for-rdu-depositing-projects/dsp-repository-dev-7702
linear: DEV-7702
linear_project: 48e9fd8122fa
---

# feat: editor pickers as in the prototype

Group C, part 1, of the [stage-testing triage update](https://linear.app/dasch/project/metadata-editor-for-rdu-depositing-projects-48e9fd8122fa/activity#project-update-0c95bb2e) (2026-11-02). The thread dropped the follow-up project, so group C stays in this project. Group C is two plans because the work is two reviewable PRs of very different weight: this one carries the two largest gaps to the prototype; `04-` carries the form polish and stacks on it.

## Overview

| Phase | Issue | Change |
|---|---|---|
| 1 | [DEV-7702](https://linear.app/dasch/issue/DEV-7702) | Contributor and contact-point picker: chosen entries as cards, one type-ahead search, roles as chips, a visible next step |
| 2 | [DEV-7686](https://linear.app/dasch/issue/DEV-7686) | Proposing changes to a person or organisation starts on its card and returns to where the user was |
| 3 | [DEV-7710](https://linear.app/dasch/issue/DEV-7710) | Disciplines can be recorded from the SNSF list; the UNESCO source is named after the list, not "Skos" |
| 4 | [DEV-7698](https://linear.app/dasch/issue/DEV-7698) | Search pickers for Disciplines, Temporal and Spatial coverage over bundled vocabularies |

Decisions from the thread: SNSF disciplines come from a one-time extraction into a JSON file treated as the source of truth (Balduin, answer 4; DPE's `temporal-coverage-enrichment.json` is the model); places and periods are a bundled subset, no live lookup (answer 5; the PRD rules out a new external dependency).

## Problem Statement / Motivation

The PRD makes the prototype normative for the UX flow (`02-metadata-editor-v1-PRD.md`, line 24), and the v1 plan never carried its pickers over. Today a chosen contributor is one line of small text above a search box that never clears; a recognised discipline, period or place is recorded by pasting a link into three plain inputs; and the SNSF list, which DaSCH uses most, cannot be chosen at all.

## Proposed Solution

- **One server-rendered type-ahead, built once** (phase 1), reused by phase 4. Mosaic gets a combobox tile (through the repo's `add-mosaic-component` skill, with its playground page) following the WAI-ARIA combobox pattern and DPE's existing one (`areas/access/dpe/web/src/pages/projects/components/project_search_input.rs:61-73`: `role="combobox"`, `aria-autocomplete`, `aria-controls`, `aria-expanded`, `"data-on:input__debounce.300ms"=(datastar::get(…))`, keyboard handling in an inline `onkeydown`; its listbox fragment at `areas/access/dpe/server/src/fragments.rs:165-167`). The typed query goes to a **non-writing search endpoint** (`GET`, answering only the listbox fragment), not the section form's POST: that POST runs `apply_and_store` before the find-agent branch (`handlers/sections.rs:705-729`), writes `updated_at` per pause, refuses a stale baseline (`:766-770`) and patches the whole region (`:1761`), which is why `an_editable_form_autosaves_on_change_and_a_locked_one_does_not` (`pages/section.rs:1107`) forbids `data-on:input` on the form; that test narrows to the form's own controls. `__debounce.Nms` is already in `check-datastar-attributes.sh`'s grammar; `data-ref`, `data-computed` and `data-style` are not, so focus handling stays on `data-init` as today (`widgets.rs:379-382`). The endpoint answers `200` with an empty listbox for an empty or over-long query (DPE's fragment returns `400`, `fragments.rs:124`, which in the editor would drop the body and fire the telemetry error listener); Datastar's automatic request cancellation covers a slower earlier response. Arrow keys move **focus** between the server-rendered choose buttons (roving focus: `ArrowDown` from the field focuses the first match, `ArrowUp`/`ArrowDown` move along them), and Enter submits the focused button natively, so it shares the no-JS write path (a POST through the form, as today's choose buttons). This is the simplest version that keeps "arrow keys don't save": no `_active` signal, no `aria-activedescendant`, no synthetic click. The full `aria-activedescendant` pattern (a private `_active` signal via `data-attr`, removed when falsy, reset whenever a result set morphs in; Enter clicking the highlighted button) is the fallback only if the accessibility reviewer rejects roving focus. Either way there is no `__key` modifier (an unknown one is ignored by Datastar and rejected by the grammar), so keys are filtered on `evt.key` in the expression; never `__prevent` on the input's keydown, which blocks typing; `__stop`, `__once` and `__capture` are outside the grammar and need a line plus a case in `check-datastar-attributes.test.sh` if used. Escape closes and keeps the current choice; the field clears after a choice. Without JavaScript the same markup is a search box, a Search submit button and one choose button per match, as today. No client-side catalog and no SSE stream (DSP principles: server authority, contained interactivity).
- **Contributor and contact point (phase 1).** Chosen entries render as cards (name, Person/Organisation badge, affiliation, Change, Remove); one search below the cards adds an entry; no "Keep <name>" option. Search results carry an affiliation line, so two people with one name can be told apart: `Agent` (`core/src/agents.rs:23`) gains a detail line built from the person's affiliation (organisation names) or the organisation's acronym, from the bodies `Agents` already holds. No match: "Not in the list? Propose a new person or organisation" inside the results (REQ-3.1). Three intents, each tested in both passes: find, choose-and-append (a new `.row` and `.contributor` pair; `apply_agent_choice`, `widgets.rs:506-524`, only overwrites an existing row) and change-row, with a hidden field recording the row being changed. The person page's affiliation picker and the funder picker use the same `agent_picker` (`entity/mod.rs:423`, `widgets.rs:528-534`), and the entity route dispatches only discard and save (`handlers/entities.rs:200-204`), so today's affiliation Search saves and clears; phase 1 gives that route the find and choose dispatch and covers funders. The detail line is computed in `AgentScope` (a proposed agent is built from its payload, `core/src/agents.rs:317`, not from bodies) and matched by `search` (`:250-253`). A contributor without a role is marked incomplete with "Choose at least one role", shown after a submit refusal or on revisit, not on the contributor just chosen; roles render as display-only chips over the checkbox set, which stays the only source of truth (`widgets.rs:699-709`; chips posting too would double every role), behind a "Choose roles" `<details>` disclosure (a closed details still submits, so it works without JS); a chip's Remove is a named submit by row key and role index, never role text (intents are colon-split, `pages/section.rs:107-127`).
- **Proposal flow (phase 2).** "Propose changes to this person's/organisation's details" moves onto the card's actions and opens the entity page. `proposed()` deliberately never redirects (`handlers/sections.rs:1345-1351`: Datastar processes a body only on a 200, so a followed `303` would morph the entity page into the section region) and a proposal is created by POST, so the enhanced path is decided before building: either the card's Propose control is its own small form outside the section form, so the browser follows the `303`, or the handler answers `200` with a script event that navigates; both passes are tested. "Back to the project" returns to the originating section at the entry: the section id travels as a hidden input and survives both redirects (`entity/mod.rs:312` gives the form no query, `handlers/entities.rs:562-568` builds the post-save redirect, and today's link is `/projects/{shortcode}`, `:332`), validated against the registry's section list for the viewer's audience with the project page as fallback (no open redirect); the card carries an anchor by field plus entity id, since row keys are positional (`widgets.rs:249`) and `<li class="repeatable-row">` has no id (`repeatable_list/mod.rs:169`). After discard the user lands back on that section with one confirmation naming the entity from the proposal ("Your proposed changes to DaSCH were discarded."; its own marker, guarded on the proposal being withdrawn, since `?done=discarded` is the project-draft notice at `sections.rs:564`), and the entry leaves the proposals box; for a discarded `New` proposal the rows referencing its id go too, since an unresolvable id renders "not found" (`widgets.rs:442-446`) and blocks submit (`sections.rs:1096`). The double "Discarded" + "Closed" page is gone.
- **SNSF disciplines (phase 3).** Every committed SNSF discipline is a text value with a leading five-digit code, like `{"en": "10404 Visual arts and Art history"}`: 126 values in 61 files on `6bc64dd3` (100 under `en` in 48 files, 20 under `de` in 10, 6 under `fr` in 6; the 73 `Skos` references are all UNESCO, so no SNSF reference exists). That stays the recorded shape: no contract change, and DPE renders it already. An SNSF row is recognised by its leading code in any language; a pick stores the English name under `en`, the only names the list carries. The list is extracted once from the prototype's `SNSF_DOMAINS` (`dsp-incubator/metadata-editor-v2/src/workflows/store.jsx:138`, 141 codes in 21 domain groups) into `areas/deposit/editor/core/data/snsf-disciplines.json`; the UNESCO **nomenclature** likewise from `unesco-skos.js` (`skos.um.es/unesco6`, 2- and 4-digit codes; its header flags most 4-digit labels as unverified, so they are checked against the live vocabulary before extraction). The committed `Skos` references are 71 `unesco6` nomenclature URLs (55 `http`, 16 `https`) and 2 `unescothes` thesaurus URLs (`0854_daschland.json:117`), so the source is named "UNESCO nomenclature" (DEV-7710 says "UNESCO Thesaurus"; H3 confirms the label), stored as type `Skos` with the `http` scheme, value unchanged. "How this is recorded" offers "From the SNSF list", "From the UNESCO nomenclature" and "My own wording".
- **Vocabulary pickers (phase 4).** Disciplines search both lists by code or name, grouped by domain. Places search a bundled subset: every GeoNames (159 distinct URLs over 230 entries), Pleiades (13) and DAI Gazetteer (12) entry the 85 projects already use, plus the 13 distinct plain-`URL` places, extracted into `data/places.json` with URL and label; a place not in the subset is still recorded as name plus authority link, the `URL` shape already in the data (the contract requires a reference, `spatial_coverage: Vec<AuthorityFileReference>`). Periods search the PeriodO (21) and Chronontology (30 distinct over 33 entries) entries in use plus a script-built subset of `chronontology-periods.json` (about 8,600 periods; only those with a timespan, since the loader drops the rest, `shared/metadata/src/chronontology.rs:89-91`, with the English preferred name as `text`). A Chronontology reference resolves by URL (`shared/metadata/src/temporal_coverage.rs:55-80`); a PeriodO one resolves only when an enrichment row's key equals its exact text, so the extraction keeps only options whose `completeness_gap` is `None`, a test runs every picker option through `unresolved_temporal_coverage`, and any PeriodO entry in use that lacks an enrichment row ships one in the same PR. Own wording stays and must still resolve to a structured date (REQ-1.14/1.15, `every_committed_temporal_coverage_resolves`). Each extraction is a checked-in script run once, so the files can be regenerated and reviewed.

## Alternative Approaches Considered

- **Client-side catalog with JS filtering** (the prototype's `SearchDrop`): the vocabularies would ship to the browser, and the no-JS path would need a second implementation.
- **SNSF as references with a PDF-fragment URL** (the prototype's `SNF_BASE#code`): changes the shape of 126 committed values or mixes two shapes for one list; DPE has no link to show for it either.
- **Live GeoNames/PeriodO lookup:** an external dependency the PRD rules out.

## Technical Considerations

- **Phase 1 and the open-redirect check:** the shell's sign-in link has one (`areas/*/shell/**` in `eng.yaml`); the phase 2 return path reuses that pattern rather than adding a second.
- **Datastar:** actions via `shared_datastar` builders; attribute names pass `check-datastar-attributes.sh`; any new attribute grammar needs a test there.
- **Focus:** after a choice focus moves to the new card's heading or the search box (pinned today by `widgets.rs:1820`); the E2E `agent-picker.spec.ts` covers both passes.
- **Data quality is not this plan's:** two malformed SNSF codes (`110302` in `084E_rich`, `103301` in `0849_operativetv`) and 17 codes that carry more than one name across the files (translations and real mismatches, `10201` as History and as Sociology) are left as they are; the "every committed code is in the list" test checks codes, never names; the picker only offers valid entries (H2).
- **Existing rows** recorded as a pasted reference keep rendering and saving unchanged.
- **Sandbox:** CSS rebuild, Playwright and `just test` run outside the sandbox.

Bindings (`eng.yaml`): test `just test`, lint `just check`, gate `just check && just test`; conventions `CLAUDE.md`, `CONVENTIONS.md`, `areas/deposit/editor/CLAUDE.md`, `modules/mosaic/CLAUDE.md`. Reviewers for every phase review: `eng:review:accessibility-reviewer`, `eng:review:ivan-reviewer`, `eng:review:maud-datastar-reviewer`, `eng:review:security-reviewer`, `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`. Verify skill: `eng:test-browser`.

## Execution and Delivery

- **Starts after groups A and B merge** (H1): phase 1 rewrites `attribution_rows`, which group B's DEV-7683 changes (roles), and phase 4 replaces the link inputs group A's DEV-7711/7685 put errors on. Starting earlier means rebasing through both.
- **One PR**, branch `feat/dev-7702-editor-prototype-pickers`, plan commit first, then one commit per issue (`allow-many-commits`, Review Notes mapping commit → issue). Plan `04-` stacks on this branch (`gh stack`).
- **Overlap:** phase 3's data extraction (script + JSON, `core/data/`) is file-disjoint from phase 1 and runs alongside it; its widget part waits for phase 1. Phase 2 waits for phase 1 (the card). Phase 4 waits for 1 and 3.
- **Tests per phase are targeted**; full gate and E2E once in phase 5. **Push once,** at ship.

## Implementation Phases

#### Phase 1: Contributor and contact-point picker (DEV-7702)

### dsp-repository
- [x] Rebase the branch onto `origin/main` carrying groups A and B
- [x] Mosaic combobox tile via `add-mosaic-component` (playground page included): listbox under the field fed by a non-writing `GET` search fragment on a debounced input, arrow keys move focus between the choose buttons (no `aria-activedescendant` unless the accessibility reviewer requires it), Enter/click chooses, Escape keeps, clears after a choice; no-JS fallback with Search and choose buttons; the form's `data-on:input` test narrowed to its own controls
- [x] `Agent` search results carry an affiliation or acronym line, computed in `AgentScope` and matched by `search`
- [x] The entity route (`handlers/entities.rs`) dispatches find and choose like `sections.rs::act`, so the affiliation and funder pickers work the same way
- [x] Three intents: find, choose-and-append, change-row, with the changed row carried in a hidden field
- [x] Contributor and contact point render chosen entries as cards with Change and Remove, and one search below them
- [x] No-match results offer "Propose a new person or organisation" (REQ-3.1)
- [x] A contributor without a role is marked incomplete after a refusal or on revisit; roles render as display-only chips over the checkbox set behind a "Choose roles" disclosure; chip Remove is a named submit by row key and role index
- [x] Rendering tests for the tile (roles, states, accessible names) and for an incomplete contributor
- [x] Rewrite `tests/agent-picker.spec.ts` for the new flow in both passes, including keyboard: arrow keys do not save, Enter does
- [x] Run `cargo test -p mosaic-tiles -p editor-core -p editor-web` and `cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/agent-picker.spec.ts`; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Proposal flow from the card (DEV-7686)

### dsp-repository
- [x] "Propose changes" sits on the card's actions and opens the entity page, with the enhanced-path mechanism (own form, or a navigating `200`) decided first
- [x] "Back to the project" returns to the originating section at the entry, through a hidden section id carried across both redirects, validated against the registry sections for the audience, anchored by field plus entity id
- [x] After discard: back on that section with one confirmation naming the entity (its own marker); the entry leaves the proposals box; a discarded New proposal's referencing rows go too
- [x] Organisation and person pages say "Save draft" like the project form
- [x] Handler tests: the return target, an invalid return target falls back to the project, the discard redirect and confirmation
- [x] Run `cargo test -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: SNSF disciplines (DEV-7710)

### dsp-repository
- [x] A checked-in extraction script and `areas/deposit/editor/core/data/snsf-disciplines.json` (code, English name, domain group; 141 codes in 21 groups) from the prototype's `SNSF_DOMAINS`
- [x] The UNESCO nomenclature extraction script, checking every label against the live vocabulary before writing (`scripts/build-unesco-nomenclature.py`)
- [ ] Deferred (owner, 2026-11-04; skos.um.es unreachable): bundle the verified UNESCO nomenclature and offer it in the picker; follow-up issue, not this PR
- [x] Every well-formed SNSF code in the committed projects is in the extracted list (test)
- [x] "How this is recorded" offers SNSF, UNESCO nomenclature and own wording; an SNSF row is recognised by its leading code in any language and a pick stores `{"en": "<code> <name>"}`; UNESCO stores type `Skos` with the `http` scheme
- [x] Round-trip test: an SNSF and a UNESCO choice survive save, submit and the canonical writer unchanged in shape
- [x] Run `cargo test -p editor-core -p editor-web`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Vocabulary pickers (DEV-7698)

### dsp-repository
- [x] Disciplines options rendered once, not 141 `<option>`s per row on every morph (deferred from phase 3's review)
- [x] Extract the places and periods in use into `core/data/places.json` and `core/data/periods.json` with a checked-in script; periods limited to options with a timespan or an enrichment row
- [x] Disciplines picker over both lists, by code or name, grouped by domain, on the phase 1 combobox
- [x] Spatial coverage picker over the place subset, with a name-plus-link fallback for a place outside it
- [x] Temporal coverage picker over the period subset and `chronontology-periods.json`; own wording kept and still checked against the enrichment table
- [x] Field hints describe the search, not "paste the link"
- [x] Tests: a pick stores the same `{type, url, text}` as the committed entry for that place or period; every picker period option passes `unresolved_temporal_coverage`; an unresolved own-wording period is still refused at submit
- [x] A Playwright test picking one discipline, place and period in both passes
- [x] Run `cargo test -p editor-core -p editor-web` and that Playwright test; both pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: Clean up and gate

### dsp-repository
- [x] Refresh ARCH-MAP.md for the editor and mosaic entries this branch changed (combobox tile, the two `/agents` routes, `Agent.detail`, picker intents), with their fingerprints (deferred from phase 1's review)
- [x] ARCH-MAP.md for phase 2: `entity::origin::Origin` + `handlers::navigate` as a second request-derived-redirect mechanism beside `deposit_shell::safe_next` (registry ids only), `EntityProposalRepository::withdraw_rewriting_draft` as a draft write from the entity route, `agents::proposal_agent`, `form::submit::drop_agent` (deferred from phase 2's review)
- [x] ARCH-MAP.md for phase 3: `editor_core::disciplines` (`snsf`, `snsf_row`, `snsf_text`, the bundled `data/snsf-disciplines.json`), `form::{SNSF_KIND, stored_kind}`, `registry::source_label` (deferred from phase 3's review)
- [x] ARCH-MAP.md for phase 4: `editor_core::vocabulary` (`List`, `Entry`, `Match`, the bundled `data/places.json` and `data/periods.json` as derived data with one writer, `scripts/build-coverage-vocabularies.py`, pinned by the drift tests, and their DPE-data dependency), `disciplines::search`, the `GET …/sections/{section}/vocabulary` route beside `/agents`, the `find-entry` and `pick` intents, `Rows.vocabulary_path`, the review surface rendering the same `control` without a route; Mosaic `combobox_groups` (deferred from phase 4's review)
- [x] Trim comments added on this branch to the "why not the obvious approach" kind
- [x] Delete tests added on this branch that would fail only if a line were deleted
- [x] Fold every `fixup!` commit into its phase commit (`GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash origin/main`), leaving the plan commit plus one commit per issue
- [x] Run `just fmt`, then `just check && just test` outside the sandbox; it passes
- [x] Run `cargo build -p deposit-server --release`, `just css-editor-release` and `just test-e2e-editor` outside the sandbox; it passes in both passes
- [x] Run `eng:test-browser` on Contributors, Contact point, Dataset (disciplines, temporal, spatial coverage) and a person page; no defect reported
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge groups A (`01-`) and B (`02-`) | Marcin | before start | Merging is the owner's decision |
| H2 | Decide whether to correct the malformed SNSF codes in `084E_rich` and `0849_operativetv` and the 17 codes carrying more than one name | RDU | after ship | Project data is RDU's |
| H3 | Confirm the source labels "From the SNSF list" and "From the UNESCO nomenclature" in the PR | Daniela, RDU | after ship | DEV-7710 asks for "UNESCO Thesaurus"; the data is the nomenclature |

## Acceptance Criteria

- [x] A chosen contributor is a card; the search clears; arrow keys don't choose; a contributor without a role is visibly incomplete
- [x] Proposing changes starts on the card and returns to the section; discard ends with one confirmation
- [x] A depositor records an SNSF discipline by searching "10404" or "art history"
- [x] A depositor records Switzerland (GeoNames 2658434) and a PeriodO period without pasting a link
- [x] Every committed project still saves and submits unchanged
- [x] `just check && just test` and `just test-e2e-editor` pass

## Dependencies & Risks

- **Combobox accessibility** is the hardest part to get right; the accessibility reviewer and the keyboard E2E test gate phase 1. Roving focus is the simple version; if the reviewer requires `aria-activedescendant`, phase 1 grows by the signal and its reset.
- **Subset coverage:** a depositor's place may be missing from the subset; the fallback keeps recording possible, and rerunning the extraction script after collections grows the subset.

## Success Metrics

Daniela re-tests DEV-7702's and DEV-7698's steps on stage and reports neither "I didn't see it" nor "search doesn't clear".

## References

- Prototype: `dsp-incubator/metadata-editor-v2/src/workflows/screen-edit-picker.jsx` (`ContributorPicker`, `SearchDrop`, `ContactPointPicker`, `DisciplinePicker`, `GazetteerPicker`, `TemporalPicker`), `store.jsx` (`SNSF_DOMAINS`, catalogs), `unesco-skos.js`
- `areas/deposit/editor/web/src/form/widgets.rs:383-508` (`agent_picker`), `:537`, `:581-721`, `:737-875`
- `areas/deposit/editor/core/src/agents.rs:23`, `:245`; `core/src/form/registry.rs:44-54`, `:391-416`
- `areas/deposit/editor/web/src/entity/mod.rs:323-332`, `handlers/entities.rs:351-417`
- `shared/metadata/src/project.rs:213-225`, `shared/metadata/src/models.rs:4`
- `areas/access/dpe/server/data/temporal-coverage-enrichment.json`, `chronontology-periods.json`; `shared/metadata/src/temporal_coverage.rs:55-80`, `chronontology.rs:89-91`
- DPE's combobox: `areas/access/dpe/web/src/pages/projects/components/project_search_input.rs:61-73`, `areas/access/dpe/server/src/fragments.rs:165-167`
- `areas/deposit/editor/web/src/handlers/sections.rs:705-729`, `:1345-1351`; `pages/section.rs:107-127`, `:1107`; `.github/scripts/check-datastar-attributes.sh`
- WAI-ARIA combobox pattern: https://www.w3.org/WAI/ARIA/apg/patterns/combobox/
