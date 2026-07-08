# DPE Architecture

The Discovery and Presentation Environment (DPE) serves research project metadata as a web application.

## Crate Structure

```
dpe-core          Pure domain types, repositories, data loading
                  Dependencies: serde, serde_json only
                       │
          ┌────────────┼────────────┐
          │            │            │
     dpe-api-oai   dpe-web     (future APIs)
     OAI-PMH 2.0  Maud views
     + axum
          │            │
          └────────────┘
                 │
           dpe-server
           Route composition
           + Datastar
           (binary: dpe-server)
```

- **dpe-core**: Framework-free domain layer. All types, repository traits, Fs implementations, and data loading.
- **dpe-api-oai**: OAI-PMH 2.0 endpoint (see [OAI-PMH Endpoint](./oai-pmh.md)). Depends only on dpe-core.
- **dpe-web**: A native library of [Maud](https://maud.lambda.xyz/) page and component functions (`fn -> Markup`). Imports dpe-core types directly.
- **dpe-server**: Thin composition root. Wires the native Axum router, the `<head>`/page shell, config, and the Datastar fragment handlers, mounting dpe-web's views and dpe-api-oai's handlers into a single Axum server.

## Hypermedia-Driven Architecture

The DPE uses a **hypermedia-driven architecture** where the server is the single source of truth for UI state. Pages are rendered as plain HTML with [Maud](https://maud.lambda.xyz/) (a compile-time HTML template macro), and interactivity is provided by [Datastar](https://data-star.dev/) (~14KB JS) instead of a client-side framework or WASM.

**Why this approach:**
- No WASM compilation step (faster builds, no `wasm32` toolchain)
- Small client-side footprint (~14KB Datastar, no hydration bundle)
- Server controls all state (HATEOAS)
- Graceful degradation — works as plain HTML links without JavaScript
- Simpler mental model — HTML attributes, not reactive signals

## Rendering Model

Pages are rendered server-side with **Maud** (`maud::html!` → `Markup`). Dynamic content updates (tab switching, search autocomplete) are handled by **Datastar SSE fragments**.

```
Initial page load:
  Browser → GET /dpe/projects/ABC1 → Maud SSR → Full HTML page

Tab switch (with JS):
  Browser → GET /dpe/projects/ABC1/tab/publications (SSE)
         ← PatchElements (#project-tabs replacement)
         ← ExecuteScript (history.replaceState for URL)

Tab switch (without JS):
  Browser → GET /dpe/projects/ABC1?tab=publications → Full page reload
```

## Fragment Route Convention

Fragment endpoints are plain Axum handlers that render Maud `Markup` to HTML strings and deliver them as Datastar SSE events. The full page and the SSE fragment render the `#project-tabs` morph root through the same `project_tabs` function, so the two paths cannot drift.

**Route pattern: resource-action nesting**

```
GET /projects/{id}              → Full page (Maud SSR)
GET /projects/{id}/tab/{tab}    → SSE fragment (Axum + Datastar)
```

Different path depths in Axum's radix trie mean no conflict and no header-based discrimination.

## HATEOAS Tab Pattern

The server returns the **complete tab component** (tab bar + panel) in each SSE response. This means:
- Server controls which tab is active (`aria-selected`)
- Server controls which tabs are visible (e.g., hide Publications if none exist)
- Server pushes the bookmarkable URL via `ExecuteScript` + `history.replaceState`

The client never needs to track tab state — the server-rendered HTML IS the state.

## Datastar Attribute Patterns

```html
<!-- Tab link with Datastar enhancement -->
<a href="/projects/ABC1?tab=publications"
   role="tab" aria-selected="false"
   data-on:click__prevent="@get('/projects/ABC1/tab/publications', {retry: 'never'})"
   data-indicator:_tab_loading>
  Publications
</a>

<!-- SSE failure fallback on container -->
<div id="project-tabs"
     data-on:datastar-fetch="
       (evt.detail.type === 'error' || evt.detail.type === 'retries-failed')
       && evt.detail.el.closest('#project-tabs')
       && (window.location.href = evt.detail.el.getAttribute('href'))
     ">
```

## Datastar Attribute Conventions

- **Signal naming**: Use `_` prefix for client-only signals (e.g., `_tab_loading`). The underscore excludes the signal from server payloads.
- **No `__debounce` on `__prevent` anchors**: Do NOT combine `__prevent` with `__debounce` or `__throttle` on anchor elements — known Datastar timing issue.
- **`retry: 'never'`**: Use on `@get()` calls where fallback to full navigation is preferred over retrying.
- **Graceful degradation**: Every Datastar-enhanced `<a>` must have a valid `href` for no-JS fallback.

## See Also

- [Project Structure](./project_structure.md) — Crate responsibilities and dependency graph
- [Testing Strategy](./testing-strategy.md) — Testing pyramid and CI pipeline
- [Operations](./operations.md) — Docker, environment variables, deployment
