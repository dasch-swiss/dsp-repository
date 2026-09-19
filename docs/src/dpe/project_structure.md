# DPE Project Structure

## Workspace Layout

```
modules/dpe/
├── core/             dpe-core          DPE's view model, caches, repositories (serde only)
├── api-oai/          dpe-api-oai       OAI-PMH 2.0 endpoint
├── web/              dpe-web           Maud view library (pages + components)
├── server/           dpe-server        Axum binary (composition root)
├── web-e2e-tests/                      Playwright E2E tests
├── public/                             Static assets
└── style/                              Tailwind CSS
```

## Dependency Graph

```
shared-metadata         ← the wire contract, shared with the editor;
  ↑                       lives in `shared/`
  ├── shared-fair       ← FAIR exposure engine: resolved graphs + representation
  │                       writers, over the contract types only; lives in `shared/`
  │
  └── dpe-core          ← DPE's view model, caches, repositories; no framework deps
        ↑
        ├── dpe-api-oai ← OAI-PMH endpoint; also depends on shared-fair
        ├── dpe-web     ← Maud pages + components
        └── dpe-server  ← composition root, Datastar fragment handlers
              ↑
              shared-telemetry  ← beacon contract + collector endpoint, shared
                                  with editor-server; lives in `shared/`
```

`dpe-api-oai`, `dpe-web` and `dpe-server` depend on `shared-metadata` directly
as well as through `dpe-core` — the contract types are theirs to import, not
`dpe-core`'s to re-export.

The arrow between `dpe-core` and `shared-fair` runs in neither direction, and
that is deliberate: `shared-fair` names only contract types, so the domain crate
never depends on the exposure engine and the engine never learns DPE's view
model. `dpe_core::resolve_inputs()` is the seam — it returns the contributor
lookup and the two temporal tables, which `dpe-api-oai` wraps in a
`shared_fair::ResolveContext` at the call site.

## Crate Responsibilities

### `shared-metadata` (`shared/metadata/`)

Not a DPE crate: the research-metadata wire contract, shared with the editor.
Holds the types a data file deserializes into (`ProjectRaw`, `Person`,
`Organization`, `Record`, `AuthorityFileReference`, …) and the rules for reading
a value out of one — `is_placeholder`, the deterministic `multilingual_value`
lookup key, `is_valid_shortcode`, W3CDTF formatting and temporal-coverage
resolution. Table loading is exposed as `load_from(data_dir)` so each service
supplies its own directory. Also holds `ContributorLookup`, the trait a service
implements over its own corpus to resolve an `Attribution` id to a `Person` or
an `Organization`. See `shared/README.md`.

### `shared-fair` (`shared/fair/`)

Not a DPE crate: the FAIR exposure engine of ADR-0005, currently with
`dpe-api-oai` as its only consumer. Builds one resolved graph per published
object — `ProjectGraph::build` applies agent resolution, placeholder filtering,
multilingual preference, creator fallback and temporal-coverage resolution once,
`RecordGraph::build` reads a record alone — and writes each representation off that graph
(`project_to_datacite`, `project_to_dublin_core`, `record_to_datacite`,
`record_to_dublin_core`). It knows no routes and no URL layout: a writer returns
a `String` or a `serde_json::Value`. What it needs from a service arrives in
`ResolveContext::new(lookup, periods, enriched)`. See `shared/README.md`.

### `dpe-core` (core/)

Framework-free domain layer — what only DPE needs. Contains:

- **View model**: `Project` and the conversions to and from `ProjectRaw` (lossy, DPE-only), `Page`, `ClusterRef`, `CollectionRef`, `ResolvedContributor`
- **Repository traits**: `ProjectRepository`, `RecordRepository`
- **Fs implementations**: `FsProjectRepository`, `FsRecordRepository` (backed by in-memory caches)
- **Data loading**: project, record, person, organization, cluster and the two temporal caches (`OnceLock<…>`) loaded from `DPE_DATA_DIR` on first access
- **Utilities**: `lang_value()`, `language_display_name()`, `get_data_dir()`, `get_public_dir()`
- **Static-asset lookup**: `cover_image_cache` scans `<public dir>/assets/images` once for the per-project cover images, so a view can tell whether a project has one before rendering an `<img>`

The directory paths and display flags are process-global `OnceLock`s set from `dpe-server`'s config at startup (`set_data_dir`, `set_public_dir`, `set_show_placeholder_values`) and read directly by `dpe-web` views, which take no application state. Reuse that pattern rather than threading new values through `AppState`.

Dependencies: `shared-metadata`, `serde`, `serde_json`, `tracing`, `ureq`.

### `dpe-api-oai` (api-oai/)

OAI-PMH 2.0 Data Provider. Implements the six required verbs (Identify, ListMetadataFormats, ListSets, ListIdentifiers, ListRecords, GetRecord). Usage is documented in [OAI-PMH Endpoint](./oai-pmh.md).

Depends on `shared-metadata` for the contract types, `dpe-core` for the view model and the `resolve_inputs()` seam, and `shared-fair` for the DataCite and Dublin Core mappings. What stays here is OAI-PMH protocol: the envelope and XML builder, the verbs, the `oai:dasch.swiss:` identifiers, the set specs, the date filters and `OaiRecord`. The corpus-wide tests over the committed data stay here too (`src/metadata/corpus.rs`), beside the data they read.

### `dpe-web` (web/)

Maud view library — a plain `lib` crate of page and component functions returning `maud::Markup`. Contains:

- **Pages**: `home`, `about`, `project`, `projects` (with filters and pagination)
- **Components**: navbar, footer, project cards, tab panels, search input — small `fn -> Markup` partials
- **Data access**: loaders and resolvers (`get_project`, `list_projects`, `get_contributors`) as plain functions over `dpe-core`

Imports `shared-metadata` and `dpe-core` types directly; depends on `maud` and `mosaic-tiles`. No Leptos, no WASM, no `cdylib`/`hydrate`/`ssr` features.

### Browser telemetry

`dpe-server` wires `POST /telemetry/collect` from **`shared-telemetry`**, which is not a DPE crate — it is shared with `editor-server` and lives in `shared/telemetry`. See `shared/README.md`, and `docs/src/repo_structure.md` → *Shared Crates* for why it sits outside `modules/dpe/`. `page_url.rs` (below) is the one part of the pipeline that stays in `dpe-server`: `page.url` normalization needs DPE's own route table, which a shared crate cannot hold.

### `dpe-server` (server/)

Composition root and Axum binary. Contains:

- **Route wiring**: native Axum routes for the Maud pages, the OAI-PMH handler, Datastar fragment endpoints, `/healthz`, `/telemetry/collect`, plus `ServeDir` static serving and a 404 fallback
- **Head/page shell**: `view.rs` — the hand-written `head()` + `page()` partials (title, content-hashed stylesheet link, conditional `traceparent` meta, fonts, Fathom, Datastar + telemetry scripts)
- **Fragment handlers**: `fragments.rs` — plain Axum handlers that render Maud `Markup` to HTML and return Datastar SSE events
- **Page-URL normalization**: `page_url.rs` — bounds the telemetry `page.url` metric attribute to DPE's own known routes, passed into `shared_telemetry::collector::collect_route`
- **Configuration**: `config.rs` — figment-based layered config (defaults → `dpe.toml` → `DPE_*` env vars)
- **Logging**: OTel-aware subscriber via `init-tracing-opentelemetry`

## Key Patterns

- **The wire contract in `shared-metadata`, DPE's view model in `dpe-core`** — never in web or API crates
- **API crates depend on `shared-metadata`, `dpe-core` and the `shared-*` crates they need** — `dpe-api-oai` takes `shared-fair` — never on each other or on `dpe-web`
- **`dpe-server` contains no business logic** — only route composition, the head/page shell, and fragment rendering
- **Fragment handlers** call dpe-web view functions and render their `Markup` with `.into_string()`, then wrap it in Datastar `PatchElements`/`ExecuteScript` SSE events
