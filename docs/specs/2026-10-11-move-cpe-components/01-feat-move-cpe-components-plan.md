---
title: "feat: move CPE's component library and its image viewer into areas/access/cpe"
type: feat
date: 2026-10-11
author: "Balduin Landolt"
status: reviewed(3)
linear: DEV-7401
linear_project: CPE establish production path
repositories:
  - name: dsp-repository
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-repository/.claude/worktrees/DEV-7401
  - name: dsp-incubator
    path: /Users/balduinlandolt/Documents/GitHub/dasch-swiss/dsp-incubator/.claude/worktrees/DEV-7401
---

# feat: move CPE's component library and its image viewer into areas/access/cpe

## Overview

CPE moves into this repository crate by crate (ADR-0007, amendment of 2026-10-02). `cpe-core` is in
review as #470 (DEV-7464). This plan moves the next crate, the component library `cpe-components`, with
its stylesheets, and the image viewer it depends on, as a cut-down crate `cpe-image-viewer`. It adds the
mechanical check of CPE's dependency seam that ADR-0007 names, and ends with the incubator building
against the vendored copies.

The work is two PRs stacked on #470, and one PR in dsp-incubator:

| PR | Repository | Branch | Base | Content |
|----|------------|--------|------|---------|
| A | dsp-repository | `worktree-DEV-7401-image-viewer` | `worktree-DEV-7464` (#470) | this plan, `cpe-image-viewer` |
| B | dsp-repository | `worktree-DEV-7401` | PR A's branch | `cpe-components`, its comments, the seam check |
| C | dsp-incubator | `worktree-DEV-7401` | `main` | the re-vendor |

There is no PRD. The requirement is DEV-7401 and ADR-0007.

## Problem Statement / Motivation

DEV-7465 (store, page assembly, wiring) cannot start before the component library is here, and the
component library cannot build here without the image viewer: `component/media.rs` renders the
`<image-viewer>` element through the incubator's `shared/dasch-image-viewer`, which is not part of CPE
and is not in this repository.

Three things in the ticket and the records do not match what the incubator holds today, and this plan
settles each:

- **The viewer is still an experiment.** The incubator's crate carries three engines (OpenSeadragon in a
  shadow root, OpenSeadragon in light DOM, OpenLayers), a geographic map view and a layered-image view,
  for a comparison that has not ended. ADR-0007 carries the viewer's packaging "as an open question, not
  a decision". CPE uses one engine and six builder methods.
- **The stylesheets are not in the component crate.** ADR-0007 puts "the token CSS and the components"
  in one `cpe-{role}` crate. In the incubator the seven stylesheets are in `cpe/static/cpe/` and the
  binary embeds them.
- **The seam is review-only.** ADR-0007 names a `Cargo.toml` check for the component-library and
  cross-capability rules. #470 deferred it and recorded both rules as **review** in `ARCH-MAP.md`.

**This plan drops the churn rule.** ADR-0007's amendment of 2026-10-02 says "A crate
moves when its churn in the incubator has dropped, not in dependency order". Its reason, as the
amendment of 2026-10-11 quotes it from the incubator's go-live plan: "moving a still-busy crate turns
every prototype change into a cross-repo change". That amendment made one exception, for `cpe-core`, and
said: "The rule stands for the crates that follow. […] Dropping the rule for them would be a changed
decision and a new record (ADR-0006)."

The owner's reasons for dropping the rule (2026-10-11):

- **Churn is planned, not observed.** The rule treats churn as something to wait out. It is the result
  of what the team chooses to work on. The recent churn in CPE's crates was the work that prepares the
  move to production (the crate split, the store built from the port, the remodelling), and with that
  done much less is expected. Measuring past commits says little about the next weeks.
- **The go-live schedule.** Serving Incunabula from this repository is the priority. Waiting for a crate
  to look quiet delays DEV-7465 and the production path by more than the cross-repository changes cost.
- **Vendoring back has proved cheap.** The cost the rule feared is a pin move and an empty
  `just vendor-diff` in the incubator, as `cpe-ports` and `sync-store` already pay at each pin move.

The rule's reason is not denied: a change to a moved crate is still a PR here followed by a pin move
there, and that cost is accepted. After this plan the only crates left are the store and page assembly,
which the amendment of 2026-10-02 moves last in any case. How the record shows the change is decision
14.

## Proposed Solution

Five phases.

1. **dsp-repository, PR A.** `cpe-image-viewer` at `areas/access/cpe/image-viewer`: the incubator's
   viewer with everything but the engine CPE uses removed, and its assets compiled in with
   `include_bytes!`. The same commit carries the new decision record that replaces the churn rule.
2. **dsp-repository, PR B, first commit.** `cpe-components` at `areas/access/cpe/components`: the
   incubator's crate copied whole, its seven stylesheets under `style/`, and four tests repointed to
   fixtures inside the crate.
3. **dsp-repository, PR B, second commit.** The crate's and the stylesheets' comments lose their history
   and plan references. Then, without a commit, the incubator is built and tested against the two moved
   crates.
4. **dsp-repository, PR B, third commit.** `check-cpe-seam.sh`, run by `just check`. The stack is
   submitted.
5. **dsp-incubator, PR C.** After the stack has merged: vendor both crates, delete `cpe/components` and
   the seven stylesheets, and repoint what named them.

### Decisions (owner, 2026-10-11)

1. **Stacked on #470**, with `gh stack`. #470 is open, green and waiting for review.
2. **The churn rule is dropped**, for the reasons in *Problem Statement*. A new record replaces it
   (decision 14).
3. **The viewer is a cut-down, temporary copy inside the capability.** It lives at
   `areas/access/cpe/image-viewer` as `cpe-image-viewer`, not in `shared/` (one consumer, and
   `shared/README.md` asks for a second area) and not inside `cpe-components`. The incubator keeps
   developing `shared/dasch-image-viewer` and chooses the final engine; the result then replaces this
   crate. A viewer fix CPE needs before that lands here and is applied by hand in the incubator.
4. **One engine.** The OpenLayers and light-DOM OpenSeadragon engines, the engine switch, `MapView`,
   `LayeredView`, `Modality` and `Strategy` are removed. The markup keeps `engine="osd-wc"`, as a
   constant, so that every page CPE renders is byte-identical.
5. **Assets are compiled in with `include_bytes!`, not `include_dir`.** ADR-0007 says the workspace has
   no `include_dir`. An enumerated table of the asset files (44 with the licence text) replaces the
   embedded directory; `build.rs` goes with it, because `include_bytes!` registers its files with
   cargo. `asset()`, `head_tag()` and `ASSET_PREFIX` keep their signatures. This is the one place the
   copy is rewritten and not only cut.
6. **The stylesheets move with the components**, as plain files under
   `areas/access/cpe/components/style/`. The crate does not embed or serve them. The three SVG files
   (two logos and the fingerprint `shell.css` uses as a mask) and `datastar.js` stay with the
   incubator's binary until DEV-7465.
7. **A playground is planned, not built here.** A follow-up ticket after DEV-7465 (H7). Until then the
   incubator's running prototype shows the components and the 147 snapshots guard them here.
8. **One plan, two PRs.** `docs/specs/README.md` says one plan is one PR; this plan amends that rule to
   admit a stack (Phase 1).

### Decisions (this plan; the owner may overrule at review)

9. **No test leaves `cpe-components`.** Four tests read incubator files outside the crate. Unlike
   #470's three, they call private items (`page_picker::group`, a test-only `render`), so they cannot be
   written again in an incubator crate. Each keeps its name and its assertions and reads a fixture
   inside the crate:
   - `editorial::tests::snapshot_editorial_alice_en`, `…_alice_de` and
     `…_incunabula_home_de` `include_str!` copies of the three narrative files, taken at `SRC`. Their
     snapshots do not change.
   - `page_picker::tests::every_page_of_every_incunabula_book_appears_exactly_once` reads Incunabula's
     5.6 MB `data.sql` through `rusqlite`. It reads instead a tab-separated extract of the one query it
     runs (book, member id, member name, in the query's order). `rusqlite` leaves the dev-dependencies.

   The fixtures are copies frozen at `SRC`. The tests stop noticing a change to the incubator's project
   data; they still check the grouping and the rendering on real input.
10. **Three more builders leave the viewer**: `Viewer::region`, `Viewer::rotation` and
    `Viewer::tile_transition`. Their documentation says the `osd-wc` engine ignores them, and their
    attributes are read only by the engine dispatch that decision 4 removes. `Viewer::osd_wc` goes with
    the private `Engine` enum. `navigator`, `adjust`, `sequence`, `auth`, `label`, `fallback`, `chrome`
    and `page` stay, whether CPE calls them or not.
11. **The inline OpenSeadragon code is not edited beyond the engine dispatch.** Its layered-image mode
    (the `layers` attribute, about 400 lines) is implemented inline and stays, although no Rust builder
    emits `layers` any more. Cutting it means hand-editing code that no test here can run in a browser.
    The README names it as unused.
12. **The viewer's comments are not cleaned in a commit of their own.** The crate is temporary. The copy
    commit rewords only what names a removed thing or an incubator path.
13. **Commit types.** `feat(cpe-image-viewer)` and `feat(cpe-components)`: the workspace gains a crate
    each time. `docs(cpe-components)` for the comments. `chore(ci)` for the seam check. With the plan
    commit, PR A has two commits and PR B three; both tick `allow-many-commits`. The commits are kept
    apart because each has its own proof (*Verification design*).
14. **The churn rule is replaced by a new record** (owner, 2026-10-11, at plan review). ADR-0006 says "A
    decision that changes rather than narrows becomes a new ADR", and #470's amendment names this very
    case as one. PR A adds a root ADR, with the next free number in `docs/adr/` (0011 on 2026-10-11),
    `status: accepted`, titled "CPE's crates move in dependency order". It names the sentence it
    replaces (ADR-0007, amendment of 2026-10-02: "A crate moves when its churn in the incubator has
    dropped, not in dependency order"), restates that sentence's reason, gives the owner's three
    reasons for dropping it as *Problem Statement* words them, and decides: a CPE crate moves when the
    next piece of work needs it here, in dependency order; the store and page assembly still move
    last, with the wiring; every move still ends with the incubator vendoring the crate back and an
    empty `just vendor-diff`. It accepts the cost the old rule avoided, and is enforced by review.
    ADR-0007 is not superseded as a whole: it gets "Amendment (2026-10-11, DEV-7401)", which says that
    the churn sentence of the amendment of 2026-10-02 is replaced by the new record.

    That amendment carries the rest, written in two steps. PR A adds the viewer's interim packaging,
    and that ADR-0007's retirement of "bundles embedded at build time" is about project bundles and
    stands: the viewer's asset files are compiled into its crate, and no project content is. PR B adds
    the stylesheets' location and the seam as enforced.
15. **The seam check enforces more than ADR-0007's sentence.** The record says CPE's *web* crate never
    depends on `mosaic-tiles` and nothing outside the capability depends on CPE's *library*. The check
    covers every crate of the capability (*The seam check*). The amendment states the rule as checked.
    `cpe-image-viewer` gets no exemption for a later DPE consumer: that consumer's PR changes the check
    and the record together.
16. **Incubator ADR numbers are cited by file name**, as in #470: 26 comment lines in the Rust source
    of `cpe-components` (the incubator's 0007, 0008, 0011, 0012, 0014; two of those numbers name an
    unrelated record here), seven comment lines in its stylesheets (0007, 0008, 0012) and four sites in
    the viewer (0010). None is in a string literal or a CSS rule. `check-adr-refs.sh` reads every
    tracked text file, so the seven stylesheet lines are the one change Phase 2 makes to the CSS.
17. **`[lints.rust] unsafe_code = "forbid"` stays in the viewer's manifest.** It is the first `[lints]`
    table in the workspace. It is the crate's own and stricter than the workspace, not an exception.

### `cpe-image-viewer`

`areas/access/cpe/image-viewer/Cargo.toml`:

```toml
[package]
name = "cpe-image-viewer"
version.workspace = true
edition = "2021"
publish = false

[dependencies]
maud = { workspace = true }

[lints.rust]
unsafe_code = "forbid"
```

Files, from `shared/dasch-image-viewer` at `SRC`:

| Path | Fate |
|------|------|
| `src/lib.rs` | copied, edited (table below) |
| `tests/render.rs` | copied, edited (table below) |
| `assets/image-viewer.js` | copied, edited (table below) |
| `assets/vendor/openseadragon.min.js`, `assets/vendor/images/*.png` (40), `assets/vendor/VENDOR.lock` | copied byte for byte |
| `scripts/vendor-osd.sh` | copied; the ADR citation and the path it prints are corrected |
| `README.md` | written new (below) |
| `assets/LICENSE-openseadragon.txt` | new: OpenSeadragon's licence text at the vendored version. Outside `assets/vendor/`, which `vendor-osd.sh` hashes into `VENDOR.lock` |
| `assets/engines/`, `assets/geo/`, `assets/vendor/openlayers/`, `vendor-src/`, `scripts/vendor-openlayers.sh`, `docs/`, `build.rs`, `Cargo.lock`, `rust-toolchain.toml`, `justfile` | not copied |

Edits to the copied files (line numbers at `1cd5781f`):

| File | Edit |
|------|------|
| `src/lib.rs:38`, `:41-43` | `include_dir` import and `static ASSETS: Dir` replaced by a `static ASSETS: &[(&str, &[u8])]` table, one `include_bytes!("../assets/<path>")` row per file under `assets/`, the path relative to the source file (`:39`, the `maud` import, stays; rustfmt wraps the rows, so this hunk is large in the baseline diff) |
| `src/lib.rs:70-82` | `asset()` finds the path in the table with a linear search; the segment check and the signature are unchanged |
| `src/lib.rs:84-94` | `content_type_for` loses the `css`, `map`, `json` and `geojson` arms and gains `txt` and `lock` as `text/plain; charset=utf-8` |
| `src/lib.rs:131-192` | `Engine` and `Engine::as_attr` deleted, doc comment included |
| `src/lib.rs:194-230` | `Strategy` and `Strategy::as_attr` deleted |
| `src/lib.rs:232-492` | `Viewer` loses the fields `engine`, `strategy`, `region`, `rotation`, `tile_transition` (`:243-246`, `:253`, each with its doc comment), their five initialisers in `Viewer::new` (`:277-280`, `:285`; `:281-284` stay) and the builders `osd_wc`, `osd`, `openlayers`, `region`, `rotation`, `tile_transition`; the type's doc comment (`:232-238`) no longer links to `Viewer::osd_wc`; `adjust`'s doc comment loses its per-engine bullets and its reference to `Viewer::osd`; `chrome`'s loses the paragraph that names `osd_wc`/`osd` |
| `src/lib.rs:494-531` | `render()` emits the literal `engine="osd-wc"`; the `strategy`, `tile-transition`, `region` and `rotation` attributes and the locals `strategy_attr` and `rotation_str` go |
| `src/lib.rs:533-803`, `:845-870` | `MapView`, `Modality`, `LayeredView`, their `Render` impls and `json_layers_array` deleted; `push_json_string`'s doc comment (`:828`) no longer names it |
| `src/lib.rs` (docs) | crate name in the two surviving doc tests; the two `cpe ADR-0010` citations; a citation of a README section names a heading the new README has |
| `assets/image-viewer.js:1273-1277` | the engine gate deleted whole: `_init` no longer reads the `engine` attribute. The declaration `const BUILTIN_ENGINE = "osd-wc"` (`:178`) stays although nothing reads it, because a surviving test pins that text; the builtin path already passes the literal `"osd-wc"` to `_emitRendered` |
| `assets/image-viewer.js:1292-1360` | `_mountEngine` deleted |
| `assets/image-viewer.js:1881-1889` | `_failLight` deleted |
| `assets/image-viewer.js:473`, `:481-488`, `:686-695` | `_engineHandle`, the `map` getter and the handle teardown deleted |
| `assets/image-viewer.js` (comments) | "Adding an engine" (`:138-177`) deleted; comments that name OpenLayers, `engines/`, `LayeredView`, `_mountEngine`, a non-builtin engine, a removed builder, `include_dir`, `build.rs`, the incubator's showcase or its recipes reworded (`:7`, `:11-17`, `:32-36`, `:78`, `:111-114`, `:228`, `:259-260`, `:367`, `:424`, `:468`, `:492`, `:549`, `:1118`, `:1202`, `:1222`, `:1257-1260`, `:1265-1272`, `:1446`, `:1515`, `:1554-1578`, `:1671`). The comment at `:549` is inside a template literal: its new text holds no backtick. The word `APPROXIMATION` stays between `_applyFilter() {` and `_buildAdjustPanel`, where a surviving test looks for it; the `cpe ADR-0010` citation at `:9` |
| `tests/render.rs` | the 20 tests of removed engines and types deleted (`:128`, `:304`, and everything from `:457` to the end of the file), with the `LayeredView`/`MapView` import and the helpers `render_map` and `render_layered`; the two tests that call `.osd_wc()` (`:67`, `:77`) deleted; the `_applyFilter` test (`:280`) no longer requires the string `openlayers.js`, and its comment (`:281-285`) and assertion message (`:299`) no longer name OpenLayers; the comments that count or name deleted things corrected (`:52-56`, `:94-97`, and the one in the exposure-and-gamma test); crate name |
| `tests/render.rs` (new) | one test: `asset()` returns every file found under `assets/`, dotfiles skipped (the table is private; a row for a file that is not there does not compile) |
| `scripts/vendor-osd.sh:6`, `:9` | the recipe name becomes the script's own path; the `docs/adr/0010` pointer becomes the file-name citation; a comment tells the reader to add an `ASSETS` row for a new file. The lines that write `VENDOR.lock`'s header (`:45-48`) are not touched: a regenerated lock must equal the copied one |

The JS keeps `engine` and `layers` in `observedAttributes`. With the gate gone, the `engine` attribute is
not read: whatever its value, the element renders OpenSeadragon. No Rust builder can emit another value,
and the README says so. `tests/render.rs` pins JS text (`BUILTIN_ENGINE = "osd-wc"`, the public method names,
the adjust-panel constants, the absence of `page` and `chrome` from `observedAttributes`); those tests
stay and pass.

`README.md`, written new, says: what the crate is; that it is a cut-down copy of dsp-incubator's
`shared/dasch-image-viewer` at `SRC` (full sha), made for CPE until the incubator has chosen the final
engine, and that the result replaces this crate; what was removed; that the `layers` mode in the JS has
no Rust builder here; that a fix lands here first and is applied by hand in the incubator; the public
API (from the source README's sections "What's in the box", "Why a web component", "Consuming it from a
Rust / Maud host", "Public JS control surface", "Overlay layer" and "Auth", without the engine sections
and without the incubator's manifest snippet; the code cites "Auth", "Overlay layer", "Public JS control
surface" and "Editing the assets" by name, so those headings exist); "Editing the assets", rewritten
for `include_bytes!` and the `ASSETS` table; "Testing", with this repository's commands; how a host
serves the assets, and that `ASSET_PREFIX` is a default a host replaces, since ADR-0007 serves the
engine's files under each project's prefix; how
OpenSeadragon is updated (`scripts/vendor-osd.sh`, then a new row in `ASSETS` for a new file); the
licence OpenSeadragon and its sprites are under, by the name the licence file gives; that the vendored
files are checked against `VENDOR.lock` by hand and not by `just verify-checksums`; that an `engine`
value other than `osd-wc` renders OpenSeadragon; and the rendering notes, with the
engine-comparison brief cited as `dsp-incubator shared/dasch-image-viewer/docs/engine-comparison-spike.md`.

### `cpe-components`

`areas/access/cpe/components/Cargo.toml`:

```toml
[package]
name = "cpe-components"
version.workspace = true
edition = "2021"
publish = false

[dependencies]
cpe-core = { path = "../core" }
cpe-image-viewer = { path = "../image-viewer" }
maud = { workspace = true }
pulldown-cmark = { workspace = true }
# The timeline skips a Resource with an unparsable date with a warning. This crate installs no
# subscriber; the host's does.
tracing = { workspace = true }

[dev-dependencies]
insta = { workspace = true }
```

Path dependencies between the capability's crates follow `sync-store`'s on `cpe-ports`. The workspace
gains no third-party dependency: `maud`, `pulldown-cmark`, `tracing` and `insta` are in
`[workspace.dependencies]`, and `Cargo.lock` gains two package entries and nothing else.

Layout:

```txt
areas/access/cpe/components/
├── Cargo.toml
├── src/                       # cpe/components/src at SRC: 26 .rs files, 147 .snap files
│   └── component/testdata/    # new: the four fixtures of decision 9
└── style/                     # cpe/static/cpe/*.css at SRC: tokens, shell, editorial, record, grid,
                               # media, incunabula
```

Edits Phase 2 makes to the copied source (line numbers at `1cd5781f`):

| Edit | Where |
|------|-------|
| `dasch_image_viewer::` becomes `cpe_image_viewer::` | `component/media.rs:690`, `:694`, `:725`, `:727`, and the prose at `:11`, `:37`, `:556` |
| Three `include_str!` paths point at `testdata/` | `component/editorial.rs:2441`, `:2451`, `:2463`, with the three comments that say "the real" file |
| `incunabula_books()` reads `testdata/incunabula-book-members.tsv`; its doc comment says what the file is | `component/page_picker.rs:699-747` |
| The 26 comment lines with a bare incubator ADR number become file-name citations; two of them are relative links to `docs/adr/` | `rg 'ADR-[0-9]{4}\|docs/adr/' src` |
| The seven stylesheet comment lines with a bare incubator ADR number become file-name citations | `style/media.css:8`, `:180`, `:198`, `:442`, `style/grid.css:189`, `style/record.css:680`, `:789` |
| A comment that quotes a pre-RC.6 Datastar attribute with its `=` is reworded so that `check-datastar-delimiters.sh` does not match it | `component/media.rs:258` |
| Rewrites that `maudfmt` or clippy force (*Formatting*), each named in the commit body | certain: `component/resource_detail.rs:670`; likely: `component/compound_viewer.rs:1166`, `:1424`, `component/annotation.rs:556`; the rest found in Phase 2 |

Incubator ADR file names: `0007-fragment-swaps-are-transient.md`,
`0008-images-are-embedded-only-resources.md`, `0010-openseadragon-integration.md`,
`0011-per-project-components.md`, `0012-one-member-navigator-declared-layout.md`,
`0014-annotations-are-a-first-class-engine-store.md`. A citation reads
`dsp-incubator cpe/docs/adr/<file name>`.

`cpe-core`, already here, cites the stylesheets as `dsp-incubator cpe/static/cpe/<file>` (16 lines) and
the component crate as `dsp-incubator cpe/components` (`labels.rs:11`, `lang.rs:61`, `util.rs:268`,
`validate/project_components.rs:30`). Phase 2 repoints the first to
`areas/access/cpe/components/style/<file>` and the second to `cpe-components`. Six of the stylesheet
citations are broken across two lines (`iiif.rs:46`, `:57`, `:208`, `:220`, `theme.rs:234`,
`validate/header_budget.rs:5`: `dsp-incubator` ends one line and the path starts the next), so a
one-line search misses them, and the longer path makes rustfmt re-wrap those comments. The edit touches
comments only, which the Phase 3 normaliser proves for `areas/access/cpe/core` too.

### The seam check

`.github/scripts/check-cpe-seam.sh`, with `.github/scripts/check-cpe-seam.test.sh`, a recipe
`check-cpe-seam` and an entry in `just check`'s dependency list and in `just test`'s script list. It
follows `check-composition-root-deps.sh`: bash, git and awk; manifests from `git ls-files`; crate names
derived from `[package]` and never listed by hand; absence is an error; safe to `source`.

Three rules, each over tracked `Cargo.toml` files that have a `[package]` table (the root manifest has
none and is read for one form only, below):

1. **Nothing outside the capability depends on a CPE crate, the port excepted.** The forbidden names are
   the package names of `areas/access/cpe/*/Cargo.toml`, less the crate at `areas/access/cpe/ports`
   (`sync-store` implements the port). No manifest outside `areas/access/cpe/` names one as a
   dependency.
2. **No CPE crate depends on Mosaic.** The forbidden names are the package names of
   `modules/mosaic/*/Cargo.toml`. No manifest under `areas/access/cpe/` names one, a manifest nested
   below a crate (a fuzz crate) included.
3. **The leaves stay leaves.** The manifests at `areas/access/cpe/core`, `areas/access/cpe/ports` and
   `areas/access/cpe/image-viewer` name no package of the capability, the port included: `cpe-core`
   and `cpe-ports` do not depend on each other. A leaf's own `[package] name` line is not a
   dependency and does not match.

The script has its own matcher. `check-composition-root-deps.sh`'s `depends_on` anchors a table header
as `[dependencies.name]` only, and so misses `[dev-dependencies.name]`, `[build-dependencies.name]`, a
target-specific table and a dotted key (`name.path = …`); its `package = "name"` branch is unanchored
and matches a comment. Here a dependency on `name`, in its exact spelling, is one of:

- a line that, after optional indentation and an optional quote, starts with `name`, then an optional
  closing quote, optional spaces and `=` or `.`;
- a table header `[…dependencies.name]`, the name optionally quoted, a trailing comment allowed;
- a line whose `package` key is not preceded by an identifier character or a `#`, and whose value is
  `"name"` or `'name'`.

That covers every dependency table, development, build and target-specific ones included, the dotted
key, and a renamed dependency. The underscore spelling (`cpe_core`) is not matched and need not be:
cargo does not resolve it. A bare key of that name in a `[features]` or metadata table is flagged too,
which is accepted. A crate's package name is read from the `name` line inside `[package]` only; `[lib]`
and `[[bin]]` have a `name` as well.

A git pathspec's `*` crosses `/`, so the crate lists are pinned by depth as the template pins its own:
`areas/access/cpe/*/Cargo.toml` with five path segments, `modules/mosaic/*/Cargo.toml` with four. The
port is excepted by its exact path, `areas/access/cpe/ports/Cargo.toml`, and the three leaves are
listed by exact path. A path in either list that is not tracked is an error: otherwise renaming a
leaf's directory would drop it from rule 3 while the gate still passed.

A manifest without a `[package]` table, the root `Cargo.toml`, is read for the `package = "name"` form
alone, against rule 1's names. A plain `cpe-core = { path = … }` entry in `[workspace.dependencies]` is
not a dependency, and the member that inherits it is caught by its own `cpe-core.workspace = true`
line. A renamed entry (`x = { package = "cpe-core", … }`) is different: the member's `x.workspace =
true` names no CPE crate, so the rename has to be caught where it is declared.

A violation is printed as the manifest, the dependency and the rule, in the template's shape; the
guidance beneath it refers to the exception list, not to `cpe-ports` by name, so that it stays true
when DEV-7465 extends the list. The script carries a comment that its matcher is a deliberate twin of
`check-composition-root-deps.sh`'s with wider coverage, and that a change to one is not a change to the
other. Under `set -euo pipefail`, a loop body ends in an `if`, never in `x && continue`.

This design was built and run in a scratch directory during plan review (2026-10-11): 44 assertions
passed under bash 3.2 and bash 5.3, each with BSD and with GNU `grep` and `awk`, and the gate passed on
a copy of this repository's manifests with the two new crates added. The matcher above is the one that
run ended with; the wording it started from missed quoted and indented keys and flagged
`subpackage = "…"`.

The exception list in rule 1 is one list of paths in the script. DEV-7465 adds the host-facing crates
there when `access-server` mounts CPE (`ARCH-MAP.md` expects `cpe-server` and, for an adapter,
`cpe-store`); the script's header says so. A CPE playground (H7) would live under `areas/access/cpe/`
and, by rule 2, could not use Mosaic's tiles; that ticket has to say which.

Test cases, one fixture repository each: a DPE crate depending on `cpe-components` fails; on
`cpe-core` fails; `sync-store` on `cpe-ports` passes; a CPE crate on `mosaic-tiles` fails;
`cpe-core` on `cpe-components` fails; `cpe-components` on `cpe-core` passes; the `.workspace = true`
form, the dotted key, a quoted and an indented key, the `[dev-dependencies.name]` and target-specific
headers and the `package =` rename are caught; a rename declared in the root's
`[workspace.dependencies]` is caught; a commented `package = "…"` line, `subpackage = "…"`, the
underscore spelling and a longer name with the same prefix do not match; a `[[bin]]` table placed
before `[package]` does not change the crate's name; a manifest nested below a crate
(`areas/access/cpe/components/fuzz/Cargo.toml`) is not taken for a CPE crate, and may depend on CPE
crates but not on Mosaic; a leaf or exception path that is not tracked is an error; a repository without
`areas/access/cpe/`, or without `modules/mosaic/`, is an error.

### Comment rules (Phase 3)

The scope is history and references: the *Delete* rule of `CONVENTIONS.md` → *Comments*. It is not a
general trim. The rules apply to the Rust comments of `cpe-components` and to the CSS comments under
`style/`, in this order; the first that fits a sentence decides.

1. **An ADR citation stays**, in the form Phase 2 gave it, even when the ids beside it go.
2. **A pointer to a file that defines a rule the code enforces stays.** A stylesheet is
   `style/<file>`; a file of `cpe-core` is an item path (`cpe_core::…`) or
   `areas/access/cpe/core/src/<file>`; a file of this crate is an item path or `src/<file>`; the
   config-language specification and the schema are `dsp-incubator cpe/docs/CONFIG_LANGUAGE.md` and
   `dsp-incubator cpe/docs/SCHEMA.md`.
3. **A pointer to a plan, a journal, a backlog item or a design note is deleted** (`docs/plans/**`,
   `docs/design/**`, `DESIGN.md`, `BACKLOG.md`, `docs/cpe-poc.md`). Where a comment in `chrome.rs` says
   a translation awaits review in `docs/plans/<name>/translations-to-review.md`, the pointer goes and
   the statement that the translation is unreviewed stays.
4. **A parenthetical made only of a project name, slice, step or decision ids is deleted whole.**
5. **A paragraph that only narrates who called, removed, reviewed or rejected something is deleted
   whole.**
6. **History that justifies a present invariant is rewritten in the present tense**, not deleted.
7. **A crate that stays in the incubator is named by its path**: `dsp-incubator cpe/engine`,
   `cpe/web`, `cpe/store`, `cpe/bin`. The incubator's guard is named as `just cpe crate-deps-check` in
   dsp-incubator.
8. **A project name stays where the prose uses it as an example or it identifies a test's input.**
9. **Nothing else changes.** No code, no string literal, no test name, no CSS rule. A comment that is
   long, or that restates the code, stays.

**A comment is re-wrapped by rustfmt, never by a helper.** In #470 a helper that tidied the text after a
deletion removed the space inside two CSS descendant selectors quoted in comments. This crate's comments
quote selectors throughout. An edit removes or rewrites whole sentences and leaves every other character
of the comment as it was.

A rough search of the Rust source finds about 300 comment lines with a slice, step or plan reference in
7,559 comment lines, and 82 such lines in the seven stylesheets.

### Verification design

Each code commit has a proof a reviewer can run again.

- **Test lists.** For a crate, `cargo test -p <crate> -- --list --format terse | grep ': test$' |
  sed 's/ (line [0-9]*)//' | LC_ALL=C sort`, run for unit, integration and doc tests alike. The `sed`
  drops the line number a doc test's name carries, which moves with every edit above it. `just test`
  runs `cargo test --tests` and so never runs a doc test; the viewer's two run only where a checkbox
  names `cargo test -p cpe-image-viewer`.
- **Phase 1, the viewer.**
  - `diff` of each of the 42 files under `assets/vendor/` against the source is empty, and the SHA-256
    of each of the 41 OpenSeadragon files is the one `VENDOR.lock` holds.
  - `diff` between the source `assets/image-viewer.js` and the copy prints no added line (`>`) except
    the reworded comments the edit table names; every removed line (`<`) is in a range the table
    names, or a blank line beside one. `node --check` accepts the file.
  - For `src/lib.rs` and `tests/render.rs`, the *formatted baseline* is the source file formatted by
    this repository's `just fmt` and nothing else. `diff` between the baseline and the committed file
    shows the table's edits and rustfmt's re-wrap around them.
  - The test list is the source's less the 22 deleted tests and the two doc tests of removed types,
    plus the one new test.
  - The markup is unchanged. A throwaway binary crate under an absolute scratch path, outside both
    repositories, depends by path on the source crate in the incubator worktree and on
    `cpe-image-viewer`, renders the same `Viewer` with both for each remaining builder alone and for
    all of them together (`navigator`, `adjust`, `sequence`, `auth`, `label`, `fallback`, `chrome`,
    `page`, one and several sources), and asserts the two strings are equal. In Phase 2, the twelve
    snapshots of `cpe-components` that contain `<image-viewer` pass without a change.
- **Phase 2, the components.**
  - `diff -r` between the source's two `snapshots/` directories and the copies is empty. `diff -r`
    between `cpe/static/cpe/*.css` and `style/` shows the seven comment lines of decision 16 and nothing
    else. No `.snap.new` exists after the tests ran. A snapshot is never accepted, renamed or edited in
    this plan.
  - The test list equals the source's, name for name: 980 tests.
  - The formatted baseline is the source's `src` formatted by `just fmt` until a run changes nothing.
    `diff -r` (snapshots and `testdata/` apart) between it and the Phase 2 commit shows the edit
    table's rows and the rustfmt re-wraps around them. Every other hunk is a failure.
  - The tests are run with `INSTA_UPDATE=no`, under which insta writes no file.
  - `cpe-core` changed in comments only: the Phase 3 normaliser, applied to every file under
    `areas/access/cpe/core/src` before and after, prints no difference.
  - Each fixture is the source file (`diff` empty), or, for the extract, the output of the test's own
    query over `data.sql` at `SRC`; the commit body gives the command.
- **Phase 3, the comments.** Between the Phase 2 commit and the Phase 3 commit:
  1. Every `.rs` file is equal once comments are removed by
     `grep -v '^[[:space:]]*//' | sed 's,[[:space:]]*//.*$,,' | cat -s`. A difference it prints is
     rustfmt re-joining a line after a trailing comment went, or a real code change; each is read.
  2. In `git diff -U0`, each added or removed Rust line that is not a whole-line comment and contains a
     `"` is read, because the normaliser also cuts after `//` inside a string literal.
  3. Every `.css` file is equal once comments are removed by `perl -0pe 's,/\*.*?\*/,,gs' | cat -s`
     and blank-only lines are dropped.
  4. The snapshots and the test list are unchanged.

  The source has no Rust block comment; its 61 `#[doc(hidden)]` are attributes, not documentation text.
- **The incubator against the moved crates, before anything merges.** At the end of Phase 3 the
  incubator's worktree is pointed at this worktree's three crates by path, uncommitted, and its test
  suite is run (*The uncommitted check in the incubator*). It is the only run of the incubator's page
  and router tests against the cut-down viewer before PR A merges, and the state H2 looks at in a
  browser.

## Alternative Approaches Considered

- **Move the viewer whole, into `shared/`.** Rejected by the owner: the second engine and the engine
  switch are an unfinished experiment, and the crate has one consumer.
- **Keep `include_dir` and amend ADR-0007.** The smallest change to the copy. Rejected by the owner: the
  record says the workspace has none.
- **Serve the viewer's assets from a `public/vendor/` directory**, as DPE and the editor do, under
  `just verify-checksums`. Rejected by the owner: it removes `asset()`, which the incubator's binary
  calls, for a crate that is to be replaced.
- **Leave the stylesheets in the incubator until DEV-7465.** A simpler re-vendor. Rejected by the owner:
  a component and its CSS would change in two repositories.
- **Delete the four tests and write them again in the incubator**, as #470 did with three. Rejected:
  they call private items (decision 9).
- **Copy `data.sql` and `schema.sql` as fixtures and keep `rusqlite`.** Keeps the test body unchanged.
  Rejected: 5.6 MB of another crate's data in a component crate, for one query.
- **Two plans, one per PR.** What `docs/specs/README.md` asks. Rejected by the owner (decision 8).
- **A first commit that is the verbatim copy**, **one squashed commit per PR**, **importing history**.
  Rejected for the reasons #470's plan gives.

## Technical Considerations

**The stack.** `origin/worktree-DEV-7464` is #470's head (`1a503920` on 2026-10-11), and this branch was
fast-forwarded onto it. The local branch `worktree-DEV-7464`, checked out in its own locked worktree,
was three commits ahead of and five behind its upstream that day: it is the copy from before a rebase.
`gh stack init` adopts local branches, one checked out in another worktree included, so H1 brings it in
line before the run. The executor works in this worktree only, creates PR A's branch from the current
`HEAD`, and fast-forwards `worktree-DEV-7401` onto it when Phase 1 is done. It uses the `gh-stack`
skill; `gh stack` is called without prompts (`init <bottom> … <top>`, `view --json`, `submit --auto`).
`submit --auto` creates draft PRs with generated titles and bodies and sets no assignee; the body and
the assignee are set afterwards with `gh pr edit`.

**When #470 changes.** A review fix to #470 is a force-push of its branch. `gh stack sync` or
`gh stack rebase` then replays PR A and PR B. Both update each branch in the worktree that owns it and
stop when that worktree is not clean, the DEV-7464 worktree included (H1). The proofs of *Verification
design* and the Fingerprint are run again after any rebase that touches a commit of this plan. The same
holds after each merge of H3: the branches above are replayed onto `main`, and a PR's commit count is
only its own layer's once that has happened.

**The source commit.** `SRC` is the sha the incubator worktree is created at: the incubator's
`origin/main` when Phase 1 starts. On 2026-10-11 that is `1cd5781f987ed860218462c086c77ce759185fcc`, and
every source line number and count in this plan refers to it. Where the source changed since, the lines
are found again by content and the counts in *Success Metrics* are taken again. Both code commits carry
`SRC` as a full sha in their body.

**The incubator may have vendored `cpe-core` by then.** #470's plan ends with an incubator PR that
deletes `cpe/core`. If it has merged before Phase 1, `SRC` has no `cpe/core`; nothing in Phases 1 to 3
reads it except the uncommitted check, which then redirects the vendored `cpe-core` in the same way.

**Drift.** Nothing freezes `cpe/components`, `cpe/static/cpe/` or `shared/dasch-image-viewer` between
`SRC` and Phase 5. Phase 5 checks the first two and stops on a change other than the comment repoints
of DEV-7464's incubator PR (H6 keeps the window empty). The
viewer is different: the incubator's copy goes on changing by design, and Phase 5 only reports what
changed there since `SRC` in PR C's description.

**Formatting.** The incubator formats with stable rustfmt defaults. This repository runs `maudfmt` and
then nightly rustfmt (`.rustfmt.toml`: width 120, `wrap_comments`, `comment_width = 100`,
`imports_granularity = "Module"`). Unlike `cpe-core`, both crates are full of `html!`:

- `maudfmt` rewrites `@match f(…)` to a form that trips `clippy::blocks_in_conditions`, and it can
  mangle an `html!` passed as a call argument
  (`dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`,
  `CONVENTIONS.md`). The fix is a `@let` or `let` binding. Each such rewrite is an edit against the
  formatted baseline and is named in the commit body. `cpe-components` has eight `@match`; one has a
  call as its scrutinee (`resource_detail.rs:670`, inside an `@for`, so the binding is a `@let`). It
  has three multi-line `Some(html! {…})` (`compound_viewer.rs:1166`, `:1424`, `annotation.rs:556`),
  30 `=> html!` match arms, 21 `return html!` and nine `@if let` over a call; whether `maudfmt` touches
  the last three kinds is not known. After `just fmt` the executor reads the diff for an `html!` body
  that lost its structure (flat indentation, `class = "…"`, `@ for`) and rewrites only those sites.
- An `#[expect(…)]` that no longer fires fails clippy under `-D warnings`. It is removed, never turned
  into `#[allow]`.
- `just fmt` is run until a run changes nothing; in #470 one pass was not enough.
- `wrap_comments` breaks a `// ── Title ───` ruler that passes 100 columns into two lines.
- Two nightly rustfmt builds can wrap comments differently
  (`docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`). If CI's
  format check disagrees with the local one, compare `cargo +nightly fmt --version` with the CI log
  before touching a comment by hand.

The incubator never formats a vendored crate (`cpe/tools/own-crates.sh`), so the text formatted here is
what it vendors.

**Snapshots.** insta compares a snapshot's content, not its header. The copied files keep the
incubator's `source:` lines, which name paths that do not exist here; that is harmless and is not fixed
by accepting them
(`docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`). The file names
carry the crate name, `cpe_components__…`, which does not change. CI sets `INSTA_UPDATE=new`; a mismatch
fails either way. An unreferenced `.snap` file fails nothing, so the copy is checked by `diff -r` and by
the test list, not by `just test` alone.

**Lints and checks that `just check` and `just test` do not run.** `just check` runs clippy without
`--all-targets`; this plan runs it with the flag on both crates. CI builds documentation on nightly and
only warns; this plan runs `cargo doc -p <crate> --no-deps` and counts the warnings in Phase 1 and 2.
Warnings about links to private items are comment edits: the viewer's are fixed in Phase 1, the
component crate's in Phase 3. `cargo machete` runs in `just check`: `include_dir` and `rusqlite` must
be gone from the manifests, as the manifests above have it. `cargo deny check` is run by no recipe; on
`origin/main` it already reports errors (every workspace crate "is unlicensed", among others), and this
plan adds no third-party package, so it is not run.

**Gates that read the index.** `check-adr-refs.sh`, `check-composition-root-deps.sh`,
`check-datastar-delimiters.sh`, the new seam check and the maudfmt loop use `git ls-files` or `git grep`. New files are staged before `just fmt` and
`just check` see them. `check-adr-refs.sh` reads every tracked text file, the JS, the shell script, the
stylesheets and the fixtures included.

**Vendored JavaScript outside the checksum gate.** `CONVENTIONS.md` puts vendored JS in a module's
`public/vendor/`, where `just verify-checksums` checks it. `openseadragon.min.js` and its 40 sprites are
compiled into a crate instead, with their own `VENDOR.lock`. Nothing in `just check` compares them with
the lock. Phase 1 compares them once; the README says so; it is recorded debt. No JS linter in this
repository reads the two JS files.

**OpenSeadragon's licence.** The source ships no licence file; the minified file's header only links to
one. Phase 1 adds the upstream licence text at the vendored version (6.0.2 per `VENDOR.lock`) as
`assets/LICENSE-openseadragon.txt` and serves it with the other assets. The executor reads the licence it
fetched and names it in the README; this plan does not assert which licence it is.

**Where the stylesheets are served from.** Nowhere in this repository until DEV-7465. The incubator's
binary embeds `cpe/static` and serves it at `/_static/…`; `cpe/web` hard-codes the seven
`/_static/cpe/<file>.css` links. In Phase 5 the binary embeds the vendored `style/` directory as a
second directory and keeps the URLs. The embedding is a module-level `include_dir!` in
`cpe/bin/src/router.rs`, and the `/_static/*path` route asks it first: after the same segment check
`serve_static` applies, a path of the form `cpe/<name>.css` is looked up as `<name>.css` there
(`style/` is flat), and every other path falls through to `serve_static` over `cpe/static` as today.
`router::build`'s signature and `cpe/bin/src/lib.rs` do not change, so none of the 80 test call sites
that pass a static directory changes, and every router and smoke test exercises the directory the
binary really embeds. `shell.css` refers to `dasch-fingerprint.svg` by a URL relative to itself, so
that file, which stays in `cpe/static/cpe/`, has to stay reachable at `/_static/cpe/` beside the
stylesheet; DEV-7465's host has to provide the same. Nothing in the incubator enumerates, hashes or
precompresses the static directory. CPE does not use Tailwind, so no `@source` glob changes.

**`eng.yaml`.** `areas/access/cpe/image-viewer/**` and `areas/access/cpe/components/**` produce HTML
(and the viewer's JS builds its own controls) but match none of the HTML overrides; the stylesheet
override `areas/*/*/style/*.css` is one level too shallow for `components/style/`. Phases 1 and 2 each
add an override that names `eng:review:accessibility-reviewer` and `eng:review:ivan-reviewer`. Neither
names a `verify` skill: nothing here can render the components in a browser yet.

**`PreEscaped`.** The component crate passes rendered Markdown and narrative HTML through `PreEscaped`
(`markdown.rs`, `editorial.rs`, and two constants in `annotation.rs` and `compound_viewer.rs`). The
sites are copied unchanged. What they trust is for the security reviewer to note in PR B's description,
not an edit in this plan.

**`check-composition-root-deps.sh`.** It forbids `access-server` a dependency on crates in `core`,
`web` and `api-*` folders. `components` and `image-viewer` are not in that list; rule 1 of the seam
check covers them, for every crate.

**`ARCH-MAP.md`.** The `areas/access/cpe` entry is refreshed with `dune:dune-map` in Phases 1 and 2. Its
Fingerprint covers `areas/access/cpe/**` and is computed from the index, so each later commit that
touches the capability recomputes it. Its "Depends on" still says CPE's coming crates depend on
`modules/mosaic`, which the seam forbids; Phase 2 corrects it. Its Local-context kit holds 14 files
against a budget of seven; this plan adds the two crates' `src/lib.rs` and no other.

**Release.** Neither crate needs a release-please entry. Both fall under the root package. No commit
carries a `Release-As` footer.

**Commits (dsp-repository).**

PR A:

1. `docs(docs): add the plan for moving CPE's component library (DEV-7401)`
2. `feat(cpe-image-viewer): add CPE's image viewer, cut down from dsp-incubator's (DEV-7401)`

PR B:

3. `feat(cpe-components): move CPE's component library in from dsp-incubator (DEV-7401)`
4. `docs(cpe-components): remove history and plan references from the comments (DEV-7401)`
5. `chore(ci): check CPE's dependency seam in just check (DEV-7401)`

Commit 2 also carries the new decision record, its entry in `docs/src/decisions.md` and ADR-0007's
amendment: the record is what permits the move, as #470's amendment travelled with its `feat` commit.

A review finding is amended into the commit it belongs to, the commits above it are rebased, the proofs
and the Fingerprint are run again, and the branches are pushed with `--force-with-lease`. Each PR
carries a "Review Notes" section. `just commit-lint` for more than one commit needs the PR body:
`PR_BODY="$(gh pr view <n> --repo dasch-swiss/dsp-repository --json body -q .body)" just commit-lint <base>`,
with the PR's own base branch as `<base>`.

**The incubator PR is one squashed commit**, as a pin move and the changes it forces are one PR there.

**Phase 5 is a delta on DEV-7464's incubator PR.** That PR (Phase 3 of #470's plan, not yet written)
vendors `cpe-core` and edits most of the files Phase 5 edits: `cpe/vendor/CRATES` and `README.md`,
`cpe/Cargo.toml`, `cpe/justfile` (`watch`, both `module-graph` loops), `cpe/tools/crate-deps.py`,
`cpe/Dockerfile`, `cpe/CLAUDE.md`, `cpe/CONTEXT.md`, `cpe/docs/ARCHITECTURE.md`,
`cpe/docs/INCUNABULA_GO_LIVE.md`, the incubator's ADRs 0001 and 0013, `cpe/web/src/page.rs`, and the
workflow's path filters. It also repoints `core/src` in the comments of five stylesheets, and re-homes
`tokens_match_the_budget_formula` into `cpe/web`. Phase 5's checkboxes name what has to be true
afterwards; the executor reads those files as that PR left them, and where a loop or a list was
reshaped for a vendored `core`, `components` follows the same shape. This is the real reason for H4:
`cpe-components` reaches `cpe-core` by `../core`, which exists in the vendored tree only once
`cpe-core` is vendored.

**The uncommitted check in the incubator.** In the incubator worktree's `cpe/Cargo.toml`, the workspace
dependencies `cpe-core` and `cpe-components` are pointed at this worktree's crates by absolute path,
`dasch-image-viewer` at this worktree's `areas/access/cpe/image-viewer` with
`package = "cpe-image-viewer"` (the rename keeps `dasch_image_viewer::` compiling in `cpe/web` and
`cpe/bin`), and the members `core` and `components` are removed. A path dependency outside a workspace
takes its inherited keys from its own workspace root, so the moved manifests resolve against this
repository's root `Cargo.toml`. The three paths are this worktree's absolute path written the same way
each time, with no `..` and no symlinked prefix: cargo identifies a path crate by its path as written,
and `cpe-components` reaches the other two by `../core` and `../image-viewer`. Nothing is committed,
and Phase 5 starts by discarding the change. The incubator's `Cargo.lock` is rewritten by this and
needs network access: three packages change their source and version, and `maud` gains this
repository's `axum` feature. The incubator's test recipe does not pass `--locked`. The incubator then
tests against this repository's `cpe-core`, so a failure may come from #470 and not from this plan; the
report says which. `cpe-core` has to be redirected in every case:
`cpe-components` depends on it by the relative path `../core`, and a second `cpe-core` in the graph
(the incubator's own, or one it has vendored by then) would be a different crate with different types.
`cargo test --workspace --all-targets` there (what `just cpe test` runs) covers the incubator's own
crates (`web`, `engine`, `bin`, `store`, `store-build`) against the moved ones; the moved crates' own
tests are not workspace members there and run in Phases 1 and 2. One incubator test reads
`cpe/core/src/validate.rs` from disk; where that directory still exists it reads the incubator's copy,
not #470's. The binary still embeds the incubator's `cpe/static`, so the moved
stylesheets are not what H2 sees. The arrangement is also saved as a patch under the scratch path, so
that it can be restored. If cargo cannot resolve the arrangement, the reason is reported in PR B's
description and the check first happens in Phase 5.

**Commands.** `eng.yaml` resolves `just check` (lint), `just test` (test) and
`just check && just test` (gate) for every path this plan touches here. Under Nix they are prefixed with
`nix develop --command`. The incubator has no `eng.yaml`; its commands are `just cpe test` and
`just cpe ci`, run from its repository root. Its Cargo workspace is `cpe/`. `just cpe ci` needs network
access. Never run `cargo fmt` in `cpe/`.

**Reviewers.** `eng:review:rust-reviewer`, `eng:review:dune-reviewer`,
`eng:review:consistency-reviewer` and `eng:review:code-simplicity-reviewer` in every phase;
`eng:review:security-reviewer` in Phase 1 (`asset()` takes a request path) and
`eng:review:devops-reviewer` in Phase 4 (a gate script). In Phases 1 and 2 the reviewers judge the
edits and the registrations, not the copied lines: a finding about the copied code's design goes to the
PR description. In Phase 3 they read the comment diff for a deleted contract and for whitespace changed
inside a quoted selector, which no mechanical check finds.

## Implementation Phases

#### Phase 1: `cpe-image-viewer` enters `areas/access/cpe/image-viewer`

### dsp-incubator

- [x] Fetch, then create the worktree named in `repositories:` on the branch `worktree-DEV-7401` from
      `origin/main`; `SRC` is its `HEAD` sha
- [x] Save the test list of `shared/dasch-image-viewer` at `SRC` (run from that directory) under an
      absolute scratch path outside the repository, as the viewer's source test list
- [x] Save the test list of `cpe-components` at `SRC` (run from `cpe/`) beside it, as the components'
      source test list

### dsp-repository

- [x] Confirm the branch is `worktree-DEV-7401` and `HEAD` is `origin/worktree-DEV-7464`'s head or a
      descendant of it; where #470 has merged, rebase on `origin/main` instead
- [x] Create and switch to the branch `worktree-DEV-7401-image-viewer` at `HEAD`
- [x] In `docs/specs/README.md` → *Plans and PRs*, reword "One plan is one PR" so that a plan whose
      parts are stacked PRs (`docs/src/git-conventions.md` → *One PR, several commits, or a stack*) is
      one plan with one phase group per PR, and "Never plan one PR per phase" still holds within a PR
- [x] Commit this plan and the README change as
      `docs(docs): add the plan for moving CPE's component library (DEV-7401)`
- [x] Where `SRC` is not `1cd5781f…`, read `git log 1cd5781f..SRC -- shared/dasch-image-viewer
      cpe/components cpe/static/cpe` in the incubator, find this plan's source line numbers again by
      content, and take the counts of *Success Metrics* again
- [x] Export `shared/dasch-image-viewer` at `SRC` with `git archive` into `.claude/tmp/`
- [x] Copy `src/lib.rs`, `tests/render.rs`, `assets/image-viewer.js`, `assets/vendor/openseadragon.min.js`,
      `assets/vendor/images/`, `assets/vendor/VENDOR.lock` and `scripts/vendor-osd.sh` to
      `areas/access/cpe/image-viewer`, and nothing else
- [x] Write `areas/access/cpe/image-viewer/Cargo.toml` as in *`cpe-image-viewer`*
- [x] Add `"areas/access/cpe/image-viewer"` to the root `Cargo.toml`'s `members`, after
      `"areas/access/cpe/core"`
- [x] Stage the crate and the root `Cargo.toml`, then run `just fmt` until a run changes no file
- [x] Copy `src/lib.rs` and `tests/render.rs` to `.claude/tmp/` as the formatted baseline, before any
      edit
- [x] Compare the SHA-256 of each of the 41 vendored OpenSeadragon files with `VENDOR.lock`; all match
- [x] Replace the embedded directory in `src/lib.rs` with the `ASSETS` table and the table lookup in
      `asset()`, and change `content_type_for`, as the edit table says
- [x] Delete `Engine`, `Strategy`, `MapView`, `Modality`, `LayeredView`, `json_layers_array`, and the
      `Viewer` fields and builders the edit table lists; `render()` emits the literal `engine="osd-wc"`
- [x] Reword the documentation in `src/lib.rs` that names a removed engine, type or builder (the edit
      table's rows for `Viewer`, `adjust`, `chrome` and `push_json_string`), and rename the crate in
      the two surviving doc tests
- [x] In `assets/image-viewer.js`, delete the engine gate, `_mountEngine`, `_failLight`,
      `_engineHandle`, the `map` getter and the handle teardown; the declaration of `BUILTIN_ENGINE`
      stays
- [x] In `assets/image-viewer.js`, delete the "Adding an engine" comment and reword the comments at the
      sites the edit table lists; the one inside the template literal gets no backtick; no other line
      changes
- [x] Run `node --check areas/access/cpe/image-viewer/assets/image-viewer.js`; it accepts the file
- [x] In `tests/render.rs`, delete the 22 tests, the import and the two helpers the edit table lists,
      drop the `openlayers.js` requirement from the `_applyFilter` test, correct the comments and the
      assertion message the table lists, and rename the crate
- [x] Add to `tests/render.rs` the test that walks `assets/`, skipping dotfiles, and asserts `asset()`
      returns each file
- [x] Fetch OpenSeadragon's licence text for the version `VENDOR.lock` names from the upstream
      repository's tag (`LICENSE.txt` at `v6.0.2`), save it as `assets/LICENSE-openseadragon.txt`, and
      add its row to `ASSETS`; where the tag has no licence file, or the file names no licence, stop
      and report
- [x] Rewrite the three `cpe ADR-0010` citations in `src/lib.rs` and `assets/image-viewer.js` to
      `dsp-incubator cpe/docs/adr/0010-openseadragon-integration.md`
- [x] Edit `scripts/vendor-osd.sh` as the edit table says (`:6`, `:9`, the `ASSETS` instruction); it
      still writes only under `assets/vendor/`
- [x] Write `areas/access/cpe/image-viewer/README.md` with the content *`cpe-image-viewer`* lists
- [x] Run `git grep -n -i 'openlayers\|MapView\|LayeredView\|_mountEngine\|include_dir\|engines/'
      -- areas/access/cpe/image-viewer`; every hit is in the README's account of what was removed
- [x] Run `git grep -n 'ADR-[0-9]' -- areas/access/cpe/image-viewer`; it prints nothing
- [x] Run `git grep -n 'adr/0010' -- areas/access/cpe/image-viewer`; every hit is the file-name
      citation `dsp-incubator cpe/docs/adr/0010-openseadragon-integration.md`
- [x] Run `just fmt` until a run changes no file
- [x] Run the three diffs of *Verification design* → *Phase 1*; each shows only what the edit table
      lists
- [x] Run `cargo build -p cpe-image-viewer` and stage `Cargo.lock`; it gained the `cpe-image-viewer`
      entry and no other package or version
- [x] Run `cargo test -p cpe-image-viewer`; it passes, doc tests included
- [x] Build and run the throwaway crate of *Verification design* that renders the builder matrix with
      the source crate and with `cpe-image-viewer`; every pair of strings is equal
- [x] Compare the test list with the viewer's source test list; the difference is the 22 deleted tests,
      the two doc tests of `MapView` and `LayeredView`, and the one new test
- [x] Run `cargo clippy -p cpe-image-viewer --all-targets --all-features -- -D warnings`; it prints no
      warning
- [x] Run `cargo doc -p cpe-image-viewer --no-deps`; fix each warning that is a broken or private
      intra-doc link by making it a code span; it then prints no warning
- [x] Add `cpe-image-viewer` to the Crates row of the scope table in `CONVENTIONS.md` and to the "Crate
      scopes" list in `docs/src/git-conventions.md`
- [x] In `docs/src/repo_structure.md`, add an `image-viewer/` line to the tree under `cpe/` and a
      `cpe-image-viewer` row to the crate table that says it depends on `maud` alone and is an interim
      copy, and correct the tree's comment on `cpe/` ("its domain crate and its port so far")
- [x] Add an override for `areas/access/cpe/image-viewer/**` to `eng.yaml` with the reviewers
      `eng:review:accessibility-reviewer` and `eng:review:ivan-reviewer`
- [x] In `areas/access/cpe/CONTEXT.md`, correct the opening paragraph and add a short section on the
      image viewer: what it is, that it is interim, and where the full one lives
- [x] Correct the sentence about CPE in `areas/access/CONTEXT.md`
- [x] Correct the Access Area bullet in the root `CONTEXT.md`
- [x] Write the new root ADR of decision 14 in `docs/adr/`, with the next free number, in the form of
      the existing records (frontmatter `status: accepted` and `date`, context, decision, considered
      options, consequences, "Enforced by: review")
- [x] Add the new record to the list in `docs/src/decisions.md`
- [x] Add "Amendment (2026-10-11, DEV-7401)" to ADR-0007 with the content decision 14 gives for PR A:
      that the churn sentence of the amendment of 2026-10-02 is replaced by the new record, cited by
      its number; the viewer's interim packaging (decisions 3 to 5) as the answer, for now, to the
      open question the record carries; that the workspace still has no `include_dir` and that no
      project bundle is embedded; and "Enforced by: review"
- [x] Correct the sentence of ADR-0007's amendment of 2026-10-11 (DEV-7464) that says the rule stands
      for the crates that follow, by a dated note beneath it that points to the new record; the
      sentence itself is not rewritten
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry and the Overview sentence about CPE with
      `dune:dune-map` (this checkbox authorises the overwrite). Done when Paths names `image-viewer/`
      and `cpe-image-viewer`; Key entities name `Viewer`, `Auth`, `asset`, `head_tag`; Depends on says
      it depends on `maud` and on no CPE crate; Boundary rules say the vendored files are checked
      against `VENDOR.lock` by hand; the kit gained `areas/access/cpe/image-viewer/src/lib.rs` only;
      and the Fingerprint is the value computed over the staged tree
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as
      `feat(cpe-image-viewer): add CPE's image viewer, cut down from dsp-incubator's (DEV-7401)`, with
      `SRC` as a full sha in the body, what was removed, and no `Release-As` footer

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: `cpe-components` and its stylesheets enter `areas/access/cpe/components`

### dsp-repository

- [x] Switch to `worktree-DEV-7401` and fast-forward it to `worktree-DEV-7401-image-viewer`
- [x] Export `cpe/components`, `cpe/static/cpe`, `cpe/store/schema.sql`,
      `cpe/projects/incunabula/data.sql` and the three narrative files of decision 9 at `SRC` with
      `git archive` into `.claude/tmp/`
- [x] Copy the export's `cpe/components/src` to `areas/access/cpe/components/src`
- [x] Copy the seven `.css` files of the export's `cpe/static/cpe` to
      `areas/access/cpe/components/style`
- [x] Write `areas/access/cpe/components/Cargo.toml` as in *`cpe-components`*
- [x] Add `"areas/access/cpe/components"` to the root `Cargo.toml`'s `members`, before
      `"areas/access/cpe/core"`
- [x] Stage the crate and the root `Cargo.toml`, then run `just fmt` until a run changes no file
- [x] Copy `areas/access/cpe/components/src` to `.claude/tmp/` as the formatted baseline, before any
      edit
- [x] Replace `dasch_image_viewer` with `cpe_image_viewer` in `src/component/media.rs`, code and prose
- [x] Copy the three narrative files to `src/component/testdata/` as `alice-home.en.md`,
      `alice-home.de.md` and `incunabula-home.de.md`, point the three `include_str!` at them, and
      reword the three comments to say each is a copy taken from dsp-incubator
- [x] Produce `src/component/testdata/incunabula-book-members.tsv` with Python's `sqlite3` module: load
      the export's `schema.sql` and `data.sql` into an in-memory database, run the query of
      `incunabula_books()`, and write one line per row, the three values joined by a tab, in the
      query's order and without trimming; no value contains a tab or a newline
- [x] Change `incunabula_books()` to read the extract with `include_str!`, split each line into exactly
      three fields on the tab without trimming, and build the same `Vec<(String, Vec<MemberHeader>)>`;
      reword its doc comment; the test's assertions do not change
- [x] Rewrite the 26 Rust comment lines that carry a bare incubator `ADR-NNNN` (the two relative
      `docs/adr/` links are among them) to `dsp-incubator cpe/docs/adr/<file name>`
- [x] Rewrite the seven stylesheet comment lines that carry a bare incubator `ADR-NNNN` in the same way
- [x] Reword the comment at `src/component/media.rs:258` so that it names the attribute without the
      `=` that `check-datastar-delimiters.sh` matches
- [x] Run `git grep -n 'ADR-[0-9]' -- areas/access/cpe/components`; it prints nothing
- [x] Run `git grep -n '\.\./\.\./\.\.\|CARGO_MANIFEST_DIR' -- areas/access/cpe/components/src`; it
      prints nothing
- [x] Read the formatted baseline's difference from the export's `src` for an `html!` body that
      `maudfmt` left without its structure (*Formatting*); rewrite each such site with a binding and
      note it for the commit body
- [x] Run `cargo clippy -p cpe-components --all-targets --all-features -- -D warnings`; fix each
      warning `maudfmt` caused with a `@let` or `let` binding and note the site for the commit body; it
      then prints no warning
- [x] Run `just fmt` until a run changes no file
- [x] Run `diff -r` between the formatted baseline and `areas/access/cpe/components/src`, snapshots
      and `testdata/` apart; it shows only the rows of the edit table and the rustfmt re-wraps around
      them
- [x] Run `diff -r` between the export's two `snapshots/` directories and the copies; it is empty
- [x] Run `diff -r` between the export's seven stylesheets and `style/`; it shows the seven rewritten
      comment lines and nothing else
- [x] Run `INSTA_UPDATE=no cargo test -p cpe-components`; it passes, and
      `git status --porcelain areas/access/cpe/components` lists no `.snap.new`
- [x] Compare the test list with the components' source test list; they are equal
- [x] Stage `Cargo.lock` and compare it with its Phase 1 state; it gained the `cpe-components` entry
      and no other package or version
- [x] Run `cargo doc -p cpe-components --no-deps`; it builds; record the number and kind of its
      warnings in the commit body
- [x] Repoint `cpe-core`'s 16 comment lines that cite `dsp-incubator cpe/static/cpe/<file>`, the six
      citations split over two lines included, to `areas/access/cpe/components/style/<file>`
- [x] Repoint `cpe-core`'s four comment lines that name `dsp-incubator cpe/components` to
      `cpe-components`
- [x] Run `rg -U 'cpe/static|dsp-incubator\s+(//[/!]\s*)?cpe/components' areas/access/cpe/core`; it
      prints nothing
- [x] Apply the Phase 3 normaliser to every file under `areas/access/cpe/core/src` at `HEAD` and in the
      working tree; the two are equal
- [x] Add `cpe-components` to the Crates row of the scope table in `CONVENTIONS.md` and to the "Crate
      scopes" list in `docs/src/git-conventions.md`
- [x] In `docs/src/repo_structure.md`, add `components/` with `src/` and `style/` to the tree under
      `cpe/` and a `cpe-components` row to the crate table that names its dependencies
- [x] In `areas/access/cpe/CONTEXT.md`, correct the opening paragraph and add a section on the component
      library: the components, the chrome message catalogue, the per-project components and their
      registry, the seven stylesheets and that nothing here serves them yet, with
      `dsp-incubator cpe/CONTEXT.md` as the glossary until DEV-7465
- [x] Correct the sentence about CPE in `areas/access/CONTEXT.md`
- [x] Correct the Access Area bullet in the root `CONTEXT.md`
- [x] Extend ADR-0007's amendment of 2026-10-11 (DEV-7401): the stylesheets live in the component
      library's `style/` directory as plain files, served by the host
- [x] Add an override for `areas/access/cpe/components/**` to `eng.yaml` with the reviewers
      `eng:review:accessibility-reviewer` and `eng:review:ivan-reviewer`
- [x] Refresh `ARCH-MAP.md`'s `areas/access/cpe` entry and the Overview sentence with `dune:dune-map`
      (this checkbox authorises the overwrite). Done when Paths names `components/` and
      `cpe-components`; Purpose no longer says only `ports/` and `core/` exist; Key entities name the
      component modules, `chrome`'s message catalogue and `PROJECT_RENDERERS`, and no longer say the
      component library's entities are not here; Depends on says `cpe-components` depends on
      `cpe-core` and `cpe-image-viewer`, and no longer says CPE's crates depend on `modules/mosaic`;
      Used by names `cpe-components` as `cpe-core`'s consumer; Boundary rules no longer call the rest
      docs-only "until CPE's other crates exist" and still read **review** for the seam (Phase 4
      changes that); the kit gained `areas/access/cpe/components/src/lib.rs` only; and the Fingerprint
      is the value computed over the staged tree
- [x] Grep the repository outside `docs/specs/` for `component library` and `not here yet`; every
      remaining hit is true after the move
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as `feat(cpe-components): move CPE's component library in from dsp-incubator (DEV-7401)`,
      with `SRC` as a full sha in the body, the four repointed tests, the command that produced the
      extract, the format-forced rewrites, and no `Release-As` footer

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: the component library's comments follow `CONVENTIONS.md`

### dsp-repository

- [x] Record `git rev-parse HEAD` as the Phase 2 commit
- [x] Apply the *Comment rules* to `src/lib.rs`, `src/chrome.rs` and `src/component/mod.rs`
- [x] Apply the *Comment rules* to `src/component/resource_detail.rs`
- [x] Apply the *Comment rules* to `src/component/resource_list.rs`
- [x] Apply the *Comment rules* to `src/component/compound_viewer.rs` and `src/component/media.rs`
- [x] Apply the *Comment rules* to `src/component/editorial.rs`, `narrative.rs`, `markdown.rs`,
      `annotation.rs` and `breadcrumb.rs`
- [x] Apply the *Comment rules* to `src/component/facet_panel.rs`, `data_links.rs`, `data_entries.rs`,
      `links.rs`, `search_bar.rs`, `page_picker.rs`, `pager.rs`, `context.rs`, `dom.rs` and `values.rs`
- [x] Apply the *Comment rules* to every file under `src/component/project/`
- [x] Apply the *Comment rules* to the seven files under `style/`
- [x] Turn each intra-doc link `cargo doc -p cpe-components --no-deps` warns about into a plain code
      span; it then prints no warning
- [x] Run `just fmt` until a run changes no file
- [x] Run checks 1 to 3 of *Verification design* → *Phase 3* against the Phase 2 commit; each difference
      printed is rustfmt re-joining a line
- [x] Read every hunk of `git diff <Phase 2 commit> -- areas/access/cpe/components` that touches a
      comment quoting a CSS selector, a path or a URL; the quoted text is unchanged character for
      character
- [x] Search the crate's Rust and CSS comment lines for `Slice [0-9]`, `[Ss]tep [0-9]`,
      `Phase [0-9]`, `decision [0-9]`, `\b[CDGORS][0-9]{1,2}\b`, `\bplan\b`, `journal`, `BACKLOG`,
      `backlog`, `#[0-9]{2,4}\b`, `components/src/`, `core/src/`, `static/cpe/`, `cpe-engine`,
      `[Pp]reviously`, `[Oo]riginally`, `no longer`, `now uses` and `was changed`; every hit that
      remains is a comment one of the rules keeps
- [x] Run `diff -r` between the Phase 2 commit's snapshots and the working tree's; it is empty
- [x] Compare the test list with the list of Phase 2; they are equal
- [x] Recompute the Fingerprint of `ARCH-MAP.md`'s `areas/access/cpe` entry over the staged tree and
      write it; the entry's other fields do not change
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as
      `docs(cpe-components): remove history and plan references from the comments (DEV-7401)`

### dsp-incubator

- [x] Arrange the incubator worktree as *The uncommitted check in the incubator* describes; commit
      nothing
- [x] Run the incubator's module-graph check on the moved source (from `cpe/`:
      `python3 -I tools/module-graph.py <this worktree>/areas/access/cpe/components/src --check`); it
      passes, or the failure is noted for PR B's description as something Phase 5 has to solve in the
      tool
- [x] Run the incubator's own crates' tests against that arrangement (from `cpe/`:
      `cargo test --workspace --all-targets`); they pass, or each failure is traced to #470 or to a
      change this plan makes on purpose and is noted for PR B's description
- [x] Save the arrangement as a patch (`git diff -- cpe/Cargo.toml cpe/Cargo.lock`) under the absolute
      scratch path, and leave it applied in the incubator worktree, uncommitted, for H2

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: CPE's dependency seam is checked by `just check`

### dsp-repository

- [x] Write `.github/scripts/check-cpe-seam.test.sh` with the cases *The seam check* lists, after
      `check-composition-root-deps.test.sh`: throwaway git repositories under
      `mktemp -d "${TMPDIR:-/tmp}/…"`, the inherited `GIT_*` variables unset, one case per failure mode,
      the passing cases, and absence as an error
- [x] Write `.github/scripts/check-cpe-seam.sh` with the three rules, the matcher and the depth filters
      of *The seam check*, and a header that states the rules, why the port is excepted, and that
      DEV-7465 adds the host-facing crates to the exception list; the test script passes
- [x] Add the recipe `check-cpe-seam` to the `justfile` in the shape of `check-composition-root-deps`,
      with `(DEV-7401, ADR-0007)` in its doc comment
- [x] Add `check-cpe-seam` to the `check` recipe's dependencies and the test script to the `test`
      recipe's script list
- [x] Run `just check-cpe-seam` on the tree; it passes
- [x] Add a path dependency on `cpe-components` to `areas/access/dpe/web/Cargo.toml`, run
      `just check-cpe-seam`; it fails and names the manifest, the dependency and the rule; restore the
      manifest, and
      `git status --porcelain areas/access/dpe` prints nothing
- [x] Extend ADR-0007's amendment of 2026-10-11 (DEV-7401): the three rules as checked, that the port
      is the one crate another capability may depend on, that DEV-7465 adds the host-facing crates, and
      "Enforced by: static-analysis (`check-cpe-seam.sh`, run by `just check`) until Bazel
      `visibility`"
- [x] In `ARCH-MAP.md`, change the seam rules in the `areas/access/cpe` entry's Boundary rules from
      **review** to **static-analysis** and name the script; where the `areas/access/cpe` and
      `areas/access/server` entries say the composition root may depend on `cpe-server` or `cpe-store`,
      add that the seam check's exception list has to name them first (DEV-7465)
- [x] Add the script to the list under `ARCH-MAP.md`'s cross-cutting "CI" entry
- [x] Add `just check-cpe-seam` to the command table in `docs/src/workflows.md`
- [x] State the seam in `areas/access/cpe/CONTEXT.md` in one paragraph that names the script
- [x] Recompute the Fingerprint of `ARCH-MAP.md`'s `areas/access/cpe` entry over the staged tree and
      write it
- [x] Run `just check`; it passes
- [x] Run `just test`; it passes
- [x] Commit as `chore(ci): check CPE's dependency seam in just check (DEV-7401)`
- [x] With the `gh-stack` skill, adopt the stack bottom to top (`worktree-DEV-7464`,
      `worktree-DEV-7401-image-viewer`, `worktree-DEV-7401`) and submit it; where the local
      `worktree-DEV-7464` is not at its upstream's head, stop and report (H1)
- [x] With `gh pr edit`, give PR A its title (the Phase 1 code commit's subject), the owner as
      assignee, no reviewer, and a body from the repository's PR template: "Part of DEV-7401",
      `allow-many-commits` ticked, a "Review Notes" section that says what each commit proves and how
      to run the proof again, the *Human Actions* table with a line for the owner to tick when H2 is
      done, and the viewer's recorded debt; the PR stays a draft
- [x] With `gh pr edit`, give PR B its title (the Phase 2 commit's subject), assignee and body in the
      same way, with "Part of DEV-7401" (PR C closes the issue), the result of the uncommitted check in
      the incubator, what the `PreEscaped` sites trust, and the recorded debt of *Dependencies & Risks*;
      the PR stays a draft
- [x] Run `just commit-lint` for PR A with `PR_BODY` set from it and `worktree-DEV-7464` as the base; it
      passes
- [x] Run `just commit-lint` for PR B with `PR_BODY` set from it and `worktree-DEV-7401-image-viewer`
      as the base; it passes

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4a: the other moved CPE crates' comments follow the same rules

Added on 2026-10-11 during execution, by the owner's decision: the comment clean-up is read strictly
(Phase 3, second pass), and the same pass runs over the CPE code that entered the monorepo before
`cpe-components`. This overrides decision 12 for the viewer. `sync-store` was checked and is left as it
is: its `§` references point into the pinned `FORMAT.md`, which defines an enforced rule.

The rules are *Comment rules (Phase 3)* with rule 3 read widely: a reference to a plan, journal,
backlog, design document, handoff, prompt file, mockup, spike or strings file, with or without a path,
is deleted; ids of spec items, states, criteria, slices, steps and decisions are deleted and, where
the reader needs the state, replaced by what it is in words; narration of who did what is deleted;
history that justifies a present rule is rewritten in the present tense; a pointer to a file that
defines an enforced rule stays; a rule the code depends on stays, in plain words. String literals,
test names and code do not change.

### dsp-repository

- [x] `cpe-image-viewer`: apply the rules to the comments of `areas/access/cpe/image-viewer/src/lib.rs`,
      `tests/render.rs`, `assets/image-viewer.js` and `scripts/vendor-osd.sh`; the vendored files under
      `assets/vendor/` are not touched; the README keeps its statement of origin and of what was
      removed, which is present state
- [x] `image-viewer.js` is equal to its previous version once comments are removed (block and line
      comments, by a small script that keeps string and template literals intact, or by reading every
      hunk of the diff), `node --check` passes, the word `APPROXIMATION` and `const BUILTIN_ENGINE =
      "osd-wc"` are still there, and `cargo test -p cpe-image-viewer` passes
- [x] Commit the viewer's change as `fixup! feat(cpe-image-viewer): add CPE's image viewer, cut down
      from dsp-incubator's (DEV-7401)`; the session folds it into PR A's commit
- [x] `cpe-core`: apply the rules to the comments of `areas/access/cpe/core/src/**/*.rs`; the `.rs`
      files are equal to their previous version after the Phase 3 normaliser (proof 1), the test list
      of `cpe-core` is unchanged, `cargo doc -p cpe-core --no-deps` prints no new warning
- [x] Commit as `docs(cpe-core): remove design and plan references the first pass left (DEV-7401)`
- [x] `cpe-ports`: apply the rules to the comments of `areas/access/cpe/ports/src/**/*.rs`, with the
      same proof; where no comment needs a change, record that in the journal and make no commit
- [x] If it changed: commit as `docs(cpe-ports): remove design and plan references from the comments
      (DEV-7401)`
- [x] Recompute the `areas/access/cpe` Fingerprint in `ARCH-MAP.md` over the staged tree where a
      commit changed the capability's files, in that commit
- [x] `just check` passes
- [x] `just test` passes

- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: the incubator builds against the vendored `cpe-components` and `cpe-image-viewer`

**Gate: H2, H3, H4** — resolve before starting this phase.

### dsp-incubator

- [ ] Discard the uncommitted arrangement of Phase 3 (`git restore cpe/Cargo.toml cpe/Cargo.lock`),
      `git fetch --prune`, and rebase the branch `worktree-DEV-7401` on `origin/main`
- [ ] Confirm `cpe/vendor/CRATES` lists `areas/access/cpe/core`; where it does not, stop and report (H4)
- [ ] Run `git diff SRC origin/main -- cpe/components cpe/static/cpe`. The only hunks allowed are
      DEV-7464's repoints of `core/src` paths in stylesheet comments. Where it shows anything else,
      stop and report it: that change has to land in dsp-repository first
- [ ] Run `git log SRC..origin/main -- shared/dasch-image-viewer` and keep the output for the PR
      description
- [ ] Set `<pin>` to the full sha of the commit on dsp-repository's `origin/main` that PR B's last
      commit became (its subject is the Phase 4 commit's unless a reviewer reworded it); confirm
      `<pin>:areas/access/cpe/components/Cargo.toml`, `<pin>:areas/access/cpe/image-viewer/Cargo.toml`
      and `<pin>:.github/scripts/check-cpe-seam.sh` exist
- [ ] Save the names of every test in the workspace before any change (from `cpe/`, with the test-list
      command of *Verification design* and `--workspace` in place of `-p <crate>`) under an absolute
      scratch path outside the repository
- [ ] Add `areas/access/cpe/image-viewer` and `areas/access/cpe/components` to `cpe/vendor/CRATES`
- [ ] In `cpe/Cargo.toml`, replace the member `"components"` with the two vendored paths, the workspace
      dependency `cpe-components` with its vendored path, and `dasch-image-viewer` with
      `cpe-image-viewer = { path = "vendor/dsp-repository/areas/access/cpe/image-viewer" }`
- [ ] `git rm -r cpe/components`
- [ ] `git rm` the seven stylesheets under `cpe/static/cpe/`; the logos and `datastar.js` stay
- [ ] Run `just cpe vendor <pin>`; the file `cpe/vendor/PIN` holds `<pin>`
- [ ] Compare every `[workspace.dependencies]` key the two vendored manifests inherit (`maud`,
      `pulldown-cmark`, `tracing`, `insta`) with dsp-repository's root `Cargo.toml` at `<pin>`; make
      `cpe/Cargo.toml`'s lines byte-equal, features included (`maud` gains `axum`), and add the `insta`
      entry, which the incubator's workspace does not have. `version` is not copied: the vendored
      crates carry the incubator's, as `cpe-ports` and `sync-store` do
- [ ] Review what else the pin move changed under `cpe/vendor/dsp-repository/`; name the dsp-repository
      commits in the PR description
- [ ] In `cpe/web` and `cpe/bin`, depend on `cpe-image-viewer` and replace `dasch_image_viewer::` with
      `cpe_image_viewer::` (`cpe/web/src/page.rs`, `cpe/bin/src/router.rs`, both manifests)
- [ ] Make the binary serve the seven stylesheets at their present URLs (`/_static/cpe/<file>.css`) from
      the vendored `areas/access/cpe/components/style` directory, as *Where the stylesheets are served
      from* describes: a module-level `include_dir!` in `cpe/bin/src/router.rs`, asked before the
      `cpe/static` lookup; `router::build`'s signature and `cpe/bin/src/lib.rs` do not change
- [ ] Add a `cargo:rerun-if-changed` line for the vendored `style` directory to `cpe/bin/build.rs` and
      name the third embedded directory in its doc comment
- [ ] Add a router test that `/_static/cpe/tokens.css`, `/_static/cpe/dasch-logo-colour.svg` and
      `/_static/cpe/dasch-fingerprint.svg` are served, and that a dot segment or `..` under
      `/_static/cpe/` is refused; the existing tests of `serve_static` pass unchanged
- [ ] Repoint the three `include_str!` of stylesheets in `cpe/web/src/page.rs` (`shell.css`,
      `editorial.css`, `record.css`) to the vendored directory
- [ ] Repoint every other reader of `static/cpe/*.css` in `cpe/` (`rg -l 'static/cpe' cpe --glob
      '!cpe/vendor/**' --glob '!cpe/docs/plans/**'`), the re-homed `tokens_match_the_budget_formula`
      included
- [ ] Delete `COPY cpe/components` and `COPY shared/dasch-image-viewer` from `cpe/Dockerfile` and
      reword its header comment: the repository-root build context is no longer needed for a shared
      crate. `cpe/prototype.json` keeps `"build_context": "root"`, which every `COPY cpe/…` relies on
- [ ] Remove `shared/dasch-image-viewer/**` from both path filters of the incubator's `cpe` workflow
- [ ] In `cpe/justfile`, repoint `watch`'s `components/src` path and the `components` entry of the
      `module-graph` and `module-graph-check` loops to the vendored path, in the shape DEV-7464's PR
      gave them for `core`
- [ ] Change `cpe/tools/crate-deps.py`: `cpe-image-viewer` is now a workspace member, so add it to the
      allowed edges of `cpe-components` and of `cpe-web` in `ALLOWED` and to the rules in the
      docstring, and leave it ungoverned itself; no fixture file changes
- [ ] Run `just cpe crate-deps-check` (it runs the tool's self-test first) and
      `just cpe module-graph-check`; both pass
- [ ] Run `git grep -n 'cpe/components\|components/src\|dasch-image-viewer\|dasch_image_viewer'` over
      the incubator outside `cpe/vendor/`, `cpe/docs/plans/`, `cpe/docs/learnings/`,
      `shared/dasch-image-viewer/` and `image-viewer-showcase/`; repoint each hit that states a
      current rule or path about CPE's use of the crate
- [ ] Add both crates to `cpe/vendor/README.md`: the keys they inherit and that `version` is not one of
      them; that the viewer is a cut-down copy and a fix lands in dsp-repository first; that the
      stylesheets are vendored with the component crate and `shell.css` needs
      `dasch-fingerprint.svg` served beside it; that `tools/module-graph.py` finds the crate by its
      directory name and its `MODULES["components"]` has to follow a module added upstream in the same
      pin-move PR; and that the guards tying Rust constants to the stylesheets
      (`header_desk_breakpoint_matches_shell_css`, `text_align_attribute_matches_the_stylesheets`,
      `tokens_match_the_budget_formula`) run here, so a CSS change in dsp-repository is first checked
      at the pin move
- [ ] Correct `cpe/docs/design/claude-design-workflow.md` where it says the design project mirrors
      `cpe/static/cpe/*.css`: the source is dsp-repository's `areas/access/cpe/components/style`
- [ ] In `shared/README.md`, change CPE's row in the consumer table: CPE builds against the vendored
      `cpe-image-viewer`; and add a note to `shared/dasch-image-viewer/README.md` that dsp-repository
      holds a cut-down copy taken at `SRC`, with the list of what a fix here has to be applied to
- [ ] Tick DEV-7401 under "Moving" in `cpe/docs/INCUNABULA_GO_LIVE.md`
- [ ] Correct the moving rule in `cpe/docs/INCUNABULA_GO_LIVE.md` ("A crate moves … when it has gone
      quiet"): dsp-repository's new record replaces it, and a crate moves in dependency order when the
      next piece of work needs it there; cite the record by its file name in dsp-repository
- [ ] Add dated notes to `cpe/docs/adr/0001-crate-layout.md` (the component crate is vendored),
      `cpe/docs/adr/0005-css-baseline.md` (the stylesheets live in dsp-repository),
      `cpe/docs/adr/0010-openseadragon-integration.md` (CPE builds against the cut-down copy) and
      `cpe/docs/adr/0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md` (the fourth
      and fifth vendored crates; a component or CSS change is a dsp-repository PR and a pin move)
- [ ] Correct `cpe/CLAUDE.md`, `cpe/CONTEXT.md` and `cpe/docs/ARCHITECTURE.md` where they say the
      component crate, the stylesheets or the viewer dependency are developed in this workspace
- [ ] Compare the workspace's test names with the list saved before the change; the new list is the
      saved one plus exactly the tests of `cpe-image-viewer`, which was not a workspace member before,
      and the router test this phase adds
- [ ] Run `just cpe test`; it passes with no `.snap.new` written
- [ ] Run `git status --porcelain --ignored cpe/vendor`; it lists no ignored file
- [ ] Run `just cpe vendor-diff`; it prints `vendor-diff: empty (<pin>)`
- [ ] Run `just cpe ci`; it passes
- [ ] Commit `cpe/vendor/PIN`, the vendored tree, `cpe/Cargo.lock`, the deletions and every forced
      change together as
      `refactor(cpe): build against the vendored cpe-components and cpe-image-viewer (DEV-7401)`
- [ ] Open a draft PR with `gh pr create --repo dasch-swiss/dsp-incubator --head worktree-DEV-7401
      --base main --draft --assignee @me`, with "Fixes DEV-7401", the viewer changes since `SRC`, and
      the *Human Actions* that remain; never enable auto-merge

- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Bring the local branch `worktree-DEV-7464` (in its own worktree) to its upstream's head, and keep that worktree clean while the stack is open | Balduin | before start | The executor works in one worktree, that branch is checked out, locked, in another, and `gh stack` replays a branch only in a clean worktree that owns it |
| H2 | In the incubator worktree as Phase 3 leaves it, run CPE and look at an Incunabula record page with one image, a book page with sequence paging, a restricted image and the zoom controls; tick the line in PR A's description. The stylesheets seen there are still the incubator's | Balduin | before Phase 5 (Why mid-plan: it needs the state Phase 3 builds and Phase 5 discards, and PR A must not merge before the viewer has been seen working) | The viewer's JS cannot be exercised from an automated browser (the source README says so) |
| H3 | After H2: merge #470, then PR A, then PR B, bottom-up (`gh stack merge`, or `gh stack sync` after each merge), each with green CI after the one below it merged | Balduin | before Phase 5 (Why mid-plan: the vendor pin must be a commit on `main`, which a rebase-merge mints only on merge, and the incubator cannot delete `cpe/components` before that commit exists) | Merging is the owner's decision |
| H4 | After #470 has merged: merge DEV-7464's incubator PR (the vendored `cpe-core`) | Balduin | before Phase 5 (Why mid-plan: it can only be opened after #470 has merged, and this plan's re-vendor builds on its `CRATES` and tooling changes) | Merging is the owner's decision |
| H5 | Merge PR C | Balduin | after ship | Merging is the owner's decision |
| H6 | From the start of execution until H5: hold incubator PRs that change `cpe/components` or `cpe/static/cpe/*.css`, or land their change in dsp-repository first. DEV-7464's incubator PR is the exception: its comment repoints in the stylesheets are expected | Balduin | before start | Which PRs merge is the owner's decision |
| H7 | Create the follow-up ticket for a CPE playground, after DEV-7465 | Balduin | after ship | A backlog decision |
| H8 | Decide whether the recorded debt becomes a Linear issue | Balduin | after ship | A backlog decision |
| H9 | Point the Claude Design project's CSS mirror (its `github.md` source path) at dsp-repository's `areas/access/cpe/components/style` | Balduin | after ship | The design project's configuration is outside both repositories |

## Acceptance Criteria

### dsp-repository

- [ ] `areas/access/cpe/image-viewer` is a workspace member named `cpe-image-viewer` that depends on
      `maud` alone, has no `build.rs`, and serves 44 files through `asset()`
- [ ] A `Viewer` renders the markup the incubator's crate renders at `SRC` for every builder that is
      left, `engine="osd-wc"` included
- [ ] No engine but `osd-wc`, no `MapView`, `LayeredView`, `Modality` or `Strategy` is left in the
      crate, in Rust or in JS
- [ ] The 41 vendored OpenSeadragon files match `VENDOR.lock`, and their licence text is at
      `assets/LICENSE-openseadragon.txt`
- [ ] The crate's README says it is an interim, cut-down copy, of which commit, and what was removed
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` and `cargo doc --no-deps` print no
      warning for either crate
- [ ] `areas/access/cpe/components` is a workspace member named `cpe-components`; its test list equals
      the incubator's at `SRC`, and all 980 pass
- [ ] The 147 snapshot files at the Phase 2 commit are byte-identical to the incubator's at `SRC`, and
      the seven stylesheets differ from it in seven comment lines only
- [ ] No test of `cpe-components` reads a file outside the crate
- [ ] The Phase 2 commit differs from the formatted baseline only by the edits its table lists
- [ ] Between the Phase 2 and the Phase 3 commit, only comments changed under
      `areas/access/cpe/components`, and the snapshots did not
- [ ] No `ADR-` followed by a digit is left in either crate; `just check-adr-refs` passes
- [ ] `just check` fails when a crate outside `areas/access/cpe/` depends on a CPE crate other than
      `cpe-ports`, when a CPE crate depends on a Mosaic crate, and when `cpe-core`, `cpe-ports` or
      `cpe-image-viewer` depends on a CPE crate; `check-cpe-seam.test.sh` runs in `just test`
- [ ] `CONVENTIONS.md`, `docs/src/git-conventions.md`, `docs/src/repo_structure.md`,
      `docs/src/workflows.md`, the three `CONTEXT.md` files and `ARCH-MAP.md` describe a CPE capability
      that holds four crates and a checked seam; `eng.yaml` names reviewers for both new crates
- [ ] No comment in `cpe-core` names the stylesheets or the component crate as incubator files
- [ ] A new accepted root ADR replaces the churn rule and gives the owner's reasons; `just check-adr-refs`
      resolves every citation of it
- [ ] ADR-0007 carries the amendment of 2026-10-11 (DEV-7401): the pointer to the new record, the
      viewer's interim packaging, the stylesheets' location, and the seam as checked
- [ ] `docs/specs/README.md` admits one plan for a stack of PRs
- [ ] `just check` and `just test` pass on each of the four code commits
- [ ] PR A has two commits and PR B three, both are drafts with `allow-many-commits` ticked, and
      `just commit-lint` passes for each

### dsp-incubator

- [ ] `cpe/components` and the seven stylesheets under `cpe/static/cpe/` no longer exist;
      `cpe-components` and `cpe-image-viewer` are workspace members under `vendor/dsp-repository/`
- [ ] No crate under `cpe/` depends on `dasch-image-viewer`; `shared/dasch-image-viewer` and
      `image-viewer-showcase` are unchanged apart from the README note
- [ ] `just cpe vendor-diff` prints `vendor-diff: empty (<pin>)`, and `<pin>` is a commit on
      dsp-repository's `main`
- [ ] The workspace's test names are the ones before the change plus the tests of `cpe-image-viewer`
      and the new router test, and all pass
- [ ] The binary serves the seven stylesheets at the URLs it served them at before
- [ ] `just cpe ci` passes

## Dependencies & Risks

- **Blocked by** DEV-7463 (merged) and DEV-7464 (#470, open). **Blocks** DEV-7465.
- **#470 changes under review.** Each fix there rebases this stack. Accepted with decision 1.
- **The cut-down JS is wrong in a way no test here sees.** `tests/render.rs` checks strings. The
  incubator's tests in Phase 3 check markup, not behaviour. H2 is the only check of behaviour, and it
  comes before PR A merges. A defect found after that is a dsp-repository PR and a new pin.
- **`maudfmt` breaks code it reformats.** Expected at a few sites. The fix is a binding; if a site
  cannot be fixed without changing rendered output, the snapshots show it and the executor stops.
- **A formatted `html!` renders differently.** Not expected: formatting changes no token. The 147
  snapshots, unchanged, are the check.
- **cargo cannot resolve the uncommitted check.** Then the incubator first meets the moved crates in
  Phase 5, after the merge. Reported in PR B's description so that the owner can weigh H3.
- **A change lands in `cpe/components` or the stylesheets between `SRC` and Phase 5.** Phase 5 finds it
  and stops. H6 keeps the window empty.
- **The two viewers drift.** By design (decision 3). Each viewer fix is applied twice until the
  incubator's viewer replaces this crate.
- **The stack cannot be adopted** because the local `worktree-DEV-7464` is stale. H1; Phase 4 stops and
  reports.
- **Phase 3 deletes a comment that carried a contract, or changes a quoted selector.** The rules keep
  any sentence that states a contract; the phase review and one checkbox read the diff for both.
- **The seam check blocks DEV-7465.** `access-server` will depend on a CPE crate. The exception list is
  one list of paths, and the script's header, `ARCH-MAP.md` and ADR-0007 say who extends it.
- **Recorded debt**, named in the PR descriptions:
  - 980 test names in `cpe-components` that do not follow `test_{what}_{condition}_{expected}`;
    renaming them renames snapshot files and breaks the test-list proof.
  - Long comments, and comments that restate the code.
  - Four fixtures frozen at `SRC`.
  - The viewer's unused `layers` mode, and its vendored files outside `just verify-checksums`.
  - The guard that ties `cpe-core`'s header budget to `tokens.css` lives in the incubator, although
    both files are now here.
  - No browser test of any component in this repository.
  - `check-composition-root-deps.sh`, which guards `access-server`, has the matcher gaps the seam check
    avoids: a `[dev-dependencies.name]` or target-specific header, a dotted key, and a quoted or
    indented key all pass it. Found during plan review; not changed here.

## Success Metrics

Counts at `1cd5781f`, taken again where `SRC` differs:

| | Source | After this plan |
|---|--------|-----------------|
| `cpe-components`: `.rs` files / lines | 26 / 39,748 | 26 files |
| `cpe-components`: tests | 980 | 980, same names |
| `cpe-components`: snapshot files | 147 | 147, byte-identical |
| Stylesheets | 7 (5,603 lines) | 7, every rule byte-identical |
| Viewer: tests in `tests/render.rs` | 47 | 26 (47 − 22 + 1) |
| Viewer: doc tests | 4 | 2 |
| Viewer: asset files | 43 after the removals | 44 with the licence |
| Viewer: `src/lib.rs` lines | 871 | fewer; no line added outside the edit table |
| Snapshots containing `<image-viewer` | 12 | 12, unchanged |

`just check` and `just test` pass on every code commit. PR C changes the incubator's test names only by
adding the viewer's.

The viewer's edit table was applied to a scratch copy during plan review (2026-10-11, source at
`1cd5781f`, a standalone manifest, a placeholder licence file): 26 tests and 2 doc tests passed, clippy
with `--all-targets` and `cargo doc` printed no warning, `node --check` accepted the JS, and the builder
matrix rendered identical markup with the old and the new crate in all 26 cases. The table's rows above
include what that run found missing.

## References

- Model: `docs/specs/2026-10-11-move-cpe-core/01-feat-move-cpe-core-plan.md` and its journal (the
  deviations and side findings there shaped *Technical Considerations* here)
- `docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md` (the seam rule, the enforcement
  paragraph, the amendments of 2026-10-02 and 2026-10-11)
- `docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md`
- `.github/scripts/check-composition-root-deps.sh` and its test script (the seam check's template)
- `docs/src/git-conventions.md` → *One PR, several commits, or a stack*
- `docs/specs/README.md` → *Plans and PRs*
- Source at `1cd5781f`: dsp-incubator `cpe/components`, `cpe/static/cpe`, `shared/dasch-image-viewer`,
  `cpe/vendor/README.md`, `cpe/docs/INCUNABULA_GO_LIVE.md` (phase 1b)
- Learnings: `docs/learnings/configuration-errors/rustfmt-nightly-drift-between-flake-lock-and-ci.md`,
  `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`,
  `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`,
  `dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md`,
  `dasch-specs/learnings/best-practices/deferred-work-lost-across-phase-handoffs.md`
- #470 (DEV-7464), DEV-7401, DEV-7463, DEV-7465
