# DPE Architecture

The Discovery and Presentation Environment (DPE) serves research project metadata as a web application.

## Hypermedia-Driven Architecture

The DPE uses a **hypermedia-driven architecture** where the server is the single source of truth for UI state. Interactivity is provided by [Datastar](https://data-star.dev/) (~14KB JS) instead of client-side frameworks or WASM.

**Why Datastar over Leptos islands:**
- No WASM compilation step (faster builds)
- Smaller client-side footprint (~14KB vs ~200KB+ WASM)
- Server controls all state (HATEOAS)
- Graceful degradation — works as plain HTML links without JavaScript
- Simpler mental model — HTML attributes, not reactive signals

## Rendering Model

Pages are rendered server-side by **Leptos SSR**. Dynamic content updates (tab switching, search autocomplete) are handled by **Datastar SSE fragments**.

```
Initial page load:
  Browser → GET /projects/ABC1 → Leptos SSR → Full HTML page

Tab switch (with JS):
  Browser → GET /projects/ABC1/tab/publications (SSE)
         ← PatchElements (#project-tabs replacement)
         ← ExecuteScript (history.replaceState for URL)

Tab switch (without JS):
  Browser → GET /projects/ABC1?tab=publications → Full page reload
```

## Fragment Route Convention

Fragment endpoints are pure Axum handlers (not Leptos routes) that render Leptos components to HTML strings and deliver them as Datastar SSE events.

**Route pattern: resource-action nesting**

```
GET /projects/{id}              → Full page (Leptos SSR)
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

Key conventions:
- `_` prefix on signals (e.g., `_tab_loading`) excludes them from server payloads
- `retry: 'never'` on `@get()` calls where navigation fallback is preferred
- Every `data-on:click__prevent` link has a valid `href` for graceful degradation
