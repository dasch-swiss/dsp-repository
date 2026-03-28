# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

This repository is a Rust-based monorepo for the DaSCH Service Platform.
The platform provides services for a long-term archive for humanities research data.

This includes:

- The "Discovery and Presentation Environment" (DPE) — built with Leptos
- The "Mosaic" component library (tiles for components, demo for documentation)

## Architecture

- **Language**: Rust
- **Web Server**: Axum HTTP server with Tokio runtime
- **Web UI**: [Leptos](https://book.leptos.dev) SSR for pages, [Datastar](https://data-star.dev/) for interactivity
- **Interactivity**: Datastar SSE fragments (hypermedia-driven, no WASM for DPE)
- **Styling**: Tailwind CSS
- **Components**: mosaic-tiles component library
- **Architecture**: Clean Architecture / Hexagonal Architecture
- **Structure**: Cargo workspace with modular crates
- **Data Layer**: Repository pattern; currently backed by static JSON files in `modules/dpe/server/data/`
- **Testing**: Cargo test for unit/integration, Playwright for E2E
- **Documentation**: mdBook with alerts plugin

See `docs/src/dpe/architecture.md` for the full architecture description including the Datastar + Leptos SSR hybrid pattern, fragment route conventions, and HATEOAS tab pattern.

## Key Directories

```txt
modules/
├── dpe/                   # Discovery and Presentation Environment
│   ├── core/              # Pure domain types, repositories, data loading (no framework deps)
│   ├── api-oai/           # OAI-PMH 2.0 API crate (depends on dpe-core only)
│   ├── app/               # Web layer: Leptos components, pages, #[server] wrappers (crate: dpe-web)
│   ├── server/            # Server binary: route composition, Datastar fragments (crate: dpe-server)
│   ├── web-e2e-tests/     # Playwright E2E tests
│   ├── public/            # Static assets
│   └── style/             # CSS / Tailwind
└── mosaic/                # Mosaic component library
    ├── tiles/             # Reusable Leptos UI components (crate: mosaic-tiles)
    ├── demo/              # Component playground application (crate: mosaic-playground)
    └── demo_macro/        # Proc macro for demo page generation (crate: mosaic-playground-macro)
```

## Setup

### Prerequisites

- **Rust**: Toolchain managed via `rustup`
- **Just**: Command runner for development tasks
- **pnpm**: Package manager for the DPE frontend

### First-Time Setup

```bash
just install-requirements
```

## Development Commands (via justfile)

```bash
# Development
just watch-dpe                 # Run DPE with hot reload
just watch-mosaic-playground    # Run Mosaic playground with hot reload
just watch                     # Watch for changes and run tests
just run                       # Run server (release mode)

# Code quality
just check                     # Run fmt checks and clippy
just fmt                       # Format all Rust code (cargo fmt + leptosfmt)
just build                     # Build all targets
just test                      # Run all tests
just clean                     # Clean build artifacts

# Documentation
just docs-build                # Build mdBook documentation
just docs-serve                # Serve docs at localhost:3000

# Setup
just install-requirements      # Install required tools
```

## Code Quality

- **Formatting**:
  Defined in `.rustfmt.toml`, use Unix newlines.
  Use `leptosfmt` for Leptos code (`modules/mosaic/`, `modules/dpe/`).
- **Linting**: Strict clippy warnings
- **Testing**: Testing pyramid (unit → integration → E2E)
- **Git**: Rebase workflow, clean commit history

## Development Workflow

**Important:** Follow ALL the steps below during development.

- **Always check with the developer before each step** —
  Check in, instead of going down the wrong path.
- **Use `just` for all commands** —
  Use `just` instead of `cargo` or `npm`.
  If available, use claude-specific just commands.
- **Tests first** —
  Every code change should be accompanied by tests.
  Start with a test suite to define expected behavior.
  Check the tests with the developer to ensure correctness.

Before considering ANY change as "done":

- **Verify that changes compile and all checks pass** —
  Run `just check` and `just test`.
  This includes formatting and linting — no need to run these earlier.
- **Check if documentation needs updating** —
  Consider `/docs/src/`, the readme, and claude-files.
  Update documentation to reflect the changes.
- **Ask before committing** —
  Never run git add or commit without explicit permission.
- **PR creation workflow** —
  1. Create as draft: `gh pr create --draft`
  2. Assign to the requesting developer: `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`
  3. Include a "Review Notes" section mentioning that separate commits should be checked for easier review

## Testing Guidelines

- **Tests first**: Unless instructed otherwise, write tests before implementing.
- **Unit tests**: Write unit tests for all new functionality.
- **Useful tests**: Every test should verify meaningful behavior.
  Never write tests that verify the behavior of the Rust compiler or external libraries.
- **Helper functions**: In tests, prefer repetition over complicated setup.
  Only use helpers if they improve clarity.

## Temporary File Management

- **Temporary files**: Use `.claude/tmp/` for scratch files during a session.
- **Ask if unsure** whether a document should be temporary or permanent.
- **Temporary files are gitignored** and won't be tracked in version control.
- **Permanent documentation** goes in `/docs/src/`.

## Architecture Principles

- Clean separation between domain, business logic, and infrastructure
- Single responsibility per crate
- MPA style server-side rendering with Datastar for interactivity (DPE)
- Domain-driven design aligned with research data concepts
- HATEOAS: server returns complete components, server pushes URLs
- Graceful degradation: all interactive elements work without JavaScript

## Conventions and Review

- See `CONVENTIONS.md` for fragment route naming, Datastar attribute patterns, and test directory conventions
- See `REVIEW.md` for the code review checklist

## Documentation Tone

- Keep the tone factual and understated.
  Documentation should be clear first of all; there is no need to praise the software.
