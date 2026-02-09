# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

This repository is a Rust-based monorepo for the DaSCH Service Platform.
The platform provides services for a long-term archive for humanities research data.

This includes:

- The "Data Presentation Environment" (DPE)
- The "Mosaic" component library (demo for documentation and tiles for components)

## Architecture

- **Language**: Rust
- **Web Server**: Axum HTTP server
- **Web UI**: Leptos for MPA style end user web interfaces
- **Web UI components**: mosaic-tiles component library
- **Architecture**: Clean Architecture/Hexagonal Architecture
- **Structure**: Cargo workspace with modular crates

## Key Directories

```txt
modules/
├── dpe/                   # Discovery and Presentation Environment
│   ├── api/               # HTML routes and templates
│   ├── dto/               # Data transfer objects
│   ├── server/            # Web server binary
│   ├── services/          # Business logic implementations
│   ├── storage/           # Data persistence layer
│   └── types/             # Domain models and trait definitions
└── mosaic/                # Mosaic component library
    ├── tiles/             # Reusable Leptos UI components
    ├── demo/              # Component showcase application
    └── demo_macro/        # Proc macro for demo page generation
```

## Setup

### Prerequisites

- **Rust**:
  Setup with the Rust toolchain and additional tooling installed using `just install-requirements`
- **Just**: Command runner for development tasks

### First-Time Setup

1. Install Rust tools:

   ```bash
   just install-requirements
   ```

## Development Commands (via justfile)

```bash
# Development workflow for mosaic (tiles and demo)
just watch-mosaic-demo         # Run Mosaic demo with hot reload
just fmt-mosaic                # Format source code in the mosaic module with leptosfmt

# Core development workflow for other components
just check                     # Run fmt and clippy checks
just build                     # Build all targets
just test                      # Run all tests
just run                       # Run main server (release mode)

just watch                     # Watch for changes and run tests

# Code quality
just fmt                       # Format Rust code

# Documentation
just docs-build               # Build mdBook documentation
just docs-serve               # Serve docs at localhost:3000

# Setup
just install-requirements     # Install Rust tools: cargo-watch, mdbook, mdbook-alerts, leptosfmt, cargo-leptos
```

## Tech Stack

- **HTTP**: Axum with macros, WebSocket support
- **UI Framework**: [Leptos](https://book.leptos.dev) with [islands feature](https://book.leptos.dev/islands.html)
- **Styling**: Tailwind CSS
- **Async**: Tokio runtime
- **Testing**: Cargo test, Playwright for E2E
- **Documentation**: mdBook with alerts plugin

## Data Layer

- **Current**: Static JSON files in `/data/json/` directory
- **Architecture**: Repository pattern with in-memory implementations

## Code Quality

- **Formatting**:
  Defined in `.rustfmt.toml`, use Unix newlines.
  Use `leptosfmt` for code in modules/mosaic.
- **Linting**: Strict clippy warnings
- **Testing**: Testing pyramid (unit → integration → E2E)
- **Git**: Rebase workflow, clean commit history

## Important Notes

### Development Workflow Practices

**Important:** Ensure to follow ALL the steps below during development.

- **Always check with the developer before each step** -
  Check in, instead of going down the wrong path
- **Use `JUST` for all commands** -
  Use `just` to run commands instead of `cargo` or `npm`.
  If available, use claude-specific just commands.
- **Use consistent style** - Follow the project's coding style. Run auto-formatting.
- **Tests first** -
  Ensure that every code change is accompanied by tests.
  Start with a test suite to define the expected behavior of the system.
  Check the tests with the developer to ensure the behavior is correct.

Before considering ANY change as "done", ensure the following:

- **Always verify that changes compile and all checks and tests pass** -
  Run `just check` and `just test`, or similar commands
- **Always check if documentation needs to be updated** -
  Before considering a change done, verify if any documentation needs to be updated.
  Consider documentation in `/docs/src/`, the readme, or the claude-files.
  Update the documentation in order to reflect the changes.
- **Always ask before committing anything to git** -
  Never run git add or commit without explicit permission.
- **GitHub PR Creation Workflow** -
  When creating pull requests:
  1. Create as draft PR with `gh pr create --draft`
  2. Assign to the requesting developer with `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`
  3. Include "Review Notes" section mentioning that separate commits should be checked for easier review

### Testing Guidelines

- **Tests first**: Unless instructed otherwise, write the tests before implementing the feature.
- **Unit tests**: Write unit tests for all new functionality.
- **Useful tests**: Ensure that tests are useful and meaningful.
  Ask yourself: What is this test verifying?
  NEVER write tests that verify the behavior of the Rust compiler or external libraries.
- **Helper functions**: In tests, it is better to repeat yourself than to complicate the test setup.
  Only use helper functions if they improve the clarity of the test.

### Temporary File Management

- **Temporary files**: Use `.claude/tmp/` for temporary files that are
- **Ask if unsure** whether a document should be temporary or permanent
- **Temporary files are gitignored** and won't be tracked in version control
- **Permanent documentation** should go in the appropriate location in `/docs/src/`

## Architecture Principles

- Clean separation between domain, business logic, and infrastructure
- Single responsibility per crate
- MPA style server-side rendering with Leptos islands for interactivity
- Domain-driven design aligned with research data concepts

## Operational Guidelines

- Whenever you make a change, check if any documentation needs to be adapted

## Memory

- Every file must end on a new line character
- In documentation, keep the tone factual and almost understated.
  Documentation should be clear first of all, there is no need to praise the software.
