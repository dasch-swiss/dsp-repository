# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
The DSP Repository is a Rust-based monorepo for the DaSCH Service Platform Repository - a long-term archive for humanities research data. It consists of the DSP Archive (storage) and Discovery and Presentation Environment (DPE) for data discovery, plus a custom design system.

## Architecture
- **Language**: Rust (Edition 2021, Toolchain 1.86.0)
- **Web Framework**: Axum HTTP server
- **Templating**: Dual approach - Askama (file-based) and Maud (macro/JSX-like)
- **Frontend**: Hypermedia-driven using DataStar (similar to HTMX)
- **Pattern**: Clean Architecture/Hexagonal Architecture
- **Structure**: Cargo workspace with modular crates

## Key Directories
```
modules/
├── dpe/                   # Discovery and Presentation Environment
│   ├── api/               # HTML routes and templates
│   ├── dto/               # Data transfer objects
│   ├── server/            # Web server binary
│   ├── services/          # Business logic implementations
│   ├── storage/           # Data persistence layer
│   └── types/             # Domain models and trait definitions
└── design_system/         # Custom design system
    ├── components/        # Reusable UI components
    └── playground/        # Full development environment with Rust server, TypeScript testing, and visual regression
```

## Development Commands (via justfile)
```bash
# Core development
just check                     # Run fmt and clippy checks
just build                     # Build all targets
just test                      # Run all tests
just run                       # Run main server (release mode)

# Development workflow
just watch                     # Watch for changes and run tests  
just run-watch-playground      # Run design system playground with hot reload

# Design system playground
just playground install         # Install frontend dependencies and browsers
just playground test            # Run TypeScript/Playwright tests
just playground test-visual     # Run visual regression tests
just playground test-headed     # Run tests with browser visible

# Code quality
just fmt                       # Format Rust code

# Documentation
just docs-build               # Build mdBook documentation
just docs-serve               # Serve docs at localhost:3000

# Setup
just install-requirements     # Install cargo-watch, mdbook, mdbook-alerts
```

## Tech Stack
- **HTTP**: Axum with macros, WebSocket support
- **Templating**: Askama + Maud
- **Hypermedia**: DataStar for interactive UI
- **Async**: Tokio runtime
- **Testing**: Cargo nextest for parallel execution
- **Documentation**: mdBook with alerts plugin
- **Design System**: TypeScript + Playwright for E2E and visual regression testing

## Data Layer
- **Current**: Static JSON files in `/data/json/` directory
- **Architecture**: Repository pattern with in-memory implementations
- **Future**: Database TBD, designed for easy swapping

## Code Quality
- **Formatting**: .rustfmt.toml with 120 char width, Unix newlines
- **File endings**: All files must end with a newline character
- **Linting**: Strict clippy warnings
- **Testing**: Testing pyramid (unit → integration → E2E with Playwright)
- **Visual Testing**: Automated visual regression testing for design system components
- **Git**: Rebase workflow, clean commit history

## Important Notes

### Development Workflow Practices

**Important:** Ensure to follow ALL the steps below during development.

- **Always check with the developer before each step** - Check in, instead of going down the wrong path
- **Use `JUST` for all commands** - Use `just` to run commands instead of `cargo` or `npm`. If availant, use claude-specific just commands.
- **Use consistent style** - Follow the project's coding style. Run auto-formatting. Add a new line at the end of every file.
- **Tests first** - Ensure that every code change is accompanied by tests. Start with a test suite to define the expected behavior of the system. Check the tests with the developer to ensure the behavior is correct.

Before considering ANY change as "done", ensure the following:

- **Always verify that changes compile and all checks and tests pass** - Run `just check` and `just test`, or similar commands
- **Always check if documentation needs to be updated** - Before considering a change done, verify if any documentation in `/docs/src/`, the readme, or the claude-files needs updates to reflect the changes.
- **Always ask before committing anything to git** - Never run git add or commit without explicit permission.
- **GitHub PR Creation Workflow** - When creating pull requests:
  1. Create as draft PR with `gh pr create --draft`
  2. Assign to the requesting developer with `gh pr edit [PR_NUMBER] --add-assignee [USERNAME]`
  3. Include "Review Notes" section mentioning that separate commits should be checked for easier review

### Temporary File Management
- **Temporary files**: Use `.claude/tmp/` for temporary files that are
- **Ask if unsure** whether a document should be temporary or permanent
- **Temporary files are gitignored** and won't be tracked in version control
- **Permanent documentation** should go in the appropriate location in `/docs/src/`

## Architecture Principles
- Clean separation between domain, business logic, and infrastructure
- Single responsibility per crate
- Server-rendered HTML with DataStar for interactivity
- Domain-driven design aligned with research data concepts

## Operational Guidelines
- Whenever you make a change, check if any documentation needs to be adapted

## Memory
- Every file must end on a new line character
- In documentation, keep the tone factual and almost understated. Documentation should be clear first of all, there is no need to praise the software.