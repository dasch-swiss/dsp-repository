# CLAUDE.md

## Project Overview

This repository is a Rust-based monorepo for the DaSCH Service Platform. It contains the Discovery and Presentation Environment (DPE) — a public, read-only, server-side rendered web application built with Maud and Axum — the metadata editor, a separate authenticated service built the same way, and the Mosaic component library (design system) both use.

## Setup

`nix develop`, or `just install-requirements` without Nix. Node is needed only for the Playwright E2E suites — the Tailwind CSS build is Node-free (standalone CLI).

**Agents:** direnv does not fire in a non-interactive shell, so the Nix dev shell is not active in your tool calls even when the developer's terminal has it. Under Nix, prefix commands with `nix develop --command` (e.g. `nix develop --command just check`); on a `just install-requirements` setup the toolchain is already on `PATH` and `nix` may not be installed at all.

## Documentation

All authoritative documentation lives in `docs/src/`; `docs/src/SUMMARY.md` is the index. `CONVENTIONS.md` holds coding conventions and the PR template (work phase); `REVIEW.md` holds the code review checklist (review phase).

## Development Workflow

**Important:** Follow ALL the steps below during development.

- **Always check with the developer before each step** — check in, instead of going down the wrong path.
- **Use `just` for all commands** — `just --list` shows every recipe. Use `just`, not `cargo` or `npm`; prefer claude-specific recipes where they exist.

Before considering ANY change as "done":

- **Verify that changes compile and all checks pass** — run `just check` and `just test`. This covers formatting and linting, so there is no need to run them earlier.
- **Check if documentation needs updating** — consider `docs/src/`, `CLAUDE.md`, `CONVENTIONS.md`, and `REVIEW.md`.
- **Commit messages** — `type(scope): subject`, both mandatory. Exactly eight types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `chore`. `revert`, `style`, and `ci` are **not** valid — use `chore(ci): ...`, fold formatting in, and `fix`/`chore` for reverts. Scope is a crate name or one of `dpe-data`, `ci`, `deps`, `docs` (see `CONVENTIONS.md`); there is no catch-all. Verify with `just commit-lint` before pushing — CI enforces it.
- **One commit per PR** — we use rebase-merge, so every branch commit lands on main verbatim. CI caps a PR at one commit; tick `allow-many-commits` in the PR body only when the work is genuinely several independent, self-contained changes. A bug you introduced earlier in the same branch is not a `fix:` — amend it into the commit that introduced it. See `docs/src/git-conventions.md`.
- **PR creation** — see `docs/src/git-conventions.md` for the PR workflow and template.

## Testing Guidelines

- **Tests first**: unless instructed otherwise, write unit tests for new functionality before implementing, and check them with the developer to confirm they define the right behavior.
- **Useful tests**: every test verifies meaningful behavior. Never write tests that verify the behavior of the Rust compiler or external libraries.
- **Helper functions**: prefer repetition over complicated setup; use helpers only if they improve clarity.

See `docs/src/dpe/testing-strategy.md` for the testing pyramid, conventions, and CI pipeline.

## Temporary File Management

Use `.claude/tmp/` for scratch files during a session; these are gitignored. Permanent documentation goes in `docs/src/`.

## Documentation Tone

Keep the tone factual and understated. Documentation should be clear first of all; there is no need to praise the software.
