# DPE Testing Strategy

The DPE follows a 4-layer testing pyramid, adapted from the [Sipi testing strategy](https://github.com/dasch-swiss/sipi/blob/main/docs/src/development/testing-strategy.md). Target distribution: ~50% unit, ~30% E2E, ~15% snapshot, ~5% fuzz.

## Testing Pyramid

```
          ╱╲
         ╱  ╲         Layer 4: Fuzz Testing (nightly CI)
        ╱────╲        cargo-fuzz, corpus persisted
       ╱      ╲
      ╱  E2E   ╲     Layer 3: E2E Tests (Playwright)
     ╱──────────╲     Tab switching, search, accessibility (axe-core)
    ╱            ╲
   ╱  Snapshots   ╲   Layer 2: Snapshot Tests (insta)
  ╱────────────────╲   SSR output, SSE fragments, ARIA attributes
 ╱                  ╲
╱    Unit Tests      ╲ Layer 1: Unit Tests (cargo test)
╱────────────────────╲ Fragment handlers, OAI protocol, domain logic
```

## Layer 1: Unit Tests

- **Location**: `#[cfg(test)]` modules in each crate
- **Runner**: `cargo test --workspace`
- **Scope**: Fragment handlers, OAI protocol, domain types, data loading, filtering/pagination
- **Crate**: dpe-core tests run independently — `cargo test -p dpe-core`

## Layer 2: Snapshot Tests (insta)

- **Dependency**: `insta` with `yaml` and `filters` features
- **Location**: Adjacent `snapshots/` directories
- **CI**: Set `INSTA_UPDATE=new` so failures produce `.snap.new` artifacts for review
- **Scope**: SSR output, SSE fragment response bodies, ARIA attributes

## Layer 3: E2E Tests (Playwright)

- **Location**: `modules/dpe/web-e2e-tests/`
- **Runner**: `npx playwright test`
- **Scope**: Tab switching, search autocomplete, scroll preservation, accessibility (axe-core), visual regression
- **Accessibility**: Full-page axe-core scans against WCAG 2.1 AA

## Layer 4: Fuzz Testing

- **Tool**: cargo-fuzz (nightly Rust)
- **Schedule**: Nightly CI, 10 minutes per target
- **Targets**: Tab name validation, SSE response construction, query parameter parsing
- **Corpus**: Persisted between runs

## CI Pipeline Budget

Target: **≤ 10 minutes** wall-clock per PR.

```
Parallel job group 1 (~2 min):
  cargo fmt --check
  cargo clippy --all-targets -Dwarnings
  cargo-deny check

Parallel job group 2 (~5 min):
  cargo nextest run --workspace
  cargo leptos build --release
  cargo-llvm-cov (coverage → Codecov)

Parallel job group 3 (~5 min):
  Playwright E2E tests
  axe-core accessibility scans
  Lighthouse CI performance budgets
```
