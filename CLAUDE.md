# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

This repository is a Rust-based monorepo for the DaSCH Service Platform. It contains the Discovery and Presentation Environment (DPE) — a server-side rendered web application built with Leptos and Axum — and the Mosaic component library (design system).

## Setup

### Prerequisites

- **Rust**: Toolchain managed via `rustup`
- **Just**: Command runner for development tasks
- **pnpm**: Package manager for the DPE frontend

### First-Time Setup

```bash
just install-requirements
```

## Key Commands

```bash
just watch-dpe                  # Run DPE with hot reload
just watch-mosaic-playground    # Run Mosaic playground with hot reload
just check                      # Run fmt checks and clippy
just fmt                        # Format all Rust code (cargo fmt + leptosfmt)
just test                       # Run all tests
just build                      # Build all targets
just docs-serve                 # Serve docs at localhost:3000
```

## Documentation

All authoritative documentation lives in `docs/src/`. Key pages:

| Topic | Location |
|-------|----------|
| Architecture | `docs/src/dpe/architecture.md` |
| Project Structure | `docs/src/dpe/project_structure.md` |
| Repo Structure & Crate Naming | `docs/src/repo_structure.md` |
| Testing Strategy | `docs/src/dpe/testing-strategy.md` |
| Observability | `docs/src/dpe/observability.md` |
| Operations (Docker, env vars) | `docs/src/dpe/operations.md` |
| Workflows & Commits | `docs/src/workflows.md` |
| Tech Stack | `docs/src/fundamentals/tech_stack.md` |
| Review Guidelines | `docs/src/fundamentals/review-guidelines.md` |
| Onboarding | `docs/src/fundamentals/onboarding.md` |

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
  Consider `docs/src/`, `CLAUDE.md`, `CONVENTIONS.md`, and `REVIEW.md`.
  Update documentation to reflect the changes.
- **Ask before committing** —
  Never run git add or commit without explicit permission.
- **PR creation** — See `docs/src/workflows.md` for the PR workflow and template.

## Testing Guidelines

- **Tests first**: Unless instructed otherwise, write tests before implementing.
- **Unit tests**: Write unit tests for all new functionality.
- **Useful tests**: Every test should verify meaningful behavior.
  Never write tests that verify the behavior of the Rust compiler or external libraries.
- **Helper functions**: In tests, prefer repetition over complicated setup.
  Only use helpers if they improve clarity.

See `docs/src/dpe/testing-strategy.md` for the testing pyramid, conventions, and CI pipeline.

## Temporary File Management

Use `.claude/tmp/` for scratch files during a session. These are gitignored. Permanent documentation goes in `docs/src/`.

## Conventions and Review

- See `CONVENTIONS.md` for coding conventions, commit rules, and PR template (work phase)
- See `REVIEW.md` for the code review checklist (review phase)

## Documentation Tone

Keep the tone factual and understated. Documentation should be clear first of all; there is no need to praise the software.
