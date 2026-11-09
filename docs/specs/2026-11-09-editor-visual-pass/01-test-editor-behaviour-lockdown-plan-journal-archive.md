# Execution Journal Archive: 01-test-editor-behaviour-lockdown-plan

## Phase 1

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 1.1 | dsp-repository | web-e2e-tests/playwright.config.ts, tests/support/auth.setup.ts | — | "Add reviewer-b@dasch.swiss … RDU2_STATE jar" | server seeded with both RDU addresses; setup saves RDU2_STATE | playwright.config.ts § RDU_EMAIL, webServer.env; auth.setup.ts § setup() | — |
| 1.2 | dsp-repository | tests/support/depositors.ts | — | "Add createDepositor(page, email, shortcodes)" | helper creates an account through /depositors/new and asserts its row; emails unique per pass | auth.setup.ts § depositor provisioning | — |
| 1.3 | dsp-repository | web-e2e-tests/playwright.config.ts | — | "Add the shortcodes Phases 4 to 10 need" | 12 unused data-set codes appended, six per-pass exports with doc comments | playwright.config.ts § DEPOSITOR_SHORTCODES; plan § Technical Considerations "Shortcodes" | — |
| 1.4 | dsp-repository | tests/status.spec.ts, tests/support/vocabulary.ts | — | "Move expectNoForbiddenVocabulary and visibleText" | status.spec.ts imports from support/vocabulary.ts; behaviour unchanged | status.spec.ts § FORBIDDEN / visibleText | — |
| 1.5 | dsp-repository | tests/support/review.ts | — | "Add tests/support/review.ts with claimReview, submitAs, approve" | three helpers, inline copies untouched | status.spec.ts § claimReview; journey.spec.ts § approve flow | — |
| 1.6 | dsp-repository | tests/support/per-pass.ts | — | "Add perPass(map)" | returns this pass's entry or throws naming the map | status.spec.ts § shortcode() | — |
| 1.7 | dsp-repository | — (gate runs) | 1.1–1.6 | "Run just lint-e2e", "Run cargo build … just test-e2e-editor" | lint and the full suite pass in both passes | — | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 1.1 | dsp-repository | complete | 466b2310 | RDU2_EMAIL reviewer-b@dasch.swiss seeded; setup signs it in, checks /review is 200, saves RDU2_STATE | none |
| 1.2 | dsp-repository | complete | 1582ef73 | tests/support/depositors.ts: createDepositor(page, email, shortcodes) and throwawayEmail(purpose), unique per pass and retry | none |
| 1.3 | dsp-repository | complete | 3632071a | 12 unused data-set codes appended to DEPOSITOR_SHORTCODES ([15]-[26]); six per-pass maps LIFECYCLE, DECISIONS, CHANGES_REQUESTED, REJECTED, TAKEOVER, ENTITY_DECISIONS_SHORTCODES | none |
| 1.4 | dsp-repository | complete | 7d64b026 | FORBIDDEN, visibleText, expectNoForbiddenVocabulary moved verbatim to tests/support/vocabulary.ts; status.spec.ts imports the check | none |
| 1.5 | dsp-repository | complete | bd98f1b6 | tests/support/review.ts: claimReview(page, shortcode), submitAs(browser, shortcode, edit), approve(page) (no decisions, no outcome assertion); exercised by an uncommitted scratch spec in both passes | none |
| 1.6 | dsp-repository | complete | 1a112ea0 | tests/support/per-pass.ts: perPass({ MAP }) returns the pass entry or throws naming MAP | none |
| 1-review-1a | dsp-repository | complete | bf84f0ff | fixup of 466b2310: setup comment and title name all three jars; RDU2 check also asserts the URL stayed /review | none |
| 1-review-1b | dsp-repository | complete | 73d6698a | fixup of 3632071a: TYPED_SHORTCODE comment reflow reverted; LIFECYCLE_SHORTCODES reason reworded | none |
| 1-review-1c | dsp-repository | complete | 9c8f47ea | fixup of 7d64b026: doc comment on expectNoForbiddenVocabulary | none |
| 1-review-1d | dsp-repository | complete | f7d275c6 | fixup of bd98f1b6: submitAs asserts the locked text and bounds its poll; approve without .first() | none |
| 1-review-1e | dsp-repository | complete | 6af7b99c | fixup of 1a112ea0: perPass throws unless given exactly one map | none |
| 1-review-2a | dsp-repository | complete | 0c8c4953 | fixup of 3632071a: 0114 and 0116 replaced by 0811 and 0812 (their names hold "Committee" and "Transfers", which failed status.spec.ts's vocabulary check on /projects in both passes); DEPOSITOR_SHORTCODES doc states the constraint | none |
| 1.7 | dsp-repository | complete | — | gates: `just test-e2e-editor` 136 passed, 23 skipped, 0 failed in 121 s; `just lint-e2e` reports the base revision's 8 errors and none in this phase's files; `tsc --noEmit` clean | none |

### Settled side findings
- [settled] Phase 1: for Phase 3: `signIn` is exported from `tests/support/auth.setup.ts`, a file whose top level calls `setup(...)`; a spec importing it from there may register or reject the setup test (unverified). Phase 3's throwaway sign-ins are better served by moving `signIn` to a support module without a top-level `setup(`, e.g. `tests/support/sign-in.ts`, that `auth.setup.ts` imports (dune-reviewer DUNE-005; check with `npx playwright test --list` first).
  settled: 2026-11-09, `signIn` moved to tests/support/sign-in.ts, which auth.setup.ts imports, in 40134258

## Phase 2

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 2.1 | dsp-repository | web-e2e-tests/tests/support/agents.ts, web-e2e-tests/tests/agent-picker.spec.ts | — | P2 cb1 (agents.ts), cb3 | cardName locates by exact text, no styling class; optionName kept if no role/name/text locator exists, recorded in Side findings | plan Phase 2; widgets.rs slot_card / agent options | — |
| 2.2 | dsp-repository | web-e2e-tests/tests/vocabulary-pickers.spec.ts | — | P2 cb2 | the two option filters locate the label by exact text | plan Phase 2; vocabulary.rs detail = code / GeoNames id | — |
| 2.gate | dsp-repository | — | 2.1, 2.2 | P2 cb4, cb5 | lint-e2e: no error beyond base's 8; agent-picker, proposals, vocabulary-pickers pass in both passes | brief test_command | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 2.1 | dsp-repository | done | 442413cf | `cardName(card, name)` matches the card text exactly (no styling class); agent-picker asserts it visible; `optionName` kept (Side findings). Canary: a wrong name fails the assertion | — |
| 2.2 | dsp-repository | done | 780ad7c6 | the place and discipline option filters match the label by exact text; details are GeoNames ids and SNSF codes, never a label | — |
| 2-review-1a | dsp-repository | done | 76bab7dc | review fix (fixup of 442413cf): cardName scoped to the card's first `p` (a person's detail line or the open picker's option could pass for the name); assertions `toBeAttached`, as the replaced `toHaveText` checked no visibility. Verified: consistency, typescript, simplicity, dune Warnings on the card-wide match; `core/src/agents.rs` affiliation_detail | — |
| 2-review-2a | dsp-repository | done | 84880907 | review fix round 2 (fixup of 442413cf), comments only: cardName names `toBeAttached` as its matcher; optionName covers vocabulary options (no badge). From re-review Suggestions (consistency, dune, simplicity); re-review found no Critical or Warning | — |
| 2.gate | dsp-repository | done | — | after the fix rounds: `just lint-e2e` 8 errors, the base revision's set (this phase's lines add none); `just test-e2e-editor` 136 passed / 23 skipped, as Phase 1's baseline | — |

## Phase 3

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 3.1 | dsp-repository | web-e2e-tests/tests/support/sign-in.ts (new), web-e2e-tests/tests/support/auth.setup.ts | — | none (preparatory refactor, Side finding DUNE-005) | `signIn` lives in a module with no top-level `setup(`; setup project still passes in both passes | journal Side findings "Phase 1: for Phase 3" | — |
| 3.2 | dsp-repository | web-e2e-tests/tests/shell.spec.ts (new) | 3.1 | Phase 3 items 1 to 8 (wrong code, next return, sign out, header menus, queue count, `/` redirect, 403, 404) | one test per checkbox, passes in chromium-js and chromium-nojs | plan "#### Phase 3", Technical Considerations "Sign-in budgets" | — |
| 3.3 | dsp-repository | none (gates) | 3.2 | Phase 3 items 9 and 10 (lint, test run) | phase files lint clean, no lint error beyond the base's 8; shell.spec.ts passes in both passes | brief test_command | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 3.1 | dsp-repository | done | 40134258 | `signIn` moved verbatim to tests/support/sign-in.ts (no top-level `setup(`); auth.setup.ts imports it. Setup project passes | — |
| 3.2 | dsp-repository | done | 3679048f | tests/shell.spec.ts, 8 tests (one per checkbox), 17 passed with setup in both passes; throwaway depositors for wrong code, `next` return and sign-out. Canaries (right code entered, Sign out skipped, queue count +1) each fail. Queue count is 0 = 0 in a targeted run (Deviations) | — |
| 3.3 | dsp-repository | done | — | gates: `just lint-e2e` reports the base's 8 errors and none in this phase's files (sign-in.ts, auth.setup.ts, shell.spec.ts); `npx playwright test tests/shell.spec.ts` 17 passed in both passes | — |
| 3-review-1a | dsp-repository | done | 8f7e98cc | review fix (fixup of 40134258): sign-in.ts exports requestCode and enterCode, signIn composes them; submit named "Sign in". Verified: simplicity Warning, dune DUNE-001 (two writers of the login flow) | — |
| 3-review-1b | dsp-repository | done | abe5ecf9 | review fix (fixup of 3679048f): redirect test asserts BASE_URL+section (suffix held on /login/code?next=…; typescript Critical, canary fails now); menu links clicked from another page (consistency, typescript Warnings); imports the sign-in steps; sign-out test renamed, file comment budget wording. 17 passed in both passes | — |

## Phase 4

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 4.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/support/depositors.ts | — | none (preparatory) | `depositorRow(page, email)` exported; `createDepositor` uses it; shell.spec.ts green in both passes | plan Phase 4; support/depositors.ts | — |
| 4.2 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/depositors.spec.ts | 4.1 | Phase 4 items 1-7 (five depositors.spec.ts items, lint-e2e, test-e2e-editor) | one test per behaviour checkbox, green in chromium-js and chromium-nojs; phase files lint clean | plan Phase 4; identity web pages/depositors.rs, handlers/login.rs | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 4.1 | dsp-repository | done | 9367047c | `depositorRow(page, email)` exported from support/depositors.ts; `createDepositor` uses it. shell.spec.ts 17/17 in both passes. | — |
| 4.2 | dsp-repository | done | 9388edd1 | tests/depositors.spec.ts: four tests, each creating its own throwaway via `createDepositor` (item 1 is their setup, no test of its own: creation is covered by auth.setup.ts); unknown code `NOSUCH1`. 9/9 with setup in both passes; `just lint-e2e` reports only the base revision's 8 errors, none in this phase's files. | — |
| 4-review-1a | dsp-repository | done | 1b9633ea | fixup of 9388edd1: removal test also asserts "No code to show" on /login/code (security and typescript Warnings: six digits alone are refused for a live account too; canary with a never-removed throwaway fails there); Accounts heading anchors the empty /depositors (typescript Warning); CODE_REJECTED moved to support/sign-in.ts, shell.spec.ts imports it (simplicity and dune Warnings). depositors + shell specs 25/25 in both passes. | — |

## Phase 5

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 5.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/projects.spec.ts, areas/deposit/editor/web-e2e-tests/playwright.config.ts | — | Phase 5 items 1-6 (projects.spec.ts) | six tests pass in both passes with setup; biome and tsc clean on the two files | plan #### Phase 5; web/src/handlers/projects.rs, pages/projects.rs | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 5.1 | dsp-repository | done | 9037d388 | projects.spec.ts: depositor list scoped to assignments, malformed and published shortcode refused, remove Cancel and Remove (404 after), submitted created project not removable; E2E3/E2E4 assigned, E2E5/E2E6 unassigned | — |
| 5-review-1a | dsp-repository | done | 83211642 | fixup of 5.1: retry guard in submitComplete, setup failure message, maps moved beside CREATED_SHORTCODES, DEPOSITOR_SHORTCODES names E2E5/E2E6 and 0813 | — |
| 5-review-2a | dsp-repository | done | 2057c251 | fixup of 5.1: submitComplete's retry guard keys on the locked form's message, not on a missing Save draft | — |

## Phase 6

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 6.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/section-lifecycle.spec.ts | none | Phase 6 checkboxes 1-6 (section-lifecycle.spec.ts) | six serial tests on LIFECYCLE_SHORTCODES pass in chromium-js and chromium-nojs; biome and tsc clean | plan "#### Phase 6"; web/src/pages/section.rs `controls`; web/src/pages/review.rs `queue` | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 6.1 | dsp-repository | done | b67c0a91 | section-lifecycle.spec.ts: discard draft (keep / yes), take back (keep waiting / yes), locked while In review with take-back, take back from In review leaves RDU's pending table; 12 tests green across both passes | — |
| 6-review-1a | dsp-repository | done | db2b548f | review fix: test 5 asserts the In review lock message (a refused claim passed it as Submitted); group retries 0; header comment dropped | — |

## Phase 7

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 7.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/review-decisions.spec.ts | — | Phase 7: all five review-decisions.spec.ts items, `just lint-e2e`, `just test-e2e-editor` | one serial test per checkbox, green in chromium-js and chromium-nojs; spec lints clean | plan "#### Phase 7"; docs/src/editor/review.md "Ending a review round" | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 7.1 | dsp-repository | done | d02ffd46 | review-decisions.spec.ts: five serial tests (retries 0) on DECISIONS_SHORTCODES, both passes green; canary (reverted field expected as submitted) fails as it should; editor suite lints at the base's 8 Biome errors, none in the new file | — |

## Phase 8

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 8.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/support/review.ts, areas/deposit/editor/web-e2e-tests/tests/review-decisions.spec.ts | — | (preparatory refactor; no checkbox) | `decision()` exported from tests/support/review.ts, review-decisions.spec.ts imports it and passes in both passes | journal Side findings Phase 7 (DUNE-001) | — |
| 8.2 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/review-outcomes.spec.ts | 8.1 | Phase 8 checkboxes 1 to 6 (request-changes refusal, returned draft with notice, accepted field read-only, reject refusal, reject with note, vocabulary); checkboxes 7 and 8 (lint, test run) ticked at the phase gate | six serial tests on CHANGES_REQUESTED_SHORTCODES and REJECTED_SHORTCODES pass in chromium-js and chromium-nojs | plan § Phase 8; web/src/handlers/review.rs finish(), web/src/pages/review.rs confirm_note(), web/src/pages/section.rs round_wording(), web/src/form/widgets.rs Mode::Accepted | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 8.1 | dsp-repository | done | 3a3881e3 | `decision()` moved from review-decisions.spec.ts to tests/support/review.ts; review-decisions.spec.ts passes in both passes | — |
| 8.2 | dsp-repository | done | 14c9e78c | review-outcomes.spec.ts: six serial tests over CHANGES_REQUESTED and REJECTED projects, both passes green; mutation canaries (no Accept, a real reject note) make tests 3 and 4 fail | — |

## Phase 9

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 9.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/support/review.ts, areas/deposit/editor/web-e2e-tests/tests/review-decisions.spec.ts | — | — (preparatory: the take-over spec saves decisions too) | `saveDecisions` lives in tests/support/review.ts; review-decisions.spec.ts passes in both passes | tests/review-decisions.spec.ts § saveDecisions | — |
| 9.2 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/review-takeover.spec.ts | 9.1 | the three `tests/review-takeover.spec.ts` items; "Run `just lint-e2e`; it passes"; "Run `just test-e2e-editor`; the new tests pass in both passes" | three serial tests pass in chromium-js and chromium-nojs; phase files lint clean | plan § Phase 9; web/src/pages/review.rs § claim_banner, status | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 9.1 | dsp-repository | done | 6a0c4e43 | `saveDecisions` moved verbatim to tests/support/review.ts; review-decisions.spec.ts passes in both passes (11 with setup) | — |
| 9.2 | dsp-repository | done | 24d1f366 | tests/review-takeover.spec.ts: three serial tests (banner naming rdu, take-over moves the holder to reviewer-b, last save wins), retries 0, TAKEOVER_SHORTCODES; 7 passed with setup in both passes; `just lint-e2e` reports only the base's 8 errors; canaries: no take-over click fails test 2, no reviewer-b save fails test 3 | — |
| 9-review-1a | dsp-repository | done | 70b08606 | fixup! of 24d1f366: three comments in review-takeover.spec.ts trimmed to their constraint (simplicity W2 and S1); comment share 8 of 92 added lines | — |

## Phase 10

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 10.1 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/support/agents.ts | — | none (preparatory) | searchAppendBox split out of searchToAppend for a search that matches nothing; importers of searchToAppend/appendAgent green in both passes | support/agents.ts searchToAppend | — |
| 10.2 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/proposals-new.spec.ts | 10.1 | Phase 10 checkboxes 1-4 (proposals-new) | new person and organisation proposals start from a no-match search, save a name, list it, rows add and remove; both passes green | plan Phase 10; tests/proposals.spec.ts, support/agents.ts | — |
| 10.3 | dsp-repository | areas/deposit/editor/web-e2e-tests/tests/entity-decisions.spec.ts | 10.2 | Phase 10 checkboxes 5-8 (entity-decisions) | submitted proposal row with Accept/Reject; undecided-proposal and rejected-entity refusals; accepted person in approved-records entities; both passes green | plan Phase 10; tests/review-decisions.spec.ts, support/review.ts | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 10.1 | dsp-repository | done | 7c5e440c | searchAppendBox split out of searchToAppend (fill plus the no-JS Search press, no option wait); agent-picker, canton, proposals, row-controls green in both passes | — |
| 10.2 | dsp-repository | done | 9c9aa5ee | proposals-new.spec.ts: person (start, save name, panel, Job titles add/remove) and organisation (start, save name+URL, panel), serial, afterAll discards; both passes green with proposals.spec.ts after it | — |
| 10.3 | dsp-repository | done | 2a00f676 | entity-decisions.spec.ts: new person proposed and referenced as Contact point on ENTITY_DECISIONS, submitted; review offers Accept/Reject; undecided-proposal and rejected-entity (names Contact point) refusals; accepted person in approved-records entities as new; serial, retries 0; both passes green, canary on operation fails | — |
| 10-review-1a | dsp-repository | done | 27e5d357 | fixup: searchAppendBox's doc states it does not wait for results | — |
| 10-review-1b | dsp-repository | done | 79f48935 | fixup: afterAll discards every live proposal from the panel; retrying URL assertion before reading it; Save draft waits for the entity POST, not telemetry; exact row-control names; PICKER_SHORTCODE doc names the coupling; canary (org test forced to fail) leaves proposals.spec.ts green | — |
| 10-review-1c | dsp-repository | done | 7431c804 | fixup: URL asserted before reading personId; entity POST wait; named missing-record failure; comments corrected | — |
| 10-review-2a | dsp-repository | done | 427a76ea | fixup: the search for the proposed person's name uses searchToAppend (it expects a match), matching searchAppendBox's doc (dune re-review suggestion) | — |
| 10-gate | dsp-repository | done | — | phase gate: proposals-new, entity-decisions and proposals.spec.ts with setup, both passes, 19 passed; `just lint-e2e` reports only the base's 8 errors, new files clean | — |

## Phase 11

### Chunk queue
| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|
| 11.1 | dsp-repository | web-e2e-tests/tests/accessibility.spec.ts, web-e2e-tests/playwright.config.ts | — | P11 #1 (axe over the listed pages), #2 (scans that pass) | each listed page scanned once with the sweep's tags; only passing scans committed; failing pages' violations captured for the baseline file | plan Phase 11; tests/support/review.ts, depositors.ts | — |
| 11.2 | dsp-repository | none in repo: workspace-root scratch script + _a11y-baseline.md | 11.1 | P11 #3 (contrast), #4 (baseline file) | ratios for text, badges, control borders, focus, hover, disabled on every in-scope page; baseline file at workspace root | plan Technical Considerations (WCAG exemptions) | — |
| 11.3 | dsp-repository | fixes only if the gate surfaces this branch's own interference | 11.1 | P11 #5 (just test-a11y-editor), #6 (whole suite both passes) | both recipes green; duration vs Phase 1 baseline (128 s, 136/23) in Side findings | brief test_command | — |

### Chunks
| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 11.1 | dsp-repository | done | 0354c009 | axe over all 11 listed page states: every one passes, so every one is a committed scan (no unscanned-page violations for the baseline). Stateful pages walk 080C (new DEPOSITOR_SHORTCODES[29], A11Y_REVIEW_SHORTCODE; unused, in the data set, name clean) in a serial retries-0 group: proposal (completed, else submit is refused) → discard confirmation → submit → take-back confirmation → rdu claims, reviewer-b scans banner + entity rows on one page. Remove page on E2E7 (A11Y_CREATED_SHORTCODE). /approved/{id}/discard reuses A11Y_STATES' approved record as a third test of its serial group. Canary: injected img without alt failed the review scan with image-alt. | — |
| 11.2 | dsp-repository | done | — (outside the repo) | Contrast measured by /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-ux-ui/_contrast/contrast.spec.ts (run via contrast.config.ts, which runs setup + accessibility.spec.ts for state first; raw output contrast-results.json, 5058 measurements over 30 page states). 0 failures (text, placeholders, input borders, focus after focus(), hover after hover()); no disabled control rendered on any page. Baseline for the PR body written to /Users/mpro/repos/dasch_swiss/_workspaces/metadata-editor-ux-ui/_a11y-baseline.md (checkbox 4's deliverable; the session pastes it at ship). | — |
| 11.3 | dsp-repository | done | — (no fix needed) | `just test-a11y-editor`: 29 passed. Whole suite (`npx playwright test`, the recipe's command, list reporter), both passes, first run: 238 passed / 23 skipped / 0 failed, 3.0 min (183 s wall), local retries 0 so no flake was masked; no cross-spec interference surfaced. `just lint-e2e`: 8 errors, all in files the base revision already flags, none in this phase's files (the provisional reading). | — |
| 11-review-1a | dsp-repository | done | 98fdb220 | Review round 1 (consistency, simplicity, dune, typescript, accessibility, security): heading/value premise before the four URL-only scans and the proposal scan (an error page at the same URL would pass axe); wait for the create to land before leaving /projects/new; config docs: A11Y_STATES_SHORTCODE also carries the approved discard page, why the two new codes are single not per pass, and the serial sequence needs a fresh server. Refuted/declined: unreferenced proposal hides entity rows (the heading assertion passes); clean up E2E7 by removing it (would duplicate projects.spec's removal test; E2E7 collides with nothing). | — |
