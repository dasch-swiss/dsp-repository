# Mosaic Demo

A demo and documentation application for the [mosaic-tiles](../tiles) component library. It serves as both a development playground and a storybook for exploring components with live examples, anatomy diagrams, and API references.

## Running

```bash
cargo leptos watch
```

The application will be available at `http://localhost:3000`.

## Structure

Each component demo lives in `src/components/[name]/` with:

- `anatomy.rs` - Component structure visualization
- `component.toml` - Metadata, example definitions, and API docs
- `examples/` - Individual example files demonstrating usage patterns

The `component.toml` defines:

- `name` - Component display name
- `description` - Summary of what the components is
- `[[examples]]` - Array of example
  - with `name` must match the Rust file
  - with `title` of the example page
  - and `description` of the example
- `[[references]]` - API documentation with component attributes
  - `attr` - Attribute name
  - `attr_type` - Rust type
  - `default` - Default value
  - `description` - What this attribute does

Routes and pages are auto-generated from the TOML metadata via a procedural macro.

## Adding Components

Use the `/add-component-to-demo` skill which provides step-by-step instructions for adding new component demos, including creating the directory structure, examples, TOML configuration, and registering routes.
