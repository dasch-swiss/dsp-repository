# Conventions

Agent reference card for the **work phase**. Pair with `REVIEW.md`, the review-phase card. All authoritative detail lives in `docs/src/`.

## Stack

- Rust, one Cargo workspace (Bazel is the decided target, ADR-0001). Services are server-rendered hypermedia: Axum for routing, Maud for HTML, Datastar for interactivity over SSE; no client framework, no WASM (ADR-0004).
- Stores are SQLite through `rusqlite`, pooled with `deadpool-sqlite` in the editor and identity stores.
- CSS is Tailwind through its standalone CLI (`just css`); Node is needed only for the Playwright E2E suites.
- Tests: `cargo test`, `insta` snapshots for rendered HTML, Playwright plus axe-core for E2E.
- Toolchain: `nix develop`, or `just install-requirements`; `just --list` shows every recipe.

## Code Conventions

- **Crate naming and API crate pattern**: See `docs/src/repo_structure.md`
- **Fragment routes and Datastar attributes**: See `docs/src/dpe/architecture.md`
- **Formatting**: Rust style in `.rustfmt.toml`. Run `just fmt`: `maudfmt` formats the `html!` Maud macro contents (stock rustfmt does not), then the nightly rustfmt pinned in `.rustfmt-nightly` handles the rest. To bump it: change the date, run `just fmt`, then `nix flake update rust-overlay`; CI, the justfile and `flake.nix` all read that file. `just check` verifies both. Land a whole-tree reformat as its own PR, and once it is on `main` add its SHA there to `.git-blame-ignore-revs`.
- **No nested `html!` as a function argument**: bind non-trivial inner markup to a Rust `let` (`let body = html! { … }; card(body)`) or extract a `fn … -> Markup` helper — don't pass a multi-element `html! { … }` block directly into a call. `maudfmt` only formats `html!` at Rust statement/`let` position; a block nested as a call argument (or via Maud's in-macro `@let x = html! { … }`) is skipped and then mangled by `cargo fmt`. Trivial one-liners like `html! { (label) }` are fine inline.
- **Linting**: Strict clippy warnings. Run `just check`.

## Comments

- A comment states what the reader must not break, never what the session discovered. Test: would it still be true and useful for someone who never saw the change that added it?
- Keep: an invariant a reader would otherwise break, a non-obvious third-party contract (lifetime, ownership, error/return semantics), the public API contract. One or two sentences each.
- Move out: what a change fixed or a test caught → commit body or PR; probe tables, benchmarks, corpus counts → `docs/` or a learning; a rejected alternative → a docs page or ADR, leaving one line and a link in the source.
- Delete: restatements of the code below, history ("previously", "now uses", "was changed to"), and any REQ id, user story, or plan-phase reference. A `TODO` without an issue id belongs in the tracker.
- A PR description describes the code and the diff, never the commit history. "Commit 1 did X, commit 2 fixed Y" describes the journey.
- A "why" longer than about five lines belongs in a file; the comment becomes a pointer to it. A block past ~12 lines is a routing signal.
- Delete by default, when adding and when trimming: per sentence, name what a reader breaks without it, cut the ones with no answer, unclear included, and hold each survivor to one or two sentences.
- Doc comments: Rust uses `///` for items, `//!` for modules, `//` inline. TypeScript (Playwright e2e only) uses `//` inline and `/** */` on exported helpers; no doc tool runs. YAML, TOML, shell and CSS carry plain comments; no doc-comment tool.

## Testing Conventions

- **Testing pyramid and strategy**: See `docs/src/dpe/testing-strategy.md`
- **Test naming**: `test_{what}_{condition}_{expected}` (e.g., `test_parse_project_missing_title_returns_error`)
- **Test locations**: `#[cfg(test)]` modules or adjacent `_tests.rs` files for unit tests; `areas/access/dpe/web-e2e-tests/` and `areas/deposit/editor/web-e2e-tests/` for DPE and editor E2E; `modules/mosaic/playground-e2e-tests/` for Mosaic E2E

## Observability Conventions

- Use `#[tracing::instrument]` for new handler and service functions
- Use `otel.kind = "internal"` on handler-level spans (middleware provides the server span)
- Metric attributes must be bounded — validate against known sets, normalize dynamic values. High-cardinality data goes to structured logs only, never to metric attributes
- Vendored JS files go in the owning module's `public/vendor/` — `areas/access/dpe/public/vendor/` or `areas/deposit/editor/public/vendor/` — and each has its own `vendor/README.md` (file, package, version, SHA-256) to update when adding or updating. The two are independent: each is updated on its own, and neither README states the other's versions. `just check` recomputes those digests and fails on drift either way (a changed file, a row without a file, a committed file without a row), so the table has to be correct, not just present. See `docs/src/security.md`.

## Data Conventions

- Project JSON files are canonical: top-level members in `ProjectRaw`'s declaration order, nested objects in theirs, language keys alphabetical, 4-space indent, trailing newline, no explicit `null`. Enforced byte-for-byte by `editor-core`'s `canonical_round_trip` test; regenerate with `CANONICALIZE_PROJECT_FILES=1 cargo test -p editor-core --test canonical_round_trip` rather than reordering by hand.
- New project `temporalCoverage` values must resolve to structured dates for OAI-PMH — add each new free-text value to `areas/access/dpe/server/data/temporal-coverage-enrichment.json` (keyed by display text, with a W3CDTF range and `source: "llm"`). See `docs/src/dpe/oai-pmh.md` and the "Adding a Project Metadata File" section of `areas/access/dpe/CLAUDE.md`.
- A new `spatialCoverage` or `temporalCoverage` reference must be a bundled option: run `python3 scripts/build-coverage-vocabularies.py` (for a new period, `python3 scripts/fetch-chronontology-periods.py` first), or `editor-core`'s drift tests fail.

## Git, Commits, and Pull Requests

All git handling — the rebase-merge workflow, commit message schema, commit organization, and PR conventions — lives in one file: `docs/src/git-conventions.md`. These rules are shared with SIPI.

The short version: `type(scope): subject`, **both mandatory**. Eight types — `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `chore` (`revert`, `style`, and `ci` are rejected; use `chore(ci): ...`). A PR lands as **one commit**; tick `allow-many-commits` in the PR body to opt out. Enforced by `just commit-lint` and the `gate` CI job.

### Scope vocabulary

Scopes are lowercase and name the concern a change serves, not the directory it sits in.

| Kind | Scopes |
|------|--------|
| Crates | `access-server`, `dpe-core`, `dpe-server`, `dpe-web`, `dpe-api-oai`, `cpe-ports`, `cpe-core`, `cpe-components`, `cpe-image-viewer`, `cpe-store`, `cpe-store-build`, `cpe-web`, `cpe-server`, `sync-store`, `media-store`, `shared-datastar`, `shared-fair`, `shared-metadata`, `shared-rate-limit`, `shared-telemetry`, `editor-core`, `editor-ports`, `editor-store`, `editor-web`, `deposit-server`, `editor-pull-requests`, `deposit-shell`, `identity-core`, `identity-ports`, `identity-store`, `identity-web`, `mosaic-tiles`, `mosaic-playground`, `dsp-cli` |
| Cross-cutting | `dpe-data` (project metadata files), `ci` (workflows, justfile), `deps` (dependency bumps), `docs` (repo-level docs) |

Documentation *about* a crate takes that crate's scope; only repo-level docs take `docs`. There is no catch-all scope — ask before inventing one. The gate checks only that a scope is present, so this list is advisory; keep to it anyway.

## Reviewers & Git History

Before opening a PR, read `git log origin/main` and recent merged PRs. Use them for two things: to confirm how the commit-message rules above are applied in practice (subject wording, which scope a kind of change takes), and to find reviewers: the people who recently authored or committed changes to the files the PR touches. This repo keeps no `CODEOWNERS`, so git history is the reviewer signal.

## Before Committing

- `just fmt`, then `just check` (formatting, clippy, and the repo's static checks) and `just test`. While iterating, `just test-changed` runs only the packages changed since `origin/main` and their dependents.
- `just validate-data` after any change under `areas/access/dpe/server/data/`.
- `just commit-lint` before pushing; the `gate` CI job runs the same check.

## Where to Go for Depth

- `docs/src/SUMMARY.md`: index of the authoritative documentation.
- `ARCH-MAP.md`: components, boundaries, local-context kits, banned constructs.
- `CONTEXT.md` (root index plus one per bounded context): domain vocabulary.
- `docs/adr/`: decisions in force.
- `docs/src/dpe/testing-strategy.md`: the testing pyramid and CI pipeline.
- `docs/src/git-conventions.md`: commits, PR shape, stacking.
