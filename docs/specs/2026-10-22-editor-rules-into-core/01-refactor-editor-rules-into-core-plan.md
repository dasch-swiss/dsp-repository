---
title: "refactor: Move the editor's form and business rules into editor-core, dedup storage_error and share the XFF key extractor"
type: refactor
date: 2026-10-22
author: "Marcin Procyk"
status: implemented
repository: dasch-swiss/dsp-repository
linear: DEV-7526
linear_project: Metadata Editor for RDU + depositing projects
---

# refactor: Move the editor's form and business rules into editor-core, dedup storage_error and share the XFF key extractor

## Overview

DEV-7375 brought the Deposit Area to ADR-0004's shape and deferred four cleanups (`docs/specs/2026-10-13-deposit-area-target-architecture/`, "Deviations from the ticket's proposals" and H3). This plan does all four: the form rules move into `editor-core`, the submit and approve gate sequences leave the handlers for `editor-core`, `editor-web`'s five `storage_error` copies become one, and `RightmostXffKeyExtractor` gets one home in a new `shared-rate-limit` crate. Behaviour does not change: same routes, bytes, refusal texts, span outcomes and log messages.

None of this reverses a prior decision: DEV-7375 deferred all four items to this issue, and ADR-0004's crate set and dependency edges are unchanged.

**Delivery: one PR, stacked on #529** (base `feature/dev-7596-mosaic-a-tables-screen-reader-only-header-widens-the-page-at`, itself on #527). Seven commits, `allow-many-commits` ticked: this plan first, then one commit per phase below, in order. Reviewers can be pointed at single commits; each phase names its reviewer set.

## Problem Statement / Motivation

- `editor-web/src/form/` holds the field registry, the obligations and the submit validation. None of it renders (no maud), yet it sits in the web crate, so `editor-core` cannot name a field's audience, obligation or shape, and every rule that needs them has to live in `editor-web`.
- The submit gate sequence (`handlers/sections.rs`, `submit`, 1018-1232) and the approve gate sequence (`handlers/review.rs`, `finish`, 359-550, plus `decided_draft` and `decisions_from`) are inline in axum handlers. The order of the gates is itself a rule (the obligation gate "must stay ahead" of `to_raw`, sections.rs 1049-1054) and nothing outside the handler states it.
- Five near-identical `storage_error` functions in `editor-web/src/handlers/` differ only in a log phrase and in "page" versus "form".
- Three byte-identical `RightmostXffKeyExtractor` copies (DPE, editor-web, identity-web), each with the same seven tests. Two areas use it, which is `shared/README.md`'s criterion. No `tower_governor` 0.6 extractor replaces it: `SmartIpKeyExtractor` takes the leftmost, client-forgeable entry.

## Proposed Solution

### Form rules into `editor-core` (Phase 3)

`editor-web/src/form/{registry,obligation,submit}.rs` move to `editor-core/src/form/` as `editor_core::form::{registry, obligation, submit}`. `core/src/form.rs` stays a file and gains the three `pub mod` lines, so the 2,084-line decoder is not renamed and `editor_core::form::{FormBody, Shape, apply, …}` keep their paths. The labels and hints move with the registry: a `Field` is one declaration of id, label, hint, obligation, audience and shape, and splitting copy from rule would give one field two definitions. None of the three files uses maud, axum, `crate::pages` or mosaic, so the crate-graph gate allows them in core, and nothing in them points back at `editor-web`.

`is_language_map` (`pub(crate)` in `widgets.rs`) and `as_multilingual` (already `pub`) move to `editor_core::multilingual` as `pub`, because moved tests and the round-trip test call them. `widgets.rs`, `form::INTENT` and `widgets::tests::every_shaped_field_has_a_control` stay in `editor-web`; that test is what ties a core `Shape` to the names a rendered control posts, so it is the drift guard between registry and widgets once they sit in different crates. `web/tests/untouched_form_round_trip.rs` moves to `core/tests/`: it hand-writes the posted pairs, so it tests the decoder against the registry, and its header's reason for living in `editor-web` is rewritten. `web/tests/depositor_vocabulary.rs` stays (it renders pages) and only its import changes.

**Module boundary**, for reviewers to check against: `editor_core::form::submit` holds per-field rules that need the registry (`typed_sentinels`, `over_cap`, `unresolved_agents`); `editor_core::submission` holds the submit decision, the ordered gate sequence, beside the existing `unresolved_temporal_coverage`.

### Business rules into `editor-core` (Phases 4 to 6)

Pure functions that return a typed refusal. Neither gate sequence awaits between gates (the first await is the write: `sections.rs:1198`, `review.rs:507`), so one synchronous function per sequence preserves behaviour as long as it keeps the order. Each handler keeps the I/O around the check: load the context, call the check, do the existing repository write, and map the refusal to its message, per-field errors, span outcome and log fields through one `match` in `sections.rs` or `review.rs` that ends in today's `refused_with` / `refused` sink. That sink is what keeps the plain and the Datastar-enhanced paths identical, so no refusal path bypasses it.

**No refusal wording, no HTTP and no tracing in core.** Core returns data; every refusal text, per-field message and formatted finding stays in `editor-web`. The refusal enums get no `Display` or `thiserror`, and no `#[non_exhaustive]`: the handler's `match` is exhaustive, so a new gate is a compile error at the one place that renders it. Logging stays in the handler modules, so event targets (`editor_web::handlers::sections`, `…::review`) do not change.

```rust
// editor_core::submission
pub enum SubmitRefusal {
    Unanswered(Vec<&'static Field>),                // A
    Incomplete(DraftError),                         // B: to_raw's error, logged as `error = %error`
    UnknownAgent(Vec<(&'static Field, String)>),    // C
    ProposalIncomplete(Vec<ProposalProblem>),       // D: one per finding, in proposal then finding order
    Sentinel(Vec<&'static Field>),                  // E
    Invalid(Vec<UnresolvedCoverage>),               // F
    Unchanged,                                      // G
}
// ProposalProblem: entity id, kind, and Unreadable | Finding(EntityFinding)
pub fn check(
    audience: Audience,
    draft: &ProjectDraft,                     // the in-memory draft after apply_posted
    agents: &Agents,                          // state.agents; check builds AgentScope::with_proposals itself
    proposals: &[EntityProposal],             // all statuses, as sections::context loads them
    periods: &HashMap<String, W3cdtfRange>,
    enrichment: &HashMap<String, EnrichedDate>,
    published: Option<&ProjectDraft>,         // state.published.get(shortcode as typed).map(from_raw)
) -> Result<(), SubmitRefusal>;

// editor_core::review
pub enum ApproveRefusal {
    Undecided { count: usize },                     // 2
    UndecidedProposal { count: usize },             // 3
    RejectedEntity(Vec<(&'static Field, String)>),  // 4: the handler dedups the labels, as review.rs 451-456 does
    Unpublishable(DraftError),                      // 5
}
/// Call only for an approving outcome.
pub fn approval(
    submitted: &ProjectDraft,
    rows: &[FieldDiff],
    state: &ReviewState,                      // as rebuilt from the body before finish
    proposals: &[EntityProposal],             // context.entity_proposals: Submitted only, decisions applied in memory
    agents: &Agents,                          // state.agents; the scope keeps decision != Some(Reject) only
) -> Result<(), ApproveRefusal>;
pub fn decided_draft(submitted: &ProjectDraft, rows: &[FieldDiff], state: &ReviewState) -> ProjectDraft;
pub fn decisions(body: &FormBody, rows: &[FieldDiff], submitted: &ProjectDraft, accept_all: bool, published: bool) -> ReviewState;
```

Signatures are a contract on inputs and order, not on spelling: execution may rename, but each input must be the one named in its comment, since a near-equivalent changes behaviour (the page's own `context.agents` also resolves proposals being rejected, and would disable gate 4; `docs/src/editor/entity-proposals.md`, "The second gate has a trap"). `accept_all` stays an argument: core never sees an intent string. `#[must_use]` goes on `decided_draft`, `decisions` and the predicates, not on the `Result`-returning checks.

**Gate numbering**, used below. Submit: A required fields answered, B `to_raw`, C agents resolve, D own live proposals complete (incl. an unreadable payload), E no typed sentinel, F temporal coverage resolves, G something changed. Approve: 1 a non-approve outcome needs a note (`review.rs:384`), 2 no undecided change (403), 3 no undecided proposal (418), 4 no reference to a rejected or unknown entity (450), 5 the decided draft is a project (470). Gate 1 becomes a predicate on `ReviewOutcome`; gates 2 to 5 are `approval`.

`DECISION_PREFIX` moves to `editor_core::review` because `decisions` reads the posted names it builds; the review page imports it. `ENTITY_DECISION_PREFIX` stays in the page: its reader, `apply_entity_decisions`, writes through the repository. `INTENT`, `NOTE`, `CONFIRMED` and `BASELINE` stay in `editor-web`: core never reads them.

**One depositor-side predicate** (Phase 5): "everything short of Approved", today spelled at `sections.rs:280` (`pending_submission`), `review.rs:109` (the review queue) and `store/src/open_work.rs:59` (the open work listed before an account's removal), becomes `SubmissionState::is_pending` in `editor_core::records`. `context()`'s lock (`sections.rs` 359-363) keeps its exhaustive `match` to `page::Locked`, so a new state cannot compile into "unlocked". And `entities.rs`'s `proposal_for` (the live proposal, else the most recently touched) moves to `editor_core::proposals`.

**What stays in the handlers, and why.**

- State derivation (`handlers/projects.rs`) and the Online decision (`reconcile.rs`) are already in core: `editor_core::status::depositor_state` and `classify_record`. What remains in `projects.rs` (`project_state`, `approved_comparison`) and `reconcile.rs` is repository I/O around those calls. The retirement skip in `reconcile.rs` (177) only saves a write: `retire` enforces the `New`-only rule itself.
- Single calls into core: the audience checks (`sections_for`, `fields_for`, core after Phase 3), `over_cap`, `is_live`, `permits_online`.
- `changed_underneath` (`sections.rs` 727-732): a string comparison between the page's own baseline value and `to_rfc3339()`, which the page writes in four places; moving the comparison without a shared revision helper only adds a way to mix the two up.
- The order of `act()`'s save gates (lock, then changed-underneath, then over-cap) is handler-owned, as today.
- Record construction for a write (`approved_record`, `returned_draft`, the withdrawal round, the new `Submission`), intent parsing, and page markers (`done_notice`, `ProposalStatus` to `page::Over`).
- The save-path accepted-field skip in `apply_posted` (`sections.rs:750`), `propose()`'s refusals, and the review page's read-model filters (`Submitted`-only proposals, cross-project live proposals, `held_by_viewer`): neither named in DEV-7526 nor part of a gate sequence.

### `storage_error` dedup (Phase 1)

One `storage_error` in `editor-web/src/handlers/mod.rs`, called from all five modules (17 call sites). What differs per surface (the phrase in the log message, "page" or "form" in the problem text) is one `const` value per module passed as an argument. Log message and page bytes stay identical per surface. The event's target changes from the five `editor_web::handlers::<module>` paths to `editor_web::handlers`; nothing filters on those targets (no `RUST_LOG` directive or test names them), and the commit message says so. `identity-web`'s copy stays: another crate, with another state and user type, and web crates may not depend on each other.

`refused`, `region` and `render_page` are **not** deduplicated. They share names, not bodies: each builds its own module's `Context` and view and calls its own page module, so a shared version would be a trait or generic over three different shapes for three callers. The fixtures `urlencode` (deposit-server, editor-web, identity-web) and `capture_logs` (editor-web, identity-web) already exist once per crate, the issue's own rule, so nothing changes for them.

### `shared-rate-limit` (Phase 2)

New crate `shared/rate-limit` (`shared-rate-limit`), manifest shaped like `shared/telemetry/Cargo.toml` (`version.workspace`, `edition = "2021"`, `publish = false`), holding `RightmostXffKeyExtractor` with its SECURITY doc paragraph and its seven tests. Its imports (`std::net`, `axum::extract::ConnectInfo`, `axum::http::Request`, `tower_governor::{key_extractor::KeyExtractor, GovernorError}`) make `axum.workspace` and `tower_governor.workspace` its only dependencies. It must not enable `tower_governor`'s `tracing` feature: features unify across the workspace, and that one makes `KeyExtractor::name()` mandatory for every implementation.

Every consumer depends on it directly and the three copies and their tests go: `dpe-server` (`router.rs`, and the `lib.rs` re-export), `access-server` (`serve.rs`, today through `dpe_server::`), `editor-web` (`routes.rs`, and the `lib.rs` re-export), `identity-web` (`routes.rs`) and `deposit-server` (`router.rs`, today through `editor_web::`). No re-export shim is left behind. Each `routes.rs` gets a `use shared_rate_limit::RightmostXffKeyExtractor;`, which the `per_ip_limit!` macro names unqualified. `check-capability-deps.sh` allows any role to depend on `shared/*`, and `check-composition-root-deps.sh` never forbids a `shared-*` crate, so neither gate changes.

`per_ip_limit!` itself stays where it is: the issue does not name it, DPE does not use it, and moving it is a separate decision.

## Technical Considerations

- **Moves are renames.** `git mv` each moved file so history follows across the crate boundary, then edit. Every call site is updated in the moving commit, with no re-export shim (`docs/specs/2026-09-15-fair-metadata-exposure/01-feat-fair-metadata-exposure-plan-journal.md`).
- **Paths after a move.** Inside the moved modules, `editor_core::X` becomes `crate::X`; `crate::form::registry` in their tests keeps working, since core also has `crate::form`. The moved integration test keeps `editor_core::` paths. In `editor-web`, `crate::form::registry` and `super::registry` become `editor_core::form::registry`.
- **Dependencies land in the commit that uses them**: `cargo-machete` runs in `just check` and fails an unused one, including an `editor-web` dev-dependency left unused by a moved test. All five extractor consumers keep `tower_governor` for their governor layers; imports only the extractor used are pruned.
- **`check-shared-paths`** (`just check`) scans `shared/*/src/*.rs` for paths into a service module; the new crate's docs and tests name none.
- **Registration points for `shared/rate-limit`**: the root `Cargo.toml` `members` (an explicit list), `Cargo.lock`, `shared/README.md` (tree and a section), `docs/src/repo_structure.md` (tree and crate table), `docs/src/git-conventions.md` and `CONVENTIONS.md` (commit-scope lists), `ARCH-MAP.md` (a component section), and `eng.yaml` (a `shared/rate-limit/**` reviewer entry, as `shared/telemetry/**` has). CI path filters, `bacon.toml`, dependabot and release-please already cover `shared/**` or the whole workspace, and no Dockerfile copies crate sources (the images are built from a prebuilt binary).
- **Gate order is a contract.** `check` returns the first refusal in order A to G and `approval` in order 2 to 5. A different order changes which message a depositor sees for a draft failing two gates. Gate B and gate D's unreadable-payload branch have no test today; gate B is reachable because `has_value` counts a number or bool as an answer (`obligation.rs:96`).
- **Delete-and-run**, the per-gate coverage check in Phases 4 and 6: for each gate, comment it out in `check` or `approval`, run `cargo test -p editor-core -p editor-web`, require at least one failure, restore it with `git checkout HEAD -- <file>`; record one line per gate in the phase review.
- **Byte identity is checked by diff, not by reading.** Each behaviour-preserving phase compares the string literals in `span.record(…)`, `tracing::*!(…)` and the refusal constants before and after (`git diff -M <phase base> -- <files> | grep -E '^[-+].*(span\.record|tracing::|const [A-Z_]+: &str)'` shows only moves), and the `assert` lines of every test that did not move (`git diff -M <phase base> | grep -E '^[-+]\s*assert'` shows only path changes).
- **Rustdoc.** CI builds docs without `-D warnings`, so a broken intra-doc link after a move only warns: `cargo doc --no-deps -p editor-core -p editor-web` shows no new warnings.
- **Commands run in the dev shell.** Under Nix, every `just` and `cargo` command below runs as `nix develop --command …` (direnv does not fire in an agent's shell; root `CLAUDE.md`, Setup).
- **Browser verification.** `eng.yaml` offers `eng:test-browser` (`offer: ask`) for `areas/*/*/web/**`. Not run: no markup changes. `just test-e2e-editor` runs both Playwright projects (`chromium-js`, `chromium-nojs`) over every page, in each phase that moves form or handler logic.

## Implementation Phases

Reviewers: every phase gets the three `eng:working` always runs (`eng:review:consistency-reviewer`, `eng:review:code-simplicity-reviewer`, `eng:review:dune-reviewer`) plus `eng:review:rust-reviewer`, and whatever `eng.yaml` adds for the paths touched. Phase-specific additions are listed under each phase. "The test count" below is `cargo test --tests -- --list 2>/dev/null | grep -c ': test$'`; its baseline is measured at the branch point before Phase 1.

#### Phase 1: One storage_error per editor-web

Adds nothing beyond the base set.

- [x] `editor-web/src/handlers/mod.rs` holds the one `storage_error`, taking each surface's `const` (log phrase, page-or-form noun); the copies in `approved.rs`, `entities.rs`, `projects.rs`, `review.rs` and `sections.rs` are deleted and their 17 call sites pass their module's const
- [x] Each surface's log message and problem-page text are byte-identical to before, checked by the literal diff in Technical Considerations
- [x] The test count equals the baseline
- [x] Run `just check && just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: One RightmostXffKeyExtractor in shared-rate-limit

Adds `eng:review:security-reviewer` (client-IP keying for rate limits).

- [x] `shared/rate-limit` (`shared-rate-limit`) exists with `RightmostXffKeyExtractor` and the seven `key_extractor` tests, registered in the root `Cargo.toml` `members`
- [x] `dpe-server` uses it; its copy, its tests and the `lib.rs` re-export are gone
- [x] `access-server`'s `/telemetry/collect` governor uses it directly
- [x] `editor-web` uses it; its copy, its tests and the `lib.rs` re-export are gone
- [x] `identity-web` uses it; its copy and its tests are gone
- [x] `deposit-server`'s beacon governor uses it directly
- [x] The "deliberate copy" doc paragraphs in editor-web and identity-web `routes.rs` are gone, and `git grep -n -E 'struct RightmostXffKeyExtractor' -- '*.rs'` finds one definition
- [x] `shared/README.md`, `docs/src/repo_structure.md`, `docs/src/git-conventions.md` and `CONVENTIONS.md` name the new crate
- [x] `ARCH-MAP.md` has a `### shared/rate-limit` component section; the `dpe-server` surface (60-61), the `editor_web` library (221-224) and the `deposit-server` entry (439-444) no longer list the extractor; the "Depends on" of `access-server`, `dpe-server`, `editor-web`, `identity-web` and `deposit-server` name `shared/rate-limit`
- [x] `areas/access/server/CLAUDE.md` and `areas/deposit/server/CLAUDE.md` "Dependency boundary" sections name `shared-rate-limit`, and the former no longer lists the extractor in `dpe-server`'s surface (62-64)
- [x] `eng.yaml` binds `eng:review:security-reviewer` to `shared/rate-limit/**`
- [x] The test count is the baseline minus 14 (three copies of seven tests become one)
- [x] Run `just check && just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: The form rules live in editor-core

Adds `eng:review:pattern-recognition-specialist` (module boundaries after the move).

- [x] `registry.rs` is `editor_core::form::registry`, moved with its tests
- [x] `obligation.rs` is `editor_core::form::obligation`, moved with its tests
- [x] `submit.rs` is `editor_core::form::submit`, moved with its tests
- [x] `is_language_map` and `as_multilingual` are `pub` in `editor_core::multilingual`; `widgets.rs`, the `sections.rs` tests (5028, 5093) and the moved tests call them there
- [x] `untouched_form_round_trip.rs` is in `core/tests/`, its header (6-8) saying why it lives there
- [x] Every path to `registry`, `obligation` or `submit` in `editor-web` (`super::registry` in `widgets.rs`, `crate::form::…` in pages, entity, handlers and tests, `editor_web::form::registry` in `depositor_vocabulary.rs`, and doc links) points at `editor_core::form::…`; `web/src/form/mod.rs` declares only `widgets` and `INTENT`, and its module doc says so; `every_shaped_field_has_a_control` stays in `widgets.rs`
- [x] The core docs that place the registry in `editor-web` are rewritten: `core/src/form.rs` (12-16, which names `every_shaped_field_has_a_control` as the drift guard; 31; 300), `core/src/review.rs` (6), the moved `submit.rs` module doc (its `server -> web -> core` reason), and `registry.rs` (756)
- [x] `areas/deposit/editor/CLAUDE.md` (Views, 31), `ARCH-MAP.md` (193-195, 207, 230, 232, 260-261, 600), `docs/src/editor/architecture.md` (`editor-core` and `editor-web` rows), `docs/src/editor/project-form.md` (123, 150), `docs/src/editor/entity-proposals.md` (46, 57) and `areas/deposit/editor/CONTEXT.md` place the registry, obligations and submit validation in `editor-core`
- [x] `cargo doc --no-deps -p editor-core -p editor-web` shows no new warnings
- [x] The test count equals Phase 2's
- [x] Run `just check && just test`; it passes
- [x] Run `just test-e2e-editor`; both Playwright projects pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: The submit gates live in editor-core

Adds `eng:review:security-reviewer` and `eng:review:ivan-reviewer` (server authority).

- [x] `editor_core::submission::check` runs gates A to G in order with the inputs Proposed Solution names, and `sections.rs` `submit` calls it, then `SubmissionRepository::create`, mapping each refusal through one `match` to today's message constant, per-field errors, `form.outcome` value and log fields, ending in `refused_with` / `refused_with_proposal_findings`
- [x] Gate D's payload parse moves into `check`, which returns one `ProposalProblem` per finding in proposal then finding order; `sections.rs` formats `"its stored data could not be read"` and `"{field}[{index}]: {message}"` as today
- [x] Gate B has a core unit test on a draft whose required member has the wrong kind
- [x] Gate D's unreadable-payload branch has a core unit test
- [x] Delete-and-run over gates A, C, D (its finding branch), E, F and G finds at least one failing test each; a gate none catches gets a core unit test
- [x] Refusal texts, span outcomes, log messages and unmoved test assertions are byte-identical, checked by the diffs in Technical Considerations
- [x] `docs/src/editor/project-form.md` (the submit gates, 152-177), `ARCH-MAP.md` and `areas/deposit/editor/CONTEXT.md` say the submit gates live in `editor_core::submission`
- [x] Run `just check && just test`; it passes
- [x] Run `just test-e2e-editor`; both Playwright projects pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: The depositor-side predicates live in editor-core

Adds `eng:review:security-reviewer`.

- [x] `SubmissionState::is_pending` in `editor_core::records` is "everything short of Approved"; `pending_submission` (`sections.rs:280`), the review queue filter (`review.rs:109`) and `store/src/open_work.rs:59` call it, and `context()`'s lock keeps its exhaustive `match`
- [x] `proposal_for` is in `editor_core::proposals`, and `entities.rs` calls it
- [x] `proposal_for`'s tie-break (a live proposal beats a newer one, else the most recently updated wins) has a core unit test
- [x] Refusal texts and unmoved test assertions are byte-identical, checked by the diffs in Technical Considerations
- [x] Run `just check && just test`; it passes
- [x] Run `just test-e2e-editor`; both Playwright projects pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: The approve gates live in editor-core

Adds `eng:review:security-reviewer` and `eng:review:ivan-reviewer`.

- [x] `decided_draft` is `editor_core::review::decided_draft`; `finish` and `approved_record` call it
- [x] `decisions_from` and `substitute` are `editor_core::review::decisions`
- [x] `DECISION_PREFIX` is in `editor_core::review`; `pages/review.rs` (48, its doc link at 51, 1071), `handlers/review.rs` and its tests (1206, 1593, 1807) use it there
- [x] `editor_core::review::approval` runs gates 2 to 5 in order with the inputs Proposed Solution names, and `finish` calls it, then the existing `ReviewRoundRepository` transition, mapping each refusal through one `match` to today's messages and log fields (the label dedup for gate 4 included), ending in `refused`
- [x] Gate 1 is a predicate on `ReviewOutcome`, and `finish` calls it
- [x] Delete-and-run over gates 1 to 5 finds at least one failing test each; a gate none catches gets a core unit test
- [x] Refusal texts, span outcomes, log messages and unmoved test assertions are byte-identical, checked by the diffs in Technical Considerations
- [x] `docs/src/editor/review.md`, `docs/src/editor/entity-proposals.md` (57), `ARCH-MAP.md`, `docs/src/editor/architecture.md` and `areas/deposit/editor/CONTEXT.md` say the approve gates live in `editor_core::review`
- [x] `/dune:dune-map update` refreshes the `areas/deposit/editor` fingerprint (deferred from the Phase 3 stop: Phases 4 to 6 change the component again)
- [x] Run `just check && just test`; it passes
- [x] Run `just test-e2e-editor`; both Playwright projects pass
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge #527, then #529, then this PR; after each parent merges, rebase this branch with `git rebase --onto origin/main <old-parent-tip>` (rebase-merge keeps the old merge base) | Marcin | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] Every gate of the submit sequence (A to G) and the approve sequence (1 to 5), and the Phase 5 predicates, is a call into `editor-core`; what stays in the handlers is the list under "What stays in the handlers"
- [x] `editor-core` holds the registry, the obligations and the submit validation
- [x] No test assertion changes except where a test moves crates; refusal texts, span outcomes and log messages are byte-identical
- [x] `git grep -n 'fn storage_error(' -- areas/deposit` finds one definition in `editor-web` and one in `identity-web`
- [x] One `RightmostXffKeyExtractor`, in `shared-rate-limit`, with its tests
- [x] `just check`, `just test` and `just test-e2e-editor` pass; `check-capability-deps.sh` and `check-composition-root-deps.sh` pass unchanged
- [x] ARCH-MAP, `docs/src/editor/architecture.md` and the `CONTEXT.md` files describe the result

## Dependencies & Risks

- **Stack.** Depends on #527 (renamed `handlers/collection.rs` to `approved.rs` and the vocabulary) and #529. A change to either before they merge means a rebase here.
- **Phase order.** Phase 6's `decisions` runs the form appliers through `registry::field`, and Phases 4 and 6 call `form::submit`, so both need Phase 3 first.
- **Gate order or inputs drifting during extraction** is the behaviour change a green suite might not catch: no test drives a refusal on the Datastar-enhanced path, and the E2E suite covers no refusal. The guards are the named inputs, the single refusal sink, and the delete-and-run check in Phases 4 and 6.
- **Size of Phase 3's diff** is mostly renames; reviewers read it with `git show -M`.

## Success Metrics

- The test count (defined under Implementation Phases): baseline after Phase 1, baseline minus 14 after Phase 2, unchanged by Phase 3, and higher after Phases 4 to 6 only by the core unit tests those phases add.
- The delete-and-run results for Phases 4 and 6, one line per gate, in each phase's review.

## References

- Issue: DEV-7526; parent plan `docs/specs/2026-10-13-deposit-area-target-architecture/01-refactor-deposit-area-target-architecture-plan.md` (Deviations, H3)
- Submit gates: `areas/deposit/editor/web/src/handlers/sections.rs:1018-1232`; approve gates: `handlers/review.rs:359-550`, `decided_draft` 576-608, `decisions_from` 845-877, `substitute` 918-928
- Already-core rules: `areas/deposit/editor/core/src/status.rs` (`depositor_state`, `classify_record`)
- Pure-rule precedent: `areas/deposit/identity/core/src/login.rs` (`locked_out`), called from `identity/web/src/handlers/login.rs`
- Gates: `.github/scripts/check-capability-deps.sh` (`shared/*` allowed for every role), `.github/scripts/check-composition-root-deps.sh`, `.github/scripts/check-shared-paths.sh`
- `tower_governor` 0.6.0 source: `key_extractor.rs` (`KeyExtractor`, `SmartIpKeyExtractor`)
- Learnings: `docs/specs/2026-09-15-fair-metadata-exposure/01-feat-fair-metadata-exposure-plan-journal.md` (no shim, `cargo-machete`, same-commit gate updates); `docs/specs/2026-10-11-move-cpe-store-web-server/01-feat-move-cpe-store-web-server-plan-journal.md` (stacked PRs, autosquash checks)
