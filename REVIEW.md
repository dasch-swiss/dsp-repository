# Code Review Checklist

Agent reference card for the **review phase**. Full details in `docs/src/fundamentals/review-guidelines.md`.

## Always Check

- [ ] Fragment endpoints follow resource-action nesting (`docs/src/dpe/architecture.md`)
- [ ] Datastar interactions have `<a href>` fallback for graceful degradation
- [ ] ARIA semantics on interactive components (`role`, `aria-selected`, `aria-controls`)
- [ ] insta snapshots added/updated for changed SSR output
- [ ] E2E test covers user-facing behavior
- [ ] axe-core scan passes on affected pages
- [ ] Unit tests for edge cases (invalid input, missing data)
- [ ] New API crates follow `dpe-api-{name}` pattern (`docs/src/repo_structure.md`)
- [ ] `dpe-core` has no framework dependencies
- [ ] CLI subcommands documented in help text
- [ ] Documentation updated when patterns change
- [ ] New env vars documented in `docs/src/dpe/operations.md`
- [ ] Commits follow conventional commits (`docs/src/workflows.md`)
- [ ] One topic per commit — apply the "and" test
- [ ] Each commit builds and passes tests
- [ ] No secrets in config files or git
- [ ] Path parameters validated before filesystem access
- [ ] New `#[instrument]` spans use `otel.kind = "internal"`, not `"server"`
- [ ] New OTel metric attributes are bounded — no free-form strings, no per-request unique values
- [ ] Vendored JS changes reflected in `modules/dpe/public/vendor/README.md`
- [ ] New routes added to `KNOWN_ROUTES` in `dpe-telemetry/src/page_url.rs` for page_url normalization

## Style

- [ ] Datastar attribute patterns match conventions (`_` prefix for signals)
- [ ] Fragment handlers in `fragments/` module, not inline
- [ ] Domain types in `dpe-core`, not in web or API crates
- [ ] Test files: `{feature}_tests.rs` (Rust), `{feature}.spec.ts` (Playwright)

## Skip

- Snapshot `.snap` file contents
- Formatting-only diffs
- `Cargo.lock` changes from dependency updates
