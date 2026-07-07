# Code Review Guidelines

Review checklist for the DSP Repository. Organized by priority.

## Always Check

**Fragment Endpoints**
- New fragment endpoints follow resource-action nesting convention (see [DPE Architecture](../dpe/architecture.md))
- New Datastar interactions have `<a href>` fallback for graceful degradation
- ARIA semantics present on interactive components (`role`, `aria-selected`, `aria-controls`)

**Testing**
- insta snapshots added/updated for changed SSR output
- E2E test covers the user-facing behavior
- axe-core scan passes on affected pages
- Unit tests for fragment handler edge cases (invalid tab, missing project, etc.)

**Architecture**
- New API crates follow the `dpe-api-{name}` pattern with `dpe-core` as only domain dependency (see [Project Structure](../repo_structure.md))
- `dpe-core` has no framework dependencies (no leptos, no axum)
- Validate command covers all data file types (DPE)
- E2E test directory naming: `web-e2e-tests/` for DPE, `playground-e2e-tests/` for Mosaic

**CLI**
- CLI subcommands are documented in help text

**Documentation**
- Documentation updated when patterns change (see [About this Documentation](../docs.md))
- New environment variables documented in [DPE Operations](../dpe/operations.md)

**Commits**
- Commits follow conventional commits (correct prefix, scope matches crate name) — see [Workflows and Conventions](../workflows.md)
- One topic per commit — apply the "and" test
- Each commit builds and passes tests

**Security**
- No secrets in config files, Cargo.toml, or git
- Path parameters validated before filesystem access

## Style

- Follow existing Datastar attribute patterns (signal naming with `_` prefix) — see [DPE Architecture](../dpe/architecture.md)
- Fragment handlers in `fragments/` module, not inline in `main.rs`
- Domain types belong in `dpe-core`, not in web or API crates
- API crate exposes a handler function (e.g., `pub async fn oai_handler(...)`) for composition in dpe-server
- Leptos components use `view!` macro consistently
- Test files follow naming convention: `{feature}_tests.rs` for Rust, `{feature}.spec.ts` for Playwright

## Skip

- Snapshot `.snap` file contents — verify accepted, don't review formatting
- Formatting-only changes (`cargo fmt` / `leptosfmt` diffs)
- `Cargo.lock` changes from dependency updates
