---
plan: docs/specs/2026-11-21-harden-cpe-engine/01-refactor-harden-cpe-engine-plan.md
target_repo: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/cpe-proj
base_commit: db8318b1
branch: refactor/dev-7633-sql-folder-loader-test-only
started: 2026-11-21
problem: >
  CPE serves Incunabula in production. Before a second project follows (HdM, then Solec), the Linear
  project "Harden the CPE engine" (P-DEV-411) asks that a project can be merged without going live on prod,
  that a strict validation runs in CI, that the engine has a browser test suite, and that the code is
  cleaned up; HdM also needs the CPE changes that so far exist only in dsp-incubator's vendored copy.
status: complete
autonomy: decide
---

# Execution Journal: 01-refactor-harden-cpe-engine-plan

## Repos

| repo | base_commit | branch | merge_strategy | status | pr |
|------|-------------|--------|----------------|--------|----|
| dsp-repository | db8318b1 | one branch per phase (gh stack #631) | rebase | shipped | #629 (plan), #630, #632 to #646, drafts |

## Phases

| phase | status | phase_base | review_fix_rounds |
|-------|--------|------------|-------------------|
| 1 | reviewed | dsp-repository@db8318b1 | 1 |
| 2 | reviewed | dsp-repository@32948a80 | 1 |
| 3 | reviewed | dsp-repository@2decb48f | 1 |
| 4 | reviewed | dsp-repository@4c2a26e7 | 2 |
| 5 | reviewed | dsp-repository@b468a02a | 1 |
| 6 | reviewed | dsp-repository@9b280465 | 2 |
| 7 | reviewed | dsp-repository@d3b989ba | 1 |
| 8 | reviewed | dsp-repository@70cbfdd9 | 1 |
| 9 | reviewed | dsp-repository@74e66065 | 2 |
| 10 | reviewed | dsp-repository@e871e970 | 1 |
| 11 | reviewed | dsp-repository@2d64819f | 0 |
| 12 | reviewed | dsp-repository@e234699d | 1 |
| 13 | reviewed | dsp-repository@b2f691d0 | 2 |
| 14 | reviewed | dsp-repository@7a2728f0 | 1 |
| 15 | reviewed | dsp-repository@3a2cd1d0 | 1 |
| 16 | reviewed | dsp-repository@09c16350 | 2 |

## Archived phases

- Phase 1: 3 chunks, ids 1.1 to 1.3, highest number 1.3, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 2: 4 chunks, ids 2.1 to 2.4, highest number 2.4, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 3: 8 chunks, ids 3.1 to 3.8, highest number 3.8, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 4: 7 chunks, ids 4.1 to 4.R2, highest number 4.5, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 5: 2 chunks, ids 5.1 to 5.R1, highest number 5.1, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 6: 9 chunks, ids 6.1 to 6.G, highest number 6.5, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 7: 5 chunks, ids 7.1 to 7.R1f, highest number 7.2, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 8: 2 chunks, ids 8.1 to 8.R1, highest number 8.1, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 9: 3 chunks, ids 9.1 to 9.R2, highest number 9.1, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 10: 4 chunks, ids 10.1 to 10.R1, highest number 10.3, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 11: 2 chunks, ids 11.1 to 11.G, highest number 11.1, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 12: 4 chunks, ids 12.1 to 12.R1b, highest number 12.2, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 13: 7 chunks, ids 13.1 to 13.R2, highest number 13.5, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 14: 4 chunks, ids 14.1 to 14.R1, highest number 14.2, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 15: 3 chunks, ids 15.1 to 15.R1, highest number 15.1, in 01-refactor-harden-cpe-engine-plan-journal-archive.md
- Phase 16: 6 chunks, ids 16.1 to 16.R2, highest number 16.4, in 01-refactor-harden-cpe-engine-plan-journal-archive.md

## Chunk queue

| id | repo | files | depends_on | checkboxes | acceptance | context | replaces |
|----|------|-------|------------|------------|------------|---------|----------|

## Chunks

| id | repo | status | commit(s) | summary | blocker |
|----|------|--------|-----------|---------|---------|
| 4.F1 | dsp-repository | done | 67733228 | Final-review fix (DUNE-002): the data-free check looks a narrative name up only when it passes `cpe_ports::is_file_name` and has no `/`, as activation does; a refused name fails the check naming the file; `is_bare` removed; test with `my page.md` and `.x.md`; operations.md sentence. Fixup for b468a02a. | |

## Deferrals

- Verify at the final review (deferred at intake): every `eng:test-browser` checkbox, and the end-to-end suites as phase-review verification.
- Phase 1: review Warning (dune-reviewer DUNE-001): the `cpe-server` ARCH-MAP fingerprint no longer matches its paths after this phase; no CI gate checks it, and `/dune:dune-map` owns the line, so it is left for the session or owner to refresh with `/dune:dune-map update`.
- Phase 2: scope (Warning), security-reviewer: a released project whose own `sync` directory is missing from a partial or stale mount while another project's is there is classed unknown (`location_present` is true) and answers a cacheable 404 instead of 503; re-activation accepts the change (`held_back_by` treats Unknown as removed) with only an info line. The classification predates this phase; this phase changes its answer from 503 to 404. Options for the owner: a warning when a project served before is unknown now (the owner decided unknown logs at info, so this needs the owner), holding such a change back until a second poll confirms it (adds state), or `Cache-Control: no-store` on every CPE 404.
- Phase 4: scope (Warning), rust-reviewer: the data-free check runs no site-block check, so shipped inputs with `default-language "xx"` or a
  `languages` list without `de`/`en` print `inputs: checked`, and activation later refuses them (`UnknownDefaultLanguage`,
  `MissingRequiredLanguage`). cpe-core's site lowering is private and needs the project's assets (cover image), so the fix is a new public
  cpe-core entry beyond the checks the plan lists; `docs/src/cpe/operations.md` now says the rest of the site block is checked by the full load only.
- Phase 13: after the fix-round cap, dune-reviewer DUNE-002/003 (Warning): the classes and attributes `cpe.js` reads are listed in its header but no test ties them to the Rust markup, and the "read by cpe.js" notes sit at the panel, the page-level entry and the tracks only (not at the numbered entry, the `data-anno-*` sites, the steps' `rel` links or the entry's `aria-current`). A renamed class passes `just test` and breaks the picker or the arrival at runtime, where `book.spec.ts` covers most but not the register's page-level exemption. Fix: a cpe-web test that renders a reader and asserts each header-listed selector, and markers at the remaining sites.

- Phase 16: review Warning (dune DUNE-004/005), not fixed (docs-only phase; a check is code): rules the runbook states with no test behind them — a component root without `.page`, a new crumb receipt signal missing from `copied_signal_reset_event`, a `hidden` element under an explicit `display`, a thumbnail box moved without its IIIF fetch constant, a class `cpe.js` reads renamed in the markup (the last one is the Phase 13 deferral above). Fix: markup-contract or snapshot tests per rule.
- Phase 16: review Warning (dune DUNE-002), kept by decision: the inline-expression exception list now stands in CONTEXT.md, the runbook and the component-library page, three copies; the plan's checkbox asks the runbook and the page to state it. A gate on `<script`/`ExecuteScript` in CPE's Rust sources, naming CONTEXT.md as the list's home, would replace two copies with a pointer.

## Deviations

- Phase 13: proposed (ivan-reviewer, security-reviewer, maud-datastar-reviewer, Warning): `cpe/cpe.js` is served unversioned and with no `Cache-Control`, as decided for CPE's static files; the HTML and the script now share a contract (the `arrival` ids object, `pickerTarget`'s `null`), so after a deploy a browser may pair new HTML with a heuristically cached old script or the reverse. Decided behaviour left in place; options for the owner: a content hash or version in the URL (`StaticDir::file`), or `Cache-Control: no-cache` on `cpe/cpe.js`. No commit.

- Phase 12: provisional: superseding the entry below, the `@` → `\x40` escape lives in `cpe_core::util::js_string_escape` itself (dune-reviewer DUNE-001: a call-site `.replace` is the trap the next `data-*` literal falls into; the function is documented for `data-*` expressions and serves `breadcrumb.rs`, `annotation.rs`, `page.rs`'s reactive href and the router). The router's `pushState` `ExecuteScript` code is untouched; its output bytes change only for a URL or title holding `@`, which a `<script>` evaluates identically. Commit 1a8c3b18.

- Phase 12: provisional: the stale-address URL in the `data-init` writes `@` as `\x40` after `js_string_escape`, because DataStar 1.0.2 rewrites `@name(` into `__action(...)` even inside string literals (a query `x=@get(1)` would reach `replaceState` altered). Alternatives set aside: plain `js_string_escape` (the decided minimum; leaves that corruption), adding `@` to `cpe_core::util::js_string_escape` (changes the router's escaping too, outside this phase). Stronger than the decided escaping, never weaker. Commit d2d2c8bd.

- Phase 4: provisional: "no project folder" fails `cpe_server::validate` (and so `access-server validate`) in every run, not only in the recipe's.
  Alternatives set aside: a check in the recipe's shell (untestable as the plan's Rust test of "the run"), or failing only when an inputs
  directory or a must-be-served shortcode is given (a verb that passes over an empty or mistyped `CPE_PROJECTS_DIR` is the vacuous pass the
  plan guards against). Why: least scope, one rule for every caller. Commit 6767cb74.
- Phase 5: proposed: the pin test cuts each file at its first `#[cfg(test)]` module, as decided; rust-reviewer (Warning) notes that non-test
  code below a file's first test module is never counted (`resource_detail.rs` already has two test modules, today with no site after them).
  Left as decided; the gap is stated in the test's comment (d577b343). Proposed alternative for the owner: strip every test module through
  its closing column-0 `}` instead of truncating, so code after it is counted.
- Phase 6: provisional: `browse.spec.ts` covers the book list, its sort order (and a sort order as a link of its own) and the list and grid
  views, not a facet or a search, which the owner-decided journey list names: Incunabula declares no `facet-panel` and no `search-bar`
  (config.kdl in `areas/access/media/data/0803/…`), so there is neither to exercise and no empty state of a search. Alternatives set aside:
  adding a facet panel or search bar to Incunabula (a project change outside this phase), or a fixture project with them (the suite is
  Incunabula's, and `cpe-web`'s Rust tests already cover facets and search). Why: least scope; the journeys left are those Incunabula
  has. Commit b2cac079.
- Phase 7: provisional: the facade exception covers the `hooks` parameter of two methods, `Cpe::activate_with_hooks` and
  `Cpe::follow_with_hooks`, not of `activate_with_hooks` alone as the patch's test has it (`BOUNDARY_EXCEPTION` lists both
  function prefixes). Alternatives set aside: `follow_with_hooks` taking hooks some other way that names no CPE type
  (a new type in `cpe-server`, more scope), or no `follow_with_hooks` (the plan asks for it). Why: the decided
  `follow_with_hooks` cannot exist otherwise; still one parameter, same type. Commit 37df3363.
- Phase 7: proposed: decided, `Activated` keeps the host's hooks and re-activation reads them from what is served. security-reviewer
  (Warning) and dune DUNE-004: if the first activation of `follow_with_hooks` panics outside a project's load, nothing is stored and every
  re-activation loads with no hooks, so a host-hooked project answers 503 until a restart (fails closed, no disclosure). Left as decided;
  `follow_with_hooks`'s doc states it (a335113f). Proposed alternative for the owner: keep the host's hooks on `Cpe` (set once by
  `activate_with_hooks`/`follow_with_hooks`), or pass them into the `cpe-reactivate` thread, instead of on `Activated`.
- Phase 7: proposed: ADR-0007 (line 16 to 18) still says no `pub` item of `cpe-server` names another CPE crate's type but `cpe-ports`', so a
  composition root depends on `cpe-server` and `cpe-ports` only; ARCH-MAP.md, `areas/access/cpe/CONTEXT.md` and the crate doc name the
  decided exception (consistency and dune, Warning). An ADR's decision is the owner's to reword. Proposed wording, in place: "...
  `cpe-ports`' types excepted, and the `hooks` parameter of the `#[doc(hidden)]` `Cpe::activate_with_hooks` and `Cpe::follow_with_hooks`,
  which names `cpe_store_build::hook::Hook` for a host outside this repository that supplies its own projects' hooks; so a composition
  root of this repository depends on `cpe-server` and `cpe-ports` and on nothing else of CPE." and the `Enforced by:` line naming
  `cpe-server`'s `no_pub_item_names_another_cpe_crate` and `the_boundary_exception_is_the_hooks_parameter_alone`.

- Phase 10: proposed: upstream `id ... verbatim=#true` (store-build mapping/ids.rs, from incubator 60651cdf) lets an id of only dots (`.`, `..`) through:
  `is_url_safe` (generic/slug.rs) accepts them and a slug could never produce one. As decided: verbatim ids are held to `is_url_safe`.
  As committed: unchanged (ported as is). Finding (security-reviewer, Warning): such an id becomes a URL segment that browsers and proxies
  dot-normalise, so the resource is unreachable or lands on a parent route; no filesystem path. Proposed: `is_url_safe` refuses an id made
  only of dots. Commit 8fa1befb. Left for the owner because it changes the incubator commit's behaviour.
- Phase 15: provisional: which box moves to share the left edge. Chosen: the narrative's head and intro, when directly followed by a
  compound viewer with a contents rail, start at the frame's edge with the rail and the breadcrumb (record.css
  `.editorial.page:has(+ .compound-viewer.page > .cv-toc) > .narr:not(.narr-wide)`). Set aside: centring the viewer (cannot share the
  intro's edge at mid widths without narrowing the reader to --read), and dropping the left-edge assertion (the drift is the defect
  seen on Alice; nothing in grid.css intends it). Breakouts of such a narrative stay centred. Commits 590d0149, c4154f4c.
## Side findings

- Phase 14: ARCH-MAP fingerprint of `areas/access/server` (and CPE's) is stale after this phase's prose edits; run `/dune:dune-map update` (owned by dune-map).
- Phase 14: suggestions not acted on (review): hoist the page policy into one `const` in `serve.rs` shared by `assemble` and the tests (dune DUNE-002); `static_files` could derive "is SVG" from `content_type` (rust); pin a DPE `ServeDir` 404 and a 405 carrying the headers (security); `if_not_present` means a future CPE page policy replaces, not merges, the baseline directives (security); CPE's own `nosniff` entries are now redundant behind the layer but kept so cpe-server is safe mounted alone (dune DUNE-003); the header values are written in four doc places (simplicity Warning, no concrete failure: kept, the plan asks both operations pages to list them); the `_static` SVG URIs in the `serve.rs` test overlap `static_files`' per-file test (simplicity).
- Phase 14: re-review (dune, consistency) suggests adding `/telemetry/collect` to the `/healthz` no-headers assertion, or narrowing ARCH-MAP's "pin every answer kind" wording; not done.
- Phase 14: devops suggests a browser smoke of DPE's CSS and JS on the preview deploy, since `nosniff` now covers `ServeDir` files (a wrong Content-Type would block one).
- Phase 13: review suggestions left as they are: the doubled-base checks in `page.rs` and `router.rs` strip `_static/cpe/cpe.js` in two places (one helper would hold it); `snapshot_arrival_init_expr`'s `starts_with`/`ends_with` repeat its snapshot; the cpe-web test matches the four function definitions by their Biome-formatted text.
- Phase 13: `check.yml`'s `lint-e2e` job lints the DPE, Mosaic and CPE suites but not `areas/deposit/editor/web-e2e-tests`, which `just lint-e2e` lints (devops-reviewer; predates this phase).
- Phase 13: Biome is run through an unpinned `npx @biomejs/biome` in every suite and for `cpe.js`; a Biome release can fail CI with no code change (devops-reviewer). `public/.biome.json` is discovered by Biome 2.6.8 (its deprecation notice names the file), checked by hand.
- Phase 13: the picker's event handlers call `cpe.*` unguarded: with `cpe.js` missing a click on an entry or a mark falls through to the native link and the arrow keys and the open-scroll do nothing (accessibility-reviewer). Covered for the entry by the aborted-script spec.
- Phase 13: `arrival_init_expr` writes three compile-time ids into the expression without `js_string_escape`; safe while they stay constants (security-reviewer, Suggestion).
- Phase 13: `book.spec.ts`'s scroll cases sample the layout once after two frames; passed 3x on the parent binary and on the tree (typescript-reviewer suggests `expect.poll`).
- Phase 12: re-review (consistency, Suggestion-level): `js_string_escape`'s doc could fold `@` into its "Escapes, in order" list and say it matters only in `data-*` expressions; `router.rs`'s escaping comment (around 1649) does not mention `@`. Simplicity: the reload case and the in-page-Back case in `book.spec.ts` share their opening steps.

- Phase 12: security-reviewer (latent, not reachable): DataStar 1.0.2 restores its `🖕JS_<c>_DS🚀` escape spans with `String.replace`, so `` $` `` / `$'` in `c` expand as replacement patterns. Unreachable today because `current_path`/`current_query` reach `stale_url_correction` raw and percent-encoded (hyper rejects non-ASCII); recheck if they are ever decoded first.
- Phase 12: dune-reviewer DUNE-002: `window.__cpeReaderUrl` is spelled in `compound_viewer.rs` and in `router.rs` with nothing tying them; goes away when #582's follow-up drops the router write.
- Phase 12: typescript-reviewer: the older paging cases in `book.spec.ts` click right after `goto` with no proof DataStar has initialised; a click before init is a plain navigation that passes the same assertions.
- Phase 12: refuted at review: `$name` inside a string literal in a DataStar expression is not rewritten (v1.0.2's regex consumes quoted literals first); the e2e case with `x=$expand@get(1)` confirms it.

- Phase 11: review (maud-datastar, rust, consistency, simplicity, dune, reference) found no Critical or Warning. Suggestion, not acted on (rust, simplicity, consistency): the `PICKER_ESCAPE_EXPR` / `PICKER_FOCUSOUT_EXPR` doc comments in `dom.rs` still name the order control as "the `.orderctl` idiom" although it now consumes the constants; one sentence ("Shared by the page picker, the crumb dataset panel, the header language menu and the order control") would cover it; `render_picker`'s doc in `compound_viewer.rs` (~960) likewise. The eng.yaml-resolved `accessibility-reviewer` and `ivan-reviewer` were not run: the output is byte-identical (no snapshot changed).
- Phase 9: the squashed commit's message should say read_snapshot holds a snapshot to the rules on its content, not "every rule a generation's snapshot.nq is held to" (a3d0b3b3's body; d3b3933b corrected the docs).
- Phase 9: ARCH-MAP.md prose for areas/access/sync changed (d3b3933b); its fingerprint, and cpe-store-build's, are left for `/dune:dune-map update`.
- Phase 9: review suggestions, not acted on (upstream code, ported as is): read_quads and read_snapshot each map a syntax error with `line + 1` (rust, simplicity); read_snapshot does not call is_shortcode (security; shortcode reaches no path); no size cap on the reader, a caller should wrap a decoder in `Read::take` (security); ids.rs drops an Owned text silently (security, debug_assert); the foreign-graph test's `.replace` anchor is brittle (rust, simplicity); areas/access/sync/CONTEXT.md has no read_snapshot entry, and "draft projects" in its doc collides with hook::Draft (dune DUNE-003); README/ARCH-MAP could say the seam re-exports exactly Calendar and DatePrecision (dune DUNE-004); a composed provided text gets only the NUL check, as a borrowed one does, documented in config-language.md (security).
- Phase 9: the rerere cache held a resolution for sync-store lib.rs that dropped the patch's doc comments on read_snapshot and map_quads; they were restored by hand. A later re-apply of this patch may hit the same cached resolution.
- Phase 1: `thiserror` was used only by `SourcesError`; once that is test-only it moves from `cpe_server`'s deps to `cpe_server_test`'s (bfebb31f).
- Phase 1: the `cpe-server` ARCH-MAP entry's prose changed (995e3fc9) but its dune-map fingerprint was left for `/dune:dune-map`, which owns it.
- Phase 1: review suggestions, not acted on: group the test-only `DiscoveredProject` methods into one `#[cfg(test)] impl` block or `mod sql_folders` (simplicity, ivan, dune); drop `from_sql_folders` from ARCH-MAP's cpe-server public-items list, since it is no longer public (consistency, dune); new router tests should prefer `Cpe::activate` with the `testonly` port fakes over `from_sql_folders` (dune).
- Phase 2: review suggestions, not acted on: `load_all` could keep its earlier shape and `retain` the hidden ids after the loop (simplicity); the explicit X-Robots-Tag loop in the hidden-unknown test repeats what the header comparison already checks (simplicity); a hidden project going from unknown to served could get a follow test that its X-Robots-Tag returns (rust); `unavailable` could be derived from `statuses` (dune DUNE-005); a shortcode field on the failed log too (rust); a log line naming hidden shortcodes whose project is unknown (security).
- Phase 2: CPE's own not-found page says "it may name a project that has not been published"; `areas/access/CONTEXT.md` asks to avoid "unpublished" (cpe-components copy, out of this phase's scope).
- Phase 2: re-review of the fix round (consistency, simplicity, dune) found no verified Critical or Warning. Simplicity's
  Warning that `matches!(.., Loaded | Failed)` no longer hides a project with no status was refuted: every arm of
  `load_all`'s loop inserts a status. Not acted on: re-wrap the `router.rs` module doc line, shorten the cpe CONTEXT
  `_Avoid_` line, and the plan's own "unreleased project" wording (the plan is historical).
- Phase 3: ARCH-MAP prose for `access-server` and `cpe-server` changed (51b74f07); their dune-map fingerprints are left for `/dune:dune-map update`.
- Phase 3: ADR-0007's remaining "Not yet in code" line for the designed offline page still names DEV-7403; the owner split that page into its own ticket, whose id the line should name once it exists.
- Phase 2: the `cpe-server` ARCH-MAP fingerprint is stale again after this phase's changes (dune DUNE-001); left for `/dune:dune-map update`.
- Phase 3: review, refuted or not acted on: devops Critical "`just validate-data` fails on a clean checkout" refuted by measurement (the verb run from the root serves incunabula from the committed `areas/access/sync/data/0803`); devops Critical on `validate` in the image without mounts reclassified as documented behaviour (no caller runs it; missing data is a failure per ADR-0007), now stated in `docs/src/cpe/operations.md`; devops Warning that `validate` now reads `CPE_*`/`cpe.toml` is the owner's decision (same variables as `serve`); security Warning that warning rendering adds a panic path to `serve` refuted (the same `render` already runs there for error reports, spans from the same parser, offset clamped); simplicity Warnings to delete tests refuted (each test is a plan checkbox) and to share fixtures across crates refuted (separate crates).
- Phase 3: review suggestions, not acted on: `writeln!` instead of `push_str(&format!(..))` in `cpe_report`; a shared `#[cfg(test)] mod fixtures` in `access-server` instead of `validate::tests` and `serve::tests` importing each other (rust, dune DUNE-003); `CpeSetup` doc naming `hidden_shortcodes` as `serve`'s alone (dune DUNE-004); `ProjectOutcome::Unknown { reason }` vs `ProjectStatus::Unknown { report }`; one shared constructor of the two port adapters for both verbs (ivan); passing a writer into `dpe_server::validate` (ivan); tests for the outer `catch_unwind` path and the `CPE: not checked` output (rust); ARCH-MAP Purpose prose naming the verb (dune DUNE-005).
- Phase 3: re-review of fix round 1 (consistency, simplicity, dune) found no Critical or verified Warning. Not acted on: simplicity's `statuses.remove(..)?` in `activation::validate` can never drop a project (a `map` with `expect` would say so) and `printable` allocates per char; dune DUNE-001 (low): escaping has two owners, `escape_debug` on a skipped folder's name in `cpe-server` and `printable` in `access-server` (no double escape: `escape_debug` leaves no control character); a `validate` run without the data variables could name them (dune DUNE-002, low).
- Phase 4: review suggestions, not acted on: `--must-serve` accepts commas and repeats though the recipe passes one (simplicity; pinned by a
  test); a shorter `validate-cpe` comment; `writeln!` over `push_str(&format!(..))` (rust); escape Unicode format characters (bidi) in
  `printable` (security, cosmetic); `MUST_SERVE` justfile variable for 0803 (ivan); move the must-serve policy into cpe-server if more such
  flags come (dune DUNE-003); a `Checks` struct for `validate`'s seven parameters (dune DUNE-004); CONTEXT entries for *data-free check* and
  *must be served* in `areas/access/cpe/CONTEXT.md` (dune DUNE-005).
- Phase 4: `remodel_of` prefixes a config parse error with its file (`0803/config.kdl: …`) in activation's 503 report too; no test or doc
  quoted the old text.
- Phase 4: `test.yml`'s `timeout-minutes` comment ("tests and image smoke, 571 s", "Phases 2 to 8") does not count the new `just validate-cpe`
  step (reference-reviewer); the first CI run's duration decides whether the comment and the 21-minute limit change.
- Phase 4: ARCH-MAP prose for `access-server` and `cpe-core`/`cpe-server` changed (4df29e95); their dune-map fingerprints are left for `/dune:dune-map update`.
- Phase 4: re-review of fix round 1 (consistency, simplicity, dune): `check_narrative_file`'s behaviour verified unchanged. Not acted on
  (cap reached, Suggestions): a parity test running activation's check and `unresolved` over one table; reuse discovery's private
  regular-file helper in `inputs.rs`; a non-NotFound error on `narrative/` is reported only through the per-file lines.
- Phase 5: the plan's checkbox "sites in `shared-datastar` and the Mosaic tiles" is wrong about `shared-datastar`: `shared/datastar` holds no
  `PreEscaped`; the test's comment names only the Mosaic tiles (icon, textarea) (d577b343).
- Phase 5: `cpe_components_test` needed `rustc_env CARGO_MANIFEST_DIR` (relative): rules_rust's default is the absolute sandbox path of the
  compile action, gone when the test runs (`read_dir` NotFound), as `docs/src/bazel.md` documents.
- Phase 5: review suggestions, not acted on: drop the doc comment's first sentence (simplicity); `.expect` per `read_dir` entry instead of
  `.flatten()`, a CRLF-tolerant cut marker, and a note that a `PreEscaped(` in a non-test comment counts too (rust); one `glob` call in
  `data` (devops); `rust_sources` duplicates DPE's helper, to share if a third crate adopts the pattern (rust); ARCH-MAP line wrapping.
- Phase 5: ARCH-MAP prose for `cpe-components` changed (9319699e); its dune-map fingerprint is left for `/dune:dune-map update`.
- Phase 5: the eng.yaml-resolved reviewers `accessibility-reviewer` and `ivan-reviewer` were not run: the diff is a test, a test target's
  `data`/`rustc_env` and one ARCH-MAP sentence, with no markup or behaviour (brief: keep the review proportionate).
- Phase 5: re-review of fix round 1 (consistency, simplicity, dune): no verified finding. Simplicity's Warning that dropping `shared-datastar`
  hides a limit was refuted by grep (no `PreEscaped` under `shared/`). Not acted on: ARCH-MAP names only the call-syntax blind spot.
- Phase 6: CPE declares a cross-document view transition (`@view-transition { navigation: auto }` under
  `prefers-reduced-motion: no-preference`, `cpe-web` `page.rs`). In headless Chromium a click right after a page turn intermittently landed on
  the root for the whole 30 s click timeout (pointer events go to the root while a transition runs); a probe found it clear within 500 ms
  in isolation. The suite runs with `reducedMotion: "reduce"`, so it does not exercise view transitions; whether a real browser ever keeps the
  root blocking that long was not established.
- Phase 6: on a strip page, axe once reported `color-contrast` on the viewer rail foot (`.rv__rail-foot > .vctl`, the details toggle)
  mid-fade on load (foreground #585858 on #7f7d7c); at rest the rail foot is `visibility: hidden` and axe passes. Reduced motion removed it in
  every later run (4 repeats).
- Phase 6: no `test.fixme` remains; the no-JavaScript pass found no broken path, so chunk 6.4 (no-JS fixes) had nothing to do.
- Phase 6: review suggestions not taken (each matches the DPE or editor suite, or is beyond the plan): axe scans only resting states, so
  the header menu, sort menu, page picker, data panel and annotation register are never scanned open (accessibility-reviewer); `results.incomplete`
  is ignored, so contrast over images is never judged; no step type-checks the suites (`tsc --noEmit` is not in `check.yml`); `axe-helpers.ts`
  imports `axe-core` types through hoisting, as the editor suite does; `reporter: "html"`, `actionTimeout: 0`, `reuseExistingServer` and
  `npm install` in the workflow follow the other suites; `.github/dependabot.yml` lists neither the editor's nor CPE's suite; `a11y-cpe.yml`
  does not trigger on `justfile` or itself, as `a11y-dpe.yml` does not; DPE's lock pins Playwright 1.58.2, the editor's and CPE's 1.63.0.
- Phase 7: review, refuted or not acted on: simplicity's Warning to set `Activated.hooks` in `from_ports` rather than pass hooks to
  `load_all` is against the plan's checkbox ("`load_all` takes the hooks"); its Warning that the facade exception is stated in five places is
  the plan's own list (ARCH-MAP, CONTEXT, crate doc), each where a reader of that file looks. Suggestions not taken: one-pass `hook_of`
  (simplicity); borrow instead of clone the hooks in `reactivate`, call `hook_of` before the port reads, derive `Debug` on `Hook` (rust);
  a test that `#[doc(hidden)]` stays on both methods (dune DUNE-002); make `hooks::resolve` test-only now that only store-build's tests
  call it (dune DUNE-003, consistency); a note that a host building this tree grants itself visibility of `cpe-store-build`
  (consistency); host rows match folder names case-sensitively (security). Reference review: no findings.
- Phase 7: ARCH-MAP prose for `cpe-server` and `cpe-store-build` changed (37df3363, a335113f, 33c5faea); their dune-map fingerprints are
  left for `/dune:dune-map update`.
- Phase 8: ARCH-MAP.md fingerprints for the cpe-core and cpe-store-build components are stale after 8affbbd0 (owned by /dune:dune-map; not edited).
- Phase 8: suggestion (dune): areas/access/cpe/CONTEXT.md's **Mapping** term does not name `ignore-class`, and its port-table row on standoff markup ("never as an opt-in flag") could be misread against the new `richtext=#true`, which is a store-side paragraph conversion, not standoff. Not edited: beyond the ported commit.
- Phase 8: suggestion (simplicity): relative "now" wording in finalise.rs ("converted only now") and config-language.md step 7 ("converted now and not before"); ragged rewraps in diagnostic.rs remodel docs and two merged lines (README ~124, config-language.md ~716). Upstream text, left as ported.
- Phase 8: suggestion (reference): config-language.md step 7 cites `dsp-incubator cpe/docs/adr/0015-…, N2`, which does not resolve here; pre-existing pattern, out of scope (DEV-7416).
- Phase 8: suggestion (security): richtext rows render through `render_richtext_fragment` (drops raw HTML, URL allow-list); a regression test feeding `javascript:`/`data:` links would pin it. Archive text written as richtext is interpreted as Markdown (headings, links, external images) by design.

- Phase 10: review suggestions on the ported code, not acted on (upstream, ported as is): cpe-core diagnostic.rs doc comments of
  DuplicateRemodelEntry, InvalidRemodelValue, UnknownRemodelReference and ConflictingRemodelAttributes do not name the new nodes' cases
  that config-language.md lists (consistency, Warning); FileLegal's field docs cite knora-base:hasAuthorship/hasCopyrightHolder where
  sync-store reads dao:authorship/dao:copyrightHolder (dune DUNE-003, consistency); files.rs cites FORMAT.md §1 for a plain xsd:string,
  §8 is the section (reference); no cap on authorship count/length, `.JP2`/`.JPX` in upper case refused as NotJp2, an empty author
  gives ", x" in the attribution (security); authorship order is the snapshot's fact order, deterministic per input (rust, security);
  CONTEXT.md has no glossary line for the declared-images-class sense of "image" (dune DUNE-004); sync CONTEXT.md could name FileLegal
  (DUNE-005). Simplicity's wish to drop "for now" in sync CONTEXT.md is not taken: it carries the owner's "left out for now, not for good".
- Phase 10: ARCH-MAP.md fingerprints of cpe-ports, cpe-core, cpe-store-build and sync-store are stale after this phase (dune DUNE-001);
  left for `/dune:dune-map update`.
- Phase 10: consistency's finding that CPE CONTEXT.md:46 ("the port carries no rights") contradicts FileLegal is refuted: that line is
  about the Bytes port, not the archive projection port.
- Phase 10: commits use the session's reminder attribution (Claude Opus 5.5), not executor-notes.md's Fable 5.2 line, since that is the
  model that wrote them; the session's squash can normalise it.

- Phase 15: the live Alice page (wonderland?page=SCH_002) scrolls 55 px sideways at 320 px; the elements past the edge are the breadcrumb's
  dataset panel (`.crumb-data__*`, `.dasch-logo__img`). Not the contents rail; unverified on this tree.
- Phase 15: in `data-narr="rail"` with `data-narr-side="start"` at >=80rem the intro sits in the narrative's second track, so it does
  not share the contents rail's edge; rail-layout.spec.ts covers only `text` mode. A narrative with sections (breakout) above a rail
  keeps its sections centred while its head and intro start at the frame edge.
- Phase 15: ARCH-MAP.md's areas/access/cpe fingerprint is stale after this phase (dune-reviewer); `/dune:dune-map update` owns it.
- Phase 15: rail-layout.spec.ts keeps its own copy of `document_head`'s stylesheet list and of the narrative shell from page.rs; a new
  stylesheet would leave the spec's page behind the served one (dune/consistency suggestion).
- Phase 16: `areas/access/cpe/CONTEXT.md` (*The domain crate*, last paragraph) says the test tying `validate/header_budget.rs` to `tokens.css` is still the incubator's; `web/tests/header_budget.rs` (`tokens_match_the_budget_formula`) runs here in `cpe_web_integration`. The sentence is stale.
- Phase 16: `areas/access/cpe/store-build/README.md`, *Adding a project*, step 1 says to write `areas/access/media/data/<shortcode>/config.kdl`; for a project the VRE bridge refreshes the authoritative copy is `areas/deposit/vre-bridge/inputs/<shortcode>/` (as its own intro says), and the `media` stand-in is a fixture copy (dune DUNE-003). The runbook names the shipped inputs.
- Phase 16: ARCH-MAP's `areas/access/cpe` fingerprint is stale after this phase (a new `areas/access/cpe/CLAUDE.md`); `/dune:dune-map` owns the line.
- Phase 16: suggestions not taken: simplicity asked to cut the runbook's restatements (pub-surface rule, layout contract, ADR pointers) to pointers and drop module-name lists; dune asked to hand architecture.md's port and activation text to CONTEXT.md by link and to drop CONTEXT.md's snapshot and fixture counts; consistency asked to move the eng.yaml override into the per-application block at the top (it sits beside CPE's other overrides, as the checkbox says).
- Phase 16: after the fix-round cap, the round-2 re-review raised wording suggestions only, not taken: the runbook's hygiene-list pointer could say "every project adds to one list, one with a hook to both" (simplicity, dune); the release paragraph could be tightened to two sentences; the 503-for-every-project clause could cite CONTEXT.md's *Unavailable project* beside architecture.md's table, whose `Failed` row implies it; operations.md's "holds no project directory" over the runbook's "empty" (consistency).

## After the run (2026-11-22)

The owner read the report and asked for the open findings to be addressed. Each fix is folded into the layer that owns
the code; the entries above keep their wording as written during the run. The choices marked "session's choice" were
taken without the owner and are his to veto.

- Phase 2 deferral (partial mount): settled. CPE's not-found answers carry `Cache-Control: no-store`, and a re-activation
  logs a `warn` line for a project that was served and is unknown now (`withdrawn_by`). Session's choice among the three
  options. The 404s of an unknown resource id inside a served project and of the fragment handlers carry no
  `Cache-Control` still.
- Phase 4 deferral (site block): settled. `cpe_core::validate::check_site` runs the site-block checks that need no data
  and no assets, sharing its functions with the full validation; the data-free check calls it. Left to the full load:
  `CoverImageNotFound`, and the nav and not-found route checks.
- Phase 5 deviation (pin test): settled. The test strips every `#[cfg(test)]` module and fails on one it cannot delimit.
  No count changed.
- Phase 7 deviation (hooks lost after a panic): settled. The host's hooks are a private field of `Cpe`, set before the
  first activation; `Activated` carries none and `load_all` no longer takes them. Session's choice of the proposed
  alternative.
- Phase 7 deviation (ADR-0007): settled. The ADR names the exception with the proposed wording.
- Phase 10 deviation (ids of only dots): settled. `is_url_safe` refuses them, for every id that becomes a URL segment,
  which departs from incubator commit 60651cdf; the incubator needs the same rule when the pin moves.
- Phase 13 deviation (`cpe.js` caching): settled. `cpe/cpe.js` is answered with `Cache-Control: no-cache`, an `ETag`
  and a 304. Session's choice over a content hash in the URL.
- Phase 13 deferral (selectors): settled. `cpe_js_header_selectors_occur_in_a_reader_page` in `cpe-web` reads the
  selector list from `cpe.js`'s header and holds it to a rendered reader page. The Phase 16 deferral's other rules
  (a component root without `.page`, the receipt signal, `hidden` under `display`, the thumbnail constant) are open.
- Alice (0854) was loaded locally from the incubator's project folder and checked in a browser. Phase 15's side finding
  is corrected: the 55 px of sideways scroll at 320 px on `wonderland?page=SCH_002` came from keyword chips that could
  not wrap, not from the breadcrumb's panel; fixed in Phase 15's layer with a spec case (55 to 0 on the page). From
  1024 px the breadcrumb, head, intro and rail share the left edge. Open, the owner's: below 1024 px the rail follows
  the chapter's whole text; the title breaks mid-word at 320 px; the contents layout has no arrow-key paging; the
  viewer's error state clips its control bar. Alice's images answer 404 on the demo IIIF host (the incubator's
  snapshot names files of an earlier upload).
- The report's "IIIF host unreachable" was wrong: the host answers, and Incunabula's scans load. A black canvas was
  seen once right after paging with the viewer partly off-screen; its cause is not established.

## Owner decisions

## Closeout

- root_cause: not a bug fix. CPE was brought into dsp-repository under time pressure and served one project. What a
  second project needs was missing or half done: a way to merge a project without it going live on prod, a strict
  validation in CI, a browser test suite, the incubator's CPE changes, and a clean-up of the JavaScript the Rust code
  carried as strings.
- investigation: four grounding passes before planning showed that the VRE bridge stack had already changed the
  answers. A project is served only where the bridge wrote its Ready token, so "keep it off prod" came down to what an
  unknown project answers and to allowlists per environment. The issue texts were stale in several places (snapshots
  and the DataStar gate already covered CPE; one inline script was left, not nine). The plan went through three review
  rounds; the dry run of the incubator's patches showed which hunks the stack had made obsolete. During execution:
  DataStar rewrites `@name(` inside string literals (Phase 12); a throwing `data-init` stops a sibling's (Phase 13);
  `CARGO_MANIFEST_DIR` is relative under Bazel, so `include_str!` through it does not compile (Phase 8); `git apply
  --3way` can reuse a recorded rerere resolution that drops part of a patch (Phase 9); CPE's view transition swallows
  clicks in headless Chromium unless motion is reduced (Phase 6).
- solution: sixteen single-commit pull requests stacked on the VRE bridge stack's tip, one phase each. The SQL-folder
  loader is test-only. A project with no Ready token answers 404 (ADR-0007 rewritten). `access-server validate` checks
  DPE and CPE strictly, with a data-free check of shipped inputs, run in CI. A Playwright suite with a no-JavaScript
  pass and axe. The incubator's four changes. No inline script in cpe-components; the arrival and the page picker in
  `cpe.js`. Baseline security headers. The contents rail's layout. CPE's runbook and two docs pages. The final review
  of the combined diff added one fix (the data-free check uses the port's file-name rule) and three doc corrections.
- prevention: the `PreEscaped` pin test; the end-to-end suite as the guard for behaviour that snapshots cannot see,
  with new cases run on the parent commit before each move; `just validate-cpe` with a must-be-served shortcode
  against a vacuous pass; `js_string_escape` for every value in a DataStar expression. Open for the owner: the
  entries under Deviations and Deferrals above, none of which is settled.
