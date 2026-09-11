# AI Agent Guide for the Metadata Editor

Editor-specific guidance; project-wide guidance is in the top-level `CLAUDE.md`, and DPE's is in `modules/dpe/CLAUDE.md`.

## Project Overview

The editor is the surface where a depositing project edits its own metadata and RDU reviews the result. It is server-side rendered with **Maud** (`maud::html!` → `Markup`) and served by **Axum**, exactly like DPE. **Datastar** adds progressive enhancement on top of forms that already work without it — there is no WASM, no hydration and no islands. Persistence is **SQLite** (`editor-server/src/db/`), not the file-backed cache DPE reads.

## Architecture and Structure

- **Architecture, URL scheme and Datastar conventions**: See `docs/src/editor/architecture.md`
- **Authentication and sessions**: See `docs/src/editor/authentication.md`
- **The project form**: See `docs/src/editor/project-form.md`
- **Operations (Docker, env vars, CLI)**: See `docs/src/editor/operations.md`
- **Observability**: See `docs/src/editor/observability.md`

Three crates: `editor-core` (`core/`, the draft model, validation and canonical writer), `editor-web` (`web/`, view functions) and `editor-server` (`server/`, the composition root).

## Code Organization Patterns

### Views (`editor-web`)

A plain library crate of `fn(...) -> maud::Markup`. Pages in `web/src/pages/`, components in `web/src/components/`, the form's field registry and widgets in `web/src/form/`. Accessibility is the **tile's** responsibility, not the caller's — an a11y failure here usually means a missing semantic method on a `mosaic-tiles` tile rather than a missing attribute at the call site.

### Routing (`editor-server`)

Routes are assembled in `server/src/router.rs`, deliberately separate from `serve()` so the routing is unit-testable. Two invariants that module owns:

- **The editor is root-mounted.** It runs on its own hostname, so there is no `/editor` prefix. A shared origin would defeat the `Sec-Fetch-Site` CSRF control.
- **The traced/untraced split is positional.** `/healthz` and `/telemetry/collect` are declared in `build_app` *after* `build_router`'s `.layer()` calls. Moving one line silently mints a span per liveness probe; tests pin it.

### Authorization

`Authenticated` and `Rdu` are **extractors, not middleware**. A handler that omits them is public, visibly, in its signature. `Rdu` composes `Authenticated`, so RDU routes are closed twice over.

### Method discipline

Every route that changes state is `POST`. The `Sec-Fetch-Site` CSRF control must exempt `GET` by necessity — a navigation from anywhere is a `GET` — so a `GET` that writes is a `GET` nothing protects. Every write URL also answers `GET`, so a rejected submission re-renders somewhere bookmarkable instead of stranding on a bare 405.

## Running Tests

- Rust: `cargo test -p editor-core -p editor-web -p editor-server`
- E2E, accessibility and the no-JavaScript pass: `just test-e2e-editor` / `just test-a11y-editor` (suite in `modules/editor/web-e2e-tests/`)

The E2E suite runs **twice**, once with JavaScript on and once with `javaScriptEnabled: false`. Neither pass subsumes the other: the no-JS pass catches a control with no submit button behind it, and the JS pass catches a control that Datastar has made inert.

## Common Pitfalls

### A row control is a submit button carrying `formaction`

Datastar calls `preventDefault` on the form's `submit` unconditionally, which **discards the submitter's URL**. A `data-on:submit` handler that posts the form's own `action` therefore turns every add and remove control on the page into a plain save, while the no-JS path keeps working perfectly. Post `evt.submitter?.formAction` instead. Server tests, rendering tests and snapshots all pass through this bug — only a browser with JavaScript enabled catches it.

### Datastar attribute syntax

The delimiter is a **colon**, not a hyphen: `data-on:submit`, `data-attr:disabled`. The pre-RC.6 hyphen forms (`data-on-`, `data-attr-`, `data-class-`, `data-style-`) are inert — the failure mode is a console error and a dead control that still renders fine and still passes any snapshot test asserting the attribute is present.

Dotted names (`data-on:change__debounce.1s`) must be written as a quoted string-literal attribute name; Maud only allows `:`/`-` between bare name fragments.

### Selecting a submit button in a test

The shell header renders `<form method="post" action="/logout">` on every signed-in page, above `<main>`. A bare `button[type="submit"]` selector therefore clicks **Sign out**. Scope to `main`, select by `formaction`, or name the button.

### Styling not applying

`modules/editor/public/assets/app.css` is **gitignored and built**: `just css-editor` (dev, unhashed) or `just css-editor-release` (content-hashed, discovered by scanning the asset dir at startup). A stale local build makes a Tailwind-dependent assertion fail for a reason that has nothing to do with the code under test — `position: sticky` with every offset `auto` is inert, and that is what an absent `top-0` produces.

### Configuration refuses unsafe combinations at startup

`EditorConfig::validate` stops the process rather than starting in a state that locks users out or leaks. Two that bite in test setups: `EDITOR_ENV=PROD` requires `EDITOR_SMTP_HOST` (a console mailer in production is a standing credential leak), and `EDITOR_LOGIN_COOLDOWN_SECS` must be **at least 1** — it cannot be set to zero.

With `EDITOR_SMTP_HOST` unset the console transport writes login codes to the log, which is how development, the PR preview and the E2E suite sign in.

## Depositor-facing vocabulary is normative

REQ-2.1 closes the state list to exactly five — Draft, Submitted, In review, Approved, Online — and REQ-2.2 forbids the words "export", "JSON", "transfer", "commit" and "pull request" anywhere a depositor reads. Both live on `editor_core::status::ProjectState`, which is what the list column, the `/states` page and the waiting-for-release notice all read, so the three cannot drift.

`modules/editor/web/tests/depositor_vocabulary.rs` asserts the forbidden words against **rendered markup with tags stripped**, and the E2E suite asserts them again against real pages. Both layers are needed, and the reason is structural: a rendering test exercises one view function in isolation and cannot see a string the server assembles into a slot that test left empty. Only the browser pass reads the assembled page. This is not hypothetical — it is how the "pull request" wording above survived until DEV-6917 added the browser pass.

RDU-facing strings are not bound by REQ-2.2. A reviewer needs the mechanism named.

## Observability

Shares `platform-telemetry` with DPE. `POST /telemetry/collect` is untraced and rate-limited per client IP, keyed on the **rightmost** `X-Forwarded-For` entry — the leftmost is client-forgeable. `server/src/page_url.rs` normalizes the `page.url` attribute; a new full-page route needs a matching entry there or its page views collapse into `other`, and no test fails.

## Best Practices for AI Agents

1. **Use `just check`** (fmt + clippy) and `just test` before considering work done
2. **Use `mosaic-tiles` components** where appropriate, and add the semantic method to the tile rather than patching ARIA at the call site
3. **Format with `just fmt`** — `maudfmt` then `cargo +nightly fmt`. Run at the end of your work
