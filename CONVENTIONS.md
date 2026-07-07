# Conventions

Agent reference card for the **work phase**. All authoritative detail lives in `docs/src/`.

## Code Conventions

- **Crate naming and API crate pattern**: See `docs/src/repo_structure.md`
- **Fragment routes and Datastar attributes**: See `docs/src/dpe/architecture.md`
- **Formatting**: Defined in `.rustfmt.toml`. Use `leptosfmt` for Leptos code. Run `just fmt`.
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

## Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/). Scopes match crate names.

| Prefix | Meaning | Changelog | Version bump |
|--------|---------|-----------|--------------|
| `feat:` | New user-visible functionality | Features | minor |
| `fix:` | Bug fix | Bug Fixes | patch |
| `perf:` | Performance improvement | Performance | patch |
| `revert:` | Revert a previous commit | Reverts | patch |
| `refactor:` | Code restructuring | hidden | none |
| `test:` | Tests | hidden | none |
| `ci:` | CI/CD | hidden | none |
| `docs:` | Documentation | hidden | none |
| `build:` | Build system, deps | hidden | none |
| `style:` | Formatting | hidden | none |
| `chore:` | Maintenance | hidden | none |

Group commits by user-visible impact. Each `feat:` or `fix:` = one changelog entry. Internal work → squash aggressively. See `docs/src/workflows.md` for full details.

## PR Template

```
Fixes LINEAR-ID, LINEAR-ID, ...

## Motivation
Why this work was needed. What problem it solves for users.

## Summary
1-3 bullet points of user-visible changes.

## Key Changes
### [Topic]
- change details

## Challenges and Decisions
What was tried, what failed, and key architecture decisions.
Structure as sub-sections when multiple challenges exist:

### [Challenge title]
**Problem:** description of the issue encountered
**Tried:** approaches that didn't work and why
**Solution:** what worked and why it's the right approach

## Gotchas
Things future developers should know. Each gotcha should be
actionable — not just "this is hard" but "do X instead of Y".

## Test Plan
- [ ] verification steps
```

See `docs/src/workflows.md` for rationale and the full "what goes where" guide.
