# AI Agent Guide for DPE

This document provides DPE-specific guidance for AI coding assistants. For project-wide guidance, see the top-level `CLAUDE.md`.

## Project Overview

DPE is a server-side rendered web application. Pages are rendered on the server with **Leptos SSR** and served as plain HTML. Interactive behavior (tab switching, live search) is handled by **Datastar**, which streams HTML fragments over SSE — there is no client-side WASM, no hydration, and no islands architecture for DPE.

## Architecture and Structure

- **Crate structure and dependency graph**: See `docs/src/dpe/project_structure.md`
- **Architecture, rendering model, and Datastar conventions**: See `docs/src/dpe/architecture.md`
- **Testing strategy and conventions**: See `docs/src/dpe/testing-strategy.md`
- **Operations (Docker, env vars, CLI)**: See `docs/src/dpe/operations.md`

## Key Technologies and Versions

- **Leptos**: 0.8.2 (SSR only — no `hydrate` feature for DPE)
- **Axum**: 0.8.8
- **Datastar**: SSE-based interactivity via `datastar` crate (0.3)
- **Tailwind CSS**: 4.x + DaisyUI
- **Rust Edition**: 2021
- **Runtime**: Tokio

## Code Organization Patterns

### Components

Components live in `web/src/components/`. Each file exports one or more Leptos `#[component]` functions. Re-export new components in `web/src/components/mod.rs`.

Subdirectories under `components/` group related pieces (e.g., `components/global/` for layout-level elements).

### Pages

Pages live in `web/src/pages/`. Each page is a top-level Leptos component rendered by the router. Subdirectories group page-specific sub-components (e.g., `pages/project/components/`).

Routes are declared in `web/src/lib.rs` inside the `<Routes>` block:

```rust
<Route path=StaticSegment("projects") view=ProjectsPage />
<Route path=path!("projects/:id") view=ProjectPage />
```

### Domain Re-exports

`web/src/domain/` re-exports types and functions from `dpe-core`. The server crate imports domain items through `dpe_web::domain` so there is a single import path.

## Running Tests

```bash
just test                              # All workspace tests
cargo test -p dpe-server               # Single crate
cargo test -p dpe-server test_name     # Specific test
```

### Snapshot tests (insta)

```bash
cargo test -p dpe-server               # Run tests — failing snapshots produce .snap.new files
cargo insta review                     # Interactive review of new/changed snapshots
cargo insta accept                     # Accept all pending snapshots
```

### E2E tests (Playwright)

```bash
cd modules/dpe/web-e2e-tests && npx playwright test
```

## When Making Changes

### Adding a New Component

1. Create file in `web/src/components/`
2. Define component with `#[component]` macro
3. Export in `web/src/components/mod.rs`
4. Import in the relevant page or layout

### Adding a New Page

1. Create file in `web/src/pages/`
2. Define page component
3. Export in `web/src/pages/mod.rs`
4. Add route in `web/src/lib.rs` inside `<Routes>`

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

- Stylesheet link is in `web/src/lib.rs`: `<Stylesheet id="leptos" href="/pkg/dpe.css" />`
- Ensure Tailwind build runs (automatic with `just watch-dpe`)
- Check DaisyUI theme configuration in `style/main.css`

### Hot-Reload Comments in Fragments

In dev mode, Leptos wraps output in `<!--hot-reload|...|-->` comments. These break Datastar morphing. Always pass fragment HTML through `strip_hot_reload_comments()`.

## Best Practices for AI Agents

1. **Read existing code first** before making changes to understand patterns
2. **Maintain consistency** with existing component and file structure
3. **Use Leptos 0.8.x syntax** — many online examples target 0.7.x
4. **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types/components
5. **Use `just check`** to verify formatting and linting before considering work done
6. **Use mosaic-tiles components** where appropriate for consistent UI
7. **Check Tailwind v4 syntax** — some classes changed from v3

## Note on Mosaic Playground

The Mosaic component playground (`modules/mosaic/playground/`) is a separate application that does use Leptos islands and WASM. That architecture does not apply to DPE. Do not conflate the two.
