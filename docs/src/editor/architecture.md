# Editor Architecture

The metadata editor is where depositing project teams edit their project metadata, RDU reviews it field by field, and approved records are collected into a pull request against this repository. Git stays the source of truth.

This page describes the service as it stands. Surfaces that are not built yet are named where they affect a decision taken now, and marked as such.

## Relationship to DPE

The editor is a **separate service** from DPE, not a section of it. They share `platform-telemetry` for the browser-beacon contract, and will share `mosaic-tiles` for components and `dpe-core` for the data contract — but not a process, an image, or an origin.

The separation is deliberate:

- DPE is public, unauthenticated and read-only. The editor is authenticated and writes state. A host-level compromise of one should not hand over the other's session cookies.
- The editor's CSRF defence requires `Sec-Fetch-Site: same-origin` on every state-changing request. On a shared origin, a request originating from DPE *is* same-origin — so any XSS in DPE, which has a far larger unauthenticated attack surface, could drive authenticated editor mutations. A `Path` on a cookie is not a security boundary and does not close this.

## Rendering model

Same as DPE: server-rendered HTML with **Maud**, served by **Axum**, with **Datastar** for interactivity over SSE. No client-side WASM, no hydration, no islands. The server is the single source of truth for UI state.

## Crates

| Crate | Folder | Role |
|-------|--------|------|
| `editor-core` | `editor/core` | Pure domain types (no Axum, Maud or database dependency) |
| `editor-web` | `editor/web` | Maud view library — the document shell, pages and components |
| `editor-server` | `editor/server` | Composition root: configuration, observability, routing, persistence |

Dependency direction is `server → web → core`. Neither library depends on `mosaic-tiles` yet: the form widgets are the first surface to render a tile. The Tailwind entry already scans the crate, so component CSS ships regardless of the Cargo dependency.

Unlike DPE, the **HTML document shell lives in the view crate** (`editor-web/src/view.rs`), not the server crate. DPE keeps `head()` + `page()` in `dpe-server`; here the server is a composition root for routing, auth and persistence, and a document shell is a view concern like any other partial.

## URL scheme

Paths are **root-mounted**. There is no `/editor` prefix.

DPE carries `/dpe/…` because it shares `repository.dasch.swiss` with other services. The editor gets its own hostname, so a prefix buys nothing — and adopting one would keep alive the path-routing option this design rejects, for the CSRF reason above.

| Path | Method | Purpose |
|------|--------|---------|
| `/healthz` | GET | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | Browser telemetry beacon. Untraced, rate-limited per IP. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

The scheme the remaining surfaces will occupy, settled up front because the router and the shell are built against it:

```
GET  /                                        → redirect to /projects
GET  /login                                   login form
POST /login                                   issue a one-time code
POST /logout
GET  /projects                                the depositor's assigned projects
GET  /projects/{shortcode}                    → redirect to the first section
GET  /projects/{shortcode}/sections/{section}  one form section
POST /projects/{shortcode}/sections/{section}
GET  /review                                  the review queue, oldest first
GET  /review/{shortcode}                      the field-by-field diff surface
```

Two decisions inside that:

- **Form sections are real URLs**, not fragment swaps. Bookmarkable, Back-friendly, and consistent with the repository's URL-based-navigation principle.
- **Review deep-links by shortcode**, not by submission id. A project has at most one pending submission, so the shortcode is unique for the purpose and reads better in a URL shared between reviewers.

## Traced and untraced routes

An Axum layer wraps only routes declared **before** it. The router therefore has two halves:

- `build_router` — everything wrapped by `OtelInResponseLayer` then `OtelAxumLayer`. `OtelInResponseLayer` is declared first so it runs *inner* and injects the `traceparent` response header; `OtelAxumLayer` is declared second so it runs *outer* and creates the server span.
- `build_app` — adds `/healthz` and `/telemetry/collect` **after** those layers, so neither is traced. A liveness probe every 30 seconds and a telemetry upload on every page view would otherwise mint a span each and bury the real traffic.

That split is positional, so it is invisible in the route table and reversible by moving one line. A test asserts `/healthz` and the beacon are absent from `build_router`.

## Datastar

The editor vendors Datastar **1.0.2**; DPE is still on 1.0.0-RC.8, and the two vendor directories are independent. See `modules/editor/public/vendor/README.md`.

Two things to get right, both of which fail quietly:

- **Keyed plugin attributes use `:`, not `-`.** `data-on:click`, `data-attr:disabled`, `data-class:open`, and `data-init` rather than `data-on-load`. This has been true since RC.6, so it matches DPE's markup too. The hyphen form produces a console error and an inert control: the page renders fine and a snapshot test asserting the attribute is present still passes.
- **`__prevent` combines with `__debounce` / `__throttle`** on 1.0.x. The rule against that combination in `docs/src/dpe/architecture.md` is about RC.8 and does not apply here.

## Styling

`modules/editor/style/main.css` is the single Tailwind entry, built by `just css-editor` (dev) or `just css-editor-release` (content-hashed). It imports the design tokens and the `mosaic-tiles` component barrel.

`@import 'tailwindcss' source(none)` means classes are collected **only** from the explicit `@source` globs, which must cover every crate that emits Tailwind classes. A missing glob produces no build error — just markup whose classes resolve to nothing. After a change that adds classes in a new location, grep the built stylesheet for them.

New Mosaic tiles are added **demand-driven**: a screen that needs a missing primitive adds it to `mosaic-tiles` with a playground showcase and a unit test at that point, rather than an up-front form kit. Their CSS goes in `mosaic-tiles/src/components/components.css`, the barrel every consumer imports.

**Check a tile against the surface you are putting it on.** Tiles are styled for light backgrounds — `link` is `text-primary-600`, which measures 2.35:1 on the footer's `bg-slate-800` and fails WCAG 2.1 AA. That is why the footer uses plain anchors inheriting `text-gray-300` (9.93:1), as DPE's does. A dark-surface variant of a tile is a design-system change, so it belongs in `mosaic-tiles` with its own showcase rather than being worked around locally.
