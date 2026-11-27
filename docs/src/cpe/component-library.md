# CPE Component Library

CPE's pages are built from its own component library, `cpe-components` in `areas/access/cpe/components/`: Maud
functions that render what `cpe-core`'s IR describes, and the stylesheets their markup is written against. It is
not Mosaic. Where it sits among CPE's crates is on [CPE Architecture](./architecture.md).

## The Components

The generic components are the modules of `component`: the editorial and narrative renderers (`editorial`,
`narrative`, `markdown`), the resource list with its facet panel and search bar (`resource_list`, `facet_panel`,
`search_bar`), the record with its media viewer (`resource_detail`, `media`), the compound viewer, which is the book
reader, with its page picker (`compound_viewer`, `page_picker`), the annotation register (`annotation`), the crumb line
and its data links (`breadcrumb`, `data_links`, `data_entries`), and the value and link renderers (`values`, `links`).
Each takes the data `cpe-store` fetched for it and a `context::RenderContext`, reads nothing itself and renders
synchronously. `component::dom` holds the DOM ids and DataStar expressions more than one component writes or reads.

The strings of the interface itself, in each of CPE's languages, are `chrome::ChromeMsg`: an engine string that varies
by language is added there, not as a local helper in a component. What a project's KDL declares (`chrome {}`,
`labels {}`) is the project's.

A component one project needs and no other lives under `component::project::<project>` and is found through a row of
`PROJECT_RENDERERS`, keyed by project id and name. Its validation half is `cpe-core`'s
`validate/project_components.rs`, which keeps the two tables in step. Incunabula's are `showcase` and `timeline`.

No playground shows the components. Their markup is pinned by the crate's insta snapshots and `cpe-web`'s, and what a
browser does with it, with and without JavaScript, by the Playwright suite in `areas/access/cpe/web-e2e-tests/`
(`just test-e2e-cpe`; `just test-a11y-cpe` for axe alone).

## The Stylesheets

The seven stylesheets are plain CSS in `components/style/`, written by hand on custom properties, with no Tailwind
and no build step. `cpe-server` compiles them into the binary and serves them under `_static/cpe/`; `cpe-web`'s
`document_head` links them in this order:

| File | What it holds |
|------|---------------|
| `tokens.css` | The design tokens: the primitive scales, the semantic roles the components use, and the few a project may set (accent, typeface) with the light and dark appearance |
| `shell.css` | Base typography and the page shell: header, header menu, main, footer, the language switcher, the skip link |
| `editorial.css` | The narrative pages and the overview |
| `record.css` | The record, the compound viewer and its contents rail |
| `grid.css` | The layout grid and the layout contract below |
| `media.css` | The record viewer and the image strip, and `.pg`, the one-at-a-time pager |
| `incunabula.css` | Incunabula's per-project components, one block each |

`grid.css` owns layout and alignment; the others keep typography and ornament. A stylesheet change is not caught by a
snapshot: `cpe-web`'s tests read four of the files to hold a few breakpoints and class names to the Rust code, and the
Playwright suite and a look in the browser cover the rest.

## The Layout Contract

The header, the crumb line, the page and the footers share one frame: `max-width: var(--shell-max)` (137.5rem, the
gutters included), centred, with `padding-inline: var(--gutter)`. The frame fills the window up to 2200px, and no
breakpoint changes it. Inside it, `.page` is a 12-column grid; a child that is not placed spans all twelve.

- **Every top-level component root carries `.page`**, on every branch, the empty and not-found ones included
  (`article.record.page`, `section.resource-list-section.page`, `article.compound-viewer.page`). A root without it
  is not centred and its content touches both edges of the window.
- **A single reading column adds `display: block`** to its `.page`, as `.editorial.page` and `.compound-viewer.page`
  do, and caps its text at `--read`.
- **A figure block carries no `.page`**: a component that renders only inside a narrative section (`div.cases`,
  `ul.editions`, `ul.strips`) is centred by that narrative's `article.editorial.page`. Its required `section=`
  (`require_section` in `cpe-core`'s `validate/showcase.rs`) keeps it out of an unslotted render.
- **A constrained dimension never grows in a narrower media query.** A `max-width` or `max-height` holds or shrinks as
  the window narrows.

The contents rail of the book reader (`.cv-toc` in `record.css`) is a grid of `minmax(12rem, 16rem) minmax(0, 1fr)`:
`minmax(0, …)` lets the reader shrink below its widest content, so the page never scrolls sideways. At 860px and below
it is one column, with the rail first in the markup and shown under the reader. When a rail follows a narrative intro, the
intro starts at the frame's edge with the rail (`editorial.css`). `rail-layout.spec.ts` measures both at eight widths.

## The Seam to Mosaic

Mosaic (`mosaic-tiles`) is the design system DPE and the editor use. CPE does not use it: its editorial register is
its own (ADR-0007). No CPE crate depends on a Mosaic crate, and no `modules/mosaic/` target grants a CPE package, so
such an edge fails Bazel's analysis. A component CPE needs is written here, in `cpe-components`, even where a Mosaic
tile looks close.

## JavaScript

CPE's pages work without JavaScript; with it, DataStar (`public/vendor/datastar.js`) adds the fragment swaps and the
interactions. The rule for JavaScript in the Rust code:

- **No inline `<script>`, and no `ExecuteScript`.** A page's behaviour is in `data-*` attributes and static files.
- **A short expression stays in its `data-*` attribute.** A DataStar action (`@get`) is built by `shared-datastar`;
  `just check-datastar-attributes` fails on one spelled out in a template.
- **A long one goes to `public/cpe.js`**, CPE's own script, as a function on `window.cpe`. A function there sees
  neither DataStar's signals nor its actions: it takes `el` or `evt`, and the ids it needs, and returns values; the
  attribute keeps the signal writes and the action. Today it holds the annotation arrival (`cpe.arrival`), the page
  picker (`cpe.pickerToggle`, `cpe.pickerTarget`) and the reader's arrow keys (`cpe.pageKey`). Its header lists the
  classes and attributes it reads, which the Rust markup writes; `cpe_js_header_selectors_occur_in_a_reader_page` in
  `cpe-web` holds the list to a rendered reader page. `document_head` loads it with `defer` before
  DataStar's module script, so `window.cpe` exists when a `data-init` runs. Biome lints it (`just lint-e2e`).
- **The listed exceptions stay inline**, short behaviour DataStar cannot express: the clipboard copy
  (`breadcrumb.rs`, `data_links.rs`), Escape, outside-click and focus-out on disclosures (`dom.rs`), clearing the
  search field (`search_bar.rs`, `facet_panel.rs`), the header menu and the crop-error marks (`web/src/page.rs`,
  `dom.rs`, `showcase.rs`), and the order control (`resource_list.rs`).

Two places do not follow the rule yet: the viewer controls in `media.rs` call into the image viewer from their
attributes, and `cpe-server`'s `router.rs` sends an `ExecuteScript` with the compound viewer's fragment, which waits
for #582.

Rendered Markdown, and nothing else, reaches the markup through `PreEscaped`. The number of call sites per file is pinned by
`pre_escaped_call_sites_match_the_pinned_table` in `components/src/lib.rs`, so a new one is a reviewed change to
that table.
