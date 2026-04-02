# DPE Project Structure

## Workspace Layout

```
modules/dpe/
├── core/             dpe-core          Pure domain (serde only)
├── api-oai/          dpe-api-oai       OAI-PMH 2.0 endpoint
├── web/              dpe-web           Leptos SSR + Datastar fragments
├── server/           dpe-server        Axum binary (composition root)
├── web-e2e-tests/                      Playwright E2E tests
├── public/                             Static assets
└── style/                              Tailwind CSS
```

## Dependency Graph

```
dpe-core          ← pure domain, no framework deps
  ↑
  ├── dpe-api-oai ← OAI-PMH endpoint
  ├── dpe-web     ← Leptos SSR pages + components
  └── dpe-server  ← composition root, Datastar fragment handlers
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

OAI-PMH 2.0 Data Provider. Implements the six required verbs (Identify, ListMetadataFormats, ListSets, ListIdentifiers, ListRecords, GetRecord).

Depends on `dpe-core` for domain types — no Leptos or web framework dependency.

### `dpe-web` (web/)

Leptos SSR web layer. Contains:

- **Pages**: `home`, `about`, `project`, `projects` (with filters and pagination)
- **Components**: navbar, footer, project cards, tab panels, search input
- **Domain re-exports**: `domain/mod.rs` re-exports `dpe-core` types for a single import path
- **Server functions**: `#[server]` wrappers around `dpe-core` functions

### `dpe-server` (server/)

Composition root and Axum binary. Contains:

- **Route wiring**: Leptos SSR routes, OAI-PMH handler, Datastar fragment endpoints, `/healthz`
- **Fragment handlers**: `fragments.rs` — pure Axum handlers that render Leptos components to HTML and return Datastar SSE events
- **Configuration**: `config.rs` — figment-based layered config (defaults → `dpe.toml` → `DPE_*` env vars)
- **Logging**: `tracing-subscriber` with env-filter and JSON support

## Key Patterns

- **Domain types in `dpe-core`**, not in web or API crates
- **API crates depend on `dpe-core` only**, never on each other or on `dpe-web`
- **`dpe-server` contains no business logic** — only route composition and fragment rendering
- **Fragment handlers** use `Owner::new()` + `view! { ... }.to_html()` to render Leptos components from pure Axum handlers
