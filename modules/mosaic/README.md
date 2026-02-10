# Mosaic

A Leptos-based UI component library for the DaSCH Service Platform.

## Crates

```txt
mosaic/
├── tiles/        # Reusable component library
├── demo/         # Showcase and documentation web application
└── demo_macro/   # Proc macro for generating documentation pages
```

- **tiles** contains the components themselves, each feature-gated for selective inclusion. A build script bundles per-component CSS, processes it with Tailwind, and outputs a single minified stylesheet.
- **demo** is a Leptos web application that renders live examples, anatomy diagrams, and API references for each component. It uses `cargo-leptos` for building and serving.
- **demo_macro** provides the `generate_component_pages!()` proc macro, which reads `component.toml` metadata and example files to generate documentation pages at compile time.

## Components

Accordion, Badge, Breadcrumb, Button, ButtonGroup, Card, Icon, Link, Popover, Tabs

## Development

```bash
just watch-mosaic-demo    # Run demo with hot reload (watches tiles changes)
just fmt-mosaic           # Format with leptosfmt
```

See each crate's README for further details.
