# Project Structure and Code Organization

## Overview

This repository is a Rust workspace structured as a monorepo. The service and design-system crates are organized as subdirectories within the `modules/` directory; the crates shared by more than one service sit in `shared/`, a sibling of it at the repository root.

Three files outside this book describe the repository for agents and reviewers and are kept current alongside the code: `ARCH-MAP.md` at the root (the component map — paths, public interfaces, dependency edges, boundary rules and their enforcement level), `CONTEXT.md` at the root plus one per bounded context (`modules/editor/CONTEXT.md`, `modules/dpe/CONTEXT.md`, `areas/archive/CONTEXT.md`) and one per shared engine (`vitrinli/CONTEXT.md`, `chischtli/CONTEXT.md`), holding the domain vocabulary, and `docs/adr/` (architecture decision records). Six ADRs are in place. Three describe where the layout below is headed: Bazel as the build system (ADR-0001), the three areas of the Trusted Repository grouped under `areas/` (`areas/deposit/`, `areas/archive/`, `areas/access/`), replacing `modules/` at the root, beside `shared/`, `mosaic/`, `vitrinli/`, `chischtli/` and `dsp-cli/` (ADR-0002), and one modulith per area, composed of capabilities behind consumer-defined ports (ADR-0003). Two bind how surfaces are built: every user-facing surface is a server-rendered hypermedia application (ADR-0004), and every Access-Area landing page is FAIR-assessable by machine (ADR-0005). One binds how the repository's own decision records are homed and cited: system-wide decisions live in the root `docs/adr/`, a root-level component's own decision history lives in its own `docs/adr/`, and a bare `ADR-NNNN` names the root series while `<component>/ADR-NNNN` names a component's (ADR-0006). All six are accepted; until the migration lands, this page describes the layout as it is.

```txt
modules/
├── dpe/                       # Discovery and Presentation Environment
│   ├── core/                  # DPE's view model, caches, repositories (crate: dpe-core)
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
│   ├── collector/             # CI binary: turns approved records into pull requests (crate: editor-collector)
│   ├── public/                # Static assets (incl. vendored JS)
│   ├── style/                 # CSS / Tailwind
│   └── Dockerfile             # Production container image
└── mosaic/                    # Mosaic component library (design system)
    ├── tiles/                 # Reusable Maud UI components (crate: mosaic-tiles)
    ├── playground/            # Component playground application (crate: mosaic-playground)
    └── playground-e2e-tests/  # Playwright E2E tests for the playground

shared/                        # Crates shared by more than one service
├── fair/                      # FAIR exposure engine: resolved graphs + writers (crate: shared-fair)
├── metadata/                  # Research-metadata wire contract (crate: shared-metadata)
└── telemetry/                 # Browser beacon contract + collector endpoint (crate: shared-telemetry)

dsp-cli/                       # Command-line client for the DaSCH Service Platform (crate: dsp-cli); a root peer, not a service
```

## Crate and Folder Naming Convention

**Crate names** follow the `{module}-{role}` pattern. **Folder names** strip the module prefix, keeping only the role part. `dsp-cli` is an explicit exception: it is a single crate named after the product it publishes, not after a module-plus-role pair, and its folder keeps the crate's full name.

| Crate | Folder | Role |
|-------|--------|------|
| `dsp-cli` | `dsp-cli` | Command-line client for the DaSCH Service Platform — a root peer of the areas, not a service (ADR-0002) |
| `dpe-core` | `dpe/core` | DPE's view model, caches and repositories over the shared contract (zero framework deps) |
| `dpe-api-oai` | `dpe/api-oai` | OAI-PMH 2.0 API (depends on `dpe-core`, `shared-metadata` and `shared-fair` only) |
| `dpe-web` | `dpe/web` | Maud pages and components (`fn -> Markup`) |
| `dpe-server` | `dpe/server` | Server binary — composes all routes |
| `shared-fair` | `shared/fair` | The FAIR exposure engine: one resolved graph per published object and one writer per representation over it (ADR-0005) — `dpe-api-oai` is its only consumer today |
| `shared-metadata` | `shared/metadata` | The research-metadata wire contract and the rules for reading a value out of it — shared by DPE and the editor |
| `shared-telemetry` | `shared/telemetry` | Browser beacon contract, validation, and the collector endpoint — shared by DPE and the editor |
| `editor-core` | `editor/core` | Pure domain types for the editor (zero framework deps) |
| `editor-web` | `editor/web` | Editor view layer, including the HTML document shell |
| `editor-server` | `editor/server` | Editor binary — composes all routes |
| `editor-collector` | `editor/collector` | CI binary — collects approved records into pull requests |
| `mosaic-tiles` | `mosaic/tiles` | Reusable UI component library |
| `mosaic-playground` | `mosaic/playground` | Component showcase application |

## Shared Crates

**A crate that more than one service depends on lives under `shared/`, never inside a service module.** As soon as a second service takes a dependency on it, move it and rename it to `shared-{role}` in the same commit.

The directory is the ownership signal, and four things read it:

- **CI path filters.** The path-filtered workflows key on a module glob — `modules/dpe/**` for DPE's preview, Scout and a11y jobs, `modules/editor/**` for the editor's. A shared crate left under one service's module is invisible to every other service's jobs: no preview deployed, no image scanned. (`check`, `test` and `gate` carry no path filters, so compilation and tests are never the gap — which is what makes this easy to miss.)
- **Dev-loop watch lists.** `bacon.toml`'s `serve` and `serve-editor` jobs each watch their own module directory. A shared crate outside `shared/` stops triggering a rebuild for whichever service does not own it, with no error — you keep testing a stale binary.
- **Directory-scoped agent instructions.** `modules/dpe/CLAUDE.md` governs everything under `modules/dpe/`, so a shared crate parked there takes its rules from one service's file. Repo-wide files are not directory-scoped, but they do accumulate crate-specific lines — `REVIEW.md` carries two, one pointing into `dpe-server`'s `page_url.rs` and one into `editor-server`'s — and those need to name the real location.
- **The dependency direction.** `shared-*` crates depend on no service crate. Anything that needs to know about one service's routes, data or configuration does not belong in one — pass it in as a parameter instead. Both halves fail the build: a service dependency is a Cargo cycle, and a hardcoded path into another module is caught by `just check-shared-paths` (`.github/scripts/check-shared-paths.sh`).

`mosaic-*` predates the convention and stays as it is: it is already a peer of the services rather than a child of one, which is the property that matters.

**The rule covers the shareable part of a crate, not necessarily the whole crate.** `dpe-core` was not moved wholesale when the editor came to need the data contract: it mixes the contract with DPE's caches, repositories, cluster logic and a DSP-API HTTP client, and moving all of it would have made every one of those a shared concern. What moved is the contract — `shared-metadata` — and `dpe-core` now depends on it like any other consumer. Read the rule as *the shared thing lives under `shared/`*; where the shared thing is a subset of a crate, extract the subset.

**The rule covers crates, not content.** `modules/dpe/server/data` stays under DPE even though the editor consumes it: DPE owns it, and the editor reads an image-baked snapshot through an explicit `EDITOR_DATA_DIR` seam rather than a compiled-in path — see [Editor Operations](./editor/operations.md#data-directory-a-deliberate-build-input). Two of the four signals do have analogues there, accepted knowingly: a data-only change matches `modules/dpe/**`, so it deploys a DPE preview but no editor preview, and `serve-editor`'s watch list does not restart the editor on a data edit. Both become visible only once the editor reads records, and both are cheaper to fix in place than a wholesale move would be.

## API Crate Pattern

Each API is a separate crate under `modules/dpe/`:

- **Naming**: `dpe-api-{name}` (e.g., `dpe-api-oai`)
- **Dependencies**: `shared-metadata` for the contract, `dpe-core` for the view model, and any `shared-*` crate it needs — `dpe-api-oai` takes `shared-fair` for the DataCite and Dublin Core mappings; never depends on other API crates or `dpe-web`
- **Entry point**: Exports a handler function (e.g., `pub async fn oai_handler(...)`)
- **Composition**: `dpe-server` wires the handler into the Axum router

For detailed crate responsibilities and the dependency graph, see [DPE Project Structure](./dpe/project_structure.md).
