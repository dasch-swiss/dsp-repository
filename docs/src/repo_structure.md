# Project Structure and Code Organization

## Overview

This repository is a Rust workspace structured as a monorepo. All Rust crates are organized as subdirectories within the `modules/` directory.

```txt
modules/
├── dpe/                       # Discovery and Presentation Environment
│   ├── core/                  # Pure domain types, repositories, data loading (crate: dpe-core)
│   ├── api-oai/               # OAI-PMH 2.0 API (crate: dpe-api-oai)
│   ├── web/                   # Web layer: Maud pages and components (crate: dpe-web)
│   ├── server/                # Server binary: route composition, Datastar fragments (crate: dpe-server)
│   ├── web-e2e-tests/         # Playwright E2E tests
│   ├── public/                # Static assets
│   ├── style/                 # CSS / Tailwind
│   └── Dockerfile             # Production container image
├── editor/                    # Metadata editor (authenticated; depositor authoring + RDU review)
│   ├── core/                  # Pure domain types (crate: editor-core)
│   ├── web/                   # View layer: document shell, Maud pages and components (crate: editor-web)
│   ├── server/                # Server binary: config, observability, route composition (crate: editor-server)
│   ├── public/                # Static assets (incl. vendored JS)
│   ├── style/                 # CSS / Tailwind
│   └── Dockerfile             # Production container image
├── platform/                  # Crates shared by more than one service
│   └── telemetry/             # Browser beacon contract + collector endpoint (crate: platform-telemetry)
└── mosaic/                    # Mosaic component library (design system)
    ├── tiles/                 # Reusable Maud UI components (crate: mosaic-tiles)
    ├── playground/            # Component playground application (crate: mosaic-playground)
    └── playground-e2e-tests/  # Playwright E2E tests for the playground
```

## Crate and Folder Naming Convention

**Crate names** follow the `{module}-{role}` pattern. **Folder names** strip the module prefix, keeping only the role part.

| Crate | Folder | Role |
|-------|--------|------|
| `dpe-core` | `dpe/core` | Pure domain types and data access (zero framework deps) |
| `dpe-api-oai` | `dpe/api-oai` | OAI-PMH 2.0 API (depends on `dpe-core` only) |
| `dpe-web` | `dpe/web` | Maud pages and components (`fn -> Markup`) |
| `dpe-server` | `dpe/server` | Server binary — composes all routes |
| `platform-telemetry` | `platform/telemetry` | Browser beacon contract, validation, and the collector endpoint — shared by DPE and the editor |
| `editor-core` | `editor/core` | Pure domain types for the editor (zero framework deps) |
| `editor-web` | `editor/web` | Editor view layer, including the HTML document shell |
| `editor-server` | `editor/server` | Editor binary — composes all routes |
| `mosaic-tiles` | `mosaic/tiles` | Reusable UI component library |
| `mosaic-playground` | `mosaic/playground` | Component showcase application |

## Shared Crates

**A crate that more than one service depends on lives under `modules/platform/`, never inside a service module.** As soon as a second service takes a dependency on it, move it and rename it to `platform-{role}` in the same commit.

The directory is the ownership signal, and four things read it:

- **CI path filters.** The path-filtered workflows key on a module glob — `modules/dpe/**` for DPE's preview, Scout and a11y jobs, `modules/editor/**` for the editor's. A shared crate left under one service's module is invisible to every other service's jobs: no preview deployed, no image scanned. (`check`, `test` and `gate` carry no path filters, so compilation and tests are never the gap — which is what makes this easy to miss.)
- **Dev-loop watch lists.** `bacon.toml`'s `serve` and `serve-editor` jobs each watch their own module directory. A shared crate outside `modules/platform/` stops triggering a rebuild for whichever service does not own it, with no error — you keep testing a stale binary.
- **Directory-scoped agent instructions.** `modules/dpe/CLAUDE.md` governs everything under `modules/dpe/`, so a shared crate parked there takes its rules from one service's file. Repo-wide files are not directory-scoped, but they do accumulate crate-specific lines — `REVIEW.md` carries two, one pointing into `dpe-server`'s `page_url.rs` and one into `editor-server`'s — and those need to name the real location.
- **The dependency direction.** `platform-*` crates depend on no service crate. Anything that needs to know about one service's routes, data or configuration does not belong in one — pass it in as a parameter instead.

`mosaic-*` predates the convention and stays as it is: it is already a peer of the services rather than a child of one, which is the property that matters.

## API Crate Pattern

Each API is a separate crate under `modules/dpe/`:

- **Naming**: `dpe-api-{name}` (e.g., `dpe-api-oai`)
- **Dependencies**: `dpe-core` for domain types; never depends on other API crates or `dpe-web`
- **Entry point**: Exports a handler function (e.g., `pub async fn oai_handler(...)`)
- **Composition**: `dpe-server` wires the handler into the Axum router

For detailed crate responsibilities and the dependency graph, see [DPE Project Structure](./dpe/project_structure.md).
