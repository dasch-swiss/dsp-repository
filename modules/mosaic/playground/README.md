# Mosaic Playground

A showcase and documentation application for the [mosaic-tiles](../tiles) component library. It serves as a development playground and storybook for exploring components with live examples and API references. It is a plain **Axum + Maud** binary — no Leptos, no WASM, no `cargo-leptos`.

## Running

```bash
just watch-mosaic-playground
```

This runs Tailwind in `--watch` mode alongside the server with hot reload. The application is available at `http://localhost:3000`.

## Structure

- `src/showcase/<name>.rs` — a hand-written showcase page per component (title, description, and rendered examples), each a `fn -> maud::Markup`.
- `src/showcase/mod.rs` — exports the showcase pages.
- `src/app.rs` — declares the routes with the native Axum router and the sidebar navigation entries.
- The document shell (head, nav, sidebar) is hand-written Maud in `src/app.rs`.

## Adding Components

Use the `/add-mosaic-component` skill. It walks through adding the tile in `tiles/`, importing its CSS into the consuming Tailwind entries, creating the `src/showcase/<name>.rs` page, exporting it in `src/showcase/mod.rs`, and registering the route and sidebar nav entry in `src/app.rs`.
