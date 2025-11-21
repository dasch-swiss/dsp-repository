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

We use [Maud](https://maud.lambda.xyz/) as our templating engine for rendering HTML in Rust.

Maud is a macro-based templating engine that allows writing HTML directly in Rust code.
It is similar to JSX, where the HTML is written inline with the Rust code.
This approach provides strong type safety and excellent integration with Rust's ownership system.

Maud offers several advantages:
- Compile-time template checking
- Full Rust syntax support within templates
- Automatic HTML escaping
- No external template files to manage
- Better IDE support with syntax highlighting and autocomplete

### Archtectural Design Patterns

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
End-to-end tests are written either in Rust or in JavaScript using Playwright.

We are looking into how to integrate Playwright into our setup.
Playwright may be used for end-to-end tests simulating *entire user flows*.

For single user interactions, we should not need Playwright.
For these, we should use the Rust testing framework.

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

Instead of the traditional single-page application architecture,
where the client is a JavaScript application that runs in the browser,
and communicates with the server via JSON over HTTP,
we are using a hypermedia approach.

In this approach, the server sends HTML pages or fragments to the client.
The client simply renders the HTML, without much JavaScript.
The client also does not need to maintain state,
this is done by the server.
Server and client keep a connection open,
through which the server can send updates to the client,
so called "server-sent events" (SSE),
which guarantees responsiveness and interactivity.

This approach has several advantages:

- The client is much simpler, as it does not need to maintain state or manage complex interactions.
- The client does not have many security concerns, as the server is the source of truth 
  and can control what is sent to the client.
- The server can update the client at any time, without the need for the client to poll for updates.

Furthermore, with this approach, we can share much of the code between the server and the client.
With the UI-code living on the server, we can use the same language (Rust) for both server and client,
and there is no strict separation between backend and frontend.

The hypermedia approach is not a new concept, it has been used in the past, 
but had been widely replaced by the single-page application approach.
Lately, the hypermedia approach has been making a comeback.

This approach is best known from [HTMX](https://htmx.org/).
HTMX is fairly well established and there is a lot of learning material available.
Its major downside is that it is not sufficient for most use-cases, when used by itself.
Most of the time, you need to combine it with other libraries, such as Alpine.js.

Rather than HTMX, we are using [DataStar](https://data-star.dev/),
which provides similar functionality,
but is more compliant with web standards,
and provides a more complete solution,
so that we do not need to combine it with other libraries.

DataStar is a fairly new project,
so there is not much learning material available yet.
However, it is very similar to HTMX,
so that most of the HTMX learning material can be applied to DataStar as well.

## Design System

Rather than using a generic design system,
and designing solutions on top of it ad hoc,
we are using a purpose-built design system.
This will help us to keep the design consistent and coherent,
reduce complexity, and give our products a clearer brand identity.

This design system is built with Tailwind as its foundation,
providing modern, utility-first styling and component patterns.

### Concept, Aim and Purpose

We build the DSP Design System using Tailwind
because it provides a robust foundation for creating consistent, maintainable interfaces
that communicate the reliability and trustworthiness that users expect from an archive.

Our approach differs from using design systems as-is for several key reasons:

- **Purpose-built focus**: Rather than using a generic, customizable system,
  we create a focused design system tailored specifically to the DSP platform's needs.
  This reduces complexity and ensures consistency by limiting options to what we actually need.
- **Brand integration**: By building our own implementation, we can bake the DSP brand
  directly into the system, eliminating the need for extensive customization layers.
- **Tech stack alignment**: Our Rust-based, Maud-native implementation aligns perfectly
  with our technology choices, allowing us to move component implementation
  to dedicated design system work rather than ad-hoc development during project work.  
  The design system work will be done in close collaboration between Dev and PM,
  and is an up-fron investment that will improve the speed and quality of project work in the long run.
  

### Implementation

It is part of the discovery process to define components needed for any project.

These are then implemented in the design system as individual, reuseable components. 
It should then be possible to import these components as Rust modules,
and use them in the project code.

The details of the implementation depend on the rendering engine we decide to use.

### Playground

We create a playground for the design system,
where we can test and experiment with the components.

This playground is a Rust web server with TypeScript testing:
- Shell interface with component navigation
- Component isolation via iframe rendering
- Variant selection and theme switching
- Live reload via WebSocket
- TypeScript/Playwright tests for functional and accessibility testing

For each component, we create a page that shows the component in action,
in all its variants and states.
We also pages for compounds and patterns,
where we show how to use the components together.
Finally, we create sample pages that show how to use the components in a real-world scenario.
