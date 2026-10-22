# AI Agent Guide for the Metadata Editor

Editor-specific guidance; project-wide guidance is in the top-level `CLAUDE.md`, and DPE's is in `areas/access/dpe/CLAUDE.md`.

## Project Overview

The editor is the surface where a depositing project edits its own metadata and RDU reviews the result. It is server-side rendered with **Maud** (`maud::html!` → `Markup`) and served by **Axum**, exactly like DPE. **Datastar** adds progressive enhancement on top of forms that already work without it — there is no WASM, no hydration and no islands. Persistence is **SQLite** (`editor-store`, with identity's own file in `identity-store`), not the file-backed cache DPE reads.

## Architecture and Structure

- **Architecture (the flow, the crates, deployment and trust boundaries)**: See `docs/src/editor/architecture.md`
- **URL scheme, method discipline, the two renderings of a write**: See `docs/src/editor/routing.md`
- **Datastar, styling and the accessibility decisions**: See `docs/src/editor/rendering.md`
- **Persistence (pools, PRAGMAs, schema)**: See `docs/src/editor/persistence.md`
- **The draft, the published set and the canonical writer**: See `docs/src/editor/project-representation.md`
- **Entity proposals**: See `docs/src/editor/entity-proposals.md`
- **The review surface**: See `docs/src/editor/review.md`
- **Project state and Online detection**: See `docs/src/editor/status.md`
- **Authentication and sessions**: See `docs/src/editor/authentication.md`
- **The project form**: See `docs/src/editor/project-form.md`
- **Publishing to Git (the approved-records endpoint and the pull-request run)**: See `docs/src/editor/publishing-to-git.md`
- **Operations (Docker, env vars, CLI)**: See `docs/src/editor/operations.md`
- **Observability**: See `docs/src/editor/observability.md`

Five crates: `editor-core` (`core/`, the draft model, the form's field registry and rules, validation and canonical writer), `editor-ports` (`ports/`, what the editor asks of identity: `Actor`, `Authenticator`, `UserDirectory`; `std` only), `editor-store` (`store/`, the SQLite implementation of `editor-core`'s repository ports, file `editor.sqlite`, and `LiveOpenWork` behind `identity-ports`), `editor-web` (`web/`, the views, the editor's handlers and routes, and the `Editor` facade `deposit-server` starts and mounts; the document shell around the views is the area's `deposit-shell`, `areas/deposit/shell`) and `editor-pull-requests` (`pull-requests/`, the CI binary that turns an approved record into a pull request — not part of the service's request path; see `docs/src/editor/publishing-to-git.md`). Identity — accounts, sessions, login codes and their rules — is in `identity-core` (`areas/deposit/identity/core`), `identity-ports` (`areas/deposit/identity/ports`), `identity-store` (`areas/deposit/identity/store`, its SQLite file `identity.sqlite` and the mail transports) and `identity-web` (`areas/deposit/identity/web`, the login and depositor pages, their handlers and routes, and the `Identity` facade `deposit-server` starts and mounts), with its own guide in `areas/deposit/identity/CLAUDE.md`; the startup-configuration pitfalls of the login flow are there. `editor-core` and `editor-web` never depend on an identity crate: an account fact the editor needs (a display name, a role, the session's deadline) reaches it through `editor-ports` (`Actor`, `UserDirectory`), and only the Deposit Area's composition root, `deposit-server` (`areas/deposit/server`, with its own guide in `areas/deposit/server/CLAUDE.md`), names both capabilities' crates.

## Code Organization Patterns

### Views (`editor-web`)

Views are `fn(...) -> maud::Markup`: pages in `web/src/pages/`, the form's widgets (the rendered controls) in `web/src/form/widgets.rs`; the field registry, obligations and per-field submit rules are `editor-core`'s (`editor_core::form::{registry, obligation, submit}`). They use no Axum; the handlers that call them are in `web/src/handlers/`. The document they are wrapped in (header, footer, the 403, 404 and problem pages) and the sign-in link are the area's, in `deposit-shell` (`areas/deposit/shell`), which depends on no capability crate. Accessibility is the **tile's** responsibility, not the caller's — an a11y failure here usually means a missing semantic method on a `mosaic-tiles` tile rather than a missing attribute at the call site.

### Routing (`editor-web`, `deposit-server`)

The editor's routes are one table in `web/src/routes.rs`, which `Editor::router` folds; identity's are `identity-web`'s `routes.rs`. `areas/deposit/server/src/router.rs` merges the two routers and adds the layers, deliberately separate from `serve()` so the assembly is unit-testable. The invariants:

- **The editor is root-mounted.** It runs on its own hostname, so there is no `/editor` prefix. A shared origin would defeat the `Sec-Fetch-Site` CSRF control.
- **The root declares no capability route.** `build_router` merges `Editor::router()` and `Identity::router()` before the OTel layers, so every page is traced and under the CSRF layer.
- **The traced/untraced split is positional.** `/healthz` and `/telemetry/collect` are declared in `build_app` *after* `build_router`'s `.layer()` calls. Moving one line silently mints a span per liveness probe; tests pin it.
- **Machine-facing routes live under `/api/v1/`.** Everything else is a browser route serving the page shell. An `/api/v1` route returns JSON or a bare status with a plain-text body, never HTML, because its callers are CI jobs and scripts; it is declared in `web/src/routes.rs`, which the root wraps in the OTel layers, so it is traced, and it is unauthenticated, never read by session cookie.

### Authorization

`Authenticated` and `Rdu` are **extractors, not middleware**. A handler that omits them is public, visibly, in its signature. `Rdu` composes `Authenticated`, so RDU routes are closed twice over. An editor handler takes the pair in `web/src/handlers/auth.rs`, which sees identity only through the `Authenticator` port and yields an `Account` (the `Actor`, its id parsed); names behind account ids come from `UserDirectory`, never from identity's repositories. The pair in `identity-web`'s `handlers/guard.rs` is identity's, for its own pages.

### Method discipline

Every route that changes state is `POST`. The `Sec-Fetch-Site` CSRF control must exempt `GET` by necessity — a navigation from anywhere is a `GET` — so a `GET` that writes is a `GET` nothing protects. Every write URL also answers `GET`, so a rejected submission re-renders somewhere bookmarkable instead of stranding on a bare 405.

## Running Tests

- Rust: `cargo test -p deposit-shell -p identity-core -p identity-ports -p identity-store -p identity-web -p editor-core -p editor-ports -p editor-store -p editor-web -p deposit-server`
- E2E, accessibility and the no-JavaScript pass: `just test-e2e-editor` / `just test-a11y-editor` (suite in `areas/deposit/editor/web-e2e-tests/`)

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

`areas/deposit/editor/public/assets/app.css` is **gitignored and built**: `just css-editor` (dev, unhashed) or `just css-editor-release` (content-hashed, discovered by scanning the asset dir at startup). A stale local build makes a Tailwind-dependent assertion fail for a reason that has nothing to do with the code under test — `position: sticky` with every offset `auto` is inert, and that is what an absent `top-0` produces.

### A schema change edits `0001` until the first deployment

Each store holds one baseline, `areas/deposit/identity/store/src/migrations/` and `areas/deposit/editor/store/src/migrations/`. Until the first production deployment (DEV-6921), change the schema by editing the affected store's `0001_initial.sql` in place; from that deployment on, both `0001` files are frozen and every change is a new `0002_*`, `0003_*`, … migration. Adding a numbered file before then records a history nobody lived through; editing `0001` after then leaves every deployed database on the old shape. See `docs/src/editor/persistence.md#schema`.

## The architecture docs describe the code as it is

`docs/src/editor/architecture.md`, `ARCH-MAP.md` and the `CONTEXT.md` files always describe the code as implemented, even where that disagrees with the target. The target lives only in the decision records, `areas/deposit/docs/adr/` (`areas/deposit/ADR-0004` is the shape). Whenever the two disagree, the ADR names the gap in a `Not yet in code:` line directly above its `Enforced by:` line, one per gap, saying in code terms what does not match and which Linear issue tracks it, or `(not scheduled)` (`docs/adr/CLAUDE.md`). Every refactor step updates the docs to the new as-built state, and the commit that closes a gap deletes its line. A reviewer then sees, per PR, both what changed and how far the code still is from the target. Never describe the target in the docs as though it were built, and never leave the line of a closed gap in an ADR.

## Depositor-facing vocabulary is normative

REQ-2.1 closes the state list to exactly five — Draft, Submitted, In review, Approved, Online — and REQ-2.2 forbids the words "export", "JSON", "transfer", "commit" and "pull request" anywhere a depositor reads. Both live on `editor_core::status::ProjectState`, which is what the list column, the `/states` page and the waiting-for-release notice all read, so the three cannot drift.

`areas/deposit/editor/web/tests/depositor_vocabulary.rs` asserts the forbidden words against **rendered markup with tags stripped**, and the E2E suite asserts them again against real pages. Both layers are needed, and the reason is structural: a rendering test exercises one view function in isolation and cannot see a string the server assembles into a slot that test left empty. Only the browser pass reads the assembled page. This is not hypothetical — it is how the "pull request" wording above survived until DEV-6917 added the browser pass.

RDU-facing strings are not bound by REQ-2.2. A reviewer needs the mechanism named.

## Publishing holds two invariants

`docs/src/editor/publishing-to-git.md` is the contract; these two are the parts a change here can quietly break.

**Nothing served or published depends on pull-request state.** `GET /api/v1/approved-records` applies no filter at all — it enumerates with `ApprovedRecordRepository::list_all`, and a test pins it by serving every record.

**The editor holds no GitHub or CI credential.** It is never told what the pull-request run did, presents nothing to GitHub and makes no outbound GitHub call.

## Observability

Shares `shared-telemetry` with DPE. `POST /telemetry/collect` is untraced and rate-limited per client IP, keyed on the **rightmost** `X-Forwarded-For` entry — the leftmost is client-forgeable. `areas/deposit/server/src/page_url.rs` normalizes the `page.url` attribute to the template of the route axum matches, built from `editor_web::route_paths()` (`web/src/routes.rs`) and `identity_web::route_paths()`, so a route declared in either table is labelled without further edits. A route added with `.route()` outside those tables is not.

## Best Practices for AI Agents

1. **Use `mosaic-tiles` components** where appropriate, and add the semantic method to the tile rather than patching ARIA at the call site
2. **Format with `just fmt`** — `maudfmt` then `cargo +nightly fmt`. Run at the end of your work
