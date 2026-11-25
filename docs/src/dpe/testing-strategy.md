# DPE Testing Strategy

The DPE follows a 4-layer testing pyramid, adapted from the [Sipi testing strategy](https://github.com/dasch-swiss/sipi/blob/main/docs/src/development/testing-strategy.md). Target distribution: ~50% unit, ~30% E2E, ~15% snapshot, ~5% fuzz.

## Testing Pyramid

```
          ╱╲
         ╱  ╲         Layer 4: Fuzz Testing (nightly CI)
        ╱────╲        libFuzzer via Bazel, corpus persisted
       ╱      ╲
      ╱  E2E   ╲     Layer 3: E2E Tests (Playwright)
     ╱──────────╲     Tab switching, search, accessibility (axe-core)
    ╱            ╲
   ╱  Snapshots   ╲   Layer 2: Snapshot Tests (insta)
  ╱────────────────╲   SSR output, SSE fragments, ARIA attributes
 ╱                  ╲
╱    Unit Tests      ╲ Layer 1: Unit Tests (bazel test)
╱────────────────────╲ Fragment handlers, OAI protocol, domain logic
```

## Layer 1: Unit Tests

- **Location**: `#[cfg(test)]` modules in each crate
- **Runner**: `just test` (`bazel test //...`)
- **Scope**: Fragment handlers, OAI protocol, domain types, data loading, filtering/pagination
- **Crate**: one package's tests run alone, for example `bazel test //areas/access/dpe/core/...` or `bazel test //shared/metadata/...`

## Layer 2: Snapshot Tests (insta)

- **Dependency**: `insta` with `yaml` and `filters` features
- **Location**: Adjacent `snapshots/` directories
- **Updates**: `.bazelrc` sets `INSTA_UPDATE=no`, so a changed snapshot fails its test; `just snap-accept <target>` rewrites it from the checkout and `git diff` is the review
- **Scope**: SSR output, SSE fragment response bodies, ARIA attributes

## Layer 3: E2E Tests (Playwright)

- **Location**: `areas/access/dpe/web-e2e-tests/`
- **Server**: the suite starts `access-server` without `CPE_BASE`, so CPE is not mounted in it and only DPE's pages
  are tested; CPE's pages are covered by its crates' own tests.
- **Runner**: `just test-a11y-dpe` (the accessibility spec) and `just test-e2e-dpe` (the facet-form spec,
  `project-filters.spec.ts`). Both build the release server and stylesheet with Bazel and pass the binary's
  path in `ACCESS_SERVER_BIN`; CI runs both in `a11y / dpe`.
- **JavaScript off**: the `chromium-no-js` Playwright project runs the tests tagged `@no-js` with JavaScript
  disabled, so the no-JS fallback of a Datastar-enhanced control is tested in a real browser; the `chromium`
  project skips those tests.
- **Scope**: Tab switching, search autocomplete, scroll preservation, the facet form with JavaScript on and
  off, accessibility (axe-core)
- **Accessibility**: Full-page axe-core scans against WCAG 2.1 AA

## Layer 4: Fuzz Testing

- **Tool**: libFuzzer through `rules_fuzzing`, `just build-fuzz` (`--config=fuzz`, stable Rust; see [Building with Bazel](../bazel.md#fuzz-targets))
- **Schedule**: Nightly CI, 10 minutes per target
- **Targets**: Tab name validation, SSE response construction, query parameter parsing
- **Corpus**: Persisted between runs

## CI Pipeline Budget

Target: **≤ 10 minutes** wall-clock per PR.

```
check (one job):
  gate scripts, just --fmt check, maudfmt and nightly rustfmt checks
  clippy over every Bazel target, -Dwarnings (just check)

test / ubuntu / stable (one job):
  bazel test //... (just test)
  build, load and start the three images (just image-smoke)

a11y / dpe, a11y / editor (on path changes):
  release server and stylesheet built with Bazel, then Playwright with axe-core
  (a11y / dpe also runs the facet-form spec, with JavaScript on and off)
```

Bazel jobs share the remote cache and executor; see [Building with Bazel](../bazel.md#remote-cache-and-execution).

## Testing Conventions

**Test naming**: Use descriptive names following the `test_{what}_{condition}_{expected}` pattern. For example: `test_parse_project_missing_title_returns_error`.

**Test locations**:
- Unit tests: In-crate `#[cfg(test)]` modules or adjacent `_tests.rs` files
- Snapshot files: `.snap` files committed to git in `snapshots/` directories
- E2E tests: `web-e2e-tests/` for DPE, `playground-e2e-tests/` for Mosaic
- Fuzz corpus: Persisted in the repository under `fuzz/corpus/`

**Test file naming**: `{feature}_tests.rs` for Rust, `{feature}.spec.ts` for Playwright.

**Snapshot tests**: Use the `insta` crate. Use `with_settings!` for scrubbing dynamic values (timestamps, IDs). CI runs with `INSTA_UPDATE=new` so unexpected changes produce `.snap.new` files for review.
