# DPE Project Structure

## Workspace Layout

```
modules/dpe/
├── core/             dpe-core          Pure domain (serde only)
├── telemetry/        dpe-telemetry     Telemetry types and validation (serde only)
├── api-oai/          dpe-api-oai       OAI-PMH 2.0 endpoint
├── web/              dpe-web           Maud view library (pages + components)
├── server/           dpe-server        Axum binary (composition root)
├── web-e2e-tests/                      Playwright E2E tests
├── public/                             Static assets
└── style/                              Tailwind CSS
```

## Dependency Graph

```
dpe-core              ← pure domain, no framework deps
  ↑
  ├── dpe-api-oai     ← OAI-PMH endpoint
  ├── dpe-web         ← Maud pages + components
  └── dpe-server      ← composition root, Datastar fragment handlers
       ↑
       dpe-telemetry  ← telemetry types and validation (serde only)
```

## Crate Responsibilities

### `dpe-core` (core/)

Framework-free domain layer. Contains:

- **Domain types**: `Project`, `Record`, `Person`, `Organization`, `Attribution`, etc.
- **Repository traits**: `ProjectRepository`, `RecordRepository`
- **Fs implementations**: `FsProjectRepository`, `FsRecordRepository` (backed by in-memory caches)
- **Data loading**: Project and record caches (`OnceLock<Vec<T>>`) loaded from JSON on first access
- **Utilities**: `lang_value()`, `get_data_dir()`

Dependencies: `serde`, `serde_json` only.

### `dpe-api-oai` (api-oai/)

OAI-PMH 2.0 Data Provider. Implements the six required verbs (Identify, ListMetadataFormats, ListSets, ListIdentifiers, ListRecords, GetRecord). Usage is documented in [OAI-PMH Endpoint](./oai-pmh.md).

Depends on `dpe-core` for domain types — no web framework dependency.

### `dpe-web` (web/)

Maud view library — a plain `lib` crate of page and component functions returning `maud::Markup`. Contains:

- **Pages**: `home`, `about`, `project`, `projects` (with filters and pagination)
- **Components**: navbar, footer, project cards, tab panels, search input — small `fn -> Markup` partials
- **Data access**: loaders and resolvers (`get_project`, `list_projects`, `get_contributors`) as plain functions over `dpe-core`

Imports `dpe-core` types directly; depends on `maud` and `mosaic-tiles`. No Leptos, no WASM, no `cdylib`/`hydrate`/`ssr` features.

### `dpe-telemetry` (telemetry/)

Telemetry types and validation logic. Extracted as a library crate so fuzz targets can test the real code. Contains:

- **Beacon types**: `BeaconPayload`, `Signal`, `WebVitalSignal`, `ErrorSignal`, etc. (serde deserialization for browser beacons)
- **Origin validation**: `is_allowed_origin()` — validates dasch.swiss subdomains
- **URL normalization**: `normalize_page_url()` — cardinality-safe page URL mapping
- **Traceparent validation**: `is_valid_traceparent()` — W3C traceparent format validation

Dependencies: `serde` only.

### `dpe-server` (server/)

Composition root and Axum binary. Contains:

- **Route wiring**: native Axum routes for the Maud pages, the OAI-PMH handler, Datastar fragment endpoints, `/healthz`, `/telemetry/collect`, plus `ServeDir` static serving and a 404 fallback
- **Head/page shell**: `view.rs` — the hand-written `head()` + `page()` partials (title, content-hashed stylesheet link, conditional `traceparent` meta, fonts, Fathom, Datastar + telemetry scripts)
- **Fragment handlers**: `fragments.rs` — plain Axum handlers that render Maud `Markup` to HTML and return Datastar SSE events
- **Telemetry collector**: `telemetry_collector.rs` — converts browser beacons to OTel metrics and structured logs (uses types from `dpe-telemetry`)
- **Configuration**: `config.rs` — figment-based layered config (defaults → `dpe.toml` → `DPE_*` env vars)
- **Logging**: OTel-aware subscriber via `init-tracing-opentelemetry`

## Key Patterns

- **Domain types in `dpe-core`**, not in web or API crates
- **API crates depend on `dpe-core` only**, never on each other or on `dpe-web`
- **`dpe-server` contains no business logic** — only route composition, the head/page shell, and fragment rendering
- **Fragment handlers** call dpe-web view functions and render their `Markup` with `.into_string()`, then wrap it in Datastar `PatchElements`/`ExecuteScript` SSE events
