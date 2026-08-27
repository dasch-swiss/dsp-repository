# Conventions

Agent reference card for the **work phase**. All authoritative detail lives in `docs/src/`.

## Code Conventions

- **Crate naming and API crate pattern**: See `docs/src/repo_structure.md`
- **Fragment routes and Datastar attributes**: See `docs/src/dpe/architecture.md`
- **Formatting**: Rust style in `.rustfmt.toml`. Run `just fmt`: `maudfmt` formats the `html!` Maud macro contents (stock rustfmt does not), then `cargo +nightly fmt` handles the rest. `just check` verifies both.
- **No nested `html!` as a function argument**: bind non-trivial inner markup to a Rust `let` (`let body = html! { … }; card(body)`) or extract a `fn … -> Markup` helper — don't pass a multi-element `html! { … }` block directly into a call. `maudfmt` only formats `html!` at Rust statement/`let` position; a block nested as a call argument (or via Maud's in-macro `@let x = html! { … }`) is skipped and then mangled by `cargo fmt`. Trivial one-liners like `html! { (label) }` are fine inline.
- **Linting**: Strict clippy warnings. Run `just check`.

## Testing Conventions

- **Testing pyramid and strategy**: See `docs/src/dpe/testing-strategy.md`
- **Test naming**: `test_{what}_{condition}_{expected}` (e.g., `test_parse_project_missing_title_returns_error`)
- **Test locations**: `#[cfg(test)]` modules or adjacent `_tests.rs` files for unit tests; `web-e2e-tests/` for DPE E2E; `playground-e2e-tests/` for Mosaic E2E

## Observability Conventions

- Use `#[tracing::instrument]` for new handler and service functions
- Use `otel.kind = "internal"` on handler-level spans (middleware provides the server span)
- Metric attributes must be bounded — validate against known sets, normalize dynamic values. High-cardinality data goes to structured logs only, never to metric attributes
- Vendored JS files go in the owning module's `public/vendor/` — `modules/dpe/public/vendor/` or `modules/editor/public/vendor/` — and each has its own `vendor/README.md` (file, package, version, SHA-256) to update when adding or updating. The two are independent: the editor is on Datastar 1.0.2 while DPE is on 1.0.0-RC.8.

## Data Conventions

- New project `temporalCoverage` values must resolve to structured dates for OAI-PMH — add each new free-text value to `modules/dpe/server/data/temporal-coverage-enrichment.json` (keyed by display text, with a W3CDTF range and `source: "llm"`). See `docs/src/dpe/oai-pmh.md` and the "Adding a Project Metadata File" section of `modules/dpe/CLAUDE.md`.

## Git, Commits, and Pull Requests

All git handling — the rebase-merge workflow, commit message schema, commit organization, and PR conventions — lives in one file: `docs/src/git-conventions.md`. These rules are shared with SIPI.

The short version: `type(scope): subject`, **both mandatory**. Eight types — `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `chore` (`revert`, `style`, and `ci` are rejected; use `chore(ci): ...`). A PR lands as **one commit**; tick `allow-many-commits` in the PR body to opt out. Enforced by `just commit-lint` and the `gate` CI job.

### Scope vocabulary

Scopes are lowercase and name the concern a change serves, not the directory it sits in.

| Kind | Scopes |
|------|--------|
| Crates | `dpe-core`, `dpe-server`, `dpe-web`, `dpe-api-oai`, `platform-metadata`, `platform-telemetry`, `editor-core`, `editor-web`, `editor-server`, `mosaic-tiles`, `mosaic-playground` |
| Cross-cutting | `dpe-data` (project metadata files), `ci` (workflows, justfile), `deps` (dependency bumps), `docs` (repo-level docs) |

Documentation *about* a crate takes that crate's scope; only repo-level docs take `docs`. There is no catch-all scope — ask before inventing one. The gate checks only that a scope is present, so this list is advisory; keep to it anyway.
