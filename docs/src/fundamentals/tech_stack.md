# Tech Stack

## Core Technologies

- **Rust** - Primary development language (Edition 2021, Toolchain 1.86.0)
- **Axum** - HTTP web framework with WebSocket support
- **Leptos** - Reactive UI framework for Rust
- **Tailwind CSS v4** - Utility-first CSS framework
- **Database TBD** - Currently using static JSON files

## Development & Testing

- **Cargo test** - Rust test runner
- **Playwright** - End-to-end testing
- **Leptosfmt** - Leptos code formatter

## Architecture Principles

We keep the design evolutionary, starting from the simplest possible solution and iterating on it.
At first, providing data from static JSON files, or working with static content, is sufficient.
Following clean architecture principles, swapping out the persistence layer is easy.

## Implementation Notes

TypeScript is used exclusively for testing and development tooling, not for production runtime code. The core application remains purely Rust-based.
