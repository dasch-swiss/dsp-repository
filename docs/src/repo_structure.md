# Project Structure and Code Organization

## Overview

This repository is a Rust workspace structured as a monorepo. All Rust crates are organized as subdirectories within the `modules/` directory.

```txt
modules/
├── dpe/                       # Discovery and Presentation Environment
│   ├── core/                  # Pure domain types, repositories, data loading (crate: dpe-core)
│   ├── api-oai/               # OAI-PMH 2.0 API (crate: dpe-api-oai)
│   ├── web/                   # Web layer: Leptos components, pages (crate: dpe-web)
│   ├── server/                # Server binary: route composition, Datastar fragments (crate: dpe-server)
│   ├── web-e2e-tests/         # Playwright E2E tests
│   ├── public/                # Static assets
│   ├── style/                 # CSS / Tailwind
│   └── Dockerfile             # Production container image
└── mosaic/                    # Mosaic component library (design system)
    ├── tiles/                 # Reusable Leptos UI components (crate: mosaic-tiles)
    ├── playground/            # Component playground application (crate: mosaic-playground)
    ├── playground_macro/      # Proc macro for playground page generation (crate: mosaic-playground-macro)
    └── playground-e2e-tests/  # Playwright E2E tests for the playground
```

## Crate and Folder Naming Convention

**Crate names** follow the `{module}-{role}` pattern. **Folder names** strip the module prefix, keeping only the role part. Hyphens in crate names become underscores in folder names when needed for Rust compatibility (proc macro crates).

| Crate | Folder | Role |
|-------|--------|------|
| `dpe-core` | `dpe/core` | Pure domain types and data access (zero framework deps) |
| `dpe-api-oai` | `dpe/api-oai` | OAI-PMH 2.0 API (depends on `dpe-core` only) |
| `dpe-web` | `dpe/web` | Leptos SSR components, pages, `#[server]` functions |
| `dpe-server` | `dpe/server` | Server binary — composes all routes |
| `mosaic-tiles` | `mosaic/tiles` | Reusable UI component library |
| `mosaic-playground` | `mosaic/playground` | Component showcase application |
| `mosaic-playground-macro` | `mosaic/playground_macro` | Proc macro for playground page generation |

## API Crate Pattern

Each API is a separate crate under `modules/dpe/`:

- **Naming**: `dpe-api-{name}` (e.g., `dpe-api-oai`)
- **Dependencies**: `dpe-core` for domain types; never depends on other API crates or `dpe-web`
- **Entry point**: Exports a handler function (e.g., `pub async fn oai_handler(...)`)
- **Composition**: `dpe-server` wires the handler into the Axum router

For detailed crate responsibilities and the dependency graph, see [DPE Project Structure](./dpe/project_structure.md).
