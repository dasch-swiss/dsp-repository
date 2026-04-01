# AI Agent Guide for DPE

This document provides guidance for AI coding assistants working with the Discovery and Presentation Environment (DPE).

## Project Overview

DPE is a server-side rendered web application. Pages are rendered on the server with **Leptos SSR** and served as plain HTML. Interactive behavior (tab switching, live search) is handled by **Datastar**, which streams HTML fragments over SSE -- there is no client-side WASM, no hydration, and no islands architecture for DPE.

## Crate Structure

```
dpe/
├── core/             # Pure domain types, repositories, data loading (crate: dpe-core)
│                     # Zero framework deps -- only serde, serde_json, tracing
├── api-oai/          # OAI-PMH 2.0 API (crate: dpe-api-oai)
│                     # Depends on dpe-core only
├── app/              # Web layer: Leptos components, pages, domain re-exports (crate: dpe-web)
│                     # Depends on dpe-core for domain types
├── server/           # Server binary and fragment handlers (crate: dpe-server)
│                     # Composes dpe-web (Leptos routes) + dpe-api-oai (API routes)
├── web-e2e-tests/    # Playwright E2E tests
├── public/           # Static assets
└── style/            # CSS / Tailwind configuration
```

### Crate Responsibilities

- **dpe-core** -- Domain types (`Project`, `Organization`, `Person`, etc.), repository functions (`get_project`, `list_projects`), and data loading from JSON files. No framework dependencies.
- **dpe-api-oai** -- OAI-PMH 2.0 XML endpoint. Depends only on dpe-core.
- **dpe-web** (app/) -- Leptos `#[component]` functions for pages and reusable UI elements. Re-exports domain types from dpe-core via its `domain` module so that the server crate has a single import path.
- **dpe-server** (server/) -- The composition root. Wires up Axum routes for Leptos pages, OAI-PMH, health checks, and Datastar fragment endpoints. Contains `fragments.rs` with pure Axum handlers.

## Key Technologies and Versions

- **Leptos**: 0.8.2 (SSR only -- no `hydrate` feature for DPE)
- **Axum**: 0.8.8
- **Datastar**: SSE-based interactivity via `datastar` crate (0.3)
- **Tailwind CSS**: 4.x + DaisyUI
- **Rust Edition**: 2021
- **Runtime**: Tokio

## Development Commands

All commands should be run from the workspace root using `just`:

```bash
just watch-dpe          # Run DPE dev server with hot reload (http://127.0.0.1:4000)
just check              # Run fmt checks + clippy
just fmt                # Format all Rust code (cargo fmt + leptosfmt)
just test               # Run all cargo tests
just build              # Build all targets
```

For running cargo tests directly within the DPE module:

```bash
cargo test -p dpe-core
cargo test -p dpe-web
cargo test -p dpe-server
cargo test -p dpe-api-oai
```

## Code Organization Patterns

### Components

Components live in `app/src/components/`. Each file exports one or more Leptos `#[component]` functions. Re-export new components in `app/src/components/mod.rs`.

Subdirectories under `components/` group related pieces (e.g., `components/global/` for layout-level elements).

### Pages

Pages live in `app/src/pages/`. Each page is a top-level Leptos component rendered by the router. Subdirectories group page-specific sub-components (e.g., `pages/project/components/`).

Routes are declared in `app/src/lib.rs` inside the `<Routes>` block:

```rust
<Route path=StaticSegment("projects") view=ProjectsPage />
<Route path=path!("projects/:id") view=ProjectPage />
```

### Domain Re-exports

`app/src/domain/` re-exports types and functions from `dpe-core`. The server crate imports domain items through `dpe_web::domain` so there is a single import path.

### Fragment Handler Pattern

Interactive updates use **Datastar SSE fragments** -- these are pure Axum route handlers (not Leptos server functions) that:

1. Receive a request (path params or Datastar `ReadSignals`)
2. Load data from dpe-core repositories
3. Render a Leptos component to an HTML string using `Owner::new()` + `.to_html()`
4. Return the HTML as a Datastar `PatchElements` SSE event, optionally followed by `ExecuteScript` events

Fragment routes are registered in `server/src/main.rs` and implemented in `server/src/fragments.rs`. Example routes:

```
GET /projects/{id}/tab/{tab}   -> tab_fragment_handler
GET /projects/search           -> search_fragment_handler
```

The key point: fragments run entirely on the server. No client-side Rust code is involved.

### Datastar Attributes in Templates

Interactive elements in Leptos templates use Datastar `data-*` attributes:

- `data-on:click` with `@get('/fragment/url')` to fetch SSE fragments
- `data-on:input__debounce` for debounced search
- `data-on:datastar-fetch` for error handling fallbacks
- Fragment targets are identified by CSS selectors (e.g., `#project-tabs`)

## Testing

### Running tests

```bash
just test                              # All workspace tests
cargo test -p dpe-server               # Single crate
cargo test -p dpe-server test_name     # Specific test
cargo nextest run                      # Faster parallel runner (if installed)
```

### Snapshot tests (insta)

```bash
cargo test -p dpe-server               # Run tests — failing snapshots produce .snap.new files
cargo insta review                     # Interactive review of new/changed snapshots
cargo insta accept                     # Accept all pending snapshots
```

Snapshot files (`.snap`) are committed to git. Use `with_settings!` to scrub dynamic values.

### E2E tests (Playwright)

```bash
cd modules/dpe/web-e2e-tests && npx playwright test
```

See `docs/src/dpe/testing-strategy.md` for the full testing pyramid documentation.

## Conventions and Review

- See `CONVENTIONS.md` for fragment route naming, Datastar attribute patterns, test directory conventions, and code style rules.
- See `REVIEW.md` for the code review checklist.

## When Making Changes

### Adding a New Component

1. Create file in `app/src/components/`
2. Define component with `#[component]` macro
3. Export in `app/src/components/mod.rs`
4. Import in the relevant page or layout

### Adding a New Page

1. Create file in `app/src/pages/`
2. Define page component
3. Export in `app/src/pages/mod.rs`
4. Add route in `app/src/lib.rs` inside `<Routes>`

### Adding a New Fragment Handler

1. Add the async handler function in `server/src/fragments.rs`
2. Register the route in `server/src/main.rs`
3. Use `Owner::new()` + `view! { ... }.to_html()` to render Leptos components
4. Return a `Sse` stream of `PatchElements` (and optionally `ExecuteScript`) events
5. Strip hot-reload comments with `strip_hot_reload_comments()` in dev mode

### Modifying Styles

- Global styles: edit `style/main.css`
- Component styles: use Tailwind / DaisyUI classes in component templates
- Tailwind input is configured in `Cargo.toml`: `tailwind-input-file = "style/main.css"`

## Common Pitfalls

### Leptos 0.8.x API

- Import via `use leptos::prelude::*;`
- Router uses `StaticSegment` and `path!` macro (different from 0.7.x)
- Signal APIs may differ from older online examples

### Styling Not Applying

- Stylesheet link is in `app/src/lib.rs`: `<Stylesheet id="leptos" href="/pkg/dpe.css" />`
- Ensure Tailwind build runs (automatic with `just watch-dpe`)
- Check DaisyUI theme configuration in `style/main.css`

### Hot-Reload Comments in Fragments

In dev mode, Leptos wraps output in `<!--hot-reload|...|-->` comments. These break Datastar morphing. Always pass fragment HTML through `strip_hot_reload_comments()`.

## Best Practices for AI Agents

1. **Read existing code first** before making changes to understand patterns
2. **Maintain consistency** with existing component and file structure
3. **Use Leptos 0.8.x syntax** -- many online examples target 0.7.x
4. **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types/components
5. **Use `just check`** to verify formatting and linting before considering work done
6. **Use mosaic-tiles components** where appropriate for consistent UI
7. **Check Tailwind v4 syntax** -- some classes changed from v3

## Note on Mosaic Playground

The Mosaic component playground (`modules/mosaic/demo/`) is a separate application that does use Leptos islands and WASM. That architecture does not apply to DPE. Do not conflate the two.

---

**Last Updated**: 2026-03-27
**Leptos Version**: 0.8.2
**For AI Agents**: This project is actively in development. Always verify current patterns by reading existing code before making changes.
