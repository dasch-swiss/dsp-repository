# Tech Stack

## Core Technologies

- **Rust** - Primary development language (Edition 2021, Toolchain 1.86.0)
- **Axum** - HTTP web framework with WebSocket support
- **Maud** - Macro-based templating engine (JSX-like syntax)
- **DataStar** - Hypermedia-driven frontend interactivity
- **Tailwind** - Design system foundation with utility-first CSS approach
- **Database TBD** - Currently using static JSON files

## Development & Testing

- **TypeScript + Playwright** - End-to-end testing for design system
- **Cargo nextest** - Parallel test execution
- **ESLint + Prettier** - JavaScript/TypeScript code quality (testing only)
- **Node.js ecosystem** - Supporting testing infrastructure

## Architecture Principles

We keep the design evolutionary, starting from the simplest possible solution and iterating on it.
At first, providing data from static JSON files, or working with static content, is sufficient.
Following clean architecture principles, swapping out the persistence layer is easy.

## Implementation Notes

The TypeScript ecosystem is used exclusively for testing and development tooling, not for production runtime code. The core application remains purely Rust-based with hypermedia-driven interactivity.

<!-- TODO: for CSS post processing, check out https://github.com/rs-tml/rcss or https://docs.rs/lightningcss/latest/lightningcss/ -->
