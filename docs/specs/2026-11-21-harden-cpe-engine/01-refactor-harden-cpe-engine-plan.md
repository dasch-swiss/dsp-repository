---
title: "refactor: harden the CPE engine before the next project goes to production"
type: refactor
date: 2026-11-21
author: "Balduin Landolt"
status: implemented
repository: dasch-swiss/dsp-repository
linear_project: harden-the-cpe-engine-4e1b7e137f11
---

# refactor: harden the CPE engine before the next project goes to production

## Overview

CPE serves Incunabula in production. Before a second project follows (HdM, then Solec), the Linear project
*Harden the CPE engine* (P-DEV-411) asks for four things: a project can be merged without going live on prod,
a strict validation runs in CI, the engine has a browser test suite, and the code is cleaned up. HdM also
needs the CPE changes that so far exist only in dsp-incubator's vendored copy.

This plan covers nine of the project's issues in dsp-repository. It ships as one stack of single-commit draft
pull requests, one phase per pull request, managed with `gh stack`
(`docs/src/git-conventions.md`, *One PR, several commits, or a stack*). The stack's trunk is
`feature/dev-8001-access-server-no-stand-in-data`, the tip of the VRE bridge stack (#610 to #628), because that
stack rewrites the start path, the stores and the data layout this work builds on.

| Phase | Pull request | Linear |
|---|---|---|
| 1 | The SQL-folder loader is test-only | DEV-7633 |
| 2 | An unknown project answers 404 | DEV-7786 |
| 3 | `access-server validate` checks CPE's projects, strictly | DEV-7403 |
| 4 | CI validates every committed CPE project | DEV-7403 |
| 5 | A test pins CPE's `PreEscaped` sites | DEV-7404 |
| 6 | A Playwright suite for Incunabula, with and without JavaScript, and an axe workflow | DEV-7404 |
| 7 | Port: a host supplies a project's hooks | DEV-7801 |
| 8 | Port: remodel features beyond Incunabula | DEV-7801 |
| 9 | Port: a hook writes text it composes | DEV-7801 |
| 10 | Port: files carry their rights; declared image classes, merged classes, membership | DEV-7801 |
| 11 | Shared disclosure expressions, and the annotation dot as markup | DEV-7510 |
| 12 | The reader's inline script becomes `data-init` | DEV-7510 |
| 13 | The arrival script and the page picker move to `cpe.js` | DEV-7510 |
| 14 | Headers that cannot break a page, and a script-blocking policy on static SVGs | DEV-7499 |
| 15 | The contents rail lays out at every width | DEV-7800 |
| 16 | CPE's runbook, architecture page and component-library page | DEV-7406 |

This plan and its execution journal are the stack's bottom pull request, below Phase 1.

`docs/specs/README.md` asks for phases as commits within one pull request. Here each phase is a change a
reviewer judges on its own and closes or advances a different issue, so the stack form applies, as it did for
the DaTEI integration and the Bazel migration.

Phases 1 to 10 are the first wave: what has to hold before a second project goes to prod, and what HdM needs.
Phases 11 to 16 are the second wave. The first wave must be green before the second starts.

## Problem Statement / Motivation

- **A merged project is live everywhere (DEV-7786).** The issue was written before the VRE bridge. With the
  bridge, a project is served only where its data and Ready token exist, and the bridge writes those only for
  shortcodes on a deployment's allowlist. What is left: a project with no token answers 503, which confirms that
  it exists, and logs a warning on every start, although on prod that state is the intended one.
- **Nothing checks a project strictly (DEV-7403).** A project that fails takes only itself offline, which is
  right for production and wrong for CI: a broken configuration merges unnoticed. `access-server validate`
  checks DPE only, and no workflow runs it.
- **No browser test covers CPE (DEV-7404).** The snapshots (177) prove markup. Nothing proves that a page
  works, with or without JavaScript, and nothing runs axe on CPE.
- **The incubator's CPE is ahead of the monorepo's (DEV-7801).** Four commits in dsp-incubator
  (#556, #559, #561, #563) changed the vendored CPE so that HdM and Alice could be drafted. HdM cannot be
  served from dsp-repository without them.
- **Hand-written JavaScript in Rust (DEV-7510).** One inline `<script>` and one `ExecuteScript` remain, plus a
  few long DataStar expressions that are small programs. They are untestable as strings and are what a
  Content-Security-Policy would have to allow.
- **Smaller items.** A legacy constructor only tests use (DEV-7633), no baseline security headers (DEV-7499),
  a layout defect on pages with a contents rail (DEV-7800), and missing pages of CPE's documentation (DEV-7406).

## Proposed Solution

The owner settled the open points on 2026-11-21; each issue in Linear carries them. They are binding for this
plan, and a review finding that would change one is listed for the owner, not acted on.

### Phase 1: the SQL-folder loader is test-only (DEV-7633)

`Cpe::from_sql_folders` (`areas/access/cpe/server/src/lib.rs:175`), `activation::from_sql_folders`
(`activation.rs:112`) and `discover_sql_folders` stay, but compile only for tests (`#[cfg(test)]`). The
router's tests load eight `data.sql` fixtures through that path; porting them to the ports would rewrite tests
that are about the router. `cpe-server`'s public constructors are then the port-based ones.

`#[cfg(test)]` is enough: every caller is a unit test inside `cpe-server` (`lib.rs`, `base_tests.rs`,
`router.rs`, `assets.rs`, `discovery.rs`), the crate has no integration-test sources (`tests/` holds only
fixtures), and `cpe_server_test` is
`rust_test(crate = ":cpe_server")`. Gating the three functions leaves helpers in `discovery.rs` that only they
use (`SourcesError`, `read_utf8`, `has_data_sql`, `config_source`, `data_sql`, `narrative_contents`, and
`narrative_files` and `is_regular_file` if nothing else calls them); each one the compiler reports as dead is
gated too, and no `dead_code` allowance is added.

`areas/access/cpe/CONTEXT.md` (lines 300 to 303) and `ARCH-MAP.md` (lines 1350, 1456 to 1458, 1500) drop the
"legacy, ends with DEV-7498" wording and describe the loader as test support, as do the doc comments at
`lib.rs:166` and in `discovery.rs`. dsp-incubator's host no longer calls the constructor (checked on its
`origin/main`, 2026-11-21); its vendored copy follows at its next pin move.

### Phase 2: an unknown project answers 404 (DEV-7786)

A project whose folder exists but which has no Ready token on this deployment is `ProjectStatus::Unknown`
(`areas/access/cpe/server/src/activation.rs:36`; produced in `port_sources`, `activation.rs:273`, when the
location is present). Today it answers the 503 page on its prefix. It will answer CPE's own not-found page
with status 404, the response an unregistered path gets, so nothing tells an unknown project from a path that
never existed. It stays out of the overview, as today. A project that *fails* (`Failed`) keeps its 503 with
`Retry-After` and `no-store`. A location that is missing or holds no project at all still marks every project
`Failed`: that is a broken deployment, not an unreleased project. A token that is broken or names a missing
generation is `Failed` too, never unknown.

How: `load_all` pushes unknown and failed ids into one `unavailable` list (`activation.rs:143` to `176`) that
`router::build` turns into the 503 branch of `fallback_handler` (`router.rs:535` to `548`). Unknown ids stop
being pushed; routes are registered for loaded projects only (`router.rs:132` to `147`), so the fallback then
reaches `render_not_found_site` (`router.rs:601`), which already answers 404 without `Retry-After`.
The warning at `activation.rs:158` becomes `tracing::info!`; it is written on every
activation, and so again on each re-activation the stores announce.

**Hidden projects.** `hidden_ids` (`activation.rs:140`) is built from every discovered folder, unknown ones
included, and `robots_for_hidden_projects` (`request.rs:123`) adds `X-Robots-Tag` to every answer under a
hidden id. A hidden unknown project's 404 would then differ from any other 404 by that header. `hidden_ids`
is therefore built from served and failed projects only. Its log line for a hidden shortcode that "names no
project folder" (`activation.rs:236`) must not fire for a folder that exists and is unknown, and the *Hidden
project* entry in `areas/access/CONTEXT.md` (line 33), which says every response under a hidden project's
prefix carries the header, is qualified: a hidden project that is unknown answers as any unknown project.

**Coming and going.** When a Ready token arrives, `sync-store` announces the project and CPE re-activates
(DEV-7622); the project then serves. When a token is removed, the project becomes unknown again and answers
404. Both paths exist; this phase changes what they answer and tests both.

**This reverses a rule.** ADR-0007 (lines 113 to 118) says: "'Not ready' is kept apart from 'not there'. 404 is
for a path that names no project", and `ARCH-MAP.md` (lines 1448 to 1449) says what did not load answers 503,
"never 404". The media-minimum plan tested that a project `media` does not hold answers 503. The owner decided
on 2026-11-21 that a project with no Ready token is, on that deployment, *not there*: a 503 tells a visitor
that an unreleased project exists. ADR-0007 is rewritten in place to say so (the old rule goes in the commit
body, per `docs/adr/CLAUDE.md`); "not ready" keeps meaning a project that has data and failed.

Releasing a project on a deployment is adding its shortcode to that deployment's bridge allowlist
(`EDITOR_VRE_BRIDGE_ALLOWLIST`), an ops-deploy change (H2). `docs/src/cpe/operations.md` gains a short section
*Releasing a project on a deployment*, and `areas/access/CONTEXT.md` defines *unknown project* (the status,
apart from `UnknownProject`, the port's refusal, and from an unknown path) and *released* in its entry format.
No new status, list or file marker is added.

### Phases 3 and 4: strict validation (DEV-7403)

**Phase 3, the verb.** `cpe-server` gains a public function that activates every project from the configured
ports once, without serving, and returns each project's outcome: served (with its warnings), unknown, or
failed (with its report). A folder discovery skips for a malformed or unreadable `SHORTCODE`
(`Discovery.skipped`, `discovery.rs:243` to `262`) is reported as failed too; a folder with no `SHORTCODE`
is no project and is not reported. Activation is blocking and synchronous (`Cpe::activate`, `lib.rs:117`), so
the verb needs no runtime. The function takes a `CpeConfig` and the ports as arguments; only `main` builds
them from the environment, so tests never touch process environment. Today
`ProjectStatus` carries no report and a failed project's report is only logged (`activation.rs:158` to `177`),
so `load_all` keeps each project's rendered report and warnings for the function to return. The outcome type
and the function are defined in `cpe-server`'s `lib.rs`, because the facade test
`no_pub_item_names_another_cpe_crate` allows no other `pub use`; the type names no `cpe-core` type, and
diagnostics cross the boundary as the text `report.render` already produces.

`access-server validate <data_dir>` keeps its required DPE data directory, checks DPE, then checks CPE.
`dpe_server::validate` returns an opaque `ExitCode` today (`areas/access/dpe/server/src/lib.rs:107`), which
cannot be combined with a second result; it returns a `bool`-like result instead, and `main.rs` turns the two
results into the exit code. CPE's configuration comes from a function `serve` and `validate` share, extracted
from `serve.rs` (`CpeConfig::load` and the two data-directory defaults, around lines 293 to 302; the hidden
shortcodes and `DPE_SITE_ADDR` between them stay in `serve`), so one configuration describes both verbs.
`docs/src/dpe/operations.md` (lines 34 to 42) and the verb's help text in `cli.rs` describe it as DPE-only
today and are brought up to date. CPE's projects are checked whether or not `CPE_BASE` is set: the verb validates what is
committed, not what is mounted.

The verb is strict: any failed project, or a DPE failure, makes it exit non-zero, after printing every
project's report (not only the first's). An unknown project prints a warning and does not fail it. `serve` is
unchanged; it gets no strict start, because CPE never stops its host. ADR-0007's sentence that a strict mode
"fails the whole start on the first invalid project" is rewritten to describe the verb, and its *Not yet in
code* sentence (lines 318 to 319) keeps only the designed offline page.

**Phase 4, CI.** Two checks, both run by one `just` recipe that `test.yml`'s required job calls after
`just test`:

- *Full validation where a stand-in is committed.* `access-server validate` runs from the repository root
  (`bazel run --run_in_cwd`, as `validate-data` does) against the committed fixtures under
  `areas/access/{sync,media}/data`, which are `serve`'s defaults. It is given the shortcodes that must be
  served (0803 today), and fails when one of them is not: otherwise a deleted fixture would turn Incunabula
  into an unknown project and the run would pass.
- *Data-free check for every committed project.* For each project folder: its `SHORTCODE` is a valid
  shortcode; and, where the VRE bridge ships inputs for that shortcode
  (`areas/deposit/vre-bridge/inputs/<shortcode>/`, laid out as `config.kdl`, `curation.csv`,
  `narrative/<name>`), the configuration parses as KDL, its remodel block loads (`cpe-core`'s `load_remodel`),
  the mapping's shortcode equals the folder's, and each named narrative has its base file or its
  default-language variant (the rule activation applies, `cpe-core`'s `validate.rs:4862` to `4900`; other
  language variants are optional). The default language is the site's `default-language`, else its first
  language; `NamedNarratives` carries only files and languages today and gains the default language. Page and
  component validation needs archive data and is not part of it. The inputs directory has no Ready token and
  no generation folder, so it is read by a small reader over a plain directory, or by the part of
  `port_sources` (`activation.rs:308` to `339`) factored out for both.

The inputs directory reaches the verb as a path argument of the recipe; no Bazel target of one area depends on
another's. This is a third read across the two areas by path, beside the two ADR-0002 names for the stand-ins
(`deposit-server` writes, `access-server` reads; the `just check` pin of the 0803 fixture,
`.github/scripts/check-stand-in-fixture.sh`); ADR-0002 names it where it names those. For 0803 the pin already
makes the inputs equal the fixture, so the data-free check adds nothing there; it is the only check for a
project with shipped inputs and no fixture, which is what HdM will be.

A committed project with neither stand-in data nor shipped inputs passes with a printed note, since nothing of
it can be checked. The recipe never skips silently: it fails when it finds no project folder, and when an
inputs directory it was given does not exist.

The verb's DPE data directory is a required argument, so the recipe passes `DPE_DATA_DIR` as `validate-data`
does, and DPE's data validation runs in CI for the first time with it. `just validate-data` passes on the
trunk today or the phase fixes what it reports; the pull request says which. The recipe adds a `bazel run` of
the binary to the required job; its effect on the job's duration is read from the first CI run.

### Phases 5 and 6: tests (DEV-7404)

**Phase 5.** A test in `cpe-components` pins the number of `PreEscaped(` call sites per source file, with the
sanctioned input of each named in a comment, as `dpe-server`'s test does for DPE
(`areas/access/dpe/server/src/metadata.rs:777` to `805`). It fails when a site is added or removed without the
pin being changed, which makes every new site a reviewed one. Like DPE's, it reads `src/**/*.rs` at run time
below `CARGO_MANIFEST_DIR` and builds its needle at run time so its own text is not counted;
`cpe_components_test` gains the sources as `data` (as `areas/access/dpe/server/BUILD.bazel:81` to `84` does).
Each file is cut at its `#[cfg(test)]` module before counting. Today's sites, all in `cpe-components`:
`editorial.rs` 10 (the markdown renderer's escaped output), `markdown.rs` 2, `compound_viewer.rs` 1 (the
reader's script, which Phase 12 removes), `annotation.rs` 1 (`ANNO_DOT`, which Phase 11 removes). `cpe-web`,
`cpe-server`, `cpe-core` and the image viewer have none. The test does not see a splice written without the
call syntax (`.map(PreEscaped)`); its comment says so.

**Phase 6.** A Playwright suite in `areas/access/cpe/web-e2e-tests/`, built like the editor's
(`areas/deposit/editor/web-e2e-tests/`): two projects, `chromium-js` and `chromium-nojs`
(`javaScriptEnabled: false`), the second ignoring the axe spec; no `setup` project, since nothing signs in.
Its configuration throws when `ACCESS_SERVER_BIN` is unset or the file is missing.

Its `webServer` runs `access-server serve` from the repository root with `CPE_BASE=/cpe` and
`DPE_SITE_ADDR=127.0.0.1:4102` (DPE's suite uses 4000, the editor's 4101). Nothing else is set: the defaults
of `CPE_PROJECTS_DIR`, `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` are the committed 0803 data when run from the
root (`areas/access/server/src/serve.rs:14`, `18`; `areas/access/cpe/server/src/config.rs:55`), and CPE's
stylesheets are compiled into the binary, so no CSS build precedes the suite. The server answers `/healthz`
as soon as it binds and activates afterwards, with CPE answering 503 until then; readiness is therefore
`webServer.url` on the project's front page, `http://127.0.0.1:4102/cpe/incunabula/`, not `port`, with a
timeout above the default.

Incunabula's pages are below `/cpe/incunabula`; `/cpe/` is the overview. Journeys, each a `{feature}.spec.ts`,
each in both passes unless noted:

| Spec | Covers |
|---|---|
| `overview.spec.ts` | `/cpe/` lists Incunabula; a path that names no project answers the not-found page with 404 |
| `browse.spec.ts` | `/books` lists resources; a sort order reorders the list; the list and grid views. Incunabula's configuration declares no facet panel and no search bar, so no facet or search journey exists to test (found while building the suite) |
| `book.spec.ts` | open a book from the list; page forward and back in the reader; Back and Forward keep the reader and the URL in step (JS pass); a stale or non-numeric `?page=` lands on a valid page |
| `strips.spec.ts` | `/strips` and one strip |
| `narrative.spec.ts` | `/`, `/narrenschiff` and `/woodcuts` render their narrative and links |
| `accessibility.spec.ts` | axe (WCAG 2.1 AA) on one page of each kind (JS pass only) |

Specs locate by role and accessible name, never by framework class or `data-testid`. They do not assert on
image loads: images come from an external IIIF host. The 503 page and the unknown-to-served transition are
covered by `cpe-server`'s tests, not by this suite.

A no-JS path the suite finds broken is fixed in this phase when the fix is small (a missing `href` fallback,
a form without an action). A larger one is marked `test.fixme` with a comment naming the defect and is listed
in the pull request's *Gotchas*; the pull request then opens with `Part of DEV-7404`, not `Fixes`.

Recipe `test-e2e-cpe` follows `test-e2e-editor` without its CSS step: it builds the binary with
`_release-bin` (`justfile:645`, which passes `--remote_download_toplevel`) and runs both projects, axe
included. `test-a11y-cpe` is the local axe-only recipe, as `test-a11y-editor` is. `install-e2e-requirements`,
`lint-e2e` and `check.yml`'s `lint-e2e` job gain the new directory (the job lacks the editor's today; that is
noted in the pull request and left alone). A workflow `a11y-cpe.yml`, shaped like `a11y-dpe.yml` including
its `ubuntu-24.04` runner pin, runs `just test-e2e-cpe` on changes under `areas/access/**`, `shared/**`,
`modules/mosaic/**` and the Bazel inputs, which is everything the binary is built from.

ADR-0004's line "Not yet in code: CPE has no end-to-end suite, so no journey of it runs with JavaScript
disabled (DEV-7404)" (line 76) is removed and its *Enforced by* line names CPE's pass; the other *Not yet in
code* line there is DPE's and stays. `docs/src/dpe/testing-strategy.md` (lines 39 to 40) says CPE's pages are
covered by its crates' tests only; that passage is rewritten for the suite.

### Phases 7 to 10: the incubator's changes (DEV-7801)

Each phase applies one incubator commit's change to the vendored tree, as a three-way apply of
`git -C <dsp-incubator> diff <commit>^ <commit> --relative=cpe/vendor/dsp-repository`, then fits it to the
code as it is on the stack. The commits were written as dsp-repository commits; their messages are the
starting point for these commits' messages.

| Phase | Incubator commit | What it brings | Fit to the stack |
|---|---|---|---|
| 7 | `521b0916` (#556, DEV-7627) | `Cpe::activate_with_hooks`: a host can supply a project's hook | Every `sync-store` hunk is dropped: they are `LiveArchiveProjection::serving` and `InvalidShortcode`, which the stack's Ready tokens replace and nothing calls. `port_sources` returns `Result<Sources, NotLoaded>` on the stack, so the hunk that adds `hooks` and `hook_of` to it is applied by hand. `follow.rs:88` is a second caller of `from_ports` the patch does not know |
| 8 | `c8bdcfe9` (#559, DEV-7628) | the mapping reads integers, list nodes, rich text and dates of mixed precision, and can leave classes out | applies cleanly (26 files) |
| 9 | `c7d6c2c8` (#561, DEV-7629) | a hook can write text it composes (`provide_text` takes a `Cow`); `read_snapshot` parses a snapshot from a reader | conflicts only in `sync-store/src/lib.rs`'s doc comment and imports. `read_snapshot` takes a reader, so it needs no fitting to generation directories; two of its tests use `serving` and `write_0804` from the parts Phase 7 leaves out and are rewritten against a reader over a snapshot file |
| 10 | `60651cdf` (#563, DEV-7630) | files carry their legal information; a declared image class; merged archive classes; membership and ordered links | applies cleanly (31 files). The stack's test `test_mapping_rights_classification_and_legal_information_serves_the_same_snapshot_as_without` (`sync-store`'s `mapping_tests.rs:878`) then fails: it is cut down to rights and classification, and a new test asserts that a file's legal information is served. `vocab.rs`'s header (lines 2 to 4), `sync-store/src/lib.rs:37` and `areas/access/cpe/CONTEXT.md:355` say legal information is left out and are corrected: authorship and copyright holder are read, access rights and data classification are still left out (owner: left out for now, not for good) |

Every patch brings its tests; none adds a file or a dependency, so no `BUILD.bazel` changes. Commit messages
start from the incubator commits' own, without their attribution lines. Phase 10 stays one pull request of
about 2,500 lines, half of them tests. The incubator half of DEV-7801 (move the pin, empty
`cpe/vendor/PATCHED`) is not in this plan (H5).

Nothing in dsp-repository calls `activate_with_hooks` or `read_snapshot`: `access-server` activates with the
hooks `cpe-store-build` registers, and reads snapshots by path. Their caller is the incubator's host, which
drafts projects against this code. They are ported so that the incubator's copy can equal the original again,
which is DEV-7801's done-criterion; each carries a doc comment naming that use, and tests keep them live.

`activate_with_hooks` names `cpe_store_build::hook::Hook` in its `hooks` parameter. That is an exception to
the rule that no `pub` item of `cpe-server` names a type of another CPE crate but `cpe-ports`'
(`ARCH-MAP.md:1354` and `1444` to `1445`), and the patch extends the facade test to allow exactly that one
parameter (`the_boundary_exception_is_the_hooks_parameter_alone`). `ARCH-MAP.md` names the exception, and says
that a host which supplies hooks depends on `cpe-store-build`; the method stays `#[doc(hidden)]`.

The patch's tests were written when `port_sources` returned a `String` error. On the stack it returns
`NotLoaded` (`activation.rs:46`, `277`), so the two tests that search the error text match
`NotLoaded::Failed(report)` and search the report, and the existing test calls of `port_sources`
(`activation.rs` around lines 764, 869, 1112) pass an empty hook list.

One thing the patch could not decide: re-activation. `follow.rs:88` re-activates a project when a store
announces a change, calling `from_ports` without hooks. A host that activated with its own hooks would lose
them on the first announcement. The activated state `Cpe` already holds and swaps (`Activated`) therefore
keeps the hooks it was built with (`load_all`, which builds it at `activation.rs:213`, takes them; the
test-only SQL-folder loader passes none), and re-activation reads the current state's hooks before it calls
`from_ports`. `Hook` (a name and a function pointer, `cpe-store-build`'s `hook.rs:31`) gains `Clone` and
`Copy` for that. `activate` remains `activate_with_hooks` with none. `Cpe::follow` activates before it starts
following (`lib.rs:153`), so a host with hooks needs a hidden `follow_with_hooks`, which `follow` calls with
none.

The dry run applied each patch in order on the stack's tip. It did not compile the patched trees, so each
phase may need fixes beyond the ones named here; the patch's own tests decide when a phase is right.

### Phases 11 to 13: JavaScript in the Rust code (DEV-7510)

The target, as the owner restated it: no inline `<script>` and no `ExecuteScript`; short expressions stay in
`data-*` attributes; long ones move to a static `cpe.js`; what DataStar cannot express and is short (clipboard,
Escape, clearing a field) stays inline as a listed exception.

Paths below are in `areas/access/cpe/components/src/component/` unless they name a crate.

- **Phase 11**, HTML-identical. `resource_list.rs` (lines 1239, 1241) uses `dom.rs`'s `PICKER_ESCAPE_EXPR` and
  `PICKER_FOCUSOUT_EXPR` instead of its verbatim copies, as `data_links.rs` and `compound_viewer.rs` already
  do. `ANNO_DOT` (`annotation.rs:183`) is a `const PreEscaped<&str>`; a constant cannot hold `html!` output, so
  it becomes a function `anno_dot() -> Markup` rendering `span.anno-dot aria-hidden="true" {}`, and its call
  sites in `compound_viewer.rs` change with it. The bytes are the same, so no snapshot changes. Phase 5's pin
  for `annotation.rs` drops to zero. The one-line `el.open = false` on click-outside stays a literal.
- **Phase 12.** `render_cv_root` (`compound_viewer.rs:337`) renders no `<script>`. What the script does today
  (`READER_URL_INIT_SCRIPT`, line 294; `stale_url_correction`, line 1167): on a full page with a stale
  `?page=` it calls `history.replaceState` to the corrected address, and in every case it seeds
  `window.__cpeReaderUrl` from `location`. Both move into a `data-init` expression on
  `article#compound-viewer`, which already carries the `popstate` handler that reads the seed
  (`POPSTATE_RELOAD_EXPR`, line 264). The corrected address is still escaped with `cpe-core`'s
  `js_string_escape` and then attribute-escaped by Maud. Phase 5's pin for `compound_viewer.rs` drops to zero.

  Two things change in behaviour and are accepted: the correction runs when DataStar initialises instead of at
  parse time, so it needs DataStar and not only JavaScript (without DataStar the `popstate` handler is gone
  too); and `data-init` runs again whenever the attribute's value changes, as when a stale full page is
  morphed to a fragment whose attribute holds only the seed. Re-seeding from `location` is idempotent. The
  `ExecuteScript` in `router.rs` stays the second writer of the seed.

  Unit tests that assert the literal `<script>` (in `compound_viewer.rs`, around lines 2035 and 3033, and in
  `cpe-web`'s `page.rs`, around lines 6494 to 6508, the hostile-address case among them) are rewritten to
  assert the attribute. `cpe-server`'s `router.rs` tests assert the `ExecuteScript`, which stays. The
  `PreEscaped` import `compound_viewer.rs` no longer needs is removed. Snapshots change and are reviewed one
  by one. `book.spec.ts` is the guard for the behaviour.
- **Phase 13.** A first-party `areas/access/cpe/public/cpe.js`, served by `static_files.rs` at `cpe/cpe.js`
  (its `FILES` row; the test compares the table with the folders and needs no other change) and loaded from
  `document_head` in `cpe-web`'s `page.rs` with `defer`, *before* DataStar's module script. Deferred classic
  scripts and module scripts run in document order, so `window.cpe` exists when DataStar runs `data-init`. It
  sits outside `public/vendor/`, so the vendor digest check does not apply (DPE's `telemetry.js` is the
  precedent). Like CPE's other static files it is served unversioned.

  A function in a static file sees neither DataStar's signals nor its actions, and the attribute gate forbids
  a raw `@get(` outside `shared-datastar`. So the functions take `el` or `evt` and return values, and the
  attributes keep the signal writes and the action:
  - `cpe.arrival(el, ids)` holds `ARRIVAL_INIT_EXPR`'s logic (`annotation.rs:61`; hash, referrer, navigation
    type, the scroll) and returns `{arrive, note}`; the attribute, rendered in `resource_detail.rs` (lines 540
    and 548), assigns `$_annoArrive` and `$_annoNote` from it. The element ids the expression interpolates
    today (`REGISTER_ID`, the entry-id prefix) are passed in by the attribute, so they stay defined in Rust.
  - `cpe.pickerToggle(el)` holds `PICKER_TOGGLE_EXPR`'s open-scroll (`compound_viewer.rs:801`).
  - `cpe.pickerTarget(evt, el)` holds `picker_click_expr`'s delegated-click logic (line 813) and returns the
    query to load, or nothing; the attribute binds it to a name and calls the `@get` built by
    `shared-datastar` with that name (`plus_js`) when there is one.
  - `cpe.pageKey(evt, el)` holds `PAGES_KEYS_EXPR`'s arrow-key logic (`pages_keydown`, line 384); the Escape
    unpin from `annotation::esc_unpin_expr` stays prepended in the attribute.

  Rust tests that assert an expression's text (`annotation.rs` around lines 1152 to 1193, with its snapshot;
  `compound_viewer.rs` around lines 3278 to 3301; `resource_detail.rs` around lines 4524 to 4533) assert the
  new attribute instead; what they pinned about the logic itself is covered by `book.spec.ts`.

  A page without JavaScript behaves as before. `book.spec.ts` gains an annotation arrival case (the emphasis
  and the scroll on arriving by fragment; the committed fixture has no link object, so the arrival note is
  not covered) and page picker cases. `cpe.js` is linted by biome, called on that one file with a
  `biome.json` beside it.

The listed exceptions, which stay inline because they are short and DataStar cannot express them: clipboard
copy (`breadcrumb.rs`, `data_links.rs`), Escape, outside-click and focus-out on disclosures (`dom.rs`),
clearing the search field (`search_bar.rs`, `facet_panel.rs`), the header menu and the crop-error marks
(`page.rs`, `dom.rs`, `showcase.rs`), the order control (`resource_list.rs`). Phase 13 records them in
`areas/access/cpe/CONTEXT.md`; Phase 16's component-library page and runbook state the rule.

Not in this plan, by the owner's triage: the viewer controls in `media.rs` (about 25 calls into the image
viewer) and the `ExecuteScript` in `router.rs`, which waits for #582. Module sizes, the six `#[allow]`s and the
`expect`s with invariant messages are not problems. History-describing comments and the references to
dsp-incubator belong to DEV-7416.

### Phase 14: baseline headers (DEV-7499)

`access-server` adds response headers to what DPE and CPE answer, pages and errors alike, in `assemble()`
(`areas/access/server/src/serve.rs:140`): `X-Content-Type-Options: nosniff` and
`Content-Security-Policy: base-uri 'self'; object-src 'none'`, each with tower-http's
`SetResponseHeaderLayer::if_not_present`, so a response that sets its own header keeps it. The two layers go
where the existing layers go, before `/healthz` and `/telemetry/collect` are added, so those two routes stay
as they are. Neither directive
restricts scripts, styles, images or connections, and no page under `areas/` or `modules/` renders a
`<base>`, `<object>` or `<embed>`, so no page can break. `access-server`'s `BUILD.bazel` gains
`@crates//:tower-http` (`MODULE.bazel` already has it with `set-header`; `deposit-server` uses the same layer).
The editor is a different binary and is not touched.

CPE's `static_files.rs` sends the script-blocking policy that `assets.rs` sends on project assets (`ASSET_CSP`,
`assets.rs:19`, made visible within the crate) on the SVG files it serves. The image viewer's assets hold no
SVG and need nothing. DPE's `public/` SVGs, served by `ServeDir`, are left as they are; they are first-party
files, and a policy for them joins the later work.
`Content-Security-Policy-Report-Only` and an enforcing policy are not in this plan (H6).

### Phase 15: the contents rail (DEV-7800)

The rail's rules are in `areas/access/cpe/components/style/record.css` (lines 688 to 704): `.cv-toc` is a grid
of `minmax(12rem, 16rem) 1fr` that becomes a column below 860 px, where the rail moves under the reader by
`order: 1`. Two suspects from the CSS: the plain `1fr` lets wide reader content push the page sideways (the
fix is `minmax(0, 1fr)`), and the intro above the viewer is centred at reading width while the rail's shell
spans the page, so the two drift apart as the window widens.

Incunabula has no page with a contents rail, so the layout is measured on a page built from the snapshot
`cpe_components__component__compound_viewer__tests__snapshot_compound_viewer_pages_rail.snap`, which holds
only the viewer's `article.compound-viewer.page`. A spec `rail-layout.spec.ts` in CPE's suite reads the file,
drops its insta header, and sets as page content what a real page puts around the viewer
(`cpe-web`'s `page.rs`, the body-component path near line 2723): `main.shell__main#main` holding an
`article.editorial.page` with a `div.editorial__intro` paragraph, then the snapshot's article. It links the
seven stylesheets from the running server (`/cpe/_static/cpe/{tokens,shell,editorial,record,grid,media,incunabula}.css`,
in `document_head`'s order) and asserts at 320, 768, 861, 1024, 1180, 1280, 1440 and 2200 px that the document
does not scroll sideways, and at every width that `.editorial__intro` and `.cv-toc` share their left edge. The
spec is written first and fails where the layout breaks; the fix addresses what it shows, which may differ
from the two suspects. If the measurement shows the intro's offset is intended by the grid contract
(`grid.css`), the left-edge assertion is dropped and the pull request says why.

The rail's place below 860 px is not changed. The stylesheet's own comment (lines 689 to 698) records that the
rail is first in the markup and shown last on a narrow screen; that is a design decision, and reversing it
would trade one order mismatch for another on wide screens.

### Phase 16: documentation (DEV-7406)

- `areas/access/cpe/CLAUDE.md`, the agent runbook: adding a project (the project folder, the VRE bridge's
  inputs, a hook in `store-build`), running `access-server validate`, the test suites, and the gotchas from
  dsp-incubator's `cpe/CLAUDE.md` that are still true of this code.
- `docs/src/cpe/architecture.md`: crates and their edges, the ports, activation and a project's statuses,
  the request path.
- `docs/src/cpe/component-library.md`: the stylesheets, the `grid.css` layout contract and the `.page` rule,
  the seam to `mosaic-tiles`, the JavaScript rule with its listed exceptions.
- `docs/src/SUMMARY.md` lists both pages; `eng.yaml` binds the new `CLAUDE.md` to `areas/access/cpe/**`.

## Technical Considerations

- **Base and rebases.** The VRE bridge stack will still change. This stack is rebased onto its tip with
  `gh stack rebase` whenever it moves; a merge commit is never used (the commit gate rejects them). When the
  VRE stack has merged, this stack's trunk becomes `main` (H1).
- **One commit per pull request.** A fix found in review of a phase is amended into that phase's commit, and
  the branches above are rebased. `just commit-lint <parent-branch>` checks each layer.
- **Commit types and scopes** follow `docs/src/git-conventions.md`: eight types, a mandatory scope from the
  crate list (`cpe-server`, `cpe-components`, `cpe-web`, `access-server`, `sync-store`, `ci`, `docs`, …),
  comma-separated when a change spans crates. CI and workflow changes are `chore(ci)`.
- **Bazel.** Every CPE target is private with its consumers listed; `access-server` sees only `cpe-server` and
  `cpe-ports`. No phase adds a visibility grant across areas. Test-only code and data stay `testonly`.
- **Snapshots** run with `INSTA_UPDATE=no`. A changed snapshot is accepted with `cargo insta accept` or by hand
  with its `assertion_line:` header removed, never by renaming the `.snap.new` file
  (`docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`).
- **Recipes that read a build output** pass `--remote_download_toplevel`
  (`docs/learnings/build-errors/remote-download-minimal-leaves-toplevel-outputs-remote-in-ci.md`).
- **No vacuous passes.** A suite or check whose precondition is missing fails; it does not return early
  (`docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`).
- **Tailwind.** CPE's stylesheets are plain CSS; if a phase moves markup that emits utility classes, the
  `@source` globs are checked (`dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`).
- **Overlap with #582.** That pull request rewrites comments in `cpe-server` (`router.rs`, `activation.rs`,
  `discovery.rs`, `lib.rs`). Phases 1 to 3 touch three of those files; they change code, keep comment edits to
  the lines they touch, and whichever lands second rebases.
- **Documents state the present.** Docs and ADRs changed here describe the code as it is after the change; a
  closed gap deletes its *Not yet in code* line.
- **Accessibility.** No phase adds or changes a control or a view. Phases 12 and 13 move behaviour without
  changing markup semantics; Phase 15 changes layout only and leaves the DOM order of the contents rail as it
  is. Phase 6 adds the axe gate.
- **Wave boundary.** Phase 11 starts only when Phases 1 to 10 are pushed as draft pull requests and their CI
  is green.

### Reviewers

Each phase review uses, as the diff warrants: `eng:review:rust-reviewer`, `eng:review:consistency-reviewer`,
`eng:review:dune-reviewer`, `eng:review:code-simplicity-reviewer`, `eng:review:reference-reviewer`,
`eng:review:maud-datastar-reviewer` (components, web, server), `eng:review:security-reviewer` (store, server,
headers, `cpe.js`), `eng:review:ivan-reviewer` and `eng:review:accessibility-reviewer` (components, web,
styles), `eng:review:typescript-reviewer` (the Playwright suite, `cpe.js`), `eng:review:devops-reviewer`
(workflows, `BUILD.bazel` of the server, the justfile).

## Implementation Phases

Every phase is one pull request with one commit, a draft assigned to the owner, its base the phase before it
(Phase 1's base is `feature/dev-8001-access-server-no-stand-in-data`). Its body follows
`.github/PULL_REQUEST_TEMPLATE.md`, opening with `Fixes DEV-NNNN` when the phase completes the issue and
`Part of DEV-NNNN` otherwise.

#### Phase 1: the SQL-folder loader is test-only (DEV-7633)

Commit: `refactor(cpe-server): make the SQL-folder loader test-only`. Fixes DEV-7633.

- [x] `Cpe::from_sql_folders`, `activation::from_sql_folders` and `discover_sql_folders` compile under `#[cfg(test)]` only
- [x] Every helper in `discovery.rs` that the non-test build then reports as dead is gated the same way; no `dead_code` allowance is added
- [x] Every test that used the loader still runs and passes unchanged in what it asserts
- [x] `areas/access/cpe/CONTEXT.md` describes the loader as test support, with no "ends with DEV-7498" wording
- [x] `ARCH-MAP.md` does the same
- [x] The doc comments on the gated items say what they are for now, not who used them
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: an unknown project answers 404 (DEV-7786)

Commit: `feat(cpe-server): answer 404 for a project with no Ready token`. Fixes DEV-7786.

- [x] Test: a project folder with no Ready token answers 404 with CPE's not-found page on its prefix and on a path below it; it would catch the 503 coming back (`a_project_that_does_not_activate_fails_alone`'s `unknown` case in `activation.rs` moves to this)
- [x] Test: that response has the same status, the same headers apart from `Date`, and the same body as a path that names no project folder; no `Retry-After`
- [x] Test: a hidden project that is unknown answers that same 404, with no `X-Robots-Tag`
- [x] Test: a hidden project that is served, and one that failed, still carry `X-Robots-Tag` on every answer
- [x] Test: an unknown project is absent from the overview
- [x] Test: a failed project still answers 503 with `Retry-After` and `Cache-Control: no-store`
- [x] Test: a project whose Ready token is broken or names a missing generation is failed (503), not unknown
- [x] Test: a missing or empty location still marks every project failed, not unknown
- [x] Test: an unknown project that receives a Ready token and an announcement serves afterwards (with `follow.rs`'s fakes)
- [x] Test: a served project whose token is removed answers 404 after the announcement (`follow.rs`'s `a_project_sync_no_longer_knows_…` asserts this instead of 503)
- [x] Unknown ids are no longer pushed into `router::build`'s `unavailable` list
- [x] `hidden_ids` is built from served and failed projects only; its doc comment says so
- [x] The "names no project folder" log does not fire for a hidden shortcode whose folder exists and is unknown
- [x] `areas/access/CONTEXT.md`'s *Hidden project* entry says a hidden project that is unknown answers as any unknown project
- [x] An unknown project is logged with `tracing::info!` and its shortcode; no warning is logged for it
- [x] Code comments that say an unknown project answers 503 (`activation.rs`, `unavailable.rs`, `router.rs`'s fallback) say what it answers now
- [x] ADR-0007's "not ready is kept apart from not there" passage is rewritten in place: a project with no Ready token is not there (404); a project that has data and fails is not ready (503); the commit body records the rule it replaces
- [x] `ARCH-MAP.md`'s "never 404" sentence says the same
- [x] `areas/access/cpe/CONTEXT.md`'s *Unavailable project* entry and its `_Avoid_` line say the same
- [x] `areas/access/CONTEXT.md` defines *unknown project* and *released* in its entry format
- [x] `docs/src/cpe/operations.md`'s status table says 404 for an unknown project
- [x] `docs/src/cpe/operations.md` has a section *Releasing a project on a deployment*
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: `access-server validate` checks CPE's projects, strictly (DEV-7403)

Commit: `feat(cpe-server,access-server): validate CPE's projects strictly`. Part of DEV-7403.

- [x] Test (`cpe-server`): the validation function reports a valid fixture project as served, a broken-configuration fixture as failed with its report as text, and a folder with no Ready token as unknown
- [x] Test (`cpe-server`): a served project's warnings are in its outcome
- [x] Test (`cpe-server`): a folder with a malformed `SHORTCODE` is reported as failed; a folder with none is not reported
- [x] Test (`access-server`, on the function `main` calls, which takes the configuration and ports as arguments): two failed projects fail the run and both reports are printed
- [x] Test (`access-server`): the run passes when the only non-served projects are unknown, and prints a warning for each
- [x] Test (`access-server`): a DPE failure fails the run even when CPE's projects pass, and CPE is still checked
- [x] Test (`access-server`): CPE's projects are checked when `CPE_BASE` is unset
- [x] Test (`access-server`): `serve` with the same broken fixture still starts and takes only that project offline
- [x] `load_all` keeps each project's rendered report and warnings
- [x] `cpe-server`'s `lib.rs` defines the validation function and its outcome type; the type names nothing outside `cpe-server`'s public surface, and `no_pub_item_names_another_cpe_crate` passes
- [x] `dpe_server::validate` returns a result `main.rs` can combine, not an `ExitCode`
- [x] `serve` and `validate` build CPE's configuration through one shared function
- [x] `access-server validate` checks DPE, then CPE, and exits non-zero when either fails
- [x] `areas/access/server/CLAUDE.md` describes the verb as it now is
- [x] The verb's help text in `cli.rs` describes both checks
- [x] `docs/src/dpe/operations.md` describes the verb as checking DPE and CPE
- [x] `docs/src/cpe/operations.md` gains a section on the verb
- [x] ADR-0007 describes the strict mode as the verb's, and its *Not yet in code* sentence keeps only the designed offline page
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: CI validates every committed CPE project (DEV-7403)

Commit: `feat(access-server,cpe-server,ci): validate every committed CPE project in CI`. Fixes DEV-7403.

- [x] Run `just validate-data` on the parent commit; it passes, or what it reports is fixed in this phase
- [x] `NamedNarratives` carries the site's default language
- [x] Test: the data-free check fails on shipped inputs whose configuration does not parse
- [x] Test: it fails when a named narrative has neither its base file nor its default-language variant, and passes when only another language's variant is missing
- [x] Test: it fails when the mapping's shortcode differs from the folder's `SHORTCODE`
- [x] Test: it fails on a project folder whose `SHORTCODE` is not a valid shortcode
- [x] Test: it passes, with a printed note, for a project folder with neither stand-in data nor shipped inputs
- [x] Test: the run fails when a shortcode it was told must be served is unknown or failed
- [x] Test: the run fails when it finds no project folder, and when the inputs directory it was given does not exist
- [x] `access-server validate` takes an optional inputs directory
- [x] `access-server validate` takes the shortcodes that must be served
- [x] `cpe-server`'s validation function runs the data-free check over every project folder when given an inputs directory
- [x] A recipe `validate-cpe *FLAGS` runs the verb from the repository root with `DPE_DATA_DIR`, against the committed fixtures, with `areas/deposit/vre-bridge/inputs` as inputs and `0803` as must-be-served
- [x] `test.yml`'s required job runs `just validate-cpe` with the Bazel flags, after `just test`
- [x] ADR-0002 names the recipe's read of the bridge's shipped inputs where it names the cross-area paths
- [x] `docs/src/cpe/operations.md` names the recipe
- [x] `docs/src/workflows.md` and `docs/src/bazel.md` list the recipe where they list `validate-data`
- [x] After the push, the required job's first run is read (`gh pr checks`); a failure or a timeout there is fixed in this commit
- [x] `CONVENTIONS.md` (*Before Committing*) names the recipe for changes to a CPE project or its shipped inputs
- [x] Run `just validate-cpe`; it passes, with 0803 served
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: a test pins CPE's `PreEscaped` sites (DEV-7404)

Commit: `test(cpe-components): pin the PreEscaped sites`. Part of DEV-7404.

- [x] A test counts `PreEscaped(` call sites per source file of `cpe-components`, each file cut at its `#[cfg(test)]` module, and compares with a pinned table; it would catch an unreviewed new site
- [x] `cpe_components_test` has the crate's sources as `data`
- [x] Each pinned file's comment names the sanctioned input of its sites
- [x] The test's comment says what it does not see (a splice without call syntax; sites in `shared-datastar` and the Mosaic tiles)
- [x] `ARCH-MAP.md`'s note that CPE's `PreEscaped` sites are checked by review names the test
- [x] The commit body states the pinned count per file
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: a Playwright suite for Incunabula and an axe workflow (DEV-7404)

Commit: `test(cpe-web,ci): add CPE's Playwright suite and axe workflow`. Fixes DEV-7404, or `Part of` it when a
`test.fixme` remains.

- [x] `areas/access/cpe/web-e2e-tests/` with `package.json`, lock file, `tsconfig.json` and `biome.json`, as the editor's suite has them
- [x] `playwright.config.ts`: projects `chromium-js` and `chromium-nojs`; `webServer` on port 4102 with `CPE_BASE=/cpe`, ready when `/cpe/incunabula/` answers
- [x] The configuration throws when `ACCESS_SERVER_BIN` is unset or the file is missing
- [x] `overview.spec.ts`
- [x] `browse.spec.ts`
- [x] `book.spec.ts`
- [x] `strips.spec.ts`
- [x] `narrative.spec.ts`
- [x] `accessibility.spec.ts`, ignored by `chromium-nojs`
- [x] Each no-JS failure found is fixed here when small, or marked `test.fixme` with a comment naming the defect and listed in the pull request's *Gotchas*
- [x] Recipe `test-e2e-cpe *FLAGS`
- [x] Recipe `test-a11y-cpe *FLAGS`
- [x] `install-e2e-requirements` and `lint-e2e` include the new directory
- [x] `check.yml`'s `lint-e2e` job includes the new directory
- [x] `.github/workflows/a11y-cpe.yml`, shaped like `a11y-dpe.yml`, runs `just test-e2e-cpe`
- [x] ADR-0004 no longer says CPE has no end-to-end suite, and its *Enforced by* line names CPE's pass
- [x] `docs/src/dpe/testing-strategy.md` describes CPE's suite where it said CPE's pages are covered by crate tests only
- [x] `docs/src/deployment.md` (workflow list), `docs/src/bazel.md` (E2E recipes), `docs/src/cpe/operations.md` (where it says suites start without `CPE_BASE`) and `areas/access/server/CLAUDE.md` (path-filter list) name the new suite, recipes and workflow
- [x] Run `just test-e2e-cpe`; both projects pass
- [x] Run `just test-a11y-cpe`; no violation
- [x] After the push, `a11y-cpe.yml`'s first run on the pull request is read (`gh pr checks`); a failure there is fixed in this commit
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: port, a host supplies a project's hooks (DEV-7801)

Commit: `feat(cpe-server,cpe-store-build): let a host supply a project's hooks`. Part of DEV-7801.

- [x] Incubator commit `521b0916` applied to this tree without its `sync-store` hunks
- [x] `port_sources` and `from_ports` take the hooks, and `hook_of` refuses a duplicate row, fitted by hand to `port_sources`' `NotLoaded` result
- [x] The existing test calls of `port_sources` pass an empty hook list
- [x] The patch's tests that search the error text match `NotLoaded::Failed` and search its report
- [x] `Hook` derives `Clone` and `Copy`
- [x] `load_all` takes the hooks and `Activated` keeps them; the SQL-folder loader passes none
- [x] Re-activation in `follow.rs` reads the current state's hooks first and passes them to `from_ports`
- [x] `Cpe::follow_with_hooks` exists, `#[doc(hidden)]`, and `follow` calls it with none
- [x] Test: a project activated with a host's hook still serves what the hook wrote after an announcement re-activates it (its own setup with `activate_with_hooks`, then `follow.rs`'s `run` fake)
- [x] The patch's three activation tests and its two boundary tests are present and pass
- [x] `Cpe::activate_with_hooks` is `#[doc(hidden)]` and its doc comment says who calls it
- [x] `ARCH-MAP.md` names the one exception to `cpe-server`'s facade rule and what a host that uses it depends on
- [x] `areas/access/cpe/CONTEXT.md` describes a host-supplied hook where it describes hooks
- [x] The pull request lists what of the patch was left out and why
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: port, remodel features beyond Incunabula (DEV-7801)

Commit: `feat(cpe-core,cpe-store-build): read integers, list nodes, rich text and mixed-precision dates`. Part of DEV-7801.

- [x] Incubator commit `c8bdcfe9` applied to this tree, with its tests and its documentation changes
- [x] `docs/src/cpe/config-language.md` describes the new remodel features and keeps the stack's changes to it
- [x] Run `just validate-cpe`; it passes
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 9: port, a hook writes text it composes (DEV-7801)

Commit: `feat(cpe-store-build,sync-store): let a hook write text it composes`. Part of DEV-7801.

- [x] Incubator commit `c7d6c2c8` applied to this tree, its conflict in `sync-store/src/lib.rs` resolved keeping the stack's doc comment and both sets of imports
- [x] `serve` and `read_snapshot` share `map_quads`; `serve` reads the generation's `snapshot.nq` as before
- [x] The patch's `read_snapshot` tests that used `serving` and `write_0804` are rewritten over a reader on a snapshot file; one asserts that a reader serves what `serve` serves for the same file
- [x] The patch's other tests are present and pass
- [x] `read_snapshot`'s doc comment says who calls it
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 10: port, files carry their rights; declared image classes, merged classes, membership (DEV-7801)

Commit: `feat(cpe-ports,sync-store,cpe-core,cpe-store-build): carry file rights, declared image classes, merged classes, membership and ordered links`. Part of DEV-7801.

- [x] Incubator commit `60651cdf` applied to this tree, with its tests
- [x] The stack's test that asserts legal information changes nothing covers rights and classification only
- [x] Test: a representation's authorship and copyright holder are served on its file
- [x] `sync-store/src/vocab.rs`'s header and `sync-store/src/lib.rs`'s list of what the port omits describe what is read now
- [x] `areas/access/cpe/CONTEXT.md`'s port table lists legal information as served
- [x] `areas/access/sync/CONTEXT.md` describes a file's legal information where it describes what `sync` serves
- [x] `cpe-store-build`'s README keeps the stack's changes beside the patch's
- [x] The committed 0803 fixtures still activate, and Incunabula's snapshots are unchanged (its generation's `snapshot.nq` holds no authorship or copyright-holder statement)
- [x] Run `just validate-cpe`; it passes
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 11: shared disclosure expressions, and the annotation dot as markup (DEV-7510)

Commit: `refactor(cpe-components): reuse the disclosure expressions and render the annotation dot as markup`. Part of DEV-7510.

- [x] `resource_list.rs` uses `PICKER_ESCAPE_EXPR` and `PICKER_FOCUSOUT_EXPR`; no verbatim copy of them remains
- [x] `ANNO_DOT` is the function `anno_dot()`, rendered with `html!`; its call sites use it, and `annotation.rs` no longer imports `PreEscaped`
- [x] The pin for `annotation.rs` is zero, and the pin test fails if it is left at one
- [x] No snapshot changes
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 12: the reader's inline script becomes `data-init` (DEV-7510)

Commit: `refactor(cpe-components,cpe-web): seed the reader's URL without an inline script`. Part of DEV-7510.

- [x] `book.spec.ts` covers: Back and Forward after paging, a reload on a paged URL, a stale `?page=` whose address is corrected, and paging after a stale address (the URL follows the page, not the correction)
- [x] Those cases pass on the parent commit's binary before the change; the pull request says so
- [x] `render_cv_root` renders no `<script>`; the seed and the stale-address `replaceState` are a `data-init` expression on `article#compound-viewer`
- [x] The unit tests that asserted the `<script>` assert the attribute, including the hostile-address case
- [x] The pin for `compound_viewer.rs` is zero, and its `PreEscaped` import is removed
- [x] Every changed snapshot is reviewed and differs only by the script's removal and the new attribute
- [x] Run `just test-e2e-cpe`; both projects pass
- [x] Run `eng:test-browser` on a book's reader; no defect reported
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 13: the arrival script and the page picker move to `cpe.js` (DEV-7510)

Commit: `refactor(cpe-components,cpe-web,cpe-server): move the arrival script and the page picker into cpe.js`. Part of DEV-7510.

- [x] `book.spec.ts` covers: arriving on an annotation by its fragment, opening the page picker, choosing a page, and the arrow keys
- [x] Those cases pass on the parent commit's binary before the change; the pull request says so
- [x] `areas/access/cpe/public/cpe.js` defines `window.cpe` with `arrival`, `pickerToggle`, `pickerTarget` and `pageKey`, with no dependency
- [x] `static_files.rs` serves it at `cpe/cpe.js` as `text/javascript`, with its `FILES` row; a test asserts the content type
- [x] `document_head` loads it with `defer`, before DataStar's script
- [x] The arrival attribute (`annotation.rs`, rendered in `resource_detail.rs`) calls `cpe.arrival` with the ids and assigns the two signals; no IIFE remains
- [x] The page picker and page-key attributes in `compound_viewer.rs` call the functions; the `@get` stays in the attribute, built by `shared-datastar`
- [x] The Rust tests that asserted the expressions' text assert the new attributes
- [x] The listed exceptions are recorded in `areas/access/cpe/CONTEXT.md`
- [x] `lint-e2e` lints `cpe.js` with biome
- [x] `check.yml`'s `lint-e2e` job lints `cpe.js`
- [x] Every changed snapshot is reviewed and differs only by the attributes and the script tag
- [x] Run `just test-e2e-cpe`; both projects pass
- [x] Run `eng:test-browser` on a book with annotations and on the page picker; no defect reported
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 14: headers that cannot break a page, and a script-blocking policy on static SVGs (DEV-7499)

Commit: `feat(access-server,cpe-server): send baseline security headers`. Part of DEV-7499.

- [x] Test: a DPE page and a CPE page carry `X-Content-Type-Options: nosniff` and `Content-Security-Policy: base-uri 'self'; object-src 'none'`
- [x] Test: CPE's 404 and a failed project's 503 carry both headers
- [x] Test (through `assemble`): an `_assets` SVG and a `_static` SVG carry the script-blocking policy and not the page policy
- [x] Test (through `assemble`): a non-SVG static file (a stylesheet, `cpe.js`) carries one `nosniff` and the page policy, not the script-blocking one
- [x] `access-server`'s `BUILD.bazel` lists `@crates//:tower-http`
- [x] `assemble()` adds the two headers with `if_not_present`, before `/healthz` and `/telemetry/collect`
- [x] `static_files.rs` sends `ASSET_CSP` on SVG files
- [x] `docs/src/cpe/operations.md` lists the headers
- [x] `docs/src/dpe/operations.md` lists the headers
- [x] Run `just test-e2e-cpe`; both projects pass
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 15: the contents rail lays out at every width (DEV-7800)

Commit: `fix(cpe-components): keep the contents rail and the reader inside the page at every width`. Fixes DEV-7800.

- [x] `rail-layout.spec.ts` builds the page from the `…pages_rail` snapshot, the shell and intro around it, and the served stylesheets, and asserts at the eight widths that the document does not scroll sideways, also with a reader child wider than its column
- [x] It asserts at the eight widths that `.editorial__intro` and `.cv-toc` share their left edge, unless the measurement shows the offset is the grid contract's, which the pull request then states
- [x] The spec fails on the parent commit at the widths that break; the pull request states those widths and the cause
- [x] `record.css` is changed so the spec passes
- [x] No snapshot changes
- [x] Run `just test-e2e-cpe`; both projects pass
- [ ] Run `eng:test-browser` on that page at the eight widths; no defect reported
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 16: CPE's runbook, architecture page and component-library page (DEV-7406)

Commit: `docs(docs): add CPE's runbook, architecture page and component-library page`. Fixes DEV-7406.

- [x] `areas/access/cpe/CLAUDE.md`
- [x] `docs/src/cpe/architecture.md`
- [x] `docs/src/cpe/component-library.md`
- [x] `docs/src/SUMMARY.md` lists both pages in its CPE section
- [x] `eng.yaml` has an override binding `areas/access/cpe/CLAUDE.md` as a convention for `areas/access/cpe/**`, beside CPE's existing overrides
- [x] `uv run <eng plugin>/scripts/eng_yaml.py resolve --files areas/access/cpe/server/src/lib.rs` lists the new convention and exits zero
- [x] `ARCH-MAP.md` names the new files where it lists CPE's documentation
- [x] The runbook and the component-library page state the JavaScript rule and its listed exceptions
- [x] Every `just` recipe, path and symbol the three documents name is checked against the tree at this commit; none is missing
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Merge the VRE bridge stack (#610 to #628) before this stack; this stack is then rebased onto `main` | Balduin | after ship | Merging is the owner's decision |
| H2 | In ops-deploy, give the bridge its settings and an allowlist per environment, with a project on prod's list only once it is released | Balduin | after ship | The agent does not change deployments |
| H3 | Confirm Phase 15's fix on Alice's Wonderland page in the incubator | Balduin | after ship | Only Alice has a page with a contents rail, and the incubator's CPE copy follows a pin move |
| H4 | Check Incunabula by hand on DEV after Phases 12 and 13 deploy | Balduin | after ship | DEV-7510's own done-criterion |
| H5 | Design how the incubator vendors a CPE without Cargo manifests, then move its pin and empty `cpe/vendor/PATCHED` | Balduin | after ship | A design decision in another repository; DEV-7801 stays open until then |
| H6 | Decide where Content-Security-Policy reports go, then plan Report-Only and enforcement | Balduin | after ship | Needs production and an owner's choice; DEV-7499 stays open |
| H7 | Decide on ADR-0013 in dsp-incubator (DEV-7802) | Balduin | after ship | The owner's acceptance |

Left for later work, each already recorded in its Linear issue: the viewer controls and the `ExecuteScript` in
`router.rs` (DEV-7510, after #582), the designed offline page (split from DEV-7403), `query.rs`'s row decoding
with `expect`.

## Acceptance Criteria

- [x] Each phase is a draft pull request with one commit, based on the phase before it, with green CI
- [x] On a deployment where a project has no Ready token, every URL under its prefix answers 404
- [x] `access-server validate` fails on a broken CPE project and CI runs it on every pull request
- [x] CPE's Playwright suite passes with and without JavaScript, and axe reports no violation
- [x] dsp-repository's CPE carries the four incubator changes, with their tests
- [x] No inline `<script>` is rendered by `cpe-components`; the only `ExecuteScript` left is the one in `router.rs`
- [x] DPE's and CPE's pages carry the baseline headers; static SVGs carry a script-blocking policy
- [x] A compound viewer with a contents rail has no horizontal scroll from 320 to 2200 px
- [x] An unknown project's 404 cannot be told from any other 404, hidden or not
- [x] ADR-0007, ADR-0002 and ADR-0004 describe the code as it is after the stack
- [x] `areas/access/cpe/CLAUDE.md`, the architecture page and the component-library page exist and match the code
- [x] Incunabula's pages are unchanged except where a phase states the change

## Dependencies & Risks

- **The VRE bridge stack changes underneath.** Each change means a rebase of up to 16 branches. Conflicts are
  most likely in `activation.rs`, `lib.rs`, `serve.rs` and `sync-store/src/lib.rs` (Phases 1 to 3, 7, 9, 10).
- **Prod behaviour of Phase 2 depends on ops-deploy** (H2). Until the bridge runs with per-environment
  allowlists, prod serves what its data location holds.
- **Phase 4's data-free check is only as strong as the shipped inputs.** A project whose inputs are not in the
  bridge's `inputs/` is not checked at all; the recipe prints that.
- **Phases 7 to 10 fit patches written against an older tree.** They were checked to apply, not to compile
  or pass tests, on this base.
- **Phases 12 and 13 move behaviour that snapshots cannot see.** Phase 6's suite is their guard, which is why
  it comes first; a case is added before each move and must pass on the old code.
- **Phase 15 cannot be seen on a real page here.** The page built from a snapshot stands in; H3 is the
  confirmation. The rail's order on narrow screens stays a recorded deviation from reading order.
- **Phase 2 reverses a clause of ADR-0007** by the owner's decision; the ADR is rewritten in that phase.
- **`cpe.js` is served unversioned**, like CPE's other static files. A browser holding an old copy against new
  attributes is possible until it revalidates; the functions are added, never renamed, within this plan.
- **#582** rewrites comments in files Phases 1 to 3 touch; whichever lands second rebases.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| A rebase onto a changed VRE stack breaks a lower phase | M | M | Rebase bottom-up with `gh stack rebase`; rerun `just test` on each layer that conflicted |
| A ported patch does not compile on this base | M | M | Fit it in its own phase; the patch's own tests decide when it is right |
| The no-JS pass finds many broken paths | M | L | Small ones fixed in Phase 6; large ones `test.fixme` and listed |
| `data-init` re-runs on a fragment patch and rewrites the URL | M | M | `book.spec.ts` covers paging, Back and Forward before the move |
| The Playwright recipes pass locally and fail in CI on a missing build output | M | L | `--remote_download_toplevel`; read the first CI run of `a11y-cpe.yml` |
| The 404 hides a deployment fault as an unreleased project | L | M | A missing or empty location stays `Failed` with 503; the info log names each unknown shortcode |

## Success Metrics

- Sixteen draft pull requests with green CI.
- `just test`, `just check`, `just validate-cpe`, `just test-e2e-cpe` and `just test-a11y-cpe` pass on the top of the stack.
- Baseline for Phase 11: zero changed snapshots. Baseline for Phase 5's pin: the `PreEscaped` count per file at
  Phase 5's commit, lowered by one each in Phases 11 and 12.

## References

- Linear: P-DEV-411; DEV-7633, DEV-7786, DEV-7403, DEV-7404, DEV-7801, DEV-7510, DEV-7499, DEV-7800, DEV-7406
- The VRE bridge stack: dsp-repository#610 to #628; `docs/specs/2026-11-19-datei-integration/` for the stack form
- `docs/src/git-conventions.md`, `CONVENTIONS.md`, `REVIEW.md`, `docs/specs/README.md`
- ADR-0002 (areas), ADR-0003 (roles), ADR-0004 (quality gates), ADR-0007 (CPE), ADR-0008 (`testonly` seam), ADR-0012 (Ready token)
- `areas/access/server/CLAUDE.md`, `areas/access/cpe/CONTEXT.md`, `areas/access/CONTEXT.md`, `ARCH-MAP.md`
- Activation and statuses: `areas/access/cpe/server/src/{activation,discovery,lib,unavailable}.rs`
- The verb: `areas/access/server/src/{cli,main}.rs`
- Headers today: `areas/access/cpe/server/src/assets.rs` (`ASSET_CSP`), `static_files.rs`
- The editor's suite as the model: `areas/deposit/editor/web-e2e-tests/`; the workflow's model: `.github/workflows/a11y-dpe.yml`
- Incubator commits: `521b0916`, `c8bdcfe9`, `c7d6c2c8`, `60651cdf` on dsp-incubator `origin/main`
- Learnings: `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`,
  `docs/learnings/build-errors/remote-download-minimal-leaves-toplevel-outputs-remote-in-ci.md`,
  `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`,
  `dasch-specs/learnings/best-practices/deferred-work-lost-across-phase-handoffs.md`
