# AI Agent Guide for CPE

CPE-specific guidance; project-wide guidance is in the top-level `CLAUDE.md`, and the host, `access-server`, has its
own in `areas/access/server/CLAUDE.md`. CPE's vocabulary is `CONTEXT.md` in this directory.

## Where to read first

- **The crates, the ports, activation and the request path**: `docs/src/cpe/architecture.md`
- **The stylesheets, the layout contract and the JavaScript rule**: `docs/src/cpe/component-library.md`
- **What a project's `config.kdl` may say**: `docs/src/cpe/config-language.md`
- **What is served on a deployment, the log lines and `access-server validate`**: `docs/src/cpe/operations.md`
- **The decisions**: ADR-0007, ADR-0008, ADR-0012 and ADR-0013 in `docs/adr/`. CPE keeps no decision series of its own.
  A citation `dsp-incubator cpe/docs/adr/<file name>` names a record of the prototype, in that repository; its numbers
  are not this repository's.

## Adding a project

A project is a folder, its shipped inputs and, where its mapping cannot say something, a hook. Nothing else in the
code names it.

1. **The project folder**: `projects/<project id>/`, named by the project id (kebab-case, the URL segment), with a
   one-line `SHORTCODE` file holding the archive's shortcode, and `assets/` for the files its pages show from the
   folder (a cover image). It holds no code and no configuration; documents about the project's data may sit beside
   them, as Incunabula's do.
2. **The shipped inputs**: `areas/deposit/vre-bridge/inputs/<shortcode>/` holds `config.kdl`, `curation.csv` when the
   `remodel {}` block reads a curated key, and `narrative/<name>`. These are the authoritative copies; the VRE bridge
   copies them into the project's `media` generations, where CPE reads them through the bytes port. Write the KDL
   against `docs/src/cpe/config-language.md`. `areas/access/cpe/store-build/README.md`, *Adding a project*, has the
   steps for the `remodel {}` block, the image classes and the annotations, and for the two hand-kept lists in
   `store-build/tests/it/hygiene.rs`, which a project with a hook adds to differently from one without.
3. **A hook, only where one is needed**: a rule the block cannot say (a condition written as code, a rule table, a fact
   about named resources) goes in a module `store-build/src/projects/<module>/mod.rs`, a row of `HOOKS` in
   `store-build/src/hooks.rs` keyed by the project id, and a `hook` node in the block that declares what it reads and
   changes. `build` refuses a hook without a declaration and the reverse, and `tests/it/hooks.rs` holds rows and
   blocks to each other. The README's *A project with a hook* has every step. A rule other projects would need is a node
   of the grammar instead, in `cpe-core` and the builder.
4. **Check it**: `just validate-cpe`. With shipped inputs alone it runs the data-free check (the KDL parses, the block
   loads and declares the folder's shortcode, the narrative it names is there); a project with committed stand-in data
   gets the full load. CI runs it after `just test`.

Merging the folder serves the project nowhere. It is released on a deployment by adding its shortcode to that
deployment's `EDITOR_VRE_BRIDGE_ALLOWLIST` in ops-deploy (`docs/src/cpe/operations.md`, *Releasing a project on a
deployment*). Until the bridge has written its data, the project has no Ready token there. While `sync`'s location
holds other projects, that makes it an unknown project, whose paths answer 404. An **Unavailable project** failed to
load and answers 503, and so does every project when `sync`'s location is missing or empty
(`docs/src/cpe/architecture.md`, *Activation and a Project's Statuses*).

Incunabula (0803) is the one project with committed stand-ins: `areas/access/sync/data/0803/` and
`areas/access/media/data/0803/`. The `media` stand-in's generation holds a copy of its shipped inputs, and
`just check-stand-in-fixture` (run by `just check`) fails when the two differ: an edit to Incunabula's inputs is made
in both places.

## Running the checks

- **While iterating**: `bazel test //areas/access/cpe/...`, or one crate's package.
- **`just validate-cpe`**: after changing a project folder, its shipped inputs or its stand-in.
- **`just test-e2e-cpe`**: the Playwright suite in `web-e2e-tests/`, on Incunabula, with and without JavaScript, axe
  included, against a release build (several minutes the first time). `just test-a11y-cpe` runs axe alone;
  `a11y-cpe.yml` runs the suite in CI. It aborts requests to other origins and runs with reduced motion. Incunabula has
  no facet panel and no search bar, so those components are covered by snapshots only.
- **`just lint-e2e`**: Biome over the suite and over `public/cpe.js`.
- **Before committing**: `just check` and `just test`, as `CONVENTIONS.md` says.

## When making changes

- **A change to the config language** changes the code and `docs/src/cpe/config-language.md` in one commit.
  `store-build/tests/it/worked_example.rs` holds the document's worked example of the `remodel` block equal to what
  Incunabula's `config.kdl` lowers to, and `cpe-store-build`'s tests read the document by path.
- **A new validation rule** gets a fixture project under `web/tests/projects/`, which `web/tests/validation.rs` loads.
- **A changed snapshot** is accepted with `just snap-accept //areas/access/cpe/<crate>:<test target>` and reviewed
  with `git diff`, never by renaming a `.snap.new` file: Bazel runs the tests with `INSTA_UPDATE=no`
  (`docs/src/bazel.md`).
- **A new file in `components/style/` or `public/`** needs a row in `server/src/static_files.rs`'s table;
  `every_file_of_the_three_folders_has_a_row` fails until it has one. A changed `public/vendor/datastar.js` needs its
  checksum in `public/vendor/README.md` (`just verify-checksums`).
- **A new `PreEscaped(` call site** fails `pre_escaped_call_sites_match_the_pinned_table` in `components/src/lib.rs`
  until the table is raised, which is the point: the site gets a review.
- **A test that reads a committed file** reads it by a path relative to the test file. Under Bazel,
  `CARGO_MANIFEST_DIR` is a relative path, and the activation's readers refuse the symbolic links runfiles are, so
  `cpe-server`'s tests read the stand-ins from a `copy_to_directory` (`ARCH-MAP.md`, `areas/access/cpe`).
- **`cpe-server`'s `pub` surface** names no type of another CPE crate but `cpe-ports`', apart from the `hooks`
  parameter of the hidden `*_with_hooks` methods: `no_pub_item_names_another_cpe_crate` and
  `the_boundary_exception_is_the_hooks_parameter_alone` in `server/src/lib.rs` hold it.

## JavaScript in the Rust code

No inline `<script>` and no `ExecuteScript`. A short expression stays in its `data-*` attribute; a long one goes to
`public/cpe.js` as a function on `window.cpe` that takes `el` or `evt` and returns values, while the attribute keeps
the signal writes and the `@get`, which `shared-datastar` builds. The listed exceptions stay inline: the clipboard copy
(`breadcrumb.rs`, `data_links.rs`), Escape, outside-click and focus-out on disclosures (`dom.rs`), clearing the search
field (`search_bar.rs`, `facet_panel.rs`), the header menu and the crop-error marks (`web/src/page.rs`, `dom.rs`,
`showcase.rs`), and the order control (`resource_list.rs`). Two places do not follow the rule yet: the viewer controls
in `media.rs`, and the `ExecuteScript` in `server/src/router.rs`, which waits for #582.
`docs/src/cpe/component-library.md`, *JavaScript*, has the detail; `CONTEXT.md`, *Inline expression*, the term.

A class or attribute `cpe.js` reads is listed in its header, one selector per line. `cpe-web`'s
`cpe_js_header_selectors_occur_in_a_reader_page` renders a reader page and fails when a listed selector is missing
from the markup, so a renamed class fails `just test`; a new selector the script reads is added to the header.
`cpe/cpe.js` is answered with `Cache-Control: no-cache` and an `ETag`, so a browser never pairs it with HTML of
another build.

## Common pitfalls

### DataStar expressions: `el`, not `$el`

DataStar compiles an expression with the parameters `el`, `$`, `__action` and `evt`. `$name` reads the signal `name`,
so `$el` and `$evt` are `undefined` and raise no error. A snapshot asserts the expression's text and never runs it:
check a new expression in a browser or in the Playwright suite.

### A signal is not reset by re-rendering the element that declares it

`data-signals` declares a signal and does not reset one that exists. A fragment swap that must clear one sends a
`PatchSignals` event beside its `PatchElements`: `router.rs`'s `copied_signal_reset_event` resets every receipt of the
crumb line (`copied`, `_dataCopied`), and a new receipt signal is added to it.

### A swapped region derives its state from the data

The `_facets` swap replaces the facet panel's `<details>`; its `open` is computed from whether a facet is selected, so
the region comes back as the visitor left it. A disclosure or other stateful element inside a swapped region is
rendered from the data the same way, never with a fixed default.

### The `.page` class on every component root

A component's root element carries `.page` on every branch, the empty one included, or its content touches both edges
of the window. Figure blocks inside a narrative (`div.cases`, `ul.editions`, `ul.strips`) are the exception. The rule
is at the top of `components/style/grid.css`.

### `hidden` loses to an explicit `display`

A class rule that sets `display` beats the user agent's `[hidden] { display: none }`, so the element shows, to
assistive technology too. Pair such a selector with its own `[hidden]` rule. Where the hidden element must keep its
space, as each `.rv__legal-plate` in the record viewer's one stacked cell does, that rule is `visibility: hidden`;
changing it to `display: none` makes the viewer resize while paging, and no test sees it (`components/style/media.css`).

### A thumbnail's box and its fetch size move together

The IIIF fetch sizes are constants in `core/src/iiif.rs` (`THUMB_WIDTH`, `PLATE_WIDTH`, `STRIP_THUMB_WIDTH`, the `*_FETCH_PX`), each
naming the CSS box it fills. Enlarging a box without its constant shows as blur on a high-density screen only.

### The header menu's breakpoint has three readers

`#header-menu` is a `popover="auto"` that `shell.css` shows in flow from 79rem. `HEADER_DESK_BREAKPOINT_PX` in
`core/src/markup_contract.rs` feeds `web/src/page.rs`'s `header_menu_resize_expr` and the desk-row budget in
`core/src/validate/header_budget.rs`; `shell.css`'s two media queries are literals that
`header_desk_breakpoint_matches_shell_css` holds to it. Move the three together. The bare `#header-menu` rule and its
`#header-menu:popover-open` twin stay separate: a selector list holding `:popover-open` is dropped whole where that
pseudo-class is unknown.

### An engine string is read against every project's nouns

A string of `chrome::ChromeMsg` that composes a class noun ("{t} von {m} {noun}") is checked against every declared
class noun of every project, in every language: German case endings make a phrase right for one project's nouns and
wrong for another's.
