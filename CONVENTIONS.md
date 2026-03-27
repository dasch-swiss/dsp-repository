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

## Test Directory Naming

- `web-e2e-tests/` for DPE Playwright tests (sibling of the app, not nested)
- `playground-e2e-tests/` for Mosaic Playwright tests
- Not `end2end/` (legacy name, will be renamed)
