# Tech Stack

## Core

| Technology | Purpose |
|-----------|---------|
| **Rust** (Edition 2021) | Primary development language |
| **Axum** | HTTP web framework (native routing, `ServeDir` for static assets) |
| **Maud** | Compile-time HTML templating (`maud::html!` → `Markup`) for server-side rendering |
| **Datastar** | SSE-based interactivity for DPE (~14KB JS, no WASM) |
| **Tailwind CSS v4** | Utility-first CSS framework |
| **Tokio** | Async runtime |
| **figment** | Layered configuration (defaults → TOML → env vars) |

## Data & Persistence

| Technology | Purpose |
|-----------|---------|
| **serde / serde_json** | Serialization and deserialization |
| **Static JSON files** | Current data storage (database TBD) |

## Testing & Quality

| Technology | Purpose |
|-----------|---------|
| **cargo test / nextest** | Rust test runner |
| **insta** | Snapshot testing for SSR output |
| **Playwright** | End-to-end browser tests |
| **axe-core** | Accessibility scanning (WCAG 2.1 AA) |
| **cargo-fuzz** | Fuzz testing (nightly CI) |

## Build & Development

| Technology | Purpose |
|-----------|---------|
| **Tailwind CSS CLI** | Standalone CLI that builds the stylesheet (`just css` / `just css-release`); no Node or build.rs CSS pipeline |
| **just** | Command runner for development workflows |
| **maudfmt** | Formats the contents of `maud::html!` macros (stock rustfmt does not) |
| **cargo +nightly fmt** | Code formatter for Rust outside the `html!` macros |
| **Biome** | Linter/formatter for E2E test TypeScript |

## Documentation & Observability

| Technology | Purpose |
|-----------|---------|
| **mdBook** + mdbook-alerts | Project documentation |
| **Fathom Analytics** | Privacy-friendly web analytics (GDPR-compliant, no cookies) |
| **tracing** + tracing-subscriber | Structured logging |

## Architecture Principles

We keep the design evolutionary, starting from the simplest possible solution and iterating on it. At first, providing data from static JSON files is sufficient. Following clean architecture principles, swapping out the persistence layer is easy.

TypeScript is used exclusively for testing and development tooling, not for production runtime code. The core application remains purely Rust-based.
