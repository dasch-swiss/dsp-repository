# CPE joins the Access Area as its second capability

CPE, the Configurable Presentation Environment, presents one project's archived data in a design and a
structure made for that project: Incunabula first, and further projects after it. It began as a prototype in
`dasch-swiss/dsp-incubator` (`cpe/`), where new projects are still prototyped. What it keeps of the prototype is
the looks and the behaviour a visitor sees and KDL as the authoring language; its code has the anatomy of this
repository. The decision, phrased so a violation is describable in code:

**Place in the area**

- **CPE is the second capability of the Access Area's modulith (ADR-0003), at `areas/access/cpe/`.** There is
  one Access-Area binary, `access-server` in `areas/access/server`, which holds wiring only and mounts DPE's and
  CPE's routers on the area's origin; `dpe-server` is DPE's library crate (`DpeConfig`, `Dpe`, `validate`). A
  project's presentation is a route prefix inside CPE's router, never a deployment of its own.
- **`cpe-server` is CPE's host crate and its facade.** It holds CPE's configuration, router and activation
  behind one type, `Cpe`, in the shape of `dpe-server`'s `Dpe`. No type of another CPE crate appears in its
  `pub` items, `cpe-ports`' types excepted, so a composition root depends on `cpe-server` and `cpe-ports` and on
  nothing else of CPE; it never names `cpe-store`. `Cpe::projects()` and `ProjectSummary` are there for a host
  that wants the list of projects itself.
- **CPE has three crate roles that ADR-0003's anatomy does not list.** `cpe-store-build` is the library that
  turns the port's snapshot and a project's `remodel {}` block into the rows of CPE's store; it depends on
  `cpe-core` and `cpe-ports` and not on `cpe-store`, whose schema it includes as a file. `cpe-components` is
  CPE's component library, and `cpe-image-viewer` its image viewer (both below).

**The port and the store**

- **CPE owns its own read-side store, built from `sync`, and never opens the archive projection itself.** The
  store is a derived read model under ADR-0008, CPE's own shape over the projection's facts, and its first
  instance. `cpe-ports` declares what CPE needs from the projection; `sync-store`, beside the data of the
  Access Area's single writer of the projection, implements it, and `FakeArchiveProjection` is the second
  adapter, in CPE's tests. The store is disposable and is rebuilt from what its ports serve; nothing else
  writes it. `sync` is at its minimum: the committed projection under `SYNC_DATA_DIR`, read from disk, behind
  the port that Chischtli's projection stream and its replay are to stand behind.
- **The port speaks archive-shaped facts as the archive records them** (classes, typed values, links, files,
  ordered membership), never CPE's presentation model. The facts follow the interim format that `dao-lift`
  (`dsp-incubator cpe/tools/dao-lift`) produces from a VRE dump, one N-Quads file per project: DAO wherever
  DAO has decided, with each deviation and its reason listed in `dao-lift`'s `FORMAT.md`. That file stays in
  the incubator, so the format is visibly neither DAO nor a commitment of this repository, and
  `areas/access/sync/data/PROVENANCE` pins the `dao-lift` commit whose `FORMAT.md` the committed file follows.
  Values are named by their source property, the project property they were recorded under, which is also
  what a project's KDL names. `sync-store` refuses the whole snapshot (`Unavailable`) when the file breaks a
  fact the port serves, and omits only what `FORMAT.md` lets a reader omit and the port does not carry, so CPE
  never receives a partial or repaired snapshot. When DAO is published, `dao-lift` and `FORMAT.md` move to it,
  and each deviation is resolved either upstream in DAO or by a change to the port.
- **A project's store is built at activation, from SQL text.** `Cpe::activate` is given the two ports. For
  each project it reads the KDL through the bytes port, lowers the `remodel {}` block, asks the archive
  projection port once for the snapshot of the block's shortcode, builds the store's rows with
  `cpe-store-build` and the project's hook, renders them as SQL text, and loads the project from that text as
  it would from a file. No store is committed, embedded or kept: it is built again at every start (ADR-0008).
  Rendering text and parsing it again costs a parse; in exchange the text is one comparable artefact.
- **The SQL text starts with a header that names where each input came from**, and ends it at the first blank
  line: the facts from the archive projection's snapshot of the shortcode, the remodel block from the bytes
  port as `<shortcode>/config.kdl`, the curation as `<shortcode>/curation.csv` when the block reads a curated
  key, and, for a project with a hook, that the hook comes from no port. Below the header, Incunabula's text
  is byte for byte the prototype's, and is pinned by its length and SHA-256; the whole text, header
  included, is pinned too.

**Remodelling**

- **Per-project remodelling is declarative first, a typed hook second, never a script.** The mapping from
  archive shape to CPE's presentation model (class and property names and types, link direction, the title
  property, which properties become list vocabularies) is declared in the `remodel {}` block of the project's
  KDL. What the mapping cannot say (composed titles, derived vocabularies such as a season or a decade, rule
  tables, image re-modelling) is a Rust hook: a module under `cpe-store-build`'s `src/projects/<project id>/`
  and one row of the const table `hooks::HOOKS`. The builder applies the mapping first and then gives the hook
  a narrow draft of the mapped resources (`cpe_store_build::hook::Draft`): the hook reads and changes what the
  block's `hook` node declares and never sees the snapshot, so the port's types stop at the builder's generic
  code. No embedded scripting language: ADR-0002 rejects untyped per-deployment configuration because no test
  covers it and no agent can trace it to an owner, and the authors of a CPE project are DaSCH developers.
- **A project's own components are code of the component library:** a module of `cpe-components`'
  `component::project` and one row of its registry `PROJECT_RENDERERS`.

**A project**

- **A project is its deposited files; its folder holds only the documents that describe its data.** A
  project's KDL, its narrative, its curation and its assets are files deposited for the project. CPE reads
  the KDL, the narrative and the curation at activation through its bytes port against `media` (ADR-0012), and
  links to the assets, which `media` serves when the request comes; an image is an IIIF URL the store builds
  (ADR-0012). No asset is consumed at activation, and no project content is compiled into any crate. Adding a
  project touches no shared registry unless it brings code, which is a `HOOKS` or `PROJECT_RENDERERS` row.
  `areas/access/cpe/projects/<project id>/` holds `SHORTCODE`, a one-line file that declares the project's
  Shortcode, and the documents that describe the project's data (`SHAPE.md`, `CURATION.md`, `PROVENANCE.md`,
  `DATA_ISSUES.md`), no code and no `data.sql`. The prototype's design documents stay in the incubator.
- **A project is discovered by its folder's `SHORTCODE`.** Activation refuses a project whose KDL declares
  another shortcode, and one whose shortcode another folder declares too.
- **The bytes port is built at the minimum ADR-0012 allows.** It serves every file the stand-in holds and
  refuses none for want of a right; the stand-in holds public files only, and CPE serves every request as the
  anonymous consumer. Activation runs once per start, so a file edited under `MEDIA_DATA_DIR` reaches CPE only
  when the Access binary restarts.
- **The project id names a project in CPE**: its URL segment and the key its hook and components are
  registered by. It is neither the project's shortname nor its Shortcode (`incunabula`, not `0803`).
- **Incunabula is the first project; the incubator's others move in one at a time**, each by the incubator
  lane below.

**A project fails alone**

- **Validation happens when a project is activated, not when the process boots.** `Cpe::new` reads nothing
  and cannot fail, and `activate` returns nothing and never panics, so CPE cannot stop or delay the area
  binary. A project that does not activate (its configuration, the projection, the build, validation, or a
  panic in any of them) is logged with its id and is offline on its own prefix: every path under it is
  answered 503 with `Retry-After` and `Cache-Control: no-store`, while DPE and every other project keep
  serving. Until `activate` has finished, every CPE path is answered that way.
- **"Not ready" is kept apart from "not there".** 404 is for a path that names no project. With no projects
  at all, the overview lists none and every other path is not found; with `sync`'s data missing, every
  project answers 503.
- **A strict mode for development, CI and tests fails the whole start on the first invalid project**, as the
  prototype's `--check` does. It is the same verb as DPE's `validate`, on the area binary. The page a project
  answers while offline is a designed page.

**The component library and the seam**

- **CPE has its own component library, inside the capability.** `cpe-components` holds CPE's design language
  (the token CSS and the components), because its editorial register is not Mosaic's and is not meant to be.
  Mosaic stays a root peer because the Deposit and Access Areas both use it. The stylesheets are plain files in
  `cpe-components`' `style/` directory; the crate neither embeds nor serves them, and `cpe-server` serves them,
  with the files of `areas/access/cpe/public/`, from a compiled-in table.
- **The seam is one-directional and mechanical.** `.github/scripts/check-cpe-seam.sh` checks, over the tracked
  `Cargo.toml` files:
  1. No crate outside `areas/access/cpe/` depends on a CPE crate except `cpe-ports`, which `sync-store` and
     `media-store` implement, and `cpe-server`, which the composition root mounts (`PORT_EXCEPTIONS`).
  2. No manifest under `areas/access/cpe/` depends on a Mosaic crate (any crate of `modules/mosaic/`).
  3. `cpe-core`, `cpe-ports` and `cpe-image-viewer` depend on no CPE crate (`LEAVES`); a new crate that must
     stay a leaf is added there.
  4. No CPE manifest names a crate of `areas/access/sync/` or `areas/access/media/` outside a
     dev-dependencies table (ADR-0008, ADR-0012).
  5. `cpe-store` depends on no CPE crate but `cpe-core`, `cpe-store-build` on none but `cpe-core` and
     `cpe-ports`, and `cpe-server` on every CPE crate but `cpe-store`, which it reaches through `cpe-web`
     (`CPE_EDGES`).

  Not checked by the script: that no CPE crate depends on DPE's crates or on `sync`'s store outside a
  dev-dependencies table, and that `cpe-components` depends on neither `axum` nor `rusqlite`.
- **The image viewer is `cpe-image-viewer`, at `areas/access/cpe/image-viewer`**: a copy of the incubator's
  `shared/dasch-image-viewer`, reduced to the one engine CPE uses (OpenSeadragon in a shadow root). It is not
  in `shared/`, because it has one consumer, and it is not part of the component library; a consumer outside
  the capability changes the seam check and this record together. Its files (its JavaScript, the vendored
  OpenSeadragon and its sprites) are compiled in through an enumerated `include_bytes!` table, and the
  workspace has no `include_dir`. Which engine is final is open: the incubator keeps developing its viewer,
  what it chooses replaces this crate, and a viewer fix CPE needs lands here and is applied by hand in the
  incubator.

**URLs**

- **CPE is mounted under a base, and a project lives under `<base>/<project id>/`.** The base has one source,
  `CPE_BASE` in `cpe-server`'s configuration: empty for a mount at the root of a host, else a path with a
  leading slash and no trailing one (`/cpe` in the image and the local run recipes). A value of another shape
  is refused, and CPE then answers 503 while its host keeps running. `Cpe::base()` returns it; CPE's router
  does not know its own prefix, and a composition root mounts it at `Cpe::base()` and writes no prefix of its
  own.
- **One module builds the URLs.** `cpe_core::url` joins the base, the project id and the two reserved
  segments, and nothing else does, so the segment's form can change without touching the routes. A handler
  sees the request path without the base; the page's own path, both redirects, the share links and the
  reader's history URLs are the base and that path. The `lang` and `appearance` cookies have `Path=<base>`
  (`/` at a root mount), so they reach every project of CPE and nothing else on the host.
- **A project is self-contained under its prefix, assets included.** The engine's shared files are at
  `<base>/<project id>/_static/…` and a project's own at `<base>/<project id>/_assets/…`; the same engine file
  is then fetched once per project, which content-hashed names and long cache lifetimes make negligible, and
  the layout holds on a project's own hostname too. A page without a project (the overview, CPE's not-found)
  loads the shared files from `<base>/_static/…`, which serves the same table. The 503 page loads no file:
  while it is the answer, every path under the base is answered by it. Whether a project's assets keep a path
  under its prefix once `media` serves them is not decided.
- **The base is the overview of the projects CPE serves.** `/cpe` and `/cpe/` are the same page, and neither
  redirects to the other. It has one teaser per loaded project (title, description, eyebrow, cover image and
  credit, all from the project's `site {}` node), each a link to `<base>/<project id>/`, in CPE's bare chrome;
  with no project loaded it says so and answers 200, and a project that did not activate is not on it.
  `cpe-web` renders it (`page::render_site_index`) and `cpe-server` routes it. A path under the base that names
  no project is CPE's not-found, which links to the overview.
- **A link in a project's content that starts with one `/` gets the base in front.** Narrative Markdown and
  its invite lists name pages as `/incunabula/books`, and the renderer writes `<base>/incunabula/books`. A
  destination that starts with `//`, has a scheme, is a fragment or is relative is written as it is, and so is
  a link in a property value, which is not narrative. The content is not changed, so it still names the
  project segment.
- **`/` and everything outside the base stay DPE's, including its 404 and its `ServeDir` fallback; every path
  under the base is CPE's to answer**, its not-found (a project's 404 under a known project, a plain CPE 404
  otherwise) and its 503 included. CPE's router is nested, never merged,
  because both routers end in a fallback. Axum's `nest` does not match `<base>/`, so the mount routes that one
  path to CPE's root. Under `nest` a leading `//` after the base collapses to one slash (`/cpe//evil.example/x`
  arrives as `/evil.example/x`); CPE's catch-all never redirects and never echoes the request path, and its
  open-redirect tests run both under a base and at a root mount.
- **CPE never stops or delays the area binary.** `access-server` builds `Cpe` from `CpeConfig`, constructs
  `sync-store`'s `LiveArchiveProjection` over `SYNC_DATA_DIR` and `media-store`'s `LiveDepositedFiles` over
  `MEDIA_DATA_DIR`, and calls `Cpe::activate` on the blocking pool
  after the listener is bound, so DPE, `/healthz` and the collector answer at every moment. With `CPE_BASE`
  unset or refused, CPE is not mounted, DPE is served alone and the binary logs why. A base that is a path the
  binary or DPE has a route for (`/healthz`, `/telemetry/collect`, `/dpe/projects`) is mounted: the route keeps
  answering, and CPE answers what no route does under it. A router that cannot be
  built with CPE at its base is built without it, with an error that names `CPE_BASE`. A CPE request is traced
  by the binary's layers, under its telemetry identity, in a span named by the route CPE reports
  (`cpe_server::MatchedRoute`), never by a path. The image carries `sync`'s data, `media`'s stand-in directory
  and the project folders and sets `SYNC_DATA_DIR`, `MEDIA_DATA_DIR`, `CPE_PROJECTS_DIR` and `CPE_BASE=/cpe`,
  so an image without a deployment's settings
  serves CPE; `CPE_PUBLIC_ORIGIN` is the deployment's, and without it CPE takes the origin from each request.
  The operating details are in `docs/src/cpe/operations.md` and `ARCH-MAP.md`.

**Persistent identifiers**

- **Data ARKs identify DSP data and keep resolving to it.** `ark:/72163/1/<shortcode>/<resource-id>` and its
  value ARKs name a data point, not a view of it. They resolve to the data (the VRE, at `app.dasch.swiss`) and
  are never re-pointed at CPE, whether or not a project is presented by CPE. Every `cpe_ports::Resource`
  carries its data ARK, and a CPE page links back to the data it presents through the crumb line's "Show in
  dataset" panel.
- **CPE's record pages are FAIR-assessable landing pages (ADR-0005)**, fed into `shared-fair`'s graphs by an
  adapter from CPE's store.
- **Presentation ARKs identify a CPE presentation view and resolve to CPE.** They are a separate kind from
  data ARKs. Their form and the resolver's mapping for them are not decided. No presentation ARK resolves to
  a CPE page until that page carries ADR-0005's metadata; on such a page `cite-as` is its presentation ARK, and
  CPE's share link may then copy it. No ARK of either kind resolves to CPE today.
- **No URL form for presentation ARKs is decided.** A uniform record route, if one is built, redirects to each
  project's own record pages and serves presentation ARKs, never data ARKs.

**Not decided here:** links between DPE's project page and a project's CPE presentation; browser telemetry
for CPE pages, which send no beacons (the `shared-telemetry` collector holds one meter scope and one path
normaliser per process, so it changes when they do).

**New projects and the incubator**

- **New projects are prototyped in the incubator, not here, and arrive with their engine changes first.** The
  engine and the design system are expected to change with every new project until they settle, and a
  prototype must be quick, must not touch the monorepo, and gets its preview link from the incubator's own CI.
  So `dsp-incubator` keeps a CPE project lane on a vendored, unchanged copy of this capability at a pinned
  commit of this repository; how that copy is built, patched and diffed is the incubator's decision
  (`0013-new-projects-are-prototyped-against-a-vendored-monorepo-cpe.md`).
- **What binds this repository:** the changes a prototype made to the copy land here as ordinary PRs before
  the project does, and a project moves in only from an empty diff. A change to a CPE crate is a PR here,
  followed by a pin move in the incubator that vendors the crate back with an empty `just vendor-diff`, also
  while the incubator's own work on that crate is busy. CPE's crates stay copyable: what they depend on inside
  this repository is a small, known closure.
- **No second engine is maintained.** The vendored copy takes the place of the prototype's own engine in the
  incubator.

**Carried over from the prototype**

- **Its decisions, cited by the file name of the record in `dsp-incubator/cpe/docs/adr/`, never by number**:
  KDL as the configuration language (`0006-config-language-kdl.md`); fragment swaps are transient and never
  write the URL (`0007-fragment-swaps-are-transient.md`); images are first-class, embedded-only Resources
  shared by links (`0008-images-are-embedded-only-resources.md`); IIIF presentation is CPE-native, with no
  Presentation manifests (`0009-iiif-presentation-cpe-native.md`); the component vocabulary, the empty-state
  family and the chrome catalog. The prototype's records stay in the incubator.
- **Interaction conventions are per capability.** DPE's tab fragment writes the URL and CPE's swaps never do;
  each is a rule of its capability, and this record does not unify them.
- **The config-language specification is `docs/src/cpe/config-language.md`**, so a change to the language
  changes the code and the document in one PR. CPE's glossary belongs in `areas/access/cpe/CONTEXT.md`.

## Considered Options

- **CPE as a capability of the Access Area's modulith (chosen).**
- **CPE as its own deployable, the prototype's shape extended** — rejected by ADR-0003: a read-side service of
  its own carries its own consumer of the archive's data products, which is the cost the modulith exists to
  avoid.
- **`sync` serving CPE's presentation shape directly** — rejected: the remodel would then live in `sync`, every
  project would edit `sync` to change how it is presented, and CPE's store would have a second writer.
- **Per-project remodelling in an embedded scripting language (Rhai, Roc)** — rejected: the ADR-0002 reason
  above; and Roc is pre-1.0 with a compiler mid-rewrite, a second toolchain inside the Bazel build ADR-0001
  introduces.
- **The hook as a file of the project's folder** — rejected: a function the builder calls has to be linked
  into the builder, which needs one place that names it, and Rust under `projects/` would need its own rules
  for the workspace, formatting, linting and unused dependencies.
- **One component library for DPE and CPE** — rejected: CPE's editorial design language is a deliberate
  difference, not drift; forcing it into Mosaic's tiles would flatten what a project-specific presentation is
  for.
- **A component ADR series for CPE** — not chosen: CPE is a capability of an area, not a root-level component,
  and its decisions are the area's (ADR-0006).
- **Prototyping new projects in the monorepo, with the incubator's prototype archived** — rejected: it loses a
  prototype that touches no monorepo branch and a preview link for free. The mechanisms weighed for the
  incubator's copy are the incubator's, in its record `0013-…`.
- **CPE merged into the area router at the root** — rejected: both routers end in a fallback, and CPE at the
  root would answer DPE's paths. This is why an empty base, which `cpe-server` accepts, is not mounted by
  `access-server`.
- **One shared `/cpe/_static/` for every project's pages** — rejected: it would not exist on a project's own
  hostname.
- **A project on its own hostname** — *deferred*: not built; it stays possible by mounting the same router at
  a different base, and comes back when a project asks for its own host.
- **A project-relative link form in narrative content** — *deferred*: it would let content stop naming the
  project segment; it comes back when a project's segment has to change.
- **A builder that writes SQLite directly** — *deferred*: it saves the parse of the SQL text; it comes back when
  that parse costs enough to matter.
- **`cpe-store-build` folded into `cpe-store`** — rejected: the per-project build code (hooks, remodelling) stays
  out of the crate that serves requests, and the builder stays a pure transform tested without a store.
  `check-cpe-seam.sh` rule 5 enforces the split.

## Consequences

- A project that brings code touches two shared registries, `hooks::HOOKS` in `cpe-store-build` and
  `PROJECT_RENDERERS` in `cpe-components`; a project without code is deposited files and a folder of documents.
- A change to a CPE crate is a cross-repository change: a PR here, then a pin move in the incubator. A change
  to the builder's output reaches the incubator's committed `data.sql` on its next re-vendor, which
  regenerates it with `just cpe store-build incunabula --inplace` (`dsp-incubator
  cpe/projects/incunabula/PROVENANCE.md`); its freshness test (`dsp-incubator
  cpe/store-build/tests/it/fresh.rs`) fails until then.
- `sync` is the first Access-Area capability without a screen.

Not yet in code: `sync` serves the committed projection from disk, not Chischtli's projection stream or its
replay (not scheduled).
Not yet in code: the projection follows `dao-lift`'s interim format, not published DAO; value order and text
language are raised upstream, the other deviations are not scheduled (DEV-7445).
Not yet in code: a project's `assets/` are in its folder of `CPE_PROJECTS_DIR`, read at activation and
served by `cpe-server` from memory, not by `media` at request time (not scheduled).
Not yet in code: CPE's images are served by DSP's IIIF server (`iiif-base` in a project's KDL), not by the
Access Area's `media` over Vitrinli (not scheduled).
Not yet in code: the bytes port and the port against `sync` serve no rights, and CPE's store and presenter
neither store nor filter on them; the target is each file and resource with its rights attached and CPE
deciding at request time (ADR-0012, ADR-0013; not scheduled).
Not yet in code: neither port announces change, and CPE does not rebuild the one project a port names; the
target is `media` and `sync` owning the trigger (ADR-0012; not scheduled).
Not yet in code: CPE serves Incunabula alone; the incubator's `alice-in-daschland` and `hdm` have not moved in
(not scheduled).
Not yet in code: `access-server validate` checks DPE's data only and fails on no CPE project, and the page of an
offline project is a plain 503 page, not a designed one (DEV-7403).
Not yet in code: CPE's record pages carry no ADR-0005 metadata, and no adapter feeds `shared-fair` from CPE's
store (DEV-7409).
Not yet in code: CPE's glossary is `dsp-incubator cpe/CONTEXT.md`, not `areas/access/cpe/CONTEXT.md` (not
scheduled).
Not yet in code: `dsp-incubator/cpe` keeps its own engine (`engine/`, `store/`, `store-build/`, `web/`, `bin/`)
and vendors only `cpe-components`, `cpe-core`, `cpe-image-viewer`, `cpe-ports` and `sync-store` (DEV-7408).

Enforced by: **static-analysis** — `check-cpe-seam.sh` and `check-composition-root-deps.sh`, run by
`just check`, for the seam and the composition root's dependencies, with Bazel `visibility` per ADR-0001 as
the target (**structure**); `cpe-server`'s tests for the facade (`no_pub_item_names_another_cpe_crate`), for
the 503 and 404 behaviour (its `activation` module, over `FakeArchiveProjection`), for Incunabula's store text
(length and SHA-256, below its header and whole, in its `activation` module), for the `SHORTCODE` rules (its
`activation` and `discovery` modules), for the overview (`router.rs`, `lib.rs`, `activation.rs`) and for the URLs a crawl
reaches (`base_tests`: a crawl of Incunabula and the fixture projects under `/cpe` that starts at the base and
fails on a path or public-origin URL outside it and on a link answered 404, with both redirects, both cookies
and the refused base); `cpe-core`'s tests of `url`; `access-server`'s `serve.rs` tests for the mount, the
unmounted cases, a base that is a route, a mount that panics, the held activation, the missing data
directory, the span name, and the absence of a `"/cpe` literal in its `src/`; `sync-store`'s tests for refused
and omitted facts and for the committed file passing `cpe_ports::contract::violations`, which it also applies
at run time; the header tests in `cpe-store-build`'s `sql.rs`; a test of `cpe-store-build` that holds the specification's worked example equal to Incunabula's
block. **Review** — the port speaking archive-shaped facts; a URL being built through `cpe_core::url` at all
(the crawl sees no page it does not reach and no path joined by hand); `serve()` itself, which no test runs;
how the overview looks and what the pages' scripts do in a browser, checked by hand on a preview; a change to
how ARKs resolve or to what CPE emits as `cite-as`; a project moving in only from an empty diff against the
incubator's copy; the incubator's regeneration of its `data.sql`; that a project's `assets/` are the one
input consumed at activation; that the image carries `sync`'s data, `media`'s stand-in directory and the
project folders and sets the variables that name them, which no job checks.
