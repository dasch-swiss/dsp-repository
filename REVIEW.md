# Code Review Guidelines

Review checklist for the DSP Repository. Organized by priority.

## Always Check

**Fragment Endpoints**
- New fragment endpoints follow resource-action nesting convention (see `CONVENTIONS.md`)
- New Datastar interactions have `<a href>` fallback for graceful degradation
- ARIA semantics present on interactive components (`role`, `aria-selected`, `aria-controls`)

**Testing**
- insta snapshots added/updated for changed SSR output
- E2E test covers the user-facing behavior
- axe-core scan passes on affected pages
- Unit tests for fragment handler edge cases (invalid tab, missing project, etc.)

**Documentation**
- Documentation updated (architecture, CONVENTIONS.md, CLAUDE.md) when patterns change
- New environment variables documented in `docs/src/dpe/operations.md`

**Commits**
- Commits follow conventional commits (correct prefix, scope matches crate name)
- One topic per commit — apply the "and" test
- Each commit builds and passes tests

**Security**
- No secrets in config files, Cargo.toml, or git
- Path parameters validated before filesystem access

## Style

- Follow existing Datastar attribute patterns (signal naming with `_` prefix)
- Fragment handlers in `fragments/` module, not inline in `main.rs`
- Domain types belong in `dpe-core` (once extracted), not in web or API crates
- Leptos components use `view!` macro consistently

## Skip

- Snapshot `.snap` file contents — verify accepted, don't review formatting
- Formatting-only changes (`cargo fmt` / `leptosfmt` diffs)
- `Cargo.lock` changes from dependency updates
