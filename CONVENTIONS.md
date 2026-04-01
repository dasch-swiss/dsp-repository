# Conventions

Project-wide conventions for the DSP Repository.

## Fragment Route Convention

Fragment routes serve Datastar SSE responses for partial page updates (tab switching, search, etc.).

**Pattern: resource-action nesting**

| Route | Handler | Response |
|-------|---------|----------|
| `GET /projects/{id}` | Leptos SSR | Full page (reads `?tab=` for initial tab) |
| `GET /projects/{id}/tab/{tab}` | Pure Axum | SSE fragment (PatchElements + ExecuteScript) |

Different path depths in Axum's radix trie — no conflict, no header discrimination needed.

## Datastar Attribute Conventions

- **Signal naming**: Use `_` prefix for client-only signals (e.g., `_tab_loading`). The underscore excludes the signal from server payloads.
- **No `__debounce` on `__prevent` anchors**: Do NOT combine `__prevent` with `__debounce` or `__throttle` on anchor elements — known Datastar timing issue.
- **`retry: 'never'`**: Use on `@get()` calls where fallback to full navigation is preferred over retrying.
- **Graceful degradation**: Every Datastar-enhanced `<a>` must have a valid `href` for no-JS fallback.

## Crate Naming Convention

All workspace crates follow the `{module}-{role}` pattern:

| Crate | Role |
|-------|------|
| `dpe-core` | Pure domain types and data access (zero framework deps) |
| `dpe-api-oai` | OAI-PMH 2.0 API (depends on `dpe-core` only) |
| `dpe-web` | Leptos SSR components, pages, `#[server]` functions |
| `dpe-server` | Server binary — composes all routes |
| `mosaic-tiles` | Reusable UI component library |
| `mosaic-playground` | Component showcase application |
| `mosaic-playground-macro` | Proc macro for demo page generation |

## API Crate Pattern

Each API is a separate crate under `modules/dpe/`:

- **Naming**: `dpe-api-{name}` (e.g., `dpe-api-oai`)
- **Dependencies**: `dpe-core` for domain types; never depends on other API crates or `dpe-web`
- **Entry point**: Exports a handler function (e.g., `pub async fn oai_handler(...)`)
- **Composition**: `dpe-server` wires the handler into the Axum router

## Test Directory Naming

- `web-e2e-tests/` for DPE Playwright tests (sibling of the app, not nested)
- `playground-e2e-tests/` for Mosaic Playwright tests

## Testing Conventions

**Testing pyramid** (approximate distribution):

| Layer | Share | Purpose |
|-------|-------|---------|
| Unit | ~50% | Pure logic, domain types, parsing |
| E2E | ~30% | Full user flows via browser |
| Snapshot | ~15% | SSR output stability |
| Fuzz | ~5% | Edge cases, malformed input |

**Unit tests**: In-crate `#[cfg(test)]` modules or adjacent `_tests.rs` files.

**Snapshot tests**: Use the `insta` crate. `.snap` files are committed to git. Use `with_settings!` for scrubbing dynamic values (timestamps, IDs). CI runs with `INSTA_UPDATE=new` so unexpected changes produce `.snap.new` files for review.

**E2E tests**: Playwright in `web-e2e-tests/` (DPE) and `playground-e2e-tests/` (Mosaic).

**Fuzz tests**: `cargo-fuzz`, run on nightly CI. Corpus files are persisted in the repository.

**Test naming**: Use descriptive names following the `test_{what}_{condition}_{expected}` pattern. For example: `test_parse_project_missing_title_returns_error`.

## Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/). Scopes match crate names: `dpe-server`, `dpe-core`, `dpe-web`, `dpe-api-oai`, `mosaic-tiles`, `mosaic-playground`, `mosaic-playground-macro`. Commits are organized by topic, not implementation journey.

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

## Pull Request Template

```
## Motivation
Why is this change needed?

## Summary
What does this PR do? (1-3 sentences)

## Key Changes
- Bullet list of significant changes

## Challenges and Decisions
Any non-obvious choices made during implementation

## Gotchas
Anything reviewers or future maintainers should watch out for

## Test Plan
How was this tested?

Closes #{issue_number}
```
