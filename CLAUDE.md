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
- **Web UI**: [Leptos](https://book.leptos.dev) with [islands](https://book.leptos.dev/islands.html) for MPA style server-side rendering
- **Styling**: Tailwind CSS
- **Components**: mosaic-tiles component library
- **Architecture**: Clean Architecture / Hexagonal Architecture
- **Structure**: Cargo workspace with modular crates
- **Data Layer**: Repository pattern; currently backed by static JSON files in `/data/json/`
- **Testing**: Cargo test for unit/integration, Playwright for E2E
- **Documentation**: mdBook with alerts plugin

## Key Directories

```txt
modules/
├── leptos-dpe/            # Discovery and Presentation Environment (Leptos app)
│   ├── app/               # Shared app logic, components, pages, domain
│   ├── server/            # Server binary
│   ├── frontend/          # Client-side (WASM) entry point
│   ├── end2end/           # Playwright E2E tests
│   ├── public/            # Static assets
│   └── style/             # CSS / Tailwind
└── mosaic/                # Mosaic component library
    ├── tiles/             # Reusable Leptos UI components
    ├── demo/              # Component showcase application
    └── demo_macro/        # Proc macro for demo page generation
```

## Setup

### Prerequisites

- **Rust**: Toolchain managed via `rustup`
- **Just**: Command runner for development tasks
- **pnpm**: Package manager for the leptos-dpe frontend

### First-Time Setup

```bash
just install-requirements
```

## Development Commands (via justfile)

```bash
# Development
just watch-leptos-dpe          # Run DPE with hot reload
just watch-mosaic-demo         # Run Mosaic demo with hot reload
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
  Use `leptosfmt` for Leptos code (`modules/mosaic/`, `modules/leptos-dpe/`).
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
- MPA style server-side rendering with Leptos islands for interactivity
- Domain-driven design aligned with research data concepts

## Documentation Tone

- Keep the tone factual and understated.
  Documentation should be clear first of all; there is no need to praise the software.
