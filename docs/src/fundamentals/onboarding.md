# Onboarding

## Rust

The main technology we use is Rust.
A solid understanding of Rust is needed,
though particularly the frontend work does not require deep knowledge of Rust.

### Rust HTTP Server

We use [Axum](https://docs.rs/axum/latest/axum/) as our HTTP server.

### Serialization and Deserialization

We use [serde](https://serde.rs/) for serialization and deserialization of data.

### Web UI

We use [Leptos](https://leptos.dev/) as our UI framework for building reactive web applications in Rust.

Leptos is a full-stack web framework that allows writing both server and client code in Rust.
It provides reactive primitives and a component model similar to modern JavaScript frameworks.

Key features:

* Leptos must only be used with the [`island` feature](https://book.leptos.dev/islands.html)
* The architecture follows the MPA paradigm, a "multi-page app"
* Server-side rendering
* Fine-grained reactivity
* Component-based architecture
* Full Rust syntax support

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
Leptos has some [built-in support for Playwright](https://book.leptos.dev/testing.html?highlight=playwrigh#playwright-with-counters).

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

The Mosaic component library provides reusable UI components built with Leptos and Tailwind CSS.

Components are defined in `modules/mosaic/tiles/` and can be previewed in the demo application at `modules/mosaic/demo/`.

To run the demo locally:

```bash
just watch-mosaic-demo
```

Pull requests that modify files in `modules/mosaic/` automatically receive a
Cloud Run preview deployment. The preview URL is posted as a comment on the PR.
