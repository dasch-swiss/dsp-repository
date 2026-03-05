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
dsp-dpe/                   # Cargo workspace for the DPE
├── leptos-dpe/            # Leptos-based DPE application
│   ├── app/               # Shared app logic
│   ├── frontend/          # Client-side WASM
│   └── server/            # Axum server binary
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
# DPE development (via `just dsp-dpe <command>`)
just dsp-dpe watch-mosaic-demo     # Run Mosaic demo with hot reload
just dsp-dpe fmt                   # Format Rust code
just dsp-dpe check                 # Run fmt and clippy checks
just dsp-dpe test                  # Run all tests
just dsp-dpe build                 # Build all targets

# Documentation
just docs-build                # Build mdBook documentation
just docs-serve                # Serve docs at localhost:3000

# Setup
just install-requirements      # Install required tools
```

## Code Quality

- **Formatting**:
  Defined in `dsp-dpe/.rustfmt.toml`, use Unix newlines.
  Use `leptosfmt` for code in `dsp-dpe/mosaic`.
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
  Run `just dsp-dpe check` and `just dsp-dpe test`.
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
