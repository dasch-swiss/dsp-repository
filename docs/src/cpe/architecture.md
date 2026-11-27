# CPE Architecture

The Configurable Presentation Environment (CPE) presents archived projects through project-specific pages: a
narrative, curated lists, facets, a book reader, records with their images. It is the Access Area's second
capability beside DPE (ADR-0007). It has no binary of its own: `access-server` nests its router under `CPE_BASE`
(`/cpe`) and activates it in the background. What it serves and how it is configured on a deployment is on
[CPE Operations](./operations.md); what a project's `config.kdl` may say is the [Config Language](./config-language.md);
the stylesheets, the layout contract and the rules for JavaScript are on the [Component Library](./component-library.md).
CPE's vocabulary is `areas/access/cpe/CONTEXT.md`.

## Crate Structure

The eight crates are in `areas/access/cpe/`, one directory each.

```
 cpe-ports          cpe-core
 the two ports      KDL config language, validation, IR, URLs
     │                  │
     │       ┌──────────┼──────────────┬─────────────────┐
     │       │          │              │                 │
     ├── cpe-store-build│          cpe-store       cpe-components ── cpe-image-viewer
     │   the remodel:   │          in-memory       Maud components,     <image-viewer>,
     │   snapshot →     │          SQLite store    seven stylesheets     OpenSeadragon
     │   SQL text       │          and queries          │
     │       │          │              │                 │
     │       │          └──────── cpe-web ───────────────┘
     │       │                    page assembly, no routes
     │       │                        │
     └───────┴─────────────────── cpe-server
                                  CpeConfig, Cpe: router, activation, validate
                                      │
                                  access-server (areas/access/server)
```

- **cpe-ports** (`ports/`): the two ports CPE consumes (see below), their fakes and the snapshot's contract
  (`contract::violations`). Depends on `std` and `shared-names` only.
- **cpe-core** (`core/`): what a project's configuration means, framework-free: the KDL language and its
  validation (`validate`), the IR a valid configuration lowers to (`ir`, `remodel`), the diagnostics
  (`diagnostic`), and every URL CPE writes (`url`). It depends on no CPE crate.
- **cpe-store-build** (`store-build/`): the remodel. `build` turns a project snapshot, its curation and the
  `remodel {}` block into the store's rows, with the project's hook where the block declares one, and
  `sql::to_sql` renders them as SQL text. It is pure: it reads no file, clock or environment.
- **cpe-store** (`store/`): the derived read model (ADR-0008), one in-memory SQLite database per project,
  opened from that SQL text over `schema.sql`, and the queries over it (`query::page::page_data`). It owns the
  connection; no `rusqlite` type is in its `pub` items.
- **cpe-components** (`components/`): the components a page is assembled from, as Maud functions, the
  interface's message catalogue (`chrome::ChromeMsg`), the per-project components behind `PROJECT_RENDERERS`,
  and the seven stylesheets in `components/style/`.
- **cpe-image-viewer** (`image-viewer/`): the `<image-viewer>` web component on a vendored OpenSeadragon, with
  its `Viewer` builder.
- **cpe-web** (`web/`): the page assembly. `project::Project` is a loaded project (`Project::load` opens the store
  and validates the configuration against it); `page` renders a page, the compound viewer's fragment, the
  overview and the not-found pages. It holds no routes.
- **cpe-server** (`server/`): the host crate, a facade in the shape of `dpe-server`: `CpeConfig`, `Cpe` (its
  router, `activate`, `follow`) and `validate`. It also serves the shared static files from a compiled-in table
  (`static_files.rs`) and each project's assets (`assets.rs`).

The edges are the `visibility` of each crate's Bazel target: a target lists the packages that may depend on it,
and any other edge fails analysis (ADR-0007). Outside `areas/access/cpe/`, only `cpe-ports` and `cpe-server` are
visible to anything, and no CPE crate depends on a Mosaic crate. No `pub` item of `cpe-server` names another CPE
crate's type, `cpe-ports`' excepted, with one exception: the `hooks` parameter of the hidden
`Cpe::activate_with_hooks` and `Cpe::follow_with_hooks` names `cpe_store_build::hook::Hook`, for a host that
builds this tree outside the repository with hooks of its own. The test `no_pub_item_names_another_cpe_crate`
holds the rule. The third-party crates each role may use are checked by `check-capability-deps.sh`
(`just check-capability-deps`, run by `just check`).

## The Ports

CPE reads its data through two ports that it declares in `cpe-ports` and other capabilities of the area implement
(ADR-0003, ADR-0012):

- **The archive projection port**, `ArchiveProjection`, which `sync` implements as `sync-store`'s
  `LiveArchiveProjection` over `SYNC_DATA_DIR`. It serves a project's `ProjectSnapshot`, its resources and list
  nodes in the archive's shape, and announces a project whose Ready token appeared, changed or was removed
  (`subscribe`). Every snapshot an adapter serves must pass `contract::violations`.
- **The bytes port**, `DepositedFiles`, which `media` implements as `media-store`'s `LiveDepositedFiles` over
  `MEDIA_DATA_DIR`. It serves a project's deposited files by shortcode and name: `config.kdl`, `curation.csv`
  and the narrative files the KDL names. It lists nothing and gives no file a meaning.

`access-server` constructs both adapters and hands them to `Cpe::follow`. CPE's tests use `FakeArchiveProjection`
and `FakeDepositedFiles`, or the `testonly` variants of the two stores. Both stores' data is written by the VRE
bridge; the committed stand-ins under `areas/access/sync/data/` and `areas/access/media/data/` are what a local run
and the tests read.

## Activation and a Project's Statuses

`Cpe::new` reads nothing and cannot fail; `Cpe::router` may be mounted at once and answers 503 until the first
activation has finished. `Cpe::follow` runs that activation and then re-activates on the archive projection port's
announcements, on a thread of its own (`cpe-reactivate`).

An activation discovers the project folders in `CPE_PROJECTS_DIR` (`discovery.rs`): a direct subfolder whose name
is a valid project id and that holds a regular `SHORTCODE` file. For each, it reads the provenance both ports
serve, asks for the snapshot, the KDL, the narrative and the curation, builds the store with `cpe-store-build` and
the project's hook, reads the provenance again, reads the project's `assets/`, and loads the project with
`Project::load`, which validates the configuration against the store (`activation.rs`). Each project comes out
with one status:

| Status | When | Its paths answer | On the overview |
|--------|------|------------------|-----------------|
| `Loaded` | It loads | its pages | yes, unless it is a Hidden project |
| `Unknown` | `sync` holds no Ready token for it while its location is there: it is not released on this deployment | CPE's not-found, 404 with `Cache-Control: no-store`, as a path that names no project | no |
| `Failed` (an Unavailable project) | Anything else goes wrong: a port, a file, the build, validation, a panic | 503 with `Retry-After` and `Cache-Control: no-store` | no |

A project fails alone: the others are served, and DPE is never stopped or delayed. A re-activation loads every
project again and replaces what is served only if every project loaded before loads again or is `Unknown` now;
otherwise it is held back (`activation::held_back_by`), logged, and tried once more after the host's retry
interval. [CPE Operations](./operations.md) lists every situation with its log line.

`cpe_server::validate` runs the same load once and serves nothing, for `access-server validate`'s strict check;
with an inputs directory it adds the data-free check of the VRE bridge's shipped inputs (`inputs.rs`).

## The Request Path

`access-server` nests `Cpe::router()` at `Cpe::base()`, the one source of the base, and adds the baseline
response headers around it. Inside, the router (`router.rs`) has:

- the **overview** at the base, with and without a trailing slash: one teaser per loaded project that is not
  hidden (`page::render_site_index`);
- the **shared static files** at `<base>/_static/…` and `<base>/<project id>/_static/…`, and each project's own
  files at `<base>/<project id>/_assets/…`; an SVG of either carries a script-blocking policy;
- one route per **page** a project's KDL declares, at `<base>/<project id><route>`, with `:name` parameters
  translated to axum's syntax (`axum_route`);
- the **fragment endpoints** of a page whose body holds the component: `_facets`, `_sort`, `_search` and
  `_compound`, each a DataStar SSE response that patches parts of the page. The segments are constants of
  `cpe-components` (`FACET_FRAGMENT_SEGMENT`, `SORT_FRAGMENT_SEGMENT`, `SEARCH_FRAGMENT_SEGMENT`,
  `COMPOUND_FRAGMENT_SEGMENT`); the `_` namespace is the engine's, and a page whose route's first segment starts
  with `_` is refused (`ReservedRoute`);
- a **fallback**: the project's not-found page under a loaded project, 503 under a failed one, CPE's own
  not-found page otherwise. It never redirects and never writes the request path into the answer.

A page request is answered in three steps. The handler reads the language and appearance preferences from the
query and cookies (`request.rs`); `Project::page_data` reads what the page's components need from the store, under
the store's lock, on the request's thread; and `page::render_page` renders the document from `cpe-components`'
functions. Every URL in it is built by `cpe_core::url` from the base. Each response carries the route's template
as `MatchedRoute` in its extensions, from which `access-server` names the request's span, and a Hidden project's
responses carry `X-Robots-Tag` (`request::robots_for_hidden_projects`).

## Decisions

CPE's decisions in this repository are root ADRs: ADR-0007 (CPE joins the Access Area, its crates, the seam,
"a project fails alone"), ADR-0008 (the derived read model), ADR-0012 (the two input channels and their ports) and
ADR-0013 (authorisation). CPE keeps no decision series of its own. The prototype's records, in dsp-incubator's
`cpe/docs/adr/`, are cited by file name (`dsp-incubator cpe/docs/adr/<file name>`) where code or a document follows
one of them; they record how the prototype decided, and their numbers are not this repository's.
