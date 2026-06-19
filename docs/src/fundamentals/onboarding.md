# Onboarding

## Toolchain & setup

Two supported paths:

- **Nix** (recommended): `nix develop` (or direnv via `.envrc`). The flake provides the full toolchain, including Node and pnpm.
- **Without Nix**: `just install-requirements` (rustup + `cargo binstall` + the e2e Node dependencies).

**Node and pnpm must be on `PATH` for all shells (non-Nix path).** `just` runs recipes in `sh`, which does not load shell-function-based version managers. A *lazy* nvm setup (where `node`/`npm`/`pnpm` are shell functions) therefore leaves them invisible to `just`, and recipes such as `lint-e2e`, `test-a11y-dpe`, and the Playwright e2e tests fail with `env: node: No such file`. To fix:

- expose your default Node bin on `PATH` at shell startup — eager-load it in your shell rc, or use a PATH-shim manager such as volta or asdf; and
- run `corepack enable` to provide `pnpm` (nvm does not ship pnpm).

Only Playwright genuinely requires Node; the Tailwind CSS build is Node-free (standalone CLI). CI provisions Node and pnpm explicitly, so this affects local non-Nix development only.

## Rust

The main technology we use is Rust.
A solid understanding of Rust is needed,
though particularly the frontend work does not require deep knowledge of Rust.

### Rust HTTP Server

We use [Axum](https://docs.rs/axum/latest/axum/) as our HTTP server.

### Serialization and Deserialization

We use [serde](https://serde.rs/) for serialization and deserialization of data.

### Web UI

We use [Maud](https://maud.lang.rs/) for server-side rendering. Maud is a compile-time HTML templating library: the `maud::html!` macro produces a `Markup` value, and view functions are plain Rust functions returning `Markup`.

Key points:

* Both DPE and the Mosaic playground are server-side rendered with Maud — no client-side WASM, no hydration, no islands
* Interactivity is provided by [Datastar](https://data-star.dev/): SSE fragment handlers render `Markup` and stream it back to the browser
* Routing is native Axum (`Router::new().route(...)`); static assets are served via `tower_http::ServeDir`
* The architecture follows the MPA paradigm, a "multi-page app"

### Architectural Design Patterns

We follow concepts such as [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
(there is also a [book](https://www.oreilly.com/library/view/clean-architecture-a/9780134494272/)),
[Hexagonal Architecture](https://en.wikipedia.org/wiki/Hexagonal_architecture_(software))
or [Onion Architecture](https://jeffreypalermo.com/2008/07/the-onion-architecture-part-1/).
Familiarity with these concepts will be helpful.

Some of the patterns must be adapted to the idioms of Rust,
but the general principles are the same.

### Testing

We follow the Testing Pyramid approach to testing,
the majority of tests are unit tests, with a smaller number of integration tests, and a few end-to-end tests.

Unit and integration tests are written in Rust, following the Rust testing best practices.
End-to-end tests can be written using Playwright.

### Domain Driven Design

We do not follow strict Domain Driven Design (DDD) principles,
but we try to follow some of the concepts.
In particular, we try to keep the language used in code aligned with the domain language.

### Test Driven Development

We should absolutely do TDD and BDD.

## Database

We are still evaluating the database to use.

For the initial development, we work with static content or JSON files.

## Mosaic Component Library

The Mosaic component library provides reusable UI components built with Maud and Tailwind CSS.

Components are defined in `modules/mosaic/tiles/` and can be previewed in the playground application at `modules/mosaic/playground/`.

To run the playground locally:

```bash
just watch-mosaic-playground
```

Pull requests that modify files in `modules/mosaic/` automatically receive a
Cloud Run preview deployment. The preview URL is posted as a comment on the PR.
