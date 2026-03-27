# Conventions

Project-wide conventions for the DSP Repository.

## Fragment Route Convention

Fragment routes serve Datastar SSE responses for partial page updates (tab switching, search, etc.).

**Pattern: resource-action nesting**

| Route | Handler | Response |
|-------|---------|----------|
| `GET /projects/{id}` | Leptos SSR | Full page (reads `?tab=` for initial tab) |
| `GET /projects/{id}/tab/{tab}` | Pure Axum | SSE fragment (PatchElements + ExecuteScript) |

Different path depths in Axum's radix trie — no conflict, no header discrimination needed.

## Datastar Attribute Conventions

- **Signal naming**: Use `_` prefix for client-only signals (e.g., `_tab_loading`). The underscore excludes the signal from server payloads.
- **No `__debounce` on `__prevent` anchors**: Do NOT combine `__prevent` with `__debounce` or `__throttle` on anchor elements — known Datastar timing issue.
- **`retry: 'never'`**: Use on `@get()` calls where fallback to full navigation is preferred over retrying.
- **Graceful degradation**: Every Datastar-enhanced `<a>` must have a valid `href` for no-JS fallback.

## Crate Naming Convention

All workspace crates follow the `{module}-{role}` pattern:

| Crate | Role |
|-------|------|
| `dpe-core` | Pure domain types and data access (zero framework deps) |
| `dpe-api-oai` | OAI-PMH 2.0 API (depends on `dpe-core` only) |
| `dpe-web` | Leptos SSR components, pages, `#[server]` functions |
| `dpe-server` | Server binary — composes all routes (binary name: `dpe`) |
| `mosaic-tiles` | Reusable UI component library |
| `mosaic-playground` | Component showcase application |
| `mosaic-playground-macro` | Proc macro for demo page generation |

## API Crate Pattern

Each API is a separate crate under `modules/dpe/`:

- **Naming**: `dpe-api-{name}` (e.g., `dpe-api-oai`)
- **Dependencies**: `dpe-core` for domain types; never depends on other API crates or `dpe-web`
- **Entry point**: Exports a handler function (e.g., `pub async fn oai_handler(...)`)
- **Composition**: `dpe-server` wires the handler into the Axum router

## Test Directory Naming

- `web-e2e-tests/` for DPE Playwright tests (sibling of the app, not nested)
- `playground-e2e-tests/` for Mosaic Playwright tests
