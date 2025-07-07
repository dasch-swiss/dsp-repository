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
End-to-end tests are written either in Rust or in JavaScript using Playwright.

We are looking into how to integrate Playwright into our setup.
Playwright may be used for the following:

- End-to-end tests simulating *entire user flows*.
- Visual regression tests.

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

This design system is based on the [IBM Carbon Design System](https://carbondesignsystem.com/),
but is customized to our needs.

### Concept, Aim and Purpose

We build the DSP Design System on top of the Carbon Design System,
because it communicates the reliability and trustworthiness that users expect from an archive.
Furthermore, it is a well established design system,
where concerns such as accessibility and usability have been well thought out.

There are several reasons why we do not use the Carbon Design System as is,
and instead build our own design system on top of it:

- It is a customizeable, generic, general purpose design system.  
  As such, it provides a lot of options and flexibility but comes with a lot of complexity.  
  By customizing it to our needs, we can reduce complexity and limit it to our needs. 
  By limiting optins (e.g. components, icons, tokens, etc.) to the ones we need,
  we can simplify design and ensure consistency in our products.
- It is customizeable, but we do not need that flexibility.  
  By creating a purpose-built design system, we can bake our brand into it, and reduce complexity even further.
- The official and community implementations of Carbon do not align with our tech stack.  
  By creating our own implementation, we can tailor it to our needs
  and move the implementation of individual components to dedicated design system work,
  rather than having to implement them in the context of project work.  
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
- TypeScript/Playwright tests for functional, accessibility, and visual regression testing

For each component, we create a page that shows the component in action,
in all its variants and states.
We also pages for compounds and patterns,
where we show how to use the components together.
Finally, we create sample pages that show how to use the components in a real-world scenario.
