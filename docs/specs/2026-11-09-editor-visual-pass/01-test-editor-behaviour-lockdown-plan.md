---
title: "test: lock down the editor's behaviour before the visual pass"
type: test
date: 2026-11-09
author: "Marcin Procyk"
status: implemented
repositories:
  - name: dsp-repository
    path: /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-ux-ui/dsp-repository
linear: DEV-7769
linear_project: "Metadata editor is ready to show to depositors"
---

# test: lock down the editor's behaviour before the visual pass

## Overview

Before the visual pass (DEV-7770 to DEV-7774) restyles any page, every flow on those pages gets an E2E test that asserts behaviour: what happens, where it lands, what is stored. Gaps only; existing coverage is listed below, not duplicated. The plan also records an accessibility and contrast baseline for the visual-pass issues to fix. No production code changes.

**Base:** the issue says to stack on #568. That whole stack (#563 → #568) merged on 2026-11-06, and `git diff origin/feat/dev-7690-editor-form-polish origin/main -- areas/deposit` is empty, so this branches from `origin/main`.

| Phase | Change |
|---|---|
| 1 | Harness: a second RDU account, per-pass projects, shared helpers |
| 2 | Re-point the existing specs' styling-class selectors |
| 3 | Sign-in, sign-out, header navigation, 403/404 |
| 4 | Depositor accounts: edit and remove |
| 5 | Projects (RDU): create refusals, remove project |
| 6 | Section form: discard draft, take the submission back |
| 7 | Review: per-field decisions, approve refusal, what an approval stores |
| 8 | Review: request changes, reject, and the depositor's notices |
| 9 | Review: take over |
| 10 | Proposals: new person and organisation, entity decisions |
| 11 | Accessibility and contrast baseline |

## Problem Statement / Motivation

The visual pass changes markup and classes on every editor page. Rendering snapshots (`areas/deposit/editor/web/src/pages/snapshots/`) will churn by design and say nothing about behaviour. The Rust handler tests cover server rules but not the assembled page in a browser, with and without JavaScript. The E2E suite covers the main journey but has no test for about half the flows the visual pass touches (inventory below), so a restyle that breaks one of them would pass CI.

## Coverage inventory

Existing coverage in `areas/deposit/editor/web-e2e-tests/tests/`. "Gap" rows are what this plan adds.

| Flow (issue scope) | Existing coverage | Gap → phase |
|---|---|---|
| Sign in with a login code | `support/auth.setup.ts` (both roles, every run) | Wrong code refused; `next` return after sign-in → 3 |
| Sign out | none | → 3 |
| Header and navigation | none (only a11y scans of pages) | Menu per role, each link lands, queue count, `/` redirect → 3 |
| 403 page | `accessibility.spec.ts` (axe scan only) | Status and that nothing RDU-only renders → 3 |
| 404 page | `accessibility.spec.ts` (axe scan only) | Status, signed in and out → 3 |
| Problem page | none | Not reachable by E2E: renders only on a database failure or an unreadable stored submission (`handlers/mod.rs::storage_error`, `handlers/review.rs::unreadable_submission`, identity's `storage_error`). Fault injection would be a production change. Covered by `deposit-shell` rendering tests; listed in the PR as uncovered. |
| Your projects (depositor list) | `status.spec.ts` (state column), `journey.spec.ts` (created project row) | Only assigned projects listed → 5 |
| Projects (RDU) | `journey.spec.ts` (created project listed under New projects) | none beyond 5's refusals |
| Create a project | `journey.spec.ts` (create, land on overview, depositor edits) | Malformed and already-published shortcode refused → 5 |
| Remove project | none | → 5 |
| What the states mean | `status.spec.ts` (link from list, five states, wait, vocabulary) | none |
| Section form: save | `journey.spec.ts`, `status.spec.ts`, `viewport.spec.ts` (no-JS save notice) | none |
| Section form: submit | `journey.spec.ts`, `status.spec.ts`; refused submit in `viewport.spec.ts` | none |
| Section form: discard draft | none | → 6 |
| Section form: take the submission back | none | → 6 |
| Review-round notices | approved notice in `status.spec.ts`; withdrawn, changes-requested, rejected: none | → 6, 8 |
| Locked form | `journey.spec.ts` (no name input after submit), `accessibility.spec.ts` (locked message) | Locked while In review, take-back still offered → 6 |
| Add/remove rows | `row-controls.spec.ts`, `round-trips.spec.ts`, `language-rows.spec.ts` | none |
| Pickers | `agent-picker.spec.ts`, `vocabulary-pickers.spec.ts`, `canton.spec.ts` | none |
| Person/organisation proposal page | `proposals.spec.ts` (change proposal from a card, back link, discard) | New person, new organisation, entity-form save and rows → 10 |
| Review queue: claim | `journey.spec.ts`, `status.spec.ts`, `approved.spec.ts` | none |
| Review: take over | none | → 9 |
| Review: per-field decisions | none (`accept all` is clicked only when present) | → 7 |
| Review: approve | `journey.spec.ts`, `status.spec.ts`, `approved.spec.ts` | Refused while undecided; what the record holds → 7 |
| Review: request changes | none | → 8 |
| Review: reject | none | → 8 |
| Review: entity decisions | none | → 10 |
| Approved records | `approved.spec.ts` (waiting for release, pull-request link) | none |
| Approved records: discard | `approved.spec.ts` | none |
| Depositor accounts: create | `support/auth.setup.ts` (creates the shared depositor) | none |
| Depositor accounts: edit, remove | none | → 4 |

Specs outside the issue's flows, kept as they are: `form-alignment.spec.ts` and the geometry tests in `viewport.spec.ts` assert layout (DEV-7691, DEV-7682). They are visual by intent, so the visual pass updates them on purpose. `character-count.spec.ts` covers the Short description counter.

## Proposed Solution

New spec files in `areas/deposit/editor/web-e2e-tests/tests/`, one per flow group, running in both Playwright projects (`chromium-js`, `chromium-nojs`). Each asserts outcomes a restyle cannot change: the URL landed on, HTTP status, a visible message's text, a field's value after a fresh `GET`, the project's state cell on `/projects`, and, for approvals, the record served by `GET /api/v1/approved-records`.

**Selectors** follow the DEV-6642 learning (`dasch-specs/learnings/best-practices/visual-parity-oracle-worktree-ab-for-big-bang-ui-refactors.md`): roles and accessible names, ids, form `name` attributes, `formaction`, visible text. Never a styling class (`font-semibold`, `btn-*`, `badge-*`, `alert-*`). App-defined structural classes the existing specs already use (`repeatable-row`, `combobox`) are acceptable where no role exists. Submit buttons are scoped to `main` or named, never `button[type="submit"]` (the header's Sign out is one).

**Isolation:** the suite shares one server across both passes, and a draft is per (user, project). Every spec that walks a lifecycle gets one project **per pass**, keyed by Playwright project name, as `STATUS_SHORTCODES` does. A spec that signs out does it in a fresh sign-in, never with the shared cookie jars: sign-out ends the session server-side and would sign every later test out.

## Technical Considerations

- **Second RDU account.** Take-over needs two reviewers. `EDITOR_RDU_EMAILS` takes a comma-separated list (`areas/deposit/server/src/config.rs`), so a second address in `playwright.config.ts` is a test-config change. Display names are the email's local part (`identity/web/src/handlers/accounts.rs`), so the second address is `reviewer-b@dasch.swiss`: `rdu2` would contain `rdu` and make a name match ambiguous.
- **Two-step controls.** Approve, Request changes and Reject each open a confirmation first; refusals (undecided fields, missing note) appear only after the confirming click. The review note is a `required` textarea, so an empty note never posts: the server's note-required refusal is reachable only with a whitespace-only note.
- **Notices only the enhanced path renders.** "Review decisions saved." renders only on the Datastar path; the plain path redirects with no marker (`handlers/review.rs::saved`). Assert it under `isJs()` only, and in both passes assert the persisted outcome on a fresh load. "Submission withdrawn" survives the plain-path redirect (`?done=withdrawn`) and is asserted in both. "You are now reviewing this submission." renders on neither path: the take-over form (`pages/review.rs::claim_banner`) has no `data-on:submit`, so it is a plain post and redirect with JavaScript on too. Filed as DEV-7809; asserted in the JS pass by a dedicated test marked `test.fail()` with that id.
- **Sign-in budgets.** Wrong codes count toward an account-level lockout and every send toward a per-address daily cap, both alive for the server's lifetime. Tests that sign in, sign out or enter a wrong code use their own throwaway depositor, never `RDU_EMAIL` or `DEPOSITOR_EMAIL`. A test that drives sign-in from a redirect cannot use `signIn()` (it starts at a bare `/login`); it fills the form on the page it was redirected to. With `reuseExistingServer` on locally, a server already running from an earlier run lacks the account: stop it before the first local run.
- **Shortcodes.** New entries go in `DEPOSITOR_SHORTCODES` (the setup assigns them to the shared depositor) and must exist in `areas/access/dpe/server/data/projects/`. Unused today: `0811`, `0812`, `0118`, `0119`, `0120`, `0121`, `0803`, `0804`, `0805`, `0806`, `0807`, `0810`. Avoid `0801` (five files share it). Avoid `0114` and `0116`: their names contain "Committee" and "Transfers", which fail the vocabulary check. Created projects use codes absent from the data set, as `E2E1`/`E2E2` do. Adding a shortcode to the list does not reach an existing local server's depositor, which was created on an earlier run: same restart note as above.
- **`auth.setup.ts` creates the depositor only if absent**, so its shortcode list is fixed at first creation per server. CI starts fresh; locally, restart.
- **Approved-record assertions** read `GET /api/v1/approved-records` through Playwright's `request` fixture (works without JavaScript). The route is rate-limited per IP: call it once per assertion, never in a poll loop.
- **Login codes** come from the server log (`support/server-log.ts`). Take `logOffset()` before triggering the send. The resend cooldown is 1 s in the suite.
- **Vocabulary.** Every new depositor-facing page state (withdrawn, changes requested, rejected notices, discard result) runs the REQ-2.2 forbidden-words check. Move `expectNoForbiddenVocabulary` from `status.spec.ts` to `tests/support/vocabulary.ts` so the new specs share it.
- **WCAG exemptions.** The issue asks for disabled-state contrast against AA. WCAG 1.4.3 and 1.4.11 exempt inactive components, so disabled states are measured and reported as information, not as failures. Non-text contrast (3:1) applies to borders only where the border is what identifies the control (an input's edge), not to decorative card borders.
- **New axe scans vs. no fixes.** A scan added for a page that fails today makes CI red, and this issue allows no fixes. Only scans that pass now are committed. A failing page's violations go in the PR list, and the visual-pass issue that fixes them adds the scan.

### Example: a per-pass project and a stored-value assertion

```ts
// playwright.config.ts
export const DECISIONS_SHORTCODES: Partial<Record<string, string>> = {
  "chromium-js": DEPOSITOR_SHORTCODES[17],
  "chromium-nojs": DEPOSITOR_SHORTCODES[18],
};

// review-decisions.spec.ts
const res = await request.get("/api/v1/approved-records");
const record = (await res.json()).records.find(
  (r: { shortcode: string }) => r.shortcode === shortcode(),
);
expect(record.project.name, "an accepted field ships the submitted value").toBe(edited);
expect(record.project.shortDescription, "a reverted field ships the published value").toBe(published);
```

## Implementation Phases

#### Phase 1: Harness for the new specs

### dsp-repository
- [x] Add `reviewer-b@dasch.swiss` to `EDITOR_RDU_EMAILS` in `playwright.config.ts`, export it as `RDU2_EMAIL`, and add an `RDU2_STATE` jar that `auth.setup.ts` signs in and saves
- [x] Add `createDepositor(page, email, shortcodes)` to `tests/support/` for the throwaway accounts Phases 3 and 4 use, with emails unique per pass
- [x] Add the shortcodes Phases 4 to 10 need to `DEPOSITOR_SHORTCODES` from the unused list in Technical Considerations, each behind a named per-pass export with a doc comment saying why it is its own project
- [x] Move `expectNoForbiddenVocabulary` and `visibleText` from `status.spec.ts` to `tests/support/vocabulary.ts`; `status.spec.ts` imports them
- [x] Add `tests/support/review.ts` with `claimReview(page, shortcode)`, `submitAs(browser, shortcode, edit)` and `approve(page)`; `journey.spec.ts`, `status.spec.ts`, `approved.spec.ts` and `accessibility.spec.ts` keep their inline copies (no churn in passing specs)
- [x] Add `perPass({ MAP })` to `tests/support/` that returns this pass's entry or throws naming the map, replacing nothing existing
- [x] Run `just lint-e2e`; this phase's files lint clean, and the recipe reports only the 8 errors the base revision already has (journal: Side findings, Deviations)
- [x] Run `cargo build -p deposit-server --release`, `just css-editor-release`, then `just test-e2e-editor`; every existing test still passes in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Restyle-proof selectors in the existing specs

### dsp-repository
- [x] Replace `span.font-semibold` in `support/agents.ts` `cardName` with a locator built on role, accessible name or text, with no styling class
- [x] `optionName` in `support/agents.ts`: no role, accessible-name or text locator separates an option's name from its kind badge and detail without a production change, so it keeps `span.font-semibold` under the next item (journal: Side findings)
- [x] Replace the two `span.font-semibold` locators in `vocabulary-pickers.spec.ts` the same way
- [x] Where no such locator exists without a production change, keep the selector and list it in the PR body under "Restyle-sensitive selectors" with its file and line
- [x] Run `just lint-e2e`; this phase's files lint clean, and the recipe reports only the 8 errors the base revision already has (journal: Side findings, Deviations)
- [x] Run `just test-e2e-editor`; `agent-picker`, `proposals` and `vocabulary-pickers` pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Sign-in, sign-out, navigation and error pages

### dsp-repository
- [x] `tests/shell.spec.ts`: one wrong six-digit code, for a throwaway depositor, stays on `/login/code` with the code-rejected message, and a protected page still redirects to `/login`
- [x] `tests/shell.spec.ts`: a signed-out visit to `/projects/{code}/sections/overview` redirects to `/login?next=…`; completing sign-in from that page (not via `signIn()`) lands on that section page
- [x] `tests/shell.spec.ts`: Sign out, for a freshly signed-in throwaway depositor, lands signed out, and `/projects` then redirects to `/login`
- [x] `tests/shell.spec.ts`: the depositor's header offers only Projects; RDU's offers Projects, Review queue, Approved records and Accounts, and each link lands on `/projects`, `/review`, `/approved`, `/depositors`
- [x] `tests/shell.spec.ts`: on one load of `/review`, the Review queue label's count equals the rows of the pending-submissions table (claimed and unclaimed), not the drafts table
- [x] `tests/shell.spec.ts`: `/` redirects a signed-in user to `/projects`
- [x] `tests/shell.spec.ts`: a depositor on `/review` gets status 403 and a page with no queue table
- [x] `tests/shell.spec.ts`: an unknown path answers 404 with the not-found text, signed in and signed out (the 404 page renders no signed-in header in either case; don't assert one)
- [x] Run `just lint-e2e`; it passes (read as: this phase's files lint clean, and the recipe reports only the 8 errors the base revision already has; journal: Deviations)
- [x] Run `just test-e2e-editor`; the new tests pass in both passes (run as `npx playwright test tests/shell.spec.ts` with its setup project, per the owner's 2026-11-09 speed decision)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Depositor accounts: edit and remove

### dsp-repository
- [x] `tests/depositors.spec.ts`: RDU creates a throwaway depositor per pass with `createDepositor`
- [x] `tests/depositors.spec.ts`: editing its name and shortcodes with Save changes lands on `/depositors` showing the new values, and a fresh load of its edit form holds them
- [x] `tests/depositors.spec.ts`: saving an unknown shortcode (neither published nor created) keeps the account, and the warning shows on its `/depositors` row and its edit form (not on the save response, which redirects)
- [x] `tests/depositors.spec.ts`: the remove confirmation's cancel keeps the account listed
- [x] `tests/depositors.spec.ts`: confirming removal drops it from `/depositors`; a sign-in attempt with its email then reaches no session: any six digits on `/login/code` are rejected (never `waitForLoginCode`, which would time out since no code is sent)
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: Projects (RDU): create refusals and remove project

### dsp-repository
- [x] `tests/projects.spec.ts`: the depositor's `/projects` lists its assigned projects and no unassigned one (a shortcode from the data set not in `DEPOSITOR_SHORTCODES`)
- [x] `tests/projects.spec.ts`: Create project with a malformed shortcode stays on `/projects/new` with the shortcode field marked invalid and its letters-and-digits message, and creates nothing
- [x] `tests/projects.spec.ts`: Create project with a published shortcode stays on `/projects/new` with "Project {code} is already published." and creates nothing
- [x] `tests/projects.spec.ts`: RDU creates a per-pass project (its own pair of codes absent from the data set, like `CREATED_SHORTCODES`), then its remove page's Cancel keeps it listed
- [x] `tests/projects.spec.ts`: Remove project lands on `/projects` with "Project {code} removed.", its row is gone, and `/projects/{code}/remove` answers 404 (the section URL does not 404: it opens an unknown project blank by design)
- [x] `tests/projects.spec.ts`: a created project holding a submission shows that it is submitted and offers no removal
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: Section form: discard draft, take the submission back, locked while in review

### dsp-repository
- [x] `tests/section-lifecycle.spec.ts`: after saving a draft (Discard draft needs one), Discard draft then the Keep the draft link leaves the saved value in place
- [x] `tests/section-lifecycle.spec.ts`: Discard draft then Yes, discard the draft restores the published value on a fresh load, the state cell reads Online, and the page passes the vocabulary check
- [x] `tests/section-lifecycle.spec.ts`: after submit, Take the submission back then the Keep waiting link leaves the form locked and the state Submitted
- [x] `tests/section-lifecycle.spec.ts`: Yes, take it back shows the Submission withdrawn notice, makes the form editable with the submitted value kept, sets the state to Draft, and passes the vocabulary check
- [x] `tests/section-lifecycle.spec.ts`: after RDU claims the submission, the depositor's form stays locked (no Save draft, no Submit for review) and still offers Take the submission back
- [x] `tests/section-lifecycle.spec.ts`: taking it back while In review sets the state to Draft and removes the project from RDU's pending-submissions table
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: Review: per-field decisions and what an approval stores

### dsp-repository
- [x] `tests/review-decisions.spec.ts`: the depositor submits a per-pass published project (Revert exists only for published ones) with two changed plain-text fields, expected values read from the page rather than typed into the test
- [x] `tests/review-decisions.spec.ts`: RDU picks Accept on one and Revert on the other, each radio scoped by its group's name (`<field label> — decision`), then Save review decisions; a fresh load shows both choices kept (the "Review decisions saved." notice asserted under `isJs()` only)
- [x] `tests/review-decisions.spec.ts`: with one field set back to Not reviewed yet and saved, Approve then its confirmation is refused with "Every change has to be decided…" and the project stays in the queue
- [x] `tests/review-decisions.spec.ts`: an in-place edit of the accepted field is saved with the decisions
- [x] `tests/review-decisions.spec.ts`: after approving, `GET /api/v1/approved-records` holds the record with the substituted value for the accepted field and the published value for the reverted one
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: Review: request changes, reject, and the depositor's notices

### dsp-repository
- [x] `tests/review-outcomes.spec.ts`: Request changes, a whitespace-only note, then its confirmation is refused with "A note is required: it is the only thing the depositor will see." and the submission stays in the queue
- [x] `tests/review-outcomes.spec.ts`: Request changes with a note, after accepting one field, sets the depositor's state to Draft and shows "RDU asked for changes" with the note on the section page
- [x] `tests/review-outcomes.spec.ts`: on that returned draft, the field RDU accepted renders as read-only text with no input of its name, and the other field stays editable
- [x] `tests/review-outcomes.spec.ts`: Reject, a whitespace-only note, then its confirmation is refused with the same note-required message
- [x] `tests/review-outcomes.spec.ts`: Reject with a note, on a second per-pass project, removes it from the queue, keeps the depositor's draft values, and shows "RDU rejected this submission" with the note
- [x] `tests/review-outcomes.spec.ts`: both depositor notices pass the vocabulary check
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 9: Review: take over

### dsp-repository
- [x] `tests/review-takeover.spec.ts`: RDU claims a per-pass submission; the second RDU account opening `/review/{code}` sees "Somebody else is reviewing this" naming `rdu`, and the first sees no such banner
- [x] `tests/review-takeover.spec.ts`: after Take over the review, the second account's fresh load shows no banner and the first account's fresh load names `reviewer-b` as the holder ("You are now reviewing this submission." is asserted in the JS pass by a dedicated `test.fail()` test for DEV-7809, see Technical Considerations)
- [x] `tests/review-takeover.spec.ts`: a decision the second account saves is what the first account then sees (last save wins)
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 10: Proposals: new person and organisation, entity decisions

### dsp-repository
- [x] `tests/proposals-new.spec.ts`: searching the contributors append box for a string that matches nothing (via `appendBox`, plus the Search button without JS; not `searchToAppend`, which expects matches) offers Propose a new person; choosing it re-renders the section with "Proposal started", and its "Open it to fill in the details" link lands on `/entities/person-…`
- [x] `tests/proposals-new.spec.ts`: on that entity form, Save draft keeps the typed name on a fresh load, and Back to the project shows it under Proposed persons and organisations
- [x] `tests/proposals-new.spec.ts`: the same for Propose a new organisation
- [x] `tests/proposals-new.spec.ts`: on the person form, adding and removing a row of one repeatable string field (pick one whose add and remove buttons have accessible names, named in the test) changes its row count, in both passes
- [x] `tests/entity-decisions.spec.ts`: the depositor submits a per-pass project referencing a new person proposal; RDU's review shows the proposal row with Accept and Reject
- [x] `tests/entity-decisions.spec.ts`: with every field decided and the proposal on Not reviewed yet, Approve then its confirmation is refused with the undecided-proposal message
- [x] `tests/entity-decisions.spec.ts`: Reject on the proposal while a field still references it, then Approve and its confirmation, is refused naming the referencing field
- [x] `tests/entity-decisions.spec.ts`: Accept on the proposal and approve; `GET /api/v1/approved-records` holds the record with that person under `entities` as a new entity
- [x] Run `just lint-e2e`; it passes
- [x] Run `just test-e2e-editor`; the new tests pass in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 11: Accessibility and contrast baseline

### dsp-repository
- [x] Run axe (the `accessibility.spec.ts` tags) once over the in-scope pages the sweep lacks: `/login/code`, `/projects/new`, a project's remove page, an entity proposal page, `/review/{code}` with a take-over banner and with entity rows, the discard-draft and take-back confirmations, `/approved/{id}/discard`, `/depositors/{id}/edit`, `/depositors/{id}/remove`
- [x] Add to `accessibility.spec.ts` a scan for each of those pages with no violations
- [x] Measure contrast on every in-scope page with a scratch Playwright script outside the repository: text, badges, input and button borders, focus rings (after `focus()`), hover (after `hover()`), and disabled states, against the background each sits on
- [x] Write the results into the PR body under "Accessibility baseline": one row per failure with page, element, state, measured ratio and required ratio, plus each axe violation from the unscanned pages, plus disabled states as a separate informational list
- [x] Run `just test-a11y-editor`; it passes
- [x] Run `just test-e2e-editor`; the whole suite passes in both passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Done 2026-11-09: the baseline had no failures; its near-threshold items are in the accessibility criterion of each visual-pass issue (DEV-7770 to DEV-7774) | Marcin | after ship | Splitting findings across issues is a scoping decision on someone else's tickets |

## Acceptance Criteria

- [ ] Every "Gap" row in the coverage inventory has a test asserting behaviour (URL, status, message text, stored value, state cell), with no styling-class selector
- [ ] Every new test runs and passes in `chromium-js` and `chromium-nojs`
- [ ] No existing test is duplicated; the PR body carries the coverage inventory
- [ ] `git diff origin/main --stat` touches only `areas/deposit/editor/web-e2e-tests/` and this plan
- [ ] The PR body lists each contrast failure with page and element, axe violations on unscanned pages, disabled states as information, the problem page as uncovered with the reason, and any restyle-sensitive selector left in place
- [ ] The PR ticks `allow-many-commits` (each phase is a self-contained test commit)

## Dependencies & Risks

- **Suite runtime.** Ten new spec files run twice, serially (`workers: 1`). Expect the E2E job to grow noticeably; measure before and after in the PR. Not a blocker.
- **Stale local server.** `reuseExistingServer` keeps a server without the second RDU account or new shortcodes. Stop it before the first local run of Phase 1.
- **A flow that behaves wrongly today.** These tests pin current behaviour. If a test reveals a bug, it is listed in the PR and filed separately; the test is written against the correct behaviour with `test.fail()` and the issue id, never adjusted to the bug.
- **Blocks** DEV-7770 and DEV-7618.

## Success Metrics

- Inventory gap rows covered: 0 of 20 today → 20 of 20 (the problem page excluded, with its reason).
- Baseline: `just test-e2e-editor` duration on `origin/main`, measured in Phase 1, against the final run in Phase 11, both stated in the PR.

## References

- Issue: [DEV-7769](https://linear.app/dasch/issue/DEV-7769)
- Mockups: <https://claude.ai/code/artifact/e9cbc566-cdad-4ed6-aed7-bf95400e3846>
- Suite config and isolation rules: `areas/deposit/editor/web-e2e-tests/playwright.config.ts`
- Editor guide (selectors, both passes, vocabulary): `areas/deposit/editor/CLAUDE.md`
- Review surface: `docs/src/editor/review.md`; entity proposals: `docs/src/editor/entity-proposals.md`; approved records API: `docs/src/editor/publishing-to-git.md`
- Learnings: `dasch-specs/learnings/best-practices/visual-parity-oracle-worktree-ab-for-big-bang-ui-refactors.md`, `dasch-specs/learnings/best-practices/test-fixture-isolation-shared-data-scope.md`
