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
- Vendored JS files go in `modules/dpe/public/vendor/` — update `vendor/README.md` when adding or updating

## Data Conventions

- New project `temporalCoverage` values must resolve to structured dates for OAI-PMH — add each new free-text value to `modules/dpe/server/data/temporal-coverage-enrichment.json` (keyed by display text, with a W3CDTF range and `source: "llm"`). See `docs/src/dpe/oai-pmh.md` and the "Adding a Project Metadata File" section of `modules/dpe/CLAUDE.md`.

## Git, Commits, and Pull Requests

All git handling — the rebase-merge workflow, commit message schema, commit organization, and PR conventions — lives in one file: `docs/src/git-conventions.md`.
