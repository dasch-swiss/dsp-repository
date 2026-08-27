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
platform-metadata     ← the wire contract, shared with the editor;
  ↑                     lives in `modules/platform/`
  │
dpe-core              ← DPE's view model, caches, repositories; no framework deps
  ↑
  ├── dpe-api-oai     ← OAI-PMH endpoint
  ├── dpe-web         ← Maud pages + components
  └── dpe-server      ← composition root, Datastar fragment handlers
       ↑
       platform-telemetry  ← beacon contract + collector endpoint, shared with
                             editor-server; lives in `modules/platform/`
```

`dpe-api-oai`, `dpe-web` and `dpe-server` depend on `platform-metadata` directly
as well as through `dpe-core` — the contract types are theirs to import, not
`dpe-core`'s to re-export.

## Crate Responsibilities

### `platform-metadata` (`modules/platform/metadata/`)

Not a DPE crate: the research-metadata wire contract, shared with the editor.
Holds the types a data file deserializes into (`ProjectRaw`, `Person`,
`Organization`, `Record`, `AuthorityFileReference`, …) and the rules for reading
a value out of one — `is_placeholder`, the deterministic `multilingual_value`
lookup key, `is_valid_shortcode`, W3CDTF formatting and temporal-coverage
resolution. Table loading is exposed as `load_from(data_dir)` so each service
supplies its own directory. See `modules/platform/README.md`.

### `dpe-core` (core/)

Framework-free domain layer — what only DPE needs. Contains:

- **View model**: `Project` and the conversions to and from `ProjectRaw` (lossy, DPE-only), `Page`, `ClusterRef`, `CollectionRef`, `ResolvedContributor`
- **Repository traits**: `ProjectRepository`, `RecordRepository`
- **Fs implementations**: `FsProjectRepository`, `FsRecordRepository` (backed by in-memory caches)
- **Data loading**: project, record, person, organization, cluster and the two temporal caches (`OnceLock<…>`) loaded from `DPE_DATA_DIR` on first access
- **Utilities**: `lang_value()`, `language_display_name()`, `get_data_dir()`

Dependencies: `platform-metadata`, `serde`, `serde_json`, `tracing`, `ureq`.

### `dpe-api-oai` (api-oai/)

OAI-PMH 2.0 Data Provider. Implements the six required verbs (Identify, ListMetadataFormats, ListSets, ListIdentifiers, ListRecords, GetRecord). Usage is documented in [OAI-PMH Endpoint](./oai-pmh.md).

Depends on `platform-metadata` for the contract types and `dpe-core` for the view model — no web framework dependency.

### `dpe-web` (web/)

Maud view library — a plain `lib` crate of page and component functions returning `maud::Markup`. Contains:

- **Pages**: `home`, `about`, `project`, `projects` (with filters and pagination)
- **Components**: navbar, footer, project cards, tab panels, search input — small `fn -> Markup` partials
- **Data access**: loaders and resolvers (`get_project`, `list_projects`, `get_contributors`) as plain functions over `dpe-core`

Imports `platform-metadata` and `dpe-core` types directly; depends on `maud` and `mosaic-tiles`. No Leptos, no WASM, no `cdylib`/`hydrate`/`ssr` features.

### Browser telemetry

`dpe-server` wires `POST /telemetry/collect` from **`platform-telemetry`**, which is not a DPE crate — it is shared with `editor-server` and lives in `modules/platform/telemetry`. See `modules/platform/README.md`, and `docs/src/repo_structure.md` → *Shared Crates* for why it sits outside `modules/dpe/`. `page_url.rs` (below) is the one part of the pipeline that stays in `dpe-server`: `page.url` normalization needs DPE's own route table, which a shared crate cannot hold.

### `dpe-server` (server/)

Composition root and Axum binary. Contains:

- **Route wiring**: native Axum routes for the Maud pages, the OAI-PMH handler, Datastar fragment endpoints, `/healthz`, `/telemetry/collect`, plus `ServeDir` static serving and a 404 fallback
- **Head/page shell**: `view.rs` — the hand-written `head()` + `page()` partials (title, content-hashed stylesheet link, conditional `traceparent` meta, fonts, Fathom, Datastar + telemetry scripts)
- **Fragment handlers**: `fragments.rs` — plain Axum handlers that render Maud `Markup` to HTML and return Datastar SSE events
- **Page-URL normalization**: `page_url.rs` — bounds the telemetry `page.url` metric attribute to DPE's own known routes, passed into `platform_telemetry::collector::collect_route`
- **Configuration**: `config.rs` — figment-based layered config (defaults → `dpe.toml` → `DPE_*` env vars)
- **Logging**: OTel-aware subscriber via `init-tracing-opentelemetry`

## Key Patterns

- **The wire contract in `platform-metadata`, DPE's view model in `dpe-core`** — never in web or API crates
- **API crates depend on `platform-metadata` and `dpe-core` only**, never on each other or on `dpe-web`
- **`dpe-server` contains no business logic** — only route composition, the head/page shell, and fragment rendering
- **Fragment handlers** call dpe-web view functions and render their `Markup` with `.into_string()`, then wrap it in Datastar `PatchElements`/`ExecuteScript` SSE events
