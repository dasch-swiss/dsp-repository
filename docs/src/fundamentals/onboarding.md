# Onboarding

## Rust

The main technology we use is Rust.
A solid understanding of Rust is needed, 
though particularly the frontend work does not require deep knowledge of Rust.

### Rust HTTP Server

We use [Axum](https://docs.rs/axum/latest/axum/) as our HTTP server.

### Serialization and Deserialization

We use [serde](https://serde.rs/) for serialization and deserialization of data.

### Rust Templating

We are still evaluating two templating engines for rendering HTML in Rust:

- [Askama](https://askama.readthedocs.io/en/stable/)
- [Maud](https://maud.lambda.xyz/)

Askama is a compile-time templating engine, that is very similar to Jinja2.
It uses a file-based approach, where separate template files are stored and dynamic content is injected into them.

Maud is a macro-based templating engine, that allows to write HTML directly in Rust code.
It is more similar to JSX, where the HTML is written inline with the Rust code.

We have to try both engines and see which one fits our needs better.
In principle, we can use both engines in the same project,
if they excel at different things.
But for simplicity, we should try to stick to one engine.

### Archtectural Design Patterns

We follow concepts such as [Clean Architecture](https://en.wikipedia.org/wiki/Clean_architecture),
[Hexagonal Architecture](https://en.wikipedia.org/wiki/Hexagonal_architecture_(software))
or [Onion Architecture](https://en.wikipedia.org/wiki/Onion_architecture).
Familiarity with these concepts will be helpful.

Some of the patterns must be adapted to the idioms of Rust,
but the general principles are the same.

### Testing

We follow the Testing Pyramid approach to testing, 
the majority of tests are unit tests, with a smaller number of integration tests, and a few end-to-end tests.

Unit and integration tests are written in Rust, following the Rust testing best practices.
For end-to-end we should see how [Playwright](https://playwright.dev/) works with our setup.


### Domain Driven Design

We do not follow strict Domain Driven Design (DDD) principles,
but we try to follow some of the concepts.
In particular, we try to keep the language used in code aligned with the domain language.

### Test Driven Development

We should absolutely do TDD and BDD.

## Database

We are still evaluating the database to use.

For the initial development, we work with static content or JSON files.

## Hypermedia

...

## Design System

...

### Concept

...

### Aim and Purpose

...

### IBM Carbon

...

### DSP Design System

...
