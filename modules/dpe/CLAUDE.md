# AI Agent Guide for DPE

This document provides DPE-specific guidance for AI coding assistants. For project-wide guidance, see the top-level `CLAUDE.md`.

## Project Overview

DPE is a server-side rendered web application. Pages are rendered on the server as plain HTML with **Maud** (`maud::html!` → `Markup`) and served by **Axum**. Interactive behavior (tab switching, live search) is handled by **Datastar**, which streams HTML fragments over SSE. There is no client-side WASM, no hydration, and no islands architecture — the server is the single source of truth for UI state.

## Architecture and Structure

- **Crate structure and dependency graph**: See `docs/src/dpe/project_structure.md`
- **Architecture, rendering model, and Datastar conventions**: See `docs/src/dpe/architecture.md`
- **Testing strategy and conventions**: See `docs/src/dpe/testing-strategy.md`
- **Operations (Docker, env vars, CLI)**: See `docs/src/dpe/operations.md`

## Key Technologies and Versions

- **Maud**: compile-time HTML templates (`html!` → `Markup`)
- **Axum**: native routing, `ServeDir` static serving, SSE
- **Datastar**: SSE-based interactivity via the `datastar` crate (0.3)
- **Tailwind CSS**: 4.x (single invocation, no DaisyUI)
- **Rust Edition**: 2021
- **Runtime**: Tokio

## Code Organization Patterns

### Views (`dpe-web`)

`dpe-web` is a plain library crate of view functions. Components live in `web/src/components/` and pages in `web/src/pages/`; each is a `fn(...) -> maud::Markup`. Split aggressively into small partials. Subdirectories group related pieces (e.g., `pages/project/components/`). There is no component macro and no re-export shim — import `dpe-core` types directly.

### Routing, head, and the page shell (`dpe-server`)

`dpe-server` is the composition root. Routes are declared in `server/src/main.rs` with the native Axum router:

```rust
.route("/dpe/projects", get(projects_page_handler))
.route("/dpe/projects/{id}", get(project_page_handler))
.route("/dpe/projects/{id}/tab/{tab}", get(fragments::tab_fragment_handler))
```

The `<head>` and outer HTML document are hand-written in `server/src/view.rs` (`head()` + `page()`): charset/viewport, the conditional `traceparent` meta tag, Google Fonts, the content-hashed stylesheet `<link>`, conditional Fathom, and the Datastar + telemetry scripts. Static assets are served by `tower_http::ServeDir` with an Axum 404 fallback.

### Fragments (Datastar SSE)

Fragment handlers live in `server/src/fragments.rs`. Each renders a Maud view to a string and returns a `Sse` stream of Datastar `PatchElements` (and optionally `ExecuteScript`) events. The `#project-tabs` morph root is rendered by the single `project_tabs` function in `dpe-web`, used by **both** the full page and the `/tab/{tab}` SSE route, so the two can never drift (a test pins this contract).

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

1. Add a `fn name(...) -> maud::Markup` in `web/src/components/`
2. Re-export it in `web/src/components/mod.rs`
3. Call it from the relevant page or layout
4. Add a unit test rendering the partial and asserting on its output

### Adding a New Page

1. Add a `fn page(...) -> maud::Markup` in `web/src/pages/`
2. Export it in `web/src/pages/mod.rs`
3. Add a handler in `dpe-server` that loads data and renders the page inside the `page()` shell
4. Register the route in `server/src/main.rs`

### Adding a New Fragment Handler

1. Add the async handler in `server/src/fragments.rs`
2. Register the route in `server/src/main.rs`
3. Render the relevant `dpe-web` view function and `.into_string()` its `Markup`
4. Return a `Sse` stream of `PatchElements` (and optionally `ExecuteScript`) events

### Modifying Styles

- The single Tailwind entry is `style/main.css` (imports `tokens.css` + the Mosaic component CSS). Tailwind content-scans both `dpe-web` and `dpe-server`.
- Use Tailwind utility classes and the Mosaic component classes in the Maud markup; there is no DaisyUI.
- Rebuild the stylesheet with `just css` (dev, unhashed `public/assets/app.css`) or `just css-release` (content-hashed). After CSS-affecting changes, grep the built CSS for the expected classes — a class that resolves to nothing is the common footgun.

## Common Pitfalls

### Maud + Datastar attribute syntax

- Colon/hyphen attribute names are written bare (`data-on:click__prevent`, `data-bind:search`).
- **Dotted** names (`data-on:input__debounce.300ms`) must be a quoted string-literal attribute name — Maud only allows `:`/`-` between bare name fragments.
- Maud HTML-escapes literal attribute values (`&` → `&amp;`); the browser decodes them back for Datastar — semantically identical to hand-written HTML.

### Nested `html!` as a function argument

Don't pass a non-trivial `html!` block directly into a component call
(`card(html! { … })`). Bind it to a Rust `let` first
(`let body = html! { … }; card(body)`) or extract a `fn … -> Markup`
helper. `maudfmt` only formats `html!` at Rust statement/`let` position; a block
nested as a call argument — or via Maud's in-macro `@let x = html! { … }` — is
skipped by `maudfmt` and then mangled by `cargo fmt` (flat indentation,
`class = "…"` with stray spaces). Trivial one-liners like `html! { (label) }`
passed inline are fine.

### Escaping

Default `(expr)` splices auto-escape. The only sanctioned `PreEscaped` site is the trusted mosaic `IconData` SVG. The search-query echo (`fragments.rs`) must stay a plain auto-escaped splice — never `PreEscaped` (the one realistic XSS reintroduction).

### Styling not applying

- The stylesheet link is emitted by `head()` in `server/src/view.rs`; the href is `/assets/app.css` in dev and the discovered `app.<hash>.css` in release.
- Ensure the Tailwind build ran (`just css`, or automatic under `just dev`).

## Observability

DPE uses OpenTelemetry for distributed tracing, metrics, and structured logging. See `docs/src/dpe/observability.md` for the full developer guide.

- **OTel middleware**: `OtelAxumLayer` creates `SPAN_KIND_SERVER` spans for HTTP requests. Use `otel.kind = "internal"` on handler-level `#[instrument]` spans.
- **Telemetry collector**: `POST /telemetry/collect` is placed after the OTel layers (untraced). It converts browser beacons into OTel metrics and structured logs. Types and validation live in the `dpe-telemetry` crate; the collector in `dpe-server` handles OTel conversion.
- **Vendored JS**: Client-side dependencies live in `modules/dpe/public/vendor/`, tracked by `vendor/README.md`. No `package.json` or Node.js build step for the runtime.
- **Traceparent**: The server renders `<meta name="traceparent">` in the HTML shell for client-side trace correlation, and injects a `traceparent` response header.

## Best Practices for AI Agents

1. **Read existing code first** before making changes to understand patterns
2. **Maintain consistency** with the existing partial/file structure
3. **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types
4. **Use `just check`** (fmt + clippy) and `just test` to verify before considering work done
5. **Use `mosaic-tiles` components** where appropriate for consistent UI
6. **Format with `just fmt`** — runs `maudfmt` (formats `maud::html!` macro contents; stock rustfmt does not) then `cargo +nightly fmt`. Run at the end of your work

## Note on the Mosaic Playground

The Mosaic component playground (`modules/mosaic/playground/`) is a separate plain Axum + Maud application. See `modules/mosaic/CLAUDE.md` for its guide.
